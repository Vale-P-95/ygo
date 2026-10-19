//! Parsing of the engine's binary message stream.
//!
//! The buffer returned by `OCG_DuelGetMessage` is a sequence of
//! `[u32 length][u8 type][payload]` records. Decision messages (`MSG_SELECT_*`,
//! `MSG_ANNOUNCE_*`, ...) ask the player for a response; the others describe
//! what happened and are turned into [`Event`]s.

use crate::consts::{hint, msg};

/// Little-endian cursor over a message payload.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.pos..self.pos + N)?;
        self.pos += N;
        bytes.try_into().ok()
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        self.take::<1>().map(|b| b[0])
    }
    pub(crate) fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_le_bytes)
    }
    pub(crate) fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_le_bytes)
    }
    pub(crate) fn u64(&mut self) -> Option<u64> {
        self.take().map(u64::from_le_bytes)
    }
    pub(crate) fn skip(&mut self, n: usize) -> Option<()> {
        (self.pos + n <= self.data.len()).then(|| self.pos += n)
    }
    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// `loc_info`: controller u8, location u8, sequence u32, position u32.
    fn loc_info(&mut self, code: u32) -> Option<CardRef> {
        Some(CardRef { code, controller: self.u8()?, location: self.u8()?, sequence: self.u32()?, position: self.u32()? })
    }
    fn card_with_info(&mut self) -> Option<CardRef> {
        let code = self.u32()?;
        self.loc_info(code)
    }
    /// code u32, controller u8, location u8, sequence u32 (no position).
    fn card_short(&mut self) -> Option<CardRef> {
        Some(CardRef { code: self.u32()?, controller: self.u8()?, location: self.u8()?, sequence: self.u32()?, position: 0 })
    }
}

/// A card as identified by the engine: its code and where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardRef {
    pub code: u32,
    pub controller: u8,
    pub location: u8,
    pub sequence: u32,
    pub position: u32,
}

/// An effect that can be activated, as listed in idle/battle/chain decisions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Activatable {
    pub card: CardRef,
    pub description: u64,
    pub client_mode: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Decision {
    IdleCmd {
        player: u8,
        summonable: Vec<CardRef>,
        special_summonable: Vec<CardRef>,
        repositionable: Vec<CardRef>,
        monster_settable: Vec<CardRef>,
        spell_settable: Vec<CardRef>,
        activatable: Vec<Activatable>,
        to_battle_phase: bool,
        to_end_phase: bool,
    },
    BattleCmd {
        player: u8,
        activatable: Vec<Activatable>,
        attackers: Vec<(CardRef, bool)>,
        to_main2: bool,
        to_end_phase: bool,
    },
    EffectYesNo {
        player: u8,
        card: CardRef,
        description: u64,
    },
    YesNo {
        player: u8,
        description: u64,
    },
    Option {
        player: u8,
        options: Vec<u64>,
    },
    SelectCard {
        player: u8,
        cancelable: bool,
        min: u32,
        max: u32,
        cards: Vec<CardRef>,
        hint: u64,
    },
    SelectUnselectCard {
        player: u8,
        finishable: bool,
        cancelable: bool,
        min: u32,
        max: u32,
        selectable: Vec<CardRef>,
        unselectable: Vec<CardRef>,
        hint: u64,
    },
    SelectChain {
        player: u8,
        forced: bool,
        /// The window that puts simultaneous trigger effects on a chain
        /// (not a chance to respond with quick effects).
        triggers: bool,
        chains: Vec<Activatable>,
    },
    SelectPlace {
        player: u8,
        count: u8,
        blocked: u32,
        disable_field: bool,
    },
    SelectPosition {
        player: u8,
        code: u32,
        positions: u8,
    },
    SelectTribute {
        player: u8,
        cancelable: bool,
        min: u32,
        max: u32,
        cards: Vec<(CardRef, u8)>,
        hint: u64,
    },
    SelectSum {
        player: u8,
        exact: bool,
        target: u32,
        min: u32,
        max: u32,
        must: Vec<(CardRef, u32)>,
        cards: Vec<(CardRef, u32)>,
        hint: u64,
    },
    Sort {
        player: u8,
        chain: bool,
        cards: Vec<CardRef>,
    },
    AnnounceRace {
        player: u8,
        count: u8,
        available: u64,
    },
    AnnounceAttribute {
        player: u8,
        count: u8,
        available: u32,
    },
    AnnounceNumber {
        player: u8,
        options: Vec<u64>,
    },
    AnnounceCard {
        player: u8,
        opcodes: Vec<u64>,
    },
    /// Remove (or place) `count` counters of `counter_type`, spread over
    /// cards that each have at most the given number.
    SelectCounter {
        player: u8,
        counter_type: u16,
        count: u16,
        cards: Vec<(CardRef, u16)>,
    },
    RockPaperScissors {
        player: u8,
    },
    /// A decision this crate does not handle yet.
    Unsupported {
        message: u8,
    },
}

impl Decision {
    pub fn player(&self) -> Option<u8> {
        use Decision::*;
        match self {
            IdleCmd { player, .. }
            | BattleCmd { player, .. }
            | EffectYesNo { player, .. }
            | YesNo { player, .. }
            | Option { player, .. }
            | SelectCard { player, .. }
            | SelectUnselectCard { player, .. }
            | SelectChain { player, .. }
            | SelectPlace { player, .. }
            | SelectPosition { player, .. }
            | SelectTribute { player, .. }
            | SelectSum { player, .. }
            | Sort { player, .. }
            | AnnounceRace { player, .. }
            | AnnounceAttribute { player, .. }
            | AnnounceNumber { player, .. }
            | AnnounceCard { player, .. }
            | SelectCounter { player, .. }
            | RockPaperScissors { player } => Some(*player),
            Unsupported { .. } => None,
        }
    }
}

/// Something that happened in the duel, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    NewTurn {
        player: u8,
    },
    NewPhase {
        phase: u16,
    },
    Draw {
        player: u8,
        codes: Vec<u32>,
    },
    Move {
        code: u32,
        from: CardRef,
        to: CardRef,
        reason: u32,
    },
    NormalSummoning {
        card: CardRef,
    },
    SpecialSummoning {
        card: CardRef,
    },
    Chaining {
        card: CardRef,
        description: u64,
        chain_link: u32,
    },
    ChainSolving {
        chain_link: u8,
    },
    ChainSolved {
        chain_link: u8,
    },
    ChainEnd,
    /// The activation of a chain link was negated.
    ChainNegated {
        chain_link: u8,
    },
    /// The effect of a chain link was negated.
    ChainDisabled {
        chain_link: u8,
    },
    /// Cards targeted by the chain link being activated.
    BecomeTarget {
        cards: Vec<CardRef>,
    },
    Set {
        card: CardRef,
    },
    FlipSummoning {
        card: CardRef,
    },
    PositionChange {
        card: CardRef,
        from: u8,
        to: u8,
    },
    Damage {
        player: u8,
        amount: u32,
    },
    Recover {
        player: u8,
        amount: u32,
    },
    PayLpCost {
        player: u8,
        amount: u32,
    },
    LpUpdate {
        player: u8,
        lp: u32,
    },
    Attack {
        attacker: CardRef,
        target: Option<CardRef>,
    },
    /// Counters placed (positive) or removed (negative) on a card.
    Counter {
        card: CardRef,
        counter_type: u16,
        delta: i32,
    },
    Win {
        player: u8,
        reason: u8,
    },
    /// The previous response was rejected by the engine.
    Retry,
    Other {
        message: u8,
    },
}

const DECISIONS: &[u8] = &[
    msg::SELECT_BATTLECMD,
    msg::SELECT_IDLECMD,
    msg::SELECT_EFFECTYN,
    msg::SELECT_YESNO,
    msg::SELECT_OPTION,
    msg::SELECT_CARD,
    msg::SELECT_CHAIN,
    msg::SELECT_PLACE,
    msg::SELECT_POSITION,
    msg::SELECT_TRIBUTE,
    msg::SORT_CHAIN,
    msg::SELECT_COUNTER,
    msg::SELECT_SUM,
    msg::SELECT_DISFIELD,
    msg::SORT_CARD,
    msg::SELECT_UNSELECT_CARD,
    msg::ANNOUNCE_RACE,
    msg::ANNOUNCE_ATTRIB,
    msg::ANNOUNCE_CARD,
    msg::ANNOUNCE_NUMBER,
    msg::ROCK_PAPER_SCISSORS,
];

pub(crate) enum Parsed {
    Event(Event),
    Decision(Decision),
    /// `MSG_HINT` of type `HINT_SELECTMSG`: the prompt of the next selection.
    SelectHint(u64),
    Ignored,
}

/// Splits a message buffer into its records.
pub(crate) fn split(buffer: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut r = Reader::new(buffer);
    while r.remaining() >= 4 {
        let Some(len) = r.u32() else { break };
        let start = r.pos;
        if r.skip(len as usize).is_none() {
            break;
        }
        out.push(&buffer[start..start + len as usize]);
    }
    out
}

pub(crate) fn parse(record: &[u8], select_hint: u64) -> Parsed {
    let Some((&kind, payload)) = record.split_first() else { return Parsed::Ignored };
    let mut r = Reader::new(payload);
    let parsed =
        if DECISIONS.contains(&kind) { parse_decision(kind, &mut r, select_hint).map(Parsed::Decision) } else { parse_event(kind, &mut r) };
    parsed.unwrap_or(if DECISIONS.contains(&kind) {
        Parsed::Decision(Decision::Unsupported { message: kind })
    } else {
        Parsed::Event(Event::Other { message: kind })
    })
}

fn activatables(r: &mut Reader, n: u32, short_location: bool) -> Option<Vec<Activatable>> {
    (0..n)
        .map(|_| {
            let card = if short_location { r.card_short()? } else { r.card_with_info()? };
            Some(Activatable { card, description: r.u64()?, client_mode: r.u8()? })
        })
        .collect()
}

fn cards(r: &mut Reader, n: u32) -> Option<Vec<CardRef>> {
    (0..n).map(|_| r.card_with_info()).collect()
}

fn short_cards(r: &mut Reader, n: u32) -> Option<Vec<CardRef>> {
    (0..n).map(|_| r.card_short()).collect()
}

fn parse_decision(kind: u8, r: &mut Reader, hint: u64) -> Option<Decision> {
    let player = r.u8()?;
    Some(match kind {
        msg::SELECT_IDLECMD => {
            let n = r.u32()?;
            let summonable = short_cards(r, n)?;
            let n = r.u32()?;
            let special_summonable = short_cards(r, n)?;
            let n = r.u32()?;
            let repositionable = (0..n)
                .map(|_| Some(CardRef { code: r.u32()?, controller: r.u8()?, location: r.u8()?, sequence: r.u8()? as u32, position: 0 }))
                .collect::<Option<_>>()?;
            let n = r.u32()?;
            let monster_settable = short_cards(r, n)?;
            let n = r.u32()?;
            let spell_settable = short_cards(r, n)?;
            let n = r.u32()?;
            let activatable = activatables(r, n, true)?;
            let to_battle_phase = r.u8()? != 0;
            let to_end_phase = r.u8()? != 0;
            Decision::IdleCmd {
                player,
                summonable,
                special_summonable,
                repositionable,
                monster_settable,
                spell_settable,
                activatable,
                to_battle_phase,
                to_end_phase,
            }
        }
        msg::SELECT_BATTLECMD => {
            let n = r.u32()?;
            let activatable = activatables(r, n, true)?;
            let n = r.u32()?;
            let attackers = (0..n)
                .map(|_| {
                    let card = CardRef { code: r.u32()?, controller: r.u8()?, location: r.u8()?, sequence: r.u8()? as u32, position: 0 };
                    Some((card, r.u8()? != 0))
                })
                .collect::<Option<_>>()?;
            Decision::BattleCmd { player, activatable, attackers, to_main2: r.u8()? != 0, to_end_phase: r.u8()? != 0 }
        }
        msg::SELECT_EFFECTYN => {
            let card = r.card_with_info()?;
            Decision::EffectYesNo { player, card, description: r.u64()? }
        }
        msg::SELECT_YESNO => Decision::YesNo { player, description: r.u64()? },
        msg::SELECT_OPTION => {
            let n = r.u8()?;
            Decision::Option { player, options: (0..n).map(|_| r.u64()).collect::<Option<_>>()? }
        }
        msg::SELECT_CARD => {
            let cancelable = r.u8()? != 0;
            let (min, max, n) = (r.u32()?, r.u32()?, r.u32()?);
            Decision::SelectCard { player, cancelable, min, max, cards: cards(r, n)?, hint }
        }
        msg::SELECT_UNSELECT_CARD => {
            let finishable = r.u8()? != 0;
            let cancelable = r.u8()? != 0;
            let (min, max) = (r.u32()?, r.u32()?);
            let n = r.u32()?;
            let selectable = cards(r, n)?;
            let n = r.u32()?;
            let unselectable = cards(r, n)?;
            Decision::SelectUnselectCard { player, finishable, cancelable, min, max, selectable, unselectable, hint }
        }
        msg::SELECT_CHAIN => {
            // The engine marks the trigger windows with this count.
            let triggers = r.u8()? == 0x7f;
            let forced = r.u8()? != 0;
            let _hint_timing = (r.u32()?, r.u32()?);
            let n = r.u32()?;
            Decision::SelectChain { player, forced, triggers, chains: activatables(r, n, false)? }
        }
        msg::SELECT_PLACE | msg::SELECT_DISFIELD => {
            Decision::SelectPlace { player, count: r.u8()?, blocked: r.u32()?, disable_field: kind == msg::SELECT_DISFIELD }
        }
        msg::SELECT_POSITION => Decision::SelectPosition { player, code: r.u32()?, positions: r.u8()? },
        msg::SELECT_TRIBUTE => {
            let cancelable = r.u8()? != 0;
            let (min, max, n) = (r.u32()?, r.u32()?, r.u32()?);
            let cards = (0..n).map(|_| Some((r.card_short()?, r.u8()?))).collect::<Option<_>>()?;
            Decision::SelectTribute { player, cancelable, min, max, cards, hint }
        }
        msg::SELECT_SUM => {
            let exact = r.u8()? == 0;
            let (target, min, max) = (r.u32()?, r.u32()?, r.u32()?);
            let n = r.u32()?;
            let must = (0..n).map(|_| Some((r.card_with_info()?, r.u32()?))).collect::<Option<_>>()?;
            let n = r.u32()?;
            let cards = (0..n).map(|_| Some((r.card_with_info()?, r.u32()?))).collect::<Option<_>>()?;
            Decision::SelectSum { player, exact, target, min, max, must, cards, hint }
        }
        msg::SORT_CARD | msg::SORT_CHAIN => {
            let n = r.u32()?;
            let cards = (0..n)
                .map(|_| Some(CardRef { code: r.u32()?, controller: r.u8()?, location: r.u32()? as u8, sequence: r.u32()?, position: 0 }))
                .collect::<Option<_>>()?;
            Decision::Sort { player, chain: kind == msg::SORT_CHAIN, cards }
        }
        msg::SELECT_COUNTER => {
            let (counter_type, count, n) = (r.u16()?, r.u16()?, r.u32()?);
            let cards = (0..n)
                .map(|_| {
                    let card = CardRef { code: r.u32()?, controller: r.u8()?, location: r.u8()?, sequence: r.u8()? as u32, position: 0 };
                    Some((card, r.u16()?))
                })
                .collect::<Option<_>>()?;
            Decision::SelectCounter { player, counter_type, count, cards }
        }
        msg::ROCK_PAPER_SCISSORS => Decision::RockPaperScissors { player },
        msg::ANNOUNCE_RACE => Decision::AnnounceRace { player, count: r.u8()?, available: r.u64()? },
        msg::ANNOUNCE_ATTRIB => Decision::AnnounceAttribute { player, count: r.u8()?, available: r.u32()? },
        msg::ANNOUNCE_NUMBER | msg::ANNOUNCE_CARD => {
            let n = r.u8()?;
            let options = (0..n).map(|_| r.u64()).collect::<Option<Vec<_>>>()?;
            if kind == msg::ANNOUNCE_NUMBER {
                Decision::AnnounceNumber { player, options }
            } else {
                Decision::AnnounceCard { player, opcodes: options }
            }
        }
        _ => Decision::Unsupported { message: kind },
    })
}

fn parse_event(kind: u8, r: &mut Reader) -> Option<Parsed> {
    let event = match kind {
        msg::RETRY => Event::Retry,
        msg::HINT => {
            let hint_type = r.u8()?;
            let _player = r.u8()?;
            let data = r.u64()?;
            return Some(if hint_type == hint::SELECTMSG { Parsed::SelectHint(data) } else { Parsed::Ignored });
        }
        msg::NEW_TURN => Event::NewTurn { player: r.u8()? },
        msg::NEW_PHASE => Event::NewPhase { phase: r.u16()? },
        msg::DRAW => {
            let player = r.u8()?;
            let n = r.u32()?;
            let codes = (0..n)
                .map(|_| {
                    let code = r.u32()?;
                    r.u32()?;
                    Some(code)
                })
                .collect::<Option<_>>()?;
            Event::Draw { player, codes }
        }
        msg::MOVE => {
            let code = r.u32()?;
            let from = r.loc_info(code)?;
            let to = r.loc_info(code)?;
            Event::Move { code, from, to, reason: r.u32()? }
        }
        msg::SUMMONING => Event::NormalSummoning { card: r.card_with_info()? },
        msg::SPSUMMONING => Event::SpecialSummoning { card: r.card_with_info()? },
        msg::CHAINING => {
            let card = r.card_with_info()?;
            let _triggering = (r.u8()?, r.u8()?, r.u32()?);
            let description = r.u64()?;
            Event::Chaining { card, description, chain_link: r.u32()? }
        }
        msg::CHAIN_SOLVING => Event::ChainSolving { chain_link: r.u8()? },
        msg::CHAIN_SOLVED => Event::ChainSolved { chain_link: r.u8()? },
        msg::CHAIN_END => Event::ChainEnd,
        msg::CHAIN_NEGATED => Event::ChainNegated { chain_link: r.u8()? },
        msg::CHAIN_DISABLED => Event::ChainDisabled { chain_link: r.u8()? },
        msg::BECOME_TARGET => {
            let n = r.u32()?;
            Event::BecomeTarget { cards: (0..n).map(|_| r.loc_info(0)).collect::<Option<_>>()? }
        }
        msg::SET => Event::Set { card: r.card_with_info()? },
        msg::FLIPSUMMONING => Event::FlipSummoning { card: r.card_with_info()? },
        msg::POS_CHANGE => {
            let card = CardRef { code: r.u32()?, controller: r.u8()?, location: r.u8()?, sequence: r.u8()? as u32, position: 0 };
            let from = r.u8()?;
            let to = r.u8()?;
            Event::PositionChange { card: CardRef { position: to as u32, ..card }, from, to }
        }
        msg::DAMAGE => Event::Damage { player: r.u8()?, amount: r.u32()? },
        msg::RECOVER => Event::Recover { player: r.u8()?, amount: r.u32()? },
        msg::PAY_LPCOST => Event::PayLpCost { player: r.u8()?, amount: r.u32()? },
        msg::LPUPDATE => Event::LpUpdate { player: r.u8()?, lp: r.u32()? },
        msg::ATTACK => {
            let attacker = r.loc_info(0)?;
            let target = r.loc_info(0)?;
            Event::Attack { attacker, target: (target.location != 0).then_some(target) }
        }
        msg::ADD_COUNTER | msg::REMOVE_COUNTER => {
            let counter_type = r.u16()?;
            let card = CardRef { code: 0, controller: r.u8()?, location: r.u8()?, sequence: r.u8()? as u32, position: 0 };
            let n = r.u16()? as i32;
            Event::Counter { card, counter_type, delta: if kind == msg::ADD_COUNTER { n } else { -n } }
        }
        msg::WIN => Event::Win { player: r.u8()?, reason: r.u8()? },
        _ => return Some(Parsed::Ignored),
    };
    Some(Parsed::Event(event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_length_prefixed_records() {
        let mut buf = Vec::new();
        for rec in [&[msg::NEW_TURN, 0][..], &[msg::NEW_PHASE, 4, 0][..]] {
            buf.extend_from_slice(&(rec.len() as u32).to_le_bytes());
            buf.extend_from_slice(rec);
        }
        let records = split(&buf);
        assert_eq!(records.len(), 2);
        assert!(matches!(parse(records[0], 0), Parsed::Event(Event::NewTurn { player: 0 })));
        assert!(matches!(parse(records[1], 0), Parsed::Event(Event::NewPhase { phase: 4 })));
    }

    #[test]
    fn parses_yes_no_decision() {
        let mut rec = vec![msg::SELECT_YESNO, 0];
        rec.extend_from_slice(&123u64.to_le_bytes());
        assert!(matches!(parse(&rec, 0), Parsed::Decision(Decision::YesNo { player: 0, description: 123 })));
    }

    #[test]
    fn truncated_decision_is_unsupported() {
        let rec = vec![msg::SELECT_YESNO, 0, 1];
        assert!(matches!(parse(&rec, 0), Parsed::Decision(Decision::Unsupported { message: msg::SELECT_YESNO })));
    }
}
