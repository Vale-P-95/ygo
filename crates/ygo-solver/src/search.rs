//! The search. A node is a real choice the engine asks for; it is reached by
//! replaying its line from the start of the duel, because the engine cannot
//! copy a duel in progress. To make the most of each replay, a worker dives:
//! after reaching a node it keeps playing the first answer of every choice
//! down to the end of the turn, and leaves the other answers in a shared
//! queue for later. A table of the states already seen (by their canonical
//! fingerprint, plus the choice being asked) makes sure that lines reaching
//! the same situation in a different order are explored once.
//!
//! Ending the turn is an answer at every free moment, and playing it means a
//! whole replay just to read a board. So the board is read on the spot,
//! before the End Phase ("provisional"), the End Phase is queued last, and
//! when the search stops the best provisional boards are played for real.
//!
//! A search can go on where it stopped ([`resume`]) and can aim at a
//! [`Goal`]: cards that must be on the field at the end.

use std::cmp::Ordering as CmpOrdering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use ygo_engine::auto::AutoPolicy;
use ygo_engine::consts::{card_type, location};
use ygo_engine::{canonical_fingerprint, italian, Action, ActionKind, Decision, DeckList, Duel, DuelState, Engine, Error, Step};

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub deck: DeckList,
    pub hand: Vec<u32>,
    /// Order of the Deck and the engine's random numbers.
    pub seed: u64,
    /// For this run (a resumed search gets a new one).
    pub time_limit: Duration,
    /// Nodes (real choices reached) at most, over all runs.
    pub max_nodes: u64,
    pub threads: usize,
    /// Real choices per line at most.
    pub max_choices: usize,
    /// Cards that must still be in the hand at the end (e.g. a handtrap).
    pub keep_in_hand: Vec<u32>,
    /// Handtraps: kept in hand they stop the opponent, so the estimate
    /// values them more than other cards in hand.
    pub hand_traps: Vec<u32>,
    pub goal: Goal,
    pub policy: AutoPolicy,
    /// Best boards included in each progress report.
    pub live_top: usize,
    /// Provisional boards played to the end of the turn when the search stops.
    pub confirm_top: usize,
    /// Best boards whose zone choices are tried the other ways at the end.
    pub zone_top: usize,
}

impl SearchConfig {
    pub fn new(deck: DeckList, hand: Vec<u32>) -> Self {
        Self {
            deck,
            hand,
            seed: 1,
            time_limit: Duration::from_secs(60),
            max_nodes: u64::MAX,
            threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
            max_choices: 60,
            keep_in_hand: Vec::new(),
            hand_traps: Vec::new(),
            goal: Goal::default(),
            policy: AutoPolicy::default(),
            live_top: 10,
            confirm_top: 40,
            zone_top: 20,
        }
    }
}

/// Where a monster of the goal must be.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GoalZone {
    #[default]
    Any,
    /// An Extra Monster Zone (some monsters work differently there, like
    /// Invoked Oceanus).
    Extra,
    /// A Main Monster Zone.
    Main,
}

/// A card wanted on the field, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalCard {
    pub code: u32,
    pub zone: GoalZone,
}

/// Cards wanted on your field at the end of the turn. The search favours
/// the lines that bring them, and ranks the boards that have them first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Goal {
    /// A card listed twice wants two copies.
    pub on_field: Vec<GoalCard>,
    /// All of them; otherwise at least one.
    pub all: bool,
    /// Stop at the first board that reaches the goal.
    pub first_only: bool,
}

impl Goal {
    pub fn is_set(&self) -> bool {
        !self.on_field.is_empty()
    }

    /// Any zone for each of `codes`.
    pub fn cards(codes: &[u32], all: bool, first_only: bool) -> Goal {
        Goal { on_field: codes.iter().map(|&code| GoalCard { code, zone: GoalZone::Any }).collect(), all, first_only }
    }

    /// How many of the wanted cards are on your field, in their zone
    /// (alternate artworks count).
    pub fn hits(&self, engine: &Engine, state: &DuelState) -> usize {
        if !self.is_set() {
            return 0;
        }
        let db = engine.cards();
        let me = &state.players[0];
        // (code, zone kind): Main Monster Zones 0-4, Extra 5-6, others.
        let mut field: Vec<(u32, GoalZone)> = me
            .monsters
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                c.as_ref()
                    .filter(|c| c.is_face_up())
                    .map(|c| (db.canonical_code(c.code), if i >= 5 { GoalZone::Extra } else { GoalZone::Main }))
            })
            .chain(me.spells.iter().flatten().map(|c| (db.canonical_code(c.code), GoalZone::Any)))
            .collect();
        let mut hits = 0;
        for wanted in &self.on_field {
            let code = db.canonical_code(wanted.code);
            if let Some(i) = field.iter().position(|&(c, z)| c == code && (wanted.zone == GoalZone::Any || wanted.zone == z)) {
                field.swap_remove(i);
                hits += 1;
            }
        }
        hits
    }

    /// The goal wants this card in an Extra Monster Zone.
    fn wants_extra_zone(&self, engine: &Engine, code: u32) -> bool {
        let db = engine.cards();
        self.on_field.iter().any(|g| g.zone == GoalZone::Extra && db.canonical_code(g.code) == db.canonical_code(code))
    }

    pub fn reached(&self, hits: usize) -> bool {
        self.is_set() && if self.all { hits == self.on_field.len() } else { hits > 0 }
    }

    /// The action picks or plays one of the wanted cards (summoning it,
    /// choosing it in a list such as the Fusion Monsters offered).
    fn involves(&self, engine: &Engine, action: &Action) -> bool {
        let db = engine.cards();
        action.card.is_some_and(|c| self.on_field.iter().any(|g| db.canonical_code(g.code) == db.canonical_code(c.code)))
    }
}

/// How to play a line again: every answer from the start, the automatic
/// ones included, and the labels of the real choices among them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub answers: Vec<Vec<u8>>,
    pub choices: Vec<String>,
    /// Where each choice is in `answers`; the choices are told in Italian.
    pub choice_at: Vec<usize>,
}

impl Line {
    fn choose(&mut self, answer: Vec<u8>, label: String) {
        self.choice_at.push(self.answers.len());
        self.answers.push(answer);
        self.choices.push(label);
    }

    /// The line up to its last choice, and that choice.
    fn split_last(&self) -> Option<(Line, Answer)> {
        let at = *self.choice_at.last()?;
        let mut prefix = self.clone();
        let answer = prefix.answers.remove(at);
        prefix.answers.truncate(at);
        prefix.choice_at.pop();
        let label = prefix.choices.pop()?;
        Some((prefix, (answer, label)))
    }
}

/// A board reached at the end of the turn, with the best line found to it.
#[derive(Debug, Clone)]
pub struct Board {
    /// Identity of the board (same cards in the same zones, up to a mirror image).
    pub key: u64,
    pub line: Line,
    /// The duel when the opponent's turn begins; for a provisional board,
    /// when the turn was ended (End Phase effects not played yet).
    pub state: DuelState,
    pub exact: bool,
    /// Cards of the starting hand no longer in the hand.
    pub cards_used: usize,
    /// A quick estimate until the real score is computed (Phase 5C).
    pub quick_score: f64,
    /// Cards of the goal on the field.
    pub goal_hits: usize,
    pub found_after: Duration,
}

impl Board {
    fn better_line_than(&self, other: &Board) -> bool {
        (self.cards_used, self.line.choices.len()) < (other.cards_used, other.line.choices.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Every line was explored.
    Exhausted,
    TimeLimit,
    NodeLimit,
    Cancelled,
    /// A board reached the goal ([`Goal::first_only`]).
    GoalReached,
}

#[derive(Debug, Clone, Default)]
pub struct Progress {
    /// Over all the runs of the search.
    pub elapsed: Duration,
    /// Real choices reached.
    pub nodes: u64,
    /// Lines played again from the start to reach a waiting choice.
    pub replays: u64,
    /// States skipped because they had already been explored.
    pub duplicates: u64,
    pub queued: usize,
    pub boards: usize,
    pub best_quick_score: f64,
    /// The best boards so far (see [`SearchConfig::live_top`]).
    pub top: Vec<Board>,
}

#[derive(Clone)]
pub struct SearchResult {
    /// Best first (goal first, then the quick estimate).
    pub boards: Vec<Board>,
    pub progress: Progress,
    pub stop: Stop,
    /// Lines that stopped on a choice without answers or a rejected answer
    /// (should stay 0: see the engine's randomized sweep).
    pub interrupted: u64,
    /// Where the search stopped, to go on with [`resume`].
    pub frontier: Frontier,
}

impl SearchResult {
    /// `Some(true)` when a board reaches the goal, `Some(false)` when no
    /// line can (every line was tried, none lost to an engine error),
    /// `None` while it is unknown.
    pub fn goal_reached(&self, goal: &Goal) -> Option<bool> {
        if self.boards.iter().any(|b| goal.reached(b.goal_hits)) {
            Some(true)
        } else if self.stop == Stop::Exhausted && self.interrupted == 0 {
            Some(false)
        } else {
            None
        }
    }
}

/// An answer to try and its label.
type Answer = (Vec<u8>, String);

/// An edge to queue: prefix, answer, priority, provisional board it confirms.
type NewEdge = (Arc<Line>, Option<Answer>, f64, Option<u64>);

/// A choice left for later: the line to its node and the answer to try.
#[derive(Clone)]
struct Edge {
    prefix: Arc<Line>,
    answer: Option<Answer>,
    priority: f64,
    order: u64,
    /// The provisional board this edge plays to the end of the turn.
    confirms: Option<u64>,
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == CmpOrdering::Equal
    }
}
impl Eq for Edge {}
impl PartialOrd for Edge {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}
impl Ord for Edge {
    /// Higher priority first; among equals the newest (deepest search first).
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.priority.total_cmp(&other.priority).then(self.order.cmp(&other.order))
    }
}

/// Everything a search knows when it stops: the choices left, the states
/// seen and the boards found.
#[derive(Clone, Default)]
pub struct Frontier {
    edges: Vec<Edge>,
    seen: Vec<HashSet<u64>>,
    boards: HashMap<u64, Board>,
    next_order: u64,
    nodes: u64,
    replays: u64,
    duplicates: u64,
    interrupted: u64,
    elapsed: Duration,
    started: bool,
}

impl std::fmt::Debug for Frontier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frontier")
            .field("edges", &self.edges.len())
            .field("boards", &self.boards.len())
            .field("nodes", &self.nodes)
            .finish()
    }
}

impl Frontier {
    /// Every line was explored: going on would find nothing new.
    pub fn is_exhausted(&self) -> bool {
        self.started && self.edges.is_empty()
    }
}

/// Priority taken off the End Phase of a provisional board: it is played
/// when there is nothing better to do (or at the end, for the best ones).
const END_PHASE_LAST: f64 = 100.0;

struct Shared<'a> {
    engine: &'a Engine,
    config: &'a SearchConfig,
    start: Instant,
    /// Search time before this run.
    before: Duration,
    cancel: &'a AtomicBool,
    queue: Mutex<Queue>,
    wake: Condvar,
    seen: Vec<Mutex<HashSet<u64>>>,
    boards: Mutex<HashMap<u64, Board>>,
    nodes: AtomicU64,
    replays: AtomicU64,
    duplicates: AtomicU64,
    interrupted: AtomicU64,
    stop: Mutex<Option<Stop>>,
    /// Playing the best provisional boards to the end: only cancelling stops it.
    confirming: AtomicBool,
}

struct Queue {
    edges: BinaryHeap<Edge>,
    busy: usize,
    next_order: u64,
}

const SEEN_SHARDS: usize = 64;

impl Shared<'_> {
    fn stopped(&self) -> bool {
        self.stop.lock().map(|s| s.is_some()).unwrap_or(true)
    }

    /// Whether a worker should leave its line now.
    fn halted(&self) -> bool {
        if self.confirming.load(Ordering::Relaxed) {
            self.cancel.load(Ordering::Relaxed)
        } else {
            self.stopped()
        }
    }

    fn set_stop(&self, why: Stop) {
        if let Ok(mut s) = self.stop.lock() {
            s.get_or_insert(why);
        }
        self.wake.notify_all();
    }

    fn elapsed(&self) -> Duration {
        self.before + self.start.elapsed()
    }

    fn check_limits(&self) {
        if self.cancel.load(Ordering::Relaxed) {
            self.set_stop(Stop::Cancelled);
        } else if self.start.elapsed() >= self.config.time_limit {
            self.set_stop(Stop::TimeLimit);
        } else if self.nodes.load(Ordering::Relaxed) >= self.config.max_nodes {
            self.set_stop(Stop::NodeLimit);
        }
    }

    /// True the first time `key` is seen.
    fn first_visit(&self, key: u64) -> bool {
        let shard = &self.seen[(key as usize) % SEEN_SHARDS];
        shard.lock().map(|mut s| s.insert(key)).unwrap_or(false)
    }

    fn push(&self, edges: Vec<NewEdge>) {
        if edges.is_empty() {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            for (prefix, answer, priority, confirms) in edges {
                let order = q.next_order;
                q.next_order += 1;
                q.edges.push(Edge { prefix, answer, priority, order, confirms });
            }
        }
        self.wake.notify_all();
    }

    /// The next edge, or `None` when the search is over.
    fn pop(&self) -> Option<Edge> {
        let mut q = self.queue.lock().ok()?;
        loop {
            if self.stopped() {
                return None;
            }
            if let Some(edge) = q.edges.pop() {
                q.busy += 1;
                return Some(edge);
            }
            if q.busy == 0 {
                drop(q);
                self.set_stop(Stop::Exhausted);
                return None;
            }
            q = self.wake.wait_timeout(q, Duration::from_millis(50)).ok()?.0;
            drop(q);
            self.check_limits();
            q = self.queue.lock().ok()?;
        }
    }

    fn done_with_edge(&self) {
        if let Ok(mut q) = self.queue.lock() {
            q.busy -= 1;
        }
        self.wake.notify_all();
    }

    /// Keeps the board with the better line; a board played to the end of
    /// the turn replaces its provisional reading.
    fn record(&self, board: Board, confirms: Option<u64>) {
        if self.config.goal.first_only && board.exact && self.config.goal.reached(board.goal_hits) {
            self.set_stop(Stop::GoalReached);
        }
        let Ok(mut boards) = self.boards.lock() else { return };
        if let Some(old) = confirms {
            if old != board.key && boards.get(&old).is_some_and(|b| !b.exact) {
                boards.remove(&old);
            }
        }
        let replace = match boards.get(&board.key) {
            None => true,
            Some(old) if old.exact == board.exact => board.better_line_than(old),
            // The real reading wins unless the provisional one has a better line.
            Some(old) if board.exact => !old.better_line_than(&board),
            Some(old) => board.better_line_than(old),
        };
        if replace {
            boards.insert(board.key, board);
        }
    }

    fn progress(&self) -> Progress {
        let (boards, best, top) = self
            .boards
            .lock()
            .map(|b| {
                let mut top: Vec<&Board> = b.values().collect();
                top.sort_by(|x, y| rank(x, y));
                let top = top.into_iter().take(self.config.live_top).cloned().collect();
                (b.len(), b.values().map(|b| b.quick_score).fold(0.0, f64::max), top)
            })
            .unwrap_or_default();
        Progress {
            elapsed: self.elapsed(),
            nodes: self.nodes.load(Ordering::Relaxed),
            replays: self.replays.load(Ordering::Relaxed),
            duplicates: self.duplicates.load(Ordering::Relaxed),
            queued: self.queue.lock().map(|q| q.edges.len()).unwrap_or(0),
            boards,
            best_quick_score: best,
            top,
        }
    }

    fn board(&self, state: DuelState, line: Line, exact: bool) -> Board {
        Board {
            key: board_key(&state),
            cards_used: cards_used(&self.config.hand, &state),
            quick_score: quick_score(self.engine, &state, &self.config.hand_traps),
            goal_hits: self.config.goal.hits(self.engine, &state),
            exact,
            line,
            state,
            found_after: self.elapsed(),
        }
    }
}

/// Best first: the goal, then a higher estimate, then fewer cards from the
/// hand, then a shorter line.
fn rank(a: &Board, b: &Board) -> CmpOrdering {
    b.goal_hits
        .cmp(&a.goal_hits)
        .then(b.quick_score.total_cmp(&a.quick_score))
        .then(a.cards_used.cmp(&b.cards_used))
        .then(a.line.choices.len().cmp(&b.line.choices.len()))
}

/// Key of a node: the state and the choice being asked (two steps of the
/// same selection share the state but not the choice).
fn node_key(state: &DuelState, decision: &Decision) -> u64 {
    let mut h = DefaultHasher::new();
    canonical_fingerprint(state).hash(&mut h);
    decision.hash(&mut h);
    h.finish()
}

/// Identity of an end board: the same cards on the field, with the same
/// positions, materials and counters, and the same hand, Graveyard,
/// banished cards and Extra Deck.
fn board_key(state: &DuelState) -> u64 {
    let mut h = DefaultHasher::new();
    look_key(state).hash(&mut h);
    for p in &state.players {
        let mut grave = p.grave.clone();
        grave.sort_unstable();
        grave.hash(&mut h);
        for pile in [&p.banished, &p.extra] {
            let mut v = pile.clone();
            v.sort_unstable();
            v.hash(&mut h);
        }
        p.lp.hash(&mut h);
    }
    h.finish()
}

/// What a board shows: the cards on the field and in hand. Which of the
/// five Main Monster Zones (or Spell & Trap Zones) holds a card is left
/// out, since on the opponent's turn it almost never matters; the Extra
/// Monster Zone is not: Invoked Oceanus banishes what goes to the
/// opponent's GY only from there.
pub fn look_key(state: &DuelState) -> u64 {
    let mut h = DefaultHasher::new();
    for p in &state.players {
        let mut cards: Vec<u64> = Vec::new();
        for (zone, c) in p.monsters.iter().enumerate() {
            if let Some(c) = c {
                cards.push(card_hash(state, (0, zone >= 5), c));
            }
        }
        for (zone, c) in p.spells.iter().enumerate() {
            if let Some(c) = c {
                // The Field Zone and the old Pendulum Zones are kinds of their own.
                cards.push(card_hash(state, (zone.saturating_sub(4), false), c));
            }
        }
        cards.sort_unstable();
        cards.hash(&mut h);
        let mut hand = p.hand.clone();
        hand.sort_unstable();
        hand.hash(&mut h);
    }
    h.finish()
}

/// A card on the field and the kind of zone it is in. An equipped card
/// names the card it is equipped to, not that card's zone.
fn card_hash(state: &DuelState, kind: (usize, bool), c: &ygo_engine::FieldCard) -> u64 {
    let mut h = DefaultHasher::new();
    let mut overlay = c.overlay.clone();
    overlay.sort_unstable();
    let equipped_to = c.equipped_to.and_then(|(ctrl, loc, seq)| {
        let p = state.players.get(ctrl as usize)?;
        let zones = if loc == location::MZONE { &p.monsters } else { &p.spells };
        zones.get(seq as usize)?.as_ref().map(|t| (ctrl, t.code))
    });
    (kind, c.code, c.is_face_up(), c.position & 0xc != 0, c.link, overlay, &c.counters, equipped_to).hash(&mut h);
    h.finish()
}

/// A rough value of a state for ordering the search (Phase 5C replaces it
/// at the end with the real score): monsters on the field, more for Extra
/// Deck monsters, set cards, cards kept in hand, more for `hand_traps`.
pub fn quick_score(engine: &Engine, state: &DuelState, hand_traps: &[u32]) -> f64 {
    let me = &state.players[0];
    let mut score = 0.0;
    for c in me.monsters.iter().flatten() {
        let extra = engine.cards().get(c.code).is_some_and(|d| d.type_ & card_type::EXTRA != 0);
        // A face-down monster applies no effect (a set Extra Deck monster almost never flips).
        score += match (extra, c.is_face_up()) {
            (true, true) => 2.0,
            (true, false) => 0.3,
            (false, _) => 1.0,
        };
    }
    for c in me.spells.iter().flatten() {
        let type_ = engine.cards().get(c.code).map_or(0, |d| d.type_);
        // A set Trap or Quick-Play Spell can be used on the opponent's turn;
        // setting any other Spell only takes it out of the hand.
        score += if c.is_face_up() {
            0.5
        } else if type_ & card_type::TRAP != 0 {
            1.0
        } else if type_ & card_type::QUICKPLAY != 0 {
            0.6
        } else {
            0.1
        };
    }
    let db = engine.cards();
    let hand_trap = |c: u32| hand_traps.iter().any(|&h| db.canonical_code(h) == db.canonical_code(c));
    score + me.hand.iter().map(|&c| if hand_trap(c) { 1.5 } else { 0.3 }).sum::<f64>()
}

/// Answers that do something come first: the dive tries them before ending
/// the turn, and Sets (which give up a Normal Summon or a card's use this
/// turn) after the rest.
///
/// Responding to your own chain with quick effects is rarely what a combo
/// needs (it spends a card or an effect for little, like discarding one for
/// ATK or turning your own monster face-down), so there not chaining comes
/// first. Triggers of the same moment are put on the chain first.
///
/// Answers that pick or play a card of the goal come before everything;
/// `hold` answers come after the others that do something.
fn dive_order(actions: &mut [Action], responding: bool, wanted: impl Fn(&Action) -> bool, hold: impl Fn(&Action) -> bool) {
    let rank = |a: &Action| match a.kind {
        _ if wanted(a) => -1,
        _ if hold(a) => 1,
        ActionKind::Activate if responding => 2,
        ActionKind::Pass if responding => 1,
        ActionKind::EndTurn => 4,
        ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish => 3,
        ActionKind::SetMonster | ActionKind::SetSpellTrap => 2,
        _ => 0,
    };
    actions.sort_by_key(rank);
}

fn cards_used(hand: &[u32], state: &DuelState) -> usize {
    let mut left = state.players[0].hand.clone();
    let mut kept = 0;
    for code in hand {
        if let Some(i) = left.iter().position(|c| c == code) {
            left.swap_remove(i);
            kept += 1;
        }
    }
    hand.len() - kept
}

/// The cards to keep are still in the hand.
fn keeps_hand(config: &SearchConfig, state: &DuelState) -> bool {
    let mut hand = state.players[0].hand.clone();
    config.keep_in_hand.iter().all(|code| match hand.iter().position(|c| c == code) {
        Some(i) => {
            hand.swap_remove(i);
            true
        }
        None => false,
    })
}

/// Plays one edge: replays its line, gives its answer, then dives.
fn work(shared: &Shared, edge: Edge) -> Result<(), Error> {
    let config = shared.config;
    let mut duel = Duel::goldfish(shared.engine, &config.deck, &config.hand, config.seed)?;
    let mut line = (*edge.prefix).clone();
    for answer in &line.answers {
        duel.advance()?;
        duel.respond(answer);
    }
    if !line.answers.is_empty() || edge.answer.is_some() {
        shared.replays.fetch_add(1, Ordering::Relaxed);
    }
    if let Some((answer, label)) = edge.answer {
        duel.advance()?;
        duel.respond(&answer);
        line.choose(answer, label);
    }
    // The card last taken from the Extra Deck: the one a Monster Zone is
    // asked for next (only those go to the Extra Monster Zone).
    let mut placing: Option<u32> = None;
    loop {
        if shared.halted() {
            return Ok(());
        }
        let (step, mut actions) = duel.advance_auto(&config.policy, &mut line.answers)?;
        let (turn, _, _) = duel.turn_info();
        let decision = match step {
            Step::Decision { decision, .. } if decision.player() == Some(0) && turn <= 1 => decision,
            // The opponent's turn (or the end of the duel): the board is done.
            _ => {
                let state = duel.state();
                if keeps_hand(config, &state) {
                    shared.record(shared.board(state, line, true), edge.confirms);
                }
                return Ok(());
            }
        };
        let state = duel.state();
        shared.nodes.fetch_add(1, Ordering::Relaxed);
        if !shared.first_visit(node_key(&state, &decision)) {
            shared.duplicates.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        if line.choices.len() >= config.max_choices {
            // Too long: end the turn, or close what is pending as fast as possible.
            dive_order(&mut actions, false, |_| false, |_| false);
            let wrap_up = |a: &Action| {
                matches!(a.kind, ActionKind::EndTurn | ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish)
            };
            let keep = actions.iter().position(wrap_up).unwrap_or(0);
            actions = vec![actions.swap_remove(keep)];
        }
        if actions.is_empty() {
            shared.interrupted.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        let wanted = |a: &Action| config.goal.involves(shared.engine, a);
        // While a goal card wants the free Extra Monster Zone, other Extra
        // Deck monsters (which would take it) are summoned last.
        let keep_extra_zone =
            config.goal.on_field.iter().any(|g| g.zone == GoalZone::Extra) && state.players[0].monsters[5..].iter().all(Option::is_none);
        let hold = |a: &Action| {
            keep_extra_zone && a.kind == ActionKind::SpecialSummon && a.card.is_some_and(|c| c.location == location::EXTRA) && !wanted(a)
        };
        let responding = matches!(decision, Decision::SelectChain { triggers: false, .. }) && duel.open_chain() > 0;
        dive_order(&mut actions, responding, wanted, hold);
        // A goal card wanted in an Extra Monster Zone goes there; other
        // monsters leave it free when they can.
        if let (Decision::SelectPlace { .. }, Some(code)) = (&decision, placing) {
            if config.goal.on_field.iter().any(|g| g.zone == GoalZone::Extra) {
                let extra = |a: &Action| matches!(*a.response.as_slice(), [0, location::MZONE, seq] if seq >= 5);
                let first = config.goal.wants_extra_zone(shared.engine, code);
                actions.sort_by_key(|a| extra(a) != first);
            }
        }
        let goal = 3.0 * config.goal.hits(shared.engine, &state) as f64;
        let priority = quick_score(shared.engine, &state, &shared.config.hand_traps) + 0.3 * line.choices.len() as f64 + goal;
        let prefix = Arc::new(line.clone());
        let mut rest = Vec::with_capacity(actions.len());
        // Ending the turn here: read the board now, play the End Phase later.
        if actions.len() > 1 {
            if let Some(i) = actions.iter().position(|a| a.kind == ActionKind::EndTurn) {
                let end = actions.remove(i);
                let label = italian::choice(&end);
                if keeps_hand(config, &state) {
                    let mut ended = line.clone();
                    ended.choose(end.response.clone(), label.clone());
                    let board = shared.board(state.clone(), ended, false);
                    let key = board.key;
                    // A board that reaches the goal is played to the end at once.
                    let end_priority =
                        if config.goal.reached(board.goal_hits) { priority + END_PHASE_LAST } else { priority - END_PHASE_LAST };
                    shared.record(board, None);
                    rest.push((prefix.clone(), Some((end.response, label)), end_priority, Some(key)));
                }
            }
        }
        // Lines that pick a card of the goal are tried before the others.
        rest.extend(actions[1..].iter().map(|a| {
            let bonus = if wanted(a) { 10.0 } else { 0.0 };
            (prefix.clone(), Some((a.response.clone(), italian::choice(a))), priority + bonus, None)
        }));
        shared.push(rest);
        let first = &actions[0];
        if let Some(c) = first.card.filter(|c| c.location == location::EXTRA) {
            placing = Some(c.code);
        }
        duel.respond(&first.response);
        line.choose(first.response.clone(), italian::choice(first));
    }
}

/// Runs the workers until the queue is empty or a limit is hit.
fn run_workers(shared: &Shared, threads: usize, mut progress: impl FnMut(&Progress)) {
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(move || {
                while let Some(edge) = shared.pop() {
                    if let Err(e) = work(shared, edge) {
                        if std::env::var_os("DUELFORGE_DEBUG_SEARCH").is_some() {
                            eprintln!("interrupted: {e}");
                        }
                        shared.interrupted.fetch_add(1, Ordering::Relaxed);
                    }
                    shared.done_with_edge();
                    shared.check_limits();
                }
            });
        }
        while !shared.stopped() {
            std::thread::sleep(Duration::from_millis(250));
            shared.check_limits();
            progress(&shared.progress());
        }
    });
}

/// Plays to the end of the turn the best provisional boards (and those
/// that reach the goal), so their End Phase effects count.
fn confirm_best(shared: &Shared) {
    let mut provisional: Vec<Board> = shared.boards.lock().map(|b| b.values().filter(|b| !b.exact).cloned().collect()).unwrap_or_default();
    provisional.sort_by(rank);
    let goal = &shared.config.goal;
    let picked: Vec<Edge> = provisional
        .iter()
        .enumerate()
        .filter(|(i, b)| *i < shared.config.confirm_top || goal.reached(b.goal_hits))
        .filter_map(|(_, b)| {
            let (prefix, answer) = b.line.split_last()?;
            Some(Edge { prefix: Arc::new(prefix), answer: Some(answer), priority: 0.0, order: 0, confirms: Some(b.key) })
        })
        .collect();
    if picked.is_empty() {
        return;
    }
    shared.confirming.store(true, Ordering::Relaxed);
    let next = AtomicU64::new(0);
    std::thread::scope(|s| {
        for _ in 0..shared.config.threads.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed) as usize;
                let Some(edge) = picked.get(i) else { break };
                if shared.cancel.load(Ordering::Relaxed) {
                    break;
                }
                // The End Phase may offer choices: their alternatives join the queue.
                if work(shared, edge.clone()).is_err() {
                    shared.interrupted.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    shared.confirming.store(false, Ordering::Relaxed);
}

/// Boards that differ only by the zone a monster was put in can do
/// different things: Invoked Oceanus banishes what goes to the opponent's
/// GY only from the Extra Monster Zone. The search may not get back to such
/// a choice in time, so for the best boards each zone choice of the line is
/// played the other ways (Extra Monster Zone or not, pointed by a Link
/// arrow or not), keeping the rest of the line.
fn zone_variants(shared: &Shared) {
    let mut best: Vec<Board> = shared.boards.lock().map(|b| b.values().filter(|b| b.exact).cloned().collect()).unwrap_or_default();
    best.sort_by(rank);
    best.truncate(shared.config.zone_top);
    let next = AtomicU64::new(0);
    std::thread::scope(|s| {
        for _ in 0..shared.config.threads.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed) as usize;
                let Some(board) = best.get(i) else { break };
                if shared.cancel.load(Ordering::Relaxed) {
                    break;
                }
                let _ = zone_variants_of(shared, &board.line);
            });
        }
    });
}

fn zone_variants_of(shared: &Shared, line: &Line) -> Result<(), Error> {
    let config = shared.config;
    // The zone choices of the line that had other kinds of zones.
    let mut spots = Vec::new();
    let mut duel = Duel::goldfish(shared.engine, &config.deck, &config.hand, config.seed)?;
    for (i, answer) in line.answers.iter().enumerate() {
        let Step::Decision { decision, .. } = duel.advance()? else { break };
        if let Decision::SelectPlace { player: 0, count: 1, .. } = decision {
            if let Some(k) = line.choice_at.iter().position(|&at| at == i) {
                let kinds = ygo_engine::auto::distinct_zones(&duel.state(), 0, duel.actions(&decision));
                let others: Vec<Action> = kinds.into_iter().filter(|a| a.response != *answer).collect();
                if !others.is_empty() {
                    spots.push((i, k, others));
                }
            }
        }
        duel.respond(answer);
    }
    if std::env::var_os("DUELFORGE_DEBUG_SEARCH").is_some() {
        eprintln!("zone variants: {} spots in a line of {} choices", spots.len(), line.choices.len());
    }
    for (i, k, others) in spots {
        for other in others {
            let mut duel = Duel::goldfish(shared.engine, &config.deck, &config.hand, config.seed)?;
            let mut played = true;
            for (j, answer) in line.answers.iter().enumerate() {
                if !matches!(duel.advance(), Ok(Step::Decision { .. })) {
                    played = false;
                    break;
                }
                duel.respond(if j == i { &other.response } else { answer });
            }
            // The rest of the line must still play and end the turn.
            let ended = played && matches!(duel.advance(), Ok(Step::End { .. }));
            if std::env::var_os("DUELFORGE_DEBUG_SEARCH").is_some() {
                eprintln!("  {} -> {}", other.label, if ended { "ok" } else { "the rest of the line does not play" });
            }
            if !ended {
                continue;
            }
            let state = duel.state();
            if keeps_hand(config, &state) {
                let mut variant = line.clone();
                variant.answers[i] = other.response.clone();
                variant.choices[k] = italian::choice(&other);
                shared.record(shared.board(state, variant, true), None);
            }
        }
    }
    Ok(())
}

/// Runs the search on `config.threads` threads. `progress` is called about
/// four times a second from the calling thread.
pub fn search(engine: &Engine, config: &SearchConfig, cancel: &AtomicBool, progress: impl FnMut(&Progress)) -> SearchResult {
    resume(engine, config, Frontier::default(), cancel, progress)
}

/// Goes on with a search from where it stopped, for `config.time_limit`
/// more (the deck, hand, keep and goal must be the same).
pub fn resume(
    engine: &Engine,
    config: &SearchConfig,
    frontier: Frontier,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&Progress),
) -> SearchResult {
    let Frontier { edges, mut seen, boards, next_order, nodes, replays, duplicates, interrupted, elapsed, started } = frontier;
    seen.resize_with(SEEN_SHARDS, HashSet::new);
    let shared = Shared {
        engine,
        config,
        start: Instant::now(),
        before: elapsed,
        cancel,
        queue: Mutex::new(Queue { edges: edges.into_iter().collect(), busy: 0, next_order }),
        wake: Condvar::new(),
        seen: seen.into_iter().map(Mutex::new).collect(),
        boards: Mutex::new(boards),
        nodes: AtomicU64::new(nodes),
        replays: AtomicU64::new(replays),
        duplicates: AtomicU64::new(duplicates),
        interrupted: AtomicU64::new(interrupted),
        stop: Mutex::new(None),
        confirming: AtomicBool::new(false),
    };
    if !started {
        shared.push(vec![(Arc::new(Line::default()), None, 0.0, None)]);
    }
    shared.check_limits();
    run_workers(&shared, config.threads, &mut progress);
    let mut stop = shared.stop.lock().ok().and_then(|s| *s).unwrap_or(Stop::Exhausted);
    if stop != Stop::Cancelled {
        confirm_best(&shared);
        zone_variants(&shared);
    }
    let final_progress = shared.progress();
    progress(&final_progress);
    let boards: HashMap<u64, Board> = shared.boards.into_inner().unwrap_or_else(|e| e.into_inner());
    let queue = shared.queue.into_inner().unwrap_or_else(|e| e.into_inner());
    // Playing the End Phases above may have queued new choices.
    if stop == Stop::Exhausted && !queue.edges.is_empty() {
        stop = Stop::TimeLimit;
    }
    let mut sorted: Vec<Board> = boards.values().cloned().collect();
    sorted.sort_by(rank);
    let interrupted = shared.interrupted.load(Ordering::Relaxed);
    let frontier = Frontier {
        edges: queue.edges.into_vec(),
        seen: shared.seen.into_iter().map(|s| s.into_inner().unwrap_or_else(|e| e.into_inner())).collect(),
        boards,
        next_order: queue.next_order,
        nodes: final_progress.nodes,
        replays: final_progress.replays,
        duplicates: final_progress.duplicates,
        interrupted,
        elapsed: final_progress.elapsed,
        started: true,
    };
    SearchResult { boards: sorted, progress: final_progress, stop, interrupted, frontier }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_last_takes_off_the_last_choice() {
        let mut line = Line::default();
        line.answers.push(vec![1]);
        line.choose(vec![2], "a".into());
        line.answers.push(vec![3]);
        line.choose(vec![4], "b".into());
        let (prefix, (answer, label)) = line.split_last().unwrap();
        assert_eq!(prefix.answers, vec![vec![1], vec![2], vec![3]]);
        assert_eq!(prefix.choices, vec!["a".to_owned()]);
        assert_eq!((answer, label.as_str()), (vec![4], "b"));
    }
}
