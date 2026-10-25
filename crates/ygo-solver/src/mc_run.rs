//! The engine-guided Monte Carlo as one call ([`run`]), for the app and the
//! CLI: N hands of 5 drawn from the deck, each searched by the Combo Lab and
//! measured in two stages, then P(board ≥ threshold) with its interval
//! ([`crate::engine_mc`]). One swap of a copy can be measured on the same
//! draws. Also holds the pieces the `combos --staged` command shares with it.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ygo_engine::{DeckList, Engine};

use crate::engine_mc::{draw_positions, estimate, hands_at, paired, presence, used_cards, Estimate, HandCache, Outcome, Paired};
use crate::probe::Cheap;
use crate::{Board, Kit, Measure, Prober, SearchConfig};

/// Cards in a hand going first.
pub const HAND: usize = 5;
/// A hand's search stops here even if its nodes are not spent (a slow deck).
pub const PER_HAND: Duration = Duration::from_secs(120);

/// Fields given the cheap measure by default, and of those the best by it
/// given the full one (`docs/plans/velocita-ricerca.md`, B14).
pub const STAGED_CHEAP: usize = 30;
pub const STAGED_FULL: usize = 15;
pub const STAGED_WINDOWS: usize = 8;
pub const STAGED_ROUNDS: usize = 1;

/// The first `top` fields of a search's boards, best first, each with its
/// boards: boards with the same field (other zones, another hand, other
/// cards in the GY) are variants of one, and each field is listed once, so
/// the top boards are different fields.
pub fn fields(boards: &[Board], top: usize) -> Vec<(u64, Vec<Board>)> {
    let mut groups: Vec<(u64, Vec<Board>)> = Vec::new();
    for board in boards {
        let key = crate::search::field_key(&board.state);
        let open = groups.len() < top;
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, g)) => g.push(board.clone()),
            None if open => groups.push((key, vec![board.clone()])),
            None => {}
        }
    }
    groups
}

/// The value of a cheap measure ([`Prober::measure_cheap`]): field, hand and
/// floodgates, and your next turn when the opponent breaks nothing. The
/// floodgates weigh 0.4, as in [`composite`], so the cut to the full
/// measure ranks by the weights of the score.
pub fn cheap_composite(m: &Measure) -> f64 {
    let rebuild = m.follow_up.map_or(0.0, |f| f.rebuild as f64);
    m.field_value + 0.9 * m.hand_value + 0.4 * m.floodgate_value + 0.4 * 0.3 * rebuild
}

/// The measured value of a board with the "Bilanciato" profile of the app:
/// the same weights as `WEIGHTS`, `FOLLOW_UP` and `REBUILD` in
/// scripts/bench.py (the cards gained in the End Phase weigh 0 there).
pub fn composite(m: &Measure) -> f64 {
    const WEIGHTS: [f64; 4] = [1.0, 0.9, 0.4, 0.0];
    const FOLLOW_UP: f64 = 0.4;
    const REBUILD: f64 = 0.3;
    let parts = [m.field_value, m.hand_value, m.floodgate_value, m.resilience];
    let rebuild = m.follow_up.map_or(0.0, |f| f.rebuild as f64);
    WEIGHTS.iter().zip(parts).map(|(w, v)| w * v).sum::<f64>() + FOLLOW_UP * REBUILD * rebuild
}

/// The cards of a search's hand and the measured value of its best board:
/// the search with its node budget, the cheap measure of the first `cheap`
/// fields, and the full one of the best `full` by it. Stops early, with a
/// meaningless outcome, when `cancel` is set.
#[allow(clippy::too_many_arguments)]
pub fn play_hand(
    engine: &Engine,
    base: &SearchConfig,
    kit: &Kit,
    deck: &DeckList,
    hand: &[u32],
    cheap: usize,
    full: usize,
    cancel: &AtomicBool,
) -> Outcome {
    let begun = Instant::now();
    let mut config = base.clone();
    config.deck = deck.clone();
    config.hand = hand.to_vec();
    let result = crate::search(engine, &config, cancel, |_| {});
    let mut prober = Prober::new(engine, deck, hand, base.seed, kit);
    let how = Cheap { windows: STAGED_WINDOWS, use_rounds: STAGED_ROUNDS };
    let mut ranked = Vec::new();
    for (_, group) in fields(&result.boards, cheap.max(1)) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let board = group[0].clone();
        if let Ok((m, partial)) = prober.measure_cheap(&board.line, how) {
            ranked.push((cheap_composite(&m), board, partial));
        }
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut best: Option<(f64, Board)> = None;
    for (_, board, partial) in ranked.into_iter().take(full.max(1)) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(m) = prober.measure_resumed(&board.line, partial) {
            let v = composite(&m);
            if best.as_ref().is_none_or(|(b, _)| v > *b) {
                best = Some((v, board));
            }
        }
    }
    // The value depends on the cards the best line used, and on the
    // handtraps, which count kept in the hand.
    let db = engine.cards();
    let traps: Vec<u32> = base.hand_traps.iter().map(|&c| db.canonical_code(c)).collect();
    let is_trap = |c: &u32| traps.contains(&db.canonical_code(*c));
    let mut used = best.as_ref().map(|(_, b)| used_cards(hand, std::slice::from_ref(b))).unwrap_or_default();
    used.retain(|c| !is_trap(c));
    used.extend(hand.iter().filter(|c| is_trap(c)));
    used.sort_unstable();
    let value = best.map_or(0.0, |(v, _)| v);
    Outcome { value, used, nodes: result.progress.nodes, seconds: begun.elapsed().as_secs_f64() }
}

/// What [`run`] asks.
#[derive(Debug, Clone)]
pub struct McConfig {
    pub hands: usize,
    /// Nodes the search spends on a hand.
    pub nodes: u64,
    /// Fixed threshold, or none for the median of the deck's hands.
    pub threshold: Option<f64>,
    pub cheap: usize,
    pub full: usize,
    pub seed: u64,
    /// Hands searched at once, one thread each.
    pub threads: usize,
    /// Not played from the hand on your own turn (the known handtraps).
    pub passive: Vec<u32>,
    /// One copy of `.0` turned into `.1`, measured on the same draws.
    pub swap: Option<(u32, u32)>,
}

/// One hand played.
#[derive(Debug, Clone, PartialEq)]
pub struct McHand {
    pub hand: Vec<u32>,
    pub value: f64,
    /// Cards the best line used (the handtraps count as used when held).
    pub used: Vec<u32>,
}

/// Which deck a hand belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McPhase {
    Deck,
    Swap,
}

/// A hand just finished, while the run goes on.
#[derive(Debug, Clone)]
pub struct McProgress {
    pub phase: McPhase,
    /// Hands finished of the `total` this phase searches.
    pub done: usize,
    pub total: usize,
    pub played: McHand,
}

/// A deck's hands before its estimate (which needs the threshold).
struct Raw {
    main: Vec<u32>,
    hands: Vec<McHand>,
    searched: usize,
    seconds: f64,
}

/// A deck measured.
#[derive(Debug, Clone)]
pub struct McDeck {
    pub main: Vec<u32>,
    pub estimate: Estimate,
    pub hands: Vec<McHand>,
    /// Hands searched (the cache answers the others).
    pub searched: usize,
    pub seconds: f64,
}

#[derive(Debug, Clone)]
pub struct McSwap {
    pub out: u32,
    pub into: u32,
    pub deck: McDeck,
    pub paired: Paired,
}

#[derive(Debug, Clone)]
pub struct McReport {
    pub threshold: f64,
    pub deck: McDeck,
    pub swap: Option<McSwap>,
    /// Stopped before the last hand: the estimate uses the hands done.
    pub stopped: bool,
    pub seconds: f64,
}

/// P(board ≥ threshold) of `deck` played by the engine; progress after each
/// hand, `cancel` stops it (the report then holds the hands done). Errors
/// when the Main Deck is too small for a hand.
pub fn run(
    engine: &Engine,
    deck: &DeckList,
    config: &McConfig,
    cancel: &AtomicBool,
    progress: &(dyn Fn(McProgress) + Sync),
) -> Result<McReport, String> {
    let started = Instant::now();
    if deck.main.len() < HAND {
        return Err(format!("Il Main Deck ha meno di {HAND} carte"));
    }
    let threads = config.threads.max(1);
    let kit = Kit::standard(engine);
    // What steers the search, read once on the deck with the incoming card
    // in it, so that both variants search with the same values.
    let mut union = deck.clone();
    union.main.extend(config.swap.map(|s| s.1));
    let mut base = SearchConfig::new(deck.clone(), Vec::new());
    base.seed = config.seed;
    base.max_nodes = config.nodes;
    base.time_limit = PER_HAND;
    base.threads = 1;
    base.opponent = Some(kit.opponent().clone());
    base.hand_traps = crate::hand_traps(engine, &union, config.seed, &kit);
    base.values = Arc::new(crate::card_values(engine, &union, config.seed, &kit, &base.hand_traps, threads));
    base.learn = Some(Arc::new(kit.clone()));
    base.policy.passive = config.passive.clone();

    let draws = draw_positions(deck.main.len(), HAND, config.hands, config.seed);
    let mut cache = HandCache::default();
    let measure = |cache: &mut HandCache, main: &[u32], phase: McPhase| -> Raw {
        let begun = Instant::now();
        let mut variant = deck.clone();
        variant.main = main.to_vec();
        let hands = hands_at(main, &draws);
        let before = cache.misses;
        let done = AtomicUsize::new(0);
        let finished: Mutex<Vec<McHand>> = Mutex::new(Vec::new());
        let play = |hand: &[u32]| {
            if cancel.load(Ordering::Relaxed) {
                return Outcome { value: 0.0, used: Vec::new(), nodes: 0, seconds: 0.0 };
            }
            let outcome = play_hand(engine, &base, &kit, &variant, hand, config.cheap, config.full, cancel);
            if !cancel.load(Ordering::Relaxed) {
                let played = McHand { hand: hand.to_vec(), value: outcome.value, used: outcome.used.clone() };
                if let Ok(mut f) = finished.lock() {
                    f.push(played.clone());
                }
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                progress(McProgress { phase, done: n, total: hands.len(), played });
            }
            outcome
        };
        let outcomes = cache.values(&hands, threads, play);
        let searched = cache.misses - before;
        let hands_played: Vec<McHand> = if cancel.load(Ordering::Relaxed) {
            // Only the hands that finished: the others hold placeholders.
            finished.into_inner().unwrap_or_default()
        } else {
            hands.iter().zip(&outcomes).map(|(h, o)| McHand { hand: h.clone(), value: o.value, used: o.used.clone() }).collect()
        };
        Raw { main: main.to_vec(), hands: hands_played, searched, seconds: begun.elapsed().as_secs_f64() }
    };
    // The estimate depends on the threshold, known only after the deck's values.
    let finish = |raw: Raw, threshold: f64| -> McDeck {
        let hands: Vec<Vec<u32>> = raw.hands.iter().map(|h| h.hand.clone()).collect();
        let values: Vec<f64> = raw.hands.iter().map(|h| h.value).collect();
        let estimate = estimate(&values, threshold, &presence(&raw.main, &hands, HAND));
        McDeck { main: raw.main, estimate, hands: raw.hands, searched: raw.searched, seconds: raw.seconds }
    };

    let first = measure(&mut cache, &deck.main, McPhase::Deck);
    let mut values: Vec<f64> = first.hands.iter().map(|h| h.value).collect();
    let threshold = config.threshold.unwrap_or_else(|| {
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied().unwrap_or(0.0)
    });
    let first = finish(first, threshold);
    let mut stopped = cancel.load(Ordering::Relaxed);
    let mut swap = None;
    if let Some((out, into)) = config.swap.filter(|_| !stopped) {
        let mut main = deck.main.clone();
        if let Some(i) = main.iter().rposition(|&c| c == out) {
            main[i] = into;
        }
        let second = measure(&mut cache, &main, McPhase::Swap);
        stopped = cancel.load(Ordering::Relaxed);
        if !stopped {
            let base_values: Vec<f64> = first.hands.iter().map(|h| h.value).collect();
            let other: Vec<f64> = second.hands.iter().map(|h| h.value).collect();
            let controls = presence(&deck.main, &hands_at(&deck.main, &draws), HAND);
            let p = paired(&base_values, &other, threshold, &controls);
            swap = Some(McSwap { out, into, deck: finish(second, threshold), paired: p });
        }
    }
    Ok(McReport { threshold, deck: first, swap, stopped, seconds: started.elapsed().as_secs_f64() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_stopped_early_still_has_an_estimate() {
        // `run` estimates from the hands done when stopped: 0, 1 or 2 hands must not panic.
        let main: Vec<u32> = (1..=40).collect();
        for n in 0..3usize {
            let hands: Vec<Vec<u32>> = (0..n).map(|i| main[i..i + HAND].to_vec()).collect();
            let values = vec![1.0; n];
            let e = estimate(&values, 0.5, &presence(&main, &hands, HAND));
            assert_eq!(e.hands, n);
            assert!(e.p_adjusted.is_finite() || n == 0, "n = {n}");
        }
    }
}
