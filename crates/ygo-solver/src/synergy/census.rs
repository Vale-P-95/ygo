//! The census of the synergy plan (`docs/plans/sinergie-archetipi.md` §2),
//! from the engine's data instead of patterns over the scripts: the
//! archetypes of the pool and the cards they share, from the set codes of
//! the card database; the cards whose effects move cards and the Extra
//! Deck monsters with a summon procedure, from the effects the scripts
//! register ([`read_anatomy`]).

use std::collections::{BTreeMap, HashMap, HashSet};

use ygo_engine::consts::{category, effect_code};
use ygo_engine::{read_anatomy, EffectKind, Engine};

use super::read::{Offer, Pool};

/// `CATEGORY_TOGRAVE`: sends cards to the GY.
const TO_GRAVE: u64 = 0x20;
/// The categories of an effect that moves cards: search, add to the
/// hand, Special Summon, send to the GY.
const MOVES: u64 = category::SEARCH | category::TOHAND | category::SPECIAL_SUMMON | TO_GRAVE;

/// The archetype of a set code without its sub-archetype (the 4 high bits:
/// "Elemental HERO" is under "HERO").
pub fn base_archetype(setcode: u16) -> u16 {
    setcode & 0x0fff
}

/// The names of the archetypes, from `archetype_setcode_constants.lua` of
/// the scripts ("SET_ELEMENTAL_HERO" becomes "Elemental Hero"). Empty
/// when the scripts do not have it.
pub fn archetype_names(engine: &Engine) -> HashMap<u16, String> {
    let Some(text) = engine.scripts().path("archetype_setcode_constants.lua").and_then(|p| std::fs::read_to_string(p).ok()) else {
        return HashMap::new();
    };
    let mut names = HashMap::new();
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else { continue };
        let (name, value) = (name.trim(), value.split("--").next().unwrap_or("").trim());
        let (Some(name), Some(hex)) = (name.strip_prefix("SET_"), value.strip_prefix("0x")) else { continue };
        let Ok(code) = u16::from_str_radix(hex, 16) else { continue };
        let title: Vec<String> = name
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().chain(c.flat_map(char::to_lowercase)).collect()).unwrap_or_default()
            })
            .collect();
        // The first name of a code: the later ones are aliases.
        names.entry(code).or_insert_with(|| title.join(" "));
    }
    names
}

/// The numbers of the pool.
#[derive(Debug, Clone, Default)]
pub struct Census {
    pub main: usize,
    pub extra: usize,
    /// Cards with a set code, and with two or more base archetypes.
    pub with_archetype: usize,
    pub in_two: usize,
    /// Base archetypes with at least one card, and with 5, 10, 20 or more.
    pub archetypes: usize,
    pub at_least: [(usize, usize); 3],
    /// Pairs of base archetypes sharing a card, the most shared first.
    pub pairs: usize,
    pub top_pairs: Vec<(u16, u16, usize)>,
    /// Cards with an activated effect that searches, adds to the hand,
    /// Special Summons or sends to the GY.
    pub movers: usize,
    /// Extra Deck monsters with a summon procedure (materials or another
    /// way to summon them) among their effects.
    pub extra_with_procedure: usize,
    /// Cards whose script could not be set up.
    pub errors: usize,
}

pub fn census(engine: &Engine, pool: &Pool) -> Census {
    let db = engine.cards();
    let codes: Vec<u32> = pool.main.iter().chain(&pool.extra).copied().collect();
    let mut out = Census { main: pool.main.len(), extra: pool.extra.len(), ..Census::default() };
    let mut members: HashMap<u16, usize> = HashMap::new();
    let mut pairs: BTreeMap<(u16, u16), usize> = BTreeMap::new();
    for &code in &codes {
        let Some(card) = db.get(code) else { continue };
        let mut bases: Vec<u16> = card.setcodes.iter().map(|&s| base_archetype(s)).collect();
        bases.sort_unstable();
        bases.dedup();
        out.with_archetype += usize::from(!bases.is_empty());
        out.in_two += usize::from(bases.len() >= 2);
        for (i, &a) in bases.iter().enumerate() {
            *members.entry(a).or_default() += 1;
            for &b in &bases[i + 1..] {
                *pairs.entry((a, b)).or_default() += 1;
            }
        }
    }
    out.archetypes = members.len();
    out.at_least = [5, 10, 20].map(|n| (n, members.values().filter(|&&m| m >= n).count()));
    out.pairs = pairs.len();
    let mut top: Vec<(u16, u16, usize)> = pairs.into_iter().map(|((a, b), n)| (a, b, n)).collect();
    top.sort_by(|x, y| y.2.cmp(&x.2).then((x.0, x.1).cmp(&(y.0, y.1))));
    top.truncate(8);
    out.top_pairs = top;

    let extra: HashSet<u32> = pool.extra.iter().copied().collect();
    // Its materials or the way it is summoned, not only a limit on how.
    let summons = |e: &ygo_engine::EffectAnatomy| {
        e.kind == EffectKind::Procedure && matches!(e.code, effect_code::SPSUMMON_PROC | effect_code::FUSION_MATERIAL)
    };
    for card in read_anatomy(engine, &codes) {
        out.errors += usize::from(card.error.is_some());
        out.movers += usize::from(card.effects.iter().any(|e| e.kind.activated() && e.category & MOVES != 0));
        out.extra_with_procedure += usize::from(extra.contains(&card.code) && card.effects.iter().any(summons));
    }
    out
}

/// What the cards of an offer have in common: the base archetypes they
/// span, and whether all share an Attribute, a Type, a Level range or a
/// kind of card. A short reading for the summary; the rule read from the
/// whole list is the next phase's (F1).
#[derive(Debug, Clone, Default)]
pub struct OfferShape {
    pub archetypes: usize,
    /// Cards of no archetype.
    pub plain: usize,
    pub attribute: Option<u32>,
    pub race: Option<u64>,
    /// Lowest and highest Level (or Rank) of the monsters offered.
    pub levels: Option<(u32, u32)>,
    /// Monster, Spell or Trap, when all are the same.
    pub kind: Option<u32>,
}

impl OfferShape {
    pub fn of(engine: &Engine, offer: &Offer) -> Self {
        use ygo_engine::consts::card_type;
        let db = engine.cards();
        let cards: Vec<_> = offer.cards.iter().filter_map(|&c| db.get(c)).collect();
        let mut archetypes: HashSet<u16> = HashSet::new();
        let mut shape = OfferShape::default();
        for c in &cards {
            archetypes.extend(c.setcodes.iter().map(|&s| base_archetype(s)));
            shape.plain += usize::from(c.setcodes.is_empty());
        }
        shape.archetypes = archetypes.len();
        let same = |values: Vec<u64>| -> Option<u64> {
            let first = *values.first()?;
            values.iter().all(|&v| v == first).then_some(first)
        };
        let kinds = |c: &&ygo_engine::Card| u64::from(c.type_ & (card_type::MONSTER | card_type::SPELL | card_type::TRAP));
        shape.kind = same(cards.iter().map(kinds).collect()).map(|k| k as u32);
        let monsters: Vec<_> = cards.iter().filter(|c| c.type_ & card_type::MONSTER != 0).collect();
        if !monsters.is_empty() && monsters.len() == cards.len() {
            shape.attribute = same(monsters.iter().map(|c| u64::from(c.attribute)).collect()).map(|a| a as u32);
            shape.race = same(monsters.iter().map(|c| c.race).collect());
            let levels = monsters.iter().map(|c| c.level);
            shape.levels = levels.clone().min().zip(levels.max());
        }
        shape
    }

    /// "LIGHT Warrior Level 1-4", "Spell", "3 archetypes"...
    pub fn describe(&self) -> String {
        use ygo_engine::consts::card_type;
        let mut parts = Vec::new();
        if let Some(a) = self.attribute.filter(|a| a.is_power_of_two()) {
            parts.push(ygo_engine::attribute_name(a.trailing_zeros()));
        }
        if let Some(r) = self.race.filter(|r| r.is_power_of_two()) {
            parts.push(ygo_engine::race_name(r.trailing_zeros()));
        }
        if let Some((lo, hi)) = self.levels {
            parts.push(if lo == hi { format!("Level {lo}") } else { format!("Level {lo}-{hi}") });
        }
        match self.kind {
            Some(card_type::SPELL) => parts.push("Spell".to_owned()),
            Some(card_type::TRAP) => parts.push("Trap".to_owned()),
            Some(card_type::MONSTER) if parts.is_empty() => parts.push("monsters".to_owned()),
            _ => {}
        }
        parts.push(format!("{} archetypes, {} of none", self.archetypes, self.plain));
        parts.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_archetype_drops_the_sub_archetype() {
        // "Elemental HERO" (0x3008) is under "HERO" (0x8).
        assert_eq!(base_archetype(0x3008), 0x8);
        assert_eq!(base_archetype(0x8), 0x8);
    }
}
