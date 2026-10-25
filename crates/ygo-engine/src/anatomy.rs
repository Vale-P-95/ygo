//! The anatomy of a card's effects, read from its script as the engine sets
//! it up: for each effect its type, the event it answers, where it is used
//! from, its once-per-turn limit, its properties (targets, delay), what it
//! declares it does (categories) and whether it has a cost, a condition and
//! a target step. A Lua wrapper around `Card.RegisterEffect`
//! (`lua/anatomy.lua`) reports each effect to the log of a reading duel, so
//! nothing of the engine changes. Plan: `docs/plans/anatomia-effetti.md`.

use crate::consts::{card_type, category, effect_code, effect_type, location, position};
use crate::duel::{Duel, DuelConfig, Engine};

const WRAPPER: &str = include_str!("../lua/anatomy.lua");

/// Cards set up in one reading duel: fewer duels, a bounded engine memory.
const CARDS_PER_DUEL: usize = 200;

/// What kind of effect it is, from its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    /// The activation of a Spell or Trap Card.
    Activation,
    Ignition,
    Trigger {
        optional: bool,
    },
    Quick {
        optional: bool,
    },
    /// A Flip effect (also a trigger, on the flip).
    Flip,
    /// Applies without being activated (an effect that changes the rules,
    /// a continuous trigger that does not start a chain).
    Continuous,
    /// How the card is Summoned, or what keeps it from being Summoned
    /// otherwise: the materials of a Fusion, Synchro, Xyz or Link Monster.
    Procedure,
    /// Given to the monster this card is equipped to, or to the Xyz Monster
    /// it is material of.
    Granted,
}

impl EffectKind {
    pub fn name(self) -> &'static str {
        match self {
            EffectKind::Activation => "activation",
            EffectKind::Ignition => "ignition",
            EffectKind::Trigger { optional: true } => "optional trigger",
            EffectKind::Trigger { optional: false } => "mandatory trigger",
            EffectKind::Quick { optional: true } => "quick",
            EffectKind::Quick { optional: false } => "mandatory quick",
            EffectKind::Flip => "flip",
            EffectKind::Continuous => "continuous",
            EffectKind::Procedure => "procedure",
            EffectKind::Granted => "granted",
        }
    }

    /// Activated, so it starts or joins a chain.
    pub fn activated(self) -> bool {
        matches!(
            self,
            EffectKind::Activation | EffectKind::Ignition | EffectKind::Trigger { .. } | EffectKind::Quick { .. } | EffectKind::Flip
        )
    }
}

/// A limit on how many times an effect is used (`SetCountLimit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CountLimit {
    pub count: u32,
    /// The card code the limit is shared through (0: this effect only).
    pub code: u32,
    /// `EFFECT_COUNT_CODE_*` flags: once per duel, per chain, "oath"…
    pub flag: u32,
    /// Which of the card's limits it is, when it has more than one.
    pub index: u32,
}

/// One effect as the script registered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectAnatomy {
    pub kind: EffectKind,
    /// `EFFECT_TYPE_*` bits.
    pub type_: u32,
    /// The event it answers (`EVENT_*`) for an activated effect, the effect
    /// code (`EFFECT_*`) for one that applies.
    pub code: u32,
    /// `CATEGORY_*` bits (see [`category::names`]).
    pub category: u64,
    pub property: [u32; 2],
    /// Where the card must be (`LOCATION_*`); 0 for its default place.
    pub range: u32,
    /// For an effect that applies to other cards: the locations on your
    /// side and on the opponent's.
    pub target_range: (u32, u32),
    pub count_limit: Option<CountLimit>,
    /// `aux.Stringid` of the effect: the same number the engine's
    /// activations carry ([`crate::Activatable::description`]).
    pub description: u64,
    /// The effect's value when it is a number (an ATK change, a code).
    pub value: i64,
    pub has_cost: bool,
    pub has_condition: bool,
    pub has_target: bool,
    pub has_operation: bool,
}

const FLAG_CARD_TARGET: u32 = 0x10;
const FLAG_DELAY: u32 = 0x1_0000;

impl EffectAnatomy {
    pub fn categories(&self) -> Vec<&'static str> {
        category::names(self.category)
    }

    pub fn targets(&self) -> bool {
        self.property[0] & FLAG_CARD_TARGET != 0
    }

    /// A trigger that does not miss the timing ("If" rather than "When").
    pub fn delayed(&self) -> bool {
        self.property[0] & FLAG_DELAY != 0
    }

    /// Both effects use up the same limit: using one turns the other off.
    pub fn shares_limit(&self, other: &EffectAnatomy) -> bool {
        match (self.count_limit, other.count_limit) {
            (Some(a), Some(b)) => a.code != 0 && a.code == b.code && a.index == b.index,
            _ => false,
        }
    }

    /// Declares that it negates or removes something, the kinds of play the
    /// probe turn's interruptions are made of.
    pub fn interrupts(&self) -> bool {
        self.category
            & (category::NEGATE | category::DISABLE | category::DISABLE_SUMMON | category::DESTROY | category::REMOVE | category::TODECK)
            != 0
    }
}

/// The effects of one card, in the order its script registers them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardAnatomy {
    pub code: u32,
    pub effects: Vec<EffectAnatomy>,
    /// What went wrong setting the card up (a Lua error, a missing script).
    pub error: Option<String>,
}

impl CardAnatomy {
    /// The effect behind an activation, from the description it carries; the
    /// first one when two effects share it (some scripts reuse a string).
    /// `None` when the description does not tell which effect it is.
    pub fn effect_of(&self, description: u64) -> Option<&EffectAnatomy> {
        // Some effects are described by a system string (1170, "Fusion
        // Summon"), so the exact match comes first whatever the number.
        let exact = self.effects.iter().find(|e| e.description == description && e.kind.activated());
        if exact.is_some() || description >> 20 != 0 {
            return exact;
        }
        // No description of the effect: a Spell activation without a string
        // (0), or an optional trigger the engine asks about with a system
        // string ("activate the effect of…?"). Certain only when the effects
        // it can be all have the same string (the same effect on several
        // events).
        let mut candidates = self.effects.iter().filter(|e| match description {
            0 => e.kind.activated() && e.description == 0,
            _ => matches!(e.kind, EffectKind::Trigger { optional: true } | EffectKind::Flip),
        });
        let first = candidates.next()?;
        candidates.all(|e| e.description == first.description).then_some(first)
    }
}

fn kind(type_: u32, code: u32) -> EffectKind {
    use effect_type as t;
    if matches!(
        code,
        effect_code::SPSUMMON_CONDITION | effect_code::REVIVE_LIMIT | effect_code::SPSUMMON_PROC | effect_code::FUSION_MATERIAL
    ) || code == effect_code::SPSUMMON_PROC_G
    {
        return EffectKind::Procedure;
    }
    if type_ & (t::EQUIP | t::XMATERIAL | t::GRANT) != 0
        && type_ & (t::IGNITION | t::TRIGGER_O | t::TRIGGER_F | t::QUICK_O | t::QUICK_F) == 0
    {
        return EffectKind::Granted;
    }
    match () {
        _ if type_ & t::ACTIVATE != 0 => EffectKind::Activation,
        _ if type_ & t::FLIP != 0 && type_ & t::CONTINUOUS == 0 => EffectKind::Flip,
        _ if type_ & t::IGNITION != 0 => EffectKind::Ignition,
        _ if type_ & t::QUICK_O != 0 => EffectKind::Quick { optional: true },
        _ if type_ & t::QUICK_F != 0 => EffectKind::Quick { optional: false },
        _ if type_ & t::TRIGGER_O != 0 => EffectKind::Trigger { optional: true },
        _ if type_ & t::TRIGGER_F != 0 => EffectKind::Trigger { optional: false },
        _ => EffectKind::Continuous,
    }
}

/// One line of the wrapper: the card's code and its effect.
pub(crate) fn parse(line: &str) -> Option<(u32, EffectAnatomy)> {
    let mut fields = line.strip_prefix("dfa ")?.split(' ').map(|f| f.parse::<i64>());
    let mut next = || fields.next()?.ok();
    let code = next()? as u32;
    let type_ = next()? as u32;
    let effect = next()? as u32;
    let category = next()? as u64;
    let property = [next()? as u32, next()? as u32];
    let range = next()? as u32;
    let target_range = (next()? as u32, next()? as u32);
    let (count, limit_code, flag, index) = (next()? as u32, next()? as u32, next()? as u32, next()? as u32);
    let description = next()? as u64;
    let value = next()?;
    let [has_cost, has_condition, has_target, has_operation] = [next()? != 0, next()? != 0, next()? != 0, next()? != 0];
    let count_limit = (count > 0).then_some(CountLimit { count, code: limit_code, flag, index });
    Some((
        code,
        EffectAnatomy {
            kind: kind(type_, effect),
            type_,
            code: effect,
            category,
            property,
            range,
            target_range,
            count_limit,
            description,
            value,
            has_cost,
            has_condition,
            has_target,
            has_operation,
        },
    ))
}

/// The anatomy of each card of `codes`, in that order. Cards without a
/// script (Normal Monsters) have no effects and no error.
pub fn read_anatomy(engine: &Engine, codes: &[u32]) -> Vec<CardAnatomy> {
    let mut out = Vec::with_capacity(codes.len());
    for chunk in codes.chunks(CARDS_PER_DUEL) {
        match reading_duel(engine) {
            Ok(mut duel) => out.extend(chunk.iter().map(|&code| read_card(engine, &mut duel, code))),
            Err(e) => out.extend(chunk.iter().map(|&code| CardAnatomy { code, effects: Vec::new(), error: Some(e.to_string()) })),
        }
    }
    out
}

/// A duel that is never started nor reused, with the wrapper loaded.
fn reading_duel(engine: &Engine) -> Result<Duel, crate::Error> {
    let duel = Duel::fresh(engine, &DuelConfig::default(), std::ptr::null_mut())?;
    if !duel.run_lua_source("anatomy.lua", WRAPPER) {
        return Err(crate::Error::Engine(format!("cannot load anatomy.lua: {}", duel.log().join("; "))));
    }
    duel.take_log();
    Ok(duel)
}

fn read_card(engine: &Engine, duel: &mut Duel, code: u32) -> CardAnatomy {
    let extra = engine.cards().get(code).is_some_and(|c| c.type_ & card_type::EXTRA != 0);
    let place = if extra { location::EXTRA } else { location::DECK };
    let missing = duel.missing_scripts().len();
    duel.add_card(0, code, place, 0, position::FACEDOWN_DEFENSE);
    let mut effects = Vec::new();
    let mut errors = Vec::new();
    let log = duel.take_log();
    let full = log.len() >= crate::duel::LOG_LINES;
    for line in log {
        match parse(&line) {
            Some((owner, effect)) if owner == code => effects.push(effect),
            // An effect a script registers on another card it creates.
            Some(_) => {}
            None => errors.push(line),
        }
    }
    if full {
        errors.push("too many effects: the log was full, some are missing".to_owned());
    }
    let normal = engine.cards().get(code).is_some_and(|c| c.type_ & card_type::NORMAL != 0 && c.type_ & card_type::PENDULUM == 0);
    if duel.missing_scripts().len() > missing && !normal {
        errors.push(format!("no script c{code}.lua"));
    }
    CardAnatomy { code, effects, error: (!errors.is_empty()).then(|| errors.join("; ")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_wrapper_line() {
        // A Fusion Summon trigger that banishes, once per turn by name.
        let (code, e) = parse("dfa 55 641 1102 4 65552 0 4 0 0 1 55 0 0 880803840 0 0 1 1 1").unwrap();
        assert_eq!(code, 55);
        assert_eq!(e.kind, EffectKind::Trigger { optional: true });
        assert_eq!(e.categories(), ["banish"]);
        assert!(e.targets() && e.delayed());
        assert_eq!(e.count_limit, Some(CountLimit { count: 1, code: 55, flag: 0, index: 0 }));
        assert!(!e.has_cost && e.has_condition && e.has_target && e.has_operation);
        assert!(parse("some Lua error").is_none());
    }

    #[test]
    fn kinds_from_types() {
        assert_eq!(kind(0x10, 1002), EffectKind::Activation);
        assert_eq!(kind(0x1 | 0x100, 1027), EffectKind::Quick { optional: true });
        assert_eq!(kind(0x1, effect_code::FUSION_MATERIAL), EffectKind::Procedure);
        assert_eq!(kind(0x2, effect_code::SPSUMMON_PROC), EffectKind::Procedure);
        assert_eq!(kind(0x2 | 0x800, 1102), EffectKind::Continuous);
        assert_eq!(kind(0x4, 100), EffectKind::Granted);
    }

    #[test]
    fn shared_limits() {
        let (_, a) = parse("dfa 9 641 1102 4 0 0 4 0 0 1 9 0 0 0 0 0 0 0 1").unwrap();
        let (_, mut b) = parse("dfa 9 257 1002 0 0 0 4 0 0 1 9 0 0 0 0 0 0 0 1").unwrap();
        assert!(a.shares_limit(&b));
        b.count_limit = Some(CountLimit { index: 1, ..b.count_limit.unwrap() });
        assert!(!a.shares_limit(&b));
    }
}
