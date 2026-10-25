//! The dry check of a board: for each effect of your cards that could be
//! used on the opponent's turn, whether your board lets you use it, with no
//! opponent playing. A Lua script (`lua/dry_check.lua`) asks each effect's
//! condition, cost and target "can it be done?" (`chk == 0`, which changes
//! nothing); questions about your side are answered by the duel, those about
//! the opponent's side or the triggering event are answered yes and kept as
//! assumptions. Plan: `docs/plans/linee-interazione.md` §3.2.

use crate::anatomy::{parse, EffectAnatomy};
use crate::duel::Duel;

const SCRIPT: &str = include_str!("../lua/dry_check.lua");

/// Why an effect cannot be used on the opponent's turn, from your board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blocked {
    /// A face-down monster's effect.
    FaceDown,
    /// The card's effects are negated.
    Negated,
    /// Its limit is used up (once per duel, or a limit that lasts).
    Limit,
    Condition,
    /// The cost cannot be paid: no card to discard, no material to detach…
    Cost,
    /// No card of yours to target where the effect needs one.
    Target,
}

impl Blocked {
    fn of(reason: u32) -> Option<Self> {
        Some(match reason {
            2 => Blocked::FaceDown,
            3 => Blocked::Negated,
            4 => Blocked::Limit,
            5 => Blocked::Condition,
            6 => Blocked::Cost,
            7 => Blocked::Target,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Blocked::FaceDown => "face-down",
            Blocked::Negated => "negated",
            Blocked::Limit => "limit used",
            Blocked::Condition => "condition",
            Blocked::Cost => "cost",
            Blocked::Target => "target",
        }
    }
}

/// What a usable effect was assumed to have (bits of [`DryEffect::assumes`]).
pub mod assume {
    /// Cards on the opponent's side (a target, a count).
    pub const OPPONENT: u32 = 0x1;
    /// The event that triggers it: an activation, a summon, an attack.
    pub const EVENT: u32 = 0x2;
    /// A phase of the opponent's turn other than the Main Phase.
    pub const PHASE: u32 = 0x4;
    /// A Lua error in a check: the check could not tell.
    pub const ERROR: u32 = 0x8;
}

/// Cards a cost uses: `count` of them, among `from` (field ids).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demand {
    pub count: u32,
    pub from: Vec<u32>,
}

/// One effect of one of your cards, as the dry check found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DryEffect {
    /// The card's identity in this duel (`Card.GetFieldID`).
    pub field_id: u32,
    pub code: u32,
    pub location: u8,
    pub sequence: u8,
    pub effect: EffectAnatomy,
    /// `None`: usable on the opponent's turn, maybe with assumptions.
    pub blocked: Option<Blocked>,
    /// [`assume`] bits.
    pub assumes: u32,
    /// The cost uses the card itself (a handtrap discarded, a card banished from the GY).
    pub self_cost: bool,
    pub demands: Vec<Demand>,
}

/// The effects of `player`'s cards in the hand, on the field, in the GY
/// and banished that could be used on the opponent's turn or apply then,
/// checked on `duel` as it is. The duel's Lua functions are put back after,
/// but the check runs code of the scripts: use a duel that is not played on.
pub fn dry_check(duel: &mut Duel, player: u8) -> Result<Vec<DryEffect>, crate::Error> {
    duel.take_log();
    let source = format!("local df_tp={player}\n{SCRIPT}");
    if !duel.run_lua_source("dry_check.lua", &source) {
        return Err(crate::Error::Engine(format!("cannot run dry_check.lua: {}", duel.take_log().join("; "))));
    }
    let log = duel.take_log();
    if log.len() >= crate::duel::LOG_LINES {
        return Err(crate::Error::Engine("dry check: the log was full".to_owned()));
    }
    let mut out = Vec::new();
    let mut ended = false;
    for line in log.iter().flat_map(|l| l.split('\n')) {
        if line == "dfd end" {
            ended = true;
        } else if let Some(err) = line.strip_prefix("dfd error ") {
            return Err(crate::Error::Engine(format!("dry check: {err}")));
        } else if let Some(effect) = parse_line(line) {
            out.push(effect);
        }
    }
    if !ended {
        return Err(crate::Error::Engine(format!("dry check did not end: {}", log.join("; "))));
    }
    Ok(out)
}

fn parse_line(line: &str) -> Option<DryEffect> {
    let (head, anatomy) = line.strip_prefix("dfd ")?.split_once(" | ")?;
    let (code, effect) = parse(anatomy)?;
    let mut fields = head.split(' ');
    let mut number = || fields.next()?.parse::<u32>().ok();
    let (field_id, location, sequence, reason, flags) = (number()?, number()?, number()?, number()?, number()?);
    let demands = match head.split(' ').nth(5)? {
        "-" => Vec::new(),
        list => list
            .split(';')
            .map(|d| {
                let (count, from) = d.split_once(':')?;
                let from = from.split(',').filter(|f| !f.is_empty()).map(|f| f.parse().ok()).collect::<Option<Vec<u32>>>()?;
                Some(Demand { count: count.parse().ok()?, from })
            })
            .collect::<Option<Vec<_>>>()?,
    };
    Some(DryEffect {
        field_id,
        code,
        location: location as u8,
        sequence: sequence as u8,
        effect,
        blocked: Blocked::of(reason),
        assumes: flags & 0xf,
        self_cost: flags & 0x10 != 0,
        demands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_line() {
        // Ash Blossom in the hand: usable if an effect is activated, discarding itself.
        let line = "dfd 12 2 0 0 18 1:12 | dfa 14558127 256 1027 16384 0 0 2 0 0 1 14558127 0 0 0 0 1 1 1 1";
        let e = parse_line(line).unwrap();
        assert_eq!((e.field_id, e.code, e.location, e.blocked), (12, 14558127, 2, None));
        assert_eq!(e.assumes, assume::EVENT);
        assert!(e.self_cost);
        assert_eq!(e.demands, [Demand { count: 1, from: vec![12] }]);
        let blocked = parse_line("dfd 3 4 1 6 0 - | dfa 9 257 1002 1 0 0 4 0 0 1 9 0 0 0 0 1 0 1 1").unwrap();
        assert_eq!(blocked.blocked, Some(Blocked::Cost));
        assert!(blocked.demands.is_empty());
        assert!(parse_line("dfd end").is_none());
    }
}
