//! Turns a [`Decision`] into the list of legal [`Action`]s, each carrying the
//! exact response bytes the engine expects (see `playerop.cpp` upstream).

use std::collections::HashSet;

use crate::announce::is_declarable;
use crate::carddb::CardDb;
use crate::consts::{location, position};
use crate::message::{Activatable, CardRef, Decision};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    NormalSummon,
    SpecialSummon,
    ChangePosition,
    SetMonster,
    SetSpellTrap,
    Activate,
    ToBattlePhase,
    ToMainPhase2,
    EndTurn,
    Attack,
    Yes,
    No,
    Pass,
    Choose,
    Finish,
    Cancel,
    Place,
    Position,
    Order,
    Announce,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub kind: ActionKind,
    pub label: String,
    pub response: Vec<u8>,
    /// The card the action is about (summoned, set, activated, attacking), if any.
    pub card: Option<CardRef>,
    /// The effect's description (`aux.Stringid`: the card's code and the
    /// string's index) for an activation, an optional effect or an option
    /// of an effect; 0 otherwise.
    pub description: u64,
}

impl Action {
    fn new(kind: ActionKind, label: impl Into<String>, response: Vec<u8>) -> Self {
        Self { kind, label: label.into(), response, card: None, description: 0 }
    }

    fn on(mut self, card: &CardRef) -> Self {
        self.card = Some(*card);
        self
    }

    fn described(mut self, description: u64) -> Self {
        self.description = description;
        self
    }
}

/// The name of the Attribute at bit `bit` of the engine's mask.
pub fn attribute_name(bit: u32) -> String {
    const NAMES: [&str; 7] = ["EARTH", "WATER", "FIRE", "WIND", "LIGHT", "DARK", "DIVINE"];
    NAMES.get(bit as usize).map_or_else(|| format!("#{bit}"), |n| (*n).to_owned())
}

/// The name of the Monster Type at bit `bit` of the engine's mask.
pub fn race_name(bit: u32) -> String {
    const NAMES: [&str; 26] = [
        "Warrior",
        "Spellcaster",
        "Fairy",
        "Fiend",
        "Zombie",
        "Machine",
        "Aqua",
        "Pyro",
        "Rock",
        "Winged Beast",
        "Plant",
        "Insect",
        "Thunder",
        "Dragon",
        "Beast",
        "Beast-Warrior",
        "Dinosaur",
        "Fish",
        "Sea Serpent",
        "Reptile",
        "Psychic",
        "Divine-Beast",
        "Creator God",
        "Wyrm",
        "Cyberse",
        "Illusion",
    ];
    NAMES.get(bit as usize).map_or_else(|| format!("#{bit}"), |n| (*n).to_owned())
}

/// Upper bound on enumerated card combinations for a single selection.
const MAX_COMBINATIONS: usize = 5000;

/// Names offered for a card declaration when none of the cards in the duel
/// may be declared (the first ones by passcode).
pub const MAX_ANNOUNCE: usize = 100;

/// Up to this many cards to order, every order is offered; above it, the
/// default order and its reverse.
const MAX_SORTED: usize = 5;

fn i32_bytes(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

fn cmd(kind: u32, seq: usize) -> Vec<u8> {
    i32_bytes(((seq as i32) << 16) | kind as i32)
}

/// Response for card selections: type 0 (u32 indices), count, indices.
fn indices_response(indices: &[usize]) -> Vec<u8> {
    let mut out = i32_bytes(0);
    out.extend_from_slice(&(indices.len() as u32).to_le_bytes());
    for &i in indices {
        out.extend_from_slice(&(i as u32).to_le_bytes());
    }
    out
}

pub fn describe_card(db: &CardDb, card: &CardRef) -> String {
    let name = if card.code == 0 { "face-down card".to_owned() } else { db.name(card.code) };
    let place = match card.location & !location::OVERLAY {
        location::MZONE => format!("Monster Zone {}", card.sequence + 1),
        location::SZONE => format!("S/T Zone {}", card.sequence + 1),
        loc => location::name(loc).to_owned(),
    };
    let side = if card.controller == 1 { "opponent's " } else { "" };
    format!("{name} ({side}{place})")
}

fn describe_activation(db: &CardDb, a: &Activatable) -> String {
    let text = db.describe(a.description);
    let card = describe_card(db, &a.card);
    if text.is_empty() {
        card
    } else {
        format!("{card}: {text}")
    }
}

/// Cards that are indistinguishable when they share a code (unknown order
/// inside a hidden zone): picking either copy leads to the same game.
fn is_interchangeable(card: &CardRef) -> bool {
    matches!(card.location, location::DECK | location::HAND | location::EXTRA | location::GRAVE | location::REMOVED)
}

fn combination_key(cards: &[CardRef], picked: &[usize]) -> Vec<(u32, u8, u32)> {
    let mut key: Vec<_> = picked
        .iter()
        .map(|&i| {
            let c = &cards[i];
            if is_interchangeable(c) {
                (c.code, c.location, u32::MAX)
            } else {
                (c.code, c.location, c.sequence | (c.controller as u32) << 16)
            }
        })
        .collect();
    key.sort_unstable();
    key
}

/// All index combinations of size `min..=max` accepted by `keep`, skipping
/// combinations equivalent to one already produced.
fn combinations(cards: &[CardRef], min: usize, max: usize, mut keep: impl FnMut(&[usize]) -> bool) -> Vec<Vec<usize>> {
    let n = cards.len();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for k in min.max(1)..=max.min(n) {
        let mut idx: Vec<usize> = (0..k).collect();
        loop {
            if keep(&idx) && seen.insert(combination_key(cards, &idx)) {
                out.push(idx.clone());
                if out.len() >= MAX_COMBINATIONS {
                    return out;
                }
            }
            // Next combination in lexicographic order.
            let Some(i) = (0..k).rev().find(|&i| idx[i] != i + n - k) else { break };
            idx[i] += 1;
            for j in i + 1..k {
                idx[j] = idx[j - 1] + 1;
            }
        }
    }
    out
}

/// Index combinations of exactly `k` out of `n`, in lexicographic order.
fn index_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
        if out.len() >= MAX_COMBINATIONS {
            return out;
        }
        let Some(i) = (0..k).rev().find(|&i| idx[i] != i + n - k) else { return out };
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// Every order of `0..n`.
fn permutations(n: usize) -> Vec<Vec<usize>> {
    fn rec(current: &mut Vec<usize>, used: &mut [bool], out: &mut Vec<Vec<usize>>) {
        if current.len() == used.len() {
            out.push(current.clone());
            return;
        }
        for i in 0..used.len() {
            if !used[i] {
                used[i] = true;
                current.push(i);
                rec(current, used, out);
                current.pop();
                used[i] = false;
            }
        }
    }
    let mut out = Vec::new();
    rec(&mut Vec::with_capacity(n), &mut vec![false; n], &mut out);
    out
}

/// Ways to take `count` counters from cards holding at most `caps[i]` each.
fn counter_splits(caps: &[u16], count: u16) -> Vec<Vec<u16>> {
    fn rec(caps: &[u16], remaining: u16, current: &mut Vec<u16>, out: &mut Vec<Vec<u16>>) {
        if out.len() >= MAX_COMBINATIONS {
            return;
        }
        let Some((&cap, rest)) = caps.split_first() else {
            if remaining == 0 {
                out.push(current.clone());
            }
            return;
        };
        // What the remaining cards can still take.
        let room: u32 = rest.iter().map(|&c| c as u32).sum();
        for k in 0..=cap.min(remaining) {
            if (remaining - k) as u32 > room {
                continue;
            }
            current.push(k);
            rec(rest, remaining - k, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    rec(caps, count, &mut Vec::with_capacity(caps.len()), &mut out);
    out
}

/// Port of `field::select_sum_check1`: can the parameters reach exactly `acc`?
fn sum_check_exact(params: &[u32], acc: i64) -> bool {
    fn rec(params: &[u32], index: usize, acc: i64) -> bool {
        if acc == 0 || index == params.len() {
            return false;
        }
        let o1 = (params[index] & 0xffff) as i64;
        let o2 = (params[index] >> 16) as i64;
        if index == params.len() - 1 {
            return acc == o1 || acc == o2;
        }
        (acc > o1 && rec(params, index + 1, acc - o1)) || (o2 > 0 && acc > o2 && rec(params, index + 1, acc - o2))
    }
    rec(params, 0, acc)
}

/// Port of the "at least" branch of `SelectSum`: reaches `acc` and no card is
/// superfluous.
fn sum_check_at_least(params: &[u32], acc: i64) -> bool {
    let (mut sum, mut max_sum, mut smallest) = (0i64, 0i64, i64::MAX);
    for &p in params {
        let o1 = (p & 0xffff) as i64;
        let o2 = (p >> 16) as i64;
        let low = if o2 != 0 && o2 < o1 { o2 } else { o1 };
        sum += low;
        max_sum += o1.max(o2);
        smallest = smallest.min(low);
    }
    !(max_sum < acc || sum - smallest >= acc)
}

fn zone_name(location: u8, sequence: u8) -> String {
    match (location, sequence) {
        (location::MZONE, 5) => "Extra Monster Zone (left)".into(),
        (location::MZONE, 6) => "Extra Monster Zone (right)".into(),
        (location::MZONE, s) => format!("Main Monster Zone {}", s + 1),
        (_, 5) => "Field Zone".into(),
        (_, 6) => "Pendulum Zone (left)".into(),
        (_, 7) => "Pendulum Zone (right)".into(),
        (_, s) => format!("Spell/Trap Zone {}", s + 1),
    }
}

/// Every legal answer to `decision`. Card declarations offer every card that
/// may be declared, up to [`MAX_ANNOUNCE`]; see [`actions_with`].
pub fn actions(decision: &Decision, db: &CardDb) -> Vec<Action> {
    actions_with(decision, db, &[])
}

/// Like [`actions`], but a card declaration offers the cards of `known`
/// (typically the cards in the duel) that may be declared, falling back to
/// any declarable card when none of them may.
pub fn actions_with(decision: &Decision, db: &CardDb, known: &[u32]) -> Vec<Action> {
    let mut out = all_actions(decision, db, known);
    if matches!(decision, Decision::IdleCmd { .. } | Decision::BattleCmd { .. } | Decision::SelectChain { .. }) {
        drop_copies(decision, &mut out);
    }
    out
}

/// What an action does, for telling apart two copies of a card: the kind,
/// the card and where it is, and the effect for activations.
fn action_key(decision: &Decision, a: &Action) -> Option<(ActionKind, u32, u8, u32, u64)> {
    let card = a.card.as_ref().filter(|c| is_interchangeable(c))?;
    let description = match decision {
        Decision::IdleCmd { activatable, .. }
        | Decision::BattleCmd { activatable, .. }
        | Decision::SelectChain { chains: activatable, .. }
            if a.kind == ActionKind::Activate =>
        {
            activatable.iter().find(|x| x.card == *card).map_or(0, |x| x.description)
        }
        _ => 0,
    };
    Some((a.kind, card.code, card.location, card.position, description))
}

/// Keeps one of the actions that do the same thing with copies of a card
/// in the hand, Deck, Graveyard or banishment: either leads to the same game.
fn drop_copies(decision: &Decision, out: &mut Vec<Action>) {
    let mut seen = HashSet::new();
    out.retain(|a| action_key(decision, a).is_none_or(|key| seen.insert(key)));
}

fn all_actions(decision: &Decision, db: &CardDb, known: &[u32]) -> Vec<Action> {
    use ActionKind as K;
    let mut out = Vec::new();
    match decision {
        Decision::IdleCmd {
            summonable,
            special_summonable,
            repositionable,
            monster_settable,
            spell_settable,
            activatable,
            to_battle_phase,
            to_end_phase,
            ..
        } => {
            for (i, c) in summonable.iter().enumerate() {
                out.push(Action::new(K::NormalSummon, format!("Normal Summon {}", describe_card(db, c)), cmd(0, i)).on(c));
            }
            for (i, c) in special_summonable.iter().enumerate() {
                out.push(Action::new(K::SpecialSummon, format!("Special Summon {}", describe_card(db, c)), cmd(1, i)).on(c));
            }
            for (i, c) in repositionable.iter().enumerate() {
                out.push(Action::new(K::ChangePosition, format!("Change position of {}", describe_card(db, c)), cmd(2, i)).on(c));
            }
            for (i, c) in monster_settable.iter().enumerate() {
                out.push(Action::new(K::SetMonster, format!("Set {}", describe_card(db, c)), cmd(3, i)).on(c));
            }
            for (i, c) in spell_settable.iter().enumerate() {
                out.push(Action::new(K::SetSpellTrap, format!("Set {}", describe_card(db, c)), cmd(4, i)).on(c));
            }
            for (i, a) in activatable.iter().enumerate() {
                out.push(
                    Action::new(K::Activate, format!("Activate {}", describe_activation(db, a)), cmd(5, i))
                        .on(&a.card)
                        .described(a.description),
                );
            }
            if *to_battle_phase {
                out.push(Action::new(K::ToBattlePhase, "Go to Battle Phase", cmd(6, 0)));
            }
            if *to_end_phase {
                out.push(Action::new(K::EndTurn, "End turn", cmd(7, 0)));
            }
        }
        Decision::BattleCmd { activatable, attackers, to_main2, to_end_phase, .. } => {
            for (i, a) in activatable.iter().enumerate() {
                out.push(
                    Action::new(K::Activate, format!("Activate {}", describe_activation(db, a)), cmd(0, i))
                        .on(&a.card)
                        .described(a.description),
                );
            }
            for (i, (c, direct)) in attackers.iter().enumerate() {
                let how = if *direct { " (can attack directly)" } else { "" };
                out.push(Action::new(K::Attack, format!("Attack with {}{how}", describe_card(db, c)), cmd(1, i)).on(c));
            }
            if *to_main2 {
                out.push(Action::new(K::ToMainPhase2, "Go to Main Phase 2", cmd(2, 0)));
            }
            if *to_end_phase {
                out.push(Action::new(K::EndTurn, "End turn", cmd(3, 0)));
            }
        }
        Decision::EffectYesNo { card, description, .. } => {
            let text = db.describe(*description);
            out.push(
                Action::new(
                    K::Yes,
                    format!("Yes, use {}{}", describe_card(db, card), if text.is_empty() { String::new() } else { format!(": {text}") }),
                    i32_bytes(1),
                )
                .on(card)
                .described(*description),
            );
            out.push(Action::new(K::No, "No", i32_bytes(0)));
        }
        Decision::YesNo { description, .. } => {
            let text = db.describe(*description);
            out.push(
                Action::new(K::Yes, if text.is_empty() { "Yes".to_owned() } else { format!("Yes: {text}") }, i32_bytes(1))
                    .described(*description),
            );
            out.push(Action::new(K::No, "No", i32_bytes(0)));
        }
        Decision::Option { options, .. } => {
            for (i, &o) in options.iter().enumerate() {
                out.push(Action::new(K::Choose, format!("Option: {}", db.describe(o)), i32_bytes(i as i32)).described(o));
            }
        }
        Decision::SelectCard { cancelable, min, max, cards, .. } => {
            for pick in combinations(cards, *min as usize, *max as usize, |_| true) {
                let names: Vec<_> = pick.iter().map(|&i| describe_card(db, &cards[i])).collect();
                let action = Action::new(K::Choose, format!("Select {}", names.join(" + ")), indices_response(&pick));
                out.push(if let [one] = pick[..] { action.on(&cards[one]) } else { action });
            }
            if *cancelable {
                out.push(Action::new(K::Cancel, "Cancel", i32_bytes(-1)));
            }
        }
        Decision::SelectUnselectCard { finishable, cancelable, selectable, unselectable, .. } => {
            let mut seen = HashSet::new();
            for (i, c) in selectable.iter().enumerate() {
                if seen.insert(combination_key(selectable, &[i])) {
                    let mut resp = i32_bytes(1);
                    resp.extend_from_slice(&(i as i32).to_le_bytes());
                    out.push(Action::new(K::Choose, format!("Select {}", describe_card(db, c)), resp).on(c));
                }
            }
            for (j, c) in unselectable.iter().enumerate() {
                let mut resp = i32_bytes(1);
                resp.extend_from_slice(&((selectable.len() + j) as i32).to_le_bytes());
                out.push(Action::new(K::Choose, format!("Unselect {}", describe_card(db, c)), resp));
            }
            if *finishable {
                out.push(Action::new(K::Finish, "Finish selection", i32_bytes(-1)));
            } else if *cancelable {
                out.push(Action::new(K::Cancel, "Cancel", i32_bytes(-1)));
            }
        }
        Decision::SelectChain { forced, chains, .. } => {
            if !*forced {
                out.push(Action::new(K::Pass, "Don't chain", i32_bytes(-1)));
            }
            for (i, a) in chains.iter().enumerate() {
                out.push(
                    Action::new(K::Activate, format!("Chain {}", describe_activation(db, a)), i32_bytes(i as i32))
                        .on(&a.card)
                        .described(a.description),
                );
            }
        }
        Decision::SelectPlace { player, count, blocked, .. } => {
            let mut zones = Vec::new();
            for (side, shift) in [(*player, 0u32), (1 - *player, 16u32)] {
                for (loc, base, n) in [(location::MZONE, 0u32, 7u8), (location::SZONE, 8u32, 8u8)] {
                    for seq in 0..n {
                        if blocked & (1u32 << (shift + base + seq as u32)) == 0 {
                            zones.push((side, loc, seq));
                        }
                    }
                }
            }
            if *count <= 1 {
                for (side, loc, seq) in zones {
                    let owner = if side == *player { "your" } else { "opponent's" };
                    out.push(Action::new(K::Place, format!("Place in {owner} {}", zone_name(loc, seq)), vec![side, loc, seq]));
                }
            } else {
                for pick in index_combinations(zones.len(), *count as usize) {
                    let chosen: Vec<_> = pick.iter().map(|&i| zones[i]).collect();
                    let label = chosen.iter().map(|&(_, l, s)| zone_name(l, s)).collect::<Vec<_>>().join(", ");
                    out.push(Action::new(
                        K::Place,
                        format!("Choose zones {label}"),
                        chosen.iter().flat_map(|&(p, l, s)| [p, l, s]).collect(),
                    ));
                }
            }
        }
        Decision::SelectPosition { positions, .. } => {
            for pos in [position::FACEUP_ATTACK, position::FACEDOWN_ATTACK, position::FACEUP_DEFENSE, position::FACEDOWN_DEFENSE] {
                if *positions as u32 & pos != 0 {
                    out.push(Action::new(K::Position, format!("Position: {}", position::name(pos)), i32_bytes(pos as i32)));
                }
            }
        }
        Decision::SelectTribute { cancelable, min, max, cards, .. } => {
            let refs: Vec<CardRef> = cards.iter().map(|(c, _)| *c).collect();
            let picks = combinations(&refs, 1, *max as usize, |pick| pick.iter().map(|&i| cards[i].1 as u32).sum::<u32>() >= *min);
            for pick in picks {
                let names: Vec<_> = pick.iter().map(|&i| describe_card(db, &refs[i])).collect();
                out.push(Action::new(K::Choose, format!("Tribute {}", names.join(" + ")), indices_response(&pick)));
            }
            if *cancelable {
                out.push(Action::new(K::Cancel, "Cancel", i32_bytes(-1)));
            }
        }
        Decision::SelectSum { exact, target, min, max, must, cards, .. } => {
            let refs: Vec<CardRef> = cards.iter().map(|(c, _)| *c).collect();
            let must_params: Vec<u32> = must.iter().map(|(_, p)| *p).collect();
            let (lo, hi) = if *exact { (*min as usize, *max as usize) } else { (1, refs.len()) };
            let picks = combinations(&refs, lo, hi, |pick| {
                let mut params = must_params.clone();
                params.extend(pick.iter().map(|&i| cards[i].1));
                if *exact {
                    sum_check_exact(&params, *target as i64)
                } else {
                    sum_check_at_least(&params, *target as i64)
                }
            });
            for pick in picks {
                let names: Vec<_> = pick.iter().map(|&i| describe_card(db, &refs[i])).collect();
                out.push(Action::new(K::Choose, format!("Select {}", names.join(" + ")), indices_response(&pick)));
            }
        }
        Decision::Sort { cards, .. } => {
            out.push(Action::new(K::Order, "Keep the default order", vec![0xff]));
            let n = cards.len();
            let orders: Vec<Vec<usize>> = if n <= MAX_SORTED { permutations(n) } else { vec![(0..n).rev().collect()] };
            let mut seen = HashSet::new();
            seen.insert(cards.iter().map(|c| c.code).collect::<Vec<_>>());
            for order in orders {
                // Orders of identical cards are the same order.
                if !seen.insert(order.iter().map(|&i| cards[i].code).collect::<Vec<_>>()) {
                    continue;
                }
                let mut response = vec![0u8; n];
                for (pos, &i) in order.iter().enumerate() {
                    response[i] = pos as u8;
                }
                let names: Vec<_> = order.iter().map(|&i| db.name(cards[i].code)).collect();
                out.push(Action::new(K::Order, format!("Order: {}", names.join(", ")), response));
            }
        }
        Decision::AnnounceRace { count, available, .. } => {
            let bits: Vec<u32> = (0..64).filter(|b| available & (1u64 << b) != 0).collect();
            for pick in index_combinations(bits.len(), *count as usize) {
                let mask = pick.iter().fold(0u64, |m, &i| m | 1u64 << bits[i]);
                let label = pick.iter().map(|&i| race_name(bits[i])).collect::<Vec<_>>().join(" + ");
                out.push(Action::new(K::Announce, format!("Declare monster type {label}"), mask.to_le_bytes().to_vec()));
            }
        }
        Decision::AnnounceAttribute { count, available, .. } => {
            let bits: Vec<u32> = (0..32).filter(|b| available & (1u32 << b) != 0).collect();
            for pick in index_combinations(bits.len(), *count as usize) {
                let mask = pick.iter().fold(0u32, |m, &i| m | 1u32 << bits[i]);
                let label = pick.iter().map(|&i| attribute_name(bits[i])).collect::<Vec<_>>().join(" + ");
                out.push(Action::new(K::Announce, format!("Declare attribute {label}"), mask.to_le_bytes().to_vec()));
            }
        }
        Decision::AnnounceNumber { options, .. } => {
            for (i, o) in options.iter().enumerate() {
                out.push(Action::new(K::Announce, format!("Declare {o}"), i32_bytes(i as i32)));
            }
        }
        Decision::AnnounceCard { opcodes, .. } => {
            let mut codes: Vec<u32> = Vec::new();
            for &code in known {
                for code in [code, db.canonical_code(code)] {
                    if !codes.contains(&code) && db.get(code).is_some_and(|c| is_declarable(c, opcodes)) {
                        codes.push(code);
                    }
                }
            }
            if codes.is_empty() {
                codes = db.declarable(opcodes);
                codes.truncate(MAX_ANNOUNCE);
            }
            for code in codes {
                out.push(Action::new(K::Announce, format!("Declare {}", db.name(code)), i32_bytes(code as i32)));
            }
        }
        Decision::SelectCounter { count, cards, .. } => {
            let caps: Vec<u16> = cards.iter().map(|&(_, n)| n).collect();
            for split in counter_splits(&caps, *count) {
                let label = split
                    .iter()
                    .zip(cards)
                    .filter(|(&k, _)| k > 0)
                    .map(|(k, (c, _))| format!("{k} from {}", describe_card(db, c)))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push(Action::new(
                    K::Choose,
                    format!("Counters: {label}"),
                    split.iter().flat_map(|&k| (k as i16).to_le_bytes()).collect(),
                ));
            }
        }
        Decision::RockPaperScissors { .. } => {
            for (value, name) in [(1, "scissors"), (2, "rock"), (3, "paper")] {
                out.push(Action::new(K::Choose, format!("Rock-paper-scissors: {name}"), i32_bytes(value)));
            }
        }
        Decision::Unsupported { .. } => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(code: u32, location: u8, sequence: u32) -> CardRef {
        CardRef { code, controller: 0, location, sequence, position: 0 }
    }

    #[test]
    fn identical_cards_in_deck_are_one_choice() {
        let cards = vec![card(1, location::DECK, 0), card(1, location::DECK, 1), card(2, location::DECK, 2)];
        let picks = combinations(&cards, 1, 1, |_| true);
        assert_eq!(picks, vec![vec![0], vec![2]]);
    }

    #[test]
    fn identical_cards_on_field_stay_distinct() {
        let cards = vec![card(1, location::MZONE, 0), card(1, location::MZONE, 1)];
        assert_eq!(combinations(&cards, 1, 1, |_| true).len(), 2);
    }

    #[test]
    fn combinations_respect_bounds() {
        let cards: Vec<_> = (0..4).map(|i| card(i, location::HAND, i)).collect();
        assert_eq!(combinations(&cards, 2, 2, |_| true).len(), 6);
        assert_eq!(combinations(&cards, 1, 4, |_| true).len(), 15);
    }

    #[test]
    fn exact_sum_matches_engine_rule() {
        // Two level-4 monsters make exactly 8; a level 4 and a level 3 do not.
        assert!(sum_check_exact(&[4, 4], 8));
        assert!(!sum_check_exact(&[4, 3], 8));
        // A card that can count as 2 or 3 (o2 in the high half).
        assert!(sum_check_exact(&[(3 << 16) | 2, 5], 8));
    }

    #[test]
    fn select_card_response_layout() {
        assert_eq!(indices_response(&[2]), vec![0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn idle_command_encoding() {
        // Activate (type 5) the effect at index 3.
        assert_eq!(cmd(5, 3), ((3i32 << 16) | 5).to_le_bytes().to_vec());
    }

    #[test]
    fn every_order_of_a_few_cards() {
        let db = CardDb::default();
        let decision = Decision::Sort {
            player: 0,
            chain: false,
            cards: vec![card(1, location::DECK, 0), card(2, location::DECK, 1), card(3, location::DECK, 2)],
        };
        let list = actions(&decision, &db);
        // Default order plus the 5 other orders.
        assert_eq!(list.len(), 6);
        assert_eq!(list[0].response, vec![0xff]);
        assert!(list.iter().skip(1).all(|a| a.response.len() == 3));
        // Identical cards: only distinct orders.
        let decision = Decision::Sort {
            player: 0,
            chain: false,
            cards: vec![card(1, location::DECK, 0), card(1, location::DECK, 1), card(2, location::DECK, 2)],
        };
        assert_eq!(actions(&decision, &db).len(), 3);
    }

    #[test]
    fn counters_are_split_every_way() {
        assert_eq!(counter_splits(&[2, 1], 2), vec![vec![1, 1], vec![2, 0]]);
        assert_eq!(counter_splits(&[1, 1, 1], 2).len(), 3);
        let db = CardDb::default();
        let decision = Decision::SelectCounter {
            player: 0,
            counter_type: 1,
            count: 1,
            cards: vec![(card(1, location::SZONE, 0), 1), (card(2, location::MZONE, 0), 3)],
        };
        let list = actions(&decision, &db);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].response, [0, 0, 1, 0]);
    }

    #[test]
    fn several_types_or_attributes_at_once() {
        let db = CardDb::default();
        let two_of_three = Decision::AnnounceAttribute { player: 0, count: 2, available: 0b111 };
        let list = actions(&two_of_three, &db);
        assert_eq!(list.iter().map(|a| u32::from_le_bytes(a.response[..4].try_into().unwrap())).collect::<Vec<_>>(), [0b011, 0b101, 0b110]);
        assert_eq!(actions(&Decision::AnnounceRace { player: 0, count: 1, available: 0b1010 }, &db).len(), 2);
        assert_eq!(actions(&Decision::RockPaperScissors { player: 0 }, &db).len(), 3);
    }

    #[test]
    fn several_zones_at_once() {
        // Everything blocked except your Main Monster Zones 1-3: two of them, three ways.
        let free: u32 = 0b111;
        let decision = Decision::SelectPlace { player: 0, count: 2, blocked: !free, disable_field: true };
        let list = actions(&decision, &CardDb::default());
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].response, [0, location::MZONE, 0, 0, location::MZONE, 1]);
    }
}
