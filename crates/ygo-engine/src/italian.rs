//! What happens in a duel, told in Italian: the explanation of a combo is
//! built from these lines. Card names and effect texts stay in English, as
//! in the card data; the game terms are the Italian ones of the TCG.

use crate::carddb::CardDb;
use crate::consts::{card_type, location, phase};
use crate::message::{CardRef, Event};

mod reason {
    pub const DESTROY: u32 = 0x1;
    pub const RELEASE: u32 = 0x2;
    pub const MATERIAL: u32 = 0x8;
    pub const BATTLE: u32 = 0x20;
    pub const COST: u32 = 0x80;
    pub const DISCARD: u32 = 0x4000;
    pub const FUSION: u32 = 0x4_0000;
    pub const SYNCHRO: u32 = 0x8_0000;
    pub const RITUAL: u32 = 0x10_0000;
    pub const XYZ: u32 = 0x20_0000;
    pub const LINK: u32 = 0x1000_0000;
}

fn who(player: u8) -> &'static str {
    if player == 0 {
        "Tu"
    } else {
        "L'avversario"
    }
}

fn name(db: &CardDb, code: u32) -> String {
    if code == 0 {
        "una carta coperta".to_owned()
    } else {
        db.name(code)
    }
}

fn from_place(loc: u8) -> &'static str {
    match loc & !location::OVERLAY {
        location::DECK => " dal Deck",
        location::HAND => " dalla mano",
        location::MZONE | location::SZONE => " dal Terreno",
        location::GRAVE => " dal Cimitero",
        location::REMOVED => " dalle carte bandite",
        location::EXTRA => " dall'Extra Deck",
        _ => "",
    }
}

fn material_for(reason_bits: u32) -> Option<&'static str> {
    [(reason::FUSION, "Fusion"), (reason::SYNCHRO, "Synchro"), (reason::XYZ, "Xyz"), (reason::LINK, "Link"), (reason::RITUAL, "Rituale")]
        .into_iter()
        .find(|(bit, _)| reason_bits & bit != 0)
        .map(|(_, kind)| kind)
}

fn moved(db: &CardDb, code: u32, from: &CardRef, to: &CardRef, why: u32) -> Option<String> {
    let card = name(db, code);
    let src = from_place(from.location);
    if to.location & location::OVERLAY != 0 {
        return Some(format!("{card} diventa materiale Xyz"));
    }
    let line = match to.location {
        location::HAND if from.location == location::DECK => format!("{card} viene aggiunta alla mano dal Deck"),
        location::HAND => format!("{card} torna in mano{src}"),
        location::GRAVE => {
            if why & reason::MATERIAL != 0 {
                match material_for(why) {
                    Some(kind) => format!("{card} va al Cimitero come materiale per un'Evocazione {kind}"),
                    None => format!("{card} va al Cimitero come materiale"),
                }
            } else if why & reason::DISCARD != 0 {
                format!("{card} viene scartata")
            } else if why & reason::RELEASE != 0 {
                format!("{card} viene offerta come tributo")
            } else if why & reason::DESTROY != 0 && why & reason::BATTLE != 0 {
                format!("{card} viene distrutta in battaglia")
            } else if why & reason::DESTROY != 0 {
                format!("{card} viene distrutta")
            } else if why & reason::COST != 0 {
                format!("{card} viene mandata al Cimitero{src} come costo")
            } else if from.location & (location::MZONE | location::SZONE) != 0 {
                format!("{card} va al Cimitero")
            } else {
                format!("{card} viene mandata al Cimitero{src}")
            }
        }
        location::REMOVED => match material_for(why).filter(|_| why & reason::MATERIAL != 0) {
            Some(kind) => format!("{card} viene bandita come materiale per un'Evocazione {kind}"),
            None if why & reason::COST != 0 => format!("{card} viene bandita{src} come costo"),
            None => format!("{card} viene bandita{src}"),
        },
        location::DECK => format!("{card} torna nel Deck{src}"),
        location::EXTRA => format!("{card} torna nell'Extra Deck{src}"),
        // Arriving on the field: summons, sets and activations have their own lines.
        location::SZONE if from.location & (location::DECK | location::GRAVE | location::REMOVED) != 0 => {
            format!("{card} viene posizionata sul Terreno{src}")
        }
        _ => return None,
    };
    Some(line)
}

fn summoned(db: &CardDb, card: &CardRef) -> String {
    let n = name(db, card.code);
    let kind = db.get(card.code).map_or(0, |c| c.type_);
    let how = [(card_type::LINK, "Link"), (card_type::XYZ, "Xyz"), (card_type::SYNCHRO, "Synchro"), (card_type::FUSION, "Fusion")]
        .into_iter()
        .find(|(bit, _)| kind & bit != 0 && card.location == location::MZONE);
    match how {
        Some((_, how)) => format!("{n} viene evocato tramite Evocazione {how}"),
        None => format!("{n} viene evocato specialmente"),
    }
}

/// One line for `event`, or `None` for events that add nothing to read.
pub fn describe(db: &CardDb, event: &Event) -> Option<String> {
    Some(match event {
        Event::NewTurn { player } => {
            if *player == 0 {
                "Inizia il tuo turno".to_owned()
            } else {
                "Inizia il turno dell'avversario".to_owned()
            }
        }
        Event::NewPhase { phase: p } => phase::name(*p).to_owned(),
        Event::Draw { player, codes } => {
            if *player == 0 {
                format!("Peschi {}", codes.iter().map(|&c| name(db, c)).collect::<Vec<_>>().join(", "))
            } else {
                format!("L'avversario pesca {} carte", codes.len())
            }
        }
        Event::Move { code, from, to, reason } => return moved(db, *code, from, to, *reason),
        Event::NormalSummoning { card } => {
            format!("{} evoca normalmente {}", who(card.controller), name(db, card.code)).replacen("Tu evoca", "Evochi", 1)
        }
        Event::SpecialSummoning { card } => summoned(db, card),
        Event::FlipSummoning { card } => format!("{} viene evocato tramite Evocazione di Scoperta", name(db, card.code)),
        Event::Set { card } => {
            if card.controller == 0 {
                format!("Posizioni {}", name(db, card.code))
            } else {
                "L'avversario posiziona una carta".to_owned()
            }
        }
        Event::Chaining { card, description, chain_link } => {
            let text = db.describe(*description);
            let card = name(db, card.code);
            if text.is_empty() {
                format!("Catena {chain_link}: {card}")
            } else {
                format!("Catena {chain_link}: {card}: {text}")
            }
        }
        Event::ChainNegated { chain_link } => format!("L'attivazione della Catena {chain_link} viene annullata"),
        Event::ChainDisabled { chain_link } => format!("L'effetto della Catena {chain_link} viene annullato"),
        Event::PositionChange { card, .. } => format!("{} cambia posizione", name(db, card.code)),
        Event::Damage { player, amount } => {
            if *player == 0 {
                format!("Subisci {amount} danni")
            } else {
                format!("L'avversario subisce {amount} danni")
            }
        }
        Event::Recover { player, amount } => format!("{} guadagna {amount} LP", who(*player)).replacen("Tu guadagna", "Guadagni", 1),
        Event::PayLpCost { player, amount } => format!("{} paga {amount} LP", who(*player)).replacen("Tu paga", "Paghi", 1),
        Event::Attack { attacker, target } => match target {
            Some(t) => {
                format!("{} attacca {}", name(db, attacker.code), if t.code == 0 { "un mostro".to_owned() } else { name(db, t.code) })
            }
            None => format!("{} attacca direttamente", name(db, attacker.code)),
        },
        Event::Win { player, .. } => {
            if *player == 0 {
                "Vinci il duello".to_owned()
            } else {
                "L'avversario vince il duello".to_owned()
            }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(code: u32, controller: u8, location: u8) -> CardRef {
        CardRef { code, controller, location, sequence: 0, position: 0 }
    }

    #[test]
    fn lines_in_italian() {
        let db = CardDb::default();
        let line = |e: Event| describe(&db, &e);
        assert_eq!(line(Event::NewTurn { player: 0 }).unwrap(), "Inizia il tuo turno");
        assert_eq!(line(Event::NormalSummoning { card: card(0, 0, location::MZONE) }).unwrap(), "Evochi normalmente una carta coperta");
        assert_eq!(line(Event::PayLpCost { player: 0, amount: 800 }).unwrap(), "Paghi 800 LP");
        assert_eq!(line(Event::PayLpCost { player: 1, amount: 800 }).unwrap(), "L'avversario paga 800 LP");
        let search = Event::Move { code: 0, from: card(0, 0, location::DECK), to: card(0, 0, location::HAND), reason: 0x40 };
        assert_eq!(line(search).unwrap(), "una carta coperta viene aggiunta alla mano dal Deck");
        let material = Event::Move {
            code: 0,
            from: card(0, 0, location::MZONE),
            to: card(0, 0, location::REMOVED),
            reason: reason::MATERIAL | reason::FUSION,
        };
        assert_eq!(line(material).unwrap(), "una carta coperta viene bandita come materiale per un'Evocazione Fusion");
        // Arriving in a Monster Zone is told by the summon line.
        assert!(line(Event::Move { code: 0, from: card(0, 0, location::HAND), to: card(0, 0, location::MZONE), reason: 0 }).is_none());
        assert_eq!(line(Event::ChainNegated { chain_link: 2 }).unwrap(), "L'attivazione della Catena 2 viene annullata");
    }
}
