//! Card declarations ("declare 1 card name"). The engine sends a small stack
//! program that says which names are valid; this is a port of
//! `is_declarable` in `playerop.cpp`, so the names offered are exactly the
//! ones the engine accepts.

use crate::carddb::Card;
use crate::consts::card_type;

mod op {
    pub const ADD: u64 = 0x4000_0000_0000_0000;
    pub const SUB: u64 = 0x4000_0001_0000_0000;
    pub const MUL: u64 = 0x4000_0002_0000_0000;
    pub const DIV: u64 = 0x4000_0003_0000_0000;
    pub const AND: u64 = 0x4000_0004_0000_0000;
    pub const OR: u64 = 0x4000_0005_0000_0000;
    pub const NEG: u64 = 0x4000_0006_0000_0000;
    pub const NOT: u64 = 0x4000_0007_0000_0000;
    pub const BAND: u64 = 0x4000_0008_0000_0000;
    pub const BOR: u64 = 0x4000_0009_0000_0000;
    pub const BNOT: u64 = 0x4000_0010_0000_0000;
    pub const BXOR: u64 = 0x4000_0011_0000_0000;
    pub const LSHIFT: u64 = 0x4000_0012_0000_0000;
    pub const RSHIFT: u64 = 0x4000_0013_0000_0000;
    pub const ALLOW_ALIASES: u64 = 0x4000_0014_0000_0000;
    pub const ALLOW_TOKENS: u64 = 0x4000_0015_0000_0000;
    pub const ISCODE: u64 = 0x4000_0100_0000_0000;
    pub const ISSETCARD: u64 = 0x4000_0101_0000_0000;
    pub const ISTYPE: u64 = 0x4000_0102_0000_0000;
    pub const ISRACE: u64 = 0x4000_0103_0000_0000;
    pub const ISATTRIBUTE: u64 = 0x4000_0104_0000_0000;
    pub const GETCODE: u64 = 0x4000_0105_0000_0000;
    pub const GETTYPE: u64 = 0x4000_0107_0000_0000;
    pub const GETRACE: u64 = 0x4000_0108_0000_0000;
    pub const GETATTRIBUTE: u64 = 0x4000_0109_0000_0000;
}

/// Cards that can always be declared (their names are treated specially).
const MARINE_DOLPHIN: u32 = 78734254;
const TWINKLE_MOSS: u32 = 13857930;

/// Whether `card` may be declared under the filter `opcodes`.
pub fn is_declarable(card: &Card, opcodes: &[u64]) -> bool {
    let mut stack: Vec<i64> = Vec::new();
    let (mut alias, mut token) = (false, false);
    let binary = |stack: &mut Vec<i64>, f: fn(i64, i64) -> i64| {
        if stack.len() >= 2 {
            let rhs = stack.pop().unwrap_or_default();
            let lhs = stack.pop().unwrap_or_default();
            stack.push(f(lhs, rhs));
        }
    };
    let unary = |stack: &mut Vec<i64>, f: &dyn Fn(i64) -> i64| {
        if let Some(v) = stack.pop() {
            stack.push(f(v));
        }
    };
    for &opcode in opcodes {
        match opcode {
            op::ADD => binary(&mut stack, i64::wrapping_add),
            op::SUB => binary(&mut stack, i64::wrapping_sub),
            op::MUL => binary(&mut stack, i64::wrapping_mul),
            op::DIV => binary(&mut stack, |a, b| if b == 0 { 0 } else { a.wrapping_div(b) }),
            op::AND => binary(&mut stack, |a, b| (a != 0 && b != 0) as i64),
            op::OR => binary(&mut stack, |a, b| (a != 0 || b != 0) as i64),
            op::NEG => unary(&mut stack, &|v| v.wrapping_neg()),
            op::NOT => unary(&mut stack, &|v| (v == 0) as i64),
            op::BAND => binary(&mut stack, |a, b| a & b),
            op::BOR => binary(&mut stack, |a, b| a | b),
            op::BXOR => binary(&mut stack, |a, b| a ^ b),
            op::BNOT => unary(&mut stack, &|v| !v),
            op::LSHIFT => binary(&mut stack, |a, b| a.checked_shl(b as u32).unwrap_or(0)),
            op::RSHIFT => binary(&mut stack, |a, b| a.checked_shr(b as u32).unwrap_or(0)),
            op::ISCODE => unary(&mut stack, &|v| (card.code == v as u32) as i64),
            op::ISTYPE => unary(&mut stack, &|v| card.type_ as i64 & v),
            op::ISRACE => unary(&mut stack, &|v| card.race as i64 & v),
            op::ISATTRIBUTE => unary(&mut stack, &|v| card.attribute as i64 & v),
            op::GETCODE => stack.push(card.code as i64),
            op::GETTYPE => stack.push(card.type_ as i64),
            op::GETRACE => stack.push(card.race as i64),
            op::GETATTRIBUTE => stack.push(card.attribute as i64),
            op::ISSETCARD => unary(&mut stack, &|v| {
                let set_code = v as i32;
                let (set_type, sub_type) = ((set_code & 0xfff) as u16, (set_code & 0xf000) as u16);
                card.setcodes.iter().any(|&sc| sc & 0xfff == set_type && sc & 0xf000 & sub_type == sub_type) as i64
            }),
            op::ALLOW_ALIASES => alias = true,
            op::ALLOW_TOKENS => token = true,
            // Anything else (GETSETCARD included, as upstream) is a literal.
            value => stack.push(value as i64),
        }
    }
    if stack.len() != 1 || stack[0] == 0 {
        return false;
    }
    let monster_token = card_type::MONSTER | card_type::TOKEN;
    card.code == MARINE_DOLPHIN
        || card.code == TWINKLE_MOSS
        || ((alias || card.alias == 0) && (token || card.type_ & monster_token != monster_token))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(code: u32, type_: u32, setcodes: Vec<u16>) -> Card {
        Card { code, type_, setcodes, ..Default::default() }
    }

    #[test]
    fn filters_like_the_engine() {
        let spell = card(1, card_type::SPELL, vec![]);
        let monster = card(2, card_type::MONSTER, vec![0x10f4]);
        let token = card(3, card_type::MONSTER | card_type::TOKEN, vec![]);
        let alt_art = Card { alias: 2, ..card(4, card_type::MONSTER, vec![]) };
        // "Declare a monster card": TYPE_MONSTER, ISTYPE.
        let is_monster = [card_type::MONSTER as u64, op::ISTYPE];
        assert!(!is_declarable(&spell, &is_monster));
        assert!(is_declarable(&monster, &is_monster));
        assert!(!is_declarable(&token, &is_monster), "tokens only with ALLOW_TOKENS");
        assert!(is_declarable(&token, &[card_type::MONSTER as u64, op::ISTYPE, op::ALLOW_TOKENS]));
        assert!(!is_declarable(&alt_art, &is_monster), "alternate artworks only with ALLOW_ALIASES");
        // "Not a Spell": TYPE_SPELL, ISTYPE, NOT.
        assert!(is_declarable(&monster, &[card_type::SPELL as u64, op::ISTYPE, op::NOT]));
        // Archetype 0xf4 matches sub-archetype 0x10f4, not the reverse.
        assert!(is_declarable(&monster, &[0xf4, op::ISSETCARD]));
        assert!(!is_declarable(&card(5, card_type::MONSTER, vec![0xf4]), &[0x10f4, op::ISSETCARD]));
        // "Code 2 or a Spell".
        assert!(is_declarable(&spell, &[2, op::ISCODE, card_type::SPELL as u64, op::ISTYPE, op::OR]));
        // A malformed program (two values left) declares nothing.
        assert!(!is_declarable(&monster, &[1, 1]));
    }
}
