//! Playing the answers of a line again where the questions changed: your
//! line with the opponent's handtrap in it, or the opponent's line in its
//! own turn, after yours, with your board on the field. Each answer is found
//! again by what it does (the card, where it is, the effect), not by its
//! place in a list.

use std::mem::Discriminant;

use ygo_engine::auto::passive_answer;
use ygo_engine::{Action, ActionKind, CardRef, Decision, DeckList, Duel, Engine, Error, Opponent, Step};

use crate::probe::{end_turn, window_answer};
use crate::search::Line;

/// Passing in a chain window.
pub(crate) const PASS: [u8; 4] = (-1i32).to_le_bytes();

/// The answers of the player of a line, without the other player's (with
/// another deck the other player is asked at other moments), each with the
/// kind of decision it answers.
#[derive(Debug, Clone, Default)]
pub(crate) struct Script {
    pub(crate) answers: Vec<Vec<u8>>,
    pub(crate) kinds: Vec<Discriminant<Decision>>,
    /// What each answer does when it plays a card: after a handtrap the
    /// same action can sit elsewhere in the list (another card became
    /// playable), or come as a trigger window instead of a yes or no.
    pub(crate) ids: Vec<Option<ActionId>>,
    /// The cards each answer picks in a selection, by what they are: after
    /// a handtrap the list can hold them in another order (a card back in
    /// the Extra Deck moves the others).
    pub(crate) picks: Vec<Option<Vec<Picked>>>,
    /// The option each answer chooses, by its description: a handtrap can
    /// add another option before it (Maxx "C" in the opponent's GY, a target
    /// for one more effect of the card).
    pub(crate) options: Vec<Option<u64>>,
    /// Where each answer is in the line, and whether it answers a free
    /// moment of your Main Phase (where the line can stop).
    pub(crate) at_line: Vec<usize>,
    pub(crate) idle: Vec<bool>,
}

impl Script {
    /// The script's first `n` answers (a line stopped there).
    pub(crate) fn up_to(&self, n: usize) -> Script {
        let n = n.min(self.answers.len());
        Script {
            answers: self.answers[..n].to_vec(),
            kinds: self.kinds[..n].to_vec(),
            ids: self.ids[..n].to_vec(),
            picks: self.picks[..n].to_vec(),
            options: self.options[..n].to_vec(),
            at_line: self.at_line[..n].to_vec(),
            idle: self.idle[..n].to_vec(),
        }
    }
}

/// A card picked in a selection: what it is, where, and whether it is the
/// chooser's own (the line of a deck is found as player 0 and played again
/// as player 1, in its turn after yours).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Picked {
    code: u32,
    location: u8,
    own: bool,
}

impl Picked {
    fn of(engine: &Engine, card: &CardRef, chooser: Option<u8>) -> Picked {
        Picked { code: engine.cards().canonical_code(card.code), location: card.location, own: Some(card.controller) == chooser }
    }
}

/// The cards of a selection, in the order the engine lists them (for a
/// sum of Levels, the cards to pick besides those it must use).
pub(crate) fn selection(decision: &Decision) -> Option<Vec<CardRef>> {
    match decision {
        Decision::SelectCard { cards, .. } => Some(cards.clone()),
        Decision::SelectTribute { cards, .. } => Some(cards.iter().map(|(c, _)| *c).collect()),
        Decision::SelectSum { cards, .. } => Some(cards.iter().map(|(c, _)| *c).collect()),
        Decision::SelectUnselectCard { selectable, .. } => Some(selectable.clone()),
        _ => None,
    }
}

/// The cards `answer` picks in `decision`: indices after a 0 and a count,
/// or one card after a 1 when cards are picked one at a time.
pub(crate) fn picked(engine: &Engine, decision: &Decision, answer: &[u8]) -> Option<Vec<Picked>> {
    let word = |i: usize| answer.get(i * 4..i * 4 + 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let cards = selection(decision)?;
    let at: Vec<usize> = match (decision, word(0)?) {
        (Decision::SelectUnselectCard { .. }, 1) => vec![usize::try_from(word(1)?).ok()?],
        (Decision::SelectCard { .. } | Decision::SelectTribute { .. } | Decision::SelectSum { .. }, 0) => {
            (0..usize::try_from(word(1)?).ok()?).map(|k| word(2 + k).and_then(|v| usize::try_from(v).ok())).collect::<Option<_>>()?
        }
        _ => return None,
    };
    at.iter().map(|&i| cards.get(i).map(|c| Picked::of(engine, c, decision.player()))).collect()
}

/// For each answer of your `line`, played against `opponent` (the duel it
/// was found in), the cards it picks when some are the opponent's: played
/// against another opponent (more cards in its Extra Deck), the line picks
/// the same cards again by what they are, not by their place in the list
/// (Hyperinvoked Aeon banishing from the opponent's Extra Deck).
pub(crate) fn line_picks(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: &Opponent,
    line: &Line,
) -> Result<Vec<Option<Vec<Picked>>>, Error> {
    let mut duel = Duel::goldfish_vs(engine, deck, hand, seed, opponent)?;
    let mut out = Vec::with_capacity(line.answers.len());
    for answer in &line.answers {
        let Step::Decision { decision, .. } = duel.advance()? else { break };
        let picks = picked(engine, &decision, answer).filter(|p| decision.player() == Some(0) && p.iter().any(|c| !c.own));
        out.push(picks);
        duel.respond(answer);
    }
    Ok(out)
}

/// The answer to `decision` that picks `picks` again, wherever they are listed.
pub(crate) fn pick_again(engine: &Engine, decision: &Decision, picks: &[Picked]) -> Option<Vec<u8>> {
    let cards = selection(decision)?;
    let mut taken = vec![false; cards.len()];
    let mut at = Vec::new();
    for p in picks {
        let i = (0..cards.len()).find(|&i| !taken[i] && Picked::of(engine, &cards[i], decision.player()) == *p)?;
        taken[i] = true;
        at.push(i as u32);
    }
    let mut out = Vec::new();
    if let Decision::SelectUnselectCard { .. } = decision {
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&at.first()?.to_le_bytes());
    } else {
        out.extend_from_slice(&0i32.to_le_bytes());
        out.extend_from_slice(&(at.len() as u32).to_le_bytes());
        for i in at {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    Some(out)
}

/// An action by what it is, not by its place in a list. `asked` when it
/// was a yes to a question, whose description is not the effect's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ActionId {
    pub(crate) code: u32,
    pub(crate) location: u8,
    pub(crate) description: u64,
    pub(crate) kind: ActionKind,
    pub(crate) asked: bool,
}

pub(crate) fn own_answers(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: &Opponent,
    line: &Line,
) -> Result<Script, Error> {
    let db = engine.cards();
    let mut duel = Duel::goldfish_against(engine, deck, hand, seed, Some(opponent))?;
    let mut own = Script::default();
    for (i, answer) in line.answers.iter().enumerate() {
        let Step::Decision { decision, .. } = duel.advance()? else { break };
        if decision.player() != Some(1) {
            let id = match &decision {
                Decision::EffectYesNo { card, description, .. } if *answer == window_answer(&decision, Some(0)) => Some(ActionId {
                    code: db.canonical_code(card.code),
                    location: card.location,
                    description: *description,
                    kind: ActionKind::Activate,
                    asked: true,
                }),
                Decision::IdleCmd { .. } | Decision::SelectChain { .. } => {
                    duel.actions(&decision).iter().find(|a| &a.response == answer).and_then(|a| {
                        a.card.map(|c| ActionId {
                            code: db.canonical_code(c.code),
                            location: c.location,
                            description: a.description,
                            kind: a.kind,
                            asked: false,
                        })
                    })
                }
                _ => None,
            };
            own.picks.push(picked(engine, &decision, answer));
            own.options.push(match decision {
                Decision::Option { .. } => duel.actions(&decision).iter().find(|a| &a.response == answer).map(|a| a.description),
                _ => None,
            });
            own.at_line.push(i);
            own.idle.push(matches!(decision, Decision::IdleCmd { .. }));
            own.answers.push(answer.clone());
            own.kinds.push(std::mem::discriminant(&decision));
            own.ids.push(id);
        }
        duel.respond(answer);
    }
    Ok(own)
}

/// The answer that chooses the option described by `description` again,
/// wherever it is listed.
pub(crate) fn option_again(decision: &Decision, actions: &[Action], description: u64) -> Option<Vec<u8>> {
    matches!(decision, Decision::Option { .. }).then_some(())?;
    actions.iter().find(|a| a.description == description).map(|a| a.response.clone())
}

/// The answer to `decision` that plays the action `id`, if it is offered.
pub(crate) fn find_again(engine: &Engine, decision: &Decision, actions: &[Action], id: ActionId) -> Option<Vec<u8>> {
    let db = engine.cards();
    match decision {
        Decision::EffectYesNo { card, .. } => {
            (db.canonical_code(card.code) == id.code && card.location == id.location).then(|| window_answer(decision, Some(0)))
        }
        Decision::IdleCmd { .. } | Decision::SelectChain { .. } => {
            let same = |a: &&ygo_engine::Action| {
                a.kind == id.kind && a.card.is_some_and(|c| db.canonical_code(c.code) == id.code && c.location == id.location)
            };
            let offered: Vec<&ygo_engine::Action> = actions.iter().filter(same).collect();
            // The same effect; another effect of the card only for a yes
            // asked before (the card's other effect would be another play:
            // a negation instead of a search).
            offered.iter().find(|a| a.description == id.description).or(offered.first().filter(|_| id.asked)).map(|a| a.response.clone())
        }
        _ => None,
    }
}

/// A question that can be declined: a chance to chain, a yes or no, or a
/// Main Phase with nothing left to do but end it.
pub(crate) fn declinable(decision: &Decision) -> bool {
    match decision {
        Decision::SelectChain { forced, .. } => !forced,
        Decision::EffectYesNo { .. } | Decision::YesNo { .. } => true,
        Decision::IdleCmd { summonable, special_summonable, repositionable, monster_settable, spell_settable, activatable, .. } => {
            summonable.is_empty()
                && special_summonable.is_empty()
                && repositionable.is_empty()
                && monster_settable.is_empty()
                && spell_settable.is_empty()
                && activatable.is_empty()
        }
        _ => false,
    }
}

/// An answer of a [`Follower`], and the place in its script of the answer
/// it plays when that is one of the script's actions.
#[derive(Debug, Clone)]
pub(crate) struct Followed {
    pub(crate) answer: Vec<u8>,
    pub(crate) at: Option<usize>,
}

/// Plays a script again in a duel that asks other questions: the opponent's
/// line in its turn after yours, with your board on the field (another turn,
/// other cards to answer, some of its cards stopped). The script's actions
/// are found again by what they are. A question the script was not asked
/// here is declined. When the script's next action is not offered (your
/// board stopped what it needed), the script goes on from the first later
/// action that is; when none is, its turn is over.
pub(crate) struct Follower<'a> {
    script: &'a Script,
    next: usize,
}

impl<'a> Follower<'a> {
    pub(crate) fn new(script: &'a Script) -> Self {
        Follower { script, next: 0 }
    }

    /// The answer to `decision`, asked to the script's player. `others_on_top`
    /// when the last link of the chain being built is the other player's.
    /// `None` at a free moment of the Main Phase with nothing of the script
    /// left to play: the turn is over.
    pub(crate) fn answer(&mut self, engine: &Engine, decision: &Decision, actions: &[Action], others_on_top: bool) -> Option<Followed> {
        let s = self.script;
        let kind = std::mem::discriminant(decision);
        let found = |k: usize| s.ids[k].and_then(|id| find_again(engine, decision, actions, id));
        let play = |this: &mut Self, k: usize, answer: Vec<u8>| {
            this.next = k + 1;
            Some(Followed { answer, at: Some(k) })
        };
        // A chance to respond to the other player's effect: the script never
        // had it (its line was found with nobody answering).
        if others_on_top && matches!(decision, Decision::SelectChain { .. }) {
            return Some(Followed { answer: passive_answer(decision, actions), at: None });
        }
        // Passes of the script the engine no longer asks for are left out
        // first: the question after them is the script's own.
        while self.next < s.answers.len() && s.kinds[self.next] != kind && s.answers[self.next] == PASS {
            self.next += 1;
        }
        // The script's next action, or the first one after its passes when
        // it is offered already (a chain took a window the script passed in).
        let candidate = if s.answers.get(self.next) == Some(&PASS.to_vec()) {
            (self.next..s.answers.len()).find(|&k| s.answers[k] != PASS).filter(|&k| s.ids[k].is_some())
        } else {
            s.ids.get(self.next).copied().flatten().map(|_| self.next)
        };
        if let Some((k, answer)) = candidate.and_then(|k| found(k).map(|a| (k, a))) {
            return play(self, k, answer);
        }
        let expected = s.ids.get(self.next).copied().flatten();
        if s.kinds.get(self.next) == Some(&kind) && expected.is_none() {
            // The script's own question with no action to find (a selection,
            // an option, a zone, a pass): its answer, the cards it picks found
            // by what they are; an answer that does not fit any more gives way
            // to a plain one.
            let raw = &s.answers[self.next];
            let again = s.picks[self.next]
                .as_ref()
                .and_then(|p| pick_again(engine, decision, p))
                .or_else(|| s.options[self.next].and_then(|o| option_again(decision, actions, o)));
            let fits = actions.iter().any(|a| a.response == *raw);
            self.next += 1;
            let answer = again.or_else(|| fits.then(|| raw.clone())).unwrap_or_else(|| fallback(decision, actions));
            return Some(Followed { answer, at: None });
        }
        if let Decision::IdleCmd { .. } = decision {
            // The script's action is gone (your board stopped what it
            // needed): it goes on from its first later action offered now,
            // or its turn is over.
            let later = (self.next..s.answers.len()).filter(|&k| s.idle[k]).find_map(|k| found(k).map(|a| (k, a)));
            if let Some((k, answer)) = later {
                return play(self, k, answer);
            }
            self.next = s.answers.len();
            return None;
        }
        // A question the script was not asked here (a trigger it did not
        // have, a window of the Draw Phase): declined, or answered plainly.
        Some(Followed { answer: fallback(decision, actions), at: None })
    }
}

impl Follower<'_> {
    /// The player ended its Main Phase: the script goes on from its first
    /// answer after its last free moment of the Main Phase (its End Phase),
    /// even when its last actions were not offered.
    pub(crate) fn end_main_phase(&mut self) {
        if let Some(last) = self.script.idle.iter().rposition(|&idle| idle) {
            self.next = last + 1;
        }
    }
}

/// A plain answer: decline when possible, else the first choice offered
/// (the end of the turn at a free moment).
fn fallback(decision: &Decision, actions: &[Action]) -> Vec<u8> {
    if let Decision::IdleCmd { .. } = decision {
        return end_turn(decision, actions);
    }
    if declinable(decision) {
        return passive_answer(decision, actions);
    }
    actions
        .iter()
        .find(|a| !matches!(a.kind, ActionKind::Cancel | ActionKind::Finish))
        .or(actions.first())
        .map_or_else(|| passive_answer(decision, actions), |a| a.response.clone())
}
