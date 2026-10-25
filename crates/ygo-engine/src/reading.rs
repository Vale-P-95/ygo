//! The reading duels of the synergies (plan `docs/plans/sinergie-archetipi.md`
//! §3.3): what the materials of each Extra Deck monster accept, and the
//! limits an effect puts on its player, both told by the scripts' own
//! filters run on every card of a duel holding the whole pool. A Lua file
//! loaded only in those duels (`lua/reading.lua`) wraps the summon
//! procedures and `Duel.RegisterEffect` and reports to the log, and gives
//! a card of the hand effects that move the card read (helpers), so nothing
//! of the engine or of a real duel changes, as in [`crate::anatomy`].

use crate::consts::{location, position};
use crate::duel::{Duel, DuelConfig, Engine};

const WRAPPER: &str = include_str!("../lua/reading.lua");

/// One material of an Extra Deck monster's summon procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialSlot {
    /// "fusion", "synchro", "xyz", "xyz on a monster", "link".
    pub kind: String,
    /// Which procedure of the card (a card can have more than one: an Xyz
    /// Summon and one on a monster) and which material of it, from 1.
    pub procedure: u32,
    pub slot: u32,
    /// How many cards it takes.
    pub min: u32,
    pub max: u32,
    /// The procedure checks more than each material alone (a Level sum, "
    /// including a Tuner of...", what the materials share): the cards
    /// accepted may not all go together.
    pub checked: bool,
    /// Calls of the filter that failed (a Lua error): those cards count as
    /// not accepted.
    pub errors: u32,
    /// The cards of the pool it accepts, sorted.
    pub cards: Vec<u32>,
}

/// The materials of one Extra Deck monster.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaterialReading {
    pub code: u32,
    /// Empty when the script hands no filter to a known procedure (its own
    /// summon procedure, a material the procedures do not take).
    pub slots: Vec<MaterialSlot>,
    pub error: Option<String>,
}

/// A limit an effect put on its player: no Normal Summon (`code` 20) or
/// no Special Summon (22), of the cards in `forbidden`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLimit {
    /// The card whose effect set it.
    pub owner: u32,
    pub code: u32,
    pub errors: u32,
    pub forbidden: Vec<u32>,
}

/// The cards of `order` whose bit is set in `hex` (bit `i` of the bitmap is
/// card `i`, four to a digit, lowest first). `None` if the duel had another
/// number of cards than `order`.
pub fn decode_bitmap(hex: &str, count: usize, order: &[u32]) -> Option<Vec<u32>> {
    if count != order.len() || hex.len() != count.div_ceil(4) {
        return None;
    }
    let mut cards = Vec::new();
    for (i, digit) in hex.chars().enumerate() {
        let nibble = digit.to_digit(16)?;
        for bit in 0..4 {
            if nibble & (1 << bit) != 0 {
                cards.push(*order.get(i * 4 + bit)?);
            }
        }
    }
    Some(cards)
}

/// The materials of each card of `codes` (Extra Deck monsters of `extra`),
/// in a duel holding `main` and `extra`. The cards a material accepts are
/// monsters of either. Every code of `main` and `extra` once.
pub fn read_materials(engine: &Engine, main: &[u32], extra: &[u32], codes: &[u32]) -> Result<Vec<MaterialReading>, crate::Error> {
    let mut order: Vec<u32> = main.iter().chain(extra).copied().collect();
    order.sort_unstable();
    order.dedup();
    let mut duel = Duel::fresh(engine, &DuelConfig::default(), std::ptr::null_mut())?;
    if !duel.run_lua_source("reading.lua", WRAPPER) {
        return Err(crate::Error::Engine(format!("cannot load reading.lua: {}", duel.log().join("; "))));
    }
    for (i, &code) in main.iter().enumerate() {
        duel.add_card(0, code, location::DECK, i as u32, position::FACEDOWN_DEFENSE);
        duel.take_log();
    }
    for (i, &code) in extra.iter().enumerate() {
        duel.add_card(0, code, location::EXTRA, i as u32, position::FACEDOWN_DEFENSE);
        duel.take_log();
    }
    duel.run_lua_source("reading.lua", "df_reset()");
    let mut out = Vec::with_capacity(codes.len());
    for &code in codes {
        let mut reading = MaterialReading { code, ..MaterialReading::default() };
        if !duel.run_lua_source("reading.lua", &format!("df_mat_report({code})")) {
            reading.error = Some(duel.take_log().join("; "));
            out.push(reading);
            continue;
        }
        let mut errors = Vec::new();
        for line in duel.take_log() {
            match parse_material(&line, &order) {
                Some(Ok(slot)) => reading.slots.push(slot),
                Some(Err(e)) => errors.push(e),
                None if line.starts_with("dfm ") => {}
                None => errors.push(line),
            }
        }
        reading.error = (!errors.is_empty()).then(|| errors.join("; "));
        out.push(reading);
    }
    Ok(out)
}

/// A line of `df_mat_report`: `None` for a card with no procedure read or
/// a line of something else.
fn parse_material(line: &str, order: &[u32]) -> Option<Result<MaterialSlot, String>> {
    let fields: Vec<&str> = line.strip_prefix("dfm ")?.split(' ').collect();
    if fields.len() != 10 {
        return None;
    }
    let n = |i: usize| fields[i].parse::<u32>().ok();
    let (Some(procedure), Some(slot), Some(min), Some(max), Some(checked), Some(errors), Some(count)) =
        (n(1), n(2), n(4), n(5), n(6), n(7), n(8))
    else {
        return Some(Err(format!("unreadable: {line}")));
    };
    let Some(cards) = decode_bitmap(fields[9], count as usize, order) else {
        return Some(Err(format!("the duel had {count} cards, the pool {}", order.len())));
    };
    Some(Ok(MaterialSlot { kind: fields[3].replace('_', " "), procedure, slot, min, max, checked: checked != 0, errors, cards }))
}

/// The description of a helper effect is this plus the event it makes
/// happen ([`add_helpers`]).
pub const HELPER: u64 = 0x7df0_0000;

/// The events a helper can make happen to the card read: sent to the GY,
/// discarded, Special or Normal Summoned, banished, destroyed, leaving the
/// field, added to the hand, returned to the Deck, Tributed, flipped.
pub const HELPER_EVENTS: [u32; 11] = [1014, 1018, 1102, 1100, 1011, 1029, 1015, 1012, 1013, 1017, 1001];

/// Gives `holder` (a card in player 0's hand) one ignition effect for each
/// event of `events` that moves `probe` so that the event happens to it:
/// its triggers then answer as in a duel. The effects are described by
/// [`HELPER`] plus the event. Needs [`watch_limits`] (the same file) first.
pub fn add_helpers(duel: &Duel, probe: u32, holder: u32, events: &[u32]) -> bool {
    let events: Vec<String> = events.iter().filter(|e| HELPER_EVENTS.contains(e)).map(u32::to_string).collect();
    if events.is_empty() {
        return true;
    }
    duel.run_lua_source("reading.lua", &format!("df_helpers({probe},{holder},{})", events.join(",")))
}

/// Loads in `duel` what reports the limits put on player 0
/// ([`take_limits`]). False if it does not load.
pub fn watch_limits(duel: &Duel) -> bool {
    duel.run_lua_source("reading.lua", WRAPPER)
}

/// The limits put on player 0 since the last call, with the cards they
/// forbid among `order` (every card code of the duel once, sorted). Takes
/// the duel's log: its other lines are dropped.
pub fn take_limits(duel: &Duel, order: &[u32]) -> Vec<PlayerLimit> {
    let mut out = Vec::new();
    for line in duel.take_log() {
        let Some(rest) = line.strip_prefix("dfl ") else { continue };
        let fields: Vec<&str> = rest.split(' ').collect();
        if fields.len() != 5 {
            continue;
        }
        let n = |i: usize| fields[i].parse::<u32>().ok();
        let (Some(owner), Some(code), Some(errors), Some(count)) = (n(0), n(1), n(2), n(3)) else { continue };
        if let Some(forbidden) = decode_bitmap(fields[4], count as usize, order) {
            out.push(PlayerLimit { owner, code, errors, forbidden });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_bitmap_lowest_bit_first() {
        let order = [10, 20, 30, 40, 50, 60];
        // 0b0101 then 0b0010: cards 0, 2 and 5.
        assert_eq!(decode_bitmap("52", 6, &order), Some(vec![10, 30, 60]));
        assert_eq!(decode_bitmap("52", 5, &order), None);
        assert_eq!(decode_bitmap("521", 6, &order), None);
    }

    #[test]
    fn parses_a_material_line() {
        let order = [1, 2, 3, 4];
        let slot = parse_material("dfm 77 1 2 xyz_on_a_monster 1 1 0 3 4 9", &order).unwrap().unwrap();
        assert_eq!(slot.kind, "xyz on a monster");
        assert_eq!((slot.procedure, slot.slot, slot.min, slot.max, slot.checked, slot.errors), (1, 2, 1, 1, false, 3));
        assert_eq!(slot.cards, vec![1, 4]);
        assert!(parse_material("dfm 77 none", &order).is_none());
        assert!(parse_material("dfm 77 1 1 link 2 2 0 0 5 00", &order).unwrap().is_err());
    }
}
