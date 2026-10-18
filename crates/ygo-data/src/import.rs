//! Conversion of the YGOPRODeck v7 `cardinfo.php?misc=yes` response.

use serde::Deserialize;

use crate::model::{Card, CardKind};
use crate::Error;

#[derive(Deserialize)]
struct Response {
    data: Vec<ApiCard>,
}

#[derive(Deserialize)]
struct ApiCard {
    id: u32,
    name: String,
    #[serde(rename = "type")]
    type_: String,
    #[serde(rename = "humanReadableCardType", default)]
    human_type: String,
    #[serde(rename = "frameType", default)]
    frame: String,
    #[serde(default)]
    desc: String,
    pend_desc: Option<String>,
    #[serde(default)]
    race: String,
    attribute: Option<String>,
    archetype: Option<String>,
    #[serde(default)]
    typeline: Vec<String>,
    level: Option<u8>,
    atk: Option<i32>,
    def: Option<i32>,
    scale: Option<u8>,
    linkval: Option<u8>,
    #[serde(default)]
    linkmarkers: Vec<String>,
    #[serde(default)]
    card_images: Vec<ApiImage>,
    banlist_info: Option<ApiBanlist>,
    #[serde(default)]
    misc_info: Vec<ApiMisc>,
}

#[derive(Deserialize)]
struct ApiImage {
    id: u32,
}

#[derive(Deserialize)]
struct ApiBanlist {
    ban_tcg: Option<String>,
    ban_ocg: Option<String>,
}

#[derive(Deserialize, Default)]
struct ApiMisc {
    #[serde(default)]
    formats: Vec<String>,
    tcg_date: Option<String>,
    ocg_date: Option<String>,
    md_rarity: Option<String>,
    #[serde(default)]
    views: u64,
    #[serde(default)]
    question_atk: u8,
    #[serde(default)]
    question_def: u8,
}

fn kind_of(type_: &str, frame: &str) -> CardKind {
    match () {
        _ if frame == "spell" || type_.contains("Spell") => CardKind::Spell,
        _ if frame == "trap" || type_.contains("Trap") => CardKind::Trap,
        _ if frame == "skill" || type_.contains("Skill") => CardKind::Skill,
        _ if frame == "token" || type_.contains("Token") => CardKind::Token,
        _ => CardKind::Monster,
    }
}

impl From<ApiCard> for Card {
    fn from(c: ApiCard) -> Self {
        let misc = c.misc_info.into_iter().next().unwrap_or_default();
        let kind = kind_of(&c.type_, &c.frame);
        let monster = kind == CardKind::Monster;
        Card {
            id: c.id,
            name: c.name,
            kind,
            frame: c.frame,
            human_type: if c.human_type.is_empty() { c.type_ } else { c.human_type },
            type_line: c.typeline,
            desc: c.desc.replace("\r\n", "\n"),
            pend_desc: c.pend_desc.map(|s| s.replace("\r\n", "\n")),
            race: c.race,
            attribute: c.attribute,
            archetype: c.archetype,
            level: c.level.filter(|_| monster),
            atk: c.atk.filter(|_| misc.question_atk == 0),
            def: c.def.filter(|_| misc.question_def == 0 && c.linkval.is_none()),
            scale: c.scale,
            link_rating: c.linkval,
            link_markers: c.linkmarkers,
            md_rarity: misc.md_rarity,
            formats: misc.formats,
            tcg_date: misc.tcg_date,
            ocg_date: misc.ocg_date,
            ban_tcg: c.banlist_info.as_ref().and_then(|b| b.ban_tcg.clone()),
            ban_ocg: c.banlist_info.and_then(|b| b.ban_ocg),
            ban_md: None,
            image_ids: c.card_images.into_iter().map(|i| i.id).collect(),
            views: misc.views,
        }
    }
}

/// Parses a full `cardinfo.php` response.
pub fn parse_cardinfo(json: &[u8]) -> Result<Vec<Card>, Error> {
    let response: Response = serde_json::from_slice(json)?;
    Ok(response.data.into_iter().map(Card::from).collect())
}

/// Parses the `checkDBVer.php` response: `[{"database_version": "147.37", ...}]`.
pub fn parse_db_version(json: &[u8]) -> Result<String, Error> {
    #[derive(Deserialize)]
    struct Version {
        database_version: String,
    }
    let versions: Vec<Version> = serde_json::from_slice(json)?;
    versions.into_iter().next().map(|v| v.database_version).ok_or_else(|| Error::Data("empty database version".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_db_version() {
        assert_eq!(parse_db_version(br#"[{"database_version":"147.37","last_update":"2026-10-16"}]"#).unwrap(), "147.37");
    }

    #[test]
    fn kinds_follow_frame_and_type() {
        assert_eq!(kind_of("Spell Card", "spell"), CardKind::Spell);
        assert_eq!(kind_of("Trap Card", "trap"), CardKind::Trap);
        assert_eq!(kind_of("Fusion Monster", "fusion"), CardKind::Monster);
        assert_eq!(kind_of("Skill Card", "skill"), CardKind::Skill);
    }
}
