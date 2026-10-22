//! A running duel: owns an engine instance and its reader callbacks.

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};
use std::mem::ManuallyDrop;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::carddb::CardDb;
use crate::consts::{card_type, duel_flags, location, position, summon_type};
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

/// Lines of the engine's log kept for each duel: a request stopped by the
/// Lua limit logs a failure for every entry into Lua after it.
const LOG_LINES: usize = 100;

unsafe extern "C" fn log_handler(payload: *mut c_void, text: *const c_char, _kind: c_int) {
    let ctx = &*(payload as *const Context);
    let mut log = ctx.log.borrow_mut();
    if log.len() < LOG_LINES {
        log.push(CStr::from_ptr(text).to_string_lossy().into_owned());
    }
}

pub struct Duel {
    raw: OCG_Duel,
    ctx: ManuallyDrop<Box<Context>>,
    /// Kept for the thread's next duel when dropped (see [`Spare`]).
    reusable: bool,
    /// With `DUELFORGE_CHECK_REUSE=1`, a reused duel plays the same in a
    /// new duel next to it, and must reach the same states.
    shadow: Option<Box<Duel>>,
    select_hint: u64,
    /// The decision waiting for an answer: after a rejected answer the engine
    /// does not ask again, so another answer must be given to this one.
    pending: Option<Decision>,
    /// The last answer given, for telling what was rejected.
    last_response: Vec<u8>,
    /// Every card code put in the duel, for card declarations.
    known: Vec<u32>,
    turn: u32,
    turn_player: u8,
    phase: u16,
    activity: TurnActivity,
    /// Links of the chain being built (0 once it starts to resolve).
    open_chain: usize,
    /// The links on the chain not resolved yet, first link first.
    chain: Vec<ChainLink>,
    /// The duel ends ([`Step::End`]) as soon as a later turn begins.
    last_turn: Option<u32>,
    /// The memory arena the engine allocates this duel in (shim/arena.cpp),
    /// or null: with one the duel can be saved and put back ([`Duel::save`]).
    arena: *mut c_void,
}

/// The arena of a duel active on this thread while the engine works for it;
/// the one active before comes back when this is dropped.
struct Entered(*mut c_void);

impl Drop for Entered {
    fn drop(&mut self) {
        unsafe { df_arena_enter(self.0) };
    }
}

/// A whole duel as it was at [`Duel::save`]: the engine's memory and what
/// this side of the engine tracks.
pub struct SavedDuel {
    arena: *mut c_void,
    memory: Vec<u8>,
    select_hint: u64,
    pending: Option<Decision>,
    last_response: Vec<u8>,
    known: Vec<u32>,
    turn: u32,
    turn_player: u8,
    phase: u16,
    activity: TurnActivity,
    open_chain: usize,
    chain: Vec<ChainLink>,
    last_turn: Option<u32>,
}

// The memory is plain bytes; the arena pointer only identifies the duel.
unsafe impl Send for SavedDuel {}

impl SavedDuel {
    /// Bytes of engine memory kept.
    pub fn size(&self) -> usize {
        self.memory.len()
    }
}

/// A link on the chain: whose, which effect of which card (the original
/// code of an alternate artwork), and from where it was activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainLink {
    pub controller: u8,
    pub code: u32,
    pub description: u64,
    pub location: u8,
}

/// A card put on the field before the duel starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Placement {
    pub code: u32,
    /// A Pendulum Monster in Spell & Trap Zone 0 or 4 is placed as a
    /// Pendulum Scale; an Extra Deck monster in a Monster Zone counts as
    /// Summoned by its own kind of summon, with `materials`.
    pub location: u8,
    pub sequence: u32,
    pub position: u32,
    /// Xyz Materials attached to it.
    pub overlay: Vec<u32>,
    /// The materials of an Extra Deck monster's summon ("gains 800 ATK for
    /// each material used"): each is a card placed in the GY, banished or
    /// attached to it, or a new one in the GY, where most materials go.
    /// Empty: an Xyz Monster's attached materials, none for the others
    /// (see [`generic_materials`]).
    pub materials: Vec<u32>,
    /// The Monster Zone of the monster a face-up card in a Spell & Trap
    /// Zone is equipped to. `None`: an Equip Card goes to the placed
    /// monster it can be equipped to (the one with the most ATK).
    pub equip_to: Option<u32>,
}

impl Placement {
    pub fn new(code: u32, location: u8, sequence: u32, position: u32) -> Self {
        Placement { code, location, sequence, position, ..Default::default() }
    }
}

/// Materials an Extra Deck monster could have been Summoned with, when the
/// real ones are not known: as many as a Link Monster's rating (each counts
/// as one), two for the others; Normal Monsters with different names,
/// which do nothing in the GY. Empty for a card not in the Extra Deck, or
/// if the database lacks them.
pub fn generic_materials(engine: &Engine, code: u32) -> Vec<u32> {
    const FILLERS: [&str; 6] =
        ["Alexandrite Dragon", "Mystical Elf", "Luster Dragon", "Gene-Warped Warwolf", "Giant Soldier of Stone", "Gemini Elf"];
    let db = engine.cards();
    let Some(card) = db.get(code).filter(|c| c.type_ & card_type::EXTRA != 0) else { return Vec::new() };
    let count = if card.type_ & card_type::LINK != 0 { (card.level & 0xff).max(1) as usize } else { 2 };
    let fillers: Vec<u32> = FILLERS.iter().filter_map(|n| db.find_by_name(n)).map(|c| c.code).collect();
    if fillers.is_empty() {
        return Vec::new();
    }
    (0..count).map(|i| fillers[i % fillers.len()]).collect()
}

/// A turn already begun when a duel is handed over
/// ([`Duel::goldfish_against_from`]): the answers given from the start,
/// then cards of the Deck that join the hand. A line played up to a point
/// goes on with one card more, and what the line used stays used (the
/// Normal Summon, effects once per turn).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Prelude {
    pub answers: Vec<Vec<u8>>,
    /// Put in the hand from the Deck with nothing happening (no draw, no
    /// event): as if they had been in the hand all along.
    pub joining: Vec<u32>,
}

/// The opponent's cards in [`Duel::goldfish_vs`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Opponent {
    pub hand: Vec<u32>,
    pub deck: Vec<u32>,
    pub extra: Vec<u32>,
}

// Each duel is a self-contained engine instance (its own Lua state); the
// shared card data and scripts are read-only.
unsafe impl Send for Duel {}

/// A duel over, kept by its thread to be emptied and used for the next one:
/// its base scripts stay loaded (shim/reuse.cpp), which saves about half of
/// what a short duel costs. `DUELFORGE_FRESH_DUELS=1` makes every duel new.
struct Spare {
    raw: OCG_Duel,
    ctx: ManuallyDrop<Box<Context>>,
}

impl Drop for Spare {
    fn drop(&mut self) {
        unsafe {
            df_duel_destroy(self.raw);
            ManuallyDrop::drop(&mut self.ctx);
        }
    }
}

thread_local! {
    static SPARE: RefCell<Option<Spare>> = const { RefCell::new(None) };
    /// See [`fresh_duels`].
    static FRESH: Cell<bool> = const { Cell::new(false) };
    /// The card scripts are not loaded ahead (see `df_duel_preload`).
    static NO_PRELOAD: Cell<bool> = const { Cell::new(false) };
    /// See [`arena_duels`].
    static ARENAS: Cell<bool> = const { Cell::new(false) };
}

/// Lua that gives each placed Extra Deck monster its materials (a placed
/// card of that code in the GY, banished or attached to it, each used
/// once, else a new one in the GY; an Xyz Monster without `materials` has
/// its attached ones), then plays what its summon does by rule: its checks
/// of the materials and its continuous effects when Special Summoned
/// ("its original ATK becomes 800 x the materials"). Its optional and
/// mandatory triggers are not played: the line that made the board used them.
fn placement_materials(engine: &Engine, placed: &[Placement]) -> String {
    let db = engine.cards();
    let mut lua = String::new();
    for p in placed {
        let type_ = db.get(p.code).map_or(0, |c| c.type_);
        if p.location != location::MZONE || type_ & card_type::EXTRA == 0 {
            continue;
        }
        let materials = if !p.materials.is_empty() {
            let codes: Vec<String> = p.materials.iter().map(|&m| db.canonical_code(m).to_string()).collect();
            format!("local g=Group.CreateGroup() for _,code in ipairs({{{}}}) do local m=df_take(c,code) if m then g:AddCard(m) end end c:SetMaterial(g)", codes.join(","))
        } else if !p.overlay.is_empty() {
            "c:SetMaterial(c:GetOverlayGroup())".to_owned()
        } else {
            String::new()
        };
        lua +=
            &format!("do local c=Duel.GetFieldCard(0,{},{}) if c then {materials} df_summoned(c) end end\n", location::MZONE, p.sequence);
    }
    if lua.is_empty() {
        return lua;
    }
    format!(
        "local used=Group.CreateGroup()
local function df_take(c,code)
  local function free(tc) return tc:GetOriginalCode()==code and not used:IsContains(tc) end
  local pool=Duel.GetMatchingGroup(free,0,{},0,nil)
  pool:Merge(c:GetOverlayGroup():Filter(free,nil))
  local m=pool:GetFirst() or Debug.AddCard(code,0,0,{},0,{})
  if m then used:AddCard(m) end
  return m
end
local function df_summoned(c)
  local eg=Group.FromCards(c)
  local effects={{c:GetOwnEffects()}}
  for _,e in ipairs(effects) do
    local v=e:GetValue()
    if e:GetCode()==EFFECT_MATERIAL_CHECK and type(v)=='function' then pcall(v,e,c) end
  end
  for _,e in ipairs(effects) do
    local t=e:GetType()
    if e:GetCode()==EVENT_SPSUMMON_SUCCESS and t&EFFECT_TYPE_SINGLE~=0 and t&EFFECT_TYPE_CONTINUOUS~=0 then
      local cond,op=e:GetCondition(),e:GetOperation()
      if op and (not cond or cond(e,0,eg,0,0,nil,0,0)) then pcall(op,e,0,eg,0,0,nil,0,0) end
    end
  end
end
{lua}",
        location::GRAVE | location::REMOVED,
        location::GRAVE,
        position::FACEUP_ATTACK,
    )
}

/// Lua that equips each placed face-up card with a target to it, and each
/// Equip Card without one to the placed monster it can be equipped to that
/// has the most ATK (none: it stays as it is).
fn placement_equips(engine: &Engine, placed: &[Placement]) -> String {
    let db = engine.cards();
    let mut lua = String::new();
    for p in placed {
        if p.location != location::SZONE || p.position & (position::FACEUP_ATTACK | position::FACEUP_DEFENSE) == 0 {
            continue;
        }
        let equip = db.get(p.code).is_some_and(|c| c.type_ & card_type::EQUIP != 0);
        let target = match p.equip_to {
            Some(zone) => format!("Duel.GetFieldCard(0,{},{zone})", location::MZONE),
            None if equip => "df_best(ec)".to_owned(),
            None => continue,
        };
        lua += &format!(
            "do local ec=Duel.GetFieldCard(0,{},{}) local tc=ec and {target} if ec and tc then Debug.PreEquip(ec,tc) end end\n",
            location::SZONE,
            p.sequence
        );
    }
    if !lua.contains("df_best") {
        return lua;
    }
    format!(
        "local function df_best(ec)
  local best=nil
  local g=Duel.GetMatchingGroup(function(m) return m:IsFaceup() and ec:CheckEquipTarget(m) end,0,{},0,nil)
  local m=g:GetFirst()
  while m do
    if not best or m:GetAttack()>best:GetAttack() then best=m end
    m=g:GetNext()
  end
  return best
end
{lua}",
        location::MZONE
    )
}

fn check_reuse() -> bool {
    static CHECK: OnceLock<bool> = OnceLock::new();
    *CHECK.get_or_init(|| std::env::var_os("DUELFORGE_CHECK_REUSE").is_some())
}

fn reuse_duels() -> bool {
    static REUSE: OnceLock<bool> = OnceLock::new();
    *REUSE.get_or_init(|| std::env::var_os("DUELFORGE_FRESH_DUELS").is_none()) && !FRESH.with(|f| f.get())
}

/// The duels this thread makes from now on live in a memory arena (`true`)
/// and can be saved and put back ([`Duel::save`], [`Duel::restore`]); they
/// are not reused (see [`fresh_duels`]).
pub fn arena_duels(on: bool) {
    ARENAS.with(|a| a.set(on));
}

/// Every duel of this thread new from now on (`true`), or reused when
/// possible: to check that a reused duel plays like a new one.
#[doc(hidden)]
pub fn fresh_duels(fresh: bool) {
    FRESH.with(|f| f.set(fresh));
    if fresh {
        SPARE.with(|s| s.borrow_mut().take());
    }
}

fn duel_options(config: &DuelConfig, ctx: &Context) -> OCG_DuelOptions {
    let payload = ctx as *const Context as *mut c_void;
    let player = OCG_Player { starting_lp: config.starting_lp, starting_draw_count: 0, draw_count_per_turn: 1 };
    OCG_DuelOptions {
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
    }
}

impl Duel {
    pub fn new(engine: &Engine, config: &DuelConfig) -> Result<Self, Error> {
        if ARENAS.with(Cell::get) {
            let arena = unsafe { df_arena_new() };
            if arena.is_null() {
                return Err(Error::Engine("no memory arena left for a new duel".to_owned()));
            }
            let before = Entered(unsafe { df_arena_enter(arena) });
            let duel = Self::fresh(engine, config, arena);
            drop(before);
            if duel.is_err() {
                unsafe { df_arena_free(arena) };
            }
            return duel;
        }
        if let Some(mut duel) = Self::reused(engine, config) {
            if check_reuse() {
                duel.shadow = Some(Box::new(Self::fresh(engine, config, std::ptr::null_mut())?));
            }
            return Ok(duel);
        }
        let mut duel = Self::fresh(engine, config, std::ptr::null_mut())?;
        duel.reusable = reuse_duels() && unsafe { df_duel_snapshot(duel.raw) } != 0;
        Ok(duel)
    }

    /// A duel with the base scripts loaded just now, in `arena` (or null).
    fn fresh(engine: &Engine, config: &DuelConfig, arena: *mut c_void) -> Result<Self, Error> {
        let ctx = Box::new(Context {
            cards: engine.cards.clone(),
            scripts: engine.scripts.clone(),
            missing_scripts: RefCell::default(),
            log: RefCell::default(),
        });
        let options = duel_options(config, &ctx);
        let mut raw: OCG_Duel = std::ptr::null_mut();
        let status = unsafe { OCG_CreateDuel(&mut raw, &options) };
        if status != OCG_DUEL_CREATION_SUCCESS || raw.is_null() {
            return Err(Error::Engine(format!("duel creation failed (status {status})")));
        }
        lua_count::install(raw);
        let mut duel = Self::with(raw, ManuallyDrop::new(ctx));
        duel.arena = arena;
        for base in ["constant.lua", "utility.lua"] {
            if !duel.load_script(base) {
                return Err(Error::Engine(format!("cannot load {base}: {}", duel.log().join("; "))));
            }
        }
        Ok(duel)
    }

    /// The thread's spare duel emptied for a new one, if it has one for
    /// this engine.
    fn reused(engine: &Engine, config: &DuelConfig) -> Option<Self> {
        let spare = SPARE.with(|s| s.borrow_mut().take())?;
        if !Arc::ptr_eq(&spare.ctx.cards, &engine.cards) || !Arc::ptr_eq(&spare.ctx.scripts, &engine.scripts) {
            return None;
        }
        spare.ctx.log.borrow_mut().clear();
        spare.ctx.missing_scripts.borrow_mut().clear();
        let options = duel_options(config, &spare.ctx);
        if unsafe { df_duel_reset(spare.raw, &options) } == 0 {
            return None;
        }
        let mut spare = ManuallyDrop::new(spare);
        let ctx = unsafe { ManuallyDrop::take(&mut spare.ctx) };
        let mut duel = Self::with(spare.raw, ManuallyDrop::new(ctx));
        duel.reusable = true;
        Some(duel)
    }

    fn with(raw: OCG_Duel, ctx: ManuallyDrop<Box<Context>>) -> Self {
        Duel {
            raw,
            ctx,
            reusable: false,
            shadow: None,
            select_hint: 0,
            pending: None,
            last_response: Vec::new(),
            known: Vec::new(),
            turn: 0,
            turn_player: 0,
            phase: 0,
            activity: TurnActivity::default(),
            open_chain: 0,
            chain: Vec::new(),
            last_turn: None,
            arena: std::ptr::null_mut(),
        }
    }

    /// Makes this duel's arena (or none) the active one while the engine
    /// works for it.
    fn enter(&self) -> Entered {
        Entered(unsafe { df_arena_enter(self.arena) })
    }

    /// The whole duel as it is now, to put back with [`Self::restore`]; only
    /// for a duel made in an arena ([`arena_duels`]).
    pub fn save(&self) -> Result<SavedDuel, Error> {
        if self.arena.is_null() {
            return Err(Error::Engine("this duel has no memory arena: it cannot be saved".to_owned()));
        }
        if unsafe { df_arena_overflowed(self.arena) } != 0 {
            return Err(Error::Engine("the duel outgrew its memory arena: it cannot be saved".to_owned()));
        }
        let mut memory = vec![0u8; unsafe { df_arena_used(self.arena) }];
        unsafe { df_arena_save(self.arena, memory.as_mut_ptr().cast()) };
        Ok(SavedDuel {
            arena: self.arena,
            memory,
            select_hint: self.select_hint,
            pending: self.pending.clone(),
            last_response: self.last_response.clone(),
            known: self.known.clone(),
            turn: self.turn,
            turn_player: self.turn_player,
            phase: self.phase,
            activity: self.activity.clone(),
            open_chain: self.open_chain,
            chain: self.chain.clone(),
            last_turn: self.last_turn,
        })
    }

    /// Puts the duel back as it was at [`Self::save`] (of this same duel).
    pub fn restore(&mut self, saved: &SavedDuel) -> Result<(), Error> {
        if saved.arena != self.arena || self.arena.is_null() {
            return Err(Error::Engine("a duel can only be put back from its own save".to_owned()));
        }
        unsafe { df_arena_restore(self.arena, saved.memory.as_ptr().cast(), saved.memory.len()) };
        self.select_hint = saved.select_hint;
        self.pending = saved.pending.clone();
        self.last_response = saved.last_response.clone();
        self.known = saved.known.clone();
        self.turn = saved.turn;
        self.turn_player = saved.turn_player;
        self.phase = saved.phase;
        self.activity = saved.activity.clone();
        self.open_chain = saved.open_chain;
        self.chain = saved.chain.clone();
        self.last_turn = saved.last_turn;
        Ok(())
    }

    /// Creates a duel for solitaire testing ("goldfishing"): player 0 starts
    /// with exactly `hand` and the rest of `deck` in a seeded random order;
    /// the opponent has no cards and answers any prompt automatically. The
    /// duel ends when the opponent's turn begins: the state then is the
    /// board you pass with (not after your next draw).
    pub fn goldfish(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64) -> Result<Self, Error> {
        let mut duel = Self::solitaire(engine, deck, hand, seed, None, &[])?;
        duel.last_turn = Some(1);
        Ok(duel)
    }

    /// Like [`Duel::goldfish`], but the opponent has `opponent`'s cards (if
    /// any) during your turn, like in [`Duel::goldfish_vs`]: a line found
    /// against it plays the same when the opponent's turn follows. Cards
    /// that look at the opponent's Deck, hand or Extra Deck offer the same
    /// choices in both. The opponent's decisions are asked like yours.
    pub fn goldfish_against(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: Option<&Opponent>) -> Result<Self, Error> {
        Self::goldfish_against_placed(engine, deck, hand, seed, opponent, &[])
    }

    /// Like [`Duel::goldfish_against`], with cards of yours already where
    /// `placed` puts them when the duel starts.
    pub fn goldfish_against_placed(
        engine: &Engine,
        deck: &DeckList,
        hand: &[u32],
        seed: u64,
        opponent: Option<&Opponent>,
        placed: &[Placement],
    ) -> Result<Self, Error> {
        let mut duel = Self::solitaire(engine, deck, hand, seed, opponent, placed)?;
        duel.last_turn = Some(1);
        Ok(duel)
    }

    /// Like [`Duel::goldfish_against_placed`], with `prelude` played: the
    /// duel goes on from there. Fails if an answer is not asked for or a
    /// card to join the hand is not in the Deck.
    pub fn goldfish_against_from(
        engine: &Engine,
        deck: &DeckList,
        hand: &[u32],
        seed: u64,
        opponent: Option<&Opponent>,
        placed: &[Placement],
        prelude: &Prelude,
    ) -> Result<Self, Error> {
        let mut duel = Self::goldfish_against_placed(engine, deck, hand, seed, opponent, placed)?;
        for answer in &prelude.answers {
            match duel.advance()? {
                Step::Decision { .. } => duel.respond(answer),
                Step::End { .. } => return Err(Error::Engine("the duel ended before the prelude did".to_owned())),
            }
        }
        for &code in &prelude.joining {
            duel.deck_to_hand(code)?;
        }
        Ok(duel)
    }

    /// Moves a card of `code` from your Deck to your hand with nothing
    /// happening (see [`Prelude::joining`]).
    pub fn deck_to_hand(&mut self, code: u32) -> Result<(), Error> {
        if let Some(shadow) = &mut self.shadow {
            shadow.deck_to_hand(code)?;
        }
        let wanted = self.ctx.cards.canonical_code(code);
        let moved = {
            let _arena = self.enter();
            unsafe { df_deck_to_hand(self.raw, 0, wanted) }
        };
        if moved == 0 {
            return Err(Error::Data(format!("{} is not in the Deck", self.ctx.cards.name(code))));
        }
        Ok(())
    }

    /// Like [`Duel::goldfish`], with the same Deck order and random numbers
    /// (so the same answers play the same turn), but the opponent has
    /// `opponent`'s cards and its decisions are asked like yours. The
    /// opponent's cards are dealt in the order given: the last card of its
    /// Deck is drawn first.
    pub fn goldfish_vs(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &Opponent) -> Result<Self, Error> {
        Self::solitaire(engine, deck, hand, seed, Some(opponent), &[])
    }

    /// Like [`Duel::goldfish_vs`], with cards of yours already on the field
    /// when the duel starts (as if Summoned or Set properly): a board to
    /// put to the test without playing the turn that makes it.
    pub fn goldfish_vs_placed(
        engine: &Engine,
        deck: &DeckList,
        hand: &[u32],
        seed: u64,
        opponent: &Opponent,
        placed: &[Placement],
    ) -> Result<Self, Error> {
        Self::solitaire(engine, deck, hand, seed, Some(opponent), placed)
    }

    fn solitaire(
        engine: &Engine,
        deck: &DeckList,
        hand: &[u32],
        seed: u64,
        opponent: Option<&Opponent>,
        placed: &[Placement],
    ) -> Result<Self, Error> {
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
        // A card placed is one of the deck's copies when the deck has it
        // (it is not in the Deck any more).
        let mut extra = deck.extra.clone();
        for p in placed {
            let wanted = engine.cards.canonical_code(p.code);
            let from_extra = engine.cards.get(p.code).is_some_and(|c| c.type_ & card_type::EXTRA != 0);
            let pile = if from_extra { &mut extra } else { &mut main };
            if let Some(at) = pile.iter().position(|&c| engine.cards.canonical_code(c) == wanted) {
                pile.remove(at);
            }
        }
        shuffle(&mut main, seed);
        let config = DuelConfig {
            seed: [seed ^ 0x9e37_79b9_7f4a_7c15, seed.rotate_left(17) | 1, 0xdead_beef, 0x1234_5678],
            flags: duel_flags::MODE_MR5 | duel_flags::PSEUDO_SHUFFLE | if opponent.is_none() { duel_flags::SIMPLE_AI } else { 0 },
            starting_lp: 8000,
        };
        let mut duel = Duel::new(engine, &config)?;
        if duel.reusable && !NO_PRELOAD.with(Cell::get) {
            let theirs = opponent.into_iter().flat_map(|o| o.deck.iter().chain(&o.extra).chain(&o.hand));
            let codes: Vec<u32> = main
                .iter()
                .chain(&extra)
                .chain(&hand_codes)
                .chain(theirs)
                .copied()
                .chain(
                    placed.iter().flat_map(|p| std::iter::once(p.code).chain(p.overlay.iter().copied()).chain(p.materials.iter().copied())),
                )
                .collect();
            if unsafe { df_duel_preload(duel.raw, codes.as_ptr(), codes.len()) } == 0 {
                // A script did more than define its card when loaded: this
                // duel is not like a new one. A new duel, and no more
                // loading ahead on this thread.
                NO_PRELOAD.with(|n| n.set(true));
                duel.reusable = false;
                duel = Self::fresh(engine, &config, std::ptr::null_mut())?;
            }
        }
        for (i, &code) in main.iter().enumerate() {
            duel.add_card(0, code, location::DECK, i as u32, position::FACEDOWN_DEFENSE);
        }
        for (i, &code) in extra.iter().enumerate() {
            duel.add_card(0, code, location::EXTRA, i as u32, position::FACEDOWN_DEFENSE);
        }
        for (i, &code) in hand_codes.iter().enumerate() {
            duel.add_card(0, code, location::HAND, i as u32, position::FACEDOWN_DEFENSE);
        }
        if let Some(o) = opponent {
            for (cards, loc) in [(&o.deck, location::DECK), (&o.extra, location::EXTRA), (&o.hand, location::HAND)] {
                for (i, &code) in cards.iter().enumerate() {
                    duel.add_card(1, code, loc, i as u32, position::FACEDOWN_DEFENSE);
                }
            }
        }
        for p in placed {
            let type_ = engine.cards.get(p.code).map_or(0, |c| c.type_);
            if p.location == location::SZONE && type_ & card_type::PENDULUM != 0 && matches!(p.sequence, 0 | 4) {
                // A Pendulum Scale: only the Debug library puts a card in a Pendulum Zone.
                if !duel.known.contains(&p.code) {
                    duel.known.push(p.code);
                }
                // LOCATION_PZONE, the left or right scale.
                let lua = format!("Debug.AddCard({},0,0,0x200,{},{})", p.code, u32::from(p.sequence == 4), position::FACEUP_ATTACK);
                duel.run_lua_source("placement.lua", &lua);
                continue;
            }
            duel.add_card(0, p.code, p.location, p.sequence, p.position);
            if p.location == location::MZONE && type_ & card_type::EXTRA != 0 {
                // Summoned by its own kind of summon ("if this card was Link Summoned").
                let kind = if type_ & card_type::LINK != 0 {
                    summon_type::LINK
                } else if type_ & card_type::XYZ != 0 {
                    summon_type::XYZ
                } else if type_ & card_type::SYNCHRO != 0 {
                    summon_type::SYNCHRO
                } else {
                    summon_type::FUSION
                };
                let lua = format!("Debug.PreSummon(Duel.GetFieldCard(0,{},{}),{kind},{})", location::MZONE, p.sequence, location::EXTRA);
                duel.run_lua_source("placement.lua", &lua);
            }
            // A card added to a taken Monster Zone becomes a material of the monster there.
            for material in &p.overlay {
                let lua = format!("Debug.AddCard({material},0,0,{},{},{})", location::MZONE, p.sequence, position::FACEUP_ATTACK);
                duel.run_lua_source("placement.lua", &lua);
            }
        }
        // Once every card is there: what a summon's materials and an Equip
        // Card's target can be.
        let materials = placement_materials(engine, placed);
        if !materials.is_empty() {
            duel.run_lua_source("placement.lua", &materials);
        }
        let equips = placement_equips(engine, placed);
        if !equips.is_empty() {
            duel.run_lua_source("placement.lua", &equips);
        }
        duel.start();
        Ok(duel)
    }

    /// Runs a script of the script folder in this duel (for diagnostics).
    #[doc(hidden)]
    pub fn run_lua(&mut self, name: &str) -> bool {
        // What it changes may not be undone for the next duel.
        self.reusable = false;
        self.load_script(name)
    }

    /// Runs Lua code in this duel (the `Debug` library included).
    fn run_lua_source(&self, name: &str, source: &str) -> bool {
        if let Some(shadow) = &self.shadow {
            shadow.run_lua_source(name, source);
        }
        let name = CString::new(name).expect("script name without NUL");
        let _arena = self.enter();
        unsafe { OCG_LoadScript(self.raw, source.as_ptr() as *const c_char, source.len() as u32, name.as_ptr()) != 0 }
    }

    fn load_script(&self, name: &str) -> bool {
        let name = CString::new(name).expect("script name without NUL");
        let payload = &**self.ctx as *const Context as *mut c_void;
        let _arena = self.enter();
        unsafe { script_reader(payload, self.raw, name.as_ptr()) != 0 }
    }

    pub fn add_card(&mut self, owner: u8, code: u32, location: u8, sequence: u32, position: u32) {
        if !self.known.contains(&code) {
            self.known.push(code);
        }
        let info = OCG_NewCardInfo { team: owner, duelist: 0, code, con: owner, loc: location as u32, seq: sequence, pos: position };
        {
            let _arena = self.enter();
            unsafe { OCG_DuelNewCard(self.raw, &info) };
        }
        if let Some(shadow) = &mut self.shadow {
            shadow.add_card(owner, code, location, sequence, position);
        }
    }

    pub fn start(&mut self) {
        {
            let _arena = self.enter();
            unsafe { OCG_StartDuel(self.raw) };
        }
        if let Some(shadow) = &mut self.shadow {
            shadow.start();
        }
    }

    /// Runs the engine until it needs a decision or the duel ends.
    pub fn advance(&mut self) -> Result<Step, Error> {
        let request = lua_count::start();
        let step = self.advance_once();
        if lua_count::finish(request) {
            // The scripts stopped half-way: the duel is not worth going on with.
            self.reusable = false;
            self.shadow = None;
            return Err(Error::Engine("a card script ran too long (DUELFORGE_LUA_LIMIT)".into()));
        }
        if let Some(mut shadow) = self.shadow.take() {
            shadow.last_turn = self.last_turn;
            let fresh = shadow.advance_once();
            let (a, b) = (format!("{step:?}"), format!("{fresh:?}"));
            let (sa, sb) = (self.state(), shadow.state());
            self.shadow = Some(shadow);
            if a != b || sa != sb {
                eprintln!("a reused duel plays differently from a new one\nreused: {a}\n{sa:?}\nnew: {b}\n{sb:?}");
                std::process::abort();
            }
        }
        step
    }

    fn advance_once(&mut self) -> Result<Step, Error> {
        let mut events = Vec::new();
        loop {
            let arena = self.enter();
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
            drop(arena);
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
            if self.last_turn.is_some_and(|last| self.turn > last) {
                self.pending = None;
                return Ok(Step::End { events });
            }
            if let Some(decision) = decision {
                self.select_hint = 0;
                self.pending = Some(decision.clone());
                return Ok(Step::Decision { decision, events });
            }
            if events.contains(&Event::Retry) {
                self.reusable = false;
                // The engine still waits for an answer to `pending`.
                let mut why = format!("{:?} answered {:?}", self.pending, self.last_response);
                for line in self.log() {
                    why.push_str("; ");
                    why.push_str(&line);
                }
                return Err(Error::Rejected(why));
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
                self.open_chain += 1;
                let code = self.ctx.cards.canonical_code(card.code);
                self.chain.push(ChainLink { controller: card.controller, code, description: *description, location: card.location });
            }
            Event::ChainSolving { .. } => self.open_chain = 0,
            Event::ChainSolved { .. } => {
                self.chain.pop();
            }
            Event::ChainEnd => {
                self.open_chain = 0;
                self.chain.clear();
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

    /// Links of the chain being built: a chain decision with links already
    /// open asks for a response, one without starts a new chain.
    pub fn open_chain(&self) -> usize {
        self.open_chain
    }

    /// The links on the chain not resolved yet, first link first: what the
    /// field does not show of a chain being built or resolved.
    pub fn chain(&self) -> &[ChainLink] {
        &self.chain
    }

    pub(crate) fn raw_query_location(&self, player: u8, loc: u8, flags: u32) -> Vec<u8> {
        let info = OCG_QueryInfo { flags, con: player, loc: loc as u32, seq: 0, overlay_seq: 0 };
        let _arena = self.enter();
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
        let _arena = self.enter();
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
        if let Some(shadow) = &mut self.shadow {
            shadow.respond(response);
        }
        self.last_response = response.to_vec();
        let _arena = self.enter();
        unsafe { OCG_DuelSetResponse(self.raw, response.as_ptr() as *const c_void, response.len() as u32) };
    }

    /// Cards in one location of one player, in engine order (zones may be
    /// empty). Each entry is `(code, position)`.
    pub fn query_location(&self, player: u8, loc: u8) -> Vec<Option<(u32, u32)>> {
        self.query_cards(player, loc, 0x1 | 0x2).into_iter().map(|c| c.map(|c| (c.code, c.position))).collect()
    }

    pub fn count(&self, player: u8, loc: u8) -> u32 {
        let _arena = self.enter();
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
        if !self.arena.is_null() {
            // Its memory goes with the arena.
            {
                let _arena = self.enter();
                unsafe { df_duel_destroy(self.raw) };
            }
            unsafe {
                df_arena_free(self.arena);
                ManuallyDrop::drop(&mut self.ctx);
            }
            return;
        }
        let ctx = unsafe { ManuallyDrop::take(&mut self.ctx) };
        let spare = Spare { raw: self.raw, ctx: ManuallyDrop::new(ctx) };
        if self.reusable && !std::thread::panicking() {
            // try_with: the thread may be exiting.
            let _ = SPARE.try_with(|s| s.borrow_mut().replace(spare));
        }
    }
}

/// Deterministic Fisher-Yates shuffle (SplitMix64): the order of your Deck in
/// a goldfish duel with `seed`, the cards of the hand taken out first.
pub fn shuffle<T>(items: &mut [T], seed: u64) {
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

/// Lua work per engine request (from an answer to the next decision). A
/// request stops once the core has entered Lua `DUELFORGE_LUA_LIMIT` times
/// (500 000 by default, 0 for no limit): a few positions make a card script
/// enter Lua 0.5-5 million times in seconds, and the search pays them again
/// at every replay, while no request of 82 searches on 41 decks of Master
/// Duel Meta entered 120 000 times (`docs/plans/velocita-ricerca.md`).
///
/// With `DUELFORGE_LUA_COUNT=<file>` each request also adds four
/// little-endian `u32` to the file at exit: thousands of instructions,
/// calls, entries into Lua, and microseconds.
mod lua_count {
    use std::os::raw::c_int;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    use crate::ffi::{df_lua_hook_install, df_lua_request_end, df_lua_request_start, OCG_Duel};

    const DEFAULT_LIMIT: u64 = 500_000;

    static RECORDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    extern "C" {
        fn atexit(f: extern "C" fn()) -> c_int;
    }

    fn output() -> Option<&'static PathBuf> {
        static OUTPUT: OnceLock<Option<PathBuf>> = OnceLock::new();
        OUTPUT
            .get_or_init(|| {
                let path = std::env::var_os("DUELFORGE_LUA_COUNT").map(PathBuf::from)?;
                unsafe { atexit(write) };
                Some(path)
            })
            .as_ref()
    }

    fn limit() -> u64 {
        static LIMIT: OnceLock<u64> = OnceLock::new();
        *LIMIT.get_or_init(|| std::env::var("DUELFORGE_LUA_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_LIMIT))
    }

    extern "C" fn write() {
        let (Some(path), Ok(records)) = (output(), RECORDS.lock()) else { return };
        let bytes: Vec<u8> = records.iter().flat_map(|v| v.to_le_bytes()).collect();
        let written =
            std::fs::OpenOptions::new().create(true).append(true).open(path).and_then(|mut f| std::io::Write::write_all(&mut f, &bytes));
        if let Err(e) = written {
            eprintln!("DUELFORGE_LUA_COUNT: cannot write {}: {e}", path.display());
        }
    }

    pub fn install(raw: OCG_Duel) {
        if output().is_some() {
            unsafe { df_lua_hook_install(raw) };
        }
    }

    pub struct Request(Option<Instant>);

    /// Starts a request: what Lua ran before it is not its own.
    pub fn start() -> Request {
        if limit() == 0 && output().is_none() {
            return Request(None);
        }
        start_with(limit())
    }

    /// A request with at most `max` entries into Lua (0: no limit).
    pub fn start_with(max: u64) -> Request {
        unsafe { df_lua_request_start(max) };
        Request(Some(Instant::now()))
    }

    /// Ends a request; true when the limit stopped it.
    pub fn finish(request: Request) -> bool {
        let Request(Some(started)) = request else { return false };
        let (mut entries, mut calls, mut instructions) = (0u64, 0u64, 0u64);
        let stopped = unsafe { df_lua_request_end(&mut entries, &mut calls, &mut instructions) } != 0;
        if output().is_some() {
            let micros = started.elapsed().as_micros();
            if let Ok(mut records) = RECORDS.lock() {
                let field = |v: u128| v.min(u32::MAX as u128) as u32;
                records.extend([field((instructions / 1000).into()), field(calls.into()), field(entries.into()), field(micros)]);
            }
        }
        stopped
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
    fn a_request_stops_at_the_lua_limit() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::var_os("DUELFORGE_DATA").map(std::path::PathBuf::from).unwrap_or_else(|| root.join("data"));
        if !data.join("scripts/constant.lua").is_file() {
            eprintln!("skipped: no game data in {}", data.display());
            return;
        }
        let engine = Engine::load(&data).unwrap();
        let ydk = std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap();
        let mut duel = Duel::goldfish_vs(&engine, &DeckList::from_ydk(&ydk).unwrap(), &[], 1, &Opponent::default()).unwrap();
        // A script that enters Lua again and again, run as a request would be:
        // past the limit each entry fails, the script goes on to its end.
        let request = lua_count::start_with(1000);
        let source = "local g = Duel.GetFieldGroup(0, LOCATION_DECK, 0)
                      for i = 1, 2000 do g:Filter(function() return true end, nil) end";
        assert!(duel.run_lua_source("loop.lua", source));
        assert!(lua_count::finish(request), "the limit stopped it");
        assert!(duel.log().len() <= LOG_LINES, "the failures do not fill the log");
        // Outside a request nothing is counted against it.
        assert!(duel.run_lua_source("short.lua", "local n = 0 for i = 1, 100 do n = n + i end"));
        duel.reusable = false;
    }

    #[test]
    fn arena_saves_and_restores_memory() {
        assert_eq!(unsafe { df_arena_selftest() }, 1);
    }

    #[test]
    fn engine_api_version_is_linked() {
        assert_eq!(Engine::api_version().0, 11);
    }
}
