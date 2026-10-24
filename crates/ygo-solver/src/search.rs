//! The search. A node is a real choice the engine asks for; it is reached by
//! replaying its line, from the longest copy of the duel the worker kept
//! along it ([`Copies`]) or from the start of the duel. To make the most of
//! each replay, a worker dives:
//! after reaching a node it keeps playing the first answer of every choice
//! down to the end of the turn, and leaves the other answers in a shared
//! queue for later. A table of the states already seen (by their canonical
//! fingerprint, plus the choice being asked) makes sure that lines reaching
//! the same situation in a different order are explored once; answers that
//! could only lead back to where the search has been (taking a card out of
//! a selection, cancelling what was just started or picked, picking the
//! same cards in another order, another target or name declared that the
//! state does not show yet) are not tried at all, since each try costs a
//! replay.
//!
//! Which answer is tried next: the one whose line led to the best board
//! after it, less a little for each answer of its line that is not the
//! dive's. So the search works around its best lines first, at every
//! depth, instead of going over the last choices of the deepest line. The
//! boards are estimated by what their cards stop, measured once per card
//! ([`crate::value`]).
//!
//! Ending the turn is an answer at every free moment, and playing it means a
//! whole replay just to read a board. So the board is read on the spot,
//! before the End Phase ("provisional"), the End Phase is queued last (and
//! played once per board), and when the search stops the best provisional
//! boards are played for real.
//!
//! A search can go on where it stopped ([`resume`]) and can aim at a
//! [`Goal`]: cards wanted at the end, on the field or off it.

use std::cmp::Ordering as CmpOrdering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use ygo_engine::auto::AutoPolicy;
use ygo_engine::consts::{card_type, location};
use ygo_engine::{
    canonical_fingerprint, fingerprint, italian, Action, ActionKind, CardDb, CardRef, ChainLink, Decision, DeckList, Duel, DuelState,
    Engine, Error, Event, Opponent, Placement, Prelude, SavedDuel, Step,
};

use crate::coverage::{self, Audit, Coverage, Links, Mechanic};
use crate::goal::{Goal, GoalPick, GoalZone};
use crate::probe::{taken_weight, Kit, Prober};
use crate::value::{CardValues, InHand, Spot};

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
    /// The opponent's cards during your turn: those of the probe turn
    /// ([`crate::Kit::opponent`]), so a line found here plays the same
    /// there. `None`: an opponent with no cards.
    pub opponent: Option<Opponent>,
    /// Best boards included in each progress report.
    pub live_top: usize,
    /// Provisional boards played to the end of the turn when the search
    /// stops, in a search of up to [`PHASE_BOARDS`] such boards; more in
    /// proportion past them ([`scaled`]).
    pub confirm_top: usize,
    /// Best boards whose zone choices are tried the other ways at the end,
    /// scaled the same way by the exact boards.
    pub zone_top: usize,
    /// What the cards are worth alone on the field ([`crate::card_values`]):
    /// the estimate counts them, so the search goes after boards that stop
    /// the opponent. Empty: only how many cards are where.
    pub values: Arc<CardValues>,
    /// The probe turn's kit, to learn during the search what cards are
    /// worth together (`None`: the values above stay as they are).
    pub learn: Option<Arc<Kit>>,
    /// Cards of yours already on the field (or in the GY) when the turn
    /// starts, to try a hand on a board (empty: the usual duel).
    pub placed: Vec<Placement>,
    /// A turn to go on from: answers played before the search starts
    /// (from the duel of `placed`), then cards that join the hand. The
    /// lines found start after it (empty: from the start of the turn).
    pub prelude: Prelude,
    /// Record what the search does with each action of your cards
    /// ([`SearchResult::coverage`]); it costs a little time.
    pub coverage: bool,
    /// Megabytes of copies of the duel each worker keeps, to start a line
    /// from the longest copy of it instead of replaying it from the start
    /// of the duel (0: replay every line).
    pub copies_mb: usize,
    /// Nodes at which the best boards are copied on the way
    /// ([`SearchResult::checkpoints`]); it does not change the search.
    /// With one thread a copy is taken at the same node in any search
    /// that gets there, so the boards a longer search copies hold those
    /// of a shorter one.
    pub checkpoints: Vec<u64>,
    /// Best boards copied at each checkpoint.
    pub checkpoint_top: usize,
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
            opponent: None,
            live_top: 10,
            confirm_top: 40,
            zone_top: 20,
            values: Arc::default(),
            learn: None,
            placed: Vec::new(),
            prelude: Prelude::default(),
            coverage: false,
            // DUELFORGE_COPIES_MB to measure with another cap.
            copies_mb: std::env::var("DUELFORGE_COPIES_MB").ok().and_then(|n| n.parse().ok()).unwrap_or(64),
            checkpoints: Vec::new(),
            checkpoint_top: 0,
        }
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
    /// when the turn was ended (End Phase effects not played yet). Shared:
    /// the boards are copied for the progress, the phases after the search
    /// and the result, and a state is most of a board's memory.
    pub state: Arc<DuelState>,
    pub exact: bool,
    /// Cards of the starting hand no longer in the hand.
    pub cards_used: usize,
    /// A quick estimate until the real score is computed (Phase 5C).
    pub quick_score: f64,
    /// Hard cards of the goal where they are wanted.
    pub goal_hits: usize,
    /// Soft cards of the goal where they are wanted.
    pub goal_soft: usize,
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
    /// What the search did with each action of your cards, when asked.
    pub coverage: Option<Coverage>,
    /// The best boards at each of [`SearchConfig::checkpoints`] the search
    /// got past, with its nodes then: before the phases after the search,
    /// best first.
    pub checkpoints: Vec<(u64, Vec<Board>)>,
}

impl SearchResult {
    /// `Some(true)` when a board reaches the goal, `Some(false)` when no
    /// line can (every line was tried, none lost to an engine error),
    /// `None` while it is unknown or there is nothing to reach (only soft
    /// cards).
    pub fn goal_reached(&self, goal: &Goal) -> Option<bool> {
        if !goal.has_target() {
            return None;
        }
        // Cards wanted off the field are only known once the End Phase is played.
        if self.boards.iter().any(|b| goal.reached(b.goal_hits) && (b.exact || !goal.off_field())) {
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

/// An edge to queue (see [`Edge`]).
struct NewEdge {
    prefix: Arc<Line>,
    answer: Option<Answer>,
    priority: f64,
    confirms: Option<u64>,
    value: f64,
    deviation: f64,
}

/// A choice left for later: the line to its node and the answer to try.
#[derive(Clone)]
struct Edge {
    prefix: Arc<Line>,
    answer: Option<Answer>,
    priority: f64,
    order: u64,
    /// The provisional board this edge plays to the end of the turn.
    confirms: Option<u64>,
    /// Estimate of the best board of the line this edge leaves.
    value: f64,
    /// Priority taken off for the answers of the line, this one included,
    /// that are not the one the dive would have given (see [`deviation`]).
    deviation: f64,
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
    /// So the other answers of a node come out from the dive's least
    /// preferred: taking them in the dive's order was measured and not kept
    /// (B1a, `docs/plans/ricerca-piano-critico.md`).
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
    /// The card values with what the search learned.
    values: Option<CardValues>,
    /// Cards (and kinds of zones) on the boards measured to learn from.
    measured: HashSet<u64>,
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
    /// Playing the best provisional boards to the end: only cancelling, or
    /// the deadline of the phases after the search, stops it.
    confirming: AtomicBool,
    /// When the phases after the search (playing the best boards to the
    /// end, their zone variants) must stop (see [`LEARN_GRACE`]).
    deadline: OnceLock<Instant>,
    /// The card values the estimate uses, raised by what the search learns.
    values: RwLock<CardValues>,
    /// Cards (and kinds of zones) on the boards measured to learn from.
    measured: Mutex<HashSet<u64>>,
    /// Nodes after which the next boards are measured.
    next_learning: AtomicU64,
    coverage: Option<Mutex<Coverage>>,
    /// Your effects that negated one of your own chain links: on your turn
    /// the opponent never answers, so chained to your own chain they only
    /// lose (Invoked Mechaba on your Aleister). Not tried again.
    own_negators: RwLock<HashSet<Mechanic>>,
    /// Your handtraps whose effect resolved with nothing happening (Maxx
    /// "C", the Mulcharmy): they wait for the opponent's moves, which never
    /// come on your turn, so activating them only loses the card. Not
    /// activated again from the hand.
    idle_hand_traps: RwLock<HashSet<u32>>,
    /// Handtraps seen to give something new to do when activated at your
    /// idle moment (Labrynth Cooclock): never left in the hand.
    useful_hand_traps: RwLock<HashSet<u32>>,
    /// The workers' duels and their copies, kept from the search for the
    /// phases after it ([`Shared::take_copies`]).
    copies: Mutex<Vec<Copies>>,
    /// The times each handtrap did nothing at your idle moment: one is not
    /// enough to leave it in the hand (Labrynth Cooclock before a Trap is Set).
    idle_strikes: Mutex<HashMap<u32, u32>>,
    /// Where the work goes (`DUELFORGE_SEARCH_STATS`): `None` otherwise.
    stats: Option<stats::Stats>,
    /// The best boards at the checkpoints passed ([`SearchConfig::checkpoints`]).
    checkpoints: Mutex<Vec<(u64, Vec<Board>)>>,
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
            self.cancel.load(Ordering::Relaxed) || self.past_deadline()
        } else {
            self.stopped()
        }
    }

    /// Cancelled, or past the deadline of the phases after the search.
    fn past_deadline(&self) -> bool {
        self.cancel.load(Ordering::Relaxed) || self.deadline.get().is_some_and(|d| Instant::now() >= *d)
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

    /// Cancelled, or out of time.
    fn check_time(&self) {
        if self.cancel.load(Ordering::Relaxed) {
            self.set_stop(Stop::Cancelled);
        } else if self.start.elapsed() >= self.config.time_limit {
            self.set_stop(Stop::TimeLimit);
        }
    }

    /// [`Shared::check_time`], and the budget of nodes. Only the workers
    /// check the nodes, between two edges: a check from another thread
    /// would stop a dive wherever it happens to be, and one thread with a
    /// budget of nodes would not search the same from run to run.
    fn check_limits(&self) {
        self.check_time();
        if self.nodes.load(Ordering::Relaxed) >= self.config.max_nodes {
            self.set_stop(Stop::NodeLimit);
        }
    }

    /// Copies the best boards once the nodes got past the next of
    /// [`SearchConfig::checkpoints`]: between two edges, where the budget
    /// of nodes is checked, so with one thread at the same node as a search
    /// with that budget stops.
    fn checkpoint(&self) {
        let checkpoints = &self.config.checkpoints;
        let nodes = self.nodes.load(Ordering::Relaxed);
        let Ok(mut taken) = self.checkpoints.lock() else { return };
        if checkpoints.get(taken.len()).is_none_or(|&c| nodes < c) {
            return;
        }
        let mut best: Vec<Board> = self.boards.lock().map(|b| b.values().cloned().collect()).unwrap_or_default();
        best.sort_by(rank);
        best.truncate(self.config.checkpoint_top);
        // Past more than one at once: the same boards for each.
        while checkpoints.get(taken.len()).is_some_and(|&c| nodes >= c) {
            taken.push((nodes, best.clone()));
        }
    }

    /// A handtrap did nothing on your turn: after [`IDLE_STRIKES`] times, and
    /// unless it was seen to give something to do, it stays in the hand.
    fn strike(&self, code: u32) {
        let useful = self.useful_hand_traps.read().is_ok_and(|u| u.contains(&code));
        let strikes = self.idle_strikes.lock().map_or(0, |mut s| {
            let n = s.entry(code).or_default();
            *n += 1;
            *n
        });
        if let (false, true, Ok(mut idle)) = (useful, strikes >= IDLE_STRIKES, self.idle_hand_traps.write()) {
            idle.insert(code);
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
            for e in edges {
                let order = q.next_order;
                q.next_order += 1;
                q.edges.push(Edge {
                    prefix: e.prefix,
                    answer: e.answer,
                    priority: e.priority,
                    order,
                    confirms: e.confirms,
                    value: e.value,
                    deviation: e.deviation,
                });
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
            self.check_time();
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
            // Its own End Phase played: the same line, with the End Phase's answers.
            Some(old) if !old.exact && board.exact && confirms == Some(board.key) && board.line.answers.starts_with(&old.line.answers) => {
                true
            }
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
                // Only the best few sorted: this runs four times a second
                // under the lock that every new board waits for.
                let mut top: Vec<&Board> = b.values().collect();
                let k = self.config.live_top.min(top.len());
                if k > 0 && k < top.len() {
                    top.select_nth_unstable_by(k - 1, |x, y| rank(x, y));
                }
                top.truncate(k);
                top.sort_by(|x, y| rank(x, y));
                let top = top.into_iter().cloned().collect();
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
        let hits = self.config.goal.hits(self.engine, &state);
        Board {
            key: board_key(&state),
            cards_used: cards_used(&self.config.hand, &state),
            quick_score: self
                .values
                .read()
                .map_or(0.0, |v| quick_score(self.engine, &state, &self.config.hand_traps, &v, self.config.opponent.as_ref())),
            goal_hits: hits.hard,
            goal_soft: hits.soft,
            exact,
            line,
            state: Arc::new(state),
            found_after: self.elapsed(),
        }
    }
}

/// What the search steers by: the quick estimate, and the cards of the goal.
fn estimate(board: &Board) -> f64 {
    preferred(board) + 3.0 * board.goal_hits as f64
}

/// What a soft card of the goal adds to the estimate: less than a hard one,
/// about what a card that stops something alone is worth.
const SOFT_GOAL: f64 = 1.0;

/// The quick estimate, with the soft cards of the goal.
fn preferred(board: &Board) -> f64 {
    board.quick_score + SOFT_GOAL * board.goal_soft as f64
}

/// Best first: the hard cards of the goal, then a higher estimate (with the
/// soft ones), then fewer cards from the hand, then a shorter line; ties by
/// the board's key, so the same search keeps the same boards.
fn rank(a: &Board, b: &Board) -> CmpOrdering {
    b.goal_hits
        .cmp(&a.goal_hits)
        .then(preferred(b).total_cmp(&preferred(a)))
        .then(a.cards_used.cmp(&b.cards_used))
        .then(a.line.choices.len().cmp(&b.line.choices.len()))
        .then(a.key.cmp(&b.key))
}

/// Key of a node: the state (its canonical fingerprint) and the choice
/// being asked (two steps of the same selection share the state but not
/// the choice). Inside something
/// being done (an activation, a summon, an effect resolving), the answers
/// since your last free moment count too: the state does not say yet what
/// is being done, and choosing Aleister the Invoker as material for
/// Salamangreat Almiraj or for Artemis is the same choice of the same state.
fn node_key(state: u64, chain: &[ChainLink], decision: &Decision, pending: &Pending) -> u64 {
    let mut h = DefaultHasher::new();
    state.hash(&mut h);
    // The chain not resolved yet: the same field with A chained to B, B to
    // A, or A still to resolve is not the same moment.
    if !chain.is_empty() {
        chain.hash(&mut h);
    }
    decision.hash(&mut h);
    if !is_free(decision) {
        pending.value().hash(&mut h);
    }
    h.finish()
}

/// A moment where you choose what to do next (not a step of something
/// already started).
fn is_free(decision: &Decision) -> bool {
    decision.player() == Some(0) && matches!(decision, Decision::IdleCmd { .. } | Decision::BattleCmd { .. } | Decision::SelectChain { .. })
}

/// The answers given since your last free moment, hashed. The cards picked
/// in the selection being made are a set: picking A then B is picking B
/// then A (the same materials, chosen in another order).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Pending {
    /// The answers before the selection being made.
    before: u64,
    /// The cards picked in it, each hashed, summed.
    picks: u64,
}

impl Pending {
    fn value(&self) -> u64 {
        if self.picks == 0 {
            return self.before;
        }
        let mut h = DefaultHasher::new();
        (self.before, self.picks).hash(&mut h);
        h.finish()
    }

    fn answered(&mut self, decision: &Decision, answer: &[u8]) {
        if let Decision::SelectUnselectCard { selectable, unselectable, hint, .. } = decision {
            // A new selection (nothing picked yet) starts a new set.
            if unselectable.is_empty() {
                *self = Pending { before: self.value(), picks: 0 };
            }
            let index = answer.get(4..8).and_then(|b| b.try_into().ok()).map(i32::from_le_bytes).filter(|&i| i >= 0);
            if let Some(i) = index.map(|i| i as usize) {
                let (card, picked) = match selectable.get(i) {
                    Some(c) => (Some(c), true),
                    None => (unselectable.get(i - selectable.len()), false),
                };
                if let Some(card) = card {
                    let mut h = DefaultHasher::new();
                    (hint, card_order(card)).hash(&mut h);
                    self.picks = if picked { self.picks.wrapping_add(h.finish()) } else { self.picks.wrapping_sub(h.finish()) };
                    return;
                }
            }
        }
        let mut h = DefaultHasher::new();
        if !is_free(decision) {
            self.value().hash(&mut h);
        }
        answer.hash(&mut h);
        *self = Pending { before: h.finish(), picks: 0 };
    }
}

/// Key of a situation met since your last free moment, to find a line
/// going round in circles: the same choice asked again with nothing
/// happened in between (choosing a Fusion Monster whose materials cannot
/// be chosen, cancelling, choosing it again). Its state is the same, and
/// it costs nothing to compute while a line is replayed.
fn loop_key(decision: &Decision, events: usize) -> u64 {
    let mut h = DefaultHasher::new();
    ("loop", decision, events).hash(&mut h);
    h.finish()
}

/// The card an answer to your idle choice Sets (monsters first). Sets
/// made one after the other change nothing but the cards set, so they are
/// made in one order only: a line that Sets A then B reaches the state of
/// the one that Sets B then A, and would cost a replay to find it out.
fn set_key(decision: &Decision, answer: &[u8]) -> Option<(u8, u32)> {
    let Decision::IdleCmd { monster_settable, spell_settable, .. } = decision else { return None };
    let v = i32::from_le_bytes(answer.get(..4)?.try_into().ok()?);
    match v & 0xffff {
        3 => monster_settable.get((v >> 16) as usize).map(|c| (0, c.code)),
        4 => spell_settable.get((v >> 16) as usize).map(|c| (1, c.code)),
        _ => None,
    }
}

/// The Set made at your last free moment, if nothing happened since. The
/// chain window opened by a Set and passed changes nothing, so the Set
/// before it still counts.
fn after_answer(last_set: &mut Option<(u8, u32)>, decision: &Decision, answer: &[u8]) {
    let passed = matches!(decision, Decision::SelectChain { .. }) && answer.get(..4) == Some(&(-1i32).to_le_bytes()[..]);
    if is_free(decision) && !passed {
        *last_set = set_key(decision, answer);
    }
}

/// The situations met since your last free moment ([`loop_key`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SinceFree {
    seen: HashSet<u64>,
    events: usize,
}

impl SinceFree {
    /// Notes a decision and the events before it; false when it was
    /// already met since the last free moment.
    fn meet(&mut self, decision: &Decision, events: usize) -> bool {
        self.events += events;
        if is_free(decision) {
            self.seen.clear();
            self.events = 0;
            return true;
        }
        self.seen.insert(loop_key(decision, self.events))
    }
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

/// The cards on your field, whatever their zones (the Extra Monster Zone
/// included) and whatever the hand: boards with the same key are variants
/// of one field, to list once.
pub fn field_key(state: &DuelState) -> u64 {
    let p = &state.players[0];
    let mut cards: Vec<u64> = p.monsters.iter().flatten().map(|c| card_hash(state, (0, false), c)).collect();
    for (zone, c) in p.spells.iter().enumerate() {
        if let Some(c) = c {
            cards.push(card_hash(state, (zone.saturating_sub(4), false), c));
        }
    }
    cards.sort_unstable();
    let mut h = DefaultHasher::new();
    cards.hash(&mut h);
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

/// A rough value of a state for ordering the search (the probe turn
/// replaces it at the end with the real score): monsters on the field,
/// more for Extra Deck monsters, set cards, cards kept in hand, more for
/// `hand_traps`, and what the cards on the field stop alone (`values`).
pub fn quick_score(engine: &Engine, state: &DuelState, hand_traps: &[u32], values: &CardValues, opponent: Option<&Opponent>) -> f64 {
    let me = &state.players[0];
    let mut score = 0.0;
    for (z, c) in me.monsters.iter().enumerate() {
        let Some(c) = c else { continue };
        let extra = engine.cards().get(c.code).is_some_and(|d| d.type_ & card_type::EXTRA != 0);
        let measured = Spot::of_monster(z, c).is_some_and(|spot| values.measured(engine, c.code, spot));
        // A face-down monster applies no effect (a set Extra Deck monster
        // almost never flips). A monster measured counts for what it stops
        // (`of_field`), and a little as a body; the others by their kind.
        score += match (extra, c.is_face_up()) {
            _ if measured => MEASURED_BODY,
            (true, true) => 2.0,
            (true, false) => 0.3,
            (false, _) => 1.0,
        };
    }
    let mut set: Vec<u32> = Vec::new();
    for c in me.spells.iter().flatten() {
        let type_ = engine.cards().get(c.code).map_or(0, |d| d.type_);
        // A second copy Set counts what it was measured to add (`of_field`).
        if !c.is_face_up() {
            let code = engine.cards().canonical_code(c.code);
            let again = set.contains(&code);
            set.push(code);
            if again && values.measured(engine, code, Spot::SetSecond) {
                continue;
            }
        }
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
    // A handtrap counts what it stops from the hand with this field (some
    // only work while you control no cards), when it was measured.
    score += me
        .hand
        .iter()
        .map(|&c| match values.in_hand(engine, c, state) {
            InHand::Usable(v) => (VALUE_WEIGHT * v).max(0.3),
            // A handtrap that cannot be used with this field (the Mulcharmy
            // going first): worth nothing, the card to discard.
            InHand::Dead => 0.0,
            InHand::Unknown if hand_trap(c) => 1.5,
            InHand::Unknown => 0.3,
        })
        .sum::<f64>();
    score + VALUE_WEIGHT * (values.of_field(engine, state) + opponent.map_or(0.0, |o| taken_from(o, state)))
}

/// What your turn took from the opponent (B5a): its cards when the turn
/// began (`opponent`) against those it still has, pile by pile, at the
/// weights of the probe turn's [`crate::probe::attrition`]. Its Extra Deck
/// banished by Hyperinvoked Aeon is worth something to the search too.
fn taken_from(opponent: &Opponent, state: &DuelState) -> f64 {
    let them = &state.players[1];
    let gone = |before: usize, now: usize| before.saturating_sub(now) as f64;
    gone(opponent.extra.len(), them.extra.len()) * taken_weight(location::EXTRA)
        + gone(opponent.hand.len(), them.hand.len()) * taken_weight(location::HAND)
        + gone(opponent.deck.len(), them.deck.len()) * taken_weight(location::DECK)
}

/// What a face-up monster whose value was measured adds to the estimate
/// besides that value: a body to use as material or to block an attack.
const MEASURED_BODY: f64 = 0.3;

/// Weight in the estimate of what the cards on the field stop, alone
/// (1 = an interruption that negates anything once): an Invoked Mechaba
/// counts as much as three more monsters that do nothing on the opponent's
/// turn, Ash Blossom kept in hand a little less than it. Tried on 16 hands
/// of Master Duel Meta decks: 2 and 3 found weaker boards than 5.
const VALUE_WEIGHT: f64 = 5.0;

/// Answers that do something come first: the dive tries them before ending
/// the turn, and Sets (which give up a Normal Summon or a card's use this
/// turn) after the rest.
///
/// Responding to your own chain with quick effects is rarely what a combo
/// needs (it spends a card or an effect for little, like discarding one for
/// ATK or turning your own monster face-down), so there not chaining comes
/// first. Triggers of the same moment are put on the chain first.
///
/// Cards that can also be used on the opponent's turn are kept for it
/// longer: Quick-Play Spells and Traps after the other activations, and
/// handtraps after the Sets (an effect that only lets you do something
/// more, like Labrynth Cooclock's, needs the cards Set first).
///
/// Answers that pick or play a card of the goal come before everything, and
/// those that send one away after everything.
fn dive_order(actions: &mut [Action], responding: bool, goal: impl Fn(&Action) -> GoalPick, held: impl Fn(&Action) -> Option<i32>) {
    let rank = |a: &Action| match (goal(a), a.kind) {
        (GoalPick::Toward, _) => -1,
        (GoalPick::Away { .. }, _) => 5,
        (_, kind) => match kind {
            ActionKind::Activate if responding => 2,
            ActionKind::Pass if responding => 1,
            ActionKind::Activate => held(a).unwrap_or(0),
            ActionKind::EndTurn => 4,
            ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish => 3,
            ActionKind::SetMonster | ActionKind::SetSpellTrap => 2,
            _ => 0,
        },
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

/// The cards to keep are still in the hand (any artwork of them).
fn keeps_hand(engine: &Engine, config: &SearchConfig, state: &DuelState) -> bool {
    let db = engine.cards();
    let mut hand: Vec<u32> = state.players[0].hand.iter().map(|&c| db.canonical_code(c)).collect();
    config.keep_in_hand.iter().all(|&code| match hand.iter().position(|&c| c == db.canonical_code(code)) {
        Some(i) => {
            hand.swap_remove(i);
            true
        }
        None => false,
    })
}

/// What can be done at a free moment, card by card.
type Offer = (ActionKind, u32, u8, u64);

fn offered(db: &CardDb, actions: &[Action]) -> HashSet<Offer> {
    actions
        .iter()
        .map(|a| (a.kind, a.card.map_or(0, |c| db.canonical_code(c.code)), a.card.map_or(0, |c| c.location), a.description))
        .collect()
}

/// A handtrap activated from the hand at your idle moment, and what could
/// be done before it: to tell, once its link resolved with nothing
/// happening, whether it gave something new to do.
fn hand_trap_trial(
    db: &CardDb,
    decision: &Decision,
    actions: &[Action],
    answer: &[u8],
    hand_trap: impl Fn(u32) -> bool,
) -> Option<(u32, HashSet<Offer>)> {
    if !matches!(decision, Decision::IdleCmd { .. }) {
        return None;
    }
    let a = actions.iter().find(|a| a.response == answer && a.kind == ActionKind::Activate)?;
    let code = db.canonical_code(a.card.filter(|c| c.location == location::HAND)?.code);
    hand_trap(code).then(|| (code, offered(db, actions)))
}

const PHASE_DRAW: u16 = 0x1;
const PHASE_STANDBY: u16 = 0x2;
const PHASE_END: u16 = 0x200;

/// Times a handtrap does nothing at your idle moment before it is left in
/// the hand for the rest of the search.
const IDLE_STRIKES: u32 = 3;

fn new_duel(shared: &Shared) -> Result<Duel, Error> {
    let config = shared.config;
    Duel::goldfish_against_from(
        shared.engine,
        &config.deck,
        &config.hand,
        config.seed,
        config.opponent.as_ref(),
        &config.placed,
        &config.prelude,
    )
}

/// What `play` keeps track of along the line it replays.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Track {
    pending: Pending,
    /// The situations met since your last free moment: meeting one again
    /// means going round in circles. The line replayed counts too: the
    /// answer of an edge can lead back to a choice made in it.
    since_free: SinceFree,
    /// Your chain links, to tell which of your effects negates another.
    links: Links,
    last_set: Option<(u8, u32)>,
    /// Answers since the last free moment (for the stats).
    after_free: u64,
}

impl Track {
    /// Gives the next answer of a line being replayed.
    fn replay(&mut self, duel: &mut Duel, db: &CardDb, answer: &[u8]) -> Result<(), Error> {
        self.after_free += 1;
        if let Step::Decision { decision, events } = duel.advance()? {
            if is_free(&decision) {
                self.after_free = 1;
            }
            self.since_free.meet(&decision, events.len());
            self.pending.answered(&decision, answer);
            if events.iter().any(|e| matches!(e, Event::Chaining { .. })) {
                self.last_set = None;
            }
            after_answer(&mut self.last_set, &decision, answer);
            for e in &events {
                let _ = self.links.follow(db, e);
            }
        }
        duel.respond(answer);
        Ok(())
    }
}

/// A worker's duel in a memory arena, and copies of it along the lines it
/// replayed (plan B13 in docs/plans/velocita-ricerca.md): a line starts
/// from the longest copy of its beginning instead of from the start of the
/// duel. A copy puts back the whole duel, so what follows is the same.
/// Copies are made where a line replayed ends, as the other answers of its
/// last choice start there too, and the lines after them (copies on the way
/// as well were measured slower: they cost more than the replay they save).
struct Copies {
    duel: Duel,
    store: Store,
    /// With `DUELFORGE_COPY_CHECK=n`, every n-th line started from a copy is
    /// also replayed from the start, and the two are compared.
    check: Option<u64>,
    started: u64,
}

struct Store {
    /// The duel as made: lines with no copy start here.
    start: SavedDuel,
    /// By the hash of the answers before them ([`prefix_hashes`]).
    copies: HashMap<u64, Copy>,
    bytes: usize,
    cap: usize,
    clock: u64,
    /// The arena filled up: no more copies (see [`Duel::save`]).
    full: bool,
}

struct Copy {
    answers: Vec<Vec<u8>>,
    saved: SavedDuel,
    track: Track,
    used: u64,
}

/// The hash of each beginning of `answers`, the empty one first.
fn prefix_hashes(answers: &[Vec<u8>]) -> Vec<u64> {
    let mut hashes = Vec::with_capacity(answers.len() + 1);
    let mut last = 0u64;
    hashes.push(last);
    for a in answers {
        let mut h = DefaultHasher::new();
        (last, a).hash(&mut h);
        last = h.finish();
        hashes.push(last);
    }
    hashes
}

impl Copies {
    fn new(shared: &Shared) -> Result<Self, Error> {
        ygo_engine::arena_duels(true);
        let duel = new_duel(shared);
        ygo_engine::arena_duels(false);
        let duel = duel?;
        let start = duel.save()?;
        let check = std::env::var("DUELFORGE_COPY_CHECK").ok().and_then(|n| n.parse().ok()).filter(|&n| n > 0);
        let store = Store { start, copies: HashMap::new(), bytes: 0, cap: shared.config.copies_mb << 20, clock: 0, full: false };
        Ok(Copies { duel, store, check, started: 0 })
    }

    /// Puts the duel where `answers` lead, from the longest copy of their
    /// beginning, and copies it there.
    fn reach(&mut self, shared: &Shared, answers: &[Vec<u8>], dive: &mut Dive) -> Result<Track, Error> {
        let db = shared.engine.cards();
        let hashes = prefix_hashes(answers);
        self.store.clock += 1;
        let clock = self.store.clock;
        let found = (1..hashes.len()).rev().find(|&k| self.store.copies.get(&hashes[k]).is_some_and(|c| c.answers[..] == answers[..k]));
        let (from, mut track) = match found.and_then(|k| Some((k, self.store.copies.get_mut(&hashes[k])?))) {
            Some((k, copy)) => {
                copy.used = clock;
                self.duel.restore(&copy.saved)?;
                (k, copy.track.clone())
            }
            None => {
                self.duel.restore(&self.store.start)?;
                (0, Track::default())
            }
        };
        if from > 0 {
            self.started += 1;
            if self.check.is_some_and(|n| self.started.is_multiple_of(n)) {
                check_copy(shared, &self.duel, &answers[..from], &track)?;
            }
        }
        for answer in &answers[from..] {
            dive.replayed += 1;
            track.replay(&mut self.duel, db, answer)?;
        }
        if from < answers.len() {
            self.store.keep(&self.duel, answers, hashes[answers.len()], &track);
        }
        Ok(track)
    }
}

impl Shared<'_> {
    /// A worker's duel in an arena with its copies: one the search made, or
    /// a new one (`None`: no copies asked, or no arena left; the lines are
    /// then replayed from the start).
    fn take_copies(&self) -> Option<Copies> {
        if self.config.copies_mb == 0 {
            return None;
        }
        if let Some(c) = self.copies.lock().ok().and_then(|mut c| c.pop()) {
            return Some(c);
        }
        Copies::new(self).inspect_err(|e| eprintln!("search: {e}; the lines are replayed from the start")).ok()
    }

    fn give_back(&self, copies: Option<Copies>) {
        if let (Some(c), Ok(mut pool)) = (copies, self.copies.lock()) {
            pool.push(c);
        }
    }
}

impl Store {
    fn keep(&mut self, duel: &Duel, answers: &[Vec<u8>], hash: u64, track: &Track) {
        if self.full || self.cap == 0 || self.copies.contains_key(&hash) {
            return;
        }
        // The copies used least recently go first, and the memory of the
        // last one makes the new copy.
        let mut spare = None;
        while self.bytes + self.start.size() > self.cap {
            let Some(c) = self.evict() else { break };
            spare = Some(c.saved);
        }
        let saved = match spare.map_or_else(|| duel.save(), |old| duel.save_over(old)) {
            Ok(saved) => saved,
            Err(e) => {
                // The copies made so far still go back in; no new ones.
                eprintln!("search: {e}; the lines are replayed from the copies made so far");
                self.full = true;
                return;
            }
        };
        self.bytes += saved.size();
        self.copies.insert(hash, Copy { answers: answers.to_vec(), saved, track: track.clone(), used: self.clock });
        while self.bytes > self.cap && self.evict().is_some() {}
    }

    /// Takes out the copy used least recently.
    fn evict(&mut self) -> Option<Copy> {
        let oldest = *self.copies.iter().min_by_key(|(_, c)| c.used)?.0;
        let c = self.copies.remove(&oldest)?;
        self.bytes -= c.saved.size();
        Some(c)
    }
}

/// A line started from a copy must be where the same line replayed from the
/// start of the duel is.
fn check_copy(shared: &Shared, duel: &Duel, answers: &[Vec<u8>], track: &Track) -> Result<(), Error> {
    let mut replayed = new_duel(shared)?;
    let mut expected = Track::default();
    for answer in answers {
        expected.replay(&mut replayed, shared.engine.cards(), answer)?;
    }
    if replayed.state() != duel.state() || expected != *track {
        eprintln!("search: a copy of the duel after {} answers is not where the replay is", answers.len());
        return Err(Error::Engine(format!("a copy of the duel after {} answers is not where the replay is", answers.len())));
    }
    Ok(())
}

/// Drops the answers that lead back where the search has been or goes
/// anyway, which would cost a replay of the whole line each:
/// - taking a card out of a selection (picking the others directly gets
///   the same sets);
/// - cancelling what was just started at a free moment, or right after a
///   card or name was picked, before anything happened (it only goes back
///   there: the free moment again, or the Fusion Monster to choose; a cancel
///   later in an effect, like choosing no card, stays);
/// - picking a card that makes a selection another line has already made
///   in another order (the same cards picked, out of the same ones).
fn without_returns(
    shared: &Shared,
    state: &DuelState,
    decision: &Decision,
    pending: &Pending,
    mut actions: Vec<Action>,
    undoable: bool,
) -> Vec<Action> {
    let only_answer = actions.len() == 1;
    if undoable && !only_answer && actions.iter().any(|a| a.kind != ActionKind::Cancel) {
        actions.retain(|a| a.kind != ActionKind::Cancel);
    }
    let Decision::SelectUnselectCard { selectable, unselectable, hint, min, max, .. } = decision else { return actions };
    let pick = |a: &Action| -> Option<usize> {
        if a.kind != ActionKind::Choose {
            return None;
        }
        let bytes: [u8; 4] = a.response.get(4..8)?.try_into().ok()?;
        Some(i32::from_le_bytes(bytes) as usize)
    };
    // The cards of the selection, picked or not: two orders of picking the
    // same cards out of the same ones meet at the same selection.
    let mut base = DefaultHasher::new();
    // What the selection is for: the answers since your last free moment.
    ("selection", fingerprint(state), pending.before, hint, min, max).hash(&mut base);
    let mut all: Vec<&CardRef> = selectable.iter().chain(unselectable).collect();
    all.sort_by_key(|c| card_order(c));
    all.iter().for_each(|c| card_order(c).hash(&mut base));
    actions
        .into_iter()
        .filter(|a| match pick(a) {
            // Taking a card out.
            Some(i) if i >= selectable.len() => false,
            Some(i) => {
                let mut picked: Vec<_> = unselectable.iter().chain(std::iter::once(&selectable[i])).map(card_order).collect();
                picked.sort_unstable();
                let mut h = base.clone();
                picked.hash(&mut h);
                shared.first_visit(h.finish())
            }
            None => true,
        })
        .collect()
}

fn card_order(c: &CardRef) -> (u8, u8, u32, u32, u32) {
    (c.controller, c.location, c.sequence, c.code, c.position)
}

/// The answer starts something that cancelling right after would only take
/// back: an action at a free moment, or a card or name picked in an effect
/// (cancelling the materials of a Fusion Summon asks for the Fusion Monster
/// again).
fn starts(decision: &Decision, kind: ActionKind) -> bool {
    if is_free(decision) {
        kind != ActionKind::Pass
    } else {
        matches!(kind, ActionKind::Choose | ActionKind::Announce)
    }
}

/// The state as far as an answer shows in it: what was activated this turn
/// and the cards' status bits (which the activation itself changes) left
/// out. An answer that changes nothing else is kept by the engine for later:
/// a target, a name declared, an option.
fn shown_state(state: &DuelState) -> u64 {
    let mut s = state.clone();
    s.activity = Default::default();
    for p in &mut s.players {
        for c in p.monsters.iter_mut().chain(p.spells.iter_mut()).flatten() {
            c.status = 0;
        }
    }
    canonical_fingerprint(&s)
}

/// Priority taken off an answer for each answer of its line that is not
/// the dive's, growing with its rank in [`dive_order`] (1 for the second
/// answer): the lines near the best ones are tried first, the ones that
/// differ in many choices, or take answers the order puts last, later.
fn deviation(rank: usize) -> f64 {
    let (base, step) = *DEVIATION;
    base + step * rank.saturating_sub(1) as f64
}

static DEVIATION: std::sync::LazyLock<(f64, f64)> = std::sync::LazyLock::new(|| {
    std::env::var("DUELFORGE_DEVIATION")
        .ok()
        .and_then(|v| v.split_once(',').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))))
        .unwrap_or((0.6, 0.0))
});

/// Plays one edge: replays its line, gives its answer, then dives. The
/// other answers met on the way are queued when the dive is over, at the
/// estimate of the best board the line reached after them (the boards
/// before an answer do not depend on it), less [`deviation`] for each
/// answer of their line that is not the dive's; the other zones of a line's
/// last choice go last.
fn work(shared: &Shared, edge: Edge, copies: Option<&mut Copies>) -> Result<(), Error> {
    let started = Instant::now();
    let mut dive =
        Dive { left: Vec::new(), boards: Vec::new(), end: stats::End::Error, nodes: 0, replay: Duration::ZERO, replayed: 0, after_free: 0 };
    if let Some(stats) = &shared.stats {
        stats.popped(&edge);
    }
    let result = play(shared, &edge, &mut dive, copies);
    if let Some(stats) = &shared.stats {
        stats.dive(&dive, started.elapsed());
    }
    // Best board estimate from each depth of the line on.
    let mut after: Vec<(usize, f64)> = dive.boards.clone();
    after.sort_by_key(|&(depth, _)| std::cmp::Reverse(depth));
    let mut best = f64::NEG_INFINITY;
    let suffix: Vec<(usize, f64)> = after
        .into_iter()
        .map(|(depth, value)| {
            best = best.max(value);
            (depth, best)
        })
        .collect();
    let fallback = dive.boards.iter().map(|b| b.1).fold(edge.value, f64::max);
    let edges = dive
        .left
        .into_iter()
        .map(|left| {
            // The last board reading at this depth or deeper.
            let value = suffix.iter().rev().find(|(d, _)| *d >= left.depth).map_or(fallback, |&(_, v)| v);
            // Lines that bring a card of the goal first, those that lose its
            // last copy last (still tried: a goal can then be found out of reach).
            let bonus = match left.goal {
                GoalPick::Toward => 10.0,
                GoalPick::Away { last: true } => -10.0,
                _ => 0.0,
            };
            let last = if left.last_zone { END_PHASE_LAST } else { 0.0 };
            let deviation = edge.deviation + deviation(left.rank);
            NewEdge {
                prefix: left.prefix,
                answer: Some(left.answer),
                priority: value - deviation + bonus - last,
                confirms: None,
                value,
                deviation,
            }
        })
        .collect();
    shared.push(edges);
    result
}

/// What a dive leaves behind: the answers it did not give, and the
/// estimates of the boards it read, by depth (choices made).
struct Dive {
    left: Vec<LeftAnswer>,
    boards: Vec<(usize, f64)>,
    /// For the stats: why the dive ended, its nodes, the time of its replay,
    /// the answers it replayed and how many of them came after the last
    /// free moment (where a copy of the duel could start them).
    end: stats::End,
    nodes: u64,
    replay: Duration,
    replayed: u64,
    after_free: u64,
}

struct LeftAnswer {
    prefix: Arc<Line>,
    answer: Answer,
    depth: usize,
    goal: GoalPick,
    kind: ActionKind,
    /// Its place among the node's answers in the dive's order (0 for the one given).
    rank: usize,
    /// Another zone for the monster of the line's last choice (see `play`).
    last_zone: bool,
}

fn play(shared: &Shared, edge: &Edge, dive: &mut Dive, copies: Option<&mut Copies>) -> Result<(), Error> {
    let config = shared.config;
    // The board was already played to the end of the turn from another line
    // reaching it, with a line as good (else it would be provisional again):
    // its End Phase would only be read once more.
    if let Some(key) = edge.confirms.filter(|_| !shared.confirming.load(Ordering::Relaxed)) {
        if shared.boards.lock().is_ok_and(|b| b.get(&key).is_none_or(|b| b.exact)) {
            dive.end = stats::End::Confirmed;
            return Ok(());
        }
    }
    let replay_started = Instant::now();
    let mut line = (*edge.prefix).clone();
    let db = shared.engine.cards();
    let mut own: Duel;
    let (duel, track) = match copies {
        Some(copies) => {
            let track = copies.reach(shared, &line.answers, dive)?;
            (&mut copies.duel, track)
        }
        None => {
            own = new_duel(shared)?;
            let mut track = Track::default();
            for answer in &line.answers {
                dive.replayed += 1;
                track.replay(&mut own, db, answer)?;
            }
            (&mut own, track)
        }
    };
    let Track { mut pending, mut since_free, mut links, mut last_set, after_free } = track;
    dive.after_free = after_free;
    let hand_trap = |code: u32| config.hand_traps.iter().any(|&h| db.canonical_code(h) == code);
    // A handtrap activated at your idle moment, until the next one, and
    // whether its link resolved with nothing happening.
    let mut trial: Option<(u32, HashSet<Offer>)> = None;
    let mut quiet = false;
    if !line.answers.is_empty() || edge.answer.is_some() {
        shared.replays.fetch_add(1, Ordering::Relaxed);
    }
    // The edge's answer started something (see `undoable`).
    let mut started = false;
    if let Some((answer, label)) = edge.answer.clone() {
        if let Step::Decision { decision, events } = duel.advance()? {
            started = duel.actions(&decision).iter().any(|a| a.response == answer && starts(&decision, a.kind));
            since_free.meet(&decision, events.len());
            pending.answered(&decision, &answer);
            if events.iter().any(|e| matches!(e, Event::Chaining { .. })) {
                last_set = None;
            }
            after_answer(&mut last_set, &decision, &answer);
            for e in &events {
                let _ = links.follow(db, e);
            }
            // Queued before the handtrap was learned to do nothing here.
            if shared.idle_hand_traps.read().is_ok_and(|idle| {
                !idle.is_empty()
                    && duel.actions(&decision).iter().any(|a| {
                        a.response == answer
                            && a.kind == ActionKind::Activate
                            && a.card.is_some_and(|c| c.location == location::HAND && idle.contains(&db.canonical_code(c.code)))
                    })
            }) {
                if let Some(stats) = &shared.stats {
                    stats.cut(stats::Cut::IdleHandTrap, 1);
                }
                shared.duplicates.fetch_add(1, Ordering::Relaxed);
                dive.end = stats::End::IdleHandTrap;
                return Ok(());
            }
            if let Some(Ok(mut c)) = shared.coverage.as_ref().map(Mutex::lock) {
                coverage::chosen(shared.engine.cards(), &decision, &duel.actions(&decision), &answer, &mut c);
            }
            if matches!(decision, Decision::IdleCmd { .. }) {
                trial = hand_trap_trial(db, &decision, &duel.actions(&decision), &answer, hand_trap);
            }
        }
        duel.respond(&answer);
        line.choose(answer, label);
    }
    // The card last taken from the Extra Deck: the one a Monster Zone is
    // asked for next (only those go to the Extra Monster Zone).
    let mut placing: Option<u32> = None;
    // Something was started at a free moment (an idle or chain window), or
    // a card or name picked, and nothing has happened since: cancelling
    // only goes back there.
    let mut undoable = started;
    // What resolved quietly in the part replayed is not news.
    let _ = links.take_silent();
    // The last answer this dive gave: stopped before its node is read, the
    // answer goes back in the queue, so a resumed search plays on from it.
    let mut given: Option<LeftAnswer> = None;
    // The dive's last answer when it was not given at a free moment: its
    // depth, its kind and the state shown before it; and whether a free
    // moment was answered (automatically) since.
    let mut hidden: Option<(usize, ActionKind, u64)> = None;
    let mut freed = false;
    dive.replay = replay_started.elapsed();
    // For the stats: the state shown at the last node, and what was given there.
    let mut last_shown: Option<(u64, ActionKind)> = None;
    loop {
        if shared.halted() {
            match given.take() {
                Some(left) => dive.left.push(left),
                None => shared.push(vec![NewEdge {
                    prefix: edge.prefix.clone(),
                    answer: edge.answer.clone(),
                    priority: edge.priority,
                    confirms: edge.confirms,
                    value: edge.value,
                    deviation: edge.deviation,
                }]),
            }
            dive.end = stats::End::Halted;
            return Ok(());
        }
        let (step, mut actions) = duel.advance_auto_seen(&config.policy, &mut line.answers, |d, a| {
            pending.answered(d, a);
            after_answer(&mut last_set, d, a);
            freed |= is_free(d);
        })?;
        let (turn, _, phase) = duel.turn_info();
        let (Step::Decision { events, .. } | Step::End { events }) = &step;
        let mut negated = false;
        let mut cover = shared.coverage.as_ref().and_then(|c| c.lock().ok());
        for e in events {
            let negation = links.follow(db, e);
            if let Some(c) = cover.as_mut() {
                coverage::count(db, e, negation.as_ref(), c);
            }
            // Negated by another of your effects; a card disabled when its
            // own link resolves may have done its job (a Pendulum Scale).
            if let Some(by) = negation.and_then(|n| n.by) {
                negated = true;
                if let Ok(mut negators) = shared.own_negators.write() {
                    negators.insert(by);
                }
            }
            // A handtrap that did nothing: the same line without it keeps
            // the card in hand (to discard for a cost, or for the
            // opponent's turn), so this one only loses. Activated at your
            // idle moment, it is judged at the next one by what it gave to
            // do; chained to something, the line goes. Either way it counts
            // towards leaving it in the hand.
            if let Some(m) = links.take_silent().filter(|m| hand_trap(m.code)) {
                match &trial {
                    Some((code, _)) if *code == m.code => quiet = true,
                    _ => {
                        negated = true;
                        shared.strike(m.code);
                    }
                }
            }
        }
        drop(cover);
        if let Step::Decision { decision: idle @ Decision::IdleCmd { .. }, .. } = &step {
            if let Some((code, before)) = trial.take().filter(|_| quiet) {
                let new = offered(db, &duel.actions(idle)).difference(&before).any(|o| o.1 != code);
                if new {
                    if let Ok(mut useful) = shared.useful_hand_traps.write() {
                        useful.insert(code);
                    }
                    if let Ok(mut idle) = shared.idle_hand_traps.write() {
                        idle.remove(&code);
                    }
                } else {
                    negated = true;
                    shared.strike(code);
                }
            }
            quiet = false;
        }
        if events.iter().any(|e| matches!(e, Event::Chaining { .. })) {
            last_set = None;
        }
        // On your turn the opponent never answers: an activation negated, or
        // its effect, was negated by one of your own cards (Invoked Mechaba
        // on your own Hyperinvoked Aeon), or a handtrap did nothing. Such a
        // line only loses.
        if negated {
            if std::env::var_os("DUELFORGE_DEBUG_NEGATION").is_some() {
                let told: Vec<String> = events.iter().filter_map(|e| italian::describe(db, e)).collect();
                eprintln!("negated after: {}\n  events: {}", line.choices.join(" | "), told.join(" | "));
            }
            dive.end = stats::End::Negated;
            return Ok(());
        }
        let happened = events.len();
        // A chain link finished resolving since the last node.
        let resolved = events.iter().any(|e| matches!(e, Event::ChainSolved { .. } | Event::ChainEnd));
        if let Step::Decision { .. } = &step {
            undoable &= happened == 0;
        }
        let decision = match step {
            Step::Decision { decision, .. } if decision.player() == Some(0) && turn <= 1 => decision,
            // The opponent's turn (or the end of the duel): the board is done.
            _ => {
                let state = duel.state();
                if keeps_hand(shared.engine, config, &state) {
                    let board = shared.board(state, line, true);
                    dive.boards.push((board.line.choices.len(), estimate(&board)));
                    shared.record(board, edge.confirms);
                }
                // The turn ended right after a zone was chosen, with nothing
                // else to choose: the other zones only move that monster, a
                // replay each, and the best boards are played with them at
                // the end anyway (`zone_variants`). Queued last.
                if let Some(at) = given.as_ref().filter(|g| g.kind == ActionKind::Place).map(|g| g.depth) {
                    for left in dive.left.iter_mut().filter(|l| l.depth == at && l.kind == ActionKind::Place) {
                        left.last_zone = true;
                    }
                }
                dive.end = stats::End::Board;
                return Ok(());
            }
        };
        let state = duel.state();
        // The last answer changed nothing shown and its link has not resolved
        // yet: the engine keeps it for later (a target, a name declared), and
        // this node's key does not tell it, since the answers given since your
        // last free moment are forgotten here. The other answers of the same
        // kind there lead to this same key: a replay each to find a duplicate.
        if let Some((depth, kind, before)) = hidden.take() {
            if (freed || is_free(&decision)) && !resolved && shown_state(&state) == before {
                let same = |l: &LeftAnswer| l.depth == depth && l.kind == kind;
                if let Some(stats) = &shared.stats {
                    stats.cut(stats::Cut::Hidden, dive.left.iter().filter(|l| same(l)).count() as u64);
                }
                dive.left.retain(|l| !same(l));
            }
        }
        shared.nodes.fetch_add(1, Ordering::Relaxed);
        dive.nodes += 1;
        if let Some(stats) = &shared.stats {
            let shown = shown_state(&state);
            if let Some((before, kind)) = last_shown.filter(|(before, _)| happened == 0 && *before == shown) {
                stats.no_change(kind, before);
            }
            last_shown = Some((shown, ActionKind::Pass));
        }
        let free = is_free(&decision);
        let canonical = canonical_fingerprint(&state);
        // The same state met again since the last free moment, by the
        // same choice with nothing happened in between or by its state.
        if !since_free.meet(&decision, happened)
            || (!free && !since_free.seen.insert(node_key(canonical, duel.chain(), &decision, &Pending::default())))
        {
            shared.duplicates.fetch_add(1, Ordering::Relaxed);
            dive.end = stats::End::Loop;
            return Ok(());
        }
        let key = node_key(canonical, duel.chain(), &decision, &pending);
        if !shared.first_visit(key) {
            shared.duplicates.fetch_add(1, Ordering::Relaxed);
            // Another order of the same choices: met at a free moment (whole
            // plays swapped, B2), in a selection with cards picked (B2b), or
            // at another step of something being done.
            dive.end = match &shared.stats {
                Some(stats) if stats.same_choices(key, &line) => {
                    if free {
                        stats::End::OtherOrder
                    } else if pending.picks != 0 {
                        stats::End::OtherOrderPicks
                    } else {
                        stats::End::OtherOrderInside
                    }
                }
                _ => stats::End::Duplicate,
            };
            return Ok(());
        }
        if let Some(stats) = &shared.stats {
            stats.first(key, &line);
        }
        // Your effects known to negate your own chain links are not chained
        // to it (they are left out like the answer policy's).
        if matches!(decision, Decision::SelectChain { .. }) && duel.open_chain() > 0 {
            if let Ok(negators) = shared.own_negators.read() {
                if !negators.is_empty() {
                    let before = actions.len();
                    actions.retain(|a| {
                        a.kind != ActionKind::Activate || !coverage::mechanic(db, &decision, a).is_some_and(|(m, _)| negators.contains(&m))
                    });
                    if let Some(stats) = &shared.stats {
                        stats.cut(stats::Cut::OwnNegator, (before - actions.len()) as u64);
                    }
                }
            }
        }
        // Handtraps known to do nothing on your turn stay in the hand.
        if let Ok(idle) = shared.idle_hand_traps.read() {
            if !idle.is_empty() {
                let before = actions.len();
                actions.retain(|a| {
                    a.kind != ActionKind::Activate
                        || !a.card.is_some_and(|c| c.location == location::HAND && idle.contains(&db.canonical_code(c.code)))
                });
                if let Some(stats) = &shared.stats {
                    stats.cut(stats::Cut::IdleHandTrap, (before - actions.len()) as u64);
                }
            }
        }
        // The cards to keep in the hand are not played or paid from it
        // while no other copy is left there (a line that spends them
        // records no board), unless nothing else can be answered.
        if !config.keep_in_hand.is_empty() {
            let spends = |a: &Action| {
                a.card.is_some_and(|c| {
                    let code = db.canonical_code(c.code);
                    let kept = config.keep_in_hand.iter().filter(|&&k| db.canonical_code(k) == code).count();
                    c.controller == 0
                        && c.location == location::HAND
                        && kept > 0
                        && state.players[0].hand.iter().filter(|&&h| db.canonical_code(h) == code).count() <= kept
                })
            };
            if actions.iter().any(|a| !spends(a)) {
                actions.retain(|a| !spends(a));
            }
        }
        // Sets in a row in one order only (left out like the answer policy's).
        if let Some(k) = last_set.filter(|_| matches!(decision, Decision::IdleCmd { .. })) {
            actions.retain(|a| set_key(&decision, &a.response).is_none_or(|s| s >= k));
        }
        let mut audit = shared.coverage.as_ref().map(|_| Audit::new(duel.actions(&decision), &actions));
        let record = |audit: Option<Audit>, chosen: Option<&[u8]>| {
            if let (Some(audit), Some(Ok(mut c))) = (audit, shared.coverage.as_ref().map(Mutex::lock)) {
                audit.record(shared.engine.cards(), &decision, chosen, &mut c);
            }
        };
        if line.choices.len() >= config.max_choices {
            // Too long: end the turn, or close what is pending as fast as possible.
            dive_order(&mut actions, false, |_| GoalPick::Other, |_| None);
            let wrap_up = |a: &Action| {
                matches!(a.kind, ActionKind::EndTurn | ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish)
            };
            let keep = actions.iter().position(wrap_up).unwrap_or(0);
            actions = vec![actions.swap_remove(keep)];
            if let Some(a) = &mut audit {
                a.cut(&actions);
            }
        }
        if actions.is_empty() {
            record(audit, None);
            shared.interrupted.fetch_add(1, Ordering::Relaxed);
            dive.end = stats::End::NoAction;
            return Ok(());
        }
        if actions.len() > 1 {
            actions = without_returns(shared, &state, &decision, &pending, actions, undoable);
            if let Some(a) = &mut audit {
                a.returns(&actions);
            }
            if actions.is_empty() {
                // Every pick leads where another line already goes.
                record(audit, None);
                shared.duplicates.fetch_add(1, Ordering::Relaxed);
                dive.end = stats::End::Returns;
                return Ok(());
            }
        }
        let goal = |a: &Action| if config.goal.is_set() { config.goal.pick(shared.engine, &decision, &state, a) } else { GoalPick::Other };
        // Quick effects in your own End Phase rarely help a combo (Hyperinvoked
        // Aeon's Attribute change, a negation of your own effect): not
        // activating them comes first there too, like when responding.
        // An empty chain window outside the Main Phase too: what can be
        // activated there is offered again at your idle moments, while a
        // quick card used at the Draw Phase is gone for every line after it.
        // (In the Main Phase, combos often go on from such a window.)
        let responding = matches!(decision, Decision::SelectChain { triggers: false, .. })
            && (duel.open_chain() > 0 || phase & (PHASE_DRAW | PHASE_STANDBY | PHASE_END) != 0);
        let held = |a: &Action| {
            let c = a.card?;
            if c.location == location::HAND && hand_trap(db.canonical_code(c.code)) {
                return Some(3);
            }
            let quick = db.get(c.code).is_some_and(|d| d.type_ & (card_type::QUICKPLAY | card_type::TRAP) != 0);
            (quick && c.location & (location::HAND | location::SZONE) != 0).then_some(1)
        };
        dive_order(&mut actions, responding, goal, held);
        // A goal card wanted in a kind of zone goes there; while one wants
        // an Extra Monster Zone, other monsters leave it free when they can
        // (holding back the other Extra Deck summons instead would also hold
        // back the Link monsters used as material on the way, like Artemis
        // for Invoked Mechaba).
        if let (Decision::SelectPlace { .. }, Some(code)) = (&decision, placing) {
            let first = match config.goal.zone_of(shared.engine, code) {
                Some(zone) => Some(zone == GoalZone::Extra),
                None if config.goal.wants_extra() => Some(false),
                None => None,
            };
            if let Some(first) = first {
                let extra = |a: &Action| matches!(*a.response.as_slice(), [0, location::MZONE, seq] if seq >= 5);
                actions.sort_by_key(|a| extra(a) != first);
            }
        }
        if let Some(stats) = &shared.stats {
            stats.order(&line, &actions);
        }
        let prefix = Arc::new(line.clone());
        let depth = line.choices.len();
        // Ending the turn here: read the board now, play the End Phase later.
        if actions.len() > 1 {
            if let Some(i) = actions.iter().position(|a| a.kind == ActionKind::EndTurn) {
                let end = actions.remove(i);
                let label = italian::choice(&end);
                if keeps_hand(shared.engine, config, &state) {
                    let mut ended = line.clone();
                    ended.choose(end.response.clone(), label.clone());
                    let board = shared.board(state.clone(), ended, false);
                    let key = board.key;
                    let value = estimate(&board);
                    dive.boards.push((depth, value));
                    // A board that reaches the goal is played to the end at once.
                    let priority = if config.goal.reached(board.goal_hits) { value + END_PHASE_LAST } else { value - END_PHASE_LAST };
                    shared.record(board, None);
                    shared.push(vec![NewEdge {
                        prefix: prefix.clone(),
                        answer: Some((end.response, label)),
                        priority,
                        confirms: Some(key),
                        value,
                        deviation: edge.deviation,
                    }]);
                }
            }
        }
        // Lines that pick a card of the goal are tried before the others
        // (see `Goal::pick`).
        dive.left.extend(actions.iter().enumerate().skip(1).map(|(rank, a)| LeftAnswer {
            prefix: prefix.clone(),
            answer: (a.response.clone(), italian::choice(a)),
            depth,
            goal: goal(a),
            kind: a.kind,
            rank,
            last_zone: false,
        }));
        let first = &actions[0];
        if matches!(decision, Decision::IdleCmd { .. }) && first.kind == ActionKind::Activate {
            trial = hand_trap_trial(db, &decision, &duel.actions(&decision), &first.response, hand_trap);
        }
        given = Some(LeftAnswer {
            prefix,
            answer: (first.response.clone(), italian::choice(first)),
            depth,
            goal: goal(first),
            kind: first.kind,
            rank: 0,
            last_zone: false,
        });
        hidden = (!free).then(|| (depth, first.kind, shown_state(&state)));
        if let Some((_, kind)) = last_shown.as_mut() {
            *kind = first.kind;
        }
        freed = false;
        record(audit, Some(&first.response));
        if let Some(c) = first.card.filter(|c| c.location == location::EXTRA) {
            placing = Some(c.code);
        }
        undoable = starts(&decision, first.kind) || (undoable && !free);
        pending.answered(&decision, &first.response);
        after_answer(&mut last_set, &decision, &first.response);
        duel.respond(&first.response);
        line.choose(first.response.clone(), italian::choice(first));
    }
}

/// Runs the workers until the queue is empty or a limit is hit.
fn run_workers(shared: &Shared, threads: usize, mut progress: impl FnMut(&Progress)) {
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(move || {
                let mut copies = shared.take_copies();
                while let Some(edge) = shared.pop() {
                    if let Err(e) = work(shared, edge, copies.as_mut()) {
                        if std::env::var_os("DUELFORGE_DEBUG_SEARCH").is_some() {
                            eprintln!("interrupted: {e}");
                        }
                        shared.interrupted.fetch_add(1, Ordering::Relaxed);
                    }
                    shared.done_with_edge();
                    shared.check_limits();
                    // Before the round of learning: a search that stops
                    // here does not learn either.
                    shared.checkpoint();
                    let learning = Instant::now();
                    learn(shared);
                    if let Some(stats) = &shared.stats {
                        stats.learned(learning.elapsed());
                    }
                }
                shared.give_back(copies);
            });
        }
        while !shared.stopped() {
            std::thread::sleep(Duration::from_millis(250));
            shared.check_time();
            progress(&shared.progress());
        }
    });
}

/// Nodes between two rounds of learning.
const LEARN_EVERY: u64 = 1500;
/// Boards measured in a round.
const LEARN_BOARDS: usize = 2;
/// Best boards a round looks at.
const LEARN_AMONG: usize = 200;
/// Boards measured in the round at the end of the search.
const LEARN_LAST: usize = 4;
/// Time a round of learning may take past the search's time limit (or past
/// its start, when it starts later): measuring a board takes well under a
/// second, but with quick effects that check every combination of
/// materials at each chain window (Kewl Tune) it can take minutes. The
/// phases after the search (the last round, playing the best boards to the
/// end, their zone variants) all stop this long after the limit too. On
/// the slow Elfnote Kewl Tune hand of `docs/plans/velocita-ricerca.md` the
/// best boards come from the whole last round and from the zone variants
/// played to their end: an earlier deadline after a search that ended
/// early loses them.
const LEARN_GRACE: Duration = Duration::from_secs(10);

/// Your cards on the field of `state`, by kind of zone, as keys, with a
/// weight: 2 for a monster from the Extra Deck, else 1.
fn field_spots(state: &DuelState) -> Vec<(u64, u32)> {
    let me = &state.players[0];
    let key = |code: u32, spot: Spot| {
        let mut h = DefaultHasher::new();
        (code, spot).hash(&mut h);
        h.finish()
    };
    let monsters = me.monsters.iter().enumerate().filter_map(|(z, c)| {
        let c = c.as_ref().filter(|c| c.is_face_up())?;
        let weight = if c.type_ & card_type::EXTRA != 0 { 2 } else { 1 };
        Some((key(c.code, Spot::of_monster(z, c)?), weight))
    });
    let spells = me.spells.iter().flatten().map(|c| (key(c.code, Spot::of_spell(c)), 1));
    monsters.chain(spells).collect()
}

/// Every [`LEARN_EVERY`] nodes, the best boards not measured yet are put to
/// the probe turn, and each card of yours that stopped something there is
/// worth at least that from then on. A card alone on the field may be worth
/// nothing and a lot next to another one (Allied Code Talker @Ignister
/// negates by Tributing a Link Monster it points to): the search learns it
/// the first time such a board comes up, and the boards already found are
/// estimated again. With one thread the rounds come at the same nodes, so
/// the search stays the same from run to run.
fn learn(shared: &Shared) {
    if shared.stopped() {
        return;
    }
    let next = shared.next_learning.load(Ordering::Relaxed);
    if shared.nodes.load(Ordering::Relaxed) < next
        || shared.next_learning.compare_exchange(next, next + LEARN_EVERY, Ordering::Relaxed, Ordering::Relaxed).is_err()
    {
        return;
    }
    learn_round(shared, LEARN_BOARDS);
}

/// One round of [`learn`]: up to `count` boards measured.
fn learn_round(shared: &Shared, count: usize) {
    let Some(kit) = shared.config.learn.as_deref() else { return };
    // The lines are measured again in a duel of their own: only the same
    // duel as the search's (no cards placed, no prelude, the kit's
    // opponent) gives their answers the same meaning.
    if !shared.config.placed.is_empty()
        || shared.config.prelude != Prelude::default()
        || shared.config.opponent.as_ref() != Some(kit.opponent())
    {
        return;
    }
    // Among the best boards, the ones with the most cards on the field not
    // measured on a board yet, Extra Deck monsters counting double (the
    // first boards are often the same cards but one).
    let picked: Vec<Board> = {
        let Ok(boards) = shared.boards.lock() else { return };
        let Ok(mut measured) = shared.measured.lock() else { return };
        let mut best: Vec<&Board> = boards.values().collect();
        best.sort_by(|a, b| rank(a, b));
        best.truncate(LEARN_AMONG);
        let mut picked = Vec::new();
        while picked.len() < count {
            let new = |b: &Board| field_spots(&b.state).iter().filter(|(k, _)| !measured.contains(k)).map(|&(_, w)| w).sum::<u32>();
            let Some((i, _)) =
                best.iter().enumerate().map(|(i, b)| (i, new(b))).filter(|&(_, n)| n > 0).max_by_key(|&(i, n)| (n, std::cmp::Reverse(i)))
            else {
                break;
            };
            let b = best.remove(i);
            measured.extend(field_spots(&b.state).into_iter().map(|(k, _)| k));
            picked.push(b.clone());
        }
        picked
    };
    let config = shared.config;
    let mut prober = Prober::new(shared.engine, &config.deck, &config.hand, config.seed, kit);
    let until = shared.start + config.time_limit.max(shared.start.elapsed()) + LEARN_GRACE;
    prober.until(until);
    let mut raised = false;
    // The longest measure of the round so far: a board that would not end
    // before `until` is not started (cut short, it would teach nothing).
    let mut longest = Duration::ZERO;
    for board in &picked {
        if shared.cancel.load(Ordering::Relaxed) || Instant::now() + longest > until {
            break;
        }
        let started = Instant::now();
        let measured = prober.measure_alone(&board.line);
        longest = longest.max(started.elapsed());
        // Cut short: what it learnt is not the whole board's.
        if prober.out_of_time() {
            break;
        }
        let Ok(measure) = measured else { continue };
        let me = &board.state.players[0];
        for i in measure.interruptions.iter().filter(|i| i.alone > 0.0) {
            let code = i.responder.code;
            let spot = match i.location {
                location::MZONE => {
                    let found = me.monsters.iter().enumerate().find_map(|(z, c)| c.as_ref().filter(|c| c.code == code).map(|c| (z, c)));
                    match found.and_then(|(z, c)| Spot::of_monster(z, c)) {
                        Some(spot) => spot,
                        None => continue,
                    }
                }
                location::SZONE => match me.spells.iter().flatten().find(|c| c.code == code) {
                    Some(c) => Spot::of_spell(c),
                    None => continue,
                },
                _ => continue,
            };
            // Seen with an empty hand: what the card does then.
            let spot = spot.without_hand().filter(|_| me.hand.is_empty()).unwrap_or(spot);
            if let Ok(mut values) = shared.values.write() {
                raised |= values.raise(shared.engine, code, spot, i.alone);
            }
        }
    }
    if raised {
        let (Ok(values), Ok(mut boards)) = (shared.values.read(), shared.boards.lock()) else { return };
        for b in boards.values_mut() {
            b.quick_score = quick_score(shared.engine, &b.state, &config.hand_traps, &values, config.opponent.as_ref());
        }
    }
}

/// Boards up to which the phases after the search take their configured
/// number of boards ([`SearchConfig::confirm_top`], `zone_top`).
pub const PHASE_BOARDS: usize = 2000;

/// The boards a phase after the search takes, out of `found`: `top` up to
/// [`PHASE_BOARDS`] boards found, the same share past them. A fixed number
/// would be a smaller share the longer the search: a board that a short
/// search plays to the end would be left out by a long one.
fn scaled(top: usize, found: usize) -> usize {
    top.max(found.saturating_mul(top) / PHASE_BOARDS)
}

/// Plays to the end of the turn the best provisional boards (and those
/// that reach the goal), so their End Phase effects count.
fn confirm_best(shared: &Shared) {
    let mut provisional: Vec<Board> = shared.boards.lock().map(|b| b.values().filter(|b| !b.exact).cloned().collect()).unwrap_or_default();
    provisional.sort_by(rank);
    let goal = &shared.config.goal;
    let top = scaled(shared.config.confirm_top, provisional.len());
    let picked: Vec<Edge> = provisional
        .iter()
        .enumerate()
        .filter(|(i, b)| *i < top || goal.reached(b.goal_hits))
        .filter_map(|(_, b)| {
            let (prefix, answer) = b.line.split_last()?;
            Some(Edge {
                prefix: Arc::new(prefix),
                answer: Some(answer),
                priority: 0.0,
                order: 0,
                confirms: Some(b.key),
                value: b.quick_score,
                deviation: 0.0,
            })
        })
        .collect();
    if picked.is_empty() {
        return;
    }
    shared.confirming.store(true, Ordering::Relaxed);
    let next = AtomicU64::new(0);
    std::thread::scope(|s| {
        for _ in 0..shared.config.threads.max(1).min(picked.len()) {
            s.spawn(|| {
                // From the copies the search made.
                let mut copies = shared.take_copies();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed) as usize;
                    let Some(edge) = picked.get(i) else { break };
                    // Past the deadline the board stays provisional (not exact).
                    if shared.past_deadline() {
                        break;
                    }
                    // The End Phase may offer choices: their alternatives join the queue.
                    if work(shared, edge.clone(), copies.as_mut()).is_err() {
                        shared.interrupted.fetch_add(1, Ordering::Relaxed);
                    }
                }
                shared.give_back(copies);
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
    best.truncate(scaled(shared.config.zone_top, best.len()));
    let next = AtomicU64::new(0);
    std::thread::scope(|s| {
        for _ in 0..shared.config.threads.max(1).min(best.len()) {
            s.spawn(|| {
                let mut copies = shared.take_copies();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed) as usize;
                    let Some(board) = best.get(i) else { break };
                    if shared.past_deadline() {
                        break;
                    }
                    let _ = zone_variants_of(shared, &board.line, copies.as_mut());
                }
                shared.give_back(copies);
            });
        }
    });
}

fn zone_variants_of(shared: &Shared, line: &Line, mut copies: Option<&mut Copies>) -> Result<(), Error> {
    if shared.past_deadline() {
        return Ok(());
    }
    let config = shared.config;
    // The zone choices of the line that had other kinds of zones, with a
    // copy of the duel there when it is in an arena: each other zone is
    // played from it, not from the start.
    let mut spots = Vec::new();
    let mut own: Duel;
    let in_arena = copies.is_some();
    let duel = match copies.as_deref_mut() {
        Some(c) => {
            c.duel.restore(&c.store.start)?;
            &mut c.duel
        }
        None => {
            own = new_duel(shared)?;
            &mut own
        }
    };
    for (i, answer) in line.answers.iter().enumerate() {
        let Step::Decision { decision, .. } = duel.advance()? else { break };
        if let Decision::SelectPlace { player: 0, count: 1, .. } = decision {
            if let Some(k) = line.choice_at.iter().position(|&at| at == i) {
                let kinds = ygo_engine::auto::distinct_zones(&duel.state(), 0, duel.actions(&decision));
                let others: Vec<Action> = kinds.into_iter().filter(|a| a.response != *answer).collect();
                if !others.is_empty() {
                    let here = in_arena.then(|| duel.save().ok()).flatten();
                    spots.push((i, k, others, here));
                }
            }
        }
        duel.respond(answer);
    }
    if std::env::var_os("DUELFORGE_DEBUG_SEARCH").is_some() {
        eprintln!("zone variants: {} spots in a line of {} choices", spots.len(), line.choices.len());
    }
    for (i, k, others, here) in spots {
        for other in others {
            if shared.past_deadline() {
                return Ok(());
            }
            // From the copy at the zone choice (its decision already read),
            // or the whole line from the start.
            let (duel, from) = match (copies.as_deref_mut(), &here) {
                (Some(c), Some(here)) => {
                    c.duel.restore(here)?;
                    c.duel.respond(&other.response);
                    (&mut c.duel, i + 1)
                }
                _ => {
                    own = new_duel(shared)?;
                    (&mut own, 0)
                }
            };
            let mut played = true;
            for (j, answer) in line.answers.iter().enumerate().skip(from) {
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
            if keeps_hand(shared.engine, config, &state) {
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
    let Frontier { edges, mut seen, boards, next_order, nodes, replays, duplicates, interrupted, elapsed, started, values, measured } =
        frontier;
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
        deadline: OnceLock::new(),
        values: RwLock::new(values.unwrap_or_else(|| (*config.values).clone())),
        measured: Mutex::new(measured),
        next_learning: AtomicU64::new(nodes + LEARN_EVERY),
        coverage: config.coverage.then(Mutex::default),
        own_negators: RwLock::default(),
        idle_hand_traps: RwLock::default(),
        useful_hand_traps: RwLock::default(),
        copies: Mutex::default(),
        idle_strikes: Mutex::default(),
        stats: std::env::var_os("DUELFORGE_SEARCH_STATS").map(|_| stats::Stats::default()),
        checkpoints: Mutex::default(),
    };
    if !started {
        shared.push(vec![NewEdge {
            prefix: Arc::new(Line::default()),
            answer: None,
            priority: 0.0,
            confirms: None,
            value: 0.0,
            deviation: 0.0,
        }]);
    }
    shared.check_limits();
    run_workers(&shared, config.threads, &mut progress);
    let mut stop = shared.stop.lock().ok().and_then(|s| *s).unwrap_or(Stop::Exhausted);
    if stop != Stop::Cancelled {
        // DUELFORGE_DEBUG_PHASES: when each phase after the search ends,
        // against the time limit (the slow decks go past it there).
        let phases = std::env::var_os("DUELFORGE_DEBUG_PHASES").is_some();
        let phase = |name: &str| {
            if phases {
                eprintln!(
                    "phase {name} done at {:.1} s (limit {:.1} s)",
                    shared.start.elapsed().as_secs_f64(),
                    config.time_limit.as_secs_f64()
                );
            }
        };
        phase("search");
        // Slow decks (quick Synchro Summons that check every combination of
        // materials at each chain window) can take minutes more here: the
        // phases after the search stop LEARN_GRACE after the time limit
        // (or after the search, if it ended later).
        let _ = shared.deadline.set(shared.start + config.time_limit.max(shared.start.elapsed()) + LEARN_GRACE);
        // What the last boards found teach, before the best ones are picked.
        learn_round(&shared, LEARN_LAST);
        phase("learning");
        confirm_best(&shared);
        phase("confirm_best");
        zone_variants(&shared);
        phase("zone_variants");
    }
    let final_progress = shared.progress();
    progress(&final_progress);
    if let (Some(stats), Ok(boards)) = (&shared.stats, shared.boards.lock()) {
        let mut best: Vec<&Board> = boards.values().collect();
        best.sort_by(|a, b| rank(a, b));
        stats.report(best.first().copied(), best.first().map_or(0.0, |b| estimate(b)), shared.start.elapsed());
    }
    let boards: HashMap<u64, Board> = shared.boards.into_inner().unwrap_or_else(|e| e.into_inner());
    let queue = shared.queue.into_inner().unwrap_or_else(|e| e.into_inner());
    // Playing the End Phases above may have queued new choices.
    if stop == Stop::Exhausted && !queue.edges.is_empty() {
        stop = Stop::TimeLimit;
    }
    let mut sorted: Vec<Board> = boards.values().cloned().collect();
    sorted.sort_by(rank);
    let interrupted = shared.interrupted.load(Ordering::Relaxed);
    let coverage = shared.coverage.map(|c| c.into_inner().unwrap_or_else(|e| e.into_inner()));
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
        values: shared.values.into_inner().ok(),
        measured: shared.measured.into_inner().unwrap_or_else(|e| e.into_inner()),
    };
    let checkpoints = shared.checkpoints.into_inner().unwrap_or_else(|e| e.into_inner());
    SearchResult { boards: sorted, progress: final_progress, stop, interrupted, frontier, coverage, checkpoints }
}

/// Where the work of a search goes (`DUELFORGE_SEARCH_STATS`, plan A2 of
/// `docs/plans/ricerca-piano-critico.md`): how each dive ends and what it
/// cost, the nodes reached again by another order of the same choices, the
/// answers that change nothing shown, and where the right answer was in the
/// order tried at each step of the best line. Printed as one line of JSON
/// on stderr at the end of the search; the search does not change.
mod stats {
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::sync::Mutex;
    use std::time::Duration;

    use ygo_engine::{Action, ActionKind};

    use super::{Board, Dive, Edge, Line};

    /// Why a dive ended.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum End {
        /// The turn ended: a board was read.
        Board,
        /// A node already explored, reached by another order of the same
        /// choices, at a free moment.
        OtherOrder,
        /// The same, inside a selection with cards already picked.
        OtherOrderPicks,
        /// The same, at another step of something being done.
        OtherOrderInside,
        /// A node already explored, reached by other choices.
        Duplicate,
        /// The same situation again since the last free moment.
        Loop,
        /// Every answer leads where another line already goes.
        Returns,
        /// One of your effects negated another, or a handtrap did nothing.
        Negated,
        /// A handtrap known to do nothing here.
        IdleHandTrap,
        /// Nothing to answer.
        NoAction,
        /// The board was already played to its end.
        Confirmed,
        /// The search stopped.
        Halted,
        /// The engine failed.
        Error,
    }

    #[derive(Default)]
    struct Dives {
        count: u64,
        nodes: u64,
        replay: Duration,
        time: Duration,
    }

    struct DiveRecord {
        end: End,
        nodes: u64,
        replay: Duration,
        time: Duration,
        best: Option<f64>,
        replayed: u64,
        after_free: u64,
    }

    #[derive(Default)]
    pub struct Stats {
        dives: Mutex<Vec<DiveRecord>>,
        /// For each node, the choices that first reached it (sorted).
        first: Mutex<HashMap<u64, u64>>,
        /// Position of each answer in the order tried, by the answers up to it.
        order: Mutex<HashMap<u64, u32>>,
        /// Answers after which nothing shown changed, by kind.
        no_change: Mutex<HashMap<String, u64>>,
        learning: Mutex<Duration>,
        /// Answers cut for what another line taught, by why.
        cut: Mutex<[u64; 3]>,
        /// Answers taken from the queue, by node: their place in the order
        /// tried and their priority.
        popped: Mutex<HashMap<u64, Vec<(u32, f64)>>>,
    }

    /// Why an answer was cut: a handtrap learned to do nothing at your idle
    /// moment, one of your effects learned to negate your own chain, the
    /// other targets of an answer that changed nothing shown.
    #[derive(Debug, Clone, Copy)]
    pub enum Cut {
        IdleHandTrap,
        OwnNegator,
        Hidden,
    }

    fn choices_key(line: &Line) -> u64 {
        let mut sorted: Vec<&String> = line.choices.iter().collect();
        sorted.sort();
        let mut h = DefaultHasher::new();
        sorted.hash(&mut h);
        h.finish()
    }

    fn answers_key(answers: &[Vec<u8>], last: &[u8]) -> u64 {
        let mut h = DefaultHasher::new();
        answers.hash(&mut h);
        last.hash(&mut h);
        h.finish()
    }

    impl Stats {
        pub(super) fn dive(&self, dive: &Dive, time: Duration) {
            let best = dive.boards.iter().map(|b| b.1).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))));
            if let Ok(mut d) = self.dives.lock() {
                d.push(DiveRecord {
                    end: dive.end,
                    nodes: dive.nodes,
                    replay: dive.replay,
                    time,
                    best,
                    replayed: dive.replayed,
                    after_free: dive.after_free,
                });
            }
        }

        pub(super) fn first(&self, key: u64, line: &Line) {
            if let Ok(mut f) = self.first.lock() {
                f.insert(key, choices_key(line));
            }
        }

        /// Whether the node `key` was first reached by the same choices as `line`.
        pub(super) fn same_choices(&self, key: u64, line: &Line) -> bool {
            self.first.lock().is_ok_and(|f| f.get(&key) == Some(&choices_key(line)))
        }

        pub(super) fn order(&self, line: &Line, actions: &[Action]) {
            if let Ok(mut o) = self.order.lock() {
                for (i, a) in actions.iter().enumerate() {
                    o.entry(answers_key(&line.answers, &a.response)).or_insert(i as u32);
                }
            }
        }

        /// An answer taken from the queue: it is out of order when a sibling
        /// the dive tried after it, at the same priority, came out before.
        pub(super) fn popped(&self, edge: &Edge) {
            let Some((answer, _)) = &edge.answer else { return };
            let Some(rank) = self.order.lock().ok().and_then(|o| o.get(&answers_key(&edge.prefix.answers, answer)).copied()) else {
                return;
            };
            let mut h = DefaultHasher::new();
            edge.prefix.answers.hash(&mut h);
            if let Ok(mut p) = self.popped.lock() {
                p.entry(h.finish()).or_default().push((rank, edge.priority));
            }
        }

        pub(super) fn no_change(&self, kind: ActionKind, _shown: u64) {
            if let Ok(mut n) = self.no_change.lock() {
                *n.entry(format!("{kind:?}")).or_default() += 1;
            }
        }

        pub(super) fn learned(&self, time: Duration) {
            if let Ok(mut l) = self.learning.lock() {
                *l += time;
            }
        }

        pub(super) fn cut(&self, why: Cut, n: u64) {
            if let (true, Ok(mut s)) = (n > 0, self.cut.lock()) {
                s[why as usize] += n;
            }
        }

        /// Prints the stats: `best` is the best board and `top` its estimate.
        pub(super) fn report(&self, best: Option<&Board>, top: f64, elapsed: Duration) {
            let Ok(dives) = self.dives.lock() else { return };
            let mut by_end: HashMap<End, Dives> = HashMap::new();
            // Dives by the best board they read: none, under 90% of the best
            // board of the search, or near it.
            let mut by_board: [Dives; 3] = Default::default();
            for d in dives.iter() {
                for e in [
                    by_end.entry(d.end).or_default(),
                    &mut by_board[match d.best {
                        None => 0,
                        Some(v) if top > 0.0 && v < 0.9 * top => 1,
                        Some(_) => 2,
                    }],
                ] {
                    e.count += 1;
                    e.nodes += d.nodes;
                    e.replay += d.replay;
                    e.time += d.time;
                }
            }
            let group = |d: &Dives| {
                format!(
                    "{{\"dives\":{},\"nodes\":{},\"replay_s\":{:.3},\"time_s\":{:.3}}}",
                    d.count,
                    d.nodes,
                    d.replay.as_secs_f64(),
                    d.time.as_secs_f64()
                )
            };
            let mut ends: Vec<String> = by_end.iter().map(|(e, d)| format!("\"{e:?}\":{}", group(d))).collect();
            ends.sort();
            // Where each answer of the best line was in the order tried
            // (none: an answer given automatically, or by a later run).
            let ranks: Vec<String> = match (best, self.order.lock()) {
                (Some(b), Ok(order)) => b
                    .line
                    .choice_at
                    .iter()
                    .map(|&at| order.get(&answers_key(&b.line.answers[..at], &b.line.answers[at])).map_or("null".into(), |r| r.to_string()))
                    .collect(),
                _ => Vec::new(),
            };
            let mut unchanged: Vec<String> =
                self.no_change.lock().map(|n| n.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect()).unwrap_or_default();
            unchanged.sort();
            let replayed: u64 = dives.iter().map(|d| d.replayed).sum();
            let after_free: u64 = dives.iter().map(|d| d.after_free).sum();
            let [idle, negators, hidden] = self.cut.lock().map_or([0; 3], |s| *s);
            // Siblings taken from the queue: how many, how many after one the
            // dive tried later at the same priority, and the place of the first
            // one taken at each node (1 is the dive's second choice).
            let (mut pops, mut inverted, mut first) = (0u64, 0u64, [0u64; 4]);
            if let Ok(p) = self.popped.lock() {
                for node in p.values() {
                    pops += node.len() as u64;
                    first[(node[0].0 as usize).clamp(1, 4) - 1] += 1;
                    for (i, &(rank, priority)) in node.iter().enumerate() {
                        inverted += node[..i].iter().any(|&(r, q)| r > rank && q == priority) as u64;
                    }
                }
            }
            eprintln!(
                "search-stats {{\"siblings\":{{\"popped\":{pops},\"inverted\":{inverted},\"first_rank\":[{},{},{},{}]}},\"replayed\":{replayed},\"after_free\":{after_free},\"cut\":{{\"idle_hand_traps\":{idle},\"own_negators\":{negators},\"hidden\":{hidden}}},\"elapsed_s\":{:.3},\"learning_s\":{:.3},\"best\":{top:.4},\"ends\":{{{}}},\"boards\":{{\"none\":{},\"weak\":{},\"near\":{}}},\"no_change\":{{{}}},\"best_line_ranks\":[{}]}}",
                first[0],
                first[1],
                first[2],
                first[3],
                elapsed.as_secs_f64(),
                self.learning.lock().map_or(0.0, |l| l.as_secs_f64()),
                ends.join(","),
                group(&by_board[0]),
                group(&by_board[1]),
                group(&by_board[2]),
                unchanged.join(","),
                ranks.join(",")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phases_after_the_search_grow_with_the_boards() {
        assert_eq!(scaled(40, 300), 40);
        assert_eq!(scaled(40, PHASE_BOARDS), 40);
        assert_eq!(scaled(40, 20_000), 400);
        assert_eq!(scaled(20, 20_000), 200);
        assert_eq!(scaled(0, 20_000), 0);
    }

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

    fn card(code: u32, sequence: u32) -> CardRef {
        CardRef { code, controller: 0, location: location::HAND, sequence, position: 0 }
    }

    fn pick(index: i32) -> Vec<u8> {
        [1i32, index].iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn the_same_cards_picked_in_another_order_meet() {
        let (a, b, c) = (card(10, 0), card(20, 1), card(30, 2));
        let select = |picked: Vec<CardRef>| Decision::SelectUnselectCard {
            player: 0,
            finishable: false,
            cancelable: false,
            min: 2,
            max: 2,
            selectable: [a, b, c].into_iter().filter(|x| !picked.contains(x)).collect(),
            unselectable: picked,
            hint: 7,
        };
        let mut ab = Pending::default();
        ab.answered(&select(vec![]), &pick(0));
        ab.answered(&select(vec![a]), &pick(0));
        let mut ba = Pending::default();
        ba.answered(&select(vec![]), &pick(1));
        ba.answered(&select(vec![b]), &pick(0));
        assert_eq!(ab.value(), ba.value());
        let mut ac = Pending::default();
        ac.answered(&select(vec![]), &pick(0));
        ac.answered(&select(vec![a]), &pick(1));
        assert_ne!(ab.value(), ac.value());
    }

    #[test]
    fn sets_are_read_from_idle_answers() {
        let idle = Decision::IdleCmd {
            player: 0,
            summonable: vec![],
            special_summonable: vec![],
            repositionable: vec![],
            monster_settable: vec![card(5, 0)],
            spell_settable: vec![card(7, 1), card(3, 2)],
            activatable: vec![],
            to_battle_phase: false,
            to_end_phase: true,
        };
        let cmd = |kind: i32, i: i32| ((i << 16) | kind).to_le_bytes().to_vec();
        assert_eq!(set_key(&idle, &cmd(3, 0)), Some((0, 5)));
        assert_eq!(set_key(&idle, &cmd(4, 1)), Some((1, 3)));
        assert_eq!(set_key(&idle, &cmd(7, 0)), None);
    }

    #[test]
    fn a_passed_chain_window_keeps_the_set_before_it() {
        let idle = Decision::IdleCmd {
            player: 0,
            summonable: vec![],
            special_summonable: vec![],
            repositionable: vec![],
            monster_settable: vec![],
            spell_settable: vec![card(7, 1)],
            activatable: vec![],
            to_battle_phase: false,
            to_end_phase: true,
        };
        let window = Decision::SelectChain { player: 0, forced: false, triggers: false, chains: vec![] };
        let mut last_set = None;
        after_answer(&mut last_set, &idle, &4i32.to_le_bytes());
        after_answer(&mut last_set, &window, &(-1i32).to_le_bytes());
        assert_eq!(last_set, Some((1, 7)));
        after_answer(&mut last_set, &window, &0i32.to_le_bytes());
        assert_eq!(last_set, None);
    }

    #[test]
    fn cancelling_takes_back_what_was_started_or_picked() {
        let idle = Decision::IdleCmd {
            player: 0,
            summonable: vec![],
            special_summonable: vec![],
            repositionable: vec![],
            monster_settable: vec![],
            spell_settable: vec![],
            activatable: vec![],
            to_battle_phase: false,
            to_end_phase: true,
        };
        let window = Decision::SelectChain { player: 0, forced: false, triggers: false, chains: vec![] };
        let select = Decision::SelectCard { player: 0, cancelable: true, min: 1, max: 1, cards: vec![card(10, 0)], hint: 0 };
        assert!(starts(&idle, ActionKind::Activate));
        assert!(starts(&window, ActionKind::Activate));
        assert!(!starts(&window, ActionKind::Pass));
        assert!(starts(&select, ActionKind::Choose));
        assert!(!starts(&select, ActionKind::Cancel));
    }

    #[test]
    fn a_target_or_a_name_declared_does_not_show() {
        let mut state = DuelState::default();
        state.players[0].monsters = vec![None; 7];
        state.players[0].monsters[0] = Some(ygo_engine::FieldCard { code: 10, position: 1, attack: 1000, ..Default::default() });
        let before = shown_state(&state);
        // Activating an effect marks the card and the turn, whatever its target.
        state.activity.activations.push((10, 160));
        state.players[0].monsters[0].as_mut().unwrap().status |= 0x10;
        assert_eq!(shown_state(&state), before);
        // What the effect does shows.
        state.players[0].monsters[0].as_mut().unwrap().attack = 2000;
        assert_ne!(shown_state(&state), before);
    }
}
