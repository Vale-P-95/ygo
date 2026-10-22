//! "Card + rest": the search on many hands that contain a given card, to
//! tell what that card leads to on average and which boards come most often.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::search::{resume, Board, Frontier, Progress, SearchConfig, Stop};

/// One sampled hand and what the search found from it.
#[derive(Debug, Clone)]
pub struct HandResult {
    pub hand: Vec<u32>,
    /// Share of all the hands with the card that this hand stands for.
    pub weight: f64,
    /// Best first, at most `keep` (see [`search_card`]).
    pub boards: Vec<Board>,
    pub stop: Stop,
    pub nodes: u64,
    /// The goal is reached (`Some(true)`), cannot be (`Some(false)`, every
    /// line tried) or is still unknown.
    pub goal: Option<bool>,
    /// To go on with this hand ([`resume_card`]).
    pub frontier: Frontier,
}

/// SplitMix64.
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) % n.max(1) as u64) as usize
    }
}

/// Up to `count` different 5-card hands from `main` that contain `card`,
/// with their weights. A hand is drawn as the card plus 4 random cards:
/// that favours hands holding k copies k times, so each counts 1/k.
pub fn sample_hands(main: &[u32], card: u32, count: usize, seed: u64) -> Vec<(Vec<u32>, f64)> {
    let Some(at) = main.iter().position(|&c| c == card) else { return Vec::new() };
    let mut rest = main.to_vec();
    rest.remove(at);
    let mut rng = Rng(seed ^ 0x5eed_c0de);
    let mut found: HashMap<Vec<u32>, f64> = HashMap::new();
    let mut order = Vec::new();
    let mut draws = 0;
    while found.len() < count && draws < count * 200 {
        draws += 1;
        let mut pool = rest.clone();
        let mut hand = vec![card];
        for _ in 0..4.min(pool.len()) {
            hand.push(pool.swap_remove(rng.below(pool.len())));
        }
        let copies = hand.iter().filter(|&&c| c == card).count() as f64;
        let mut key = hand.clone();
        key.sort_unstable();
        if !found.contains_key(&key) {
            order.push(key.clone());
        }
        *found.entry(key).or_default() += 1.0 / copies;
    }
    let total: f64 = found.values().sum();
    order.into_iter().map(|h| (h.clone(), found[&h] / total.max(f64::EPSILON))).collect()
}

/// What to search in "card + rest".
#[derive(Debug, Clone)]
pub struct CardSearch {
    pub card: u32,
    /// Different hands to try.
    pub hands: usize,
    pub per_hand: Duration,
    /// Boards kept per hand.
    pub keep: usize,
}

/// Searches each sampled hand for `per_hand`. `progress` gets the index of
/// the hand being searched, all the sampled hands and the progress.
pub fn search_card(
    engine: &ygo_engine::Engine,
    base: &SearchConfig,
    what: &CardSearch,
    cancel: &AtomicBool,
    progress: impl FnMut(usize, &[(Vec<u32>, f64)], &Progress),
) -> Vec<HandResult> {
    let sampled = sample_hands(&base.deck.main, what.card, what.hands, base.seed);
    let previous = sampled.into_iter().map(|(hand, weight)| (hand, weight, Frontier::default())).collect();
    resume_card(engine, base, what, previous, cancel, progress)
}

/// Goes on with each hand of a "card + rest" search for `per_hand` more:
/// `previous` holds the hands, their weights and where each search stopped.
/// Hands already explored completely are not searched again.
pub fn resume_card(
    engine: &ygo_engine::Engine,
    base: &SearchConfig,
    what: &CardSearch,
    previous: Vec<(Vec<u32>, f64, Frontier)>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, &[(Vec<u32>, f64)], &Progress),
) -> Vec<HandResult> {
    let hands: Vec<(Vec<u32>, f64)> = previous.iter().map(|(h, w, _)| (h.clone(), *w)).collect();
    let mut out = Vec::new();
    for (i, (hand, weight, frontier)) in previous.into_iter().enumerate() {
        let mut config = base.clone();
        config.hand = hand.clone();
        config.time_limit = what.per_hand;
        let done = frontier.is_exhausted() || cancel.load(Ordering::Relaxed);
        let result = if done {
            // Nothing left to try (or stopped): read what was found.
            let mut config = config.clone();
            config.time_limit = Duration::ZERO;
            let stop_now = AtomicBool::new(true);
            resume(engine, &config, frontier, &stop_now, |_| {})
        } else {
            resume(engine, &config, frontier, cancel, |p| progress(i, &hands, p))
        };
        let goal = result.goal_reached(&config.goal);
        let stop = if done && !cancel.load(Ordering::Relaxed) { Stop::Exhausted } else { result.stop };
        let mut boards = result.boards;
        boards.truncate(what.keep);
        out.push(HandResult { hand, weight, boards, stop, nodes: result.progress.nodes, goal, frontier: result.frontier });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_contain_the_card_and_weights_add_up() {
        let mut main: Vec<u32> = (1..=37).collect();
        main.extend([99, 99, 99]);
        let hands = sample_hands(&main, 99, 30, 7);
        assert_eq!(hands.len(), 30);
        assert!(hands.iter().all(|(h, _)| h.len() == 5 && h.contains(&99)));
        let total: f64 = hands.iter().map(|(_, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-9);
        assert!(sample_hands(&main, 1234, 5, 7).is_empty());
    }
}
