//! Monte Carlo simulation of opening hands with cards that change the deck:
//! draw spells (Pot of Desires, Upstart Goblin…), excavators (Pot of
//! Prosperity, Pot of Duality) and searchers ("Magical Meltdown adds Aleister").
//!
//! A card that resolves its effect stops counting for its own roles: it
//! has turned into what it drew or fetched. Cards can also be limited to
//! one copy per turn, and cards that need the Normal Summon count one at a
//! time (the best one for the condition): resolving the effect of one of
//! them uses the summon too.
//!
//! Hands are simulated in fixed chunks, each with its own seed derived from
//! the run's seed, so a run gives the same result on any number of threads.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::cond::{Cond, Op, Target};
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
    /// Looks at the top `n` cards and adds the best one (Pot of Prosperity,
    /// Pot of Duality); the others go to the bottom of the deck.
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
    /// Per card with an effect: share of hands where it resolved.
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
    /// How much the card helps when nothing completes the hand: targets the
    /// condition wants more of, minus those it wants fewer of.
    worth: i32,
    effect: Option<Effect>,
    /// What a search can add, as indices into `cards`.
    search: Vec<u16>,
    once: bool,
    normal_summon: bool,
}

/// Sign of each target in the condition: +1 when more copies help (at least,
/// or at most under a "not"), -1 when they hurt, 0 when both or exactly.
fn polarity(cond: &Cond, sign: i32, targets: &[Target], out: &mut [Option<i32>]) {
    match cond {
        Cond::All { items } | Cond::Any { items } => items.iter().for_each(|c| polarity(c, sign, targets, out)),
        Cond::Not { item } => polarity(item, -sign, targets, out),
        Cond::Count { target, op, .. } => {
            let s = match op {
                Op::AtLeast => sign,
                Op::AtMost => -sign,
                Op::Exactly => 0,
            };
            if let Some(i) = targets.iter().position(|t| t == target) {
                out[i] = Some(if out[i].is_none_or(|x| x == s) { s } else { 0 });
            }
        }
    }
}

/// One hand being played. The deck is split in place: `deck[..top]` has
/// left it, `deck[top..bottom]` can be drawn, and `deck[bottom..]` was put
/// at the bottom (excavated, not added): it can be searched, not drawn.
#[derive(Clone, Default)]
struct Turn {
    hand: Vec<u16>,
    /// Cards of `hand` that resolved their effect.
    spent: Vec<bool>,
    /// Cards whose effect resolved, each once.
    resolved: Vec<u16>,
    /// Normal Summon cards whose effect the player chose not to use.
    declined: Vec<u16>,
    summon_used: bool,
    top: usize,
    bottom: usize,
}

impl Turn {
    fn draw(&mut self, deck: &mut [u16], rng: &mut Rng) -> Option<u16> {
        if self.top >= self.bottom {
            return None;
        }
        let j = self.top + rng.below((self.bottom - self.top) as u32) as usize;
        deck.swap(self.top, j);
        self.top += 1;
        Some(deck[self.top - 1])
    }

    /// Takes the card at `p` (to be drawn or at the bottom) out of the deck.
    fn take(&mut self, deck: &mut [u16], p: usize) {
        if p >= self.bottom {
            deck.swap(p, self.bottom);
            self.bottom += 1;
            deck.swap(self.bottom - 1, self.top);
        } else {
            deck.swap(p, self.top);
        }
        self.top += 1;
    }

    /// Puts the card at `p`, out of the deck, back at the bottom.
    fn put_at_bottom(&mut self, deck: &mut [u16], p: usize) {
        self.top -= 1;
        deck.swap(p, self.top);
        self.bottom -= 1;
        deck.swap(self.top, self.bottom);
    }

    /// Cards in hand that still count (they did not resolve an effect).
    fn kept(&self) -> Vec<u16> {
        self.hand.iter().zip(&self.spent).filter(|(_, s)| !**s).map(|(&c, _)| c).collect()
    }
}

impl<'a> Prepared<'a> {
    fn new(config: &'a SimConfig) -> Self {
        let mut set = BTreeSet::new();
        config.cond.targets(&mut set);
        let targets: Vec<Target> = set.into_iter().collect();
        let mut signs = vec![None; targets.len()];
        polarity(&config.cond, 1, &targets, &mut signs);
        let mut cards: Vec<PreparedCard> = config
            .deck
            .cards
            .iter()
            .map(|c| {
                let rule = config.rules.iter().find(|r| r.id == c.id);
                let hits: Vec<usize> = targets
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| match t {
                        Target::Role(r) => c.roles.contains(r),
                        Target::Card(id) => c.id == *id,
                    })
                    .map(|(i, _)| i)
                    .collect();
                PreparedCard {
                    id: c.id,
                    worth: hits.iter().map(|&t| signs[t].unwrap_or(0)).sum(),
                    hits,
                    effect: rule.and_then(|r| r.effect.clone()),
                    search: Vec::new(),
                    once: rule.is_some_and(|r| r.once_per_turn),
                    normal_summon: rule.is_some_and(|r| r.normal_summon),
                }
            })
            .collect();
        let ids: Vec<u32> = cards.iter().map(|c| c.id).collect();
        for card in &mut cards {
            if let Some(Effect::Search { targets }) = &card.effect {
                card.search = targets.iter().filter_map(|id| ids.iter().position(|x| x == id)).map(|i| i as u16).collect();
            }
        }
        let copies = config.deck.cards.iter().enumerate().flat_map(|(i, c)| std::iter::repeat_n(i as u16, c.copies as usize)).collect();
        Self { cond: &config.cond, targets, copies, cards, hand: config.hand }
    }

    /// Whether the counted cards satisfy the condition, letting the player
    /// pick which Normal Summon card to count. `resolving` is a card whose
    /// effect is about to resolve: its name and the summon are taken.
    fn satisfied(&self, counted: &[u16], turn: &Turn, resolving: Option<u16>) -> bool {
        let summon_used = turn.summon_used || resolving.is_some_and(|c| self.cards[c as usize].normal_summon);
        let mut base = vec![0u32; self.targets.len()];
        // A once per turn card that resolved has no copy left to count.
        let mut seen: Vec<u16> = turn.resolved.iter().chain(&resolving).copied().filter(|&c| self.cards[c as usize].once).collect();
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
                if !summon_used && !summons.contains(&c) {
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

    /// Whether `c`, in hand, could still resolve its effect.
    fn can_resolve(&self, turn: &Turn, c: u16) -> bool {
        let card = &self.cards[c as usize];
        card.effect.is_some()
            && !(card.once && turn.resolved.contains(&c))
            && !(card.normal_summon && (turn.summon_used || turn.declined.contains(&c)))
    }

    /// Where in the deck are the cards that `c` can search (to be drawn or at the bottom).
    fn searchable(&self, turn: &Turn, deck: &[u16], c: u16) -> Vec<usize> {
        let wanted = &self.cards[c as usize].search;
        (turn.top..deck.len()).filter(|&p| wanted.contains(&deck[p])).collect()
    }

    /// The candidate that completes `kept`, if any.
    fn completing(&self, candidates: &[u16], kept: &[u16], turn: &Turn, resolving: Option<u16>) -> Option<u16> {
        let mut with = kept.to_vec();
        with.push(0);
        let last = with.len() - 1;
        candidates.iter().copied().find(|&c| {
            with[last] = c;
            self.satisfied(&with, turn, resolving)
        })
    }

    /// The candidate that best completes `kept`: one that satisfies the
    /// condition if any, else the one that helps most (never one the
    /// condition wants fewer of when another does no harm), one whose
    /// effect can still resolve on ties.
    fn best_of(&self, candidates: &[u16], kept: &[u16], turn: &Turn, resolving: Option<u16>) -> Option<u16> {
        self.completing(candidates, kept, turn, resolving)
            .or_else(|| candidates.iter().copied().max_by_key(|&c| (self.cards[c as usize].worth, self.can_resolve(turn, c))))
    }

    /// The effect to resolve next, in the order a player would pick. A
    /// search whose pick is already clear goes first, as draws and
    /// excavations could banish or take its target; then draws, then
    /// excavations; other searches wait to pick knowing what those gave.
    fn next_effect(&self, turn: &Turn, deck: &[u16]) -> Option<usize> {
        let (mut draw, mut dig, mut search) = (None, None, None);
        for (i, &c) in turn.hand.iter().enumerate() {
            if turn.spent[i] || !self.can_resolve(turn, c) {
                continue;
            }
            match &self.cards[c as usize].effect {
                Some(Effect::Draw { .. }) => {
                    draw.get_or_insert(i);
                }
                Some(Effect::Excavate { .. }) => {
                    dig.get_or_insert(i);
                }
                Some(Effect::Search { .. }) => {
                    let found: Vec<u16> = self.searchable(turn, deck, c).into_iter().map(|p| deck[p]).collect();
                    let Some(&first) = found.first() else { continue };
                    let mut kept = turn.kept();
                    kept.remove(kept.iter().position(|&x| x == c).expect("the searcher is in hand"));
                    if found.iter().all(|&x| x == first) || self.completing(&found, &kept, turn, Some(c)).is_some() {
                        return Some(i);
                    }
                    search.get_or_insert(i);
                }
                None => {}
            }
        }
        draw.or(dig).or(search)
    }

    /// Resolves the effect of `turn.hand[i]`.
    fn resolve(&self, turn: &mut Turn, i: usize, deck: &mut [u16], rng: &mut Rng) {
        let c = turn.hand[i];
        let card = &self.cards[c as usize];
        turn.spent[i] = true;
        if !turn.resolved.contains(&c) {
            turn.resolved.push(c);
        }
        turn.summon_used |= card.normal_summon;
        let added: Option<u16> = match card.effect.as_ref().expect("a card with an effect") {
            Effect::Draw { n: k, banish_top } => {
                for _ in 0..*banish_top {
                    turn.draw(deck, rng);
                }
                for _ in 0..*k {
                    if let Some(a) = turn.draw(deck, rng) {
                        turn.hand.push(a);
                        turn.spent.push(false);
                    }
                }
                None
            }
            Effect::Excavate { n: k } => {
                let start = turn.top;
                let seen: Vec<u16> = (0..*k).filter_map(|_| turn.draw(deck, rng)).collect();
                let pick = self.best_of(&seen, &turn.kept(), turn, Some(c));
                if let Some(pick) = pick {
                    // The others go to the bottom: still in the deck for a search.
                    deck.swap(start, start + seen.iter().position(|&x| x == pick).unwrap_or(0));
                    for _ in 1..seen.len() {
                        let p = turn.top - 1;
                        turn.put_at_bottom(deck, p);
                    }
                }
                pick
            }
            Effect::Search { .. } => {
                // Only cards still in the deck can be added.
                let found = self.searchable(turn, deck, c);
                let candidates: Vec<u16> = found.iter().map(|&p| deck[p]).collect();
                let pick = self.best_of(&candidates, &turn.kept(), turn, Some(c));
                if let Some(pick) = pick {
                    turn.take(deck, found[candidates.iter().position(|&x| x == pick).unwrap_or(0)]);
                }
                pick
            }
        };
        if let Some(a) = added {
            turn.hand.push(a);
            turn.spent.push(false);
        }
    }

    /// Resolves the effects in hand and tells whether the hand ends up
    /// meeting the condition. When the effect of a Normal Summon card could
    /// resolve, the player takes the better of using the summon on it or
    /// keeping it (for another card).
    fn play(&self, mut turn: Turn, deck: &mut [u16], rng: &mut Rng) -> (bool, Turn) {
        while let Some(i) = self.next_effect(&turn, deck) {
            let c = turn.hand[i];
            if self.cards[c as usize].normal_summon {
                let (mut other_deck, mut other_rng) = (deck.to_vec(), rng.clone());
                let mut keep = turn.clone();
                keep.declined.push(c);
                let (kept_ok, kept_turn) = self.play(keep, &mut other_deck, &mut other_rng);
                self.resolve(&mut turn, i, deck, rng);
                let (ok, turn) = self.play(turn, deck, rng);
                if ok || !kept_ok {
                    return (ok, turn);
                }
                deck.copy_from_slice(&other_deck);
                *rng = other_rng;
                return (kept_ok, kept_turn);
            }
            self.resolve(&mut turn, i, deck, rng);
        }
        (self.satisfied(&turn.kept(), &turn, None), turn)
    }

    /// Simulates one hand on `deck` (a permutation pool reused between hands).
    fn hand(&self, deck: &mut [u16], rng: &mut Rng, used: &mut [u32]) -> bool {
        let mut turn = Turn { bottom: deck.len(), ..Turn::default() };
        turn.hand = (0..self.hand).filter_map(|_| turn.draw(deck, rng)).collect();
        turn.spent = vec![false; turn.hand.len()];
        let (ok, turn) = self.play(turn, deck, rng);
        // A card counts once per hand, however many copies resolved.
        for &c in &turn.resolved {
            used[c as usize] += 1;
        }
        ok
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
    fn one_normal_summon_per_hand_even_for_effects() {
        let ns =
            |id, targets: Vec<u32>| CardRule { id, effect: Some(Effect::Search { targets }), normal_summon: true, ..Default::default() };
        let two = Cond::role("starter", Op::AtLeast, 2);
        // A and B search a Starter when Normal Summoned: of the six hands only {A, B} fails.
        let d = deck(&[(1, 1, &[]), (2, 1, &[]), (3, 1, &["starter"]), (4, 1, &["starter"])]);
        let rules = vec![ns(1, vec![3, 4]), ns(2, vec![3, 4])];
        let r = run(&SimConfig { deck: d, cond: two.clone(), hand: 2, rules, hands: 30_000, seed: 9 }, 2);
        assert!((r.p - 5.0 / 6.0).abs() < 0.015, "{}", r.p);
        // A searcher and a Starter that both need the summon: one or the other, so only {N, C} works.
        let d = deck(&[(1, 1, &[]), (2, 1, &["starter"]), (3, 1, &["starter"])]);
        let rules = vec![ns(1, vec![3]), CardRule { id: 2, normal_summon: true, ..Default::default() }];
        let r = run(&SimConfig { deck: d, cond: two, hand: 2, rules, hands: 30_000, seed: 10 }, 2);
        assert!((r.p - 1.0 / 3.0).abs() < 0.015, "{}", r.p);
        // 3 + 3 summoned searchers for a Starter and an Extender: 0.247812 by exact enumeration.
        let d = deck(&[(1, 3, &[]), (2, 3, &[]), (3, 3, &["starter"]), (4, 3, &["extender"]), (5, 28, &[])]);
        let cond = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::role("extender", Op::AtLeast, 1)]);
        let r = run(&SimConfig { deck: d, cond, hand: 5, rules: vec![ns(1, vec![3]), ns(2, vec![4])], hands: 400_000, seed: 2024 }, 4);
        assert!(r.lo <= 0.247_812 && 0.247_812 <= r.hi, "{} [{}, {}]", r.p, r.lo, r.hi);
    }

    #[test]
    fn effects_resolve_in_the_order_a_player_would_pick() {
        // The search goes before Pot of Desires, which would banish its only target.
        let d = deck(&[(1, 1, &[]), (2, 1, &[]), (3, 1, &["starter"])]);
        let rules = vec![
            CardRule { id: 1, effect: Some(Effect::Draw { n: 2, banish_top: 10 }), once_per_turn: true, ..Default::default() },
            CardRule { id: 2, effect: Some(Effect::Search { targets: vec![3] }), ..Default::default() },
        ];
        let r = run(&SimConfig { deck: d, cond: Cond::role("starter", Op::AtLeast, 1), hand: 2, rules, hands: 3_000, seed: 4 }, 1);
        assert_eq!(r.hits, r.hands);
        // A search for either of two cards waits for the excavation, then takes
        // the one it left at the bottom of the deck.
        let d = deck(&[(1, 1, &[]), (2, 1, &[]), (3, 1, &[]), (4, 1, &[])]);
        let rules = vec![
            CardRule { id: 1, effect: Some(Effect::Excavate { n: 2 }), ..Default::default() },
            CardRule { id: 2, effect: Some(Effect::Search { targets: vec![3, 4] }), ..Default::default() },
        ];
        let both = Cond::all([Cond::card(3, Op::AtLeast, 1), Cond::card(4, Op::AtLeast, 1)]);
        let r = run(&SimConfig { deck: d, cond: both, hand: 2, rules, hands: 3_000, seed: 5 }, 1);
        assert_eq!(r.hits, r.hands);
    }

    #[test]
    fn a_once_per_turn_card_that_resolved_counts_no_more() {
        // S, a Starter, fetches T once per turn: S + S gives one Starter, S + T two (2/3).
        let d = deck(&[(1, 2, &["starter"]), (2, 1, &["starter"])]);
        let rules = vec![CardRule { id: 1, effect: Some(Effect::Search { targets: vec![2] }), once_per_turn: true, ..Default::default() }];
        let r = run(&SimConfig { deck: d, cond: Cond::role("starter", Op::AtLeast, 2), hand: 2, rules, hands: 30_000, seed: 6 }, 2);
        assert!((r.p - 2.0 / 3.0).abs() < 0.015, "{}", r.p);
    }

    #[test]
    fn effects_are_shares_of_hands() {
        // Both copies of X resolve in {X, X}: still one hand. X resolves in 5 hands of 6.
        let d = deck(&[(1, 2, &[]), (2, 2, &["starter"])]);
        let rules = vec![CardRule { id: 1, effect: Some(Effect::Search { targets: vec![2] }), ..Default::default() }];
        let r = run(&SimConfig { deck: d, cond: Cond::role("starter", Op::AtLeast, 1), hand: 2, rules, hands: 30_000, seed: 8 }, 2);
        let share = r.effects.iter().find(|(id, _)| *id == 1).unwrap().1;
        assert!((share - 5.0 / 6.0).abs() < 0.015, "{share}");
    }

    #[test]
    fn excavations_avoid_cards_the_condition_wants_fewer_of() {
        // One-card hands. E excavates 2, U draws 1; with S a Starter and B a Brick, E
        // seeing U and B adds U, which draws S: E always works, U in 2 cases of 3 (2/3 in all).
        let rules = vec![
            CardRule { id: 1, effect: Some(Effect::Excavate { n: 2 }), once_per_turn: true, ..Default::default() },
            CardRule { id: 2, effect: Some(Effect::Draw { n: 1, banish_top: 0 }), ..Default::default() },
        ];
        let no_brick = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::negate(Cond::role("brick", Op::AtLeast, 1))]);
        let d = deck(&[(1, 1, &[]), (2, 1, &[]), (3, 1, &["brick"]), (4, 1, &["starter"])]);
        let r = run(&SimConfig { deck: d, cond: no_brick, hand: 1, rules: rules.clone(), hands: 60_000, seed: 12 }, 2);
        assert!((r.p - 2.0 / 3.0).abs() < 0.01, "{}", r.p);
        // On a tie, the card whose effect can still resolve: U rather than a card without roles.
        let d = deck(&[(1, 1, &[]), (2, 1, &[]), (3, 1, &[]), (4, 1, &["starter"])]);
        let r = run(&SimConfig { deck: d, cond: Cond::role("starter", Op::AtLeast, 1), hand: 1, rules, hands: 60_000, seed: 13 }, 2);
        assert!((r.p - 2.0 / 3.0).abs() < 0.01, "{}", r.p);
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
