//! The probe turn against a chosen deck (#44, level 1): the board is put to
//! the test by the board breakers of that deck, and the line by its
//! handtraps, each weighted by the chance the opponent has it in hand.
//! Interruptions and floodgates stay measured with the standard kit.
//!
//! Both are read from the engine: the breakers are the cards of the deck
//! that take or negate the opponent's cards (by text, as the roles do) and
//! that the engine lets it play from the hand; the handtraps are those it
//! can use from the hand during your turn ([`hand_traps`]).

use std::collections::HashMap;
use std::mem::Discriminant;

use ygo_engine::auto::passive_answer;
use ygo_engine::consts::{location, position};
use ygo_engine::{ActionKind, Decision, DeckList, Duel, Engine, Error, Event, Opponent, Placement, Step};

use crate::probe::{end_turn, find_act, forward, hand_traps, window_answer, Act, Breaker, Kit, Measure, Prelude, Prober, ResponderId};
use crate::roles::breakers_by_text;
use crate::search::Line;

/// Cards the opponent holds during your turn (it goes second) and once it
/// has drawn for its own.
const OPENING: usize = 5;
const ITS_TURN: usize = 6;

/// Moments each handtrap is tried at, the first ones of the line.
const MAX_TRIES: usize = 24;

/// The opponent's deck, read once for all the boards measured against it.
pub struct Rival {
    /// Cards in its Main Deck, and the copies of each (by original code).
    size: usize,
    copies: HashMap<u32, u32>,
    /// The standard kit with its breakers instead of the standard ones.
    kit: Kit,
    /// The standard kit's opponent: the one the lines were found against.
    standard: Opponent,
    /// The card each breaker of the kit plays; empty when it has none the
    /// probe turn can play (the standard ones are used then).
    cards: Vec<u32>,
    /// Its handtraps (original codes).
    pub hand_traps: Vec<u32>,
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
        for (c, summon) in playable(engine, kit, &others, seed) {
            breakers.push(Breaker::card(db.name(c), 1.0, c, summon));
            cards.push(c);
        }
        let kit = if breakers.is_empty() { kit.clone() } else { kit.with_breakers(breakers) };
        let hand_traps = hand_traps(engine, deck, seed, &kit);
        Rival { size: deck.main.len(), copies, kit, cards, hand_traps, standard: standard.opponent().clone() }
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
    let own = own_answers(engine, deck, hand, seed, &rival.standard, line)?;
    let mut out = Matchup { standard_breakers: rival.cards.is_empty(), ..Matchup::default() };

    if out.standard_breakers {
        out.resilience = measure.resilience;
    } else {
        let prober = Prober::new(engine, deck, hand, seed, &rival.kit);
        let results = prober.breakers(Prelude::Own(&own.answers), &measure.interruptions, &rival.kit.breakers);
        // Each card played its best way for the opponent.
        for (&code, result) in rival.cards.iter().zip(&results) {
            match out.breakers.iter_mut().find(|b| b.code == code) {
                Some(b) if b.remaining <= result.remaining => {}
                Some(b) => {
                    b.remaining = result.remaining;
                    b.stopped_by = result.stopped_by;
                    b.not_applicable = result.not_applicable;
                }
                None => out.breakers.push(RivalBreaker {
                    code,
                    chance: 1.0 - rival.none_of(&[code], ITS_TURN),
                    remaining: result.remaining,
                    stopped_by: result.stopped_by,
                    not_applicable: result.not_applicable,
                }),
            }
        }
        out.breakers.sort_by(|a, b| a.remaining.total_cmp(&b.remaining).then(b.chance.total_cmp(&a.chance)));
        // The opponent plays the most harmful breaker it has: the i-th
        // when it has it and none of those before.
        let mut before: Vec<u32> = Vec::new();
        for b in &out.breakers {
            let none_before = rival.none_of(&before, ITS_TURN);
            before.push(b.code);
            out.resilience += (none_before - rival.none_of(&before, ITS_TURN)) * b.remaining;
        }
        out.resilience += rival.none_of(&before, ITS_TURN) * measure.field_value;
        out.breaker_chance = 1.0 - rival.none_of(&before, ITS_TURN);
    }

    out.hand_traps = hand_trap_stops(engine, deck, hand, seed, &rival.kit, &own, &rival.hand_traps)?;
    for h in &mut out.hand_traps {
        h.chance = 1.0 - rival.none_of(&[h.code], OPENING);
    }
    out.hand_traps.sort_by(|a, b| b.stops.cmp(&a.stops).then(b.chance.total_cmp(&a.chance)).then(a.code.cmp(&b.code)));
    let stopping: Vec<u32> = out.hand_traps.iter().filter(|h| h.stops > 0).map(|h| h.code).collect();
    out.hand_trap_chance = 1.0 - rival.none_of(&stopping, OPENING);
    Ok(out)
}

/// One move of a deck's typical opening (the probe turn against a chosen
/// deck, level 2): a card played from somewhere, and how often its best
/// lines play it.
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

/// The moves of `deck`'s best lines from `hands` random opening hands
/// (searched with `nodes` nodes each), the most played first.
pub fn rival_plays(engine: &Engine, deck: &DeckList, seed: u64, hands: usize, nodes: u64, threads: usize) -> Vec<RivalPlay> {
    let db = engine.cards();
    let kit = Kit::standard(engine);
    let traps = hand_traps(engine, deck, seed, &kit);
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
    let lines: std::sync::Mutex<Vec<Vec<(u32, ActionKind, u8)>>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, samples.len().max(1)) {
            s.spawn(|| loop {
                let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(hand) = samples.get(i) else { break };
                let mut config = crate::search::SearchConfig::new(deck.clone(), hand.clone());
                config.seed = seed;
                config.threads = 1;
                config.max_nodes = nodes;
                config.time_limit = std::time::Duration::from_secs(60);
                config.opponent = Some(kit.opponent().clone());
                config.hand_traps = traps.clone();
                let result = crate::search::search(engine, &config, &std::sync::atomic::AtomicBool::new(false), |_| {});
                let Some(best) = result.boards.first() else { continue };
                let Ok(mut duel) = Duel::goldfish_against(engine, deck, hand, seed, Some(kit.opponent())) else { continue };
                let mut moves = Vec::new();
                for answer in &best.line.answers {
                    let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
                    if decision.player() == Some(0) && matches!(decision, Decision::IdleCmd { .. } | Decision::SelectChain { .. }) {
                        let actions = duel.actions(&decision);
                        if let Some(a) = actions.iter().find(|a| &a.response == answer) {
                            if let (ActionKind::Activate | ActionKind::NormalSummon | ActionKind::SpecialSummon, Some(card)) =
                                (a.kind, a.card)
                            {
                                moves.push((db.canonical_code(card.code), a.kind, card.location));
                            }
                        }
                    }
                    duel.respond(answer);
                }
                if let Ok(mut all) = lines.lock() {
                    all.push(moves);
                }
            });
        }
    });
    let lines = lines.into_inner().unwrap_or_default();
    let mut found: HashMap<(u32, ActionKind, u8), (u32, u32, usize)> = HashMap::new();
    for line in &lines {
        let mut seen: Vec<(u32, ActionKind, u8)> = Vec::new();
        for (i, m) in line.iter().enumerate() {
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

/// Chance that none of `copies` cards of a deck of `size` is among `drawn`.
pub(crate) fn none_drawn(size: usize, copies: u32, drawn: usize) -> f64 {
    (0..drawn).map(|i| (size as f64 - f64::from(copies) - i as f64).max(0.0) / size.saturating_sub(i).max(1) as f64).product()
}

/// Your answers of a line, without the opponent's (with another deck the
/// opponent is asked at other moments), each with the kind of decision it
/// answers.
struct Own {
    answers: Vec<Vec<u8>>,
    kinds: Vec<Discriminant<Decision>>,
}

fn own_answers(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &Opponent, line: &Line) -> Result<Own, Error> {
    let mut duel = Duel::goldfish_against(engine, deck, hand, seed, Some(opponent))?;
    let mut own = Own { answers: Vec::new(), kinds: Vec::new() };
    for answer in &line.answers {
        let Step::Decision { decision, .. } = duel.advance()? else { break };
        if decision.player() != Some(1) {
            own.answers.push(answer.clone());
            own.kinds.push(std::mem::discriminant(&decision));
        }
        duel.respond(answer);
    }
    Ok(own)
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
}

/// Passing in a chain window.
const PASS: [u8; 4] = (-1i32).to_le_bytes();

/// Plays your answers with the opponent holding `hand_traps`, passing
/// everywhere, or using `use_at` (a handtrap, at a window of the replay
/// where none is used).
fn replay(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: &Opponent,
    own: &Own,
    use_at: Option<(u32, usize)>,
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
        if decision.player() != Some(1) {
            if used {
                let kind = std::mem::discriminant(&decision);
                // You do not respond to the handtrap, nor once it resolved
                // where the line was not asked to (the line never did).
                let window = matches!(decision, Decision::SelectChain { triggers: false, .. });
                if window && (theirs_on_top || own.kinds.get(next).is_some_and(|k| *k != kind)) {
                    duel.respond(&window_answer(&decision, None));
                    continue;
                }
                // Passes of the line the engine no longer asks for are left out.
                while next < own.answers.len() && own.kinds[next] != kind && own.answers[next] == PASS {
                    next += 1;
                }
                // Asked something else: the handtrap changed the line.
                if own.kinds.get(next).is_some_and(|k| *k != kind) {
                    if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() {
                        eprintln!("   diverged at answer {next}: asked {decision:?}, line had {:?}", own.answers[next]);
                    }
                    out.diverged = true;
                    break;
                }
            }
            let Some(answer) = own.answers.get(next) else { break };
            duel.respond(answer);
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
        let pick = use_at.filter(|(_, w)| *w == out.windows.len()).and_then(|(code, _)| offered.iter().find(|(c, _)| *c == code));
        match pick {
            Some(&(_, i)) => {
                used = true;
                duel.respond(&window_answer(&decision, Some(i)));
            }
            None => duel.respond(&passive_answer(&decision, &actions)),
        }
        out.windows.push(Moment { offered, playing: last });
    }
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
    kit: &Kit,
    own: &Own,
    hand_traps: &[u32],
) -> Result<Vec<RivalHandTrap>, Error> {
    let opponent = Opponent { hand: hand_traps.to_vec(), deck: kit.opponent().deck.clone(), extra: Vec::new() };
    let base = replay(engine, deck, hand, seed, &opponent, own, None)?;
    let mut out = Vec::new();
    for &code in hand_traps {
        let windows: Vec<usize> = (0..base.windows.len()).filter(|&w| base.windows[w].offered.iter().any(|(c, _)| *c == code)).collect();
        if windows.is_empty() {
            continue;
        }
        let mut stops = 0;
        let mut at = Vec::new();
        for &w in windows.iter().take(MAX_TRIES) {
            let tried = replay(engine, deck, hand, seed, &opponent, own, Some((code, w)));
            if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() {
                let db = engine.cards();
                match &tried {
                    Ok(r) if r.end != base.end => {
                        let gone: Vec<String> = base.end.iter().filter(|c| !r.end.contains(c)).map(|c| db.name(c.1)).collect();
                        let new: Vec<String> = r.end.iter().filter(|c| !base.end.contains(c)).map(|c| db.name(c.1)).collect();
                        eprintln!("   {} at {w}: lost {gone:?} got {new:?}", db.name(code));
                    }
                    Ok(r) => eprintln!("   {} at {w}: diverged {}", db.name(code), r.diverged),
                    Err(e) => eprintln!("   {} at {w}: {e}", db.name(code)),
                }
            }
            let stopped = tried.ok().is_none_or(|r| r.diverged || r.end != base.end);
            if stopped {
                stops += 1;
                if let Some(card) = base.windows[w].playing.filter(|c| !at.contains(c)) {
                    at.push(card);
                }
            }
        }
        out.push(RivalHandTrap { code, chance: 0.0, windows: windows.len() as u32, stops, at });
    }
    Ok(out)
}

/// The cards among `cards` the opponent can play as a breaker from its
/// hand, and whether by Special Summoning them: those the engine offers in
/// its Main Phase against a board of three monsters and two Set cards.
fn playable(engine: &Engine, kit: &Kit, cards: &[u32], seed: u64) -> Vec<(u32, bool)> {
    let code = |name: &str| engine.cards().find_by_name(name).map(|c| c.code);
    let (Some(monster), Some(set)) = (code("Alexandrite Dragon"), code("Trap Hole")) else { return Vec::new() };
    if cards.is_empty() {
        return Vec::new();
    }
    let place =
        |code: u32, location: u8, sequence: u32, position: u32| Placement { code, location, sequence, position, overlay: Vec::new() };
    let placed: Vec<Placement> = (0..3)
        .map(|i| place(monster, location::MZONE, i, position::FACEUP_ATTACK))
        .chain((1..3).map(|i| place(set, location::SZONE, i, position::FACEDOWN_DEFENSE)))
        .collect();
    let deck = DeckList { main: vec![monster; 40], ..DeckList::default() };
    let opponent = Opponent { hand: cards.to_vec(), deck: kit.opponent().deck.clone(), extra: Vec::new() };
    let Ok(mut duel) = Duel::goldfish_vs_placed(engine, &deck, &[], seed, &opponent, &placed) else { return Vec::new() };
    for _ in 0..200 {
        let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
        let actions = duel.actions(&decision);
        if duel.turn_info().0 > 1 && decision.player() == Some(1) && matches!(decision, Decision::IdleCmd { .. }) {
            return cards
                .iter()
                .filter_map(|&c| {
                    if find_act(engine, &actions, Act::Activate(c, location::HAND)).is_some() {
                        Some((c, false))
                    } else {
                        find_act(engine, &actions, Act::SpecialSummon(c)).map(|_| (c, true))
                    }
                })
                .collect();
        }
        let answer = if decision.player() == Some(1) { passive_answer(&decision, &actions) } else { end_turn(&decision, &actions) };
        duel.respond(&answer);
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::none_drawn;

    #[test]
    fn chance_of_drawing_none() {
        // Three copies in 40 cards, 6 drawn: 1 - (34·33·32)/(40·39·38).
        assert!((none_drawn(40, 3, 6) - 34.0 * 33.0 * 32.0 / (40.0 * 39.0 * 38.0)).abs() < 1e-12);
        assert_eq!(none_drawn(40, 0, 6), 1.0);
        assert_eq!(none_drawn(40, 40, 6), 0.0);
    }
}
