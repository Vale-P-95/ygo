//! The search. A node is a real choice the engine asks for; it is reached by
//! replaying its line from the start of the duel, because the engine cannot
//! copy a duel in progress. To make the most of each replay, a worker dives:
//! after reaching a node it keeps playing the first answer of every choice
//! down to the end of the turn, and leaves the other answers in a shared
//! queue for later. A table of the states already seen (by their canonical
//! fingerprint, plus the choice being asked) makes sure that lines reaching
//! the same situation in a different order are explored once.

use std::cmp::Ordering as CmpOrdering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use ygo_engine::auto::AutoPolicy;
use ygo_engine::consts::card_type;
use ygo_engine::{canonical_fingerprint, italian, Action, ActionKind, Decision, DeckList, Duel, DuelState, Engine, Error, Step};

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub deck: DeckList,
    pub hand: Vec<u32>,
    /// Order of the Deck and the engine's random numbers.
    pub seed: u64,
    pub time_limit: Duration,
    /// Nodes (real choices reached) at most.
    pub max_nodes: u64,
    pub threads: usize,
    /// Real choices per line at most.
    pub max_choices: usize,
    /// Cards that must still be in the hand at the end (e.g. a handtrap).
    pub keep_in_hand: Vec<u32>,
    /// Cards that must be on the field at the end.
    pub require_on_field: Vec<u32>,
    pub policy: AutoPolicy,
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
            require_on_field: Vec::new(),
            policy: AutoPolicy::default(),
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
}

/// A board reached at the end of the turn, with the best line found to it.
#[derive(Debug, Clone)]
pub struct Board {
    /// Identity of the board (same cards in the same places).
    pub key: u64,
    pub line: Line,
    /// The duel when the opponent's turn begins.
    pub state: DuelState,
    /// Cards of the starting hand no longer in the hand.
    pub cards_used: usize,
    /// A quick estimate until the real score is computed (Phase 5C).
    pub quick_score: f64,
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
}

#[derive(Debug, Clone, Default)]
pub struct Progress {
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
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Best first (by the quick estimate).
    pub boards: Vec<Board>,
    pub progress: Progress,
    pub stop: Stop,
    /// Lines that stopped on a choice without answers or a rejected answer
    /// (should stay 0: see the engine's randomized sweep).
    pub interrupted: u64,
}

/// An answer to try and its label.
type Answer = (Vec<u8>, String);

/// A choice left for later: the line to its node and the answer to try.
struct Edge {
    prefix: Arc<Line>,
    answer: Option<Answer>,
    priority: f64,
    order: u64,
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

struct Shared<'a> {
    engine: &'a Engine,
    config: &'a SearchConfig,
    start: Instant,
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

    fn set_stop(&self, why: Stop) {
        if let Ok(mut s) = self.stop.lock() {
            s.get_or_insert(why);
        }
        self.wake.notify_all();
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

    fn push(&self, edges: Vec<(Arc<Line>, Option<Answer>, f64)>) {
        if edges.is_empty() {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            for (prefix, answer, priority) in edges {
                let order = q.next_order;
                q.next_order += 1;
                q.edges.push(Edge { prefix, answer, priority, order });
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

    fn record(&self, board: Board) {
        let Ok(mut boards) = self.boards.lock() else { return };
        match boards.get(&board.key) {
            Some(old) if !board.better_line_than(old) => {}
            _ => {
                boards.insert(board.key, board);
            }
        }
    }

    fn progress(&self) -> Progress {
        let (boards, best) =
            self.boards.lock().map(|b| (b.len(), b.values().map(|b| b.quick_score).fold(0.0, f64::max))).unwrap_or_default();
        Progress {
            elapsed: self.start.elapsed(),
            nodes: self.nodes.load(Ordering::Relaxed),
            replays: self.replays.load(Ordering::Relaxed),
            duplicates: self.duplicates.load(Ordering::Relaxed),
            queued: self.queue.lock().map(|q| q.edges.len()).unwrap_or(0),
            boards,
            best_quick_score: best,
        }
    }
}

/// Key of a node: the state and the choice being asked (two steps of the
/// same selection share the state but not the choice).
fn node_key(state: &DuelState, decision: &Decision) -> u64 {
    let mut h = DefaultHasher::new();
    canonical_fingerprint(state).hash(&mut h);
    decision.hash(&mut h);
    h.finish()
}

/// Identity of an end board as a player sees it: the same cards on the
/// field (face-up or not, with their materials and counters), in the hand,
/// in the Graveyard, banished and in the Extra Deck, whatever the zones.
fn board_key(state: &DuelState) -> u64 {
    let mut h = DefaultHasher::new();
    for p in &state.players {
        let mut field: Vec<_> = p
            .monsters
            .iter()
            .chain(&p.spells)
            .flatten()
            .map(|c| {
                let mut overlay = c.overlay.clone();
                overlay.sort_unstable();
                (c.code, c.is_face_up(), c.position & 0xc != 0, overlay, c.counters.clone())
            })
            .collect();
        field.sort();
        field.hash(&mut h);
        for pile in [&p.hand, &p.grave] {
            let mut v = pile.clone();
            v.sort_unstable();
            v.hash(&mut h);
        }
        for pile in [&p.banished, &p.extra] {
            let mut v = pile.clone();
            v.sort_unstable();
            v.hash(&mut h);
        }
        p.lp.hash(&mut h);
    }
    h.finish()
}

/// A rough value of a state for ordering the search (Phase 5C replaces it
/// at the end with the real score): monsters on the field, more for Extra
/// Deck monsters, set cards, cards kept in hand.
pub fn quick_score(engine: &Engine, state: &DuelState) -> f64 {
    let me = &state.players[0];
    let mut score = 0.0;
    for c in me.monsters.iter().flatten() {
        let extra = engine.cards().get(c.code).is_some_and(|d| d.type_ & card_type::EXTRA != 0);
        score += if extra { 2.0 } else { 1.0 };
    }
    for c in me.spells.iter().flatten() {
        score += if c.is_face_up() { 0.5 } else { 1.0 };
    }
    score + 0.3 * me.hand.len() as f64
}

/// Answers that do something come first: the dive tries them before ending the turn.
fn dive_order(actions: &mut [Action]) {
    let rank = |a: &Action| match a.kind {
        ActionKind::EndTurn => 3,
        ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish => 2,
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

fn satisfies(config: &SearchConfig, state: &DuelState) -> bool {
    let me = &state.players[0];
    let mut hand = me.hand.clone();
    for code in &config.keep_in_hand {
        match hand.iter().position(|c| c == code) {
            Some(i) => {
                hand.swap_remove(i);
            }
            None => return false,
        }
    }
    config.require_on_field.iter().all(|code| me.monsters.iter().chain(&me.spells).flatten().any(|c| c.code == *code))
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
    loop {
        if shared.stopped() {
            return Ok(());
        }
        let (step, mut actions) = duel.advance_auto(&config.policy, &mut line.answers)?;
        let (turn, _, _) = duel.turn_info();
        let decision = match step {
            Step::Decision { decision, .. } if decision.player() == Some(0) && turn <= 1 => decision,
            // The opponent's turn (or the end of the duel): the board is done.
            _ => {
                let state = duel.state();
                if satisfies(config, &state) {
                    shared.record(Board {
                        key: board_key(&state),
                        cards_used: cards_used(&config.hand, &state),
                        quick_score: quick_score(shared.engine, &state),
                        line,
                        state,
                        found_after: shared.start.elapsed(),
                    });
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
            dive_order(&mut actions);
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
        dive_order(&mut actions);
        let priority = quick_score(shared.engine, &state) + 0.3 * line.choices.len() as f64;
        let prefix = Arc::new(line.clone());
        let rest: Vec<_> =
            actions[1..].iter().map(|a| (prefix.clone(), Some((a.response.clone(), italian::choice(a))), priority)).collect();
        shared.push(rest);
        let first = &actions[0];
        duel.respond(&first.response);
        line.choose(first.response.clone(), italian::choice(first));
    }
}

/// Runs the search on `config.threads` threads. `progress` is called about
/// four times a second from the calling thread.
pub fn search(engine: &Engine, config: &SearchConfig, cancel: &AtomicBool, mut progress: impl FnMut(&Progress)) -> SearchResult {
    let shared = Shared {
        engine,
        config,
        start: Instant::now(),
        cancel,
        queue: Mutex::new(Queue { edges: BinaryHeap::new(), busy: 0, next_order: 0 }),
        wake: Condvar::new(),
        seen: (0..SEEN_SHARDS).map(|_| Mutex::new(HashSet::new())).collect(),
        boards: Mutex::new(HashMap::new()),
        nodes: AtomicU64::new(0),
        replays: AtomicU64::new(0),
        duplicates: AtomicU64::new(0),
        interrupted: AtomicU64::new(0),
        stop: Mutex::new(None),
    };
    shared.push(vec![(Arc::new(Line::default()), None, 0.0)]);
    std::thread::scope(|s| {
        for _ in 0..config.threads.max(1) {
            let shared = &shared;
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
    let final_progress = shared.progress();
    progress(&final_progress);
    let mut boards: Vec<Board> = shared.boards.into_inner().map(|b| b.into_values().collect()).unwrap_or_default();
    boards.sort_by(|a, b| {
        b.quick_score.total_cmp(&a.quick_score).then(a.cards_used.cmp(&b.cards_used)).then(a.line.choices.len().cmp(&b.line.choices.len()))
    });
    SearchResult {
        boards,
        progress: final_progress,
        stop: shared.stop.into_inner().ok().flatten().unwrap_or(Stop::Exhausted),
        interrupted: shared.interrupted.load(Ordering::Relaxed),
    }
}
