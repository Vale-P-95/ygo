//! The catalog's filters by what a card's effects do (`eff:`), read from
//! the card's script when the engine has it and from its text
//! (`docs/plans/anatomia-effetti.md` §3). Each card gets a mask of
//! [`EffectFilter`] bits; the caller that has the engine builds it.

use serde::{Deserialize, Serialize};

use crate::psct::{Part, Reading};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectFilter {
    /// Negates an activation, an effect or a Summon.
    Negate,
    Destroy,
    Banish,
    /// Adds a card from the Deck to the hand.
    Search,
    Draw,
    /// Special Summons from the Deck.
    DeckSummon,
    /// Does something to the opponent's Extra Deck.
    OpponentExtra,
    /// Has a Quick Effect.
    Quick,
    /// Has an effect used from the hand (not the activation of a Spell or Trap).
    Hand,
    /// Has an effect used from the GY.
    Graveyard,
}

impl EffectFilter {
    pub const ALL: [EffectFilter; 10] = [
        EffectFilter::Negate,
        EffectFilter::Destroy,
        EffectFilter::Banish,
        EffectFilter::Search,
        EffectFilter::Draw,
        EffectFilter::DeckSummon,
        EffectFilter::OpponentExtra,
        EffectFilter::Quick,
        EffectFilter::Hand,
        EffectFilter::Graveyard,
    ];

    pub fn bit(self) -> u16 {
        1 << self as u16
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "negate" | "nega" | "negazione" => EffectFilter::Negate,
            "destroy" | "distrugge" | "distruggi" => EffectFilter::Destroy,
            "banish" | "bandisce" | "bandisci" => EffectFilter::Banish,
            "search" | "cerca" => EffectFilter::Search,
            "draw" | "pesca" => EffectFilter::Draw,
            "deck-summon" | "evoca-dal-deck" | "evoca-deck" => EffectFilter::DeckSummon,
            "opp-extra" | "extra-avversario" => EffectFilter::OpponentExtra,
            "quick" | "rapido" => EffectFilter::Quick,
            "hand" | "mano" => EffectFilter::Hand,
            "gy" | "graveyard" | "cimitero" => EffectFilter::Graveyard,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            EffectFilter::Negate => "Effetto: nega",
            EffectFilter::Destroy => "Effetto: distrugge",
            EffectFilter::Banish => "Effetto: bandisce",
            EffectFilter::Search => "Effetto: cerca nel Deck",
            EffectFilter::Draw => "Effetto: pesca",
            EffectFilter::DeckSummon => "Effetto: Evoca dal Deck",
            EffectFilter::OpponentExtra => "Effetto: Extra Deck avversario",
            EffectFilter::Quick => "Effetto rapido",
            EffectFilter::Hand => "Effetto dalla mano",
            EffectFilter::Graveyard => "Effetto dal Cimitero",
        }
    }
}

/// An effect as the card's script registers it, in the terms the filters need.
#[derive(Debug, Clone, Copy)]
pub struct ScriptEffect<'a> {
    /// Activated (ignition, trigger, Quick, Flip, or a Spell or Trap activation).
    pub activated: bool,
    /// The activation of a Spell or Trap Card itself.
    pub card_activation: bool,
    pub quick: bool,
    /// Where it is used from (the engine's location bits).
    pub range: u32,
    /// The engine's category names (`ygo_engine::consts::category::NAMES`).
    pub categories: &'a [&'static str],
}

const HAND: u32 = 0x2;
const GY: u32 = 0x10;

/// The filters a card matches: what its script declares, plus what its text
/// says that the script leaves inside an operation (the banish of the
/// opponent's Extra Deck, a Summon from the Deck).
pub fn effect_mask(text: &str, reading: &Reading, script: Option<&[ScriptEffect]>) -> u16 {
    let lower = text.to_lowercase();
    let mut mask = 0;
    let mut add = |on: bool, f: EffectFilter| {
        if on {
            mask |= f.bit();
        }
    };
    let mut categories: Vec<&str> = reading.effects.iter().flat_map(|e| e.categories.iter().copied()).collect();
    if let Some(script) = script {
        categories.extend(script.iter().filter(|e| e.activated).flat_map(|e| e.categories.iter().copied()));
        let used_from = |zone: u32| script.iter().any(|e| e.activated && !e.card_activation && e.range & zone != 0);
        add(script.iter().any(|e| e.quick), EffectFilter::Quick);
        add(used_from(HAND), EffectFilter::Hand);
        add(used_from(GY), EffectFilter::Graveyard);
    }
    let has = |name: &str| categories.contains(&name);
    add(has("negate effect") || has("negate activation") || has("negate summon"), EffectFilter::Negate);
    add(has("destroy"), EffectFilter::Destroy);
    add(has("banish"), EffectFilter::Banish);
    add(has("search"), EffectFilter::Search);
    add(has("draw"), EffectFilter::Draw);
    add(lower.contains("(quick effect)"), EffectFilter::Quick);
    add(["this card in your hand", "this card from your hand", "discard this card"].iter().any(|p| lower.contains(p)), EffectFilter::Hand);
    add(
        ["this card in your gy", "this card is in your gy", "this card from your gy"].iter().any(|p| lower.contains(p)),
        EffectFilter::Graveyard,
    );
    add(lower.contains("opponent's extra deck"), EffectFilter::OpponentExtra);
    let from_deck = ["from your deck", "from your hand or deck", "from your hand, deck", "from your hand and/or deck"];
    add(
        reading.pieces.iter().filter(|p| p.part == Some(Part::Action)).any(|p| {
            let action = p.text.to_lowercase();
            action.contains("special summon") && from_deck.iter().any(|d| action.contains(d))
        }),
        EffectFilter::DeckSummon,
    );
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psct::{read, Frame};

    // Texts written for the tests in the style of the cards.

    fn mask(text: &str, frame: Frame, script: Option<&[ScriptEffect]>) -> Vec<EffectFilter> {
        let m = effect_mask(text, &read(text, frame), script);
        EffectFilter::ALL.into_iter().filter(|f| m & f.bit() != 0).collect()
    }

    #[test]
    fn from_the_text_alone() {
        let text = "If this card is Special Summoned: You can banish 3 cards from your opponent's Extra Deck, face-down.\nYou can discard this card; Special Summon 1 \"Test\" monster from your Deck.";
        assert_eq!(
            mask(text, Frame::Monster, None),
            [EffectFilter::Banish, EffectFilter::DeckSummon, EffectFilter::OpponentExtra, EffectFilter::Hand]
        );
    }

    #[test]
    fn the_script_adds_what_the_text_does_not_say() {
        let text = "When your opponent activates a card or effect (Quick Effect): You can send this card to the GY; negate that effect.";
        let negate: &[&'static str] = &["negate effect"];
        let script = [ScriptEffect { activated: true, card_activation: false, quick: true, range: HAND, categories: negate }];
        assert_eq!(mask(text, Frame::Monster, Some(&script)), [EffectFilter::Negate, EffectFilter::Quick, EffectFilter::Hand]);
    }

    #[test]
    fn a_spell_activation_is_not_from_the_hand() {
        let text = "Add 1 \"Test\" card from your Deck to your hand.";
        let search: &[&'static str] = &["search", "to hand"];
        let script = [ScriptEffect { activated: true, card_activation: true, quick: false, range: HAND, categories: search }];
        assert_eq!(mask(text, Frame::SpellTrap, Some(&script)), [EffectFilter::Search]);
    }

    #[test]
    fn names_and_labels() {
        for f in EffectFilter::ALL {
            assert!(!f.label().is_empty());
        }
        assert_eq!(EffectFilter::parse("cimitero"), Some(EffectFilter::Graveyard));
        assert_eq!(EffectFilter::parse("other"), None);
    }
}
