//! Answers to decisions that do not change what the search can reach, so
//! that a line is a list of real choices ("activate and resolve" becomes
//! one step once the chain prompts with a single answer are taken).

use crate::actions::{Action, ActionKind};
use crate::carddb::CardDb;
use crate::consts::{card_type, location, position};
use crate::duel::{Duel, Step};
use crate::message::Decision;
use crate::Error;

#[derive(Debug, Clone)]
pub struct AutoPolicy {
    /// Summon in face-up Attack Position when offered (else face-up Defense).
    pub positions: bool,
    /// Never enter the Battle Phase (going first, no one can attack).
    pub skip_battle: bool,
    /// Cards never summoned, set or activated.
    pub forbid: Vec<u32>,
    /// Cards not activated from the hand (handtraps on your own turn).
    pub passive: Vec<u32>,
    /// Among your free zones, offer one per kind: Extra Monster Zone or not,
    /// and which of your Link arrows point to it. Zones that differ only by
    /// position are taken as the same (faster, rarely loses a line).
    pub zones: bool,
    /// Only Traps and Quick-Play Spells are Set: any other Spell can be
    /// activated straight from the hand, and Set it only leaves the hand.
    pub useful_sets_only: bool,
    /// The opponent's effects (code, description) it uses on your turn, the
    /// first in the list first; with none it does nothing ([`opponent_answer`]).
    pub opponent_responders: Vec<(u32, u64)>,
}

impl Default for AutoPolicy {
    fn default() -> Self {
        Self {
            positions: true,
            skip_battle: true,
            forbid: Vec::new(),
            passive: Vec::new(),
            zones: true,
            useful_sets_only: true,
            opponent_responders: Vec::new(),
        }
    }
}

/// The answers the search should consider for `decision`. Never empties a
/// list that had answers: if every answer is ruled out, they all stay.
/// Cards are compared by their original code (alternate artworks included).
pub fn choices(decision: &Decision, actions: Vec<Action>, policy: &AutoPolicy, db: &CardDb) -> Vec<Action> {
    let skip_battle = policy.skip_battle && matches!(decision, Decision::IdleCmd { .. });
    let listed = |list: &[u32], code: u32| {
        let code = db.canonical_code(code);
        list.iter().any(|&c| db.canonical_code(c) == code)
    };
    let pointless_set = |a: &Action| {
        let Some(c) = a.card.filter(|_| policy.useful_sets_only && a.kind == ActionKind::SetSpellTrap) else { return false };
        let type_ = db.get(c.code).map_or(0, |d| d.type_);
        type_ & card_type::SPELL != 0 && type_ & card_type::QUICKPLAY == 0
    };
    let keep = |a: &Action| {
        if skip_battle && a.kind == ActionKind::ToBattlePhase || pointless_set(a) {
            return false;
        }
        match a.card {
            Some(c) if listed(&policy.forbid, c.code) => false,
            Some(c) if a.kind == ActionKind::Activate && c.location == location::HAND && listed(&policy.passive, c.code) => false,
            _ => true,
        }
    };
    if actions.iter().any(keep) {
        actions.into_iter().filter(keep).collect()
    } else {
        actions
    }
}

/// The answer to give without asking, if there is nothing to choose.
pub fn auto_answer(decision: &Decision, actions: &[Action], policy: &AutoPolicy) -> Option<usize> {
    if actions.len() == 1 {
        return Some(0);
    }
    if policy.positions {
        if let Decision::SelectPosition { positions, .. } = decision {
            for preferred in [position::FACEUP_ATTACK, position::FACEUP_DEFENSE] {
                if *positions as u32 & preferred != 0 {
                    return actions.iter().position(|a| a.response == (preferred as i32).to_le_bytes());
                }
            }
        }
    }
    None
}

/// Link arrows (bottom-left, bottom, ...) and the zone they point to, from
/// a Link monster in `from` (Monster Zones: 0-4 main, 5-6 extra).
fn arrow_targets(from: usize) -> &'static [(u32, usize)] {
    const BL: u32 = 0x1;
    const B: u32 = 0x2;
    const BR: u32 = 0x4;
    const L: u32 = 0x8;
    const R: u32 = 0x20;
    const TL: u32 = 0x40;
    const T: u32 = 0x80;
    const TR: u32 = 0x100;
    match from {
        0 => &[(R, 1), (TR, 5)],
        1 => &[(L, 0), (R, 2), (T, 5)],
        2 => &[(L, 1), (R, 3), (TL, 5), (TR, 6)],
        3 => &[(L, 2), (R, 4), (T, 6)],
        4 => &[(L, 3), (TL, 6)],
        5 => &[(BL, 0), (B, 1), (BR, 2)],
        6 => &[(BL, 2), (B, 3), (BR, 4)],
        _ => &[],
    }
}

/// One zone per kind among the offered places of `player`'s own field;
/// other places (the opponent's field) stay as they are.
pub fn distinct_zones(state: &crate::state::DuelState, player: u8, actions: Vec<Action>) -> Vec<Action> {
    let me = &state.players[player as usize];
    let pointed = |zone: usize| -> Vec<(usize, u32)> {
        let mut by = Vec::new();
        for (from, card) in me.monsters.iter().enumerate() {
            let Some(card) = card else { continue };
            if card.link.0 == 0 || !card.is_face_up() {
                continue;
            }
            for &(arrow, to) in arrow_targets(from) {
                if to == zone && card.link.1 & arrow != 0 {
                    by.push((from, arrow));
                }
            }
        }
        by
    };
    // A Spell or Trap placed in a middle zone leaves both Pendulum Zones
    // free for the Scales set after it.
    let middle = actions.iter().any(|a| matches!(*a.response.as_slice(), [p, loc, 1..=3] if p == player && loc == location::SZONE));
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for a in actions {
        let kind = match *a.response.as_slice() {
            [p, loc, seq] if p == player && loc == location::MZONE => Some((loc, seq >= 5, pointed(seq as usize))),
            [p, loc, 0 | 4] if p == player && loc == location::SZONE && middle => continue,
            [p, loc, seq] if p == player && loc == location::SZONE && seq < 5 => Some((loc, false, Vec::new())),
            _ => None,
        };
        match kind {
            Some(kind) if seen.contains(&kind) => {}
            Some(kind) => {
                seen.push(kind);
                out.push(a);
            }
            None => out.push(a),
        }
    }
    out
}

/// The answer of a player who does nothing: not chaining, no, cancel or
/// finish when offered, else the first answer.
pub fn passive_answer(decision: &Decision, actions: &[Action]) -> Vec<u8> {
    let pass = actions.iter().find(|a| matches!(a.kind, ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish));
    match pass.or(actions.first()) {
        Some(a) => a.response.clone(),
        None if matches!(decision, Decision::EffectYesNo { .. } | Decision::YesNo { .. }) => 0i32.to_le_bytes().to_vec(),
        None => (-1i32).to_le_bytes().to_vec(),
    }
}

/// The opponent's answer on your turn: in a chance to respond, its effect
/// that comes first in `responders` among those offered; in its own link
/// (activating or resolving), your cards first; else [`passive_answer`].
pub fn opponent_answer(decision: &Decision, actions: &[Action], responders: &[(u32, u64)], db: &CardDb, own_link: bool) -> Vec<u8> {
    let rank = |a: &Action| {
        let c = a.card.filter(|_| a.kind == ActionKind::Activate)?;
        let code = db.canonical_code(c.code);
        responders.iter().position(|&(r, d)| db.canonical_code(r) == code && d == a.description)
    };
    match decision {
        Decision::SelectChain { .. } => {
            if let Some(a) = actions.iter().filter(|a| rank(a).is_some()).min_by_key(|a| rank(a)) {
                return a.response.clone();
            }
        }
        Decision::EffectYesNo { card, description, .. } => {
            let code = db.canonical_code(card.code);
            if responders.iter().any(|&(r, d)| db.canonical_code(r) == code && d == *description) {
                return 1i32.to_le_bytes().to_vec();
            }
        }
        // Its own link goes forward: it never cancels (the engine would
        // offer the effect again), and picks your cards first.
        Decision::SelectCard { cards, .. } if own_link => {
            // Picks are index lists: the one with the most of your cards.
            let yours = |a: &Action| {
                let Some(picks) = a.response.get(8..).filter(|_| a.kind == ActionKind::Choose) else { return 0 };
                let indices = picks.as_chunks::<4>().0;
                indices.iter().filter(|&&i| cards.get(u32::from_le_bytes(i) as usize).is_some_and(|c| c.controller == 0)).count()
            };
            let pick = actions.iter().filter(|a| yours(a) > 0).max_by_key(|a| (yours(a), std::cmp::Reverse(a.response.len())));
            if let Some(a) = pick.or_else(|| actions.iter().find(|a| a.kind == ActionKind::Choose)) {
                return a.response.clone();
            }
        }
        Decision::SelectUnselectCard { finishable, selectable, .. } if own_link => {
            let finish = actions.iter().find(|a| *finishable && a.kind == ActionKind::Finish);
            let picks = &actions[..selectable.len().min(actions.len())];
            let pick = finish.or_else(|| picks.iter().find(|a| a.card.is_some_and(|c| c.controller == 0))).or(picks.first());
            if let Some(a) = pick {
                return a.response.clone();
            }
        }
        Decision::YesNo { .. } => {}
        _ if own_link => {
            if let Some(a) = actions.iter().find(|a| a.kind != ActionKind::Cancel) {
                return a.response.clone();
            }
        }
        _ => {}
    }
    passive_answer(decision, actions)
}

/// See [`Duel::advance_auto_seen`].
const MAX_OPPONENT_DECISIONS: usize = 200;

impl Duel {
    /// Like [`Duel::advance`], but answers the decisions [`auto_answer`]
    /// takes care of (and the opponent's, passing), appending those answers
    /// to `answers` so the line can be replayed. Returns at the next real
    /// choice or at the end.
    pub fn advance_auto(&mut self, policy: &AutoPolicy, answers: &mut Vec<Vec<u8>>) -> Result<(Step, Vec<Action>), Error> {
        self.advance_auto_seen(policy, answers, |_, _| {})
    }

    /// Like [`Duel::advance_auto`], telling `seen` each decision answered
    /// on the way and its answer.
    pub fn advance_auto_seen(
        &mut self,
        policy: &AutoPolicy,
        answers: &mut Vec<Vec<u8>>,
        mut seen: impl FnMut(&Decision, &[u8]),
    ) -> Result<(Step, Vec<Action>), Error> {
        let mut events = Vec::new();
        // Past this many of its decisions the opponent stops answering: an
        // effect it can neither finish nor leave would come back for ever.
        let mut theirs = 0;
        loop {
            match self.advance()? {
                Step::Decision { decision, events: mut new } => {
                    events.append(&mut new);
                    // The opponent does nothing on your turn, but for the
                    // effects it is given.
                    if decision.player() == Some(1) {
                        let actions = self.actions(&decision);
                        theirs += 1;
                        let answer = if policy.opponent_responders.is_empty() || theirs > MAX_OPPONENT_DECISIONS {
                            passive_answer(&decision, &actions)
                        } else {
                            let own_link = self.chain().last().is_some_and(|l| l.controller == 1);
                            opponent_answer(&decision, &actions, &policy.opponent_responders, self.cards(), own_link)
                        };
                        seen(&decision, &answer);
                        self.respond(&answer);
                        answers.push(answer);
                        continue;
                    }
                    let mut actions = choices(&decision, self.actions(&decision), policy, self.cards());
                    if policy.zones {
                        if let Decision::SelectPlace { player, count: 1, .. } = decision {
                            actions = distinct_zones(&self.state(), player, actions);
                        }
                    }
                    match auto_answer(&decision, &actions, policy) {
                        Some(i) => {
                            seen(&decision, &actions[i].response);
                            answers.push(actions[i].response.clone());
                            self.respond(&actions[i].response);
                        }
                        None => return Ok((Step::Decision { decision, events }, actions)),
                    }
                }
                Step::End { events: mut new } => {
                    events.append(&mut new);
                    return Ok((Step::End { events }, Vec::new()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::actions;
    use crate::message::{Activatable, CardRef};

    fn card(code: u32, controller: u8) -> CardRef {
        CardRef { code, controller, location: location::MZONE, sequence: 0, position: position::FACEUP_ATTACK }
    }

    #[test]
    fn the_opponent_answers_only_with_its_responders() {
        let db = CardDb::default();
        let chains = [(10, 100), (20, 200), (30, 300)]
            .map(|(code, description)| Activatable { card: card(code, 1), description, client_mode: 0 })
            .to_vec();
        let decision = Decision::SelectChain { player: 1, forced: false, triggers: false, chains };
        let offered = actions(&decision, &db);
        let index = |answer: Vec<u8>| i32::from_le_bytes(answer.try_into().unwrap());
        // The first in the list among those offered, whatever the order offered.
        assert_eq!(index(opponent_answer(&decision, &offered, &[(40, 400), (30, 300), (10, 100)], &db, false)), 2);
        // Another effect of the same card is not one of them.
        assert_eq!(index(opponent_answer(&decision, &offered, &[(20, 201)], &db, false)), -1);
        assert_eq!(index(opponent_answer(&decision, &offered, &[], &db, false)), -1);
    }

    #[test]
    fn in_its_own_link_the_opponent_aims_at_your_cards() {
        let db = CardDb::default();
        let decision =
            Decision::SelectCard { player: 1, cancelable: false, min: 1, max: 1, cards: vec![card(10, 1), card(20, 0)], hint: 0 };
        let offered = actions(&decision, &db);
        assert_eq!(opponent_answer(&decision, &offered, &[(1, 1)], &db, true), offered[1].response);
        assert_eq!(opponent_answer(&decision, &offered, &[(1, 1)], &db, false), passive_answer(&decision, &offered));
    }
}
