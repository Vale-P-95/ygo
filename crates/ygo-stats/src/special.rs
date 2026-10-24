//! Cards that change what an opening hand reaches: draw spells (Pot of
//! Desires), excavators (Pot of Prosperity), draws with a cost (Allure of
//! Darkness), bridges between monsters (Small World) and searches (Seventh
//! Tachyon, Reinforcement of the Army).
//!
//! The calculations know general mechanisms, never card names: which card
//! has which mechanism is data, in `special.json`. Every probability is
//! exact, enumerated over groups of cards like `exact::probability`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::cond::Target;
use crate::exact::{binom, p_none, DeckModel, ExactError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Mechanism {
    /// Banishes `banish_top` cards face-down from the top of the Deck, then
    /// draws `n`. Needs `min_extra` cards in the Extra Deck to pay its cost.
    #[serde(rename_all = "camelCase")]
    Draw {
        n: u32,
        #[serde(default)]
        banish_top: u32,
        #[serde(default)]
        min_extra: u32,
    },
    /// Looks at the top cards (as many as one of `n`, the player's choice)
    /// and adds one. With `banish_extra` it first banishes as many cards
    /// from the Extra Deck.
    #[serde(rename_all = "camelCase")]
    Excavate {
        n: Vec<u32>,
        #[serde(default)]
        banish_extra: bool,
    },
    /// Draws `n`, then banishes a card matching `cost` from the hand;
    /// without one the whole hand is lost.
    DrawThenBanish { n: u32, cost: Filter },
    /// Reveals a monster in hand, banishes from the Deck a monster that
    /// shares exactly `share` properties with it, then adds a monster that
    /// shares exactly `share` properties with the banished one.
    Bridge { share: u32 },
    /// Adds a card matching `filter` from the Deck.
    Search { filter: Filter },
    /// Reveals an Extra Deck monster whose name starts with one of
    /// `reveal`, then adds a Deck monster whose Level is its Rank and that
    /// has its Type or its Attribute; puts `put_back` cards from the hand
    /// on top of the Deck.
    #[serde(rename_all = "camelCase")]
    RevealSearch {
        reveal: Vec<String>,
        #[serde(default)]
        put_back: u32,
    },
}

/// Which cards a search finds or a cost takes. Empty fields match anything.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    /// `monster`, `spell` or `trap`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Monster Type, or Spell/Trap subtype (`Field`, `Quick-Play`…).
    #[serde(default)]
    pub race: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub max_level: Option<u8>,
}

impl Filter {
    pub fn matches(&self, card: &Facts) -> bool {
        self.kind.as_ref().is_none_or(|k| *k == card.kind)
            && self.race.as_ref().is_none_or(|r| card.race.as_ref() == Some(r))
            && self.attribute.as_ref().is_none_or(|a| card.attribute.as_ref() == Some(a))
            && self.max_level.is_none_or(|max| card.level.is_some_and(|l| l <= max))
    }
}

/// A card mapped to its mechanism, by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Known {
    pub name: String,
    pub mechanism: Mechanism,
}

/// The cards the lab knows, from `special.json`.
pub fn known() -> &'static [Known] {
    static KNOWN: OnceLock<Vec<Known>> = OnceLock::new();
    KNOWN.get_or_init(|| serde_json::from_str(include_str!("special.json")).expect("special.json is valid"))
}

/// The mechanism of a card, if the lab knows it.
pub fn mechanism_of(name: &str) -> Option<&'static Mechanism> {
    known().iter().find(|k| k.name == name).map(|k| &k.mechanism)
}

/// What the calculations read of a card. A bridge compares Attribute,
/// Type, Level, ATK and DEF; a `None` (a `?` ATK, say) matches nothing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub id: u32,
    pub name: String,
    /// `monster`, `spell` or `trap`.
    pub kind: String,
    pub race: Option<String>,
    pub attribute: Option<String>,
    /// Level, or Rank.
    pub level: Option<u8>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
}

impl Facts {
    pub fn is_monster(&self) -> bool {
        self.kind == "monster"
    }

    /// How many properties the two monsters have in common.
    pub fn shared(&self, other: &Facts) -> u32 {
        fn same<T: PartialEq>(a: &Option<T>, b: &Option<T>) -> u32 {
            u32::from(a.is_some() && a == b)
        }
        same(&self.attribute, &other.attribute)
            + same(&self.race, &other.race)
            + same(&self.level, &other.level)
            + same(&self.atk, &other.atk)
            + same(&self.def, &other.def)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialInput {
    pub deck: DeckModel,
    pub hand: u32,
    /// The card or role the hand wants.
    pub target: Target,
    /// Every card of the Main Deck, once.
    pub cards: Vec<Facts>,
    /// The Extra Deck, one entry per copy.
    pub extra: Vec<Facts>,
    /// The cards to calculate, by passcode, with their mechanism.
    pub special: Vec<(u32, Mechanism)>,
}

/// A path of a bridge: `from` in hand, `via` banished from the Deck, `to` added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chain {
    pub from: u32,
    pub via: u32,
    pub to: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Variant {
    pub n: u32,
    pub with: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialResult {
    pub id: u32,
    pub copies: u32,
    pub mechanism: Mechanism,
    /// Chance to open the card.
    pub open: f64,
    /// Chance to have the target in hand, or to reach it with this card.
    pub with: f64,
    /// For a choice of how many cards to excavate: `with` for each.
    pub variants: Vec<Variant>,
    /// With the card and without the target in hand: chance that its cost
    /// banishes at least one copy of the target (Draw with `banish_top`).
    pub banish_risk: Option<f64>,
    /// With the card in hand: chance that its cost cannot be paid and the
    /// hand is lost (DrawThenBanish).
    pub lose_hand: Option<f64>,
    /// Copies in the Main Deck that pay the cost (DrawThenBanish).
    pub payers: Option<u32>,
    /// Why the card cannot resolve in this deck, in Italian.
    pub blocked: Option<String>,
    /// Every bridge path between Main Deck monsters (Bridge).
    pub chains: Vec<Chain>,
    /// Cards the card adds from the Deck (Search, RevealSearch), or that
    /// reach the target through a bridge from the hand (Bridge).
    pub links: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialReport {
    /// Chance to open the target, without any special card.
    pub base: f64,
    pub target_copies: u32,
    pub size: u32,
    pub cards: Vec<SpecialResult>,
}

const SELF: u8 = 1;
const TARGET: u8 = 2;
/// Pays a DrawThenBanish cost, or opens a bridge to the target.
const MARK: u8 = 4;

/// Above this many paths a deck's bridges are cut short (a Main Deck of
/// monsters all alike would give tens of thousands).
const MAX_CHAINS: usize = 5000;

pub fn special(input: &SpecialInput) -> Result<SpecialReport, ExactError> {
    let deck = &input.deck;
    let size = deck.size();
    let hand = input.hand;
    if hand > size {
        return Err(ExactError::HandTooLarge);
    }
    let roles = |id: u32| deck.cards.iter().find(|c| c.id == id).map_or(&[][..], |c| &c.roles[..]);
    let hits = |id: u32| match &input.target {
        Target::Card(t) => id == *t,
        Target::Role(r) => roles(id).contains(r),
    };
    let target_copies: u32 = deck.cards.iter().filter(|c| hits(c.id)).map(|c| c.copies).sum();
    let base = 1.0 - p_none(size, target_copies, hand);
    let facts: Vec<&Facts> = input.cards.iter().filter(|f| deck.cards.iter().any(|c| c.id == f.id)).collect();
    let monsters: Vec<&Facts> = facts.iter().copied().filter(|f| f.is_monster()).collect();
    // The deck left after an opening hand without the target.
    let rest = size - hand;
    let draw_hit = |n: u32| 1.0 - p_none(rest, target_copies, n.min(rest));
    let no_mark = |_: u32| false;
    // A search finds the target when one of its cards is in `links`: all
    // its copies are in the Deck when the hand has none.
    let found = |links: &[u32]| f64::from(links.iter().any(|&id| hits(id)));

    let mut cards = Vec::new();
    for (id, mechanism) in &input.special {
        let Some(entry) = deck.cards.iter().find(|c| c.id == *id) else { continue };
        let mut result = SpecialResult {
            id: *id,
            copies: entry.copies,
            mechanism: mechanism.clone(),
            open: 1.0 - p_none(size, entry.copies, hand),
            with: base,
            variants: Vec::new(),
            banish_risk: None,
            lose_hand: None,
            payers: None,
            blocked: None,
            chains: Vec::new(),
            links: Vec::new(),
        };
        let reach = |mark: &dyn Fn(u32) -> bool, then: &dyn Fn(&[Group]) -> f64| reach_chance(deck, *id, &hits, mark, hand, then);
        match mechanism {
            Mechanism::Draw { n, banish_top, min_extra } => {
                if (input.extra.len() as u32) < *min_extra {
                    result.blocked = Some(format!("servono almeno {min_extra} carte nell'Extra Deck"));
                } else {
                    result.with = reach(&no_mark, &|_| draw_hit(*n));
                }
                if *banish_top > 0 {
                    result.banish_risk = Some(1.0 - p_none(rest, target_copies, (*banish_top).min(rest)));
                }
            }
            Mechanism::Excavate { n, banish_extra } => {
                for &k in n {
                    if !*banish_extra || input.extra.len() as u32 >= k {
                        result.variants.push(Variant { n: k, with: reach(&no_mark, &|_| draw_hit(k)) });
                    }
                }
                match result.variants.iter().map(|v| v.with).reduce(f64::max) {
                    Some(best) => result.with = best,
                    None => result.blocked = Some(format!("servono almeno {} carte nell'Extra Deck", n.iter().min().unwrap_or(&0))),
                }
            }
            Mechanism::DrawThenBanish { n, cost } => {
                let pays = |c: u32| c != *id && facts.iter().any(|f| f.id == c && cost.matches(f));
                result.payers = Some(deck.cards.iter().filter(|c| pays(c.id)).map(|c| c.copies).sum());
                result.with = reach(&pays, &|held| draw_then_banish(held, *n, rest));
                if result.open > 0.0 {
                    result.lose_hand = Some(lose_hand(deck, *id, hand, *n, &pays) / result.open);
                }
            }
            Mechanism::Bridge { share } => {
                let chains = bridges(&monsters, *share);
                let links: Vec<u32> =
                    monsters.iter().map(|m| m.id).filter(|&a| !hits(a) && chains.iter().any(|c| c.from == a && hits(c.to))).collect();
                // A linked monster in hand is enough: the target is still in the Deck.
                result.with = reach(&|c| links.contains(&c), &|held| f64::from(held.iter().any(|g| g.flags & MARK != 0 && g.held > 0)));
                result.chains = chains.into_iter().take(MAX_CHAINS).collect();
                result.links = links;
            }
            Mechanism::Search { filter } => {
                result.links = facts.iter().filter(|f| f.id != *id && filter.matches(f)).map(|f| f.id).collect();
                let hit = found(&result.links);
                result.with = reach(&no_mark, &|_| hit);
            }
            Mechanism::RevealSearch { reveal, .. } => {
                let shown: Vec<&Facts> = input.extra.iter().filter(|x| reveal.iter().any(|r| x.name.starts_with(r.as_str()))).collect();
                if shown.is_empty() {
                    result.blocked = Some("nell'Extra Deck non c'è un mostro da rivelare".to_owned());
                }
                result.links = monsters
                    .iter()
                    .filter(|m| {
                        shown.iter().any(|x| {
                            m.level.is_some()
                                && m.level == x.level
                                && (m.race.is_some() && m.race == x.race || m.attribute.is_some() && m.attribute == x.attribute)
                        })
                    })
                    .map(|m| m.id)
                    .collect();
                let hit = found(&result.links);
                result.with = reach(&no_mark, &|_| hit);
            }
        }
        cards.push(result);
    }
    Ok(SpecialReport { base, target_copies, size, cards })
}

/// Every path `a → b → c` between the monsters, with `share` properties in
/// common at each step. `c` can have the name of `a` (another copy).
pub fn bridges(monsters: &[&Facts], share: u32) -> Vec<Chain> {
    let mut out = Vec::new();
    for a in monsters {
        for b in monsters.iter().filter(|b| a.shared(b) == share) {
            for c in monsters.iter().filter(|c| b.shared(c) == share) {
                out.push(Chain { from: a.id, via: b.id, to: c.id });
            }
        }
    }
    out
}

/// Cards of the deck with the same flags, and how many of them are in hand.
#[derive(Debug, Clone, Copy)]
struct Group {
    flags: u8,
    copies: u32,
    held: u32,
}

/// Chance of a hand with the target, or with `special` and `then` (given the
/// hand's groups) of reaching it. `mark` sets the mechanism's own flag.
fn reach_chance(
    deck: &DeckModel,
    special: u32,
    hits: &dyn Fn(u32) -> bool,
    mark: &dyn Fn(u32) -> bool,
    hand: u32,
    then: &dyn Fn(&[Group]) -> f64,
) -> f64 {
    let mut by_flags: BTreeMap<u8, u32> = BTreeMap::new();
    for c in &deck.cards {
        let flags = [(c.id == special, SELF), (hits(c.id), TARGET), (mark(c.id), MARK)].iter().filter(|f| f.0).fold(0, |acc, f| acc | f.1);
        *by_flags.entry(flags).or_default() += c.copies;
    }
    let copies: Vec<u32> = by_flags.values().copied().collect();
    let mut groups: Vec<Group> = by_flags.into_iter().map(|(flags, copies)| Group { flags, copies, held: 0 }).collect();
    let mut p = 0.0;
    split(&copies, hand, &mut |counts, weight| {
        for (g, &k) in groups.iter_mut().zip(counts) {
            g.held = k;
        }
        let has = |flag: u8| groups.iter().any(|g| g.flags & flag != 0 && g.held > 0);
        let hit = if has(TARGET) {
            1.0
        } else if has(SELF) {
            then(&groups)
        } else {
            0.0
        };
        p += weight * hit;
    });
    p / binom(deck.size(), hand)
}

/// After drawing `n` from the `rest` cards left, the chance to hold the
/// target and pay the cost (a `MARK` card) without losing its last copy.
fn draw_then_banish(groups: &[Group], n: u32, rest: u32) -> f64 {
    let left: Vec<u32> = groups.iter().map(|g| g.copies - g.held).collect();
    let n = n.min(rest);
    let mut p = 0.0;
    split(&left, n, &mut |drawn, weight| {
        let count = |want: u8, avoid: u8| -> u32 {
            groups.iter().zip(drawn).filter(|(g, _)| g.flags & want == want && g.flags & avoid == 0).map(|(g, &d)| g.held + d).sum()
        };
        let (target_payer, target_other, other_payer) = (count(TARGET | MARK, 0), count(TARGET, MARK), count(MARK, TARGET));
        let has_target = target_payer + target_other > 0;
        let pays_keeping_it = other_payer > 0 || target_payer >= 2 || (target_payer >= 1 && target_other >= 1);
        if has_target && pays_keeping_it {
            p += weight;
        }
    });
    p / binom(rest, n)
}

/// Chance to open `special` with no card that pays its cost, and draw none
/// with its `n` draws: the hand is lost. Not divided by the chance to open it.
fn lose_hand(deck: &DeckModel, special: u32, hand: u32, n: u32, pays: &dyn Fn(u32) -> bool) -> f64 {
    let size = deck.size();
    let copies = |f: &dyn Fn(u32) -> bool| deck.cards.iter().filter(|c| f(c.id)).map(|c| c.copies).sum::<u32>();
    let own = copies(&|id| id == special);
    let payers = copies(pays);
    let others = size - own - payers;
    let rest = size - hand;
    (1..=own.min(hand)).map(|k| binom(own, k) * binom(others, hand - k) * p_none(rest, payers, n.min(rest))).sum::<f64>()
        / binom(size, hand)
}

/// Visits every way to take `k` cards from groups of `copies`, with the
/// number of ways to do it.
fn split(copies: &[u32], k: u32, visit: &mut impl FnMut(&[u32], f64)) {
    fn go(copies: &[u32], i: usize, left: u32, weight: f64, counts: &mut Vec<u32>, visit: &mut impl FnMut(&[u32], f64)) {
        if i == copies.len() {
            if left == 0 {
                visit(counts, weight);
            }
            return;
        }
        let room: u32 = copies[i + 1..].iter().sum();
        for x in left.saturating_sub(room)..=left.min(copies[i]) {
            counts.push(x);
            go(copies, i + 1, left - x, weight * binom(copies[i], x), counts, visit);
            counts.pop();
        }
    }
    go(copies, 0, k, 1.0, &mut Vec::with_capacity(copies.len()), visit);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::CardEntry;

    const S: u32 = 1;
    const T: u32 = 2;
    const P: u32 = 3;
    const X: u32 = 4;

    fn deck(entries: &[(u32, u32)]) -> DeckModel {
        DeckModel { cards: entries.iter().map(|&(id, copies)| CardEntry { id, copies, roles: vec![] }).collect() }
    }

    fn monster(id: u32, attribute: &str, race: &str, level: u8, atk: i32, def: i32) -> Facts {
        Facts {
            id,
            name: format!("Monster {id}"),
            kind: "monster".into(),
            race: Some(race.into()),
            attribute: Some(attribute.into()),
            level: Some(level),
            atk: Some(atk),
            def: Some(def),
        }
    }

    fn input(deck: DeckModel, hand: u32, cards: Vec<Facts>, mechanism: Mechanism) -> SpecialInput {
        SpecialInput { deck, hand, target: Target::Card(T), cards, extra: vec![], special: vec![(S, mechanism)] }
    }

    /// Every ordered deal of the first `depth` cards, with `check` on it.
    fn brute(deck: &DeckModel, depth: usize, check: impl Fn(&[u32]) -> bool) -> f64 {
        let cards: Vec<u32> = deck.cards.iter().flat_map(|c| std::iter::repeat_n(c.id, c.copies as usize)).collect();
        fn go(
            cards: &[u32],
            used: &mut Vec<bool>,
            seq: &mut Vec<u32>,
            depth: usize,
            check: &dyn Fn(&[u32]) -> bool,
            hits: &mut (u64, u64),
        ) {
            if seq.len() == depth {
                hits.1 += 1;
                hits.0 += u64::from(check(seq));
                return;
            }
            for i in 0..cards.len() {
                if !used[i] {
                    used[i] = true;
                    seq.push(cards[i]);
                    go(cards, used, seq, depth, check, hits);
                    seq.pop();
                    used[i] = false;
                }
            }
        }
        let mut hits = (0, 0);
        go(&cards, &mut vec![false; cards.len()], &mut Vec::new(), depth, &check, &mut hits);
        hits.0 as f64 / hits.1 as f64
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn draws_and_excavations_match_brute_force() {
        let d = deck(&[(S, 1), (T, 2), (X, 6)]);
        let hand = 3;
        // Banish 2, then draw 1: the target must be the 6th card.
        let report = special(&input(d.clone(), hand, vec![], Mechanism::Draw { n: 1, banish_top: 2, min_extra: 0 })).unwrap();
        let want = brute(&d, 6, |s| s[..3].contains(&T) || (s[..3].contains(&S) && s[5] == T));
        assert!(close(report.cards[0].with, want), "{} vs {want}", report.cards[0].with);
        let risk = report.cards[0].banish_risk.unwrap();
        assert!(close(risk, 1.0 - p_none(6, 2, 2)), "{risk}");
        // Excavate 3, add 1.
        let report = special(&input(d.clone(), hand, vec![], Mechanism::Excavate { n: vec![3], banish_extra: false })).unwrap();
        let want = brute(&d, 6, |s| s[..3].contains(&T) || (s[..3].contains(&S) && s[3..6].contains(&T)));
        assert!(close(report.cards[0].with, want));
        assert!(close(report.base, brute(&d, 3, |s| s.contains(&T))));
    }

    #[test]
    fn excavation_with_an_extra_deck_cost_needs_the_extra_deck() {
        let d = deck(&[(S, 1), (T, 2), (X, 20)]);
        let mut i = input(d, 5, vec![], Mechanism::Excavate { n: vec![3, 6], banish_extra: true });
        let report = special(&i).unwrap();
        assert!(report.cards[0].blocked.is_some() && close(report.cards[0].with, report.base));
        i.extra = vec![Facts::default(); 4];
        let report = special(&i).unwrap();
        assert_eq!(report.cards[0].variants.iter().map(|v| v.n).collect::<Vec<_>>(), [3]);
        i.extra = vec![Facts::default(); 15];
        let report = special(&i).unwrap();
        let (three, six) = (report.cards[0].variants[0].with, report.cards[0].variants[1].with);
        assert!(three < six && close(report.cards[0].with, six));
    }

    #[test]
    fn draw_then_banish_matches_brute_force() {
        // P pays the cost; the target pays it too in the second deck.
        let dark = |id| Facts { kind: "monster".into(), attribute: Some("DARK".into()), ..monster(id, "DARK", "Fiend", 4, 0, 0) };
        let cost = Filter { kind: Some("monster".into()), attribute: Some("DARK".into()), ..Filter::default() };
        for target_pays in [false, true] {
            let d = deck(&[(S, 2), (T, 2), (P, 2), (X, 3)]);
            let mut cards = vec![dark(P)];
            if target_pays {
                cards.push(dark(T));
            }
            let pays = |c: u32| c == P || (target_pays && c == T);
            let report = special(&input(d.clone(), 3, cards, Mechanism::DrawThenBanish { n: 2, cost: cost.clone() })).unwrap();
            let want = brute(&d, 5, |s| {
                if s[..3].contains(&T) {
                    return true;
                }
                if !s[..3].contains(&S) {
                    return false;
                }
                // The hand after the draw, without the card that resolved.
                let mut after: Vec<u32> = s.to_vec();
                after.remove(after.iter().position(|&c| c == S).unwrap());
                (0..after.len()).any(|i| pays(after[i]) && after.iter().enumerate().any(|(j, &c)| j != i && c == T))
            });
            let r = &report.cards[0];
            assert!(close(r.with, want), "target pays {target_pays}: {} vs {want}", r.with);
            let lose = brute(&d, 5, |s| s[..3].contains(&S) && !s.iter().any(|&c| pays(c)));
            assert!(close(r.lose_hand.unwrap() * r.open, lose), "{} vs {lose}", r.lose_hand.unwrap() * r.open);
        }
    }

    #[test]
    fn bridges_find_the_paths_and_the_chance() {
        // A shares only the Level with B; B shares only the Attribute with T.
        let a = monster(P, "DARK", "Fiend", 4, 1000, 1000);
        let b = monster(X, "LIGHT", "Fairy", 4, 2000, 0);
        let t = monster(T, "LIGHT", "Dragon", 8, 3000, 2500);
        let d = deck(&[(S, 1), (T, 1), (P, 3), (X, 1), (9, 34)]);
        let report = special(&input(d, 5, vec![a, b, t], Mechanism::Bridge { share: 1 })).unwrap();
        let r = &report.cards[0];
        assert!(r.chains.contains(&Chain { from: P, via: X, to: T }));
        assert!(r.chains.contains(&Chain { from: T, via: X, to: P }), "the way back works too");
        assert!(!r.chains.iter().any(|c| c.from == P && c.via == T), "two properties apart");
        assert_eq!(r.links, [P]);
        // The target, or no target and Small World with A among 39 cards (inclusion-exclusion).
        let no_t = p_none(40, 1, 5);
        let sw_a_without_t = 1.0 - p_none(39, 1, 5) - p_none(39, 3, 5) + p_none(39, 4, 5);
        assert!(close(r.with, report.base + no_t * sw_a_without_t), "{} vs {}", r.with, report.base + no_t * sw_a_without_t);
    }

    #[test]
    fn searches_follow_the_filter_and_the_revealed_monster() {
        let warrior = monster(T, "EARTH", "Warrior", 4, 1800, 1200);
        let big = monster(P, "EARTH", "Warrior", 7, 2500, 2000);
        let d = deck(&[(S, 1), (T, 1), (P, 1), (X, 37)]);
        let filter = Filter { kind: Some("monster".into()), race: Some("Warrior".into()), max_level: Some(4), ..Filter::default() };
        let report = special(&input(d.clone(), 5, vec![warrior.clone(), big.clone()], Mechanism::Search { filter })).unwrap();
        assert_eq!(report.cards[0].links, [T]);
        let want = 1.0 - p_none(40, 2, 5);
        assert!(close(report.cards[0].with, want));

        // Rank 4 WATER Warrior in the Extra Deck: the Level 4 Warrior shares Type and Level.
        let mut i = input(d, 5, vec![warrior, big], Mechanism::RevealSearch { reveal: vec!["Number 101:".into()], put_back: 1 });
        let report = special(&i).unwrap();
        assert!(report.cards[0].blocked.is_some() && report.cards[0].links.is_empty());
        i.extra = vec![Facts { name: "Number 101: Test".into(), ..monster(50, "WATER", "Warrior", 4, 2100, 1000) }];
        let report = special(&i).unwrap();
        assert_eq!(report.cards[0].links, [T]);
        assert!(close(report.cards[0].with, want));
    }

    #[test]
    fn known_cards_parse() {
        assert!(known().len() >= 10);
        assert!(matches!(mechanism_of("Small World"), Some(Mechanism::Bridge { share: 1 })));
        assert!(matches!(mechanism_of("Seventh Tachyon"), Some(Mechanism::RevealSearch { .. })));
        assert!(mechanism_of("Ash Blossom & Joyous Spring").is_none());
    }
}
