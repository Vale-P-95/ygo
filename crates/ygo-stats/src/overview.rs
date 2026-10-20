//! The lab's overview of a deck: the main probabilities going first (5
//! cards) and second (6), role distributions, per-card odds and the
//! opponent's chance of holding a handtrap.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::cond::{Cond, Op};
use crate::exact::{binom, p_none, pmf, probability, DeckModel};
use crate::roles::{BREAKER, BRICK, EXTENDER, HANDTRAP, STARTER};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandStats {
    pub hand: u32,
    /// At least one Starter.
    pub starter: f64,
    /// At least two Starters.
    pub starter2: f64,
    /// At least one Handtrap.
    pub handtrap: f64,
    /// A Starter and a Handtrap, two different cards.
    pub ideal: f64,
    /// Plays through one interruption: two Starters, or a Starter and an
    /// Extender (two different cards).
    pub resist: f64,
    /// Resists and holds a Handtrap besides those two cards.
    pub solid: f64,
    /// No Starter at all.
    pub brick: f64,
    /// At least one card marked Brick/Garnet.
    pub garnet: f64,
    /// Board breaker in hand (matters going second).
    pub breaker: f64,
    /// Expected copies in hand, per role.
    pub expected: BTreeMap<String, f64>,
    /// P(exactly k Starters), k = 0..=4, then 5 or more.
    pub starter_dist: Vec<f64>,
    pub handtrap_dist: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardOdds {
    pub id: u32,
    pub copies: u32,
    /// At least one copy in the opening hand, going first / second.
    pub first: f64,
    pub second: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub size: u32,
    /// Copies per role.
    pub roles: BTreeMap<String, u32>,
    pub first: HandStats,
    pub second: HandStats,
    pub cards: Vec<CardOdds>,
    /// Cards seen (5..=10) for the "draw it by" curves.
    pub seen: Vec<u32>,
    /// At least one of 1, 2 and 3 copies among the cards seen.
    pub by_copies: Vec<Vec<f64>>,
    /// The opponent's 5-card hand from 40 holds at least one handtrap, for 0..=20 handtraps.
    pub opponent: Vec<f64>,
}

fn dist(size: u32, hits: u32, hand: u32) -> Vec<f64> {
    let mut out: Vec<f64> = (0..5).map(|k| pmf(size, hits, hand, k)).collect();
    out.push((5..=hand).map(|k| pmf(size, hits, hand, k)).sum());
    out
}

/// Bits of the roles the hand presets look at.
const S: usize = 1;
const E: usize = 2;
const H: usize = 4;

/// Cards in hand with any of the roles in `mask`, from the count per role
/// combination (index = bits of Starter, Extender, Handtrap).
fn any_of(counts: &[u32; 8], mask: usize) -> u32 {
    (1..8).filter(|m| m & mask != 0).map(|m| counts[m]).sum()
}

// The presets need different cards for different jobs: a card with two roles
// does one of them. Each checks that the jobs can be given to distinct cards
// (Hall's condition: every group of jobs has as many cards as jobs).

/// A Starter and a Handtrap.
fn ideal(c: &[u32; 8]) -> bool {
    any_of(c, S) >= 1 && any_of(c, H) >= 1 && any_of(c, S | H) >= 2
}

/// A Starter and a second Starter or Extender.
fn resist(c: &[u32; 8]) -> bool {
    any_of(c, S) >= 1 && any_of(c, S | E) >= 2
}

/// `resist` and, besides, a Handtrap.
fn solid(c: &[u32; 8]) -> bool {
    resist(c) && any_of(c, H) >= 1 && any_of(c, S | H) >= 2 && any_of(c, S | E | H) >= 3
}

/// Exact probability that a hand meets `test`, given how many of its cards
/// have each combination of Starter, Extender and Handtrap.
fn by_cards(deck: &DeckModel, hand: u32, test: fn(&[u32; 8]) -> bool) -> f64 {
    let mut groups = [0u32; 8];
    for card in &deck.cards {
        let has = |r: &str| card.roles.iter().any(|x| x == r);
        groups[(has(STARTER) as usize) * S + (has(EXTENDER) as usize) * E + (has(HANDTRAP) as usize) * H] += card.copies;
    }
    fn split(groups: &[u32; 8], i: usize, left: u32, weight: f64, counts: &mut [u32; 8], test: fn(&[u32; 8]) -> bool) -> f64 {
        if i == 8 {
            return if left == 0 && test(counts) { weight } else { 0.0 };
        }
        let room: u32 = groups[i + 1..].iter().sum();
        (left.saturating_sub(room)..=left.min(groups[i]))
            .map(|x| {
                counts[i] = x;
                split(groups, i + 1, left - x, weight * binom(groups[i], x), counts, test)
            })
            .sum()
    }
    let size = deck.size();
    if hand > size {
        return 0.0;
    }
    split(&groups, 0, hand, 1.0, &mut [0; 8], test) / binom(size, hand)
}

fn hand_stats(deck: &DeckModel, hand: u32) -> HandStats {
    let size = deck.size();
    let hand = hand.min(size);
    let p = |c: &Cond| probability(deck, c, hand).unwrap_or(0.0);
    let at_least = |role: &str| 1.0 - p_none(size, deck.copies_with(role), hand);
    let mut expected = BTreeMap::new();
    for card in &deck.cards {
        for role in &card.roles {
            *expected.entry(role.clone()).or_insert(0.0) += card.copies as f64 * hand as f64 / size.max(1) as f64;
        }
    }
    HandStats {
        hand,
        starter: at_least(STARTER),
        starter2: p(&Cond::role(STARTER, Op::AtLeast, 2)),
        handtrap: at_least(HANDTRAP),
        ideal: by_cards(deck, hand, ideal),
        resist: by_cards(deck, hand, resist),
        solid: by_cards(deck, hand, solid),
        brick: p_none(size, deck.copies_with(STARTER), hand),
        garnet: at_least(BRICK),
        breaker: at_least(BREAKER),
        expected,
        starter_dist: dist(size, deck.copies_with(STARTER), hand),
        handtrap_dist: dist(size, deck.copies_with(HANDTRAP), hand),
    }
}

pub fn overview(deck: &DeckModel) -> Overview {
    let size = deck.size();
    let mut roles: BTreeMap<String, u32> = BTreeMap::new();
    for card in &deck.cards {
        for role in &card.roles {
            *roles.entry(role.clone()).or_default() += card.copies;
        }
    }
    let first = hand_stats(deck, 5);
    let second = hand_stats(deck, 6);
    let cards = deck
        .cards
        .iter()
        .map(|c| CardOdds {
            id: c.id,
            copies: c.copies,
            first: 1.0 - p_none(size, c.copies, 5.min(size)),
            second: 1.0 - p_none(size, c.copies, 6.min(size)),
        })
        .collect();
    let seen: Vec<u32> = (5..=10).collect();
    let by_copies = (1..=3).map(|k| seen.iter().map(|&n| 1.0 - p_none(size, k, n.min(size))).collect()).collect();
    let opponent = (0..=20).map(|h| 1.0 - p_none(40, h, 5)).collect();
    Overview { size, roles, first, second, cards, seen, by_copies, opponent }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::CardEntry;

    /// The example of the lab plan: Invoked with 13 Starters, 11 Extenders,
    /// 10 Handtraps, 3 Board breakers and 3 other cards.
    fn invoked() -> DeckModel {
        let groups: [(u32, u32, &str); 5] = [(1, 13, STARTER), (2, 11, EXTENDER), (3, 10, HANDTRAP), (4, 3, BREAKER), (5, 3, "")];
        DeckModel {
            cards: groups
                .iter()
                .map(|(id, copies, role)| CardEntry {
                    id: *id,
                    copies: *copies,
                    roles: if role.is_empty() { vec![] } else { vec![role.to_string()] },
                })
                .collect(),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 5e-4
    }

    #[test]
    fn matches_the_numbers_in_the_plan() {
        let o = overview(&invoked());
        assert_eq!(o.size, 40);
        let f = &o.first;
        assert!(close(f.starter, 0.8773) && close(f.starter2, 0.5306) && close(f.handtrap, 0.7834), "{f:?}");
        assert!(close(f.ideal, 0.6701) && close(f.resist, 0.8414) && close(f.solid, 0.6345) && close(f.brick, 0.1227), "{f:?}");
        let s = &o.second;
        assert!(close(s.starter, 0.9229) && close(s.ideal, 0.7714) && close(s.resist, 0.9081) && close(s.brick, 0.0771), "{s:?}");
        assert!(close(f.expected[STARTER], 1.625) && close(s.expected[HANDTRAP], 1.5));
        assert!(close(f.handtrap_dist[1], 0.416) && close(f.handtrap_dist.iter().sum::<f64>(), 1.0));
        assert!(close(o.by_copies[2][0], 0.3376) && close(o.opponent[10], 0.7834));
        assert_eq!(f.garnet, 0.0);
    }

    fn deck(entries: &[(u32, u32, &[&str])]) -> DeckModel {
        DeckModel {
            cards: entries
                .iter()
                .map(|(id, copies, roles)| CardEntry { id: *id, copies: *copies, roles: roles.iter().map(|r| r.to_string()).collect() })
                .collect(),
        }
    }

    #[test]
    fn a_card_with_two_roles_is_still_one_card() {
        // References: every hand enumerated, each job given to a different card.
        let check = |d: &DeckModel, want: [[f64; 3]; 2]| {
            let o = overview(d);
            for (h, w) in [&o.first, &o.second].into_iter().zip(want) {
                let got = [h.ideal, h.resist, h.solid];
                assert!(got.iter().zip(w).all(|(a, b)| (a - b).abs() < 1e-9), "{got:?} vs {w:?}");
            }
        };
        // Starter+Extender ×3, Starters ×6, Extenders ×3, Handtraps ×10.
        let d = deck(&[(1, 3, &[STARTER, EXTENDER]), (2, 6, &[STARTER]), (4, 3, &[EXTENDER]), (5, 10, &[HANDTRAP]), (9, 18, &[])]);
        let o = overview(&d);
        assert!((o.first.resist - 0.461_729_948_572).abs() < 1e-9 && (o.second.resist - 0.577_738_264_580).abs() < 1e-9, "{o:?}");
        assert!((o.first.solid - 0.317_936_863_989).abs() < 1e-9 && (o.second.solid - 0.457_271_036_218).abs() < 1e-9, "{o:?}");
        // Starter+Handtrap ×3, Starters ×9, Handtraps ×9.
        let d = deck(&[(1, 3, &[STARTER, HANDTRAP]), (2, 9, &[STARTER]), (5, 9, &[HANDTRAP]), (9, 19, &[])]);
        let o = overview(&d);
        assert!((o.first.ideal - 0.701_280_227_596).abs() < 1e-9 && (o.second.ideal - 0.801_678_833_258).abs() < 1e-9, "{o:?}");
        // Every mix of the three roles.
        let d = deck(&[
            (1, 2, &[STARTER, EXTENDER]),
            (2, 2, &[STARTER, HANDTRAP]),
            (3, 1, &[EXTENDER, HANDTRAP]),
            (4, 1, &[STARTER, EXTENDER, HANDTRAP]),
            (5, 5, &[STARTER]),
            (6, 4, &[EXTENDER]),
            (7, 6, &[HANDTRAP]),
            (9, 19, &[]),
        ]);
        check(&d, [[0.577_622_156_570, 0.591_181_262_234, 0.394_211_012_632], [0.690_611_664_296, 0.706_888_062_151, 0.548_154_690_260]]);
    }
}
