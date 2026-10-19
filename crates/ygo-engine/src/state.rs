//! The state of a duel read from the engine, and its fingerprint: two lines
//! that reach the same state in a different order get the same fingerprint,
//! so the search explores what follows only once.

use crate::consts::location;
use crate::duel::Duel;
use crate::message::Reader;

mod query {
    pub const CODE: u32 = 0x1;
    pub const POSITION: u32 = 0x2;
    pub const TYPE: u32 = 0x8;
    pub const LEVEL: u32 = 0x10;
    pub const RANK: u32 = 0x20;
    pub const ATTRIBUTE: u32 = 0x40;
    pub const RACE: u32 = 0x80;
    pub const ATTACK: u32 = 0x100;
    pub const DEFENSE: u32 = 0x200;
    pub const EQUIP_CARD: u32 = 0x4000;
    pub const OVERLAY_CARD: u32 = 0x1_0000;
    pub const COUNTERS: u32 = 0x2_0000;
    pub const OWNER: u32 = 0x4_0000;
    pub const STATUS: u32 = 0x8_0000;
    pub const LINK: u32 = 0x80_0000;
    pub const END: u32 = 0x8000_0000;
}

/// Engine status bit: the card's effects are negated.
pub const STATUS_DISABLED: u32 = 0x1;

/// A card on the field, with what effects changed about it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FieldCard {
    pub code: u32,
    pub position: u32,
    pub type_: u32,
    pub level: u32,
    pub rank: u32,
    pub attribute: u32,
    pub race: u64,
    pub attack: i32,
    pub defense: i32,
    /// Link rating and arrows.
    pub link: (u32, u32),
    /// Xyz materials, by code.
    pub overlay: Vec<u32>,
    /// `(counter type, count)`.
    pub counters: Vec<(u16, u16)>,
    pub owner: u8,
    pub status: u32,
    /// The card this one is equipped to: controller, location, sequence.
    pub equipped_to: Option<(u8, u8, u32)>,
}

impl FieldCard {
    pub fn is_face_up(&self) -> bool {
        self.position & 0x5 != 0
    }

    pub fn is_negated(&self) -> bool {
        self.status & STATUS_DISABLED != 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerState {
    pub lp: i32,
    pub hand: Vec<u32>,
    /// From the bottom to the top.
    pub deck: Vec<u32>,
    /// `(code, face-up)`: face-up Pendulum monsters are public.
    pub extra: Vec<(u32, bool)>,
    pub grave: Vec<u32>,
    /// `(code, face-up)`.
    pub banished: Vec<(u32, bool)>,
    /// Main Monster Zones 0-4, then the Extra Monster Zones 5 and 6.
    pub monsters: Vec<Option<FieldCard>>,
    /// Spell & Trap Zones 0-4, the Field Zone 5, and the old Pendulum Zones 6 and 7.
    pub spells: Vec<Option<FieldCard>>,
}

/// Everything the engine shows about a duel, plus what happened this turn
/// that changes what can still be done (see [`TurnActivity`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DuelState {
    pub players: [PlayerState; 2],
    pub turn: u32,
    pub turn_player: u8,
    pub phase: u16,
    pub activity: TurnActivity,
}

/// What was done this turn. The engine does not tell which "once per turn"
/// effects were used, but two lines that activated the same effects and
/// summoned the same monsters have used the same ones.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TurnActivity {
    /// `(code, effect description)` of each activation, sorted.
    pub activations: Vec<(u32, u64)>,
    /// Codes of the monsters Special Summoned, sorted.
    pub special_summons: Vec<u32>,
    pub normal_summon_used: bool,
}

/// A card's raw query answer.
fn parse_card(r: &mut Reader, first_size: u16) -> Option<FieldCard> {
    let mut card = FieldCard::default();
    let mut size = first_size;
    loop {
        let flag = r.u32()?;
        let body = size as usize - 4;
        match flag {
            query::CODE => card.code = r.u32()?,
            query::POSITION => card.position = r.u32()?,
            query::TYPE => card.type_ = r.u32()?,
            query::LEVEL => card.level = r.u32()?,
            query::RANK => card.rank = r.u32()?,
            query::ATTRIBUTE => card.attribute = r.u32()?,
            query::RACE => card.race = r.u64()?,
            query::ATTACK => card.attack = r.u32()? as i32,
            query::DEFENSE => card.defense = r.u32()? as i32,
            query::OWNER => card.owner = r.u8()?,
            query::STATUS => card.status = r.u32()?,
            query::LINK => card.link = (r.u32()?, r.u32()?),
            query::OVERLAY_CARD => {
                let n = r.u32()?;
                card.overlay = (0..n).map(|_| r.u32()).collect::<Option<_>>()?;
            }
            query::COUNTERS => {
                let n = r.u32()?;
                card.counters = (0..n)
                    .map(|_| {
                        let v = r.u32()?;
                        Some(((v & 0xffff) as u16, (v >> 16) as u16))
                    })
                    .collect::<Option<_>>()?;
            }
            query::EQUIP_CARD => {
                let (controller, loc, sequence, _position) = (r.u8()?, r.u8()?, r.u32()?, r.u32()?);
                card.equipped_to = (loc != 0).then_some((controller, loc, sequence));
            }
            query::END => return Some(card),
            _ => r.skip(body)?,
        }
        size = r.u16()?;
    }
}

const FIELD_FLAGS: u32 = query::CODE
    | query::POSITION
    | query::TYPE
    | query::LEVEL
    | query::RANK
    | query::ATTRIBUTE
    | query::RACE
    | query::ATTACK
    | query::DEFENSE
    | query::EQUIP_CARD
    | query::OVERLAY_CARD
    | query::COUNTERS
    | query::OWNER
    | query::STATUS
    | query::LINK;

const PILE_FLAGS: u32 = query::CODE | query::POSITION;

impl Duel {
    /// Cards in one location of one player, in engine order; empty zones are `None`.
    pub fn query_cards(&self, player: u8, loc: u8, flags: u32) -> Vec<Option<FieldCard>> {
        let buffer = self.raw_query_location(player, loc, flags);
        let mut r = Reader::new(&buffer);
        let mut out = Vec::new();
        if r.u32().is_none() {
            return out;
        }
        while r.remaining() >= 2 {
            let Some(size) = r.u16() else { break };
            if size == 0 {
                out.push(None);
                continue;
            }
            match parse_card(&mut r, size) {
                Some(card) => out.push(Some(card)),
                None => break,
            }
        }
        out
    }

    /// The whole state of the duel.
    pub fn state(&self) -> DuelState {
        let lp = self.life_points();
        let player = |p: u8| {
            let codes = |loc| self.query_cards(p, loc, PILE_FLAGS).into_iter().flatten().collect::<Vec<_>>();
            let face = |cards: Vec<FieldCard>| cards.into_iter().map(|c| (c.code, c.is_face_up())).collect();
            PlayerState {
                lp: lp[p as usize],
                hand: codes(location::HAND).into_iter().map(|c| c.code).collect(),
                deck: codes(location::DECK).into_iter().map(|c| c.code).collect(),
                extra: face(codes(location::EXTRA)),
                grave: codes(location::GRAVE).into_iter().map(|c| c.code).collect(),
                banished: face(codes(location::REMOVED)),
                monsters: self.query_cards(p, location::MZONE, FIELD_FLAGS),
                spells: self.query_cards(p, location::SZONE, FIELD_FLAGS),
            }
        };
        let (turn, turn_player, phase) = self.turn_info();
        DuelState { players: [player(0), player(1)], turn, turn_player, phase, activity: self.activity().clone() }
    }

    /// Life points of both players.
    pub fn life_points(&self) -> [i32; 2] {
        let buffer = self.raw_query_field();
        let mut r = Reader::new(&buffer);
        let mut lp = [0; 2];
        if r.u32().is_none() {
            return lp;
        }
        for slot in &mut lp {
            let Some(v) = r.u32() else { return lp };
            *slot = v as i32;
            // 7 monster and 8 spell zones: flag, then position and material count when occupied.
            for _ in 0..15 {
                match r.u8() {
                    Some(1) => {
                        if r.skip(5).is_none() {
                            return lp;
                        }
                    }
                    Some(_) => {}
                    None => return lp,
                }
            }
            if r.skip(6 * 4).is_none() {
                return lp;
            }
        }
        lp
    }

    /// Fingerprint of the state: equal states, equal fingerprints. Piles are
    /// compared as sets of cards (their order is the deck order or history),
    /// zones exactly, since zones and Link arrows matter.
    pub fn fingerprint(&self) -> u64 {
        fingerprint(&self.state())
    }
}

/// FNV-1a over a canonical encoding of the state.
pub fn fingerprint(state: &DuelState) -> u64 {
    let mut h = Fnv::default();
    h.u32(state.turn);
    h.u32(state.turn_player as u32);
    h.u32(state.phase as u32);
    for p in &state.players {
        h.u32(p.lp as u32);
        let mut sorted = |codes: &[u32]| {
            let mut v = codes.to_vec();
            v.sort_unstable();
            h.u32(v.len() as u32);
            v.iter().for_each(|&c| h.u32(c));
        };
        sorted(&p.hand);
        sorted(&p.deck);
        sorted(&p.grave);
        for pile in [&p.extra, &p.banished] {
            let mut v = pile.clone();
            v.sort_unstable();
            h.u32(v.len() as u32);
            v.iter().for_each(|&(c, up)| h.u32(c ^ (up as u32) << 31));
        }
        for zone in p.monsters.iter().chain(&p.spells) {
            match zone {
                None => h.u32(0),
                Some(c) => {
                    for v in [
                        c.code,
                        c.position,
                        c.type_,
                        c.level,
                        c.rank,
                        c.attribute,
                        c.attack as u32,
                        c.defense as u32,
                        c.link.0,
                        c.link.1,
                        c.owner as u32,
                        c.status,
                    ] {
                        h.u32(v);
                    }
                    h.u32((c.race >> 32) as u32);
                    h.u32(c.race as u32);
                    let mut overlay = c.overlay.clone();
                    overlay.sort_unstable();
                    h.u32(overlay.len() as u32);
                    overlay.iter().for_each(|&o| h.u32(o));
                    h.u32(c.counters.len() as u32);
                    c.counters.iter().for_each(|&(t, n)| h.u32((t as u32) << 16 | n as u32));
                    let (ec, el, es) = c.equipped_to.unwrap_or_default();
                    h.u32((ec as u32) << 24 | (el as u32) << 16 | es);
                }
            }
        }
    }
    let a = &state.activity;
    h.u32(a.activations.len() as u32);
    for &(code, desc) in &a.activations {
        h.u32(code);
        h.u32((desc >> 32) as u32);
        h.u32(desc as u32);
    }
    h.u32(a.special_summons.len() as u32);
    a.special_summons.iter().for_each(|&c| h.u32(c));
    h.u32(a.normal_summon_used as u32);
    h.0
}

struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn u32(&mut self, v: u32) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}
