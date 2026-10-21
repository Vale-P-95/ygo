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
//! - **Extender**: on the board the best starter makes (put on the field
//!   and in the GY), it makes the board better by at least
//!   [`EXTENDER_GAIN`] than cards that do nothing would, and the best line
//!   uses it (a short search can end a little higher by chance). (Searching from
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

use ygo_engine::consts::{card_type, location, position};
use ygo_engine::{DeckList, DuelState, Engine, Placement};

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
/// Nodes of each test on the starter's board (more to try there).
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
    /// The best board reached from `cards` and blanks up to five, with
    /// `placed` on the field and in the GY.
    fn run(&self, cards: &[u32], placed: &[Placement]) -> Option<Board> {
        let mut hand = cards.to_vec();
        while hand.len() < 5 {
            hand.push(self.blank);
        }
        let mut config = self.base.clone();
        config.deck = self.deck.clone();
        config.hand = hand;
        config.placed = placed.to_vec();
        if !placed.is_empty() {
            config.max_nodes = BOARD_NODES;
        }
        let result = search(self.engine, &config, &AtomicBool::new(false), |_| {});
        result.boards.into_iter().next()
    }

    /// The best estimate of [`Tester::run`].
    fn best(&self, cards: &[u32], placed: &[Placement]) -> f64 {
        self.run(cards, placed).map_or(f64::NEG_INFINITY, |b| b.quick_score)
    }

    /// The best estimate of [`Tester::run`] with `card`, if its line uses it.
    fn best_using(&self, card: u32, placed: &[Placement]) -> f64 {
        let db = self.engine.cards();
        match self.run(&[card], placed) {
            Some(b) if !b.state.players[0].hand.iter().any(|&c| db.canonical_code(c) == card) => b.quick_score,
            _ => f64::NEG_INFINITY,
        }
    }
}

/// Your field and GY in `state`, to put there again.
fn placements(state: &DuelState) -> Vec<Placement> {
    let me = &state.players[0];
    let mut out = Vec::new();
    for (z, c) in me.monsters.iter().enumerate() {
        let Some(c) = c else { continue };
        out.push(Placement {
            code: c.code,
            location: location::MZONE,
            sequence: z as u32,
            position: c.position,
            overlay: c.overlay.clone(),
        });
    }
    for (z, c) in me.spells.iter().enumerate() {
        let Some(c) = c else { continue };
        out.push(Placement { code: c.code, location: location::SZONE, sequence: z as u32, position: c.position, overlay: Vec::new() });
    }
    for (i, &code) in me.grave.iter().enumerate() {
        out.push(Placement { code, location: location::GRAVE, sequence: i as u32, position: position::FACEUP_ATTACK, overlay: Vec::new() });
    }
    out
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
    let blanks = tester.best(&[], &[]);
    step();
    // Alone in hand.
    let alone: HashMap<u32, f64> = parallel(&tested, threads, |&c| {
        let v = tester.best(&[c], &[]);
        step();
        v
    });
    let starters: Vec<u32> = tested.iter().copied().filter(|c| alone[c] - blanks >= STARTER_GAIN).collect();
    let best_starter = starters.iter().copied().max_by(|a, b| alone[a].total_cmp(&alone[b]).then(b.cmp(a)));
    // On the best starter's board, against blanks on the same board.
    let board = best_starter.and_then(|s| tester.run(&[s], &[])).map(|b| placements(&b.state)).unwrap_or_default();
    let on_board = if board.is_empty() { f64::NEG_INFINITY } else { tester.best(&[], &board) };
    let with_starter: HashMap<u32, f64> = if board.is_empty() {
        HashMap::new()
    } else {
        parallel(&tested, threads, |&c| {
            let v = if starters.contains(&c) { on_board } else { tester.best_using(c, &board) };
            step();
            v
        })
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
