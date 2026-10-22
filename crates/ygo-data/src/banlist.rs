//! Forbidden & Limited lists missing from the YGOPRODeck card data, that is
//! Master Duel's. The list ships inside the app (`banlists/md.json`); when
//! the app updates its data it also looks for a newer copy in the repository,
//! so a new list does not need a new release.

use std::path::Path;

use serde::Deserialize;

use crate::Error;

pub const MD_EMBEDDED: &str = include_str!("../banlists/md.json");
pub const MD_URL: &str = "https://raw.githubusercontent.com/Vale-P-95/ygo/HEAD/crates/ygo-data/banlists/md.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Banlist {
    /// Only `md` for now.
    pub format: String,
    /// Date the list took effect, `YYYY-MM-DD` (so dates compare as text).
    pub effective: String,
    pub cards: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Entry {
    /// YGOPRODeck id of the card.
    pub id: u32,
    /// For people editing the file; the id is what counts.
    pub name: String,
    /// Copies allowed: 0 Forbidden, 1 Limited, 2 Semi-Limited.
    pub limit: u8,
}

/// Status name stored with the cards, as YGOPRODeck spells it for TCG/OCG.
pub fn status_name(limit: u8) -> &'static str {
    match limit {
        0 => "Forbidden",
        1 => "Limited",
        _ => "Semi-Limited",
    }
}

/// Whether `s` is shaped like `YYYY-MM-DD`.
pub(crate) fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
}

impl Banlist {
    pub fn parse(json: &str) -> Result<Self, Error> {
        let list: Banlist = serde_json::from_str(json)?;
        if list.format != "md" {
            return Err(Error::Data(format!("banlist di un formato sconosciuto: {}", list.format)));
        }
        if !is_date(&list.effective) {
            return Err(Error::Data(format!("data della banlist non valida: {}", list.effective)));
        }
        if let Some(e) = list.cards.iter().find(|e| e.limit > 2) {
            return Err(Error::Data(format!("{}: limite {} non valido", e.name, e.limit)));
        }
        Ok(list)
    }

    /// The list built into the app.
    pub fn embedded_md() -> Self {
        Self::parse(MD_EMBEDDED).expect("the embedded Master Duel list is valid")
    }

    /// The newer of the built-in list and a downloaded copy at `path`.
    pub fn newest_md(path: &Path) -> Self {
        let embedded = Self::embedded_md();
        match std::fs::read_to_string(path).ok().and_then(|s| Self::parse(&s).ok()) {
            Some(downloaded) if downloaded.effective > embedded.effective => downloaded,
            _ => embedded,
        }
    }
}

/// Downloads the latest Master Duel list to `path` if it is newer than
/// `current` (an effective date). Returns whether a newer list arrived.
pub async fn update_md(client: &reqwest::Client, path: &Path, current: &str) -> Result<bool, Error> {
    let text = client.get(MD_URL).send().await?.error_for_status()?.text().await?;
    let list = Banlist::parse(&text)?;
    if list.effective.as_str() <= current {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_list_is_valid() {
        let list = Banlist::embedded_md();
        assert!(list.cards.len() > 100);
        let mut ids: Vec<u32> = list.cards.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), list.cards.len(), "each card once");
    }

    #[test]
    fn rejects_bad_lists() {
        assert!(Banlist::parse(r#"{"format":"tcg","effective":"2026-10-06","cards":[]}"#).is_err());
        assert!(Banlist::parse(r#"{"format":"md","effective":"6 Oct","cards":[]}"#).is_err());
        assert!(Banlist::parse(r#"{"format":"md","effective":"2026-10-06","cards":[{"id":1,"name":"X","limit":3}]}"#).is_err());
    }

    #[test]
    fn prefers_a_newer_download() {
        let dir = std::env::temp_dir().join(format!("duelforge-banlist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("md.json");
        std::fs::write(&path, r#"{"format":"md","effective":"2000-01-01","cards":[]}"#).unwrap();
        assert_eq!(Banlist::newest_md(&path), Banlist::embedded_md(), "an older download is ignored");
        std::fs::write(&path, r#"{"format":"md","effective":"2999-01-01","cards":[{"id":1,"name":"X","limit":0}]}"#).unwrap();
        assert_eq!(Banlist::newest_md(&path).effective, "2999-01-01");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
