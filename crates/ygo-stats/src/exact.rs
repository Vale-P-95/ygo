//! Exact probabilities for a hand drawn without replacement: the
//! multivariate hypergeometric distribution over groups of cards.
//!
//! Cards are grouped into "atoms": cards named by the condition each get
//! their own atom, the others are grouped by which of the condition's roles
//! they have. Every way to split the hand among the atoms is enumerated and
//! weighted by the number of hands that produce it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::cond::{Cond, Target};

/// The Main Deck as the lab sees it: each card once, with copies and roles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeckModel {
    pub cards: Vec<CardEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEntry {
    pub id: u32,
    pub copies: u32,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl DeckModel {
    /// From a Main Deck (one passcode per copy) and a role → passcodes map.
    pub fn from_ids(main: &[u32], roles: &BTreeMap<String, Vec<u32>>) -> Self {
        let mut cards: Vec<CardEntry> = Vec::new();
        for id in main {
            match cards.iter_mut().find(|c| c.id == *id) {
                Some(c) => c.copies += 1,
                None => cards.push(CardEntry { id: *id, copies: 1, roles: Vec::new() }),
            }
        }
        for (role, ids) in roles {
            for card in cards.iter_mut().filter(|c| ids.contains(&c.id)) {
                if !card.roles.contains(role) {
                    card.roles.push(role.clone());
                }
            }
        }
        Self { cards }
    }

    pub fn size(&self) -> u32 {
        self.cards.iter().map(|c| c.copies).sum()
    }

    /// Copies of cards that have `role`.
    pub fn copies_with(&self, role: &str) -> u32 {
        self.cards.iter().filter(|c| c.roles.iter().any(|r| r == role)).map(|c| c.copies).sum()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExactError {
    /// The hand is bigger than the deck.
    HandTooLarge,
    /// Too many cases to enumerate: use a simulation instead.
    TooComplex { cases: f64 },
}

impl std::fmt::Display for ExactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExactError::HandTooLarge => write!(f, "la mano ha più carte del deck"),
            ExactError::TooComplex { cases } => write!(f, "condizione troppo complessa per il calcolo esatto ({cases:.0} casi)"),
        }
    }
}

impl std::error::Error for ExactError {}

/// Above this many cases the exact calculation gives way to simulation.
pub const MAX_CASES: f64 = 5e6;

/// Binomial coefficient as a float (exact for the sizes of a deck).
pub fn binom(n: u32, k: u32) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// Probability that none of `hits` copies is among `hand` cards drawn from `size`.
pub fn p_none(size: u32, hits: u32, hand: u32) -> f64 {
    if hand > size {
        return if hits > 0 { 0.0 } else { 1.0 };
    }
    binom(size - hits.min(size), hand) / binom(size, hand)
}

/// Probability of exactly `k` of `hits` copies among `hand` cards drawn from `size`.
pub fn pmf(size: u32, hits: u32, hand: u32, k: u32) -> f64 {
    if hand > size || k > hits || k > hand || hand - k > size - hits {
        return 0.0;
    }
    binom(hits, k) * binom(size - hits, hand - k) / binom(size, hand)
}

struct Atom {
    copies: u32,
    /// Indices into the condition's targets that this atom counts for.
    targets: Vec<usize>,
}

/// Exact probability that a `hand`-card hand satisfies `cond`.
pub fn probability(deck: &DeckModel, cond: &Cond, hand: u32) -> Result<f64, ExactError> {
    let size = deck.size();
    if hand > size {
        return Err(ExactError::HandTooLarge);
    }
    let mut set = BTreeSet::new();
    cond.targets(&mut set);
    let targets: Vec<Target> = set.into_iter().collect();

    // Group cards by the targets they count for.
    let mut groups: BTreeMap<Vec<usize>, u32> = BTreeMap::new();
    for card in &deck.cards {
        let mut key: Vec<usize> = Vec::new();
        for (i, t) in targets.iter().enumerate() {
            let hit = match t {
                Target::Role(r) => card.roles.iter().any(|x| x == r),
                Target::Card(id) => card.id == *id,
            };
            if hit {
                key.push(i);
            }
        }
        *groups.entry(key).or_default() += card.copies;
    }
    let atoms: Vec<Atom> = groups.into_iter().map(|(targets, copies)| Atom { copies, targets }).collect();

    let cases = if atoms.is_empty() { 1.0 } else { binom(hand + atoms.len() as u32 - 1, atoms.len() as u32 - 1) };
    if cases > MAX_CASES {
        return Err(ExactError::TooComplex { cases });
    }

    let total = binom(size, hand);
    let mut counts = vec![0u32; targets.len()];
    let mut p = 0.0;
    enumerate(&atoms, 0, hand, 1.0, &mut counts, &mut |weight, counts| {
        let count = |t: &Target| targets.iter().position(|x| x == t).map_or(0, |i| counts[i]);
        if cond.eval(&count) {
            p += weight;
        }
    });
    Ok(p / total)
}

/// Visits every split of `left` cards among `atoms[i..]`, with the number of
/// hands producing it and the per-target counts.
fn enumerate(atoms: &[Atom], i: usize, left: u32, weight: f64, counts: &mut [u32], visit: &mut impl FnMut(f64, &[u32])) {
    let Some(atom) = atoms.get(i) else {
        if left == 0 {
            visit(weight, counts);
        }
        return;
    };
    // Cards still to place must fit in the atoms that follow.
    let room: u32 = atoms[i + 1..].iter().map(|a| a.copies).sum();
    let lo = left.saturating_sub(room);
    for x in lo..=left.min(atom.copies) {
        for &t in &atom.targets {
            counts[t] += x;
        }
        enumerate(atoms, i + 1, left - x, weight * binom(atom.copies, x), counts, visit);
        for &t in &atom.targets {
            counts[t] -= x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cond::Op;

    fn deck(entries: &[(u32, u32, &[&str])]) -> DeckModel {
        DeckModel {
            cards: entries
                .iter()
                .map(|(id, copies, roles)| CardEntry { id: *id, copies: *copies, roles: roles.iter().map(|r| r.to_string()).collect() })
                .collect(),
        }
    }

    #[test]
    fn three_of_in_forty() {
        // The classic: a 3-of in a 40-card deck, 5-card hand.
        let d = deck(&[(1, 3, &[]), (2, 37, &[])]);
        let p = probability(&d, &Cond::card(1, Op::AtLeast, 1), 5).unwrap();
        assert!((p - 0.337_55).abs() < 1e-5, "{p}");
        assert!((p - (1.0 - p_none(40, 3, 5))).abs() < 1e-12);
    }

    #[test]
    fn matches_brute_force_with_overlapping_roles() {
        // 10 cards, some with two roles; check every 4-card hand by hand.
        let d = deck(&[(1, 2, &["starter"]), (2, 1, &["starter", "handtrap"]), (3, 3, &["handtrap"]), (4, 1, &["extender"]), (5, 3, &[])]);
        let cond = Cond::all([
            Cond::role("starter", Op::AtLeast, 1),
            Cond::any([Cond::role("handtrap", Op::AtLeast, 2), Cond::card(4, Op::Exactly, 1)]),
            Cond::negate(Cond::card(5, Op::AtLeast, 2)),
        ]);
        let cards: Vec<&CardEntry> = d.cards.iter().flat_map(|c| std::iter::repeat_n(c, c.copies as usize)).collect();
        let (mut good, mut all) = (0u32, 0u32);
        let n = cards.len();
        for a in 0..n {
            for b in a + 1..n {
                for c in b + 1..n {
                    for e in c + 1..n {
                        let hand = [cards[a], cards[b], cards[c], cards[e]];
                        let count = |t: &Target| {
                            hand.iter()
                                .filter(|card| match t {
                                    Target::Role(r) => card.roles.contains(r),
                                    Target::Card(id) => card.id == *id,
                                })
                                .count() as u32
                        };
                        all += 1;
                        good += cond.eval(&count) as u32;
                    }
                }
            }
        }
        let exact = probability(&d, &cond, 4).unwrap();
        assert!((exact - good as f64 / all as f64).abs() < 1e-12, "{exact} vs {good}/{all}");
    }

    #[test]
    fn refuses_impossible_or_huge_cases() {
        let d = deck(&[(1, 3, &[])]);
        assert_eq!(probability(&d, &Cond::card(1, Op::AtLeast, 1), 5), Err(ExactError::HandTooLarge));
        // 40 different cards each named by the condition, 20-card hand.
        let many = DeckModel { cards: (1..=40).map(|id| CardEntry { id, copies: 1, roles: vec![] }).collect() };
        let cond = Cond::any((1..=40).map(|id| Cond::card(id, Op::AtLeast, 1)));
        assert!(matches!(probability(&many, &cond, 20), Err(ExactError::TooComplex { .. })));
    }

    #[test]
    fn deck_model_from_a_deck_list() {
        let roles = BTreeMap::from([("starter".to_owned(), vec![7, 9]), ("handtrap".to_owned(), vec![9])]);
        let d = DeckModel::from_ids(&[7, 7, 8, 9, 9, 9], &roles);
        assert_eq!(d.size(), 6);
        assert_eq!((d.copies_with("starter"), d.copies_with("handtrap")), (5, 3));
        assert_eq!(d.cards.iter().find(|c| c.id == 9).unwrap().roles, ["handtrap", "starter"]);
    }
}
