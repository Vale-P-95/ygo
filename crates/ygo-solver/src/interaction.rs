//! What a board stops at the end of the turn, by kind and place, with
//! nothing about the opponent's turn after it (docs/plans/interazione-fine-turno.md).

use ygo_engine::consts::location;

use crate::probe::{Effect, Measure};

/// What a board stops at the end of the turn: the interruptions (field, GY,
/// banished and hand) and the floodgates, all in interruptions.
pub fn interaction_value(m: &Measure) -> f64 {
    m.field_value + m.hand_value + m.floodgate_value
}

/// A measured board's interaction at the end of the turn, by kind: the
/// times its effects stop something, by what they do and where they are,
/// the effects the engine let you use on the opponent's turn, the
/// floodgates and the cards left in hand and in the GY.
#[derive(Debug, Default, PartialEq)]
pub struct InteractionProfile {
    pub value: f64,
    /// Times your effects stop something: negations (of an activation, a
    /// summon or an effect), removals (banish, to the Deck, to the hand,
    /// destroy, to the GY) and the rest (locks, position or control).
    pub negations: u32,
    pub removals: u32,
    pub others: u32,
    /// The same times, by where the card is: field, GY, banished, hand.
    pub by_place: [u32; 4],
    /// Effects that stop something alone but nothing with your others,
    /// which need what they use (Invoked Mechaba sends the last card of
    /// your hand, Ash Blossom): not counted above.
    pub shared: usize,
    /// Cards you can use on the opponent's turn, stopping something or not.
    pub activatable: usize,
    pub floodgates: usize,
    pub hand: usize,
    pub grave: usize,
}

impl InteractionProfile {
    /// `hand` and `grave` are the sizes of the player's hand and Graveyard on the board.
    pub fn of(m: &Measure, hand: usize, grave: usize) -> Self {
        let mut p = InteractionProfile {
            value: interaction_value(m),
            activatable: m.offered.len(),
            floodgates: m.floodgates.len(),
            hand,
            grave,
            ..Default::default()
        };
        for i in &m.interruptions {
            if i.value <= 0.0 {
                p.shared += usize::from(i.alone > 0.0);
                continue;
            }
            let times = i.uses.max(1);
            match i.effect {
                Effect::NegateAndRemove | Effect::NegateSummon | Effect::NegateActivation | Effect::NegateEffect => p.negations += times,
                Effect::Banish | Effect::ToDeck | Effect::Bounce | Effect::Destroy | Effect::ToGrave => p.removals += times,
                Effect::Lock | Effect::Position => p.others += times,
            }
            let place = match i.location {
                location::HAND => 3,
                location::GRAVE => 1,
                location::REMOVED => 2,
                _ => 0,
            };
            p.by_place[place] += times;
        }
        p
    }

    pub fn total(&self) -> u32 {
        self.negations + self.removals + self.others
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::{Interruption, Probe, ResponderId};

    #[test]
    fn counts_by_kind_and_place() {
        let stop = |location, effect, uses, value| Interruption {
            responder: ResponderId { code: 1, description: 0 },
            location,
            effect,
            targeted: false,
            uses,
            covers: vec![],
            coverage: 0.0,
            value,
            alone: 0.5,
        };
        let m = Measure {
            interruptions: vec![
                stop(location::MZONE, Effect::NegateActivation, 2, 0.5),
                stop(location::HAND, Effect::NegateEffect, 1, 0.5),
                stop(location::GRAVE, Effect::Banish, 0, 0.5),
                stop(location::REMOVED, Effect::Lock, 1, 0.5),
                stop(location::HAND, Effect::NegateEffect, 1, 0.0),
            ],
            floodgates: vec![Probe::ExtraSummon],
            field_value: 1.5,
            hand_value: 0.8,
            floodgate_value: 0.6,
            offered: vec![(1, location::MZONE), (2, location::HAND)],
            ..Default::default()
        };
        let p = InteractionProfile::of(&m, 2, 3);
        // Each response counts its uses, and once at least.
        assert_eq!((p.negations, p.removals, p.others, p.total()), (3, 1, 1, 5));
        assert_eq!((p.by_place, p.shared), ([2, 1, 1, 1], 1));
        assert_eq!((p.activatable, p.floodgates, p.hand, p.grave), (2, 1, 2, 3));
        assert!((p.value - 2.9).abs() < 1e-9);
    }
}
