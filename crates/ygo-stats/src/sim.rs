//! Monte Carlo simulation of opening hands with cards that change the deck:
//! draw spells (Pot of Desires, Upstart Goblin…), excavators (Pot of
//! Prosperity, Pot of Duality) and searchers ("Magical Meltdown adds Aleister").
//!
//! A card that resolves its effect stops counting for its own roles: it
//! has turned into what it drew or fetched. Cards can also be limited to
//! one copy per turn, and cards that need the Normal Summon count one at a
//! time (the best one for the condition).
//!
//! Hands are simulated in fixed chunks, each with its own seed derived from
//! the run's seed, so a run gives the same result on any number of threads.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::cond::{Cond, Target};
use crate::exact::DeckModel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Effect {
    /// Banishes `banish_top` cards from the top of the deck, then draws `n`
    /// (Pot of Desires: 10 and 2; Upstart Goblin: 0 and 1).
    #[serde(rename_all = "camelCase")]
    Draw {
        n: u32,
        #[serde(default)]
        banish_top: u32,
    },
    /// Looks at the top `n` cards and adds the best one (Pot of Prosperity, Pot of Duality).
    Excavate { n: u32 },
    /// Adds one of `targets` from the deck.
    Search { targets: Vec<u32> },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRule {
    pub id: u32,
    #[serde(default)]
    pub effect: Option<Effect>,
    /// Only one copy resolves its effect and counts for its roles.
    #[serde(default)]
    pub once_per_turn: bool,
    /// Needs the Normal Summon: only one such card counts per hand.
    #[serde(default)]
    pub normal_summon: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimConfig {
    pub deck: DeckModel,
    pub cond: Cond,
    pub hand: u32,
    #[serde(default)]
    pub rules: Vec<CardRule>,
    pub hands: u64,
    #[serde(default)]
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub hands: u64,
    pub p: f64,
    /// 95% Wilson interval.
    pub lo: f64,
    pub hi: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimResult {
    pub hands: u64,
    pub hits: u64,
    pub p: f64,
    pub lo: f64,
    pub hi: f64,
    pub checkpoints: Vec<Checkpoint>,
    /// Per card with an effect: how often it resolved, per hand.
    pub effects: Vec<(u32, f64)>,
    pub cancelled: bool,
}

/// Wilson score interval for `hits` out of `n` (z = 1.96).
pub fn wilson(hits: u64, n: u64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let (n, p, z) = (n as f64, hits as f64 / n as f64, 1.959_964);
    let d = 1.0 + z * z / n;
    let c = p + z * z / (2.0 * n);
    let r = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt();
    (((c - r) / d).max(0.0), ((c + r) / d).min(1.0))
}

/// xoshiro256** seeded with SplitMix64: small, fast and reproducible.
#[derive(Clone)]
pub struct Rng([u64; 4]);

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut x = seed;
        let mut next = || {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        Rng([next(), next(), next(), next()])
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }
}

/// The deck flattened for fast sampling.
struct Prepared<'a> {
    cond: &'a Cond,
    targets: Vec<Target>,
    /// One entry per copy: index into `cards`.
    copies: Vec<u16>,
    cards: Vec<PreparedCard>,
    hand: u32,
}

struct PreparedCard {
    id: u32,
    /// Indices into `targets` this card counts for.
    hits: Vec<usize>,
    effect: Option<Effect>,
    once: bool,
    normal_summon: bool,
}

impl<'a> Prepared<'a> {
    fn new(config: &'a SimConfig) -> Self {
        let mut set = BTreeSet::new();
        config.cond.targets(&mut set);
        let targets: Vec<Target> = set.into_iter().collect();
        let cards: Vec<PreparedCard> = config
            .deck
            .cards
            .iter()
            .map(|c| {
                let rule = config.rules.iter().find(|r| r.id == c.id);
                PreparedCard {
                    id: c.id,
                    hits: targets
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| match t {
                            Target::Role(r) => c.roles.contains(r),
                            Target::Card(id) => c.id == *id,
                        })
                        .map(|(i, _)| i)
                        .collect(),
                    effect: rule.and_then(|r| r.effect.clone()),
                    once: rule.is_some_and(|r| r.once_per_turn),
                    normal_summon: rule.is_some_and(|r| r.normal_summon),
                }
            })
            .collect();
        let copies = config.deck.cards.iter().enumerate().flat_map(|(i, c)| std::iter::repeat_n(i as u16, c.copies as usize)).collect();
        Self { cond: &config.cond, targets, copies, cards, hand: config.hand }
    }

    fn index_of(&self, id: u32) -> Option<u16> {
        self.cards.iter().position(|c| c.id == id).map(|i| i as u16)
    }

    /// Whether the counted cards satisfy the condition, letting the player
    /// pick which Normal Summon card to count.
    fn satisfied(&self, counted: &[u16]) -> bool {
        let mut base = vec![0u32; self.targets.len()];
        let mut seen: Vec<u16> = Vec::with_capacity(counted.len());
        let mut summons: Vec<u16> = Vec::new();
        for &c in counted {
            let card = &self.cards[c as usize];
            if card.once {
                if seen.contains(&c) {
                    continue;
                }
                seen.push(c);
            }
            if card.normal_summon {
                if !summons.contains(&c) {
                    summons.push(c);
                }
                continue;
            }
            for &t in &card.hits {
                base[t] += 1;
            }
        }
        let check = |counts: &[u32]| {
            let count = |t: &Target| self.targets.iter().position(|x| x == t).map_or(0, |i| counts[i]);
            self.cond.eval(&count)
        };
        if summons.is_empty() {
            return check(&base);
        }
        summons.iter().any(|&s| {
            let mut counts = base.clone();
            for &t in &self.cards[s as usize].hits {
                counts[t] += 1;
            }
            check(&counts)
        })
    }

    /// Simulates one hand on `deck` (a permutation pool reused between hands).
    fn hand(&self, deck: &mut [u16], rng: &mut Rng, used: &mut [u32]) -> bool {
        let n = deck.len() as u32;
        let mut top = 0u32;
        let mut draw = |deck: &mut [u16], top: &mut u32| -> Option<u16> {
            if *top >= n {
                return None;
            }
            let j = *top + rng.below(n - *top);
            deck.swap(*top as usize, j as usize);
            *top += 1;
            Some(deck[*top as usize - 1])
        };
        let mut hand: Vec<u16> = (0..self.hand).filter_map(|_| draw(deck, &mut top)).collect();
        // Cards that resolved an effect, and names already resolved (once per turn).
        let mut spent = vec![false; hand.len()];
        let mut resolved_names: Vec<u16> = Vec::new();
        let mut progress = true;
        while progress {
            progress = false;
            for i in 0..hand.len() {
                if spent.get(i).copied().unwrap_or(false) {
                    continue;
                }
                let c = hand[i];
                let card = &self.cards[c as usize];
                let Some(effect) = &card.effect else { continue };
                if card.once && resolved_names.contains(&c) {
                    continue;
                }
                let added: Vec<u16> = match effect {
                    Effect::Draw { n: k, banish_top } => {
                        for _ in 0..*banish_top {
                            draw(deck, &mut top);
                        }
                        (0..*k).filter_map(|_| draw(deck, &mut top)).collect()
                    }
                    Effect::Excavate { n: k } => {
                        let seen: Vec<u16> = (0..*k).filter_map(|_| draw(deck, &mut top)).collect();
                        let kept: Vec<u16> = hand.iter().enumerate().filter(|(j, _)| !spent[*j] && *j != i).map(|(_, &x)| x).collect();
                        self.best_of(&seen, &kept).into_iter().collect()
                    }
                    Effect::Search { targets } => {
                        let wanted: Vec<u16> = targets.iter().filter_map(|id| self.index_of(*id)).collect();
                        // Only cards still in the deck can be added.
                        let available: Vec<usize> = (top as usize..deck.len()).filter(|&p| wanted.contains(&deck[p])).collect();
                        if available.is_empty() {
                            continue;
                        }
                        let candidates: Vec<u16> = available.iter().map(|&p| deck[p]).collect();
                        let kept: Vec<u16> = hand.iter().enumerate().filter(|(j, _)| !spent[*j] && *j != i).map(|(_, &x)| x).collect();
                        let pick = self.best_of(&candidates, &kept).unwrap_or(candidates[0]);
                        let p = available[candidates.iter().position(|&x| x == pick).unwrap_or(0)];
                        deck.swap(top as usize, p);
                        top += 1;
                        vec![pick]
                    }
                };
                spent[i] = true;
                resolved_names.push(c);
                used[c as usize] += 1;
                for a in added {
                    hand.push(a);
                    spent.push(false);
                }
                progress = true;
            }
        }
        let counted: Vec<u16> = hand.iter().zip(&spent).filter(|(_, s)| !**s).map(|(&c, _)| c).collect();
        self.satisfied(&counted)
    }

    /// The candidate that best completes `kept`: one that satisfies the
    /// condition if any, else the one counting for the most targets.
    fn best_of(&self, candidates: &[u16], kept: &[u16]) -> Option<u16> {
        let mut with = kept.to_vec();
        with.push(0);
        let last = with.len() - 1;
        for &c in candidates {
            with[last] = c;
            if self.satisfied(&with) {
                return Some(c);
            }
        }
        candidates.iter().copied().max_by_key(|&c| self.cards[c as usize].hits.len())
    }
}

const CHUNK: u64 = 4096;

fn chunk_seed(seed: u64, chunk: u64) -> u64 {
    seed ^ chunk.wrapping_mul(0xD1B5_4A32_D192_ED03).rotate_left(17)
}

/// Runs the simulation, calling `progress` after each batch of chunks.
/// Stops early (marking the result cancelled) when `cancel` becomes true.
pub fn simulate(config: &SimConfig, threads: usize, cancel: &AtomicBool, mut progress: impl FnMut(&Checkpoint)) -> SimResult {
    let prepared = Prepared::new(config);
    let total_chunks = config.hands.div_ceil(CHUNK).max(1);
    let threads = threads.max(1);
    let mut hits = 0u64;
    let mut done = 0u64;
    let mut used = vec![0u64; prepared.cards.len()];
    let mut checkpoints = Vec::new();
    let mut next_chunk = 0u64;
    // Batches grow so the first estimates arrive quickly.
    let mut batch = threads as u64;
    let mut cancelled = false;
    while next_chunk < total_chunks {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let end = (next_chunk + batch).min(total_chunks);
        let chunks: Vec<u64> = (next_chunk..end).collect();
        let results: Vec<(u64, u64, Vec<u32>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let mine: Vec<u64> = chunks.iter().copied().skip(t).step_by(threads).collect();
                    let prepared = &prepared;
                    scope.spawn(move || {
                        let mut deck = prepared.copies.clone();
                        let mut used = vec![0u32; prepared.cards.len()];
                        let (mut h, mut n) = (0u64, 0u64);
                        for chunk in mine {
                            // Each chunk starts from the same deck order: results do not depend on threads.
                            deck.copy_from_slice(&prepared.copies);
                            let mut rng = Rng::new(chunk_seed(config.seed, chunk));
                            let count = CHUNK.min(config.hands.saturating_sub(chunk * CHUNK));
                            for _ in 0..count {
                                h += prepared.hand(&mut deck, &mut rng, &mut used) as u64;
                                n += 1;
                            }
                        }
                        (h, n, used)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("simulation thread")).collect()
        });
        for (h, n, u) in results {
            hits += h;
            done += n;
            for (a, b) in used.iter_mut().zip(u) {
                *a += b as u64;
            }
        }
        let (lo, hi) = wilson(hits, done);
        let cp = Checkpoint { hands: done, p: hits as f64 / done.max(1) as f64, lo, hi };
        checkpoints.push(cp);
        progress(&cp);
        next_chunk = end;
        batch = (batch * 2).min(threads as u64 * 16);
    }
    let (lo, hi) = wilson(hits, done);
    let effects =
        prepared.cards.iter().zip(&used).filter(|(c, _)| c.effect.is_some()).map(|(c, &u)| (c.id, u as f64 / done.max(1) as f64)).collect();
    SimResult { hands: done, hits, p: hits as f64 / done.max(1) as f64, lo, hi, checkpoints, effects, cancelled }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cond::Op;
    use crate::exact::{probability, CardEntry};

    fn deck(entries: &[(u32, u32, &[&str])]) -> DeckModel {
        DeckModel {
            cards: entries
                .iter()
                .map(|(id, copies, roles)| CardEntry { id: *id, copies: *copies, roles: roles.iter().map(|r| r.to_string()).collect() })
                .collect(),
        }
    }

    fn run(config: &SimConfig, threads: usize) -> SimResult {
        simulate(config, threads, &AtomicBool::new(false), |_| {})
    }

    #[test]
    fn agrees_with_the_exact_result_without_effects() {
        let d = deck(&[(1, 13, &["starter"]), (2, 10, &["handtrap"]), (3, 17, &[])]);
        let cond = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]);
        let exact = probability(&d, &cond, 5).unwrap();
        let r = run(&SimConfig { deck: d, cond, hand: 5, rules: vec![], hands: 200_000, seed: 7 }, 4);
        assert_eq!(r.hands, 200_000);
        assert!(r.lo <= exact && exact <= r.hi, "{exact} not in [{}, {}]", r.lo, r.hi);
        assert!(r.checkpoints.len() > 2 && r.checkpoints.last().unwrap().hands == 200_000);
    }

    #[test]
    fn same_seed_same_result_on_any_thread_count() {
        let d = deck(&[(1, 3, &["starter"]), (2, 37, &[])]);
        let config = SimConfig { deck: d, cond: Cond::role("starter", Op::AtLeast, 1), hand: 5, rules: vec![], hands: 50_000, seed: 42 };
        assert_eq!(run(&config, 1).hits, run(&config, 3).hits);
    }

    #[test]
    fn searchers_and_draw_spells_raise_the_odds() {
        // 3 starters; 3 searchers that fetch a starter; 2 Upstart Goblin.
        let d = deck(&[(1, 3, &["starter"]), (2, 3, &[]), (3, 2, &[]), (4, 32, &[])]);
        let cond = Cond::role("starter", Op::AtLeast, 1);
        let base = probability(&d, &cond, 5).unwrap();
        let rules = vec![
            CardRule { id: 2, effect: Some(Effect::Search { targets: vec![1] }), ..Default::default() },
            CardRule { id: 3, effect: Some(Effect::Draw { n: 1, banish_top: 0 }), ..Default::default() },
        ];
        let r = run(&SimConfig { deck: d.clone(), cond: cond.clone(), hand: 5, rules, hands: 100_000, seed: 1 }, 2);
        // Searchers act as extra starters: at least the odds of 6 starters.
        let six = probability(&deck(&[(1, 6, &["starter"]), (4, 34, &[])]), &cond, 5).unwrap();
        assert!(r.p > six - 0.01 && r.p > base + 0.2, "{} vs base {base}, six {six}", r.p);
        assert!(r.effects.iter().any(|(id, f)| *id == 2 && *f > 0.0));
    }

    #[test]
    fn searched_cards_must_still_be_in_the_deck() {
        // One starter, one searcher for it: holding both never gives two starters.
        let d = deck(&[(1, 1, &["starter"]), (2, 1, &[]), (3, 3, &[])]);
        let rules = vec![CardRule { id: 2, effect: Some(Effect::Search { targets: vec![1] }), ..Default::default() }];
        let r = run(&SimConfig { deck: d, cond: Cond::role("starter", Op::AtLeast, 2), hand: 5, rules, hands: 5_000, seed: 3 }, 1);
        assert_eq!(r.hits, 0);
    }

    #[test]
    fn once_per_turn_and_normal_summon_limits() {
        // Hand always holds two copies of A and one B (5 cards, all drawn).
        let d = deck(&[(1, 2, &["starter"]), (2, 1, &["starter"]), (3, 2, &[])]);
        let two = Cond::role("starter", Op::AtLeast, 2);
        let config = |rules| SimConfig { deck: d.clone(), cond: two.clone(), hand: 5, rules, hands: 200, seed: 0 };
        assert_eq!(run(&config(vec![]), 1).hits, 200);
        // A once per turn: A + B still make two.
        assert_eq!(run(&config(vec![CardRule { id: 1, once_per_turn: true, ..Default::default() }]), 1).hits, 200);
        // A and B both need the Normal Summon: only one counts.
        let ns = vec![
            CardRule { id: 1, normal_summon: true, ..Default::default() },
            CardRule { id: 2, normal_summon: true, ..Default::default() },
        ];
        assert_eq!(run(&config(ns), 1).hits, 0);
    }

    #[test]
    fn wilson_interval() {
        let (lo, hi) = wilson(59, 102);
        assert!((lo - 0.4815).abs() < 1e-3 && (hi - 0.6697).abs() < 1e-3, "{lo} {hi}");
    }
}

#[cfg(test)]
mod speed {
    use super::*;
    use crate::cond::Op;
    use crate::exact::CardEntry;

    /// `cargo test -p ygo-stats --release -- --ignored --nocapture speed`
    #[test]
    #[ignore]
    fn million_hands() {
        let roles = |r: &str| if r.is_empty() { vec![] } else { vec![r.to_string()] };
        let deck = DeckModel {
            cards: [(1, 13, "starter"), (2, 11, "extender"), (3, 10, "handtrap"), (4, 3, ""), (5, 3, "")]
                .iter()
                .map(|(id, c, r)| CardEntry { id: *id, copies: *c, roles: roles(r) })
                .collect(),
        };
        let cond = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]);
        let rules =
            vec![CardRule { id: 4, effect: Some(Effect::Draw { n: 2, banish_top: 10 }), once_per_turn: true, ..Default::default() }];
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let start = std::time::Instant::now();
        let r = simulate(&SimConfig { deck, cond, hand: 5, rules, hands: 1_000_000, seed: 1 }, threads, &AtomicBool::new(false), |_| {});
        println!("{} hands on {threads} threads in {:?}: p = {:.4}", r.hands, start.elapsed(), r.p);
    }
}
