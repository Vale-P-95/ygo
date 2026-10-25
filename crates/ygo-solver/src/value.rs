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
//! two materials to detach, another Extra Deck monster as Summoned with
//! materials that do nothing, banished ([`generic_materials`]: Apollousa's
//! ATK counts them), an Equip Spell on a monster that does nothing (when it
//! can be equipped to it). Handtraps are measured in the hand alone, with
//! your field empty and with a monster on it: some only work while you
//! control no cards (Infinite Impermanence, the Mulcharmy). One that cannot
//! even be used with your field is a dead card going first: discarding it
//! costs nothing, it is the card to discard.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use ygo_engine::consts::{card_type, location, position};
use ygo_engine::{generic_materials, DeckList, DuelState, Engine, FieldCard, Operation, Placement};

use crate::probe::{taken_weight, Effect, Kit, Prober};

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
    /// [`Spot::Main`] with your hand empty (some effects cost a card from
    /// the hand, or work only while it is empty).
    MainNoHand,
    /// [`Spot::Extra`] with your hand empty.
    ExtraNoHand,
    /// [`Spot::FaceUp`] with your hand empty.
    FaceUpNoHand,
    /// A face-down monster in a Main Monster Zone (a Flip monster).
    FaceDown,
    /// A Pendulum Scale.
    Pendulum,
    /// A second copy face-down in the Spell & Trap Zone, next to the first:
    /// what it adds (often nothing, for a card used once per turn).
    SetSecond,
}

impl Spot {
    /// Where your monster `c` in Monster Zone `zone` counts (`None`: a
    /// face-down Extra Deck monster, which almost never flips).
    pub(crate) fn of_monster(zone: usize, c: &FieldCard) -> Option<Spot> {
        if c.is_face_up() {
            Some(if zone >= 5 { Spot::Extra } else { Spot::Main })
        } else if c.type_ & card_type::EXTRA == 0 {
            Some(Spot::FaceDown)
        } else {
            None
        }
    }

    /// Where your card `c` in the Spell & Trap Zone counts.
    pub(crate) fn of_spell(c: &FieldCard) -> Spot {
        if c.type_ & card_type::MONSTER != 0 {
            Spot::Pendulum
        } else if c.is_face_up() {
            Spot::FaceUp
        } else {
            Spot::Set
        }
    }

    /// The same spot with your hand empty, for the spots measured both ways.
    pub(crate) fn without_hand(self) -> Option<Spot> {
        match self {
            Spot::Main => Some(Spot::MainNoHand),
            Spot::Extra => Some(Spot::ExtraNoHand),
            Spot::FaceUp => Some(Spot::FaceUpNoHand),
            _ => None,
        }
    }

    /// Where the card is put to be measured (the spot with a hand).
    fn on_field(self) -> Spot {
        match self {
            Spot::MainNoHand => Spot::Main,
            Spot::ExtraNoHand => Spot::Extra,
            Spot::FaceUpNoHand => Spot::FaceUp,
            other => other,
        }
    }
}

/// Measured values of single cards on the field ([`Spot`] by [`Spot`]).
#[derive(Debug, Clone, Default)]
pub struct CardValues {
    values: HashMap<(u32, Spot), f64>,
    /// Handtraps the engine let you use from the hand there.
    usable: HashSet<(u32, Spot)>,
    /// What monsters do arriving, effect by effect (`(code, description)`):
    /// their Summon triggers on the opponent's cards, as declared when they
    /// were activated in a search ([`arrival_value`]). Placed on the field
    /// to be measured, a monster does not arrive: Hyperinvoked Aeon alone
    /// stops nothing, Summoned it banishes 3 cards of the opponent's Extra
    /// Deck. The estimate counts it when the value measured is lower.
    arrivals: HashMap<(u32, u64), f64>,
    /// [`CardValues::arrivals`] summed by card.
    floors: HashMap<u32, f64>,
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
    /// The value of `code` at `spot` (0 for a card never measured); for a
    /// face-up monster, at least what it does arriving.
    pub fn get(&self, engine: &Engine, code: u32, spot: Spot) -> f64 {
        let code = engine.cards().canonical_code(code);
        let measured = self.values.get(&(code, spot)).copied().unwrap_or(0.0);
        match spot {
            Spot::Main | Spot::Extra | Spot::MainNoHand | Spot::ExtraNoHand => measured.max(self.floors.get(&code).copied().unwrap_or(0.0)),
            _ => measured,
        }
    }

    /// Raises what the effect `description` of `code` does arriving to
    /// `value`; true if it was lower.
    pub fn arrival(&mut self, engine: &Engine, code: u32, description: u64, value: f64) -> bool {
        let code = engine.cards().canonical_code(code);
        let entry = self.arrivals.entry((code, description)).or_insert(0.0);
        if value <= *entry + 1e-9 {
            return false;
        }
        *entry = value;
        let total = self.arrivals.iter().filter(|(k, _)| k.0 == code).map(|(_, v)| v).sum();
        self.floors.insert(code, total);
        true
    }

    /// Whether `value` is more than what is known of the effect
    /// `description` of `code` arriving.
    pub fn arrival_below(&self, engine: &Engine, code: u32, description: u64, value: f64) -> bool {
        let code = engine.cards().canonical_code(code);
        value > self.arrivals.get(&(code, description)).copied().unwrap_or(0.0) + 1e-9
    }

    /// The sum of the values of your cards on the field in `state` (with
    /// your hand empty, their values measured so when they were; a second
    /// copy Set, what it adds next to the first).
    pub fn of_field(&self, engine: &Engine, state: &DuelState) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let me = &state.players[0];
        let value = |code: u32, spot: Spot| match spot.without_hand() {
            Some(empty) if me.hand.is_empty() && self.measured(engine, code, empty) => self.get(engine, code, empty),
            _ => self.get(engine, code, spot),
        };
        let mut total = 0.0;
        for (z, c) in me.monsters.iter().enumerate() {
            if let Some(spot) = c.as_ref().and_then(|c| Spot::of_monster(z, c)) {
                total += value(c.as_ref().map_or(0, |c| c.code), spot);
            }
        }
        let mut set: Vec<u32> = Vec::new();
        for c in me.spells.iter().flatten() {
            let spot = Spot::of_spell(c);
            if spot == Spot::Set {
                let code = engine.cards().canonical_code(c.code);
                let again = set.contains(&code);
                set.push(code);
                if again && self.measured(engine, code, Spot::SetSecond) {
                    total += self.get(engine, code, Spot::SetSecond);
                    continue;
                }
            }
            total += value(c.code, spot);
        }
        total
    }

    /// What `code` is worth in your hand with the field of `state`.
    pub fn in_hand(&self, engine: &Engine, code: u32, state: &DuelState) -> InHand {
        let me = &state.players[0];
        let empty = me.monsters.iter().chain(&me.spells).all(Option::is_none);
        let key = (engine.cards().canonical_code(code), if empty { Spot::Hand } else { Spot::HandWithField });
        match self.values.get(&key) {
            None => InHand::Unknown,
            // Dead only when the field is what stops it (usable with an
            // empty field, not with cards): alone in the hand some
            // handtraps cannot pay their cost, which tells nothing.
            Some(_) if !self.usable.contains(&key) => {
                if key.1 == Spot::HandWithField && self.usable.contains(&(key.0, Spot::Hand)) {
                    InHand::Dead
                } else {
                    InHand::Unknown
                }
            }
            Some(&v) => InHand::Usable(v),
        }
    }

    /// Whether `code` was measured at `spot` (its value, 0 included, is known).
    pub fn measured(&self, engine: &Engine, code: u32, spot: Spot) -> bool {
        self.values.contains_key(&(engine.cards().canonical_code(code), spot))
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

    /// What each card does arriving, as far as known (for diagnostics).
    pub fn arrivals(&self) -> Vec<(u32, f64)> {
        let mut out: Vec<_> = self.floors.iter().map(|(&c, &v)| (c, v)).collect();
        out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// The cards measured, best first (for diagnostics).
    pub fn list(&self) -> Vec<(u32, Spot, f64)> {
        let mut out: Vec<_> = self.values.iter().map(|(&(c, s), &v)| (c, s, v)).collect();
        out.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
        out
    }
}

/// The cards of `deck` that can stay on the field, and where (with your
/// hand and without it).
fn spots(engine: &Engine, deck: &DeckList) -> Vec<(u32, Spot)> {
    let db = engine.cards();
    let mut out = Vec::new();
    for &code in deck.main.iter().chain(&deck.extra) {
        let code = db.canonical_code(code);
        let Some(card) = db.get(code) else { continue };
        let t = card.type_;
        let mut add = |spot: Spot| {
            for spot in std::iter::once(spot).chain(spot.without_hand()) {
                if !out.contains(&(code, spot)) {
                    out.push((code, spot));
                }
            }
        };
        if t & card_type::MONSTER != 0 {
            add(Spot::Main);
            if t & card_type::EXTRA != 0 {
                add(Spot::Extra);
            } else {
                add(Spot::FaceDown);
            }
            if t & card_type::PENDULUM != 0 {
                add(Spot::Pendulum);
            }
        } else if t & card_type::TRAP != 0 || t & card_type::QUICKPLAY != 0 {
            add(Spot::Set);
            if deck.main.iter().filter(|&&c| db.canonical_code(c) == code).count() >= 2 {
                add(Spot::SetSecond);
            }
            // A Continuous Trap stays face-up once activated.
            if t & card_type::TRAP != 0 && t & card_type::CONTINUOUS != 0 {
                add(Spot::FaceUp);
            }
        } else if t & (card_type::CONTINUOUS | card_type::FIELD | card_type::EQUIP) != 0 {
            add(Spot::FaceUp);
        }
    }
    out
}

/// Where `code` goes to be measured at `spot` (`filler`: a card that does
/// nothing, for Xyz Materials and for a field that is not empty).
fn placement(engine: &Engine, code: u32, spot: Spot, filler: u32) -> Vec<Placement> {
    let t = engine.cards().get(code).map_or(0, |c| c.type_);
    let at = |location, sequence, position| Placement::new(code, location, sequence, position);
    match spot {
        // Put there the same way; only the prober's hand differs.
        Spot::MainNoHand | Spot::ExtraNoHand | Spot::FaceUpNoHand => placement(engine, code, spot.on_field(), filler),
        Spot::Main | Spot::Extra => {
            let mut p = at(location::MZONE, if spot == Spot::Main { 2 } else { 5 }, position::FACEUP_ATTACK);
            if t & card_type::XYZ != 0 {
                p.overlay = vec![filler; 2];
                return vec![p];
            }
            // Banished, so that the GY stays empty as for the other cards:
            // in the GY they would be targets for the opponent's plays.
            p.materials = generic_materials(engine, code);
            let mut out: Vec<Placement> = p
                .materials
                .iter()
                .enumerate()
                .map(|(i, &m)| Placement::new(m, location::REMOVED, i as u32, position::FACEUP_ATTACK))
                .collect();
            out.push(p);
            out
        }
        Spot::FaceDown => vec![at(location::MZONE, 2, position::FACEDOWN_DEFENSE)],
        // A Pendulum Monster in Spell & Trap Zone 0 is placed as a Scale.
        Spot::Pendulum => vec![at(location::SZONE, 0, position::FACEUP_ATTACK)],
        Spot::Set => vec![at(location::SZONE, 2, position::FACEDOWN_DEFENSE)],
        Spot::SetSecond => vec![at(location::SZONE, 1, position::FACEDOWN_DEFENSE), at(location::SZONE, 2, position::FACEDOWN_DEFENSE)],
        Spot::FaceUp if t & card_type::FIELD != 0 => vec![at(location::SZONE, 5, position::FACEUP_ATTACK)],
        Spot::FaceUp if t & card_type::EQUIP != 0 => {
            vec![Placement::new(filler, location::MZONE, 2, position::FACEUP_ATTACK), at(location::SZONE, 2, position::FACEUP_ATTACK)]
        }
        Spot::FaceUp => vec![at(location::SZONE, 2, position::FACEUP_ATTACK)],
        Spot::Hand => Vec::new(),
        // A field and a GY as after a combo (Dimension Shifter needs an empty GY).
        Spot::HandWithField => vec![
            Placement::new(filler, location::MZONE, 2, position::FACEUP_ATTACK),
            Placement::new(filler, location::GRAVE, 0, position::FACEUP_ATTACK),
        ],
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
                let mut no_hand = Prober::new(engine, deck, &[], seed, kit);
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(code, spot)) = todo.get(i) else { break };
                    let placed = placement(engine, code, spot, filler);
                    let measured = if matches!(spot, Spot::Hand | Spot::HandWithField) {
                        // The handtrap alone in the hand (its own Deck copy).
                        let mine = [code];
                        Prober::new(engine, deck, &mine, seed, kit).measure_placed_unbroken(&placed).map(|m| {
                            (m.hand_value, m.offered.iter().any(|&(c, loc)| db.canonical_code(c) == code && loc == location::HAND))
                        })
                    } else {
                        let prober = if spot.on_field() == spot { &mut prober } else { &mut no_hand };
                        prober.measure_placed_unbroken(&placed).map(|m| (m.field_value + FLOODGATE_WEIGHT * m.floodgate_value, true))
                    };
                    // A turn that could not be played tells nothing: left unmeasured.
                    let Ok((value, usable)) = measured else { continue };
                    if let Ok(mut f) = found.lock() {
                        f.insert((code, spot), (value, usable));
                    }
                }
            });
        }
    });
    let mut found: HashMap<(u32, Spot), (f64, bool)> = found.into_inner().unwrap_or_default();
    // Two copies were measured together: the second is worth what they add
    // to the first alone.
    let pairs: Vec<(u32, f64)> = found.iter().filter(|(k, _)| k.1 == Spot::SetSecond).map(|(k, v)| (k.0, v.0)).collect();
    for (code, both) in pairs {
        match found.get(&(code, Spot::Set)) {
            Some(&(one, _)) => found.insert((code, Spot::SetSecond), ((both - one).max(0.0), true)),
            None => found.remove(&(code, Spot::SetSecond)),
        };
    }
    let values = CardValues {
        values: found.iter().map(|(&k, &(v, _))| (k, v)).collect(),
        usable: found.iter().filter(|(k, &(_, u))| u && matches!(k.1, Spot::Hand | Spot::HandWithField)).map(|(&k, _)| k).collect(),
        ..CardValues::default()
    };
    if let Ok(mut c) = cache().lock() {
        c.insert(key, values.clone());
    }
    values
}

/// What a declaration of a Summon trigger is worth to the estimate, in
/// interruptions (`None`: not one on the opponent's cards that the weights
/// know). With the weights that exist, no new ones (§4.4 of
/// `docs/plans/anatomia-effetti.md`): how well the category takes a card
/// away ([`Effect::value`]), times the cards, times what one card of the
/// opponent is worth where it is ([`taken_weight`]: the least, for a
/// declaration of more places), times 0.85 when it targets, as for the
/// interruptions of the probe turn.
pub fn arrival_value(op: &Operation) -> Option<f64> {
    if !op.on_arrival() || op.count == 0 {
        return None;
    }
    let effect = match op.category {
        0x1 => Effect::Destroy,
        0x4 => Effect::Banish,
        0x8 => Effect::Bounce,
        0x10 => Effect::ToDeck,
        // Tributed, sent, milled or discarded.
        0x2 | 0x20 | 0x40 | 0x80 => Effect::ToGrave,
        0x1000 | 0x2000 => Effect::Position,
        0x4000 => Effect::NegateEffect,
        _ => return None,
    };
    let weight = [location::EXTRA, location::HAND, location::DECK, location::MZONE, location::SZONE]
        .into_iter()
        .filter(|&loc| op.location & loc as u32 != 0)
        .map(taken_weight)
        .min_by(f64::total_cmp)?;
    Some(effect.value() * op.count as f64 * weight * if op.targets { 0.85 } else { 1.0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ygo_engine::consts::{effect_type, event};

    fn op(event: u32, category: u64, count: u32, location: u32, targets: bool) -> Operation {
        Operation {
            code: 33166263,
            description: 0,
            effect_type: effect_type::SINGLE | effect_type::TRIGGER_O,
            event,
            category,
            count,
            location,
            targets,
        }
    }

    #[test]
    fn arrivals_are_worth_what_the_probe_turn_weighs() {
        // Hyperinvoked Aeon: 3 cards of the opponent's Extra Deck banished.
        let aeon = arrival_value(&op(event::SPSUMMON_SUCCESS, 0x4, 3, location::EXTRA as u32, false)).unwrap();
        assert!((aeon - Effect::Banish.value() * 3.0 * taken_weight(location::EXTRA)).abs() < 1e-9);
        // A targeted destruction on the field, from the hand or the field: the least.
        let field = arrival_value(&op(event::SUMMON_SUCCESS, 0x1, 1, (location::MZONE | location::HAND) as u32, true)).unwrap();
        assert!((field - Effect::Destroy.value() * taken_weight(location::HAND) * 0.85).abs() < 1e-9);
        // Not a trigger of its own Summon, or a category the weights do not know.
        assert!(arrival_value(&op(event::DESTROYED, 0x4, 3, location::EXTRA as u32, false)).is_none());
        assert!(arrival_value(&op(event::SPSUMMON_SUCCESS, 0x10000, 1, location::HAND as u32, false)).is_none());
    }
}
