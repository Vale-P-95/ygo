//! Scripted play: follow a line written as a list of steps, each matched
//! against the labels of the legal actions. Used by the CLI and by tests
//! that pin down known combo lines and rulings.

use crate::actions::{actions, Action, ActionKind};
use crate::duel::{Duel, Step};
use crate::message::{Decision, Event};
use crate::Error;

/// What happened while following the script, in order.
pub enum Trace<'a> {
    Events(&'a [Event]),
    Chose(&'a Action),
}

#[derive(Debug)]
pub enum ScriptEnd {
    /// All steps were used; the duel waits on `decision`.
    Finished {
        decision: Decision,
        actions: Vec<Action>,
    },
    /// No legal action matched `step`.
    NoMatch {
        step: String,
        decision: Decision,
        actions: Vec<Action>,
    },
    /// A step chose "End turn"; the response was not sent, so the board can
    /// still be inspected as it is at the end of the line.
    TurnEnded,
    DuelEnded,
}

/// Decisions with an obvious answer when no step mentions them: zone,
/// position and ordering, and (while steps remain) declining to chain. Once
/// the steps are used up, chain windows stop the script so that optional
/// triggers are never skipped silently.
fn default_action(decision: &Decision, list: &[Action], steps_left: bool) -> Option<usize> {
    match decision {
        Decision::SelectPlace { .. } | Decision::SelectPosition { .. } | Decision::Sort { .. } => Some(0),
        Decision::SelectChain { forced: false, .. } if steps_left => list.iter().position(|a| a.kind == ActionKind::Pass),
        _ => None,
    }
}

/// Plays `steps` in order. A step matches the first action whose label
/// contains it (case-insensitive).
pub fn run_script(duel: &mut Duel, steps: &[&str], mut trace: impl FnMut(Trace)) -> Result<ScriptEnd, Error> {
    let mut pending = steps.iter().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).rev().collect::<Vec<_>>();
    loop {
        let (decision, events) = match duel.advance()? {
            Step::End { events } => {
                trace(Trace::Events(&events));
                return Ok(ScriptEnd::DuelEnded);
            }
            Step::Decision { decision, events } => (decision, events),
        };
        trace(Trace::Events(&events));
        let list = actions(&decision, duel.cards());
        let matched = pending.last().and_then(|step| list.iter().position(|a| a.label.to_lowercase().contains(step.as_str())));
        let index = match matched {
            Some(i) => {
                pending.pop();
                i
            }
            None => match default_action(&decision, &list, !pending.is_empty()) {
                Some(i) => i,
                None => {
                    return Ok(match pending.pop() {
                        Some(step) => ScriptEnd::NoMatch { step, decision, actions: list },
                        None => ScriptEnd::Finished { decision, actions: list },
                    })
                }
            },
        };
        trace(Trace::Chose(&list[index]));
        if list[index].kind == ActionKind::EndTurn {
            return Ok(ScriptEnd::TurnEnded);
        }
        duel.respond(&list[index].response);
    }
}
