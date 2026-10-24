//! The Monte Carlo played by the engine (PIANO §7.4): hands drawn from the
//! deck, each searched by the Combo Lab, and the value of the best board of
//! each. What it measures is "P(board ≥ threshold)", not an estimate from
//! the roles.
//!
//! Two things keep it affordable when the deck changes by a copy:
//! - **Common random numbers.** Hands are drawn as positions of the Main
//!   Deck, and a variant of the deck keeps every card that stays in its
//!   position ([`respell`]). Changing a copy changes only the hands that
//!   hold that position, and the difference between two decks is measured
//!   on the same draws, with far less noise than two independent samples.
//! - **A cache by the cards that matter.** A card that was in a hand
//!   searched, and that no line of any hand searched so far has moved out
//!   of the hand, counts as blank: two hands that differ only in such cards
//!   get the same value. A card never searched yet is not blank. As cards
//!   become live the keys get finer, so a hand never takes the value of a
//!   hand it differs from in a card some line used.
//!
//! - **Control variates.** Whether a hand holds each card is known
//!   exactly before any search (hypergeometric odds), and it explains much
//!   of whether the hand reaches a board. The regression estimator
//!   ([`adjusted`]) corrects the share of the sample by how far the
//!   sample's cards are from their exact odds, and its interval is that of
//!   what the cards do not explain: narrower for the same hands.
//!
//! The value of a hand also depends on the rest of the deck (what a card
//! searches, what is left to draw); the cache leaves that out, as the
//! exact calculation of the roles does.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::card::Rng;
use crate::search::Board;

/// What one hand reached.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// Value of the best board (0 when nothing was reached or measured).
    pub value: f64,
    /// Cards of the hand the value depends on, sorted, with repeats: those
    /// the best board's line moved out of the hand ([`used_cards`]) and the
    /// handtraps, which count kept in the hand.
    pub used: Vec<u32>,
    pub nodes: u64,
    pub seconds: f64,
}

/// Up to `count` draws of `hand` positions out of a Main Deck of `deck_len`
/// cards, from `seed`. The same seed gives the same draws for every deck
/// of that size.
pub fn draw_positions(deck_len: usize, hand: usize, count: usize, seed: u64) -> Vec<Vec<usize>> {
    let mut rng = Rng(seed ^ 0x3c6e_f372_fe94_f82b);
    let hand = hand.min(deck_len);
    (0..count)
        .map(|_| {
            // A partial Fisher-Yates shuffle: the first `hand` cards.
            let mut order: Vec<usize> = (0..deck_len).collect();
            for i in 0..hand {
                let j = i + rng.below(deck_len - i);
                order.swap(i, j);
            }
            let mut drawn = order[..hand].to_vec();
            drawn.sort_unstable();
            drawn
        })
        .collect()
}

/// The hands of `main` at `draws`, each sorted.
pub fn hands_at(main: &[u32], draws: &[Vec<usize>]) -> Vec<Vec<u32>> {
    draws
        .iter()
        .map(|d| {
            let mut hand: Vec<u32> = d.iter().filter_map(|&i| main.get(i).copied()).collect();
            hand.sort_unstable();
            hand
        })
        .collect()
}

/// `base` with the copies of `counts` (cards missing there: none), keeping
/// every card that stays in its position: a card with fewer copies leaves
/// its last positions, and the new copies take them, in the order of
/// `counts`. Cards of `counts` not in `base` join; the size is the sum of
/// `counts` (positions past `base` are added at the end).
pub fn respell(base: &[u32], counts: &BTreeMap<u32, u32>) -> Vec<u32> {
    let mut seen: HashMap<u32, u32> = HashMap::new();
    let mut out: Vec<Option<u32>> = base
        .iter()
        .map(|&c| {
            let n = seen.entry(c).or_default();
            *n += 1;
            (*n <= counts.get(&c).copied().unwrap_or(0)).then_some(c)
        })
        .collect();
    let mut joining = Vec::new();
    for (&c, &want) in counts {
        let have = seen.get(&c).copied().unwrap_or(0).min(want);
        joining.extend(std::iter::repeat_n(c, (want - have) as usize));
    }
    let mut joining = joining.into_iter();
    for slot in out.iter_mut().filter(|s| s.is_none()) {
        *slot = joining.next();
    }
    out.extend(joining.map(Some));
    out.into_iter().flatten().collect()
}

/// Cards of `hand` that the boards moved out of the hand (played, sent as
/// a cost, banished, shuffled back...): the most copies of each that any
/// of them took. A card only revealed, or that came back to the hand, does
/// not count.
pub fn used_cards(hand: &[u32], boards: &[Board]) -> Vec<u32> {
    let mut start: BTreeMap<u32, usize> = BTreeMap::new();
    for &c in hand {
        *start.entry(c).or_default() += 1;
    }
    let mut used: BTreeMap<u32, usize> = BTreeMap::new();
    for board in boards {
        let end = &board.state.players[0].hand;
        for (&c, &n) in &start {
            let left = n.saturating_sub(end.iter().filter(|&&x| x == c).count());
            let u = used.entry(c).or_default();
            *u = (*u).max(left);
        }
    }
    used.into_iter().flat_map(|(c, n)| std::iter::repeat_n(c, n)).collect()
}

/// The values of the hands searched, by the cards that matter (see the
/// module's comment).
#[derive(Debug, Default)]
pub struct HandCache {
    searched: Vec<(Vec<u32>, Outcome)>,
    /// Cards of the hands searched, and those some line moved out of the hand.
    seen: HashSet<u32>,
    live: HashSet<u32>,
    index: HashMap<Vec<u32>, usize>,
    /// Lookups answered from the cache, and hands searched.
    pub hits: usize,
    pub misses: usize,
}

/// Stands for a card searched that no line used, in a key.
const BLANK: u32 = 0;

impl HandCache {
    /// `hand` with the cards searched that no line used as [`BLANK`], sorted.
    pub fn key(&self, hand: &[u32]) -> Vec<u32> {
        let blank = |c: &u32| self.seen.contains(c) && !self.live.contains(c);
        let mut key: Vec<u32> = hand.iter().map(|c| if blank(c) { BLANK } else { *c }).collect();
        key.sort_unstable();
        key
    }

    pub fn get(&self, hand: &[u32]) -> Option<&Outcome> {
        self.index.get(&self.key(hand)).map(|&i| &self.searched[i].1)
    }

    /// Cards some line used so far.
    pub fn live(&self) -> &HashSet<u32> {
        &self.live
    }

    pub fn len(&self) -> usize {
        self.searched.len()
    }

    pub fn is_empty(&self) -> bool {
        self.searched.is_empty()
    }

    pub fn insert(&mut self, hand: Vec<u32>, outcome: Outcome) {
        let before = (self.seen.len(), self.live.len());
        self.seen.extend(hand.iter().copied());
        self.live.extend(outcome.used.iter().copied());
        self.searched.push((hand, outcome));
        if (self.seen.len(), self.live.len()) != before {
            // Other keys: index every hand again (the first one of a key wins).
            self.index.clear();
            for i in 0..self.searched.len() {
                let key = self.key(&self.searched[i].0);
                self.index.entry(key).or_insert(i);
            }
        } else {
            let i = self.searched.len() - 1;
            let key = self.key(&self.searched[i].0);
            self.index.entry(key).or_insert(i);
        }
    }

    /// The value of each of `hands`, searching with `play` (on `threads`
    /// threads) the hands the cache cannot answer. Hands that share a key
    /// are searched once; when a search makes a card live, the hands that
    /// shared a key with it are looked up again, and searched if their key
    /// is now new. The result does not depend on the threads.
    pub fn values(&mut self, hands: &[Vec<u32>], threads: usize, play: impl Fn(&[u32]) -> Outcome + Sync) -> Vec<Outcome> {
        let mut searched = 0;
        loop {
            let mut todo: Vec<&Vec<u32>> = Vec::new();
            let mut keys = HashSet::new();
            for hand in hands {
                if self.get(hand).is_none() && keys.insert(self.key(hand)) {
                    todo.push(hand);
                }
            }
            if todo.is_empty() {
                break;
            }
            let next = AtomicUsize::new(0);
            let done: Mutex<Vec<Option<Outcome>>> = Mutex::new(vec![None; todo.len()]);
            std::thread::scope(|s| {
                for _ in 0..threads.clamp(1, todo.len()) {
                    s.spawn(|| loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(hand) = todo.get(i) else { break };
                        let outcome = play(hand);
                        if let Ok(mut d) = done.lock() {
                            d[i] = Some(outcome);
                        }
                    });
                }
            });
            let done = done.into_inner().unwrap_or_default();
            searched += todo.len();
            for (hand, outcome) in todo.into_iter().zip(done) {
                self.insert(hand.clone(), outcome.unwrap_or(Outcome { value: 0.0, used: Vec::new(), nodes: 0, seconds: 0.0 }));
            }
        }
        self.misses += searched;
        self.hits += hands.len().saturating_sub(searched);
        hands.iter().filter_map(|h| self.get(h).cloned()).collect()
    }
}

/// Wilson's 95% interval of a share: `hits` of `n`.
pub fn wilson(hits: usize, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    const Z: f64 = 1.959_964;
    let (n, p) = (n as f64, hits as f64 / n as f64);
    let denom = 1.0 + Z * Z / n;
    let centre = (p + Z * Z / (2.0 * n)) / denom;
    let half = Z * (p * (1.0 - p) / n + Z * Z / (4.0 * n * n)).sqrt() / denom;
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

/// What N hands say about a deck.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub hands: usize,
    /// Share of the hands whose best board is worth at least the threshold,
    /// with its 95% interval (Wilson).
    pub p: f64,
    pub low: f64,
    pub high: f64,
    /// Mean value of the best board, with its 95% interval (normal).
    pub mean: f64,
    pub mean_half: f64,
    /// The share corrected by the control variates, with the half width of
    /// its 95% interval ([`adjusted`]).
    pub p_adjusted: f64,
    pub p_adjusted_half: f64,
}

/// `controls`: the control variates of each hand and their exact means
/// ([`presence`]); empty for none.
pub fn estimate(values: &[f64], threshold: f64, controls: &Controls) -> Estimate {
    let n = values.len();
    let hits = values.iter().filter(|&&v| v >= threshold).count();
    let (low, high) = wilson(hits, n);
    let (mean, sd) = mean_sd(values);
    let indicators: Vec<f64> = values.iter().map(|&v| if v >= threshold { 1.0 } else { 0.0 }).collect();
    let (p_adjusted, p_adjusted_half) = adjusted(&indicators, controls);
    Estimate {
        hands: n,
        p: hits as f64 / n.max(1) as f64,
        low,
        high,
        mean,
        mean_half: 1.959_964 * sd / (n.max(1) as f64).sqrt(),
        p_adjusted: p_adjusted.clamp(0.0, 1.0),
        p_adjusted_half,
    }
}

/// Control variates: one row per hand, and the exact mean of each column.
#[derive(Debug, Clone, Default)]
pub struct Controls {
    pub rows: Vec<Vec<f64>>,
    pub means: Vec<f64>,
}

/// Whether each hand holds each of the `cards` of `main`, and the exact
/// odds of it in `hand` cards; cards in every hand or in none are left out.
pub fn presence(main: &[u32], hands: &[Vec<u32>], hand: usize) -> Controls {
    let mut cards: Vec<u32> = main.to_vec();
    cards.sort_unstable();
    cards.dedup();
    let size = main.len();
    // P(no copy among `hand` cards) = C(size - k, hand) / C(size, hand).
    let none = |k: usize| (0..hand).map(|i| (size - k).saturating_sub(i) as f64 / (size - i) as f64).product::<f64>();
    let odds: Vec<(u32, f64)> = cards
        .into_iter()
        .map(|c| (c, 1.0 - none(main.iter().filter(|&&x| x == c).count())))
        .filter(|&(_, p)| p > 1e-12 && p < 1.0 - 1e-12)
        .collect();
    let rows = hands.iter().map(|h| odds.iter().map(|&(c, _)| if h.contains(&c) { 1.0 } else { 0.0 }).collect()).collect();
    Controls { rows, means: odds.into_iter().map(|(_, p)| p).collect() }
}

/// The regression estimator of the mean of `y`: the least-squares fit of
/// `y` on the control variates, centred on their exact means, read at
/// those means. It is the sample mean moved by how far the sample's
/// controls are from their means. The half width of the 95% interval
/// comes from the residuals and the fit's own uncertainty; with
/// too few hands for the controls it falls back to the plain mean.
pub fn adjusted(y: &[f64], controls: &Controls) -> (f64, f64) {
    let n = y.len();
    let k = controls.means.len();
    let (mean, sd) = mean_sd(y);
    let plain = (mean, 1.959_964 * sd / (n.max(1) as f64).sqrt());
    if k == 0 || n < 2 * (k + 1) || controls.rows.len() != n {
        return plain;
    }
    // Normal equations of y = a + b·(x - μ), with a small ridge on b.
    let dim = k + 1;
    let mut a = vec![vec![0.0; dim]; dim];
    let mut rhs = vec![0.0; dim];
    let centred = |row: &[f64]| -> Vec<f64> { std::iter::once(1.0).chain(row.iter().zip(&controls.means).map(|(x, m)| x - m)).collect() };
    for (row, &yi) in controls.rows.iter().zip(y) {
        let x = centred(row);
        for i in 0..dim {
            rhs[i] += x[i] * yi;
            for j in 0..dim {
                a[i][j] += x[i] * x[j];
            }
        }
    }
    for (i, r) in a.iter_mut().enumerate().skip(1) {
        r[i] += 1e-6 * n as f64;
    }
    let mut e0 = vec![0.0; dim];
    e0[0] = 1.0;
    let (Some(beta), Some(inverse)) = (solve(a.clone(), rhs), solve(a, e0)) else { return plain };
    let sse: f64 = controls
        .rows
        .iter()
        .zip(y)
        .map(|(row, &yi)| {
            let fit: f64 = centred(row).iter().zip(&beta).map(|(x, b)| x * b).sum();
            (yi - fit).powi(2)
        })
        .sum();
    // Var(a) = σ² [(XᵀX)⁻¹]₀₀: the residual variance, and what fitting b adds.
    let df = (n - dim) as f64;
    let half = 1.959_964 * (sse / df * inverse[0].max(0.0)).sqrt();
    (beta[0], half)
}

/// Gaussian elimination with partial pivoting; `None` for a singular system.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let (top, rest) = a.split_at_mut(col + 1);
        let pivot_row = &top[col];
        for (i, r) in rest.iter_mut().enumerate() {
            let f = r[col] / pivot_row[col];
            for (x, p) in r[col..].iter_mut().zip(&pivot_row[col..]) {
                *x -= f * p;
            }
            b[col + 1 + i] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let s: f64 = (row + 1..n).map(|j| a[row][j] * x[j]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Some(x)
}

fn mean_sd(values: &[f64]) -> (f64, f64) {
    let n = values.len().max(1) as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    (mean, var.sqrt())
}

/// The difference between a variant and the base deck on the same draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paired {
    /// Change of P(board ≥ threshold), with the half width of its 95%
    /// interval: on the same draws, then corrected by the control variates.
    pub dp: f64,
    pub dp_half: f64,
    pub dp_adjusted: f64,
    pub dp_adjusted_half: f64,
    /// Change of the mean value, with the half width of its 95% interval.
    pub dmean: f64,
    pub dmean_half: f64,
    /// Draws whose hand changed.
    pub changed: usize,
}

/// `base` and `other` are the values of the same draws, `controls` those
/// of the base deck's hands.
pub fn paired(base: &[f64], other: &[f64], threshold: f64, controls: &Controls) -> Paired {
    let n = base.len().min(other.len());
    let hit = |v: f64| if v >= threshold { 1.0 } else { 0.0 };
    let dp: Vec<f64> = (0..n).map(|i| hit(other[i]) - hit(base[i])).collect();
    let dv: Vec<f64> = (0..n).map(|i| other[i] - base[i]).collect();
    let root = (n.max(1) as f64).sqrt();
    let (dp_mean, dp_sd) = mean_sd(&dp);
    let (dv_mean, dv_sd) = mean_sd(&dv);
    let (dp_adjusted, dp_adjusted_half) = adjusted(&dp, controls);
    Paired {
        dp: dp_mean,
        dp_half: 1.959_964 * dp_sd / root,
        dp_adjusted,
        dp_adjusted_half,
        dmean: dv_mean,
        dmean_half: 1.959_964 * dv_sd / root,
        changed: dv.iter().filter(|d| d.abs() > 1e-12).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(value: f64, used: &[u32]) -> Outcome {
        Outcome { value, used: used.to_vec(), nodes: 1, seconds: 0.0 }
    }

    #[test]
    fn draws_are_hands_of_distinct_positions_and_repeat_with_the_seed() {
        let draws = draw_positions(40, 5, 300, 9);
        assert_eq!(draws.len(), 300);
        assert!(draws.iter().all(|d| d.len() == 5 && d.windows(2).all(|w| w[0] < w[1]) && d[4] < 40));
        assert_eq!(draws, draw_positions(40, 5, 300, 9));
        // Every position about as often (5/40 of the draws, 37.5 on average).
        let mut seen = [0usize; 40];
        for d in &draws {
            for &i in d {
                seen[i] += 1;
            }
        }
        assert!(seen.iter().all(|&n| (15..=65).contains(&n)), "{seen:?}");
    }

    #[test]
    fn a_variant_keeps_the_cards_that_stay_in_place() {
        let base = vec![1, 1, 1, 2, 2, 3, 4];
        // One copy of 1 out, one of 5 in: only the last 1 moves.
        let counts = BTreeMap::from([(1, 2), (2, 2), (3, 1), (4, 1), (5, 1)]);
        assert_eq!(respell(&base, &counts), vec![1, 1, 5, 2, 2, 3, 4]);
        // A bigger deck grows at the end.
        let counts = BTreeMap::from([(1, 3), (2, 2), (3, 1), (4, 1), (6, 2)]);
        assert_eq!(respell(&base, &counts), vec![1, 1, 1, 2, 2, 3, 4, 6, 6]);
        // Draws that miss the changed position see the same hand.
        let draws = draw_positions(7, 3, 50, 1);
        let a = hands_at(&base, &draws);
        let b = hands_at(&respell(&base, &BTreeMap::from([(1, 2), (2, 2), (3, 1), (4, 1), (5, 1)])), &draws);
        for (d, (x, y)) in draws.iter().zip(a.iter().zip(&b)) {
            assert_eq!(d.contains(&2), x != y, "{d:?} {x:?} {y:?}");
        }
    }

    #[test]
    fn cards_searched_that_no_line_used_count_as_blank() {
        let mut cache = HandCache::default();
        cache.insert(vec![1, 2, 3, 4, 5], outcome(10.0, &[1, 2]));
        // 3, 4 and 5 were searched and never used: the same key.
        assert_eq!(cache.get(&[1, 2, 5, 4, 3]).map(|o| o.value), Some(10.0));
        assert_eq!(cache.get(&[1, 2, 3, 3, 3]).map(|o| o.value), Some(10.0));
        // 6 was never searched: not blank.
        assert!(cache.get(&[1, 2, 6, 4, 5]).is_none());
        assert!(cache.get(&[1, 1, 3, 4, 5]).is_none());
        // 3 becomes live: a hand without it no longer has the first one's value.
        cache.insert(vec![3, 7, 8, 9, 10], outcome(4.0, &[3]));
        assert!(cache.get(&[1, 2, 3, 4, 4]).is_some());
        assert!(cache.get(&[1, 2, 5, 4, 4]).is_none());
        assert_eq!(cache.get(&[1, 2, 3, 9, 10]).map(|o| o.value), Some(10.0));
    }

    #[test]
    fn values_search_each_key_once_and_look_again_when_cards_become_live() {
        // Cards below 10 are used, and 30 too with 5; the value is the number of used cards.
        let play = |hand: &[u32]| {
            let used: Vec<u32> = hand.iter().copied().filter(|&c| c < 10 || (c == 30 && hand.contains(&5))).collect();
            outcome(used.len() as f64, &used)
        };
        let mut cache = HandCache::default();
        cache.values(&[vec![1, 30, 31], vec![2, 20, 21]], 2, play);
        assert_eq!((cache.misses, cache.hits), (2, 0));
        // 30, 31, 20 and 21 are blank: [1, 21, 30] is the first hand, [2, 20, 31] the second.
        let values: Vec<f64> = cache.values(&[vec![1, 21, 30], vec![1, 2, 20], vec![2, 20, 31]], 3, play).iter().map(|o| o.value).collect();
        assert_eq!((values, cache.misses, cache.hits), (vec![1.0, 2.0, 1.0], 3, 2));
        // [5, 30] and [5, 31] share a key; the search of the first makes 30 live, so the second is searched too.
        let values: Vec<f64> = cache.values(&[vec![5, 30], vec![5, 31]], 1, play).iter().map(|o| o.value).collect();
        assert_eq!((values, cache.misses, cache.hits), (vec![2.0, 1.0], 5, 2));
    }

    #[test]
    fn intervals_match_references() {
        // Wilson, 120 of 200: 0.5308 .. 0.6650.
        let (low, high) = wilson(120, 200);
        assert!((low - 0.5308).abs() < 5e-4 && (high - 0.6650).abs() < 5e-4, "{low} {high}");
        assert_eq!(wilson(0, 0), (0.0, 1.0));
        let e = estimate(&[1.0, 5.0, 9.0, 3.0], 4.0, &Controls::default());
        assert_eq!((e.hands, e.p, e.mean, e.p_adjusted), (4, 0.5, 4.5, 0.5));
        // Paired: one hand of four goes over the threshold.
        let p = paired(&[1.0, 5.0, 9.0, 3.0], &[1.0, 5.0, 9.0, 6.0], 4.0, &Controls::default());
        assert_eq!((p.dp, p.dmean, p.changed), (0.25, 0.75, 1));
        assert!(p.dp_half > 0.0);
    }

    #[test]
    fn presence_odds_are_hypergeometric() {
        let main: Vec<u32> = (0..37).map(|i| 100 + i).chain([7, 7, 7]).collect();
        let c = presence(&main, &[vec![7, 100, 101, 102, 103]], 5);
        // Three copies in 40, five cards: 1 - C(37,5)/C(40,5) = 0.33755.
        let at = |code: u32| {
            let mut cards: Vec<u32> = main.clone();
            cards.sort_unstable();
            cards.dedup();
            cards.iter().position(|&x| x == code).unwrap()
        };
        assert!((c.means[at(7)] - 0.337_550).abs() < 1e-6, "{:?}", c.means);
        assert!((c.means[at(100)] - 0.125).abs() < 1e-12);
        assert_eq!((c.rows[0][at(7)], c.rows[0][at(104)]), (1.0, 0.0));
    }

    #[test]
    fn control_variates_narrow_the_interval_without_moving_the_truth() {
        // A deck of 40 where a hand reaches a board when it holds card 1
        // (3 copies) or card 2 (3 copies): exact P = 1 - C(34,5)/C(40,5).
        let main: Vec<u32> = [1, 1, 1, 2, 2, 2].into_iter().chain((0..34).map(|i| 100 + i)).collect();
        let truth = 1.0 - (0..5).map(|i| (34 - i) as f64 / (40 - i) as f64).product::<f64>();
        let mut inside = 0;
        let mut narrower = 0;
        for seed in 0..40 {
            let hands = hands_at(&main, &draw_positions(40, 5, 200, seed));
            let values: Vec<f64> = hands.iter().map(|h| if h.contains(&1) || h.contains(&2) { 1.0 } else { 0.0 }).collect();
            let e = estimate(&values, 0.5, &presence(&main, &hands, 5));
            if (e.p_adjusted - truth).abs() <= e.p_adjusted_half {
                inside += 1;
            }
            if e.p_adjusted_half < 0.5 * (e.high - e.low) {
                narrower += 1;
            }
        }
        // About 95% of the intervals hold the truth; all are narrower than Wilson's.
        assert!(inside >= 34, "{inside} of 40");
        assert_eq!(narrower, 40);
    }
}
