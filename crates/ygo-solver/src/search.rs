//! The search. A node is a real choice the engine asks for; it is reached by
//! replaying its line from the start of the duel, because the engine cannot
//! copy a duel in progress. To make the most of each replay, a worker dives:
//! after reaching a node it keeps playing the first answer of every choice
//! down to the end of the turn, and leaves the other answers in a shared
//! queue for later. A table of the states already seen (by their canonical
//! fingerprint, plus the choice being asked) makes sure that lines reaching
//! the same situation in a different order are explored once; answers that
//! could only lead back to where the search has been (taking a card out of
//! a selection, cancelling what was just started, picking the same cards
//! in another order) are not tried at all, since each try costs a replay.
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
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

use ygo_engine::auto::AutoPolicy;
use ygo_engine::consts::{card_type, location};
use ygo_engine::{
    canonical_fingerprint, fingerprint, italian, Action, ActionKind, CardRef, Decision, DeckList, Duel, DuelState, Engine, Error, Opponent,
    Placement, Step,
};

use crate::probe::{Kit, Prober};
use crate::value::{CardValues, Spot};

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
    /// Provisional boards played to the end of the turn when the search stops.
    pub confirm_top: usize,
    /// Best boards whose zone choices are tried the other ways at the end.
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

    /// The zone the goal wants this card in, if it names one.
    fn zone_of(&self, engine: &Engine, code: u32) -> Option<GoalZone> {
        let db = engine.cards();
        self.on_field.iter().find(|g| g.zone != GoalZone::Any && db.canonical_code(g.code) == db.canonical_code(code)).map(|g| g.zone)
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

/// An edge to queue (see [`Edge`]).
struct NewEdge {
    prefix: Arc<Line>,
    answer: Option<Answer>,
    priority: f64,
    confirms: Option<u64>,
    value: f64,
    discrepancies: u32,
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
    /// Answers of the line, this one included, that are not the one the
    /// dive would have given.
    discrepancies: u32,
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
    /// Playing the best provisional boards to the end: only cancelling stops it.
    confirming: AtomicBool,
    /// The card values the estimate uses, raised by what the search learns.
    values: RwLock<CardValues>,
    /// Cards (and kinds of zones) on the boards measured to learn from.
    measured: Mutex<HashSet<u64>>,
    /// Nodes after which the next boards are measured.
    next_learning: AtomicU64,
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
                    discrepancies: e.discrepancies,
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
            // Its own End Phase played: the same line, with the End Phase's answers.
            Some(old) if !old.exact && board.exact && confirms == Some(board.key) => true,
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
            quick_score: self.values.read().map_or(0.0, |v| quick_score(self.engine, &state, &self.config.hand_traps, &v)),
            goal_hits: self.config.goal.hits(self.engine, &state),
            exact,
            line,
            state,
            found_after: self.elapsed(),
        }
    }
}

/// What the search steers by: the quick estimate, and the cards of the goal.
fn estimate(board: &Board) -> f64 {
    board.quick_score + 3.0 * board.goal_hits as f64
}

/// Best first: the goal, then a higher estimate, then fewer cards from the
/// hand, then a shorter line; ties by the board's key, so the same search
/// keeps the same boards.
fn rank(a: &Board, b: &Board) -> CmpOrdering {
    b.goal_hits
        .cmp(&a.goal_hits)
        .then(b.quick_score.total_cmp(&a.quick_score))
        .then(a.cards_used.cmp(&b.cards_used))
        .then(a.line.choices.len().cmp(&b.line.choices.len()))
        .then(a.key.cmp(&b.key))
}

/// Key of a node: the state and the choice being asked (two steps of the
/// same selection share the state but not the choice). Inside something
/// being done (an activation, a summon, an effect resolving), the answers
/// since your last free moment count too: the state does not say yet what
/// is being done, and choosing Aleister the Invoker as material for
/// Salamangreat Almiraj or for Artemis is the same choice of the same state.
fn node_key(state: &DuelState, decision: &Decision, pending: &Pending) -> u64 {
    let mut h = DefaultHasher::new();
    canonical_fingerprint(state).hash(&mut h);
    decision.hash(&mut h);
    if !is_free(decision) {
        pending.0.hash(&mut h);
    }
    h.finish()
}

/// A moment where you choose what to do next (not a step of something
/// already started).
fn is_free(decision: &Decision) -> bool {
    decision.player() == Some(0) && matches!(decision, Decision::IdleCmd { .. } | Decision::BattleCmd { .. } | Decision::SelectChain { .. })
}

/// The answers given since your last free moment, hashed.
#[derive(Clone, Copy, Default)]
struct Pending(u64);

impl Pending {
    fn answered(&mut self, decision: &Decision, answer: &[u8]) {
        let mut h = DefaultHasher::new();
        if !is_free(decision) {
            self.0.hash(&mut h);
        }
        answer.hash(&mut h);
        self.0 = h.finish();
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
pub fn quick_score(engine: &Engine, state: &DuelState, hand_traps: &[u32], values: &CardValues) -> f64 {
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
    // A handtrap counts what it stops from the hand with this field (some
    // only work while you control no cards), when it was measured.
    score += me
        .hand
        .iter()
        .map(|&c| match values.in_hand(engine, c, state) {
            Some(v) => (VALUE_WEIGHT * v).max(0.3),
            None if hand_trap(c) => 1.5,
            None => 0.3,
        })
        .sum::<f64>();
    score + VALUE_WEIGHT * values.of_field(engine, state)
}

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
/// Answers that pick or play a card of the goal come before everything.
fn dive_order(actions: &mut [Action], responding: bool, wanted: impl Fn(&Action) -> bool) {
    let rank = |a: &Action| match a.kind {
        _ if wanted(a) => -1,
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

const PHASE_END: u16 = 0x200;

fn new_duel(shared: &Shared) -> Result<Duel, Error> {
    let config = shared.config;
    Duel::goldfish_against_placed(shared.engine, &config.deck, &config.hand, config.seed, config.opponent.as_ref(), &config.placed)
}

/// Drops the answers that lead back where the search has been or goes
/// anyway, which would cost a replay of the whole line each:
/// - taking a card out of a selection (picking the others directly gets
///   the same sets);
/// - cancelling what was just started at a free moment, before anything
///   happened (it is the free moment again; a cancel in the middle of an
///   effect, like choosing no card, stays);
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
    ("selection", fingerprint(state), pending.0, hint, min, max).hash(&mut base);
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

/// Priority taken off an answer for each answer of its line that is not
/// the dive's: the lines near the best ones are tried first, the ones that
/// differ in many choices later.
const DISCREPANCY: f64 = 0.6;

/// Plays one edge: replays its line, gives its answer, then dives. The
/// other answers met on the way are queued when the dive is over, at the
/// estimate of the best board the line reached after them (the boards
/// before an answer do not depend on it), less [`DISCREPANCY`] for each
/// answer of their line that is not the dive's.
fn work(shared: &Shared, edge: Edge) -> Result<(), Error> {
    let mut dive = Dive { left: Vec::new(), boards: Vec::new() };
    let result = play(shared, &edge, &mut dive);
    let discrepancies = edge.discrepancies + 1;
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
            let bonus = if left.wanted { 10.0 } else { 0.0 };
            NewEdge {
                prefix: left.prefix,
                answer: Some(left.answer),
                priority: value - DISCREPANCY * discrepancies as f64 + bonus,
                confirms: None,
                value,
                discrepancies,
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
}

struct LeftAnswer {
    prefix: Arc<Line>,
    answer: Answer,
    depth: usize,
    wanted: bool,
}

fn play(shared: &Shared, edge: &Edge, dive: &mut Dive) -> Result<(), Error> {
    let config = shared.config;
    let mut duel = new_duel(shared)?;
    let mut line = (*edge.prefix).clone();
    let mut pending = Pending::default();
    for answer in &line.answers {
        if let Step::Decision { decision, .. } = duel.advance()? {
            pending.answered(&decision, answer);
        }
        duel.respond(answer);
    }
    if !line.answers.is_empty() || edge.answer.is_some() {
        shared.replays.fetch_add(1, Ordering::Relaxed);
    }
    if let Some((answer, label)) = edge.answer.clone() {
        if let Step::Decision { decision, .. } = duel.advance()? {
            pending.answered(&decision, &answer);
        }
        duel.respond(&answer);
        line.choose(answer, label);
    }
    // The card last taken from the Extra Deck: the one a Monster Zone is
    // asked for next (only those go to the Extra Monster Zone).
    let mut placing: Option<u32> = None;
    // Something was started at a free moment (an idle or chain window) and
    // nothing has happened since: cancelling only goes back to that moment.
    let mut undoable = false;
    // The situations met since your last free moment: meeting one again
    // means going round in circles (choosing a Fusion Monster whose
    // materials cannot be chosen, cancelling, choosing it again).
    let mut since_free: HashSet<u64> = HashSet::new();
    loop {
        if shared.halted() {
            return Ok(());
        }
        let (step, mut actions) = duel.advance_auto_seen(&config.policy, &mut line.answers, |d, a| pending.answered(d, a))?;
        let (turn, _, phase) = duel.turn_info();
        if let Step::Decision { events, .. } = &step {
            undoable &= events.is_empty();
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
                return Ok(());
            }
        };
        let state = duel.state();
        shared.nodes.fetch_add(1, Ordering::Relaxed);
        let free = is_free(&decision);
        if free {
            since_free.clear();
        } else if !since_free.insert(node_key(&state, &decision, &Pending::default())) {
            shared.duplicates.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        if !shared.first_visit(node_key(&state, &decision, &pending)) {
            shared.duplicates.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        if line.choices.len() >= config.max_choices {
            // Too long: end the turn, or close what is pending as fast as possible.
            dive_order(&mut actions, false, |_| false);
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
        if actions.len() > 1 {
            actions = without_returns(shared, &state, &decision, &pending, actions, undoable);
            if actions.is_empty() {
                // Every pick leads where another line already goes.
                shared.duplicates.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        }
        let wanted = |a: &Action| config.goal.involves(shared.engine, a);
        // Quick effects in your own End Phase rarely help a combo (Hyperinvoked
        // Aeon's Attribute change, a negation of your own effect): not
        // activating them comes first there too, like when responding.
        let window = matches!(decision, Decision::SelectChain { triggers: false, .. });
        let responding = window && (duel.open_chain() > 0 || phase == PHASE_END);
        dive_order(&mut actions, responding, wanted);
        // A goal card wanted in a kind of zone goes there; while one wants
        // an Extra Monster Zone, other monsters leave it free when they can
        // (holding back the other Extra Deck summons instead would also hold
        // back the Link monsters used as material on the way, like Artemis
        // for Invoked Mechaba).
        if let (Decision::SelectPlace { .. }, Some(code)) = (&decision, placing) {
            let first = match config.goal.zone_of(shared.engine, code) {
                Some(zone) => Some(zone == GoalZone::Extra),
                None if config.goal.on_field.iter().any(|g| g.zone == GoalZone::Extra) => Some(false),
                None => None,
            };
            if let Some(first) = first {
                let extra = |a: &Action| matches!(*a.response.as_slice(), [0, location::MZONE, seq] if seq >= 5);
                actions.sort_by_key(|a| extra(a) != first);
            }
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
                        discrepancies: edge.discrepancies,
                    }]);
                }
            }
        }
        // Lines that pick a card of the goal are tried before the others.
        dive.left.extend(actions[1..].iter().map(|a| LeftAnswer {
            prefix: prefix.clone(),
            answer: (a.response.clone(), italian::choice(a)),
            depth,
            wanted: wanted(a),
        }));
        let first = &actions[0];
        if let Some(c) = first.card.filter(|c| c.location == location::EXTRA) {
            placing = Some(c.code);
        }
        undoable = (free && first.kind != ActionKind::Pass) || (undoable && !free);
        pending.answered(&decision, &first.response);
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
                    learn(shared);
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

/// Nodes between two rounds of learning.
const LEARN_EVERY: u64 = 1500;
/// Boards measured in a round.
const LEARN_BOARDS: usize = 2;
/// Best boards a round looks at.
const LEARN_AMONG: usize = 200;
/// Boards measured in the round at the end of the search.
const LEARN_LAST: usize = 4;

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
        Some((key(c.code, if z >= 5 { Spot::Extra } else { Spot::Main }), weight))
    });
    let spells = me.spells.iter().flatten().map(|c| (key(c.code, if c.is_face_up() { Spot::FaceUp } else { Spot::Set }), 1));
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
    let mut raised = false;
    for board in &picked {
        let Ok(measure) = prober.measure(&board.line) else { continue };
        let me = &board.state.players[0];
        for i in measure.interruptions.iter().filter(|i| i.alone > 0.0) {
            let code = i.responder.code;
            let spot = match i.location {
                location::MZONE => match me.monsters.iter().position(|c| c.as_ref().is_some_and(|c| c.code == code)) {
                    Some(z) if z >= 5 => Spot::Extra,
                    Some(_) => Spot::Main,
                    None => continue,
                },
                location::SZONE => match me.spells.iter().flatten().find(|c| c.code == code) {
                    Some(c) if c.is_face_up() => Spot::FaceUp,
                    Some(_) => Spot::Set,
                    None => continue,
                },
                _ => continue,
            };
            if let Ok(mut values) = shared.values.write() {
                raised |= values.raise(shared.engine, code, spot, i.alone);
            }
        }
    }
    if raised {
        let (Ok(values), Ok(mut boards)) = (shared.values.read(), shared.boards.lock()) else { return };
        for b in boards.values_mut() {
            b.quick_score = quick_score(shared.engine, &b.state, &config.hand_traps, &values);
        }
    }
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
            Some(Edge {
                prefix: Arc::new(prefix),
                answer: Some(answer),
                priority: 0.0,
                order: 0,
                confirms: Some(b.key),
                value: b.quick_score,
                discrepancies: 0,
            })
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
    let mut duel = new_duel(shared)?;
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
            let mut duel = new_duel(shared)?;
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
        values: RwLock::new(values.unwrap_or_else(|| (*config.values).clone())),
        measured: Mutex::new(measured),
        next_learning: AtomicU64::new(nodes + LEARN_EVERY),
    };
    if !started {
        shared.push(vec![NewEdge {
            prefix: Arc::new(Line::default()),
            answer: None,
            priority: 0.0,
            confirms: None,
            value: 0.0,
            discrepancies: 0,
        }]);
    }
    shared.check_limits();
    run_workers(&shared, config.threads, &mut progress);
    let mut stop = shared.stop.lock().ok().and_then(|s| *s).unwrap_or(Stop::Exhausted);
    if stop != Stop::Cancelled {
        // What the last boards found teach, before the best ones are picked.
        learn_round(&shared, LEARN_LAST);
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
        values: shared.values.into_inner().ok(),
        measured: shared.measured.into_inner().unwrap_or_else(|e| e.into_inner()),
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
