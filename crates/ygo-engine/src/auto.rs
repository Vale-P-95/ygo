//! Answers to decisions that do not change what the search can reach, so
//! that a line is a list of real choices ("activate and resolve" becomes
//! one step once the chain prompts with a single answer are taken).

use crate::actions::{Action, ActionKind};
use crate::consts::position;
use crate::duel::{Duel, Step};
use crate::message::Decision;
use crate::Error;

#[derive(Debug, Clone)]
pub struct AutoPolicy {
    /// Summon in face-up Attack Position when offered (else face-up Defense).
    pub positions: bool,
    /// Never enter the Battle Phase (going first, no one can attack).
    pub skip_battle: bool,
}

impl Default for AutoPolicy {
    fn default() -> Self {
        Self { positions: true, skip_battle: true }
    }
}

/// The answers the search should consider for `decision`.
pub fn choices(decision: &Decision, actions: Vec<Action>, policy: &AutoPolicy) -> Vec<Action> {
    if policy.skip_battle && matches!(decision, Decision::IdleCmd { .. }) {
        return actions.into_iter().filter(|a| a.kind != ActionKind::ToBattlePhase).collect();
    }
    actions
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

impl Duel {
    /// Like [`Duel::advance`], but answers the decisions [`auto_answer`]
    /// takes care of, appending those answers to `answers` so the line can
    /// be replayed. Returns at the next real choice or at the end.
    pub fn advance_auto(&mut self, policy: &AutoPolicy, answers: &mut Vec<Vec<u8>>) -> Result<(Step, Vec<Action>), Error> {
        let mut events = Vec::new();
        loop {
            match self.advance()? {
                Step::Decision { decision, events: mut new } => {
                    events.append(&mut new);
                    let actions = choices(&decision, self.actions(&decision), policy);
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
