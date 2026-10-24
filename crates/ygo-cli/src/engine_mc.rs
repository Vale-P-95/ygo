//! `engine-mc`: the Monte Carlo played by the engine (PIANO §7.4,
//! docs/plans/idee-nuove.md §4). N hands of 5 cards, each searched and
//! measured as `combos --staged` does with a smaller budget; the value of a
//! hand is its best board's measured value, and the deck's is
//! P(board ≥ threshold). Swaps of a copy are measured on the same draws,
//! with the hands searched before taken from the cache
//! ([`ygo_solver::engine_mc`]); `--climb` lets the optimizer choose them.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use ygo_engine::{DeckList, Engine};
use ygo_solver::engine_mc::{
    draw_positions, estimate, hands_at, paired, presence, respell, used_cards, Controls, Estimate, HandCache, Outcome, Paired,
};
use ygo_stats::{CardEntry, DeckModel};

/// Cards in a hand going first.
const HAND: usize = 5;
/// A hand's search stops here even if its nodes are not spent (a slow deck).
const PER_HAND: Duration = Duration::from_secs(120);

pub(crate) struct Options {
    pub deck: PathBuf,
    pub hands: usize,
    pub nodes: u64,
    pub threshold: Option<f64>,
    pub cheap: usize,
    pub full: usize,
    pub swaps: String,
    pub climb: usize,
    pub candidates: String,
    pub threads: Option<usize>,
    pub seed: u64,
    pub out: PathBuf,
    /// Compare with the roles found by playing (`roles`).
    pub roles: bool,
}

/// One deck measured: the values of the draws and what it cost.
struct Measured {
    main: Vec<u32>,
    values: Vec<f64>,
    outcomes: Vec<Outcome>,
    searched: usize,
    seconds: f64,
}

pub(crate) fn engine_mc(dir: &Path, o: Options) -> Result<()> {
    let started = Instant::now();
    let engine = crate::load_engine(dir)?;
    let db = engine.cards();
    let text = std::fs::read_to_string(&o.deck).with_context(|| format!("cannot read {}", o.deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    if deck.main.len() < HAND {
        bail!("the Main Deck has fewer than {HAND} cards");
    }
    let threads = o.threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    // Cards that may enter: --candidates and the Side Deck, Main Deck cards only.
    let mut candidates = crate::resolve_hand(&engine, &o.candidates)?;
    candidates.extend(&deck.side);
    candidates.retain(|&c| db.get(c).is_some_and(|card| !card.is_extra_deck_monster()));
    candidates.sort_unstable();
    candidates.dedup();
    let swaps = parse_swaps(&engine, &deck.main, &o.swaps)?;
    candidates.extend(swaps.iter().map(|&(_, into)| into).filter(|c| !deck.main.contains(c)));
    candidates.sort_unstable();
    candidates.dedup();

    // What steers the search, read once on the deck with every candidate in
    // it, so that every variant searches with the same values.
    let kit = ygo_solver::Kit::standard(&engine);
    let mut union = deck.clone();
    union.main.extend(&candidates);
    let mut base = ygo_solver::SearchConfig::new(deck.clone(), Vec::new());
    base.seed = o.seed;
    base.max_nodes = o.nodes;
    base.time_limit = PER_HAND;
    base.threads = 1;
    base.opponent = Some(kit.opponent().clone());
    base.hand_traps = ygo_solver::hand_traps(&engine, &union, o.seed, &kit);
    base.values = std::sync::Arc::new(ygo_solver::card_values(&engine, &union, o.seed, &kit, &base.hand_traps, threads));
    base.learn = Some(std::sync::Arc::new(kit.clone()));
    base.policy.passive = crate::known_hand_traps(&engine);
    let prepared = started.elapsed().as_secs_f64();

    let draws = draw_positions(deck.main.len(), HAND, o.hands, o.seed);
    let controls = presence(&deck.main, &hands_at(&deck.main, &draws), HAND);
    let cache = RefCell::new(HandCache::default());
    let measure = |main: &[u32]| -> Measured {
        let begun = Instant::now();
        let mut variant = deck.clone();
        variant.main = main.to_vec();
        let hands = hands_at(main, &draws);
        let before = cache.borrow().misses;
        let play = |hand: &[u32]| play_hand(&engine, &base, &kit, &variant, hand, o.cheap, o.full);
        let outcomes = cache.borrow_mut().values(&hands, threads, play);
        let searched = cache.borrow().misses - before;
        let values = outcomes.iter().map(|x| x.value).collect();
        Measured { main: main.to_vec(), values, outcomes, searched, seconds: begun.elapsed().as_secs_f64() }
    };

    eprintln!("deck: {} hands, {} nodes each, {threads} at once", o.hands, o.nodes);
    let first = measure(&deck.main);
    // Without a threshold, the median of the deck: the share that reacts most to a change.
    let threshold = o.threshold.unwrap_or_else(|| {
        let mut v = first.values.clone();
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    });
    let base_estimate = estimate(&first.values, threshold, &controls);
    eprintln!("deck: P = {:.1}% in {:.0} s", 100.0 * base_estimate.p_adjusted, first.seconds);
    let mut variants: Vec<(String, Measured, Paired)> = Vec::new();
    for &(out, into) in &swaps {
        let main = swapped(&deck.main, out, into);
        let m = measure(&main);
        let p = paired(&first.values, &m.values, threshold, &controls);
        let label = format!("-1 {} +1 {}", db.name(out), db.name(into));
        eprintln!("{label}: ΔP = {:+.1} points, {} hands searched in {:.0} s", 100.0 * p.dp_adjusted, m.searched, m.seconds);
        variants.push((label, m, p));
    }

    // The optimizer: swaps ranked by what the engine saw, measured on the same draws.
    let mut climbed = None;
    if o.climb > 0 {
        let banlist = ygo_data::Banlist::embedded_md();
        let limits: BTreeMap<u32, u32> = deck
            .main
            .iter()
            .chain(&candidates)
            .filter_map(|&c| {
                let canonical = db.canonical_code(c);
                banlist.cards.iter().find(|e| e.id == c || e.id == canonical).map(|e| (c, e.limit as u32))
            })
            .collect();
        let measured: RefCell<HashMap<Vec<u32>, Measured>> = RefCell::new(HashMap::new());
        let main_of = |d: &DeckModel| respell(&deck.main, &d.cards.iter().map(|c| (c.id, c.copies)).collect());
        let config = ygo_stats::ClimbConfig {
            deck: DeckModel::from_ids(&deck.main, &BTreeMap::new()),
            candidates: candidates.iter().map(|&id| CardEntry { id, copies: 0, roles: Vec::new() }).collect(),
            limits,
            locked: Vec::new(),
            budget: o.climb,
            // A swap is kept when the low end of its gain's 95% interval is above 0.
            margin: 0.0,
        };
        let judge = |current: &DeckModel, next: &DeckModel| {
            let (from, to) = (main_of(current), main_of(next));
            // The current deck was measured: the base, or a swap kept.
            let a = measured.borrow().get(&from).map_or_else(|| first.values.clone(), |m| m.values.clone());
            let b = measure(&to);
            let controls = presence(&from, &hands_at(&from, &draws), HAND);
            let p = paired(&a, &b.values, threshold, &controls);
            let objective = estimate(&b.values, threshold, &presence(&to, &hands_at(&to, &draws), HAND)).p_adjusted;
            eprintln!(
                "climb: {} → P = {:.1}%, ΔP {:+.1} ± {:.1} points, {} hands searched",
                describe(&engine, &deck.main, &to),
                100.0 * objective,
                100.0 * p.dp_adjusted,
                100.0 * p.dp_adjusted_half,
                b.searched
            );
            measured.borrow_mut().insert(to, b);
            Some(ygo_stats::Judged { objective, gain_low: p.dp_adjusted - p.dp_adjusted_half })
        };
        let rank = |d: &DeckModel| {
            let main = main_of(d);
            let all = measured.borrow();
            let outcomes = match all.get(&main) {
                Some(m) => m.outcomes.clone(),
                None if main == deck.main => first.outcomes.clone(),
                None => return Vec::new(),
            };
            rank_swaps(&main, &hands_at(&main, &draws), &outcomes, &candidates, threshold)
        };
        let result = ygo_stats::climb(&config, base_estimate.p_adjusted, rank, judge);
        climbed = Some(result);
    }

    let roles = o.roles.then(|| {
        let guesses =
            ygo_solver::guess_roles(&engine, &deck, &kit, &base.hand_traps, &base.policy.passive, base.values.clone(), threads, |_, _| {});
        compare_roles(&engine, &deck.main, &guesses, &hands_at(&deck.main, &draws), &first.values, threshold)
    });
    let c = cache.borrow();
    let mut summary =
        summary(&engine, &o, threshold, &first, &base_estimate, &variants, climbed.as_ref(), &c, prepared, started.elapsed().as_secs_f64());
    if let Some(line) = roles {
        summary.push_str(&line);
    }
    print!("{summary}");
    std::fs::create_dir_all(&o.out).with_context(|| format!("cannot create {}", o.out.display()))?;
    std::fs::write(o.out.join("summary.txt"), &summary)?;
    let json = report_json(&engine, &o, threshold, &draws, &controls, &first, &variants, climbed.as_ref());
    std::fs::write(o.out.join("hands.json"), serde_json::to_string_pretty(&json)?)?;
    Ok(())
}

/// "Out>In" pairs, semicolon-separated: the deck's own copy of Out.
fn parse_swaps(engine: &Engine, main: &[u32], text: &str) -> Result<Vec<(u32, u32)>> {
    let db = engine.cards();
    let mut swaps = Vec::new();
    for item in text.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let (out, into) = item.split_once('>').with_context(|| format!("a swap is \"Out>In\": {item:?}"))?;
        let out = *crate::resolve_hand(engine, out)?.first().context("empty swap")?;
        let into = *crate::resolve_hand(engine, into)?.first().context("empty swap")?;
        let out = main
            .iter()
            .copied()
            .find(|&c| db.canonical_code(c) == db.canonical_code(out))
            .with_context(|| format!("{} is not in the Main Deck", db.name(out)))?;
        swaps.push((out, into));
    }
    Ok(swaps)
}

/// `main` with its last copy of `out` turned into `into`.
fn swapped(main: &[u32], out: u32, into: u32) -> Vec<u32> {
    let mut main = main.to_vec();
    if let Some(i) = main.iter().rposition(|&c| c == out) {
        main[i] = into;
    }
    main
}

/// The cards of a search's hand and the measured value of its best board:
/// the search with its node budget, the cheap measure of the first `cheap`
/// fields, and the full one of the best `full` by it.
fn play_hand(
    engine: &Engine,
    base: &ygo_solver::SearchConfig,
    kit: &ygo_solver::Kit,
    deck: &DeckList,
    hand: &[u32],
    cheap: usize,
    full: usize,
) -> Outcome {
    let begun = Instant::now();
    let mut config = base.clone();
    config.deck = deck.clone();
    config.hand = hand.to_vec();
    let result = ygo_solver::search(engine, &config, &AtomicBool::new(false), |_| {});
    let mut prober = ygo_solver::Prober::new(engine, deck, hand, base.seed, kit);
    let how = ygo_solver::probe::Cheap { windows: crate::STAGED_WINDOWS, use_rounds: crate::STAGED_ROUNDS };
    let mut ranked = Vec::new();
    for (_, group) in crate::fields(&result.boards, cheap.max(1)) {
        let board = group[0].clone();
        if let Ok((m, partial)) = prober.measure_cheap(&board.line, how) {
            ranked.push((crate::cheap_composite(&m), board, partial));
        }
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut best: Option<(f64, ygo_solver::Board)> = None;
    for (_, board, partial) in ranked.into_iter().take(full.max(1)) {
        if let Ok(m) = prober.measure_resumed(&board.line, partial) {
            let v = crate::composite(&m);
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

/// How often each card of `main` made a good board: the share of the draws
/// with it in the hand whose best board reached the threshold with the card
/// used. Cards never drawn get `None`.
fn usefulness(hands: &[Vec<u32>], outcomes: &[Outcome], threshold: f64) -> HashMap<u32, f64> {
    let mut count: HashMap<u32, (usize, usize)> = HashMap::new();
    for (hand, outcome) in hands.iter().zip(outcomes) {
        let mut cards = hand.clone();
        cards.dedup();
        for c in cards {
            let e = count.entry(c).or_default();
            e.0 += 1;
            if outcome.value >= threshold && outcome.used.contains(&c) {
                e.1 += 1;
            }
        }
    }
    count.into_iter().map(|(c, (n, good))| (c, good as f64 / n as f64)).collect()
}

/// The swaps of a deck, most promising first: out, the cards that least
/// often make a good board; in, the candidates (never measured in this
/// deck), then the deck's cards that most often do. A pair ranks by the
/// sum of its two places.
fn rank_swaps(main: &[u32], hands: &[Vec<u32>], outcomes: &[Outcome], candidates: &[u32], threshold: f64) -> Vec<(u32, u32)> {
    let useful = usefulness(hands, outcomes, threshold);
    let mut cards: Vec<u32> = main.to_vec();
    cards.sort_unstable();
    cards.dedup();
    let score = |c: &u32| useful.get(c).copied().unwrap_or(0.0);
    let mut outs = cards.clone();
    outs.sort_by(|a, b| score(a).total_cmp(&score(b)).then(a.cmp(b)));
    let mut ins: Vec<u32> = candidates.iter().copied().filter(|c| !main.contains(c)).collect();
    let mut more = cards;
    more.sort_by(|a, b| score(b).total_cmp(&score(a)).then(a.cmp(b)));
    ins.extend(more);
    let mut pairs: Vec<(usize, u32, u32)> =
        outs.iter().enumerate().flat_map(|(i, &out)| ins.iter().enumerate().map(move |(j, &into)| (i + j, out, into))).collect();
    pairs.sort_by_key(|&(at, _, _)| at);
    pairs.into_iter().map(|(_, out, into)| (out, into)).collect()
}

/// The roles found by playing against the Monte Carlo: the exact odds of
/// a Starter in the hand, and how often the hands with one and without one
/// reached the threshold.
fn compare_roles(
    engine: &Engine,
    main: &[u32],
    guesses: &[ygo_solver::RoleGuess],
    hands: &[Vec<u32>],
    values: &[f64],
    threshold: f64,
) -> String {
    let db = engine.cards();
    let starters: Vec<u32> =
        main.iter().copied().filter(|&c| guesses.iter().any(|g| g.code == db.canonical_code(c) && g.roles.contains(&"starter"))).collect();
    let roles = BTreeMap::from([("starter".to_string(), starters.clone())]);
    let exact = ygo_stats::probability(&DeckModel::from_ids(main, &roles), &ygo_stats::Cond::role("starter", ygo_stats::Op::AtLeast, 1), 5)
        .unwrap_or(f64::NAN);
    let (mut with, mut without) = ((0, 0), (0, 0));
    for (hand, &v) in hands.iter().zip(values) {
        let side = if hand.iter().any(|c| starters.contains(c)) { &mut with } else { &mut without };
        side.0 += 1;
        side.1 += usize::from(v >= threshold);
    }
    let share = |(n, k): (usize, usize)| 100.0 * k as f64 / n.max(1) as f64;
    let mut names: Vec<String> = starters.iter().map(|&c| db.name(c)).collect();
    names.dedup();
    format!(
        "roles: P(≥1 Starter) exact {:.1}% ({}) · board ≥ {threshold:.2}: {:.1}% of {} hands with a Starter, {:.1}% of {} without\n",
        100.0 * exact,
        names.join(", "),
        share(with),
        with.0,
        share(without),
        without.0
    )
}

/// The copies that changed from `from` to `to`, as "-1 Name, +1 Name".
fn describe(engine: &Engine, from: &[u32], to: &[u32]) -> String {
    let count = |v: &[u32]| {
        let mut m: BTreeMap<u32, i64> = BTreeMap::new();
        for &c in v {
            *m.entry(c).or_default() += 1;
        }
        m
    };
    let (a, b) = (count(from), count(to));
    let mut parts = Vec::new();
    for c in a.keys().chain(b.keys().filter(|c| !a.contains_key(c))) {
        let d = b.get(c).copied().unwrap_or(0) - a.get(c).copied().unwrap_or(0);
        if d != 0 {
            parts.push(format!("{d:+} {}", engine.cards().name(*c)));
        }
    }
    if parts.is_empty() {
        "the deck".into()
    } else {
        parts.join(", ")
    }
}

#[allow(clippy::too_many_arguments)]
fn summary(
    engine: &Engine,
    o: &Options,
    threshold: f64,
    first: &Measured,
    e: &Estimate,
    variants: &[(String, Measured, Paired)],
    climbed: Option<&ygo_stats::ClimbResult>,
    cache: &HandCache,
    prepared: f64,
    total: f64,
) -> String {
    let mut s = String::new();
    let nodes: u64 = first.outcomes.iter().map(|x| x.nodes).sum();
    let hand_seconds: f64 = first.outcomes.iter().map(|x| x.seconds).sum();
    let _ = writeln!(s, "engine-mc {} · {} hands · {} nodes · threshold {threshold:.2}", o.deck.display(), o.hands, o.nodes);
    let _ = writeln!(
        s,
        "P(board ≥ {:.2}) = {:.1}% ± {:.1} points (control variates) · sample {:.1}% (Wilson 95%: {:.1}–{:.1}%) · mean value {:.2} ± {:.2}",
        threshold,
        100.0 * e.p_adjusted,
        100.0 * e.p_adjusted_half,
        100.0 * e.p,
        100.0 * e.low,
        100.0 * e.high,
        e.mean,
        e.mean_half
    );
    let mut sorted = first.values.clone();
    sorted.sort_by(f64::total_cmp);
    let q = |f: f64| sorted.get(((sorted.len() as f64 - 1.0) * f).round() as usize).copied().unwrap_or(0.0);
    let _ = writeln!(s, "values: 10% {:.1} · 25% {:.1} · median {:.1} · 75% {:.1} · 90% {:.1}", q(0.1), q(0.25), q(0.5), q(0.75), q(0.9));
    let _ = writeln!(
        s,
        "deck: {} hands searched for {} draws in {:.0} s ({:.1} s per hand on one thread, {} nodes), setup {:.0} s",
        first.searched,
        o.hands,
        first.seconds,
        hand_seconds / first.searched.max(1) as f64,
        nodes / first.outcomes.len().max(1) as u64,
        prepared
    );
    for (label, m, p) in variants {
        let _ = writeln!(
            s,
            "{label}: ΔP {:+.1} ± {:.1} points (same draws: {:+.1} ± {:.1}), Δmean {:+.2} ± {:.2} · {} draws changed · {} searched ({:.0}% avoided) in {:.0} s",
            100.0 * p.dp_adjusted,
            100.0 * p.dp_adjusted_half,
            100.0 * p.dp,
            100.0 * p.dp_half,
            p.dmean,
            p.dmean_half,
            p.changed,
            m.searched,
            100.0 * (1.0 - m.searched as f64 / o.hands.max(1) as f64),
            m.seconds
        );
    }
    if let Some(r) = climbed {
        let changes: Vec<String> =
            r.changes.iter().map(|c| format!("{} {}→{}", engine.cards().name(c.id), c.from, c.to)).collect::<Vec<_>>();
        let _ = writeln!(
            s,
            "climb: {:.1}% → {:.1}% after {} decks measured · {}",
            100.0 * r.before,
            100.0 * r.after,
            r.tried.len(),
            if changes.is_empty() { "no change".into() } else { changes.join(", ") }
        );
    }
    let _ = writeln!(
        s,
        "cache: {} hands searched, {} answered ({:.0}%), {} live cards · total {:.0} s",
        cache.misses,
        cache.hits,
        100.0 * cache.hits as f64 / (cache.hits + cache.misses).max(1) as f64,
        cache.live().len(),
        total
    );
    s
}

#[allow(clippy::too_many_arguments)]
fn report_json(
    engine: &Engine,
    o: &Options,
    threshold: f64,
    draws: &[Vec<usize>],
    controls: &Controls,
    first: &Measured,
    variants: &[(String, Measured, Paired)],
    climbed: Option<&ygo_stats::ClimbResult>,
) -> serde_json::Value {
    let db = engine.cards();
    let deck_json = |m: &Measured| {
        let hands = hands_at(&m.main, draws);
        let e = estimate(&m.values, threshold, controls);
        serde_json::json!({
            "p": e.p,
            "pAdjusted": e.p_adjusted,
            "pAdjustedHalf": e.p_adjusted_half,
            "searched": m.searched,
            "seconds": m.seconds,
            "hands": hands.iter().zip(&m.outcomes).map(|(h, x)| serde_json::json!({
                "hand": h.iter().map(|&c| db.name(c)).collect::<Vec<_>>(),
                "value": x.value,
                "used": x.used.iter().map(|&c| db.name(c)).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    };
    serde_json::json!({
        "deck": o.deck.display().to_string(),
        "hands": o.hands,
        "nodes": o.nodes,
        "threshold": threshold,
        "seed": o.seed,
        "base": deck_json(first),
        "variants": variants.iter().map(|(label, m, p)| serde_json::json!({
            "label": label, "dp": p.dp_adjusted, "dpHalf": p.dp_adjusted_half, "dpSameDraws": p.dp, "dmean": p.dmean, "measured": deck_json(m),
        })).collect::<Vec<_>>(),
        "climb": climbed.map(|r| serde_json::to_value(r).unwrap_or_default()),
    })
}
