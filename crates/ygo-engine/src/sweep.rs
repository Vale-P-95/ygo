//! The randomized sweep: duels between random decks built from the card
//! pool, where both players give random legal answers. It reports every
//! decision that had no answer and every answer the engine rejected, so the
//! search never meets a choice it cannot make. Script errors logged by the
//! engine are reported apart: they are bugs in card scripts, not here.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use crate::consts::{card_type, duel_flags, location, position};
use crate::duel::{shuffle, Duel, DuelConfig, Engine, Step};
use crate::message::{Decision, Event};
use crate::{ActionKind, DeckList, Error};

#[derive(Debug, Clone)]
pub struct SweepConfig {
    pub duels: u32,
    /// Decisions per duel, both players together.
    pub max_decisions: u32,
    /// Turns per duel.
    pub max_turns: u32,
    pub seed: u64,
    pub threads: usize,
}

impl Default for SweepConfig {
    fn default() -> Self {
        Self { duels: 50, max_decisions: 300, max_turns: 4, seed: 1, threads: 4 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProblemKind {
    /// A decision with no legal answer: the line would stop there.
    NoAnswer,
    /// The engine rejected an answer built by this crate.
    Rejected,
    /// The engine failed outside a decision.
    Engine,
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub kind: ProblemKind,
    pub duel: u32,
    pub step: u32,
    /// The decision, as debug text.
    pub decision: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct SweepReport {
    pub duels: u32,
    pub decisions: u64,
    /// Answers given, by kind of decision.
    pub by_decision: BTreeMap<&'static str, u64>,
    pub problems: Vec<Problem>,
    /// Distinct script errors logged by the engine, with how often.
    pub script_errors: BTreeMap<String, u32>,
    pub elapsed: Duration,
}

impl SweepReport {
    /// Problems of this crate (script errors excluded).
    pub fn failures(&self) -> impl Iterator<Item = &Problem> {
        self.problems.iter()
    }

    fn merge(&mut self, other: SweepReport) {
        self.duels += other.duels;
        self.decisions += other.decisions;
        for (k, v) in other.by_decision {
            *self.by_decision.entry(k).or_default() += v;
        }
        self.problems.extend(other.problems);
        for (k, v) in other.script_errors {
            *self.script_errors.entry(k).or_default() += v;
        }
    }
}

pub fn decision_kind(decision: &Decision) -> &'static str {
    match decision {
        Decision::IdleCmd { .. } => "idle",
        Decision::BattleCmd { .. } => "battle",
        Decision::EffectYesNo { .. } => "effect yes/no",
        Decision::YesNo { .. } => "yes/no",
        Decision::Option { .. } => "option",
        Decision::SelectCard { .. } => "select card",
        Decision::SelectUnselectCard { .. } => "select/unselect card",
        Decision::SelectChain { .. } => "chain",
        Decision::SelectPlace { .. } => "place",
        Decision::SelectPosition { .. } => "position",
        Decision::SelectTribute { .. } => "tribute",
        Decision::SelectSum { .. } => "sum",
        Decision::Sort { .. } => "sort",
        Decision::AnnounceRace { .. } => "announce type",
        Decision::AnnounceAttribute { .. } => "announce attribute",
        Decision::AnnounceNumber { .. } => "announce number",
        Decision::AnnounceCard { .. } => "announce card",
        Decision::SelectCounter { .. } => "counter",
        Decision::RockPaperScissors { .. } => "rock-paper-scissors",
        Decision::Unsupported { .. } => "unsupported",
    }
}

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn chance(&mut self, p: f64) -> bool {
        ((self.next() >> 11) as f64 / (1u64 << 53) as f64) < p
    }
}

/// Cards with a script, grouped by archetype.
pub struct CardPool {
    main: Vec<u32>,
    extra: Vec<u32>,
    archetypes: Vec<(Vec<u32>, Vec<u32>)>,
    /// The set code of each archetype, in the same order.
    setcodes: Vec<u16>,
}

impl CardPool {
    pub fn new(engine: &Engine) -> Self {
        let mut cards: Vec<_> = engine
            .cards()
            .iter()
            .filter(|c| c.alias == 0 && c.type_ & card_type::TOKEN == 0)
            .filter(|c| c.type_ & (card_type::MONSTER | card_type::SPELL | card_type::TRAP) != 0)
            .filter(|c| engine.scripts().has_card_script(c.code))
            .collect();
        cards.sort_by_key(|c| c.code);
        let (mut main, mut extra) = (Vec::new(), Vec::new());
        let mut sets: BTreeMap<u16, (Vec<u32>, Vec<u32>)> = BTreeMap::new();
        for c in cards {
            let list = if c.is_extra_deck_monster() { &mut extra } else { &mut main };
            list.push(c.code);
            for &sc in &c.setcodes {
                let entry = sets.entry(sc & 0xfff).or_default();
                if c.is_extra_deck_monster() {
                    entry.1.push(c.code);
                } else {
                    entry.0.push(c.code);
                }
            }
        }
        let (setcodes, archetypes) = sets.into_iter().filter(|(_, (m, _))| m.len() >= 8).unzip();
        Self { main, extra, archetypes, setcodes }
    }

    pub fn archetypes(&self) -> usize {
        self.archetypes.len()
    }

    /// `count` decks of different archetypes (with at least `min_cards` Main
    /// Deck cards each), for the robustness sweep of the whole pipeline: the
    /// archetype's Main Deck cards ×3 up to 40 (random cards fill a smaller
    /// one), its Extra Deck monsters then random ones up to 15. The same
    /// `seed` gives the same decks.
    pub fn archetype_decks(&self, count: usize, min_cards: usize, seed: u64) -> Vec<(u16, DeckList)> {
        let mut rng = Rng::new(seed ^ 0x51ee_d0ec);
        let mut order: Vec<usize> = (0..self.archetypes.len()).filter(|&i| self.archetypes[i].0.len() >= min_cards).collect();
        shuffle(&mut order, rng.next());
        order
            .into_iter()
            .take(count)
            .map(|i| {
                let (m, e) = &self.archetypes[i];
                let mut deck = DeckList::default();
                let mut m = m.clone();
                shuffle(&mut m, rng.next());
                for &code in &m {
                    if deck.main.len() + 3 > 40 {
                        break;
                    }
                    deck.main.extend([code; 3]);
                }
                while deck.main.len() < 40 && !self.main.is_empty() {
                    deck.main.push(self.main[rng.below(self.main.len())]);
                }
                let mut e = e.clone();
                shuffle(&mut e, rng.next());
                deck.extra.extend(e.iter().take(15));
                while deck.extra.len() < 15 && !self.extra.is_empty() {
                    deck.extra.push(self.extra[rng.below(self.extra.len())]);
                }
                (self.setcodes[i], deck)
            })
            .collect()
    }

    /// An archetype deck (13 cards ×3 and fillers) three times out of four,
    /// otherwise 40 random cards; 15 Extra Deck monsters.
    fn deck(&self, rng: &mut Rng) -> DeckList {
        let mut deck = DeckList::default();
        if !self.archetypes.is_empty() && rng.chance(0.75) {
            let (m, e) = &self.archetypes[rng.below(self.archetypes.len())];
            let mut m = m.clone();
            shuffle(&mut m, rng.next());
            for &code in m.iter().take(13) {
                deck.main.extend([code; 3]);
            }
            let mut e = e.clone();
            shuffle(&mut e, rng.next());
            deck.extra.extend(e.iter().take(15));
        }
        while deck.main.len() < 40 && !self.main.is_empty() {
            deck.main.push(self.main[rng.below(self.main.len())]);
        }
        while deck.extra.len() < 15 && !self.extra.is_empty() {
            deck.extra.push(self.extra[rng.below(self.extra.len())]);
        }
        deck
    }
}

/// A duel where both players' decisions come back to the caller.
pub fn open_duel(engine: &Engine, decks: [&DeckList; 2], seed: u64) -> Result<Duel, Error> {
    let config = DuelConfig {
        seed: [seed ^ 0x9e37_79b9_7f4a_7c15, seed.rotate_left(29) | 1, 0x5eed, 0xd0e1],
        flags: duel_flags::MODE_MR5 | duel_flags::PSEUDO_SHUFFLE,
        starting_lp: 8000,
    };
    let mut duel = Duel::new(engine, &config)?;
    for (player, deck) in decks.iter().enumerate() {
        let player = player as u8;
        let mut main = deck.main.clone();
        shuffle(&mut main, seed.wrapping_add(player as u64 + 1));
        let (hand, rest) = main.split_at(5.min(main.len()));
        for (i, &code) in rest.iter().enumerate() {
            duel.add_card(player, code, location::DECK, i as u32, position::FACEDOWN_DEFENSE);
        }
        for (i, &code) in deck.extra.iter().enumerate() {
            duel.add_card(player, code, location::EXTRA, i as u32, position::FACEDOWN_DEFENSE);
        }
        for (i, &code) in hand.iter().enumerate() {
            duel.add_card(player, code, location::HAND, i as u32, position::FACEDOWN_DEFENSE);
        }
    }
    duel.start();
    Ok(duel)
}

fn play_duel(engine: &Engine, pool: &CardPool, config: &SweepConfig, index: u32, report: &mut SweepReport) {
    let mut rng = Rng::new(config.seed.wrapping_mul(0x2545_f491_4f6c_dd1d) ^ index as u64);
    let decks = [pool.deck(&mut rng), pool.deck(&mut rng)];
    let problem = |kind, step, decision: String, detail: String| Problem { kind, duel: index, step, decision, detail };
    let mut duel = match open_duel(engine, [&decks[0], &decks[1]], rng.next()) {
        Ok(d) => d,
        Err(e) => {
            report.problems.push(problem(ProblemKind::Engine, 0, String::new(), e.to_string()));
            return;
        }
    };
    report.duels += 1;
    let (mut turns, mut in_turn) = (0u32, 0u32);
    let mut step = 0;
    let mut rejected: Vec<Vec<u8>> = Vec::new();
    while step < config.max_decisions {
        let decision = match duel.advance() {
            Ok(Step::Decision { decision, events }) => {
                for e in &events {
                    if matches!(e, Event::NewTurn { .. }) {
                        turns += 1;
                        in_turn = 0;
                    }
                }
                rejected.clear();
                decision
            }
            Ok(Step::End { .. }) => break,
            Err(Error::Rejected(log)) => {
                let Some(decision) = duel.pending().cloned() else { break };
                report.problems.push(problem(
                    ProblemKind::Rejected,
                    step,
                    format!("{decision:?}"),
                    format!("answer {:?}; {log}", rejected.last()),
                ));
                decision
            }
            Err(e) => {
                report.problems.push(problem(ProblemKind::Engine, step, String::new(), e.to_string()));
                break;
            }
        };
        if turns > config.max_turns {
            break;
        }
        let list: Vec<_> = duel.actions(&decision).into_iter().filter(|a| !rejected.contains(&a.response)).collect();
        if list.is_empty() {
            if rejected.is_empty() {
                report.problems.push(problem(ProblemKind::NoAnswer, step, format!("{decision:?}"), String::new()));
            }
            break;
        }
        // Mostly keep playing; end the turn now and then, and always after a while.
        let action = if matches!(decision, Decision::IdleCmd { .. }) {
            let end = list.iter().find(|a| a.kind == ActionKind::EndTurn);
            let battle = list.iter().find(|a| a.kind == ActionKind::ToBattlePhase);
            let rest: Vec<_> = list.iter().filter(|a| a.kind != ActionKind::EndTurn && a.kind != ActionKind::ToBattlePhase).collect();
            match (end, battle) {
                (_, Some(battle)) if rng.chance(0.08) => battle,
                (Some(end), _) if rest.is_empty() || in_turn > 40 || rng.chance(0.06) => end,
                _ if !rest.is_empty() => rest[rng.below(rest.len())],
                _ => &list[rng.below(list.len())],
            }
        } else {
            &list[rng.below(list.len())]
        };
        *report.by_decision.entry(decision_kind(&decision)).or_default() += 1;
        report.decisions += 1;
        rejected.push(action.response.clone());
        duel.respond(&action.response);
        step += 1;
        in_turn += 1;
    }
    for line in duel.log() {
        *report.script_errors.entry(line).or_default() += 1;
    }
}

/// Runs the sweep on `config.threads` threads. Each duel depends only on
/// the seed and its index, so the report does not depend on the threads.
pub fn sweep(engine: &Engine, config: &SweepConfig) -> SweepReport {
    let start = Instant::now();
    let pool = CardPool::new(engine);
    let threads = config.threads.max(1);
    let mut report = SweepReport::default();
    let parts: Vec<SweepReport> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let pool = &pool;
                s.spawn(move || {
                    let mut part = SweepReport::default();
                    for index in (t as u32..config.duels).step_by(threads) {
                        play_duel(engine, pool, config, index, &mut part);
                    }
                    part
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    for part in parts {
        report.merge(part);
    }
    report.problems.sort_by_key(|p| (p.kind, p.duel, p.step));
    report.elapsed = start.elapsed();
    report
}

/// Counts of the problems by kind, for a summary line.
pub fn summary(report: &SweepReport) -> HashMap<ProblemKind, usize> {
    let mut out = HashMap::new();
    for p in &report.problems {
        *out.entry(p.kind).or_default() += 1;
    }
    out
}
