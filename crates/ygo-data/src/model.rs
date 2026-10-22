//! Card data as shown by the app (deck builder, viewer, search results).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardKind {
    Monster,
    Spell,
    Trap,
    Skill,
    Token,
}

impl CardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CardKind::Monster => "monster",
            CardKind::Spell => "spell",
            CardKind::Trap => "trap",
            CardKind::Skill => "skill",
            CardKind::Token => "token",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "monster" => CardKind::Monster,
            "spell" => CardKind::Spell,
            "trap" => CardKind::Trap,
            "skill" => CardKind::Skill,
            "token" => CardKind::Token,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// Passcode: the key shared by every data source.
    pub id: u32,
    pub name: String,
    pub kind: CardKind,
    /// Card frame: `effect`, `fusion`, `spell`, `effect_pendulum`, ...
    pub frame: String,
    /// e.g. "Fusion Effect Monster", "Quick-Play Spell Card".
    pub human_type: String,
    /// Monster type line, e.g. ["Machine", "Fusion", "Effect"].
    pub type_line: Vec<String>,
    pub desc: String,
    pub pend_desc: Option<String>,
    /// Monster type for monsters, spell/trap subtype otherwise.
    pub race: String,
    pub attribute: Option<String>,
    pub archetype: Option<String>,
    /// Level or Rank.
    pub level: Option<u8>,
    /// `None` when unknown (`?`) or not a monster.
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub scale: Option<u8>,
    pub link_rating: Option<u8>,
    pub link_markers: Vec<String>,
    /// Master Duel rarity: Common, Rare, Super Rare, Ultra Rare.
    pub md_rarity: Option<String>,
    pub formats: Vec<String>,
    pub tcg_date: Option<String>,
    pub ocg_date: Option<String>,
    pub ban_tcg: Option<String>,
    pub ban_ocg: Option<String>,
    /// Master Duel status (Forbidden, Limited, Semi-Limited), from our own list.
    #[serde(default)]
    pub ban_md: Option<String>,
    /// Ids of the available artworks (the first is the main one).
    pub image_ids: Vec<u32>,
    pub views: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Facets {
    pub attributes: Vec<String>,
    pub races: Vec<String>,
    pub archetypes: Vec<String>,
    pub formats: Vec<String>,
}

/// A card whose status changed with a list update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanChange {
    /// `tcg`, `ocg` or `md`.
    pub format: String,
    pub id: u32,
    /// `YYYY-MM-DD`: when the list took effect (Master Duel) or when the card
    /// data showed the change (TCG, OCG).
    pub date: String,
    /// Forbidden, Limited or Semi-Limited; `None` is unlimited.
    pub before: Option<String>,
    pub after: Option<String>,
}
