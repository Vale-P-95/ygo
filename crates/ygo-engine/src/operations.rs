//! What activated effects declare they will do to the opponent's cards,
//! read while a duel is played: `lua/operations.lua` wraps
//! `Duel.SetOperationInfo` and reports each declaration once per duel and
//! effect, and the log handler keeps those lines apart from the log
//! ([`crate::Duel::take_operations`]). The static anatomy sees "banishes";
//! this sees how many cards and from where (Hyperinvoked Aeon's trigger:
//! 3 cards of the opponent's Extra Deck), plan `docs/plans/anatomia-effetti.md` §4.2.

use crate::consts::{effect_type, event};

pub(crate) const SCRIPT: &str = include_str!("../lua/operations.lua");

/// Lines of the operations kept for each duel, as for the log.
pub(crate) const OPERATION_LINES: usize = 256;

/// One declaration of an effect being activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Operation {
    /// The original code of the card the effect belongs to.
    pub code: u32,
    pub description: u64,
    /// `EFFECT_TYPE_*` bits.
    pub effect_type: u32,
    /// The event a trigger answers (`EVENT_*`).
    pub event: u32,
    /// One `CATEGORY_*` bit.
    pub category: u64,
    /// The opponent's cards it is declared to affect.
    pub count: u32,
    /// Where those cards are (`LOCATION_*` bits).
    pub location: u32,
    /// Whether the effect targets.
    pub targets: bool,
}

impl Operation {
    /// A line of `operations.lua` ("dfo code description type event
    /// category count location target").
    pub(crate) fn parse(line: &str) -> Option<Self> {
        let mut f = line.strip_prefix("dfo ")?.split(' ');
        let mut next = || f.next()?.parse::<i64>().ok();
        let code = next()? as u32;
        let description = next()? as u64;
        let effect_type = next()? as u32;
        let event = next()? as u32;
        let category = next()? as u64;
        let count = next()? as u32;
        let location = next()? as u32;
        let targets = next()? != 0;
        Some(Operation { code, description, effect_type, event, category, count, location, targets })
    }

    /// A trigger of the card's own Summon: what the card does arriving.
    pub fn on_arrival(&self) -> bool {
        self.effect_type & effect_type::SINGLE != 0
            && self.effect_type & (effect_type::TRIGGER_O | effect_type::TRIGGER_F) != 0
            && matches!(self.event, event::SUMMON_SUCCESS | event::FLIP_SUMMON_SUCCESS | event::SPSUMMON_SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_read_back() {
        let op = Operation::parse("dfo 7473 1195568 129 1102 4 3 64 0").unwrap();
        assert_eq!((op.code, op.category, op.count, op.location, op.targets), (7473, 4, 3, 64, false));
        assert!(op.on_arrival());
        assert!(Operation::parse("dfa 1 2 3").is_none());
        // A quick effect is not a trigger of the card's Summon.
        assert!(!Operation::parse("dfo 7473 1 2 1002 4 3 64 16").unwrap().on_arrival());
    }
}
