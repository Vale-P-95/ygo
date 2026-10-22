//! A line played again from the start, told in Italian, with the state it
//! reaches: what the Combo Lab shows for a board.

use ygo_engine::consts::{location, reason};
use ygo_engine::{italian, ActionKind, CardDb, Decision, DeckList, Duel, DuelState, Engine, Error, Event, Opponent, Step};

use crate::search::Line;

/// One step of the explanation: the choice made (`None` for what happens
/// before the first choice, such as the opening phases) and what followed,
/// in Italian, and the same for a picture: what kind of choice, on which
/// card, what happened to which cards and the state it left.
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
    /// The state once the step is done: when the next choice is asked, or
    /// at the end of the line.
    pub state: Option<DuelState>,
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
            steps.last_mut().expect("one step").state = Some(duel.state());
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
    let state = duel.state();
    let last = steps.last_mut().expect("one step");
    tell(last, &tail);
    last.state = Some(state.clone());
    steps.retain(|s| s.choice.is_some() || !s.events.is_empty());
    Ok((state, steps))
}

/// Where the handtraps played most can stop a line while you play it,
/// whatever the board at the end: two lines to the same board are not
/// worth the same if one stops at the first Ash Blossom.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Exposure {
    /// Your effects that add a card from the Deck to the hand, Special
    /// Summon from the Deck or send from the Deck to the GY (Ash Blossom &
    /// Joyous Spring negates them).
    pub ash: u32,
    /// Your monster effects activated on the field (Effect Veiler,
    /// Infinite Impermanence, Ghost Ogre & Snow Rabbit).
    pub on_field: u32,
    /// Your summons: from the fifth, Nibiru, the Primal Being.
    pub summons: u32,
    /// Your Special Summons (monsters summoned together count once): a
    /// card each for Maxx "C".
    pub special_summons: u32,
    /// Cards added from the Deck to your hand, drawn by an effect too:
    /// after the first, Droll & Lock Bird stops the rest.
    pub searches: u32,
    /// Cards of your Deck sent to the GY or banished as a cost (Pot of
    /// Desires banishes ten): what the line spends besides the hand.
    pub deck_cost: u32,
    /// Life Points paid as a cost.
    pub lp_paid: u32,
}

impl Exposure {
    /// The moments a handtrap stops something: each effect Ash or a
    /// negation on the field can hit, Nibiru once 5 summons are reached,
    /// Droll once a second card would be added.
    pub fn choke_points(&self) -> u32 {
        self.ash + self.on_field + u32::from(self.summons >= 5) + u32::from(self.searches >= 2)
    }
}

/// The kind of handtrap that stops a line at a choke point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChokeKind {
    /// An effect that takes a card from the Deck (Ash Blossom & Joyous Spring).
    DeckTouch,
    /// A monster effect activated on the field (Effect Veiler, Infinite
    /// Impermanence, Ghost Ogre & Snow Rabbit).
    FieldEffect,
    /// The fifth summon (Nibiru, the Primal Being).
    FifthSummon,
    /// The second card added from the Deck (Droll & Lock Bird).
    SecondSearch,
}

/// A moment of a line where a handtrap stops it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChokePoint {
    pub kind: ChokeKind,
    /// The line's answer given at your free moment where what is stopped
    /// began: played up to it (not included), the turn can end there.
    pub answer: usize,
}

/// Plays `line` again and counts its [`Exposure`].
pub fn exposure(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: Option<&Opponent>,
    line: &Line,
) -> Result<Exposure, Error> {
    exposure_points(engine, deck, hand, seed, opponent, line).map(|(x, _)| x)
}

/// [`exposure`], and where each choke point is in the line.
pub fn exposure_points(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: Option<&Opponent>,
    line: &Line,
) -> Result<(Exposure, Vec<ChokePoint>), Error> {
    let mut duel = Duel::goldfish_against(engine, deck, hand, seed, opponent)?;
    let mut out = Exposure::default();
    // Chain links: yours, and already counted for Ash; the link resolving.
    let mut links: std::collections::HashMap<u32, (bool, bool)> = std::collections::HashMap::new();
    let mut solving: Vec<u32> = Vec::new();
    // Who is Special Summoning (the end of a summon does not say).
    let mut summoning_mine = false;
    let mut count = |events: &[Event], out: &mut Exposure| {
        // A link of yours resolving whose effect takes a card from the Deck: one point for Ash Blossom.
        let from_deck = |solving: &[u32], links: &mut std::collections::HashMap<u32, (bool, bool)>, out: &mut Exposure| {
            if let Some(link) = solving.last().and_then(|l| links.get_mut(l)) {
                if link.0 && !link.1 {
                    link.1 = true;
                    out.ash += 1;
                }
            }
        };
        for e in events {
            match e {
                Event::Chaining { card, chain_link, .. } => {
                    let mine = card.controller == 0;
                    links.insert(*chain_link, (mine, false));
                    if mine && card.location == location::MZONE {
                        out.on_field += 1;
                    }
                }
                Event::ChainSolving { chain_link } => solving.push(*chain_link as u32),
                Event::ChainSolved { .. } => {
                    solving.pop();
                }
                Event::ChainEnd => {
                    links.clear();
                    solving.clear();
                }
                Event::Move { from, to, reason: why, .. } if from.location == location::DECK && to.controller == 0 => {
                    if why & reason::COST != 0 && matches!(to.location, location::GRAVE | location::REMOVED) {
                        out.deck_cost += 1;
                    }
                    if to.location == location::HAND {
                        out.searches += 1;
                    }
                    if matches!(to.location, location::HAND | location::MZONE | location::GRAVE) {
                        from_deck(&solving, &mut links, out);
                    }
                }
                // Drawn by an effect (a chain link resolving): Ash Blossom
                // negates it, Droll & Lock Bird stops it like a search.
                Event::Draw { player: 0, codes } if !solving.is_empty() => {
                    out.searches += codes.len() as u32;
                    from_deck(&solving, &mut links, out);
                }
                Event::NormalSummoning { card } | Event::FlipSummoning { card } if card.controller == 0 => out.summons += 1,
                Event::SpecialSummoning { card } => {
                    summoning_mine = card.controller == 0;
                    out.summons += u32::from(summoning_mine);
                }
                Event::SpecialSummoned if summoning_mine => out.special_summons += 1,
                Event::PayLpCost { player: 0, amount } => out.lp_paid += amount,
                _ => {}
            }
        }
    };
    // The choke points: what each count reached after the events of a
    // step, against what it was before, at the last free moment.
    let mut points = Vec::new();
    let mut idle: Option<usize> = None;
    let note = |before: &Exposure, after: &Exposure, idle: Option<usize>, points: &mut Vec<ChokePoint>| {
        let Some(answer) = idle else { return };
        let mut add = |kind, n: u32| points.extend((0..n).map(|_| ChokePoint { kind, answer }));
        add(ChokeKind::DeckTouch, after.ash - before.ash);
        add(ChokeKind::FieldEffect, after.on_field - before.on_field);
        add(ChokeKind::FifthSummon, u32::from(before.summons < 5 && after.summons >= 5));
        add(ChokeKind::SecondSearch, u32::from(before.searches < 2 && after.searches >= 2));
    };
    for (i, answer) in line.answers.iter().enumerate() {
        let Step::Decision { decision, events } = duel.advance()? else { break };
        let before = out;
        count(&events, &mut out);
        note(&before, &out, idle, &mut points);
        if matches!(decision, Decision::IdleCmd { player: 0, .. }) {
            idle = Some(i);
        }
        duel.respond(answer);
    }
    let (Step::Decision { events, .. } | Step::End { events }) = duel.advance()?;
    let before = out;
    count(&events, &mut out);
    note(&before, &out, idle, &mut points);
    Ok((out, points))
}
