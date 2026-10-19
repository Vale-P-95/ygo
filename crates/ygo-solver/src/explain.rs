//! A line played again from the start, told in Italian, with the state it
//! reaches: what the Combo Lab shows for a board.

use ygo_engine::{italian, DeckList, Duel, DuelState, Engine, Error, Event, Opponent, Step};

use crate::search::Line;

/// One step of the explanation: the choice made (`None` for what happens
/// before the first choice, such as the opening phases) and what followed,
/// in Italian.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplainedStep {
    pub choice: Option<String>,
    pub events: Vec<String>,
}

/// Plays `line` again (against the opponent it was found against) and
/// returns the state at the end and the steps.
pub fn explain(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: Option<&Opponent>,
    line: &Line,
) -> Result<(DuelState, Vec<ExplainedStep>), Error> {
    let mut duel = Duel::goldfish_against(engine, deck, hand, seed, opponent)?;
    let tell = |events: &[Event]| events.iter().filter_map(|e| italian::describe(engine.cards(), e)).collect::<Vec<_>>();
    let mut steps = vec![ExplainedStep::default()];
    let mut next_choice = 0;
    for (i, answer) in line.answers.iter().enumerate() {
        let Step::Decision { events, .. } = duel.advance()? else { break };
        steps.last_mut().expect("one step").events.extend(tell(&events));
        // A real choice starts a new step; automatic answers belong to the current one.
        if line.choice_at.get(next_choice) == Some(&i) {
            steps.push(ExplainedStep { choice: line.choices.get(next_choice).cloned(), events: Vec::new() });
            next_choice += 1;
        }
        duel.respond(answer);
    }
    let tail = match duel.advance()? {
        Step::Decision { events, .. } | Step::End { events } => events,
    };
    steps.last_mut().expect("one step").events.extend(tell(&tail));
    steps.retain(|s| s.choice.is_some() || !s.events.is_empty());
    Ok((duel.state(), steps))
}
