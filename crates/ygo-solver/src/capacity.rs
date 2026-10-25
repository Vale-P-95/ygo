//! What a board can stop on the opponent's turn, read from your cards'
//! effects with no opponent playing (docs/plans/linee-interazione.md §2-§3):
//! the effects that could be used then and do something to the opponent,
//! those your board lets you use (the dry check), and how many of them you
//! can use in the same turn, with the limits they share and the cards their
//! costs need (the capacity).

use std::collections::HashMap;

use ygo_engine::consts::{event, location};
use ygo_engine::{assume, dry_check, Blocked, Demand, DryEffect, Duel, EffectKind, Engine, Error, Step};

use crate::probe::Effect;
use crate::search::{Line, SearchConfig};

/// What an interruption does to the opponent, from the categories its
/// script declares, best first (the scale of the probe turn's [`Effect`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    NegateSummon,
    NegateActivation,
    Banish,
    ToDeck,
    NegateEffect,
    Bounce,
    Destroy,
    Discard,
    ToGrave,
    Control,
    Position,
}

impl Class {
    pub const ALL: [Class; 11] = [
        Class::NegateSummon,
        Class::NegateActivation,
        Class::Banish,
        Class::ToDeck,
        Class::NegateEffect,
        Class::Bounce,
        Class::Destroy,
        Class::Discard,
        Class::ToGrave,
        Class::Control,
        Class::Position,
    ];

    /// The probe turn's values: the same scale, not its measure.
    pub fn quality(self) -> f64 {
        match self {
            Class::NegateSummon => Effect::NegateSummon.value(),
            Class::NegateActivation => Effect::NegateActivation.value(),
            Class::Banish => Effect::Banish.value(),
            Class::ToDeck => Effect::ToDeck.value(),
            Class::NegateEffect => Effect::NegateEffect.value(),
            Class::Bounce => Effect::Bounce.value(),
            // A card out of the hand or taken weighs as one destroyed (B5a for control).
            Class::Destroy | Class::Discard | Class::Control => Effect::Destroy.value(),
            Class::ToGrave => Effect::ToGrave.value(),
            Class::Position => Effect::Position.value(),
        }
    }

    pub fn is_negation(self) -> bool {
        matches!(self, Class::NegateSummon | Class::NegateActivation | Class::NegateEffect)
    }

    pub fn label(self) -> &'static str {
        match self {
            Class::NegateSummon => "negate summon",
            Class::NegateActivation => "negate activation",
            Class::Banish => "banish",
            Class::ToDeck => "to deck",
            Class::NegateEffect => "negate effect",
            Class::Bounce => "to hand",
            Class::Destroy => "destroy",
            Class::Discard => "discard",
            Class::ToGrave => "to GY",
            Class::Control => "control",
            Class::Position => "position",
        }
    }
}

/// `CATEGORY_*` bits.
mod cat {
    pub const DESTROY: u64 = 0x1;
    pub const REMOVE: u64 = 0x4;
    pub const TOHAND: u64 = 0x8;
    pub const TODECK: u64 = 0x10;
    pub const TOGRAVE: u64 = 0x20;
    pub const HANDES: u64 = 0x80;
    pub const POSITION: u64 = 0x1000;
    pub const CONTROL: u64 = 0x2000;
    pub const DISABLE: u64 = 0x4000;
    pub const DISABLE_SUMMON: u64 = 0x8000;
    pub const NEGATE: u64 = 0x1000_0000;
}

/// Effect codes that keep the opponent from doing something, when a
/// continuous effect applies them to the opponent's side: negated effects,
/// no activations, no summons or Sets, no draws, cards that cannot move, no
/// attacks or Battle Phase, cards sent elsewhere instead of the GY.
const FLOODGATE_CODES: [u32; 18] = [2, 6, 20, 21, 22, 23, 24, 25, 63, 65, 66, 67, 68, 85, 86, 185, 186, 260];

/// Events the opponent's plays cause: an effect of yours that waits for one
/// answers the opponent.
fn answers_opponent(code: u32) -> bool {
    matches!(
        code,
        event::CHAINING
            | event::SUMMON_SUCCESS
            | event::FLIP_SUMMON_SUCCESS
            | event::SPSUMMON_SUCCESS
            | event::SUMMON
            | event::SPSUMMON
            | event::ATTACK_ANNOUNCE
            | event::TO_HAND
    ) || code == EVENT_FLIP_SUMMON
        || code == EVENT_BE_BATTLE_TARGET
        || code & event::PHASE != 0
}

const EVENT_FLIP_SUMMON: u32 = 1104;
const EVENT_BE_BATTLE_TARGET: u32 = 1131;

/// The usual number of uses of an effect without a limit and with no cost
/// that runs out first (the probe turn's rounds).
const MAX_USES: u32 = 3;

/// Interruptions of one board beyond which the capacity is found greedily.
const EXACT_UP_TO: usize = 24;

/// One effect of yours that does something to the opponent on its turn.
#[derive(Debug, Clone, PartialEq)]
pub struct Interruption {
    pub code: u32,
    pub field_id: u32,
    pub location: u8,
    pub class: Class,
    pub kind: EffectKind,
    pub description: u64,
    /// Times it can be used in the opponent's turn on its own.
    pub uses: u32,
    /// The limit it shares with copies and other effects of the same name.
    pub group: Option<(u32, u32)>,
    /// Uses the shared limit allows.
    pub group_uses: u32,
    /// [`assume`] bits: what the dry check took for granted.
    pub assumes: u32,
    /// Cards each use needs (costs, the card itself).
    pub demands: Vec<Demand>,
    /// `None`: usable; otherwise what on your board stops it.
    pub blocked: Option<Blocked>,
}

impl Interruption {
    /// 0 field, 1 GY, 2 banished, 3 hand (as in [`crate::InteractionProfile::by_place`]).
    pub fn place(&self) -> usize {
        match self.location {
            location::HAND => 3,
            location::GRAVE => 1,
            location::REMOVED => 2,
            _ => 0,
        }
    }
}

/// A board's capacity: how many separate interruptions it can use in one
/// opponent's turn, and the profile of the best plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Capacity {
    /// C: separate interruptions in one opponent's turn.
    pub count: u32,
    /// C_q: the same plan weighed by what each does.
    pub quality: f64,
    /// The interruptions your board lets you use, and those it blocks.
    pub interruptions: Vec<Interruption>,
    /// Uses of each of `interruptions` in the plan.
    pub plan: Vec<u32>,
    /// Uses of the plan by class ([`Class::ALL`] order) and by place.
    pub by_class: [u32; 11],
    pub by_place: [u32; 4],
    /// Uses of the plan that assume nothing about the opponent's side.
    pub without_opponent: u32,
    /// Continuous effects on the opponent's side: (card, effect code).
    pub floodgates: Vec<(u32, u32)>,
    /// Effects usable on the opponent's turn that do nothing to it (a quick
    /// summon, a protection): cards.
    pub responses: Vec<u32>,
    /// Checks the dry check could not do (a Lua error): assumed usable.
    pub unchecked: usize,
    /// Whether the plan is the best one or a greedy one (too many effects).
    pub exact: bool,
}

impl Capacity {
    pub fn negations(&self) -> u32 {
        self.uses_where(|c| c.is_negation())
    }

    pub fn removals(&self) -> u32 {
        self.uses_where(|c| !c.is_negation())
    }

    fn uses_where(&self, f: impl Fn(Class) -> bool) -> u32 {
        Class::ALL.iter().zip(self.by_class).filter(|(c, _)| f(**c)).map(|(_, n)| n).sum()
    }

    /// Uses of the plan away from the field (hand, GY, banished): they stay after a board wipe.
    pub fn off_field(&self) -> u32 {
        self.by_place[1..].iter().sum()
    }
}

/// What an effect does to the opponent, if it is used on its turn.
pub fn classify(e: &DryEffect) -> Option<Class> {
    let a = &e.effect;
    let usable_then = match a.kind {
        EffectKind::Quick { .. } | EffectKind::Activation => true,
        EffectKind::Trigger { .. } => answers_opponent(a.code),
        _ => false,
    };
    if !usable_then {
        return None;
    }
    let c = a.category;
    if c & cat::DISABLE_SUMMON != 0 {
        return Some(Class::NegateSummon);
    }
    if c & cat::NEGATE != 0 {
        return Some(Class::NegateActivation);
    }
    if c & cat::DISABLE != 0 {
        return Some(Class::NegateEffect);
    }
    // A removal is the opponent's when its checks looked at the opponent's
    // side or at the event (an effect that only targets your cards, a
    // discard to draw, is not), or when there is no target step to tell.
    let theirs = e.assumes & (assume::OPPONENT | assume::EVENT) != 0 || !a.has_target;
    if !theirs {
        return None;
    }
    [
        (cat::REMOVE, Class::Banish),
        (cat::TODECK, Class::ToDeck),
        (cat::TOHAND, Class::Bounce),
        (cat::DESTROY, Class::Destroy),
        (cat::HANDES, Class::Discard),
        (cat::TOGRAVE, Class::ToGrave),
        (cat::CONTROL, Class::Control),
        (cat::POSITION, Class::Position),
    ]
    .into_iter()
    .find(|(bit, _)| c & bit != 0)
    .map(|(_, class)| class)
}

fn is_floodgate(e: &DryEffect) -> bool {
    let a = &e.effect;
    a.kind == EffectKind::Continuous && a.type_ & 0x2 != 0 && a.target_range.1 != 0 && FLOODGATE_CODES.contains(&a.code)
}

/// Times an effect can be used in one opponent's turn on its own.
fn uses_of(e: &DryEffect, demands: &[Demand]) -> u32 {
    if let Some(limit) = e.effect.count_limit {
        return limit.count.max(1);
    }
    // A Trap or a Quick-Play Spell is used once; so is a card that pays with itself.
    if e.effect.kind == EffectKind::Activation || e.self_cost {
        return 1;
    }
    demands.iter().map(|d| d.from.len() as u32 / d.count.max(1)).min().unwrap_or(1).clamp(1, MAX_USES)
}

/// The capacity of a board from the dry check of its effects.
pub fn capacity(effects: &[DryEffect]) -> Capacity {
    let mut out = Capacity::default();
    for e in effects {
        if e.blocked.is_none() && is_floodgate(e) {
            out.floodgates.push((e.code, e.effect.code));
            continue;
        }
        let Some(class) = classify(e) else {
            let quick = matches!(e.effect.kind, EffectKind::Quick { .. });
            if quick && e.blocked.is_none() && !out.responses.contains(&e.code) {
                out.responses.push(e.code);
            }
            continue;
        };
        let demands: Vec<Demand> = e.demands.iter().filter(|d| d.count > 0).cloned().collect();
        let group = e.effect.count_limit.filter(|l| l.code != 0).map(|l| (l.code, l.index));
        out.unchecked += usize::from(e.assumes & assume::ERROR != 0);
        out.interruptions.push(Interruption {
            code: e.code,
            field_id: e.field_id,
            location: e.location,
            class,
            kind: e.effect.kind,
            description: e.effect.description,
            uses: uses_of(e, &demands),
            group,
            group_uses: e.effect.count_limit.map_or(u32::MAX, |l| l.count.max(1)),
            assumes: e.assumes,
            demands,
            blocked: e.blocked,
        });
    }
    let usable: Vec<usize> = (0..out.interruptions.len()).filter(|&i| out.interruptions[i].blocked.is_none()).collect();
    let (plan, exact) = best_plan(&out.interruptions, &usable);
    out.exact = exact;
    out.plan = vec![0; out.interruptions.len()];
    for (&i, &n) in usable.iter().zip(&plan) {
        out.plan[i] = n;
        let it = &out.interruptions[i];
        out.count += n;
        out.quality += it.class.quality() * f64::from(n);
        out.by_class[Class::ALL.iter().position(|&c| c == it.class).expect("a class")] += n;
        out.by_place[it.place()] += n;
        if it.assumes & assume::OPPONENT == 0 {
            out.without_opponent += n;
        }
    }
    out
}

/// The best numbers of uses of `items` (by count, then by quality), with
/// the shared limits and each card paying one cost at most.
fn best_plan(all: &[Interruption], items: &[usize]) -> (Vec<u32>, bool) {
    let items: Vec<&Interruption> = items.iter().map(|&i| &all[i]).collect();
    let mut plan = vec![0; items.len()];
    if items.len() > EXACT_UP_TO {
        // Greedy: best class first, one use at a time while it fits.
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by(|&a, &b| items[b].class.quality().total_cmp(&items[a].class.quality()));
        for i in order {
            while plan[i] < items[i].uses {
                plan[i] += 1;
                if !fits(&items, &plan) {
                    plan[i] -= 1;
                    break;
                }
            }
        }
        return (plan, false);
    }
    let mut best = (plan.clone(), 0, 0.0);
    let mut nodes = 0usize;
    search_plan(&items, 0, &mut plan, &mut best, &mut nodes);
    (best.0, nodes < NODE_CAP)
}

const NODE_CAP: usize = 200_000;

fn search_plan(items: &[&Interruption], at: usize, plan: &mut Vec<u32>, best: &mut (Vec<u32>, u32, f64), nodes: &mut usize) {
    *nodes += 1;
    let count: u32 = plan.iter().sum();
    let quality: f64 = items.iter().zip(plan.iter()).map(|(it, &n)| it.class.quality() * f64::from(n)).sum();
    if count > best.1 || (count == best.1 && quality > best.2 + 1e-12) {
        *best = (plan.clone(), count, quality);
    }
    if at == items.len() || *nodes >= NODE_CAP {
        return;
    }
    let bound: u32 = count + items[at..].iter().map(|it| it.uses).sum::<u32>();
    if bound < best.1 {
        return;
    }
    for n in (0..=items[at].uses).rev() {
        plan[at] = n;
        if n == 0 || fits(&items[..=at], &plan[..=at]) {
            search_plan(items, at + 1, plan, best, nodes);
        }
    }
    plan[at] = 0;
}

/// Whether `plan` keeps the shared limits and finds every cost its cards,
/// each card paying once (a matching of cost slots to cards).
fn fits(items: &[&Interruption], plan: &[u32]) -> bool {
    let mut by_group: HashMap<(u32, u32), u32> = HashMap::new();
    for (it, &n) in items.iter().zip(plan) {
        if let Some(g) = it.group {
            let used = by_group.entry(g).or_default();
            *used += n;
            if *used > it.group_uses {
                return false;
            }
        }
    }
    let slots: Vec<&[u32]> = items
        .iter()
        .zip(plan)
        .flat_map(|(it, &n)| (0..n).flat_map(move |_| it.demands.iter().flat_map(|d| (0..d.count).map(move |_| d.from.as_slice()))))
        .collect();
    let mut owner: HashMap<u32, usize> = HashMap::new();
    for s in 0..slots.len() {
        let mut seen = Vec::new();
        if !augment(s, &slots, &mut owner, &mut seen) {
            return false;
        }
    }
    true
}

fn augment(slot: usize, slots: &[&[u32]], owner: &mut HashMap<u32, usize>, seen: &mut Vec<u32>) -> bool {
    for &card in slots[slot] {
        if seen.contains(&card) {
            continue;
        }
        seen.push(card);
        match owner.get(&card).copied() {
            Some(other) if !augment(other, slots, owner, seen) => continue,
            _ => {
                owner.insert(card, slot);
                return true;
            }
        }
    }
    false
}

/// Plays `line` from the search's start and checks the board it reaches,
/// when the opponent's turn begins: its dry check and capacity.
pub fn board_capacity(engine: &Engine, config: &SearchConfig, line: &Line) -> Result<(Vec<DryEffect>, Capacity), Error> {
    let mut duel = Duel::goldfish_against_from(
        engine,
        &config.deck,
        &config.hand,
        config.seed,
        config.opponent.as_ref(),
        &config.placed,
        &config.prelude,
    )?;
    for answer in &line.answers {
        match duel.advance()? {
            Step::Decision { .. } => duel.respond(answer),
            Step::End { .. } => break,
        }
    }
    duel.advance()?;
    let effects = dry_check(&mut duel, 0)?;
    let capacity = capacity(&effects);
    Ok((effects, capacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ygo_engine::{CountLimit, EffectAnatomy};

    fn effect(field_id: u32, code: u32, kind: EffectKind, category: u64, limit: Option<CountLimit>) -> DryEffect {
        DryEffect {
            field_id,
            code,
            location: location::HAND,
            sequence: 0,
            effect: EffectAnatomy {
                kind,
                type_: 0x100,
                code: event::CHAINING,
                category,
                property: [0, 0],
                range: u32::from(location::HAND),
                target_range: (0, 0),
                count_limit: limit,
                description: 0,
                value: 0,
                has_cost: true,
                has_condition: true,
                has_target: true,
                has_operation: true,
            },
            blocked: None,
            assumes: assume::EVENT,
            self_cost: true,
            demands: vec![Demand { count: 1, from: vec![field_id] }],
        }
    }

    const QUICK: EffectKind = EffectKind::Quick { optional: true };

    #[test]
    fn two_copies_with_a_limit_by_name_count_once() {
        let limit = Some(CountLimit { count: 1, code: 7, flag: 0, index: 0 });
        let c = capacity(&[effect(1, 7, QUICK, cat::DISABLE, limit), effect(2, 7, QUICK, cat::DISABLE, limit)]);
        assert_eq!((c.count, c.negations(), c.by_place[3]), (1, 1, 1));
        assert!(c.exact);
    }

    #[test]
    fn two_costs_cannot_pay_with_the_same_card() {
        // Two effects of a monster that both discard the one card in the hand.
        let mut a = effect(1, 8, QUICK, cat::NEGATE, None);
        let mut b = effect(1, 9, QUICK, cat::REMOVE, None);
        for e in [&mut a, &mut b] {
            e.self_cost = false;
            e.location = location::MZONE;
            e.demands = vec![Demand { count: 1, from: vec![50] }];
        }
        let c = capacity(&[a, b]);
        assert_eq!(c.count, 1);
        // The negation is kept: same count, better quality.
        assert_eq!((c.negations(), c.by_place[0]), (1, 1));
    }

    #[test]
    fn blocked_and_own_removals_do_not_count() {
        let mut blocked = effect(1, 8, QUICK, cat::NEGATE, None);
        blocked.blocked = Some(Blocked::Cost);
        // A removal that only looked at your side (no event, no opponent).
        let mut own = effect(2, 9, QUICK, cat::DESTROY, None);
        own.assumes = 0;
        let c = capacity(&[blocked, own]);
        assert_eq!(c.count, 0);
        assert_eq!(c.interruptions.len(), 1);
        assert_eq!(c.responses, [9]);
    }

    #[test]
    fn ignition_effects_are_not_interruptions() {
        let c = capacity(&[effect(1, 8, EffectKind::Ignition, cat::DESTROY, None)]);
        assert_eq!((c.count, c.interruptions.len()), (0, 0));
    }
}
