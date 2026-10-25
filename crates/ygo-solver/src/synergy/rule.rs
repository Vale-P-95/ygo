//! A rule that people can read from the cards an effect accepts (plan
//! `docs/plans/sinergie-archetipi.md` §3.3 (a), "Riassunto leggibile"):
//! "LIGHT monster, Level 4 or lower" from the list the engine offered. The
//! rule only shows the bridge and tells why a card is accepted: the truth
//! stays the list.
//!
//! The rule is a conjunction of conditions on the columns of the card
//! database, learned greedily: every condition keeps all the accepted cards
//! (it is drawn from what they share), and each step takes the one that
//! leaves out the most cards of the universe not accepted, the way a
//! consistent conjunction is learned from positive and negative examples.
//! What the rule still lets in are its exceptions; a handful of accepted
//! cards with exceptions left become a rule by name.

use std::collections::HashMap;

use ygo_engine::consts::card_type;
use ygo_engine::{attribute_name, race_name, Card, CardDb};

/// Kinds of card a condition can ask for, beyond monster, Spell and Trap.
const FLAGS: [(u32, &str); 18] = [
    (0x1000, "Tuner"),
    (card_type::NORMAL, "Normal"),
    (0x20, "Effect"),
    (card_type::RITUAL, "Ritual"),
    (card_type::PENDULUM, "Pendulum"),
    (0x20_0000, "Flip"),
    (0x200, "Spirit"),
    (0x400, "Union"),
    (0x800, "Gemini"),
    (0x40_0000, "Toon"),
    (card_type::FUSION, "Fusion"),
    (card_type::SYNCHRO, "Synchro"),
    (card_type::XYZ, "Xyz"),
    (card_type::LINK, "Link"),
    (card_type::QUICKPLAY, "Quick-Play"),
    (card_type::CONTINUOUS, "Continuous"),
    (card_type::FIELD, "Field"),
    (card_type::EQUIP, "Equip"),
];

const KINDS: u32 = card_type::MONSTER | card_type::SPELL | card_type::TRAP;

/// One condition of a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// A card of the archetype (the engine's `IsSetCard`: the base
    /// archetype, and the sub-archetype when the set code has one).
    Archetype(u16),
    /// Monster, Spell or Trap: any of the kinds in the mask.
    Kind(u32),
    /// A monster of one of the Attributes in the mask.
    Attribute(u32),
    /// A monster of one of the Types in the mask.
    Race(u64),
    LevelAtMost(u32),
    LevelAtLeast(u32),
    /// A card of this kind (Tuner, Quick-Play...).
    Flag(u32),
    AttackAtMost(i32),
    AttackAtLeast(i32),
    DefenseAtMost(i32),
    DefenseAtLeast(i32),
    /// These cards and no other.
    Named(Vec<u32>),
}

fn is_monster(card: &Card) -> bool {
    card.type_ & card_type::MONSTER != 0
}

fn level(card: &Card) -> u32 {
    card.level & 0xff
}

/// The engine's `IsSetCard` for one set code.
pub fn in_archetype(card: &Card, setcode: u16) -> bool {
    card.setcodes.iter().any(|&s| (s & 0x0fff) == (setcode & 0x0fff) && (s & setcode) == setcode)
}

impl Condition {
    pub fn matches(&self, card: &Card) -> bool {
        let monster = is_monster(card);
        match self {
            Condition::Archetype(s) => in_archetype(card, *s),
            Condition::Kind(mask) => card.type_ & mask != 0,
            Condition::Attribute(mask) => monster && card.attribute & mask != 0,
            Condition::Race(mask) => monster && card.race & mask != 0,
            Condition::LevelAtMost(n) => monster && level(card) <= *n,
            Condition::LevelAtLeast(n) => monster && level(card) >= *n,
            Condition::Flag(f) => card.type_ & f != 0,
            Condition::AttackAtMost(n) => monster && card.attack >= 0 && card.attack <= *n,
            Condition::AttackAtLeast(n) => monster && card.attack >= *n,
            Condition::DefenseAtMost(n) => monster && card.defense >= 0 && card.defense <= *n,
            Condition::DefenseAtLeast(n) => monster && card.defense >= *n,
            Condition::Named(codes) => codes.binary_search(&card.code).is_ok(),
        }
    }

    /// The order conditions are shown in, and the preference between two
    /// that leave out as many cards.
    fn rank(&self) -> u8 {
        match self {
            Condition::Named(_) => 0,
            Condition::Archetype(_) => 1,
            Condition::Kind(_) => 2,
            Condition::Attribute(_) => 3,
            Condition::Race(_) => 4,
            Condition::LevelAtMost(_) | Condition::LevelAtLeast(_) => 5,
            Condition::Flag(_) => 6,
            _ => 7,
        }
    }

    fn describe(&self, db: &CardDb, archetypes: &HashMap<u16, String>) -> String {
        let bits = |mask: u64, name: &dyn Fn(u32) -> String| -> String {
            (0..64).filter(|b| mask & (1u64 << b) != 0).map(name).collect::<Vec<_>>().join(" or ")
        };
        match self {
            Condition::Archetype(s) => {
                let name = archetypes.get(s).or_else(|| archetypes.get(&(s & 0x0fff))).cloned().unwrap_or_else(|| format!("0x{s:x}"));
                format!("\"{name}\"")
            }
            Condition::Kind(mask) => bits(u64::from(*mask), &|b| {
                match 1u32 << b {
                    card_type::MONSTER => "monster",
                    card_type::SPELL => "Spell",
                    _ => "Trap",
                }
                .to_owned()
            }),
            Condition::Attribute(mask) => bits(u64::from(*mask), &|b| attribute_name(b).to_owned()),
            Condition::Race(mask) => bits(*mask, &|b| race_name(b).to_owned()),
            Condition::LevelAtMost(n) => format!("Level {n} or lower"),
            Condition::LevelAtLeast(n) => format!("Level {n} or higher"),
            Condition::Flag(f) => FLAGS.iter().find(|(x, _)| x == f).map_or("?", |(_, n)| n).to_owned(),
            Condition::AttackAtMost(n) => format!("ATK {n} or less"),
            Condition::AttackAtLeast(n) => format!("ATK {n} or more"),
            Condition::DefenseAtMost(n) => format!("DEF {n} or less"),
            Condition::DefenseAtLeast(n) => format!("DEF {n} or more"),
            Condition::Named(codes) => codes.iter().map(|&c| format!("\"{}\"", db.name(c))).collect::<Vec<_>>().join(" or "),
        }
    }
}

/// A rule read from the cards an effect accepts, among a universe.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rule {
    pub conditions: Vec<Condition>,
    /// Cards accepted, and cards of the universe the rule lets in that are
    /// not accepted.
    pub accepted: usize,
    pub exceptions: usize,
}

impl Rule {
    pub fn exact(&self) -> bool {
        self.exceptions == 0
    }

    pub fn matches(&self, card: &Card) -> bool {
        self.conditions.iter().all(|c| c.matches(card))
    }

    /// "LIGHT, Warrior, Level 4 or lower", with the exceptions when there
    /// are some; "any card" when every card of the universe is accepted.
    pub fn describe(&self, db: &CardDb, archetypes: &HashMap<u16, String>) -> String {
        let mut conditions: Vec<&Condition> = self.conditions.iter().collect();
        conditions.sort_by_key(|c| c.rank());
        let mut text = if conditions.is_empty() {
            "any card".to_owned()
        } else {
            conditions.iter().map(|c| c.describe(db, archetypes)).collect::<Vec<_>>().join(", ")
        };
        if self.exceptions > 0 {
            text.push_str(&format!(" (but {} such cards are not accepted)", self.exceptions));
        }
        text
    }
}

/// Rules by name are kept for this many accepted cards at most.
const NAMED_AT_MOST: usize = 5;
const MAX_CONDITIONS: usize = 6;

/// The rule for the cards `accepted` among `universe` (both canonical
/// codes; accepted cards out of the universe are left out). An empty
/// rule when nothing is accepted.
pub fn learn(db: &CardDb, accepted: &[u32], universe: &[u32]) -> Rule {
    let mut positive: Vec<&Card> = accepted.iter().filter_map(|&c| db.get(c)).collect();
    positive.sort_by_key(|c| c.code);
    positive.dedup_by_key(|c| c.code);
    let is_positive = |code: u32| positive.binary_search_by_key(&code, |c| c.code).is_ok();
    let mut negative: Vec<&Card> = universe.iter().filter(|&&c| !is_positive(c)).filter_map(|&c| db.get(c)).collect();
    let accepted = positive.len();
    if accepted == 0 {
        return Rule::default();
    }
    let mut conditions = Vec::new();
    while !negative.is_empty() && conditions.len() < MAX_CONDITIONS {
        let best = candidates(&positive)
            .into_iter()
            .filter(|c| !conditions.contains(c))
            .map(|c| (negative.iter().filter(|n| !c.matches(n)).count(), c))
            .filter(|(left_out, _)| *left_out > 0)
            .max_by(|(a, x), (b, y)| a.cmp(b).then(y.rank().cmp(&x.rank())));
        let Some((_, condition)) = best else { break };
        negative.retain(|n| condition.matches(n));
        conditions.push(condition);
    }
    // A condition that leaves out nothing the others let in says nothing.
    let mut k = 0;
    while k < conditions.len() && conditions.len() > 1 {
        let others: Vec<&Condition> = conditions.iter().enumerate().filter(|(i, _)| *i != k).map(|(_, c)| c).collect();
        let lets_in = universe
            .iter()
            .filter(|&&c| !is_positive(c))
            .filter_map(|&c| db.get(c))
            .filter(|card| others.iter().all(|c| c.matches(card)))
            .count();
        if lets_in == negative.len() {
            conditions.remove(k);
        } else {
            k += 1;
        }
    }
    if !negative.is_empty() && accepted <= NAMED_AT_MOST {
        return Rule { conditions: vec![Condition::Named(positive.iter().map(|c| c.code).collect())], accepted, exceptions: 0 };
    }
    Rule { conditions, accepted, exceptions: negative.len() }
}

/// The conditions that every accepted card meets.
fn candidates(positive: &[&Card]) -> Vec<Condition> {
    let mut out = Vec::new();
    let all = |f: &dyn Fn(&Card) -> bool| positive.iter().all(|c| f(c));
    // Archetypes: the set codes of the first card, and their base, that
    // every card has.
    let first = positive[0];
    let mut setcodes: Vec<u16> = first.setcodes.iter().flat_map(|&s| [s, s & 0x0fff]).collect();
    setcodes.sort_unstable();
    setcodes.dedup();
    for s in setcodes {
        if all(&|c| in_archetype(c, s)) {
            out.push(Condition::Archetype(s));
        }
    }
    let kinds = positive.iter().fold(0, |m, c| m | (c.type_ & KINDS));
    if kinds != KINDS {
        out.push(Condition::Kind(kinds));
    }
    for (flag, _) in FLAGS {
        if all(&|c| c.type_ & flag != 0) {
            out.push(Condition::Flag(flag));
        }
    }
    if all(&is_monster) {
        out.push(Condition::Attribute(positive.iter().fold(0, |m, c| m | c.attribute)));
        out.push(Condition::Race(positive.iter().fold(0, |m, c| m | c.race)));
        let levels = positive.iter().map(|c| level(c));
        out.push(Condition::LevelAtMost(levels.clone().max().unwrap_or(0)));
        out.push(Condition::LevelAtLeast(levels.min().unwrap_or(0)));
        if all(&|c| c.attack >= 0) {
            out.push(Condition::AttackAtMost(positive.iter().map(|c| c.attack).max().unwrap_or(0)));
            out.push(Condition::AttackAtLeast(positive.iter().map(|c| c.attack).min().unwrap_or(0)));
        }
        if all(&|c| c.defense >= 0 && c.type_ & card_type::LINK == 0) {
            out.push(Condition::DefenseAtMost(positive.iter().map(|c| c.defense).max().unwrap_or(0)));
            out.push(Condition::DefenseAtLeast(positive.iter().map(|c| c.defense).min().unwrap_or(0)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monster(code: u32, attribute: u32, race: u64, level: u32, setcodes: &[u16]) -> Card {
        Card {
            code,
            alias: 0,
            setcodes: setcodes.to_vec(),
            type_: card_type::MONSTER | 0x20,
            level,
            attribute,
            race,
            attack: 1000,
            defense: 1000,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
            name: format!("Card {code}"),
            desc: String::new(),
            strings: Vec::new(),
        }
    }

    fn db() -> CardDb {
        let mut cards = vec![
            // LIGHT (0x10) Warriors (0x1) of Level 4 or lower: what is accepted.
            monster(1, 0x10, 0x1, 4, &[]),
            monster(2, 0x10, 0x1, 3, &[0x3008]),
            monster(3, 0x10, 0x1, 1, &[]),
            // Others: a DARK Warrior, a LIGHT Warrior of Level 7, a LIGHT Fairy.
            monster(4, 0x20, 0x1, 4, &[]),
            monster(5, 0x10, 0x1, 7, &[0x8]),
            monster(6, 0x10, 0x4, 2, &[0x8]),
        ];
        let mut spell = monster(7, 0, 0, 0, &[]);
        spell.type_ = card_type::SPELL;
        cards.push(spell);
        CardDb::from_cards(cards)
    }

    #[test]
    fn learns_attribute_type_and_level() {
        let db = db();
        let rule = learn(&db, &[1, 2, 3], &[1, 2, 3, 4, 5, 6, 7]);
        assert!(rule.exact());
        assert_eq!(rule.accepted, 3);
        let text = rule.describe(&db, &HashMap::new());
        assert!(text.contains("LIGHT") && text.contains("Warrior") && text.contains("Level 4 or lower"), "{text}");
        for code in 1..=7 {
            assert_eq!(rule.matches(db.get(code).unwrap()), code <= 3, "card {code}");
        }
    }

    #[test]
    fn archetypes_follow_the_engine_and_few_cards_go_by_name() {
        let db = db();
        // 2 is an "Elemental HERO" (0x3008), so a "HERO" (0x8) too.
        assert!(in_archetype(db.get(2).unwrap(), 0x8));
        assert!(!in_archetype(db.get(5).unwrap(), 0x3008));
        let rule = learn(&db, &[2, 5, 6], &[1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(rule.conditions, vec![Condition::Archetype(0x8)]);
        // Nothing 1 and 6 share leaves out 2: by name.
        let named = learn(&db, &[1, 6], &[1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(named.conditions, vec![Condition::Named(vec![1, 6])]);
        assert!(learn(&db, &[], &[1, 2]).conditions.is_empty());
        // Everything accepted: no condition at all.
        assert!(learn(&db, &[1, 2, 3, 4, 5, 6, 7], &[1, 2, 3, 4, 5, 6, 7]).conditions.is_empty());
    }
}
