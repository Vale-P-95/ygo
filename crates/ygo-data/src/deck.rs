//! Decks: model, legality check per format, `.ydk` and `ydke://` codecs.

use std::collections::{BTreeMap, HashMap};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::model::{Card, CardKind};
use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// Master Duel.
    #[default]
    Md,
    Tcg,
    Ocg,
}

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Md => "md",
            Format::Tcg => "tcg",
            Format::Ocg => "ocg",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "md" => Format::Md,
            "tcg" => Format::Tcg,
            "ocg" => Format::Ocg,
            _ => return None,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Format::Md => "Master Duel",
            Format::Tcg => "TCG",
            Format::Ocg => "OCG",
        }
    }

    /// Name of the format in the card data (`formats` list).
    fn data_name(self) -> &'static str {
        self.label()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    pub id: Option<i64>,
    pub name: String,
    pub format: Format,
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
    pub side: Vec<u32>,
    /// Cards being considered ("maybe board"): not part of the deck, never validated or exported.
    #[serde(default)]
    pub maybe: Vec<u32>,
    /// Cards grouped by role ("starter", "handtrap", ...), for the test hand and the statistics.
    #[serde(default)]
    pub roles: BTreeMap<String, Vec<u32>>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    /// Cards the issue is about (highlighted in the deck).
    pub cards: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Legality {
    pub legal: bool,
    pub issues: Vec<Issue>,
    /// Effective date of the Forbidden & Limited list used, when known.
    #[serde(default)]
    pub banlist: Option<String>,
}

fn is_extra_deck_card(card: &Card) -> bool {
    card.type_line.iter().any(|t| matches!(t.as_str(), "Fusion" | "Synchro" | "Xyz" | "Link"))
}

/// Copies allowed by the format's Forbidden & Limited list.
fn limit(card: &Card, format: Format) -> u8 {
    let status = match format {
        Format::Tcg => card.ban_tcg.as_deref(),
        Format::Ocg => card.ban_ocg.as_deref(),
        Format::Md => card.ban_md.as_deref(),
    };
    match status {
        Some("Banned" | "Forbidden") => 0,
        Some("Limited") => 1,
        Some("Semi-Limited") => 2,
        _ => 3,
    }
}

fn error(message: String, cards: Vec<u32>) -> Issue {
    Issue { severity: Severity::Error, message, cards }
}

/// Checks a deck against the construction rules of its format.
pub fn validate(deck: &Deck, cards: &HashMap<u32, Card>) -> Legality {
    let mut issues = Vec::new();
    let format = deck.format;
    let (main, extra, side) = (deck.main.len(), deck.extra.len(), deck.side.len());
    if main < 40 {
        issues.push(error(format!("Il Main Deck ha {main} carte: ne servono almeno 40."), vec![]));
    }
    if main > 60 {
        issues.push(error(format!("Il Main Deck ha {main} carte: al massimo 60."), vec![]));
    }
    if extra > 15 {
        issues.push(error(format!("L'Extra Deck ha {extra} carte: al massimo 15."), vec![]));
    }
    if side > 15 {
        issues.push(error(format!("Il Side Deck ha {side} carte: al massimo 15."), vec![]));
    }

    let unknown: Vec<u32> = deck.main.iter().chain(&deck.extra).chain(&deck.side).copied().filter(|id| !cards.contains_key(id)).collect();
    if !unknown.is_empty() {
        issues.push(error(format!("{} carte non riconosciute (passcode sconosciuti).", unknown.len()), unknown));
    }

    let mut wrong_place = Vec::new();
    for id in &deck.main {
        if cards.get(id).is_some_and(is_extra_deck_card) {
            wrong_place.push((*id, "va nell'Extra Deck, non nel Main"));
        }
    }
    for id in &deck.extra {
        if cards.get(id).is_some_and(|c| !is_extra_deck_card(c)) {
            wrong_place.push((*id, "non è una carta dell'Extra Deck"));
        }
    }
    for (id, why) in wrong_place {
        issues.push(error(format!("{}: {why}.", cards[&id].name), vec![id]));
    }

    // Each distinct card once, in deck order.
    let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut order = Vec::new();
    for id in deck.main.iter().chain(&deck.extra).chain(&deck.side) {
        let n = counts.entry(*id).or_default();
        if *n == 0 {
            order.push(*id);
        }
        *n += 1;
    }
    for id in order {
        let Some(card) = cards.get(&id) else { continue };
        let copies = counts[&id];
        if matches!(card.kind, CardKind::Skill | CardKind::Token) {
            issues.push(error(format!("{}: non si può mettere in un deck.", card.name), vec![id]));
            continue;
        }
        if !card.formats.iter().any(|f| f == format.data_name()) {
            issues.push(error(format!("{}: non disponibile in {}.", card.name, format.label()), vec![id]));
        }
        let allowed = limit(card, format);
        if allowed == 0 && copies > 0 {
            issues.push(error(format!("{}: proibita in {}.", card.name, format.label()), vec![id]));
        } else if copies > allowed as usize {
            let why = if allowed < 3 { format!(" (limitata a {allowed} in {})", format.label()) } else { String::new() };
            issues.push(error(format!("{}: {copies} copie, al massimo {allowed}{why}.", card.name), vec![id]));
        }
    }

    let legal = issues.iter().all(|i| i.severity != Severity::Error);
    Legality { legal, issues, banlist: None }
}

/// Main, Extra and Side Deck passcodes.
pub type Sections = (Vec<u32>, Vec<u32>, Vec<u32>);

pub fn to_ydk(deck: &Deck) -> String {
    let mut out = String::from("#created by DuelForge\n#main\n");
    for (header, cards) in [("", &deck.main), ("#extra\n", &deck.extra), ("!side\n", &deck.side)] {
        out.push_str(header);
        for id in cards.iter() {
            out.push_str(&id.to_string());
            out.push('\n');
        }
    }
    out
}

pub fn parse_ydk(text: &str) -> Result<Sections, Error> {
    let (mut main, mut extra, mut side) = (Vec::new(), Vec::new(), Vec::new());
    let mut section = 0;
    for line in text.lines().map(str::trim) {
        match line {
            "" => {}
            "#main" => section = 1,
            "#extra" => section = 2,
            "!side" => section = 3,
            l if l.starts_with('#') || l.starts_with('!') => {}
            l => {
                let id: u32 = l.parse().map_err(|_| Error::Data(format!("riga non valida nel file .ydk: {l:?}")))?;
                match section {
                    1 => main.push(id),
                    2 => extra.push(id),
                    3 => side.push(id),
                    _ => return Err(Error::Data("passcode fuori dalle sezioni #main/#extra/!side".into())),
                }
            }
        }
    }
    Ok((main, extra, side))
}

fn encode_ids(ids: &[u32]) -> String {
    let bytes: Vec<u8> = ids.iter().flat_map(|id| id.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn decode_ids(part: &str) -> Result<Vec<u32>, Error> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(part.trim()).map_err(|e| Error::Data(format!("ydke non valido: {e}")))?;
    if bytes.len() % 4 != 0 {
        return Err(Error::Data("ydke non valido: lunghezza errata".into()));
    }
    Ok(bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
}

/// `ydke://<main>!<extra>!<side>!`, each part base64 of little-endian u32s.
pub fn to_ydke(deck: &Deck) -> String {
    format!("ydke://{}!{}!{}!", encode_ids(&deck.main), encode_ids(&deck.extra), encode_ids(&deck.side))
}

pub fn parse_ydke(url: &str) -> Result<Sections, Error> {
    let body = url.trim().strip_prefix("ydke://").ok_or_else(|| Error::Data("un link ydke inizia con ydke://".into()))?;
    let parts: Vec<&str> = body.split('!').collect();
    if parts.len() < 3 {
        return Err(Error::Data("ydke non valido: servono tre sezioni".into()));
    }
    Ok((decode_ids(parts[0])?, decode_ids(parts[1])?, decode_ids(parts[2])?))
}

/// Accepts either format.
pub fn parse_any(text: &str) -> Result<Sections, Error> {
    if text.trim_start().starts_with("ydke://") {
        parse_ydke(text)
    } else {
        parse_ydk(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: u32, name: &str, kind: CardKind, type_line: &[&str], formats: &[&str]) -> Card {
        Card {
            id,
            name: name.into(),
            kind,
            frame: String::new(),
            human_type: String::new(),
            type_line: type_line.iter().map(|s| s.to_string()).collect(),
            desc: String::new(),
            pend_desc: None,
            race: String::new(),
            attribute: None,
            archetype: None,
            level: None,
            atk: None,
            def: None,
            scale: None,
            link_rating: None,
            link_markers: vec![],
            md_rarity: None,
            formats: formats.iter().map(|s| s.to_string()).collect(),
            tcg_date: None,
            ocg_date: None,
            ban_tcg: None,
            ban_ocg: None,
            ban_md: None,
            image_ids: vec![id],
            views: 0,
        }
    }

    fn pool() -> HashMap<u32, Card> {
        let all = ["TCG", "OCG", "Master Duel"];
        let mut cards = vec![
            card(1, "Monster A", CardKind::Monster, &["Spellcaster", "Effect"], &all),
            card(2, "Spell B", CardKind::Spell, &[], &all),
            card(3, "Fusion C", CardKind::Monster, &["Machine", "Fusion", "Effect"], &all),
            card(4, "OCG Only D", CardKind::Monster, &["Fiend", "Effect"], &["OCG"]),
        ];
        cards[1].ban_tcg = Some("Limited".into());
        cards.into_iter().map(|c| (c.id, c)).collect()
    }

    fn deck(format: Format, main: Vec<u32>, extra: Vec<u32>) -> Deck {
        Deck { format, main, extra, ..Default::default() }
    }

    #[test]
    fn legal_forty_card_deck() {
        let main: Vec<u32> = std::iter::repeat_n(1, 3).chain(std::iter::repeat_n(2, 1)).collect();
        let mut cards = pool();
        // 40 distinct-enough cards: add filler monsters 100..136.
        let mut main = main;
        for id in 100..136 {
            cards.insert(id, card(id, &format!("Filler {id}"), CardKind::Monster, &["Warrior", "Effect"], &["TCG", "OCG", "Master Duel"]));
            main.push(id);
        }
        let result = validate(&deck(Format::Tcg, main, vec![3]), &cards);
        assert!(result.legal, "{:?}", result.issues);
    }

    #[test]
    fn reports_size_section_copies_and_availability() {
        let cards = pool();
        let result = validate(&deck(Format::Tcg, vec![1, 1, 1, 1, 2, 2, 3, 4], vec![1]), &cards);
        assert!(!result.legal);
        let messages: Vec<&str> = result.issues.iter().map(|i| i.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("almeno 40")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("Fusion C: va nell'Extra Deck")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("Monster A: non è una carta dell'Extra Deck")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("Monster A: 5 copie, al massimo 3")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("Spell B: 2 copie, al massimo 1 (limitata a 1 in TCG)")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("OCG Only D: non disponibile in TCG")), "{messages:?}");
    }

    #[test]
    fn master_duel_follows_its_own_list() {
        let mut cards = pool();
        cards.get_mut(&1).unwrap().ban_md = Some("Forbidden".into());
        let result = validate(&deck(Format::Md, vec![1, 2, 2], vec![]), &cards);
        let messages: Vec<&str> = result.issues.iter().map(|i| i.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("Monster A: proibita in Master Duel")), "{messages:?}");
        assert!(!messages.iter().any(|m| m.contains("Spell B")), "TCG limits must not apply to Master Duel: {messages:?}");
    }

    #[test]
    fn ydk_and_ydke_round_trip() {
        let d = Deck { main: vec![86120751, 14558128, 14558128], extra: vec![75286621], side: vec![25311006], ..Default::default() };
        assert_eq!(parse_any(&to_ydk(&d)).unwrap(), (d.main.clone(), d.extra.clone(), d.side.clone()));
        let url = to_ydke(&d);
        assert!(url.starts_with("ydke://"));
        assert_eq!(parse_any(&url).unwrap(), (d.main, d.extra, d.side));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_ydk("#main\nabc\n").is_err());
        assert!(parse_ydke("ydke://!!").map(|s| s.0.is_empty()).unwrap_or(false));
        assert!(parse_ydke("ydke://AAA!").is_err());
    }
}
