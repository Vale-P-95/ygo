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

    /// The locations of a mask (`LOCATION_*` bits, the Field and Pendulum
    /// Zones included), joined.
    pub fn names(mask: u32) -> String {
        let mut names: Vec<_> = (0..8).map(|bit| 1u8 << bit).filter(|&loc| mask & loc as u32 != 0).map(name).collect();
        if mask & 0x100 != 0 {
            names.push("Field Zone");
        }
        if mask & 0x200 != 0 {
            names.push("Pendulum Zone");
        }
        names.join(", ")
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
    pub const NORMAL: u32 = 0x10;
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
    pub const SUMMONED: u8 = 61;
    pub const SPSUMMONING: u8 = 62;
    pub const SPSUMMONED: u8 = 63;
    pub const FLIPSUMMONING: u8 = 64;
    pub const FLIPSUMMONED: u8 = 65;
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
    /// The select hint of a choice of monsters to Special Summon (`HINTMSG_SPSUMMON`).
    pub const MSG_SPSUMMON: u64 = 509;
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

/// Effect types (`EFFECT_TYPE_*` of `constant.lua`).
pub mod effect_type {
    pub const SINGLE: u32 = 0x1;
    pub const FIELD: u32 = 0x2;
    pub const EQUIP: u32 = 0x4;
    pub const ACTIVATE: u32 = 0x10;
    pub const FLIP: u32 = 0x20;
    pub const IGNITION: u32 = 0x40;
    pub const TRIGGER_O: u32 = 0x80;
    pub const QUICK_O: u32 = 0x100;
    pub const TRIGGER_F: u32 = 0x200;
    pub const QUICK_F: u32 = 0x400;
    pub const CONTINUOUS: u32 = 0x800;
    pub const XMATERIAL: u32 = 0x1000;
    pub const GRANT: u32 = 0x2000;
}

/// What an effect declares it does (`CATEGORY_*` of `constant.lua`), in bit
/// order, with the names the anatomy prints.
pub mod category {
    pub const DESTROY: u64 = 0x1;
    pub const REMOVE: u64 = 0x4;
    pub const TOHAND: u64 = 0x8;
    pub const TODECK: u64 = 0x10;
    pub const SPECIAL_SUMMON: u64 = 0x200;
    pub const DISABLE: u64 = 0x4000;
    pub const DISABLE_SUMMON: u64 = 0x8000;
    pub const SEARCH: u64 = 0x2_0000;
    pub const NEGATE: u64 = 0x1000_0000;

    pub const NAMES: [&str; 33] = [
        "destroy",
        "release",
        "banish",
        "to hand",
        "to deck",
        "to GY",
        "mill",
        "discard",
        "Normal Summon",
        "Special Summon",
        "token",
        "flip",
        "position",
        "control",
        "negate effect",
        "negate summon",
        "draw",
        "search",
        "equip",
        "damage",
        "recover",
        "ATK change",
        "DEF change",
        "counter",
        "coin",
        "dice",
        "leave GY",
        "level change",
        "negate activation",
        "announce",
        "Fusion Summon",
        "to Extra Deck",
        "set",
    ];

    /// The names of the categories in `mask`.
    pub fn names(mask: u64) -> Vec<&'static str> {
        NAMES.iter().enumerate().filter(|(bit, _)| mask & (1 << bit) != 0).map(|(_, name)| *name).collect()
    }
}

/// Effect codes of the summoning procedures and conditions (`EFFECT_*`).
pub mod effect_code {
    pub const SPSUMMON_CONDITION: u32 = 30;
    pub const REVIVE_LIMIT: u32 = 31;
    pub const SPSUMMON_PROC: u32 = 34;
    pub const FUSION_MATERIAL: u32 = 230;
    pub const SPSUMMON_PROC_G: u32 = 320;
}

/// Events that trigger effects (`EVENT_*`), the ones the anatomy names.
pub mod event {
    pub const FREE_CHAIN: u32 = 1002;
    pub const REMOVE: u32 = 1011;
    pub const TO_HAND: u32 = 1012;
    pub const TO_GRAVE: u32 = 1014;
    pub const LEAVE_FIELD: u32 = 1015;
    pub const CHAINING: u32 = 1027;
    pub const DESTROYED: u32 = 1029;
    pub const SUMMON_SUCCESS: u32 = 1100;
    pub const FLIP_SUMMON_SUCCESS: u32 = 1101;
    pub const SPSUMMON_SUCCESS: u32 = 1102;
    pub const SUMMON: u32 = 1103;
    pub const SPSUMMON: u32 = 1105;
    pub const BE_MATERIAL: u32 = 1108;
    pub const ATTACK_ANNOUNCE: u32 = 1130;
    pub const PHASE: u32 = 0x1000;

    pub fn name(code: u32) -> Option<&'static str> {
        Some(match code {
            FREE_CHAIN => "any time",
            REMOVE => "banished",
            TO_HAND => "added to the hand",
            TO_GRAVE => "sent to the GY",
            LEAVE_FIELD => "leaves the field",
            CHAINING => "an effect is activated",
            DESTROYED => "destroyed",
            SUMMON_SUCCESS => "Normal Summoned",
            FLIP_SUMMON_SUCCESS => "Flip Summoned",
            SPSUMMON_SUCCESS => "Special Summoned",
            SUMMON => "a monster would be Normal Summoned",
            SPSUMMON => "a monster would be Special Summoned",
            BE_MATERIAL => "used as material",
            ATTACK_ANNOUNCE => "an attack is declared",
            c if c & PHASE != 0 && c < 0x2000 => "phase",
            _ => return None,
        })
    }
}
