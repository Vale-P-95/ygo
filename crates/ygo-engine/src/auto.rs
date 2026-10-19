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
}

impl Default for AutoPolicy {
    fn default() -> Self {
        Self { positions: true, skip_battle: true, forbid: Vec::new(), passive: Vec::new(), zones: true, useful_sets_only: true }
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
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for a in actions {
        let kind = match *a.response.as_slice() {
            [p, loc, seq] if p == player && loc == location::MZONE => Some((loc, seq >= 5, pointed(seq as usize))),
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

impl Duel {
    /// Like [`Duel::advance`], but answers the decisions [`auto_answer`]
    /// takes care of (and the opponent's, passing), appending those answers
    /// to `answers` so the line can be replayed. Returns at the next real
    /// choice or at the end.
    pub fn advance_auto(&mut self, policy: &AutoPolicy, answers: &mut Vec<Vec<u8>>) -> Result<(Step, Vec<Action>), Error> {
        let mut events = Vec::new();
        loop {
            match self.advance()? {
                Step::Decision { decision, events: mut new } => {
                    events.append(&mut new);
                    // The opponent does nothing on your turn.
                    if decision.player() == Some(1) {
                        let answer = passive_answer(&decision, &self.actions(&decision));
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
