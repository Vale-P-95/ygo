//! A running duel: owns an engine instance and its reader callbacks.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::sync::Arc;

use crate::carddb::CardDb;
use crate::consts::{duel_flags, location, position};
use crate::deck::DeckList;
use crate::ffi::*;
use crate::message::{self, Decision, Event, Parsed};
use crate::scripts::ScriptStore;
use crate::state::TurnActivity;
use crate::Error;

/// Card data and scripts shared by every duel. Cheap to clone.
#[derive(Clone)]
pub struct Engine {
    cards: Arc<CardDb>,
    scripts: Arc<ScriptStore>,
}

impl Engine {
    /// Loads `<data_dir>/cdb/*.cdb` and `<data_dir>/scripts`.
    pub fn load(data_dir: &Path) -> Result<Self, Error> {
        Ok(Self::new(CardDb::load_dir(&data_dir.join("cdb"))?, ScriptStore::load_dir(&data_dir.join("scripts"))?))
    }

    pub fn new(cards: CardDb, scripts: ScriptStore) -> Self {
        Self { cards: Arc::new(cards), scripts: Arc::new(scripts) }
    }

    pub fn cards(&self) -> &CardDb {
        &self.cards
    }

    pub fn scripts(&self) -> &ScriptStore {
        &self.scripts
    }

    /// Version of the engine's C API.
    pub fn api_version() -> (i32, i32) {
        let (mut major, mut minor) = (0, 0);
        unsafe { OCG_GetVersion(&mut major, &mut minor) };
        (major, minor)
    }
}

#[derive(Debug, Clone)]
pub struct DuelConfig {
    pub seed: [u64; 4],
    pub flags: u64,
    pub starting_lp: u32,
}

impl Default for DuelConfig {
    fn default() -> Self {
        Self { seed: [1, 2, 3, 4], flags: duel_flags::MODE_MR5, starting_lp: 8000 }
    }
}

/// State reached after processing: either a decision to answer or the end.
#[derive(Debug, Clone)]
pub enum Step {
    Decision { decision: Decision, events: Vec<Event> },
    End { events: Vec<Event> },
}

/// Data handed to the engine callbacks. Only shared references are created
/// from the payload pointer because the script reader is re-entrant
/// (`utility.lua` loads further scripts while being loaded).
struct Context {
    cards: Arc<CardDb>,
    scripts: Arc<ScriptStore>,
    missing_scripts: RefCell<Vec<String>>,
    log: RefCell<Vec<String>>,
}

unsafe extern "C" fn card_reader(payload: *mut c_void, code: u32, data: *mut OCG_CardData) {
    let ctx = &*(payload as *const Context);
    let out = &mut *data;
    out.code = code;
    if let Some(card) = ctx.cards.get(code) {
        let setcodes: Box<[u16]> = card.setcodes.iter().copied().chain([0]).collect();
        out.setcodes = Box::into_raw(setcodes) as *mut u16;
        out.alias = card.alias;
        out.type_ = card.type_;
        out.level = card.level;
        out.attribute = card.attribute;
        out.race = card.race;
        out.attack = card.attack;
        out.defense = card.defense;
        out.lscale = card.lscale;
        out.rscale = card.rscale;
        out.link_marker = card.link_marker;
    }
}

unsafe extern "C" fn card_reader_done(_payload: *mut c_void, data: *mut OCG_CardData) {
    let data = &mut *data;
    if data.setcodes.is_null() {
        return;
    }
    let mut len = 0;
    while *data.setcodes.add(len) != 0 {
        len += 1;
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(data.setcodes, len + 1)));
    data.setcodes = std::ptr::null_mut();
}

unsafe extern "C" fn script_reader(payload: *mut c_void, duel: OCG_Duel, name: *const c_char) -> c_int {
    let ctx = &*(payload as *const Context);
    let name = CStr::from_ptr(name);
    let name_str = name.to_string_lossy();
    match ctx.scripts.load(&name_str) {
        Some(chunk) => OCG_LoadScript(duel, chunk.as_ptr() as *const c_char, chunk.len() as u32, name.as_ptr()),
        None => {
            // Code 0 is a placeholder the engine probes for; it has no script.
            if name_str != "c0.lua" {
                ctx.missing_scripts.borrow_mut().push(name_str.into_owned());
            }
            0
        }
    }
}

unsafe extern "C" fn log_handler(payload: *mut c_void, text: *const c_char, _kind: c_int) {
    let ctx = &*(payload as *const Context);
    ctx.log.borrow_mut().push(CStr::from_ptr(text).to_string_lossy().into_owned());
}

pub struct Duel {
    raw: OCG_Duel,
    ctx: Box<Context>,
    select_hint: u64,
    /// The decision waiting for an answer: after a rejected answer the engine
    /// does not ask again, so another answer must be given to this one.
    pending: Option<Decision>,
    /// Every card code put in the duel, for card declarations.
    known: Vec<u32>,
    turn: u32,
    turn_player: u8,
    phase: u16,
    activity: TurnActivity,
}

// Each duel is a self-contained engine instance (its own Lua state); the
// shared card data and scripts are read-only.
unsafe impl Send for Duel {}

impl Duel {
    pub fn new(engine: &Engine, config: &DuelConfig) -> Result<Self, Error> {
        let ctx = Box::new(Context {
            cards: engine.cards.clone(),
            scripts: engine.scripts.clone(),
            missing_scripts: RefCell::default(),
            log: RefCell::default(),
        });
        let payload = &*ctx as *const Context as *mut c_void;
        let player = OCG_Player { starting_lp: config.starting_lp, starting_draw_count: 0, draw_count_per_turn: 1 };
        let options = OCG_DuelOptions {
            seed: config.seed,
            flags: config.flags,
            team1: player,
            team2: player,
            card_reader: Some(card_reader),
            payload1: payload,
            script_reader: Some(script_reader),
            payload2: payload,
            log_handler: Some(log_handler),
            payload3: payload,
            card_reader_done: Some(card_reader_done),
            payload4: payload,
            enable_unsafe_libraries: 0,
        };
        let mut raw: OCG_Duel = std::ptr::null_mut();
        let status = unsafe { OCG_CreateDuel(&mut raw, &options) };
        if status != OCG_DUEL_CREATION_SUCCESS || raw.is_null() {
            return Err(Error::Engine(format!("duel creation failed (status {status})")));
        }
        let duel = Duel {
            raw,
            ctx,
            select_hint: 0,
            pending: None,
            known: Vec::new(),
            turn: 0,
            turn_player: 0,
            phase: 0,
            activity: TurnActivity::default(),
        };
        for base in ["constant.lua", "utility.lua"] {
            if !duel.load_script(base) {
                return Err(Error::Engine(format!("cannot load {base}: {}", duel.log().join("; "))));
            }
        }
        Ok(duel)
    }

    /// Creates a duel for solitaire testing ("goldfishing"): player 0 starts
    /// with exactly `hand` and the rest of `deck` in a seeded random order;
    /// the opponent has no cards and answers any prompt automatically.
    pub fn goldfish(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64) -> Result<Self, Error> {
        let mut main = deck.main.clone();
        let mut hand_codes = Vec::with_capacity(hand.len());
        for &code in hand {
            // Match alternate artworks too, and keep the deck's own copy.
            let wanted = engine.cards.canonical_code(code);
            let at = main
                .iter()
                .position(|&c| engine.cards.canonical_code(c) == wanted)
                .ok_or_else(|| Error::Data(format!("{} is not in the Main Deck (or not enough copies)", engine.cards.name(code))))?;
            hand_codes.push(main.remove(at));
        }
        shuffle(&mut main, seed);
        let config = DuelConfig {
            seed: [seed ^ 0x9e37_79b9_7f4a_7c15, seed.rotate_left(17) | 1, 0xdead_beef, 0x1234_5678],
            flags: duel_flags::MODE_MR5 | duel_flags::PSEUDO_SHUFFLE | duel_flags::SIMPLE_AI,
            starting_lp: 8000,
        };
        let mut duel = Duel::new(engine, &config)?;
        for (i, &code) in main.iter().enumerate() {
            duel.add_card(0, code, location::DECK, i as u32, position::FACEDOWN_DEFENSE);
        }
        for (i, &code) in deck.extra.iter().enumerate() {
            duel.add_card(0, code, location::EXTRA, i as u32, position::FACEDOWN_DEFENSE);
        }
        for (i, &code) in hand_codes.iter().enumerate() {
            duel.add_card(0, code, location::HAND, i as u32, position::FACEDOWN_DEFENSE);
        }
        duel.start();
        Ok(duel)
    }

    fn load_script(&self, name: &str) -> bool {
        let name = CString::new(name).expect("script name without NUL");
        let payload = &*self.ctx as *const Context as *mut c_void;
        unsafe { script_reader(payload, self.raw, name.as_ptr()) != 0 }
    }

    pub fn add_card(&mut self, owner: u8, code: u32, location: u8, sequence: u32, position: u32) {
        if !self.known.contains(&code) {
            self.known.push(code);
        }
        let info = OCG_NewCardInfo { team: owner, duelist: 0, code, con: owner, loc: location as u32, seq: sequence, pos: position };
        unsafe { OCG_DuelNewCard(self.raw, &info) };
    }

    pub fn start(&mut self) {
        unsafe { OCG_StartDuel(self.raw) };
    }

    /// Runs the engine until it needs a decision or the duel ends.
    pub fn advance(&mut self) -> Result<Step, Error> {
        let mut events = Vec::new();
        loop {
            let status = unsafe { OCG_DuelProcess(self.raw) };
            let buffer = unsafe {
                let mut len = 0u32;
                let ptr = OCG_DuelGetMessage(self.raw, &mut len);
                if ptr.is_null() {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()
                }
            };
            let mut decision = None;
            for record in message::split(&buffer) {
                match message::parse(record, self.select_hint) {
                    Parsed::Event(event) => {
                        self.track(&event);
                        events.push(event)
                    }
                    Parsed::Decision(d) => decision = Some(d),
                    Parsed::SelectHint(hint) => self.select_hint = hint,
                    Parsed::Ignored => {}
                }
            }
            if let Some(decision) = decision {
                self.select_hint = 0;
                self.pending = Some(decision.clone());
                return Ok(Step::Decision { decision, events });
            }
            if events.contains(&Event::Retry) {
                // The engine still waits for an answer to `pending`.
                return Err(Error::Rejected(self.log().join("; ")));
            }
            self.pending = None;
            match status {
                OCG_DUEL_STATUS_END => return Ok(Step::End { events }),
                OCG_DUEL_STATUS_AWAITING => {
                    return Ok(Step::Decision { decision: Decision::Unsupported { message: 0 }, events });
                }
                _ => {}
            }
        }
    }

    /// Keeps turn, phase and this turn's activity up to date.
    fn track(&mut self, event: &Event) {
        let a = &mut self.activity;
        match event {
            Event::NewTurn { player } => {
                self.turn += 1;
                self.turn_player = *player;
                *a = TurnActivity::default();
            }
            Event::NewPhase { phase } => self.phase = *phase,
            Event::NormalSummoning { card } if card.controller == self.turn_player => a.normal_summon_used = true,
            Event::Set { card } if card.location == location::MZONE && card.controller == self.turn_player => a.normal_summon_used = true,
            Event::SpecialSummoning { card } => {
                a.special_summons.push(card.code);
                a.special_summons.sort_unstable();
            }
            Event::Chaining { card, description, .. } => {
                a.activations.push((card.code, *description));
                a.activations.sort_unstable();
            }
            _ => {}
        }
    }

    /// `(turn, turn player, phase)`; turn 1 is the first.
    pub fn turn_info(&self) -> (u32, u8, u16) {
        (self.turn, self.turn_player, self.phase)
    }

    /// What was done this turn.
    pub fn activity(&self) -> &TurnActivity {
        &self.activity
    }

    pub(crate) fn raw_query_location(&self, player: u8, loc: u8, flags: u32) -> Vec<u8> {
        let info = OCG_QueryInfo { flags, con: player, loc: loc as u32, seq: 0, overlay_seq: 0 };
        unsafe {
            let mut len = 0u32;
            let ptr = OCG_DuelQueryLocation(self.raw, &mut len, &info);
            if ptr.is_null() {
                return Vec::new();
            }
            std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()
        }
    }

    pub(crate) fn raw_query_field(&self) -> Vec<u8> {
        unsafe {
            let mut len = 0u32;
            let ptr = OCG_DuelQueryField(self.raw, &mut len);
            if ptr.is_null() {
                return Vec::new();
            }
            std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()
        }
    }

    /// The decision waiting for an answer, also after a rejected answer.
    pub fn pending(&self) -> Option<&Decision> {
        self.pending.as_ref()
    }

    /// Codes of every card put in the duel (first seen first).
    pub fn known_codes(&self) -> &[u32] {
        &self.known
    }

    /// Legal answers to `decision`; card declarations offer the cards in this duel first.
    pub fn actions(&self, decision: &Decision) -> Vec<crate::Action> {
        crate::actions_with(decision, &self.ctx.cards, &self.known)
    }

    pub fn respond(&mut self, response: &[u8]) {
        unsafe { OCG_DuelSetResponse(self.raw, response.as_ptr() as *const c_void, response.len() as u32) };
    }

    /// Cards in one location of one player, in engine order (zones may be
    /// empty). Each entry is `(code, position)`.
    pub fn query_location(&self, player: u8, loc: u8) -> Vec<Option<(u32, u32)>> {
        self.query_cards(player, loc, 0x1 | 0x2).into_iter().map(|c| c.map(|c| (c.code, c.position))).collect()
    }

    pub fn count(&self, player: u8, loc: u8) -> u32 {
        unsafe { OCG_DuelQueryCount(self.raw, player, loc as u32) }
    }

    /// Scripts the engine asked for but that were not found.
    pub fn missing_scripts(&self) -> Vec<String> {
        self.ctx.missing_scripts.borrow().clone()
    }

    /// Messages logged by the engine and by card scripts (errors included).
    pub fn log(&self) -> Vec<String> {
        self.ctx.log.borrow().clone()
    }

    pub fn cards(&self) -> &CardDb {
        &self.ctx.cards
    }
}

impl Drop for Duel {
    fn drop(&mut self) {
        unsafe { OCG_DestroyDuel(self.raw) };
    }
}

/// Deterministic Fisher-Yates shuffle (SplitMix64).
pub(crate) fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    for i in (1..items.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_is_deterministic_permutation() {
        let mut a: Vec<u32> = (0..40).collect();
        let mut b = a.clone();
        shuffle(&mut a, 7);
        shuffle(&mut b, 7);
        assert_eq!(a, b);
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(sorted, (0..40).collect::<Vec<_>>());
        assert_ne!(a, (0..40).collect::<Vec<_>>());
    }

    #[test]
    fn engine_api_version_is_linked() {
        assert_eq!(Engine::api_version().0, 11);
    }
}
