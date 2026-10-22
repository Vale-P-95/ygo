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
//!   one of its last free moments, the card joins the hand and the search goes on
//!   from there: it makes the board better by at least [`EXTENDER_GAIN`]
//!   than going on without it, or by [`EXTENDER_GAIN_BRINGING`] when it
//!   brings a card to the field, and the best line uses it (a short search
//!   can end a little higher by chance). What the starter's line used
//!   stays used: the Normal Summon, effects once per turn. (Searching from
//!   a hand with two real cards would not do: with more to try, a short
//!   search ends lower than with the starter alone.) A starter is tried
//!   too, on the line of another starter (the best one's on the second's).
//! - **Board breaker**: its text removes or negates the opponent's cards.
//! - **Brick**: a monster that cannot be summoned by itself (it needs
//!   Tributes or another way in) and alone does nothing.
//!
//! Each test is a short search on one thread with a node budget, so the
//! same deck always gets the same roles.

use std::collections::{HashMap, HashSet};
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
/// A smaller rise is enough when the card brings to the field a card of
/// the deck the board without it does not have: a card only Set, or used
/// for its own effect, rises the estimate by up to 1 and brings nothing.
pub const EXTENDER_GAIN_BRINGING: f64 = 0.75;
/// Nodes of each test alone in hand.
const NODES: u64 = 400;
/// Nodes of each test that goes on from the starter's line (more to try there).
const BOARD_NODES: u64 = 800;
/// The last free moments of the starter's line where a card joins the
/// hand. The last one alone is too late for a card that spends a monster
/// the line is done with: at the end that monster is part of the board, or
/// already gone.
const MOMENTS: usize = 2;

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

    /// The best board of [`Tester::run`] with `card` joining the hand
    /// after `prelude`, if its line uses it.
    fn best_using(&self, cards: &[u32], prelude: &Prelude, card: u32) -> Option<Board> {
        let db = self.engine.cards();
        let joined = Prelude { answers: prelude.answers.clone(), joining: vec![card] };
        let in_hand = |b: &Board| b.state.players[0].hand.iter().filter(|&&c| db.canonical_code(c) == card).count();
        // The starter's own copies still in the hand count too.
        let before = cards.iter().filter(|&&c| db.canonical_code(c) == card).count();
        self.run(cards, &joined).filter(|b| in_hand(b) <= before)
    }

    /// The answers of `board`'s line (found from `cards`) up to its last
    /// [`MOMENTS`] free moments, the last first (the one that ends the Main
    /// Phase): the turns to go on from.
    fn last_free_moments(&self, cards: &[u32], board: &Board) -> Vec<Prelude> {
        let config = &self.base;
        let Ok(mut duel) = Duel::goldfish_against(self.engine, &self.deck, &self.hand(cards), config.seed, config.opponent.as_ref()) else {
            return Vec::new();
        };
        let mut idle = Vec::new();
        for (i, answer) in board.line.answers.iter().enumerate() {
            let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
            if matches!(decision, Decision::IdleCmd { .. }) {
                idle.push(i);
            }
            duel.respond(answer);
        }
        idle.iter().rev().take(MOMENTS).map(|&i| Prelude { answers: board.line.answers[..i].to_vec(), joining: Vec::new() }).collect()
    }

    /// The cards of yours on the field of `board`.
    fn field(&self, board: &Board) -> HashSet<u32> {
        let db = self.engine.cards();
        let me = &board.state.players[0];
        me.monsters.iter().chain(&me.spells).flatten().map(|c| db.canonical_code(c.code)).collect()
    }

    /// The line of `starter` alone up to its last free moments, each with
    /// the best board going on from there without another card.
    fn starter_line(&self, starter: u32) -> Option<StarterLine> {
        let board = self.run(&[starter], &Prelude::default())?;
        let moments: Vec<Moment> = self
            .last_free_moments(&[starter], &board)
            .into_iter()
            .filter_map(|prelude| {
                let without = self.run(&[starter], &prelude)?;
                Some(Moment { score: without.quick_score, field: self.field(&without), prelude })
            })
            .collect();
        (!moments.is_empty()).then_some(StarterLine { starter, moments })
    }

    /// How much better `card` joining the hand makes the board of `line`,
    /// and whether it brings to the field a card (other than itself) the
    /// board without it does not have: at the last moment, or at the one
    /// before when that is not enough.
    fn gain_on(&self, line: &StarterLine, card: u32) -> (f64, bool) {
        let mut best = (f64::NEG_INFINITY, false);
        for m in &line.moments {
            let Some(with) = self.best_using(&[line.starter], &m.prelude, card) else { continue };
            let found = (with.quick_score - m.score, self.field(&with).iter().any(|c| *c != card && !m.field.contains(c)));
            if extends(found) {
                return found;
            }
            if found.0 > best.0 {
                best = found;
            }
        }
        best
    }
}

/// A rise of the estimate (and whether the card brings a new card to the
/// field) that makes a card an extender.
fn extends((gain, brings): (f64, bool)) -> bool {
    gain >= EXTENDER_GAIN || (brings && gain >= EXTENDER_GAIN_BRINGING)
}

/// A starter's line, where the other cards are tried as extenders.
struct StarterLine {
    starter: u32,
    moments: Vec<Moment>,
}

/// A free moment of a starter's line: the answers up to it, and the best
/// board going on from there without another card.
struct Moment {
    prelude: Prelude,
    score: f64,
    field: HashSet<u32>,
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
    let mut starters: Vec<u32> = tested.iter().copied().filter(|c| alone[c] - blanks >= STARTER_GAIN).collect();
    starters.sort_by(|a, b| alone[b].total_cmp(&alone[a]).then(a.cmp(b)));
    // In the best starter's turn, going on with the card against going on
    // without it; the best starter itself in the second one's turn.
    let lines: Vec<StarterLine> = starters.iter().take(2).filter_map(|&s| tester.starter_line(s)).collect();
    let extending: HashMap<u32, (u32, (f64, bool))> = parallel(&tested, threads, |&c| {
        let line = lines.iter().find(|l| l.starter != c);
        let v = line.map_or((0, (f64::NEG_INFINITY, false)), |l| (l.starter, tester.gain_on(l, c)));
        step();
        v
    });

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
                }
                if let Some(&(s, (gain, brings))) = extending.get(&code) {
                    if extends((gain, brings)) {
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
            kaiju || (opponent && many) || (spell_trap && removes && controls) || takes_as_material(&text)
        })
        .collect()
}

/// A sentence of `text` uses the opponent's monsters as material or as a
/// Tribute (Super Polymerization, the "from either field" Fusions): they
/// leave the field. Detaching an Xyz material "from a monster on either
/// field" takes no monster.
fn takes_as_material(text: &str) -> bool {
    // Split at ":" too: "if this card is sent to the GY as material: destroy 1
    // monster your opponent controls" removes it, it does not use it.
    text.split(['.', ';', ':', '\n']).any(|s| {
        // A Fusion Summon "by banishing ... 1 monster your opponent controls" names no material.
        let material = s.contains("material") || s.contains("tribut") || s.contains("fusion summon");
        // "monster(s) your opponent controls", "1 face-up monster your opponent controls".
        let controlled = s.match_indices("your opponent controls").any(|(i, _)| {
            s[..i].trim_end().ends_with("monster") || s[..i].trim_end().ends_with("monsters") || s[..i].trim_end().ends_with("monster(s)")
        });
        let theirs = s.contains("either field") || controlled;
        material && theirs && !s.contains("detach")
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taking_the_opponents_monsters_as_material_breaks_a_board() {
        let takes = |t: &str| takes_as_material(&t.to_lowercase());
        assert!(takes(
            "Discard 1 card; Fusion Summon 1 Fusion Monster from your Extra Deck, using monsters from either field as Fusion Material."
        ));
        assert!(takes("● Fusion Summon 1 Level 8 or higher Fusion Monster from your Extra Deck, by banishing Fusion Materials listed on it from either field."));
        assert!(takes("● Fusion Summon 1 \"Invoked\" Fusion Monster from your Extra Deck, by banishing 1 monster you control and 1 face-up monster your opponent controls."));
        assert!(takes("You can Special Summon this card to your opponent's field by Tributing 1 monster(s) your opponent controls."));
        // Xyz materials detached, monsters of your own, a condition on the opponent's field.
        assert!(!takes("Detach 1 material from a monster on either field, and if you do, excavate the top 4 cards of your Deck."));
        assert!(!takes("Fusion Summon 1 Fusion Monster from your Extra Deck, using monsters from your hand as Fusion Material."));
        assert!(!takes("If your opponent controls a monster: You can Special Summon this card. You can use this card as Link Material."));
        assert!(!takes("If this card is sent to the GY as Synchro Material: You can target 1 monster your opponent controls; destroy it."));
    }
}
