//! The role of each card of a deck, found by playing it: what the card does
//! when it is the only card of the deck in your hand (with cards that do
//! nothing), and what it adds to the best of those hands. Its effect, its
//! archetype and the rest of the deck all count, because the engine plays
//! them: a card that searches its archetype starts a combo only if the
//! deck has something to search.
//!
//! - **Handtrap**: used from the hand during the opponent's turn
//!   ([`crate::hand_traps`]).
//! - **Starter**: alone, it reaches a board (the estimate of the search
//!   rises by at least [`STARTER_GAIN`]).
//! - **Extender**: in the turn of the best starter's line, played up to
//!   its last free moment, the card joins the hand and the search goes on
//!   from there: it makes the board better by at least [`EXTENDER_GAIN`]
//!   than going on without it, and the best line uses it (a short search
//!   can end a little higher by chance). What the starter's line used
//!   stays used: the Normal Summon, effects once per turn. (Searching from
//!   a hand with two real cards would not do: with more to try, a short
//!   search ends lower than with the starter alone.)
//! - **Board breaker**: its text removes or negates the opponent's cards.
//! - **Brick**: a monster that cannot be summoned by itself (it needs
//!   Tributes or another way in) and alone does nothing.
//!
//! Each test is a short search on one thread with a node budget, so the
//! same deck always gets the same roles.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ygo_engine::consts::card_type;
use ygo_engine::{Decision, DeckList, Duel, Engine, Prelude, Step};

use crate::probe::Kit;
use crate::search::{search, Board, SearchConfig};
use crate::value::{CardValues, Spot};

/// What the tests found for a card.
#[derive(Debug, Clone, PartialEq)]
pub struct RoleGuess {
    pub code: u32,
    /// Role keys ("starter", "extender", "handtrap", "breaker", "brick").
    pub roles: Vec<&'static str>,
    /// Why, in Italian, one line per role.
    pub reasons: Vec<String>,
}

/// Rise of the estimate that makes a card a starter (about two monsters
/// that stay on the field, or one that stops the opponent).
pub const STARTER_GAIN: f64 = 3.0;
/// Rise of the estimate a card adds to the best starter to be an extender.
pub const EXTENDER_GAIN: f64 = 1.5;
/// Nodes of each test alone in hand.
const NODES: u64 = 400;
/// Nodes of each test that goes on from the starter's line (more to try there).
const BOARD_NODES: u64 = 800;

/// A card that does nothing: a Ritual Spell whose Ritual Monster is not in
/// the deck (it can still be discarded, like a card you would not play; it
/// is never Set nor used as material).
const BLANK: &str = "Black Luster Ritual";

struct Tester<'a> {
    engine: &'a Engine,
    deck: DeckList,
    blank: u32,
    base: SearchConfig,
}

impl Tester<'_> {
    /// The hand of `cards` and blanks up to five.
    fn hand(&self, cards: &[u32]) -> Vec<u32> {
        let mut hand = cards.to_vec();
        while hand.len() < 5 {
            hand.push(self.blank);
        }
        hand
    }

    /// The best board reached from `cards` and blanks up to five, going on
    /// from `prelude` (played from that hand).
    fn run(&self, cards: &[u32], prelude: &Prelude) -> Option<Board> {
        let mut config = self.base.clone();
        config.deck = self.deck.clone();
        config.hand = self.hand(cards);
        config.prelude = prelude.clone();
        if !prelude.answers.is_empty() {
            config.max_nodes = BOARD_NODES;
        }
        let result = search(self.engine, &config, &AtomicBool::new(false), |_| {});
        result.boards.into_iter().next()
    }

    /// The best estimate of [`Tester::run`].
    fn best(&self, cards: &[u32], prelude: &Prelude) -> f64 {
        self.run(cards, prelude).map_or(f64::NEG_INFINITY, |b| b.quick_score)
    }

    /// The best estimate of [`Tester::run`] with `card` joining the hand
    /// after `prelude`, if its line uses it.
    fn best_using(&self, cards: &[u32], prelude: &Prelude, card: u32) -> f64 {
        let db = self.engine.cards();
        let joined = Prelude { answers: prelude.answers.clone(), joining: vec![card] };
        let in_hand = |b: &Board| b.state.players[0].hand.iter().filter(|&&c| db.canonical_code(c) == card).count();
        // The starter's own copies still in the hand count too.
        let before = cards.iter().filter(|&&c| db.canonical_code(c) == card).count();
        match self.run(cards, &joined) {
            Some(b) if in_hand(&b) <= before => b.quick_score,
            _ => f64::NEG_INFINITY,
        }
    }

    /// The answers of `board`'s line (found from `cards`) up to its last
    /// free moment, the one that ends the Main Phase: the turn to go on from.
    fn up_to_last_free_moment(&self, cards: &[u32], board: &Board) -> Option<Prelude> {
        let config = &self.base;
        let mut duel = Duel::goldfish_against(self.engine, &self.deck, &self.hand(cards), config.seed, config.opponent.as_ref()).ok()?;
        let mut last_idle = None;
        for (i, answer) in board.line.answers.iter().enumerate() {
            let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
            if matches!(decision, Decision::IdleCmd { .. }) {
                last_idle = Some(i);
            }
            duel.respond(answer);
        }
        Some(Prelude { answers: board.line.answers[..last_idle?].to_vec(), joining: Vec::new() })
    }
}

/// Guesses the roles of the cards of the Main Deck of `deck`.
/// `passive` are the cards never played from the hand on your turn (the
/// well-known handtraps, as in the Combo Lab): the other handtraps, often
/// an archetype's quick effects, are tried as starters and extenders too.
/// `progress(done, total)` is called after each test.
#[allow(clippy::too_many_arguments)]
pub fn guess_roles(
    engine: &Engine,
    deck: &DeckList,
    kit: &Kit,
    hand_traps: &[u32],
    passive: &[u32],
    values: Arc<CardValues>,
    threads: usize,
    progress: impl Fn(usize, usize) + Sync,
) -> Vec<RoleGuess> {
    let db = engine.cards();
    let Some(blank) = db.find_by_name(BLANK).map(|c| c.code) else { return Vec::new() };
    let mut cards: Vec<u32> = Vec::new();
    for &c in &deck.main {
        let c = db.canonical_code(c);
        if !cards.contains(&c) {
            cards.push(c);
        }
    }
    let is_hand_trap = |c: u32| hand_traps.iter().any(|&h| db.canonical_code(h) == c);
    // The blanks come from the Deck like any card in hand.
    let mut with_blanks = deck.clone();
    with_blanks.main.extend([blank; 5]);
    let mut base = SearchConfig::new(with_blanks.clone(), Vec::new());
    base.threads = 1;
    base.max_nodes = NODES;
    base.time_limit = Duration::from_secs(120);
    base.hand_traps = hand_traps.to_vec();
    base.values = values.clone();
    base.opponent = Some(kit.opponent().clone());
    base.policy.passive = passive.to_vec();
    base.live_top = 0;
    base.confirm_top = 5;
    base.zone_top = 0;
    let tester = Tester { engine, deck: with_blanks, blank, base };

    let is_passive = |c: u32| passive.iter().any(|&p| db.canonical_code(p) == c);
    let tested: Vec<u32> = cards.iter().copied().filter(|&c| !is_passive(c)).collect();
    let total = tested.len() * 2 + 1;
    let done = AtomicUsize::new(0);
    let step = || progress(done.fetch_add(1, Ordering::Relaxed) + 1, total);
    let none = Prelude::default();
    let blanks = tester.best(&[], &none);
    step();
    // Alone in hand.
    let alone: HashMap<u32, f64> = parallel(&tested, threads, |&c| {
        let v = tester.best(&[c], &none);
        step();
        v
    });
    let starters: Vec<u32> = tested.iter().copied().filter(|c| alone[c] - blanks >= STARTER_GAIN).collect();
    let best_starter = starters.iter().copied().max_by(|a, b| alone[a].total_cmp(&alone[b]).then(b.cmp(a)));
    // In the best starter's turn, going on with the card against going on without it.
    let line = best_starter.and_then(|s| Some((s, tester.up_to_last_free_moment(&[s], &tester.run(&[s], &none)?)?)));
    let on_board = line.as_ref().map_or(f64::NEG_INFINITY, |(s, prelude)| tester.best(&[*s], prelude));
    let with_starter: HashMap<u32, f64> = match &line {
        None => HashMap::new(),
        Some((s, prelude)) => parallel(&tested, threads, |&c| {
            let v = if starters.contains(&c) { on_board } else { tester.best_using(&[*s], prelude, c) };
            step();
            v
        }),
    };

    let breakers: Vec<u32> = breakers_by_text(engine, &cards);
    cards
        .iter()
        .map(|&code| {
            let mut roles = Vec::new();
            let mut reasons = Vec::new();
            if is_hand_trap(code) {
                roles.push("handtrap");
                reasons.push(if values.usable(engine, code, Spot::Hand) && !values.usable(engine, code, Spot::HandWithField) {
                    "Si usa dalla mano solo se non controlli carte: per secondo; da primo è la carta da scartare".to_owned()
                } else {
                    "Si usa dalla mano nel turno dell'avversario".to_owned()
                });
            }
            if let Some(&v) = alone.get(&code) {
                if v - blanks >= STARTER_GAIN {
                    roles.push("starter");
                    reasons.push(format!("Da sola porta a una board (stima +{:.1})", v - blanks));
                } else if let (Some(s), Some(&w)) = (best_starter, with_starter.get(&code)) {
                    let gain = w - on_board;
                    if gain.is_finite() && gain >= EXTENDER_GAIN {
                        roles.push("extender");
                        reasons.push(format!("Allunga la combo di {} (stima +{gain:.1})", db.name(s)));
                    }
                }
                if roles.is_empty() && v - blanks < 0.5 && !breakers.contains(&code) && not_summonable_alone(engine, code) {
                    roles.push("brick");
                    reasons.push("Da sola non si può evocare né usare".to_owned());
                }
            }
            if breakers.contains(&code) {
                roles.push("breaker");
                reasons.push("Toglie o nega carte dell'avversario (dal testo)".to_owned());
            }
            RoleGuess { code, roles, reasons }
        })
        .collect()
}

/// A Main Deck monster that needs Tributes, or that cannot be Normal Summoned.
fn not_summonable_alone(engine: &Engine, code: u32) -> bool {
    engine.cards().get(code).is_some_and(|c| {
        c.type_ & card_type::MONSTER != 0
            && c.type_ & card_type::EXTRA == 0
            && (c.level & 0xff >= 5 || c.desc.to_lowercase().contains("cannot be normal summoned"))
    })
}

/// Board breakers by their text: they destroy, banish, return, negate or
/// take the opponent's cards, several at once or from a Spell or Trap.
pub(crate) fn breakers_by_text(engine: &Engine, cards: &[u32]) -> Vec<u32> {
    const ALL: [&str; 6] = ["destroy all", "banish all", "return all", "negate the effects of all", "shuffle all", "send all"];
    cards
        .iter()
        .copied()
        .filter(|&code| {
            let Some(card) = engine.cards().get(code) else { return false };
            let text = card.desc.to_lowercase();
            let controls = text.contains("your opponent controls") || text.contains("your opponent currently controls");
            let opponent = controls || text.contains("on the field");
            let many = ALL.iter().any(|p| text.contains(p)) || (text.contains("as many") && text.contains("as possible"));
            // The Kaiju and Lava Golem: Special Summoned to the opponent's
            // field by Tributing its monsters (older texts say "your opponent controls").
            let kaiju = text.contains("by tributing 1 monster your opponent controls")
                || (text.contains("to your opponent's field") && text.contains("by tributing"));
            let spell_trap = card.type_ & (card_type::SPELL | card_type::TRAP) != 0;
            let removes = ["destroy", "banish", "return", "negate", "shuffle"].iter().any(|w| text.contains(w));
            kaiju || (opponent && many) || (spell_trap && removes && controls)
        })
        .collect()
}

/// Runs `f` on every item on `threads` threads.
fn parallel<T: Sync + Copy + Into<u32>, R: Send>(items: &[T], threads: usize, f: impl Fn(&T) -> R + Sync) -> HashMap<u32, R> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(HashMap::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, items.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(item);
                if let Ok(mut o) = out.lock() {
                    o.insert((*item).into(), r);
                }
            });
        }
    });
    out.into_inner().unwrap_or_default()
}
