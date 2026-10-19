//! A line played again from the start, told in Italian, with the state it
//! reaches: what the Combo Lab shows for a board.

use ygo_engine::consts::location;
use ygo_engine::{italian, ActionKind, CardDb, DeckList, Duel, DuelState, Engine, Error, Event, Opponent, Step};

use crate::search::Line;

/// One step of the explanation: the choice made (`None` for what happens
/// before the first choice, such as the opening phases) and what followed,
/// in Italian, and the same for a picture: what kind of choice, on which
/// card, and what happened to which cards.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplainedStep {
    pub choice: Option<String>,
    pub events: Vec<String>,
    /// The kind of the choice made.
    pub kind: Option<ActionKind>,
    /// The card the choice is about, when it is about one.
    pub card: Option<u32>,
    /// What followed, one entry per event told in `events`.
    pub happened: Vec<Happening>,
}

/// Something that happened to a card, for a picture of the step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Happening {
    pub kind: HappeningKind,
    /// The card (0 when none, such as a new phase).
    pub code: u32,
    /// The chain link, for an activation or a negation.
    pub chain_link: u32,
    /// Yours (not the opponent's).
    pub mine: bool,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HappeningKind {
    /// A new turn or phase.
    Phase,
    Draw,
    Summon,
    Activate,
    Negate,
    Set,
    ToHand,
    ToGrave,
    Banish,
    ToDeck,
    ToExtra,
    /// Attached as an Xyz Material.
    Material,
    /// Moved to or within the field.
    ToField,
    Other,
}

fn happening(db: &CardDb, event: &Event) -> Option<Happening> {
    let text = italian::describe(db, event)?;
    let (kind, code, chain_link, mine) = match event {
        Event::NewTurn { .. } | Event::NewPhase { .. } => (HappeningKind::Phase, 0, 0, true),
        Event::Draw { player, codes } => {
            (HappeningKind::Draw, if *player == 0 { codes.first().copied().unwrap_or(0) } else { 0 }, 0, *player == 0)
        }
        Event::Move { code, to, .. } => {
            let kind = match to.location {
                location::HAND => HappeningKind::ToHand,
                location::GRAVE => HappeningKind::ToGrave,
                location::REMOVED => HappeningKind::Banish,
                location::DECK => HappeningKind::ToDeck,
                location::EXTRA => HappeningKind::ToExtra,
                location::MZONE | location::SZONE => HappeningKind::ToField,
                l if l & location::OVERLAY != 0 => HappeningKind::Material,
                _ => HappeningKind::Other,
            };
            (kind, *code, 0, to.controller == 0)
        }
        Event::NormalSummoning { card } | Event::SpecialSummoning { card } | Event::FlipSummoning { card } => {
            (HappeningKind::Summon, card.code, 0, card.controller == 0)
        }
        Event::Set { card } => (HappeningKind::Set, card.code, 0, card.controller == 0),
        Event::Chaining { card, chain_link, .. } => (HappeningKind::Activate, card.code, *chain_link, card.controller == 0),
        Event::ChainNegated { chain_link } | Event::ChainDisabled { chain_link } => (HappeningKind::Negate, 0, *chain_link as u32, true),
        _ => (HappeningKind::Other, 0, 0, true),
    };
    Some(Happening { kind, code, chain_link, mine, text })
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
    let db = engine.cards();
    let tell = |step: &mut ExplainedStep, events: &[Event]| {
        for h in events.iter().filter_map(|e| happening(db, e)) {
            step.events.push(h.text.clone());
            step.happened.push(h);
        }
    };
    let mut steps = vec![ExplainedStep::default()];
    let mut next_choice = 0;
    for (i, answer) in line.answers.iter().enumerate() {
        let Step::Decision { decision, events } = duel.advance()? else { break };
        tell(steps.last_mut().expect("one step"), &events);
        // A real choice starts a new step; automatic answers belong to the current one.
        if line.choice_at.get(next_choice) == Some(&i) {
            let action = duel.actions(&decision).into_iter().find(|a| a.response == *answer);
            steps.push(ExplainedStep {
                choice: line.choices.get(next_choice).cloned(),
                kind: action.as_ref().map(|a| a.kind),
                card: action.and_then(|a| a.card).map(|c| c.code),
                ..ExplainedStep::default()
            });
            next_choice += 1;
        }
        duel.respond(answer);
    }
    let tail = match duel.advance()? {
        Step::Decision { events, .. } | Step::End { events } => events,
    };
    tell(steps.last_mut().expect("one step"), &tail);
    steps.retain(|s| s.choice.is_some() || !s.events.is_empty());
    Ok((duel.state(), steps))
}
