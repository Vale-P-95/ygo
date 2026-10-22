//! The probe turn against a chosen deck (#44, level 1): the board is put to
//! the test by the board breakers of that deck, and the line by its
//! handtraps, each weighted by the chance the opponent has it in hand.
//! Interruptions and floodgates stay measured with the standard kit.
//!
//! Both are read from the engine: the breakers are the cards of the deck
//! that take or negate the opponent's cards (by text, as the roles do), that
//! the engine lets it play from the hand and that, played against a plain
//! field, take something off it; the handtraps are those it can use from
//! the hand during your turn ([`hand_traps`]).

use std::collections::HashMap;

use ygo_engine::auto::passive_answer;
use ygo_engine::consts::{location, position};
use ygo_engine::{Action, ActionKind, CardRef, Decision, DeckList, Duel, Engine, Error, Event, Opponent, Placement, Step};

use crate::follow::{declinable, find_again, own_answers, pick_again, Script, PASS};
use crate::probe::{
    end_turn, find_act, forward, hand_traps, window_answer, Act, Breaker, Effect, Kit, Measure, Prelude, Prober, ResponderId,
};
use crate::roles::breakers_by_text;
use crate::search::Line;

/// Cards the opponent holds during your turn (it goes second) and once it
/// has drawn for its own.
const OPENING: usize = 5;
const ITS_TURN: usize = 6;

/// Moments each handtrap is tried at, the first ones of the line.
const MAX_TRIES: usize = 24;

/// Points where the line is stopped to weigh what a drawing handtrap gives.
const MAX_STOPS: usize = 12;

/// Board breakers assumed in a deck whose own the probe turn cannot play
/// (the standard ones stand in for them): about 5 in 40, so that it has
/// one in a bit more than half of its hands.
const GENERIC_OUTS: u32 = 5;

/// The opponent's deck, read once for all the boards measured against it.
pub struct Rival {
    /// Cards in its Main Deck, and the copies of each (by original code).
    size: usize,
    copies: HashMap<u32, u32>,
    /// The standard kit with its breakers instead of the standard ones.
    kit: Kit,
    /// The standard kit: its opponent is the one the lines were found
    /// against, and it measures the boards.
    standard: Kit,
    /// The card each breaker of the kit plays; empty when it has none the
    /// probe turn can play (the standard ones are used then).
    cards: Vec<u32>,
    /// Its handtraps (original codes).
    pub hand_traps: Vec<u32>,
    /// Its Main Deck, what it draws from (Maxx "C" draws many).
    main: Vec<u32>,
}

impl Rival {
    /// Reads `deck`'s breakers and handtraps, against the standard `kit`.
    pub fn new(engine: &Engine, deck: &DeckList, seed: u64, standard: &Kit) -> Rival {
        let kit = standard;
        let db = engine.cards();
        let mut copies: HashMap<u32, u32> = HashMap::new();
        for &c in &deck.main {
            *copies.entry(db.canonical_code(c)).or_default() += 1;
        }
        let mut unique: Vec<u32> = copies.keys().copied().collect();
        unique.sort_unstable();
        let candidates = breakers_by_text(engine, &unique);
        // A card of the standard kit keeps how it is played there (both
        // targets of Lightning Storm, every card Forbidden Droplet can send).
        let mut breakers: Vec<Breaker> = Vec::new();
        let mut cards: Vec<u32> = Vec::new();
        let mut others: Vec<u32> = Vec::new();
        for &c in &candidates {
            let known: Vec<&Breaker> = kit.breakers.iter().filter(|b| b.code().is_some_and(|k| db.canonical_code(k) == c)).collect();
            if known.is_empty() {
                others.push(c);
            }
            for b in known {
                breakers.push(b.clone());
                cards.push(c);
            }
        }
        for (c, summon) in playable(engine, deck, &others, seed) {
            breakers.push(Breaker::card(db.name(c), 1.0, c, summon));
            cards.push(c);
        }
        let kit = if breakers.is_empty() { kit.clone() } else { kit.with_breakers(breakers, deck, seed) };
        let hand_traps = hand_traps(engine, deck, seed, &kit);
        Rival { size: deck.main.len(), copies, kit, cards, hand_traps, standard: standard.clone(), main: deck.main.clone() }
    }

    /// The kit the probe turn plays against it: the standard one with its
    /// breakers and its Deck.
    pub fn kit(&self) -> &Kit {
        &self.kit
    }

    /// The cards played as breakers (none: the standard ones are used).
    pub fn breakers(&self) -> &[u32] {
        &self.cards
    }

    fn copies(&self, code: u32) -> u32 {
        self.copies.get(&code).copied().unwrap_or(0)
    }

    /// Chance that none of these cards is among `drawn` cards of the deck.
    fn none_of(&self, cards: &[u32], drawn: usize) -> f64 {
        none_drawn(self.size, cards.iter().map(|&c| self.copies(c)).sum(), drawn)
    }

    /// The interruption value expected in its turn, holding `cards` cards,
    /// and the chance it has one of `breakers` (the most harmful first): it
    /// plays the i-th when it has it and none of those before, and leaves
    /// `field_value` when it has none.
    fn against(&self, breakers: &[RivalBreaker], field_value: f64, cards: usize) -> (f64, f64) {
        let mut value = 0.0;
        let mut before: Vec<u32> = Vec::new();
        for b in breakers {
            let none_before = self.none_of(&before, cards);
            before.push(b.code);
            value += (none_before - self.none_of(&before, cards)) * b.remaining;
        }
        let none = self.none_of(&before, cards);
        (value + none * field_value, 1.0 - none)
    }
}

/// The opponent's breakers against the board your answers `own` reach,
/// each card played its best way, the most harmful first.
fn rival_breakers(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    rival: &Rival,
    own: &[Vec<u8>],
    measure: &Measure,
) -> Vec<RivalBreaker> {
    let prober = Prober::new(engine, deck, hand, seed, &rival.kit);
    let results = prober.breakers(Prelude::Own(own), &measure.interruptions, &rival.kit.breakers);
    let mut out: Vec<RivalBreaker> = Vec::new();
    for (&code, result) in rival.cards.iter().zip(&results) {
        match out.iter_mut().find(|b| b.code == code) {
            Some(b) if b.remaining <= result.remaining => {}
            Some(b) => {
                b.remaining = result.remaining;
                b.stopped_by = result.stopped_by;
                b.not_applicable = result.not_applicable;
            }
            None => out.push(RivalBreaker {
                code,
                chance: 1.0 - rival.none_of(&[code], ITS_TURN),
                remaining: result.remaining,
                stopped_by: result.stopped_by,
                not_applicable: result.not_applicable,
            }),
        }
    }
    out.sort_by(|a, b| a.remaining.total_cmp(&b.remaining).then(b.chance.total_cmp(&a.chance)));
    out
}

/// One of the opponent's breakers against the board.
#[derive(Debug, Clone)]
pub struct RivalBreaker {
    pub code: u32,
    /// Chance it is among the 6 cards the opponent has in its turn.
    pub chance: f64,
    /// Interruption value of the field left after it (played its best way).
    pub remaining: f64,
    /// One of your effects negated it.
    pub stopped_by: Option<ResponderId>,
    /// It could not be used against this board.
    pub not_applicable: bool,
}

/// One of the opponent's handtraps against the line.
#[derive(Debug, Clone)]
pub struct RivalHandTrap {
    pub code: u32,
    /// Chance it is among the 5 cards the opponent has during your turn.
    pub chance: f64,
    /// Moments of the line the engine let it be used.
    pub windows: u32,
    /// Those where using it changes the field or hand you end with.
    pub stops: u32,
    /// Your cards being played at those moments, the first first.
    pub at: Vec<u32>,
    /// The most cards the opponent draws using it (Maxx "C", the
    /// Mulcharmy), at the moment that draws it most.
    pub drawn: u32,
    /// Each moment tried, in the order of the line.
    pub moments: Vec<HandTrapMoment>,
}

/// A moment of your line the opponent uses a handtrap at.
#[derive(Debug, Clone)]
pub struct HandTrapMoment {
    /// Your card being played then (the last you used).
    pub playing: Option<u32>,
    /// It changes the field or hand you end with.
    pub stops: bool,
    /// Cards the opponent draws from then to the end of your turn, less
    /// those it shuffles back into the Deck.
    pub drawn: u32,
}

/// The board and its line against the opponent's deck.
#[derive(Debug, Clone, Default)]
pub struct Matchup {
    /// Its breakers, the most harmful first.
    pub breakers: Vec<RivalBreaker>,
    /// It has no breaker the probe turn can play: the resilience is the
    /// standard one.
    pub standard_breakers: bool,
    /// Chance it has at least one of its breakers.
    pub breaker_chance: f64,
    /// Interruption value of the field expected after the most harmful
    /// breaker it has (all of it when it has none).
    pub resilience: f64,
    /// Its handtraps that can be used against the line, those that stop
    /// it most first.
    pub hand_traps: Vec<RivalHandTrap>,
    /// Chance it has at least one that stops something.
    pub hand_trap_chance: f64,
}

/// Measures the board reached by `line` (found against the standard kit's
/// opponent, and measured with the standard kit in `measure`) against `rival`.
pub fn matchup(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    line: &Line,
    measure: &Measure,
    rival: &Rival,
) -> Result<Matchup, Error> {
    let own = own_answers(engine, deck, hand, seed, rival.standard.opponent(), line)?;
    let mut out = Matchup { standard_breakers: rival.cards.is_empty(), ..Matchup::default() };

    if out.standard_breakers {
        out.resilience = measure.resilience;
    } else {
        out.breakers = rival_breakers(engine, deck, hand, seed, rival, &own.answers, measure);
        (out.resilience, out.breaker_chance) = rival.against(&out.breakers, measure.field_value, ITS_TURN);
    }

    out.hand_traps = hand_trap_stops(engine, deck, hand, seed, &rival.main, &own, &rival.hand_traps)?;
    for h in &mut out.hand_traps {
        h.chance = 1.0 - rival.none_of(&[h.code], OPENING);
    }
    out.hand_traps.sort_by(|a, b| b.stops.cmp(&a.stops).then(b.chance.total_cmp(&a.chance)).then(a.code.cmp(&b.code)));
    let stopping: Vec<u32> = out.hand_traps.iter().filter(|h| h.stops > 0).map(|h| h.code).collect();
    out.hand_trap_chance = 1.0 - rival.none_of(&stopping, OPENING);
    Ok(out)
}

/// A point where your line can stop and end the turn, while the opponent
/// draws from a handtrap (Maxx "C", a Mulcharmy) used at its first moment.
#[derive(Debug, Clone)]
pub struct StopPoint {
    /// Your answers played before ending the turn (all: the whole line),
    /// the line's choices among them (the steps shown), and your last card
    /// played then.
    pub answers: usize,
    pub choices: usize,
    pub playing: Option<u32>,
    /// Interruption and floodgate value of the board you end on.
    pub field_value: f64,
    pub floodgate_value: f64,
    /// Cards the opponent has drawn by the end of your turn.
    pub drawn: u32,
    /// Value expected in its turn: it holds 6 cards plus those drawn, and
    /// plays the most harmful of its breakers it has. The interruptions
    /// left after it, and the floodgates when it has none (a breaker takes
    /// them too).
    pub expected: f64,
}

/// Where to stop the line against one handtrap that draws.
#[derive(Debug, Clone)]
pub struct DrawTradeOff {
    pub code: u32,
    /// The points tried, in the order of the line, the whole line last.
    pub stops: Vec<StopPoint>,
    /// The one that keeps the most for its turn.
    pub best: usize,
    /// The deck has no breaker the probe turn can play: the standard ones
    /// are used, with [`GENERIC_OUTS`] of them assumed in the deck.
    pub standard_breakers: bool,
}

/// Maxx "C" and the Mulcharmy stop nothing, but each card your line plays
/// after them gives the opponent a card: more chances of a breaker in its
/// turn. For each of its handtraps that draws, used at the first moment it
/// can be, the value your board keeps in its turn if the line stops at each
/// free moment of your Main Phase (and ends the turn there), and the best
/// of them. A few seconds: each stop is measured with the probe turn.
pub fn draw_trade_off(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    line: &Line,
    rival: &Rival,
) -> Result<Vec<DrawTradeOff>, Error> {
    let db = engine.cards();
    let own = own_answers(engine, deck, hand, seed, rival.standard.opponent(), line)?;
    let opponent = Opponent { hand: rival.hand_traps.clone(), deck: rival.main.clone(), extra: Vec::new() };
    let base = replay(engine, deck, hand, seed, &opponent, &own, Play::default())?;
    // The handtraps that draw, at the first moment each can be used.
    let mut drawing: Vec<(u32, usize)> = Vec::new();
    for &code in &rival.hand_traps {
        let Some(w) = base.windows.iter().position(|m| m.offered.iter().any(|(c, _)| *c == code)) else { continue };
        let full = replay(engine, deck, hand, seed, &opponent, &own, Play { use_at: Some((code, w)), stop_at: None })?;
        if full.drawn > 0 {
            drawing.push((code, w));
        }
    }
    if drawing.is_empty() {
        return Ok(Vec::new());
    }

    // The free moments of your Main Phase, at most MAX_STOPS evenly
    // spread, then the whole line.
    let idle: Vec<usize> = (1..own.answers.len()).filter(|&i| own.idle[i]).collect();
    let mut stops: Vec<usize> =
        if idle.len() <= MAX_STOPS { idle } else { (0..MAX_STOPS).map(|k| idle[k * idle.len() / MAX_STOPS]).collect() };
    stops.push(own.answers.len());
    // The board at each stop, measured as the line cut there.
    let mut boards = Vec::new();
    for &s in &stops {
        let cut = own.at_line.get(s).copied().unwrap_or(line.answers.len());
        let part = Line {
            answers: line.answers[..cut].to_vec(),
            choices: line.choices.iter().zip(&line.choice_at).filter(|(_, &at)| at < cut).map(|(c, _)| c.clone()).collect(),
            choice_at: line.choice_at.iter().copied().filter(|&at| at < cut).collect(),
        };
        let choices = part.choice_at.len();
        let measure = Prober::new(engine, deck, hand, seed, &rival.standard).measure(&part)?;
        let breakers =
            if rival.cards.is_empty() { Vec::new() } else { rival_breakers(engine, deck, hand, seed, rival, &own.answers[..s], &measure) };
        // Your last card played before the stop.
        let playing = (0..s).rev().find_map(|i| own.ids[i].map(|id| id.code)).map(|c| db.canonical_code(c));
        boards.push((s, choices, playing, measure, breakers));
    }

    let mut out = Vec::new();
    for (code, w) in drawing {
        let mut points = Vec::new();
        for (s, choices, playing, measure, breakers) in &boards {
            let stop_at = (*s < own.answers.len()).then_some(*s);
            let drawn = replay(engine, deck, hand, seed, &opponent, &own, Play { use_at: Some((code, w)), stop_at })?.drawn;
            let cards = ITS_TURN + drawn as usize;
            let (interruptions, none) = if rival.cards.is_empty() {
                let none = none_drawn(rival.size.max(40), GENERIC_OUTS, cards);
                (none * measure.field_value + (1.0 - none) * measure.resilience, none)
            } else {
                let (value, chance) = rival.against(breakers, measure.field_value, cards);
                (value, 1.0 - chance)
            };
            let expected = interruptions + none * measure.floodgate_value;
            points.push(StopPoint {
                answers: *s,
                choices: *choices,
                playing: *playing,
                field_value: measure.field_value,
                floodgate_value: measure.floodgate_value,
                drawn,
                expected,
            });
        }
        // The best; on a tie, the one giving fewer cards, then the first
        // (as much, with less played).
        let mut best = 0;
        for (i, p) in points.iter().enumerate() {
            let b = &points[best];
            if p.expected > b.expected + 1e-9 || ((p.expected - b.expected).abs() <= 1e-9 && p.drawn < b.drawn) {
                best = i;
            }
        }
        out.push(DrawTradeOff { code, stops: points, best, standard_breakers: rival.cards.is_empty() });
    }
    Ok(out)
}

/// One move of a deck's best lines (the probe turn against a chosen deck,
/// level 2): a card played from somewhere, and how often the lines play it.
#[derive(Debug, Clone)]
pub struct RivalPlay {
    pub code: u32,
    /// Activated, Normal Summoned or Special Summoned.
    pub kind: ActionKind,
    /// Where the card was.
    pub location: u8,
    /// Share of the lines that play it, and that play it first.
    pub share: f64,
    pub first: f64,
    /// Its average place in the lines that play it (1 = first move).
    pub order: f64,
}

/// The best line of a deck from one opening hand: the hand, the moves it
/// plays (a card, how, from where) and its answers, to play it again in the
/// opponent's turn after yours.
#[derive(Debug, Clone)]
pub struct RivalLine {
    pub hand: Vec<u32>,
    pub moves: Vec<(u32, ActionKind, u8)>,
    pub(crate) script: Script,
    /// For each answer of the script, the move it plays (index in `moves`).
    pub(crate) move_of: Vec<Option<usize>>,
}

/// The best lines of `deck` from `hands` random opening hands (searched with
/// `nodes` nodes each), in the order of the hands; a hand with no board has
/// no line.
pub fn rival_lines(engine: &Engine, deck: &DeckList, seed: u64, hands: usize, nodes: u64, threads: usize) -> Vec<RivalLine> {
    // Random hands of 5, the same for the same seed.
    let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    let samples: Vec<Vec<u32>> = (0..hands)
        .map(|_| {
            let mut main = deck.main.clone();
            for i in (1..main.len()).rev() {
                main.swap(i, (next() % (i as u64 + 1)) as usize);
            }
            main.truncate(5);
            main
        })
        .collect();
    let at = std::sync::atomic::AtomicUsize::new(0);
    let found: std::sync::Mutex<Vec<Option<RivalLine>>> = std::sync::Mutex::new(vec![None; samples.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, samples.len().max(1)) {
            s.spawn(|| loop {
                let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(hand) = samples.get(i) else { break };
                let line = rival_line(engine, deck, hand, seed, nodes);
                if let Ok(mut all) = found.lock() {
                    all[i] = line;
                }
            });
        }
    });
    found.into_inner().unwrap_or_default().into_iter().flatten().collect()
}

/// The best line of `deck` from `hand`, searched with `nodes` nodes against
/// the standard kit's opponent, as the Combo Lab searches; `None` when the
/// hand makes no board.
pub fn rival_line(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, nodes: u64) -> Option<RivalLine> {
    let kit = Kit::standard(engine);
    let mut config = crate::search::SearchConfig::new(deck.clone(), hand.to_vec());
    config.seed = seed;
    config.threads = 1;
    config.max_nodes = nodes;
    config.time_limit = std::time::Duration::from_secs(60);
    config.opponent = Some(kit.opponent().clone());
    config.hand_traps = hand_traps(engine, deck, seed, &kit);
    let result = crate::search::search(engine, &config, &std::sync::atomic::AtomicBool::new(false), |_| {});
    let best = result.boards.first()?;
    let script = own_answers(engine, deck, hand, seed, kit.opponent(), &best.line).ok()?;
    let mut moves = Vec::new();
    let move_of = script
        .ids
        .iter()
        .map(|id| {
            let id = id.filter(|id| matches!(id.kind, ActionKind::Activate | ActionKind::NormalSummon | ActionKind::SpecialSummon))?;
            moves.push((id.code, id.kind, id.location));
            Some(moves.len() - 1)
        })
        .collect();
    Some(RivalLine { hand: hand.to_vec(), moves, script, move_of })
}

/// The moves of `deck`'s best lines from `hands` random opening hands, the
/// most played first.
pub fn rival_plays(engine: &Engine, deck: &DeckList, seed: u64, hands: usize, nodes: u64, threads: usize) -> Vec<RivalPlay> {
    plays_of(&rival_lines(engine, deck, seed, hands, nodes, threads))
}

/// The moves of `lines`, each with how often the lines play it.
pub fn plays_of(lines: &[RivalLine]) -> Vec<RivalPlay> {
    let mut found: HashMap<(u32, ActionKind, u8), (u32, u32, usize)> = HashMap::new();
    for line in lines {
        let mut seen: Vec<(u32, ActionKind, u8)> = Vec::new();
        for (i, m) in line.moves.iter().enumerate() {
            if seen.contains(m) {
                continue;
            }
            seen.push(*m);
            let entry = found.entry(*m).or_default();
            entry.0 += 1;
            entry.1 += u32::from(i == 0);
            entry.2 += i + 1;
        }
    }
    let total = lines.len().max(1) as f64;
    let mut plays: Vec<RivalPlay> = found
        .into_iter()
        .map(|((code, kind, location), (lines, first, places))| RivalPlay {
            code,
            kind,
            location,
            share: f64::from(lines) / total,
            first: f64::from(first) / total,
            order: places as f64 / f64::from(lines.max(1)),
        })
        .collect();
    plays.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.order.total_cmp(&b.order)).then(a.code.cmp(&b.code)));
    plays
}

/// One of the opponent's moves as the probe turn plays it.
#[derive(Debug, Clone)]
pub struct RivalMove {
    pub code: u32,
    pub kind: ActionKind,
    pub location: u8,
    /// The line it belongs to (index in the lines given).
    pub line: usize,
    /// Its share of all the moves: every line weighs the same, and every
    /// move of a line the same.
    pub weight: f64,
    /// Made on an empty board, and against this board with no answer.
    pub possible: bool,
    pub here: bool,
}

/// One of your effects against the opponent's lines.
#[derive(Debug, Clone)]
pub struct MoveStop {
    pub responder: ResponderId,
    pub location: u8,
    pub effect: Effect,
    /// The moves it stops (indices in [`MovesMatchup::moves`]).
    pub stops: Vec<usize>,
    /// Share of the moves it takes away, in each line at its best moment,
    /// averaged over the lines (0 in a line where it takes nothing).
    pub coverage: f64,
    pub value: f64,
}

/// A board against what a deck does in its turn: its best lines played
/// again after yours (level 2 of the probe turn against a deck).
#[derive(Debug, Clone, Default)]
pub struct MovesMatchup {
    /// Lines measured (a line whose run failed is left out).
    pub lines: usize,
    pub moves: Vec<RivalMove>,
    pub stops: Vec<MoveStop>,
    /// Moves your board stops with no answer (indices in `moves`), and
    /// their share of the moves the lines make on an empty board.
    pub floodgated: Vec<usize>,
    pub floodgate_share: f64,
    /// Interruption value against these lines, averaged over them.
    pub value: f64,
    /// Share of the moves the board lets the lines make that they still
    /// make with all your answers used together, averaged over the lines.
    pub left: f64,
}

/// Measures the board reached by `line` (found against the standard kit's
/// opponent) against `lines` of `rival` ([`rival_lines`]), each played
/// again in the opponent's turn after yours with the line's hand and Deck:
/// which of its moves each of your effects stops, and which your board
/// stops with no answer.
#[allow(clippy::too_many_arguments)]
pub fn against_moves(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    line: &Line,
    rival: &DeckList,
    lines: &[RivalLine],
    standard: &Kit,
) -> Result<MovesMatchup, Error> {
    let own = own_answers(engine, deck, hand, seed, standard.opponent(), line)?;
    let mut measured = Vec::new();
    for (index, rival_line) in lines.iter().enumerate().filter(|(_, l)| !l.moves.is_empty()) {
        let kit = standard.with_line(engine, rival, rival_line, seed);
        let weights = vec![1.0; rival_line.moves.len()];
        match Prober::new(engine, deck, hand, seed, &kit).against_moves(Prelude::Own(&own.answers), &weights) {
            Ok(m) => measured.push((index, rival_line, m)),
            Err(e) if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() => eprintln!("   a line of the rival not measured: {e}"),
            Err(_) => {}
        }
    }
    let n = measured.len();
    let mut out = MovesMatchup { lines: n, left: if n == 0 { 1.0 } else { 0.0 }, ..MovesMatchup::default() };
    for (index, rival_line, m) in measured {
        let share = 1.0 / n as f64;
        let base = out.moves.len();
        for (i, &(code, kind, location)) in rival_line.moves.iter().enumerate() {
            out.moves.push(RivalMove {
                code,
                kind,
                location,
                line: index,
                weight: share / rival_line.moves.len() as f64,
                possible: m.possible[i],
                here: m.here[i],
            });
        }
        out.floodgated.extend(m.floodgated.iter().map(|i| base + i));
        out.floodgate_share += share * m.floodgate_share;
        out.value += share * m.value;
        out.left += share * m.left;
        for i in m.interruptions {
            let stops = i.stops.iter().map(|s| base + s);
            match out.stops.iter_mut().find(|s| s.responder == i.responder) {
                Some(s) => {
                    s.stops.extend(stops);
                    s.coverage += share * i.coverage;
                    s.value += share * i.value;
                }
                None => out.stops.push(MoveStop {
                    responder: i.responder,
                    location: i.location,
                    effect: i.effect,
                    stops: stops.collect(),
                    coverage: share * i.coverage,
                    value: share * i.value,
                }),
            }
        }
    }
    out.stops.sort_by(|a, b| b.value.total_cmp(&a.value));
    Ok(out)
}

/// Chance that none of `copies` cards of a deck of `size` is among `drawn`.
pub(crate) fn none_drawn(size: usize, copies: u32, drawn: usize) -> f64 {
    (0..drawn).map(|i| (size as f64 - f64::from(copies) - i as f64).max(0.0) / size.saturating_sub(i).max(1) as f64).product()
}

/// A moment the opponent could respond: the handtraps offered (and where
/// in the list), and your card being played then.
struct Moment {
    offered: Vec<(u32, usize)>,
    playing: Option<u32>,
}

/// Your line played again with the opponent holding its handtraps.
#[derive(Default)]
struct Replay {
    windows: Vec<Moment>,
    /// Your field and hand at the end.
    end: Vec<(u8, u32, u32)>,
    /// The line could not be played on as it was.
    diverged: bool,
    /// Cards the opponent drew during your turn, less those it shuffled
    /// back from its hand into the Deck (the hand limit of a Mulcharmy).
    drawn: u32,
}

/// How a replay goes: the opponent uses a handtrap (`use_at`: the card, at
/// a window of the replay where none is used) or passes everywhere; your
/// line stops at an answer (`stop_at`) and ends the turn, or goes on.
#[derive(Debug, Clone, Copy, Default)]
struct Play {
    use_at: Option<(u32, usize)>,
    stop_at: Option<usize>,
}

/// Plays your answers with the opponent holding `hand_traps`, as `play` says.
fn replay(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: &Opponent,
    own: &Script,
    play: Play,
) -> Result<Replay, Error> {
    let db = engine.cards();
    let mut duel = Duel::goldfish_against(engine, deck, hand, seed, Some(opponent))?;
    let mut out = Replay::default();
    let mut next = 0;
    let mut used = false;
    let mut last: Option<u32> = None;
    // The last link of the chain is the opponent's (a handtrap used).
    let mut theirs_on_top = false;
    // Your last decision, answered with the line's answer.
    let mut asked: Option<Decision> = None;
    let mut rejected = 0;
    // The opponent's cards drawn until its turn begins.
    let mut drawn: i64 = 0;
    let mut over = false;
    for _ in 0..4000 {
        let step = match duel.advance() {
            // After a handtrap your line may be asked something new, of the
            // same kind as its next answer, that does not fit it: you answer
            // it simply, and the line goes on.
            Err(Error::Rejected(_)) if used && rejected < 20 => {
                rejected += 1;
                let Some(decision) = asked.take() else { break };
                next -= 1;
                duel.respond(&passive_answer(&decision, &duel.actions(&decision)));
                continue;
            }
            step => step?,
        };
        let (Step::Decision { events, .. } | Step::End { events }) = &step;
        for e in events {
            match e {
                _ if over => break,
                Event::NewTurn { player: 1 } => over = true,
                Event::Draw { player: 1, codes } => drawn += codes.len() as i64,
                Event::Move { from, to, .. }
                    if from.controller == 1 && from.location == location::HAND && to.location == location::DECK =>
                {
                    drawn -= 1;
                }
                _ => {}
            }
        }
        let Step::Decision { decision, events } = step else { break };
        if duel.turn_info().0 > 1 {
            break;
        }
        for e in &events {
            match e {
                Event::Chaining { card, .. }
                | Event::NormalSummoning { card }
                | Event::FlipSummoning { card }
                | Event::SpecialSummoning { card }
                    if card.controller == 0 =>
                {
                    last = Some(db.canonical_code(card.code));
                }
                _ => {}
            }
            match e {
                Event::Chaining { card, .. } => theirs_on_top = card.controller == 1,
                Event::ChainEnd => theirs_on_top = false,
                _ => {}
            }
        }
        let actions = duel.actions(&decision);
        if decision.player() != Some(1) && play.stop_at.is_some_and(|s| next >= s) {
            // The line stops here: the turn ends.
            duel.respond(&end_turn(&decision, &actions));
            continue;
        }
        if decision.player() != Some(1) {
            if used {
                let kind = std::mem::discriminant(&decision);
                // Passes of the line the engine no longer asks for are left
                // out first: the question after them is the line's own.
                while next < own.answers.len() && own.kinds[next] != kind && own.answers[next] == PASS {
                    next += 1;
                }
                let mut expected = own.ids.get(next).copied().flatten();
                // A pass of the line where its next action is offered
                // already (the handtrap's chain took a window the line
                // passed in): the action is played, the passes left out.
                if expected.is_none() && own.answers.get(next) == Some(&PASS.to_vec()) {
                    let after = (next..own.answers.len()).find(|&k| own.answers[k] != PASS);
                    if let Some((k, id)) = after.and_then(|k| own.ids[k].map(|id| (k, id))) {
                        if find_again(engine, &decision, &actions, id).is_some() {
                            next = k;
                            expected = Some(id);
                        }
                    }
                }
                // The line's next action, found again by what it is.
                if let Some(answer) = expected.and_then(|id| find_again(engine, &decision, &actions, id)) {
                    duel.respond(&answer);
                    asked = Some(decision);
                    next += 1;
                    continue;
                }
                // A question the line was not asked here (to respond to the
                // handtrap, a trigger it brought, the End Phase asked again):
                // you decline it, and the line goes on. A chance to chain to
                // the handtrap is one; a yes or no of your effect resolving
                // under it is the line's own.
                let same_question = own.kinds.get(next) == Some(&kind) && expected.is_none();
                let to_theirs = theirs_on_top && matches!(decision, Decision::SelectChain { .. });
                if declinable(&decision) && (to_theirs || !same_question) {
                    duel.respond(&passive_answer(&decision, &actions));
                    continue;
                }
                // Asked something else, or the line's action is gone: the
                // handtrap changed the line.
                if own.kinds.get(next).is_some_and(|k| *k != kind) || expected.is_some() {
                    if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() {
                        eprintln!("   diverged at answer {next}: asked {decision:?}, line had {:?} {:?}", own.answers[next], expected);
                    }
                    out.diverged = true;
                    break;
                }
            }
            let Some(answer) = own.answers.get(next) else { break };
            // After a handtrap, the cards the line picks are found again by
            // what they are, not by their place in the list.
            let again = own.picks.get(next).and_then(Option::as_ref).filter(|_| used).and_then(|p| pick_again(engine, &decision, p));
            duel.respond(again.as_ref().unwrap_or(answer));
            asked = Some(decision);
            next += 1;
            continue;
        }
        if used {
            // The handtrap's own choices: your cards first.
            let answer = match &decision {
                Decision::SelectChain { forced: true, .. } => actions
                    .iter()
                    .find(|a| a.kind == ActionKind::Activate)
                    .map_or_else(|| window_answer(&decision, None), |a| a.response.clone()),
                Decision::SelectChain { .. } => window_answer(&decision, None),
                _ => forward(&decision, &actions, |a| a.card.is_some_and(|c| c.controller == 0)).unwrap_or_else(|| {
                    let pick = actions
                        .iter()
                        .find(|a| a.kind == ActionKind::Yes)
                        .or_else(|| actions.iter().find(|a| a.kind != ActionKind::Cancel && a.card.is_some_and(|c| c.controller == 0)))
                        .or_else(|| actions.iter().find(|a| a.kind != ActionKind::Cancel))
                        .or(actions.first());
                    pick.map_or_else(|| passive_answer(&decision, &actions), |a| a.response.clone())
                }),
            };
            duel.respond(&answer);
            continue;
        }
        let offered: Vec<(u32, usize)> = match &decision {
            Decision::SelectChain { chains, .. } => chains
                .iter()
                .enumerate()
                .filter(|(_, a)| a.card.location == location::HAND)
                .map(|(i, a)| (db.canonical_code(a.card.code), i))
                .collect(),
            Decision::EffectYesNo { card, .. } if card.location == location::HAND => vec![(db.canonical_code(card.code), 0)],
            _ => Vec::new(),
        };
        let pick = play.use_at.filter(|(_, w)| *w == out.windows.len()).and_then(|(code, _)| offered.iter().find(|(c, _)| *c == code));
        match pick {
            Some(&(_, i)) => {
                used = true;
                duel.respond(&window_answer(&decision, Some(i)));
            }
            None => duel.respond(&passive_answer(&decision, &actions)),
        }
        out.windows.push(Moment { offered, playing: last });
    }
    out.drawn = drawn.max(0) as u32;
    let me = &duel.state().players[0];
    out.end = me
        .monsters
        .iter()
        .chain(&me.spells)
        .flatten()
        .map(|c| (location::MZONE, db.canonical_code(c.code), c.position))
        .chain(me.hand.iter().map(|&c| (location::HAND, db.canonical_code(c), 0)))
        .collect();
    out.end.sort_unstable();
    Ok(out)
}

/// For each of `hand_traps` offered during your line: the moments it is
/// offered, those where using it changes the field or hand you end with
/// (it stops something; Maxx "C" makes the opponent draw but stops nothing)
/// and your cards being played then.
fn hand_trap_stops(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    rival: &[u32],
    own: &Script,
    hand_traps: &[u32],
) -> Result<Vec<RivalHandTrap>, Error> {
    // Its own Deck: the kit's few cards run out when it draws (Maxx "C").
    let opponent = Opponent { hand: hand_traps.to_vec(), deck: rival.to_vec(), extra: Vec::new() };
    let base = replay(engine, deck, hand, seed, &opponent, own, Play::default())?;
    let mut out = Vec::new();
    for &code in hand_traps {
        let windows: Vec<usize> = (0..base.windows.len()).filter(|&w| base.windows[w].offered.iter().any(|(c, _)| *c == code)).collect();
        if windows.is_empty() {
            continue;
        }
        let mut stops = 0;
        let mut at = Vec::new();
        let mut moments = Vec::new();
        for &w in windows.iter().take(MAX_TRIES) {
            let tried = replay(engine, deck, hand, seed, &opponent, own, Play { use_at: Some((code, w)), stop_at: None });
            if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() {
                let db = engine.cards();
                match &tried {
                    Ok(r) if !missing(&base.end, &r.end).is_empty() => {
                        let gone: Vec<String> = base.end.iter().filter(|c| !r.end.contains(c)).map(|c| db.name(c.1)).collect();
                        let new: Vec<String> = r.end.iter().filter(|c| !base.end.contains(c)).map(|c| db.name(c.1)).collect();
                        eprintln!("   {} at {w}: lost {gone:?} got {new:?}", db.name(code));
                    }
                    Ok(r) => eprintln!("   {} at {w}: diverged {}", db.name(code), r.diverged),
                    Err(e) => eprintln!("   {} at {w}: {e}", db.name(code)),
                }
            }
            // Stopped: something of your field or hand is missing at the end
            // (ending with more, as when it lets you summon one more card, is not).
            let drawn = tried.as_ref().map_or(0, |r| r.drawn);
            let stopped = tried.ok().is_none_or(|r| r.diverged || !missing(&base.end, &r.end).is_empty());
            if stopped {
                stops += 1;
                if let Some(card) = base.windows[w].playing.filter(|c| !at.contains(c)) {
                    at.push(card);
                }
            }
            moments.push(HandTrapMoment { playing: base.windows[w].playing, stops: stopped, drawn });
        }
        let drawn = moments.iter().map(|m| m.drawn).max().unwrap_or(0);
        out.push(RivalHandTrap { code, chance: 0.0, windows: windows.len() as u32, stops, at, drawn, moments });
    }
    Ok(out)
}

/// The cards of `before` missing from `after` (both sorted), copies counted.
fn missing<T: Ord + Copy>(before: &[T], after: &[T]) -> Vec<T> {
    let mut out = Vec::new();
    let mut rest = after.iter().peekable();
    for b in before {
        while rest.peek().is_some_and(|a| *a < b) {
            rest.next();
        }
        if rest.peek() == Some(&b) {
            rest.next();
        } else {
            out.push(*b);
        }
    }
    out
}

/// The cards among `cards` the opponent can play as a breaker from its
/// hand, and whether by Special Summoning them: those the engine offers in
/// its Main Phase against a field of three monsters and two Set cards, and
/// that take something off that field when played (a card less, or one
/// negated). A card that only lowers ATK, or destroys in battle, is no
/// breaker. The opponent has its own Main and Extra Deck: some pay a cost
/// from there.
fn playable(engine: &Engine, rival: &DeckList, cards: &[u32], seed: u64) -> Vec<(u32, bool)> {
    let code = |name: &str| engine.cards().find_by_name(name).map(|c| c.code);
    // An Effect Monster, so that negating it shows (its effect works only
    // in the hand), a Fusion and a Link: some breakers take only a monster
    // of the kind they send from the Extra Deck (Ultimate Slayer).
    let (Some(filler), Some(monster), Some(set)) = (code("Alexandrite Dragon"), code("Thunder Dragon"), code("Trap Hole")) else {
        return Vec::new();
    };
    let monsters = [monster, code("Flame Swordsman").unwrap_or(monster), code("Link Spider").unwrap_or(monster)];
    if cards.is_empty() {
        return Vec::new();
    }
    let place =
        |code: u32, location: u8, sequence: u32, position: u32| Placement { code, location, sequence, position, overlay: Vec::new() };
    let placed: Vec<Placement> = (0..3)
        .map(|i| place(monsters[i as usize], location::MZONE, i, position::FACEUP_ATTACK))
        .chain((1..3).map(|i| place(set, location::SZONE, i, position::FACEDOWN_DEFENSE)))
        .collect();
    let deck = DeckList { main: vec![filler; 40], ..DeckList::default() };
    // A plain card on top of its Deck: the one it draws for its turn, so
    // that no card a breaker needs from the Deck is drawn.
    let mut rival_deck = rival.main.clone();
    rival_deck.push(filler);
    let opponent = Opponent { hand: cards.to_vec(), deck: rival_deck, extra: rival.extra.clone() };
    // The duel at the opponent's first Main Phase.
    let start = || -> Option<(Duel, Decision)> {
        let mut duel = Duel::goldfish_vs_placed(engine, &deck, &[], seed, &opponent, &placed).ok()?;
        for _ in 0..200 {
            let Ok(Step::Decision { decision, .. }) = duel.advance() else { return None };
            if duel.turn_info().0 > 1 && decision.player() == Some(1) && matches!(decision, Decision::IdleCmd { .. }) {
                return Some((duel, decision));
            }
            let actions = duel.actions(&decision);
            let answer = if decision.player() == Some(1) { passive_answer(&decision, &actions) } else { end_turn(&decision, &actions) };
            duel.respond(&answer);
        }
        None
    };
    let Some((duel, decision)) = start() else { return Vec::new() };
    let actions = duel.actions(&decision);
    let offered: Vec<(u32, bool)> = cards
        .iter()
        .filter_map(|&c| {
            if find_act(engine, &actions, Act::Activate(c, location::HAND)).is_some() {
                Some((c, false))
            } else {
                find_act(engine, &actions, Act::SpecialSummon(c)).map(|_| (c, true))
            }
        })
        .collect();
    if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() {
        let names = |codes: Vec<u32>| codes.iter().map(|&c| engine.cards().name(c)).collect::<Vec<_>>();
        eprintln!("breaker candidates {:?}, played {:?}", names(cards.to_vec()), names(offered.iter().map(|o| o.0).collect()));
    }
    let yours: Vec<u32> = placed.iter().map(|p| p.code).collect();
    offered.into_iter().filter(|&(c, summon)| takes_from_field(engine, &start, &yours, c, summon)).collect()
}

/// Plays `code` (Special Summoned, or activated from the hand) in the duel
/// `start` gives, the opponent going for your cards: true when your field
/// is left with one of `yours` less, or one negated (a Kaiju given to you
/// in place of the monster it tributes does not count).
fn takes_from_field(engine: &Engine, start: &dyn Fn() -> Option<(Duel, Decision)>, yours: &[u32], code: u32, summon: bool) -> bool {
    let db = engine.cards();
    let standing = |duel: &Duel| -> usize {
        let me = &duel.state().players[0];
        me.monsters.iter().chain(&me.spells).flatten().filter(|c| yours.contains(&c.code) && !c.is_negated()).count()
    };
    let Some((mut duel, decision)) = start() else { return false };
    let before = standing(&duel);
    let act = if summon { Act::SpecialSummon(code) } else { Act::Activate(code, location::HAND) };
    let Some(answer) = find_act(engine, &duel.actions(&decision), act) else { return false };
    duel.respond(&answer);
    let its = |c: &CardRef| db.canonical_code(c.code) == db.canonical_code(code);
    let mut again = true;
    for _ in 0..100 {
        let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
        let actions = duel.actions(&decision);
        let answer = if decision.player() != Some(1) {
            passive_answer(&decision, &actions)
        } else if let Decision::IdleCmd { .. } = decision {
            // Back in its Main Phase: the card's effect on the field once
            // (summoned, then it destroys), else it is done.
            let on_field = [location::MZONE, location::SZONE].into_iter().find_map(|l| find_act(engine, &actions, Act::Activate(code, l)));
            match on_field.filter(|_| std::mem::take(&mut again)) {
                Some(a) => a,
                None => break,
            }
        } else {
            going_for_you(&decision, &actions, its)
        };
        duel.respond(&answer);
    }
    standing(&duel) < before
}

/// The opponent's answer while its breaker is played: its card's effects
/// chained, yes, and your cards picked when it picks (else anything but
/// cancelling).
fn going_for_you(decision: &Decision, actions: &[Action], its: impl Fn(&CardRef) -> bool) -> Vec<u8> {
    let yours = |a: &Action| a.card.is_some_and(|c| c.controller == 0);
    if let Some(answer) = forward(decision, actions, yours) {
        return answer;
    }
    // The cards an answer to a selection picks, by index.
    let picked = |a: &Action| -> Vec<usize> {
        let word = |i: usize| a.response.get(i * 4..i * 4 + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        (2..).map_while(word).collect()
    };
    let all_yours = |cards: Vec<&CardRef>| {
        actions.iter().find(|a| {
            let p = picked(a);
            !p.is_empty() && p.iter().all(|&i| cards.get(i).is_some_and(|c| c.controller == 0))
        })
    };
    let pick = match decision {
        Decision::SelectChain { .. } => actions.iter().find(|a| a.kind == ActionKind::Activate && a.card.as_ref().is_some_and(&its)),
        Decision::EffectYesNo { .. } | Decision::YesNo { .. } => actions.iter().find(|a| a.kind == ActionKind::Yes),
        Decision::SelectCard { cards, .. } => {
            all_yours(cards.iter().collect()).or_else(|| actions.iter().find(|a| a.kind != ActionKind::Cancel))
        }
        Decision::SelectTribute { cards, .. } => {
            all_yours(cards.iter().map(|(c, _)| c).collect()).or_else(|| actions.iter().find(|a| a.kind != ActionKind::Cancel))
        }
        _ => None,
    };
    pick.map_or_else(|| passive_answer(decision, actions), |a| a.response.clone())
}

#[cfg(test)]
mod tests {
    use super::none_drawn;

    #[test]
    fn missing_counts_copies() {
        assert_eq!(super::missing(&[1, 2, 2, 3], &[2, 3, 4]), vec![1, 2]);
        assert!(super::missing(&[1, 2], &[0, 1, 2, 5]).is_empty());
    }

    #[test]
    fn chance_of_drawing_none() {
        // Three copies in 40 cards, 6 drawn: 1 - (34·33·32)/(40·39·38).
        assert!((none_drawn(40, 3, 6) - 34.0 * 33.0 * 32.0 / (40.0 * 39.0 * 38.0)).abs() < 1e-12);
        assert_eq!(none_drawn(40, 0, 6), 1.0);
        assert_eq!(none_drawn(40, 40, 6), 0.0);
    }
}
