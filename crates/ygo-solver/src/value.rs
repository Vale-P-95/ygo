//! What a card is worth on the field at the end of your turn, whatever the
//! line that put it there: the probe turn is played with that card alone on
//! the field, and what it stops is its value (the interruptions it gives,
//! the opponent's plays it locks out). The search steers by these values,
//! so that it goes after boards that stop the opponent rather than boards
//! with many monsters that do nothing on the opponent's turn; the boards
//! it finds are then measured for real, together.
//!
//! The values are measured once per card and kind of zone (an Extra Deck
//! monster in the Extra Monster Zone and in a Main Monster Zone: Invoked
//! Oceanus banishes what goes to the GY only from the first) and kept for
//! the next searches with the same Deck. An Xyz Monster is put there with
//! two materials to detach. Handtraps are measured in the hand alone, with
//! your field empty and with a monster on it: some only work while you
//! control no cards (Infinite Impermanence, the Mulcharmy). One that cannot
//! even be used with your field is a dead card going first: discarding it
//! costs nothing, it is the card to discard.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use ygo_engine::consts::{card_type, location, position};
use ygo_engine::{DeckList, DuelState, Engine, Placement};

use crate::probe::{Kit, Prober};

/// Where a card is at the end of the turn, as far as its value goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spot {
    /// A face-up monster in a Main Monster Zone.
    Main,
    /// A face-up monster in an Extra Monster Zone.
    Extra,
    /// A face-down card in the Spell & Trap Zone.
    Set,
    /// A face-up Spell or Trap (Continuous, Field).
    FaceUp,
    /// In the hand, while you control no cards.
    Hand,
    /// In the hand, while you control a card.
    HandWithField,
}

/// Measured values of single cards on the field ([`Spot`] by [`Spot`]).
#[derive(Debug, Clone, Default)]
pub struct CardValues {
    values: HashMap<(u32, Spot), f64>,
    /// Handtraps the engine let you use from the hand there.
    usable: HashSet<(u32, Spot)>,
}

/// What a card in your hand is worth with a given field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InHand {
    /// Not a handtrap measured.
    Unknown,
    /// A handtrap that cannot be used with this field (the Mulcharmy while
    /// you control a card): a free discard.
    Dead,
    /// A handtrap that can be used, and what it stops.
    Usable(f64),
}

/// Weight of the floodgates in a card's value, next to its interruptions
/// (the same as in the default profile of the app).
const FLOODGATE_WEIGHT: f64 = 0.8;

impl CardValues {
    /// The value of `code` at `spot` (0 for a card never measured).
    pub fn get(&self, engine: &Engine, code: u32, spot: Spot) -> f64 {
        let code = engine.cards().canonical_code(code);
        self.values.get(&(code, spot)).copied().unwrap_or(0.0)
    }

    /// The sum of the values of your cards on the field in `state`.
    pub fn of_field(&self, engine: &Engine, state: &DuelState) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let me = &state.players[0];
        let monsters = me.monsters.iter().enumerate().filter_map(|(z, c)| {
            let c = c.as_ref().filter(|c| c.is_face_up())?;
            Some(self.get(engine, c.code, if z >= 5 { Spot::Extra } else { Spot::Main }))
        });
        let spells = me.spells.iter().flatten().map(|c| self.get(engine, c.code, if c.is_face_up() { Spot::FaceUp } else { Spot::Set }));
        monsters.chain(spells).sum()
    }

    /// What `code` is worth in your hand with the field of `state`.
    pub fn in_hand(&self, engine: &Engine, code: u32, state: &DuelState) -> InHand {
        let me = &state.players[0];
        let empty = me.monsters.iter().chain(&me.spells).all(Option::is_none);
        let key = (engine.cards().canonical_code(code), if empty { Spot::Hand } else { Spot::HandWithField });
        match self.values.get(&key) {
            None => InHand::Unknown,
            Some(_) if !self.usable.contains(&key) => InHand::Dead,
            Some(&v) => InHand::Usable(v),
        }
    }

    /// Whether the handtrap `code` can be used from the hand at `spot`.
    pub fn usable(&self, engine: &Engine, code: u32, spot: Spot) -> bool {
        self.usable.contains(&(engine.cards().canonical_code(code), spot))
    }

    /// Raises the value of `code` at `spot` to `value` (what it was seen to
    /// stop on a real board); true if it was lower.
    pub fn raise(&mut self, engine: &Engine, code: u32, spot: Spot, value: f64) -> bool {
        let entry = self.values.entry((engine.cards().canonical_code(code), spot)).or_insert(0.0);
        if value > *entry + 1e-9 {
            *entry = value;
            true
        } else {
            false
        }
    }

    /// The cards measured, best first (for diagnostics).
    pub fn list(&self) -> Vec<(u32, Spot, f64)> {
        let mut out: Vec<_> = self.values.iter().map(|(&(c, s), &v)| (c, s, v)).collect();
        out.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
        out
    }
}

/// The cards of `deck` that can stay on the field, and where.
fn spots(engine: &Engine, deck: &DeckList) -> Vec<(u32, Spot)> {
    let db = engine.cards();
    let mut out = Vec::new();
    for &code in deck.main.iter().chain(&deck.extra) {
        let code = db.canonical_code(code);
        let Some(card) = db.get(code) else { continue };
        let t = card.type_;
        let mut add = |spot| {
            if !out.contains(&(code, spot)) {
                out.push((code, spot));
            }
        };
        if t & card_type::MONSTER != 0 {
            add(Spot::Main);
            if t & card_type::EXTRA != 0 {
                add(Spot::Extra);
            }
        } else if t & card_type::TRAP != 0 || t & card_type::QUICKPLAY != 0 {
            add(Spot::Set);
        } else if t & (card_type::CONTINUOUS | card_type::FIELD) != 0 {
            add(Spot::FaceUp);
        }
    }
    out
}

/// Where `code` goes to be measured at `spot` (`filler`: a card that does
/// nothing, for Xyz Materials and for a field that is not empty).
fn placement(engine: &Engine, code: u32, spot: Spot, filler: u32) -> Vec<Placement> {
    let t = engine.cards().get(code).map_or(0, |c| c.type_);
    let at = |location, sequence, position| Placement { code, location, sequence, position, overlay: None };
    match spot {
        Spot::Main | Spot::Extra => {
            let mut p = at(location::MZONE, if spot == Spot::Main { 2 } else { 5 }, position::FACEUP_ATTACK);
            if t & card_type::XYZ != 0 {
                p.overlay = Some((filler, 2));
            }
            vec![p]
        }
        Spot::Set => vec![at(location::SZONE, 2, position::FACEDOWN_DEFENSE)],
        Spot::FaceUp if t & card_type::FIELD != 0 => vec![at(location::SZONE, 5, position::FACEUP_ATTACK)],
        Spot::FaceUp => vec![at(location::SZONE, 2, position::FACEUP_ATTACK)],
        Spot::Hand => Vec::new(),
        Spot::HandWithField => {
            vec![Placement { code: filler, location: location::MZONE, sequence: 2, position: position::FACEUP_ATTACK, overlay: None }]
        }
    }
}

/// Values already measured, by Deck (the probe turn uses the Deck: an
/// effect that Special Summons from it needs the cards to be there).
fn cache() -> &'static Mutex<HashMap<u64, CardValues>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, CardValues>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// Measures the cards of `deck` that can stay on the field, one at a time
/// (on `threads` threads), or takes them from the values already measured.
///
/// The hand during those turns holds a card of each kind that does
/// nothing on the opponent's turn (a Normal Monster, a Normal Spell, a
/// Normal Trap), so that an effect that sends a card of the same kind
/// (Invoked Mechaba) can pay for it.
pub fn card_values(engine: &Engine, deck: &DeckList, seed: u64, kit: &Kit, hand_traps: &[u32], threads: usize) -> CardValues {
    let mut key = std::collections::hash_map::DefaultHasher::new();
    (&deck.main, &deck.extra, seed, hand_traps).hash(&mut key);
    let key = key.finish();
    if let Some(found) = cache().lock().ok().and_then(|c| c.get(&key).cloned()) {
        return found;
    }
    let db = engine.cards();
    let hand: Vec<u32> =
        ["Alexandrite Dragon", "Pot of Greed", "Trap Hole"].iter().filter_map(|n| db.find_by_name(n)).map(|c| c.code).collect();
    let Some(&filler) = hand.first() else { return CardValues::default() };
    let mut with_hand = deck.clone();
    with_hand.main.extend(&hand);
    let mut todo = spots(engine, deck);
    for &code in hand_traps {
        let code = db.canonical_code(code);
        todo.extend([(code, Spot::Hand), (code, Spot::HandWithField)]);
    }
    let next = AtomicUsize::new(0);
    let found = Mutex::new(HashMap::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, todo.len().max(1)) {
            s.spawn(|| {
                let mut prober = Prober::new(engine, &with_hand, &hand, seed, kit);
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(code, spot)) = todo.get(i) else { break };
                    let placed = placement(engine, code, spot, filler);
                    let measured = if matches!(spot, Spot::Hand | Spot::HandWithField) {
                        // The handtrap alone in the hand (its own Deck copy).
                        let mine = [code];
                        Prober::new(engine, deck, &mine, seed, kit).measure_placed(&placed).map(|m| {
                            (m.hand_value, m.offered.iter().any(|&(c, loc)| db.canonical_code(c) == code && loc == location::HAND))
                        })
                    } else {
                        prober.measure_placed(&placed).map(|m| (m.field_value + FLOODGATE_WEIGHT * m.floodgate_value, true))
                    };
                    let (value, usable) = measured.unwrap_or((0.0, false));
                    if let Ok(mut f) = found.lock() {
                        f.insert((code, spot), (value, usable));
                    }
                }
            });
        }
    });
    let found: HashMap<(u32, Spot), (f64, bool)> = found.into_inner().unwrap_or_default();
    let values = CardValues {
        values: found.iter().map(|(&k, &(v, _))| (k, v)).collect(),
        usable: found.iter().filter(|(k, &(_, u))| u && matches!(k.1, Spot::Hand | Spot::HandWithField)).map(|(&k, _)| k).collect(),
    };
    if let Ok(mut c) = cache().lock() {
        c.insert(key, values.clone());
    }
    values
}
