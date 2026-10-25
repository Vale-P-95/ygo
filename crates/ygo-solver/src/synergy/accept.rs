//! What a card accepts, put together from its readings (hand, field, GY):
//! for each of its effects that picks cards, the cards offered where they
//! were and the rule read from them; the effects that move cards and that
//! no reading played, marked "from the script, not tried" with what the
//! script's filters look at. And how much of the pool the readings cover.

use std::collections::{BTreeMap, HashSet};

use ygo_engine::consts::{category, location};
use ygo_engine::{CardAnatomy, CardDb, EffectAnatomy};

use super::read::{Offer, Place, Pool, Reading};
use super::rule::{learn, Rule};

/// `CATEGORY_TOGRAVE`.
const TO_GRAVE: u64 = 0x20;
/// An effect that moves cards: searches, adds to the hand, Special Summons
/// or sends to the GY (the census's "movers").
pub const MOVES: u64 = category::SEARCH | category::TOHAND | category::SPECIAL_SUMMON | TO_GRAVE;

/// The effects of a card that move cards, the ones a reading should play.
pub fn moving_effects(anatomy: &CardAnatomy) -> Vec<&EffectAnatomy> {
    anatomy.effects.iter().filter(|e| e.kind.activated() && e.category & MOVES != 0).collect()
}

/// For each effect of `moving` (as [`moving_effects`] gives them), whether
/// one of the activations seen is that effect.
/// Effects that share a description (the same effect on two events, "If
/// this card is Normal or Special Summoned") count as played together.
pub fn played(anatomy: &CardAnatomy, moving: &[&EffectAnatomy], activated: &HashSet<u64>) -> Vec<bool> {
    let hit: Vec<&EffectAnatomy> = activated.iter().filter_map(|&d| anatomy.effect_of(d)).collect();
    moving
        .iter()
        .map(|e| hit.iter().any(|h| std::ptr::eq(*h, *e) || (e.description >> 20 != 0 && h.description == e.description)))
        .collect()
}

/// Whether a card still needs reading from the field and from the GY,
/// after the readings that played `activated`: an effect that moves cards
/// and was not played works from the field (or is the activation of a
/// Spell or Trap, which a Set card may manage with the GY filled, or a
/// trigger of something that happens there), or from the GY. An Extra Deck monster is read from the field whenever it has
/// such an effect.
pub fn wanted(anatomy: &CardAnatomy, extra: bool, activated: &HashSet<u64>) -> (bool, bool) {
    let moving = moving_effects(anatomy);
    let done = played(anatomy, &moving, activated);
    let field_zones = u32::from(location::MZONE | location::SZONE) | 0x300;
    let (mut field, mut grave) = (extra && !moving.is_empty(), false);
    for (e, d) in moving.iter().zip(done) {
        if d {
            continue;
        }
        // Triggers when it is sent to the GY from the field, leaves it, is
        // destroyed or Tributed there, flipped; when it is added to the hand
        // from the GY.
        let trigger = matches!(e.kind, ygo_engine::EffectKind::Trigger { .. } | ygo_engine::EffectKind::Flip);
        field |= e.range & field_zones != 0
            || e.kind == ygo_engine::EffectKind::Activation
            || e.kind == ygo_engine::EffectKind::Flip
            || (trigger && matches!(e.code, 1014 | 1015 | 1029 | 1017));
        grave |= e.range & u32::from(location::GRAVE) != 0 || (trigger && e.code == 1012);
    }
    (field, grave)
}

/// Whether the activation of a Spell or Trap is an effect that moves cards
/// not played yet: then it is read from the field Set ([`super::Scene::set`]).
pub fn activation_wanted(anatomy: &CardAnatomy, activated: &HashSet<u64>) -> bool {
    let moving = moving_effects(anatomy);
    moving.iter().zip(played(anatomy, &moving, activated)).any(|(e, d)| !d && e.kind == ygo_engine::EffectKind::Activation)
}

/// The cards a choice from `location` could have offered: the Deck holds
/// the Main Deck pool, the Extra Deck the Extra Deck pool, the GY the
/// sample `grave` ([`Pool::grave_sample`]). `None` where the reading deck
/// puts nothing (the hand, the field, the banished cards).
pub fn universe(pool: &Pool, location: u8, grave: &[u32]) -> Option<Vec<u32>> {
    match location {
        location::DECK => Some(pool.main.clone()),
        location::EXTRA => Some(pool.extra.clone()),
        location::GRAVE if !grave.is_empty() => Some(grave.to_vec()),
        _ => None,
    }
}

/// One choice of an effect: where it was read, what it offered and the
/// rule read from it (none where there is no universe).
#[derive(Debug, Clone)]
pub struct Accepted {
    pub place: Place,
    pub offer: Offer,
    pub rule: Option<Rule>,
}

/// What a card accepts, effect by effect.
#[derive(Debug, Clone, Default)]
pub struct Accepts {
    pub code: u32,
    /// By the description of the effect that asked.
    pub effects: BTreeMap<u64, Vec<Accepted>>,
    /// Effects that move cards that no reading played: the description,
    /// and what the script's filters look at.
    pub untried: Vec<(u64, Vec<&'static str>)>,
}

/// Puts together the readings of a card. `grave` is the GY of the readings
/// from the field and the GY, `script` the card's Lua source, for the
/// effects no reading played.
pub fn accepts(
    db: &CardDb,
    pool: &Pool,
    grave: &[u32],
    anatomy: &CardAnatomy,
    readings: &[(Place, &Reading)],
    script: Option<&str>,
) -> Accepts {
    let mut out = Accepts { code: anatomy.code, ..Accepts::default() };
    let mut activated = HashSet::new();
    for (place, r) in readings {
        activated.extend(r.activated.iter().copied());
        for o in &r.offers {
            let rule = universe(pool, o.location, grave).map(|u| learn(db, &o.cards, &u));
            out.effects.entry(o.effect).or_default().push(Accepted { place: *place, offer: o.clone(), rule });
        }
    }
    let moving = moving_effects(anatomy);
    let hints = script.map(script_filters).unwrap_or_default();
    for (e, done) in moving.iter().zip(played(anatomy, &moving, &activated)) {
        if !done {
            out.untried.push((e.description, hints.clone()));
        }
    }
    out
}

/// What the filters of a script look at, by the card functions they call:
/// the safety net of the plan (§3.3 (a)) for the effects no reading played.
/// A filter by property says nothing of the archetype or name.
pub fn script_filters(script: &str) -> Vec<&'static str> {
    const CALLS: [(&str, &str); 7] = [
        ("IsAttribute", "Attribute"),
        ("IsRace", "Type"),
        ("IsLevel", "Level"),
        ("IsRank", "Rank"),
        ("IsType", "kind of card"),
        ("IsSetCard", "archetype"),
        ("IsCode", "name"),
    ];
    CALLS.iter().filter(|(call, _)| script.contains(call)).map(|(_, what)| *what).collect()
}

/// How much of a set of cards the readings cover: effects that move cards,
/// by kind, and how many of them a reading played.
#[derive(Debug, Clone, Default)]
pub struct Coverage {
    /// Kind of effect → (effects that move cards, played).
    pub by_kind: BTreeMap<&'static str, (usize, usize)>,
    /// Cards with an effect that moves cards, and those with all played.
    pub cards: usize,
    pub cards_all: usize,
    /// Effects not played, by what they ask before anything (a cost, a
    /// condition, a target), for the summary.
    pub missed_with: BTreeMap<&'static str, usize>,
}

impl Coverage {
    pub fn add(&mut self, anatomy: &CardAnatomy, activated: &HashSet<u64>) {
        let moving = moving_effects(anatomy);
        if moving.is_empty() {
            return;
        }
        let done = played(anatomy, &moving, activated);
        self.cards += 1;
        self.cards_all += usize::from(done.iter().all(|&d| d));
        for (e, d) in moving.iter().zip(done) {
            let entry = self.by_kind.entry(e.kind.name()).or_default();
            entry.0 += 1;
            entry.1 += usize::from(d);
            if !d {
                let why = match () {
                    _ if e.has_condition => "a condition",
                    _ if e.has_cost => "a cost",
                    _ if e.targets() => "a target",
                    _ => "nothing declared",
                };
                *self.missed_with.entry(why).or_default() += 1;
            }
        }
    }

    pub fn effects(&self) -> (usize, usize) {
        self.by_kind.values().fold((0, 0), |(a, b), (x, y)| (a + x, b + y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_filters_name_what_they_look_at() {
        let script = "function s.filter(c) return c:IsAttribute(ATTRIBUTE_LIGHT) and c:IsLevelBelow(4) and c:IsAbleToHand() end";
        assert_eq!(script_filters(script), vec!["Attribute", "Level"]);
        assert!(script_filters("function s.op() end").is_empty());
    }
}
