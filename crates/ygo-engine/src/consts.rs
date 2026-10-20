//! Constants from `ocgapi_constants.h` used by the bindings.

pub mod location {
    pub const DECK: u8 = 0x01;
    pub const HAND: u8 = 0x02;
    pub const MZONE: u8 = 0x04;
    pub const SZONE: u8 = 0x08;
    pub const GRAVE: u8 = 0x10;
    pub const REMOVED: u8 = 0x20;
    pub const EXTRA: u8 = 0x40;
    pub const OVERLAY: u8 = 0x80;

    pub fn name(loc: u8) -> &'static str {
        match loc & !OVERLAY {
            DECK => "Deck",
            HAND => "Hand",
            MZONE => "Monster Zone",
            SZONE => "Spell/Trap Zone",
            GRAVE => "GY",
            REMOVED => "Banished",
            EXTRA => "Extra Deck",
            0 => "-",
            _ => "?",
        }
    }
}

pub mod position {
    pub const FACEUP_ATTACK: u32 = 0x1;
    pub const FACEDOWN_ATTACK: u32 = 0x2;
    pub const FACEUP_DEFENSE: u32 = 0x4;
    pub const FACEDOWN_DEFENSE: u32 = 0x8;

    pub fn name(pos: u32) -> &'static str {
        match pos {
            FACEUP_ATTACK => "face-up Attack",
            FACEDOWN_ATTACK => "face-down Attack",
            FACEUP_DEFENSE => "face-up Defense",
            FACEDOWN_DEFENSE => "face-down Defense",
            // Spells and Traps report both bits of a side.
            p if p & (FACEUP_ATTACK | FACEUP_DEFENSE) != 0 => "face-up",
            _ => "face-down",
        }
    }
}

/// Why a card moved (`Event::Move::reason`).
pub mod reason {
    pub const DESTROY: u32 = 0x1;
    pub const RELEASE: u32 = 0x2;
    pub const MATERIAL: u32 = 0x8;
    pub const EFFECT: u32 = 0x40;
    pub const BATTLE: u32 = 0x20;
    pub const COST: u32 = 0x80;
    pub const DISCARD: u32 = 0x4000;
    pub const FUSION: u32 = 0x4_0000;
    pub const SYNCHRO: u32 = 0x8_0000;
    pub const RITUAL: u32 = 0x10_0000;
    pub const XYZ: u32 = 0x20_0000;
    pub const LINK: u32 = 0x1000_0000;
}

/// Kinds of summon (`SUMMON_TYPE_*` of the scripts).
pub mod summon_type {
    pub const FUSION: u32 = 0x4300_0000;
    pub const SYNCHRO: u32 = 0x4600_0000;
    pub const XYZ: u32 = 0x4900_0000;
    pub const LINK: u32 = 0x4c00_0000;
}

pub mod card_type {
    pub const MONSTER: u32 = 0x1;
    pub const SPELL: u32 = 0x2;
    pub const TRAP: u32 = 0x4;
    pub const FUSION: u32 = 0x40;
    pub const QUICKPLAY: u32 = 0x1_0000;
    pub const CONTINUOUS: u32 = 0x2_0000;
    pub const EQUIP: u32 = 0x4_0000;
    pub const FIELD: u32 = 0x8_0000;
    pub const RITUAL: u32 = 0x80;
    pub const SYNCHRO: u32 = 0x2000;
    pub const TOKEN: u32 = 0x4000;
    pub const XYZ: u32 = 0x80_0000;
    pub const PENDULUM: u32 = 0x100_0000;
    pub const LINK: u32 = 0x400_0000;
    /// Monsters that live in the Extra Deck.
    pub const EXTRA: u32 = FUSION | SYNCHRO | XYZ | LINK;
}

pub mod msg {
    pub const RETRY: u8 = 1;
    pub const HINT: u8 = 2;
    pub const WIN: u8 = 5;
    pub const SELECT_BATTLECMD: u8 = 10;
    pub const SELECT_IDLECMD: u8 = 11;
    pub const SELECT_EFFECTYN: u8 = 12;
    pub const SELECT_YESNO: u8 = 13;
    pub const SELECT_OPTION: u8 = 14;
    pub const SELECT_CARD: u8 = 15;
    pub const SELECT_CHAIN: u8 = 16;
    pub const SELECT_PLACE: u8 = 18;
    pub const SELECT_POSITION: u8 = 19;
    pub const SELECT_TRIBUTE: u8 = 20;
    pub const SORT_CHAIN: u8 = 21;
    pub const SELECT_COUNTER: u8 = 22;
    pub const SELECT_SUM: u8 = 23;
    pub const SELECT_DISFIELD: u8 = 24;
    pub const SORT_CARD: u8 = 25;
    pub const SELECT_UNSELECT_CARD: u8 = 26;
    pub const NEW_TURN: u8 = 40;
    pub const NEW_PHASE: u8 = 41;
    pub const MOVE: u8 = 50;
    pub const POS_CHANGE: u8 = 53;
    pub const SET: u8 = 54;
    pub const SUMMONING: u8 = 60;
    pub const SPSUMMONING: u8 = 62;
    pub const SPSUMMONED: u8 = 63;
    pub const FLIPSUMMONING: u8 = 64;
    pub const CHAINING: u8 = 70;
    pub const CHAIN_SOLVING: u8 = 72;
    pub const CHAIN_SOLVED: u8 = 73;
    pub const CHAIN_END: u8 = 74;
    pub const CHAIN_NEGATED: u8 = 75;
    pub const CHAIN_DISABLED: u8 = 76;
    pub const BECOME_TARGET: u8 = 83;
    pub const DRAW: u8 = 90;
    pub const DAMAGE: u8 = 91;
    pub const RECOVER: u8 = 92;
    pub const LPUPDATE: u8 = 94;
    pub const PAY_LPCOST: u8 = 100;
    pub const ADD_COUNTER: u8 = 101;
    pub const REMOVE_COUNTER: u8 = 102;
    pub const ATTACK: u8 = 110;
    pub const ROCK_PAPER_SCISSORS: u8 = 132;
    pub const ANNOUNCE_RACE: u8 = 140;
    pub const ANNOUNCE_ATTRIB: u8 = 141;
    pub const ANNOUNCE_CARD: u8 = 142;
    pub const ANNOUNCE_NUMBER: u8 = 143;
}

pub mod hint {
    pub const SELECTMSG: u8 = 3;
}

pub mod duel_flags {
    pub const PSEUDO_SHUFFLE: u64 = 0x10;
    pub const SIMPLE_AI: u64 = 0x40;
    pub const PZONE: u64 = 0x800;
    pub const EMZONE: u64 = 0x2000;
    pub const FSX_MMZONE: u64 = 0x4000;
    pub const TRAP_MONSTERS_NOT_USE_ZONE: u64 = 0x8000;
    pub const TRIGGER_ONLY_IN_LOCATION: u64 = 0x20000;
    /// Master Rule 5 (April 2020), used by TCG, OCG and Master Duel.
    pub const MODE_MR5: u64 = PZONE | EMZONE | FSX_MMZONE | TRAP_MONSTERS_NOT_USE_ZONE | TRIGGER_ONLY_IN_LOCATION;
}

pub mod phase {
    pub fn name(phase: u16) -> &'static str {
        match phase {
            0x01 => "Draw Phase",
            0x02 => "Standby Phase",
            0x04 => "Main Phase 1",
            0x08 | 0x10 | 0x20 | 0x40 | 0x80 => "Battle Phase",
            0x100 => "Main Phase 2",
            0x200 => "End Phase",
            _ => "?",
        }
    }
}
