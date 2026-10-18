//! The search language of the card browser. Filters chosen in the UI are
//! written into the same text, so what the user sees is what is searched:
//!
//! ```text
//! aleister                      words in name or text (prefix match)
//! "negate the activation"       exact phrase
//! name:invoked                  words in the name only
//! type:fusion  t:spell          card kind or subtype (fusion, quick-play, tuner, ...)
//! attr:dark  race:spellcaster   attribute, monster type
//! arch:invoked                  archetype
//! atk>=2500  def<1000  lv:4-6   numeric ranges (atk, def, level/rank, link, scale)
//! format:md  rarity:ur          Master Duel availability and rarity
//! ban:limited                   TCG Forbidden/Limited status
//! sort:atk                      name, atk, def, level, popular, new
//! ```
//! Italian aliases are accepted (`tipo`, `attributo`, `livello`, `magia`, ...).

use serde::{Deserialize, Serialize};

use crate::model::CardKind;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

impl Range {
    pub fn is_set(&self) -> bool {
        self.min.is_some() || self.max.is_some()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    /// Best text match first, or by name without text.
    #[default]
    Relevance,
    Name,
    Atk,
    Def,
    Level,
    Popular,
    New,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub words: Vec<String>,
    pub phrases: Vec<String>,
    pub name_words: Vec<String>,
    pub kinds: Vec<CardKind>,
    /// Monster type-line entries (Fusion, Tuner, ...) or Spell/Trap
    /// subtypes (Quick-Play, Field, ...).
    pub subtypes: Vec<String>,
    pub attributes: Vec<String>,
    pub races: Vec<String>,
    pub archetypes: Vec<String>,
    pub level: Range,
    pub atk: Range,
    pub def: Range,
    pub link: Range,
    pub scale: Range,
    pub formats: Vec<String>,
    pub rarities: Vec<String>,
    pub bans: Vec<String>,
    pub sort: Sort,
}

/// A recognised filter, for display as a removable chip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chip {
    /// The exact token in the query text (removing it removes the filter).
    pub token: String,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parsed {
    pub query: SearchQuery,
    pub chips: Vec<Chip>,
    /// Tokens that could not be understood.
    pub errors: Vec<String>,
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in input.chars() {
        match ch {
            '"' => {
                current.push(ch);
                quoted = !quoted;
            }
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').trim().to_owned()
}

const SUBTYPES: &[(&str, &str)] = &[
    ("fusion", "Fusion"),
    ("synchro", "Synchro"),
    ("xyz", "Xyz"),
    ("link", "Link"),
    ("ritual", "Ritual"),
    ("rituale", "Ritual"),
    ("pendulum", "Pendulum"),
    ("tuner", "Tuner"),
    ("effect", "Effect"),
    ("effetto", "Effect"),
    ("normal", "Normal"),
    ("normale", "Normal"),
    ("flip", "Flip"),
    ("gemini", "Gemini"),
    ("spirit", "Spirit"),
    ("union", "Union"),
    ("toon", "Toon"),
    ("quick-play", "Quick-Play"),
    ("quickplay", "Quick-Play"),
    ("rapida", "Quick-Play"),
    ("continuous", "Continuous"),
    ("continua", "Continuous"),
    ("field", "Field"),
    ("terreno", "Field"),
    ("equip", "Equip"),
    ("equipaggiamento", "Equip"),
    ("counter", "Counter"),
    ("contro", "Counter"),
];

fn kind_alias(v: &str) -> Option<CardKind> {
    Some(match v {
        "monster" | "mostro" | "mostri" => CardKind::Monster,
        "spell" | "magia" | "magie" => CardKind::Spell,
        "trap" | "trappola" | "trappole" => CardKind::Trap,
        "skill" | "abilità" => CardKind::Skill,
        "token" => CardKind::Token,
        _ => return None,
    })
}

fn attribute_alias(v: &str) -> Option<&'static str> {
    Some(match v {
        "dark" | "oscurità" | "oscurita" => "DARK",
        "light" | "luce" => "LIGHT",
        "earth" | "terra" => "EARTH",
        "water" | "acqua" => "WATER",
        "fire" | "fuoco" => "FIRE",
        "wind" | "vento" => "WIND",
        "divine" | "divino" => "DIVINE",
        _ => return None,
    })
}

fn format_alias(v: &str) -> Option<&'static str> {
    Some(match v {
        "md" | "masterduel" | "master duel" => "Master Duel",
        "tcg" => "TCG",
        "ocg" => "OCG",
        "goat" => "GOAT",
        "dl" | "duellinks" | "duel links" => "Duel Links",
        "speed" | "speed duel" => "Speed Duel",
        "edison" => "Edison",
        _ => return None,
    })
}

fn rarity_alias(v: &str) -> Option<&'static str> {
    Some(match v {
        "n" | "c" | "common" | "comune" => "Common",
        "r" | "rare" | "rara" => "Rare",
        "sr" | "super" | "super rare" => "Super Rare",
        "ur" | "ultra" | "ultra rare" => "Ultra Rare",
        _ => return None,
    })
}

fn ban_alias(v: &str) -> Option<&'static str> {
    Some(match v {
        "forbidden" | "banned" | "proibita" | "proibite" => "Banned",
        "limited" | "limitata" | "limitate" => "Limited",
        "semi" | "semi-limited" | "semi-limitata" | "semilimitata" => "Semi-Limited",
        _ => return None,
    })
}

fn sort_alias(v: &str) -> Option<Sort> {
    Some(match v {
        "name" | "nome" => Sort::Name,
        "atk" => Sort::Atk,
        "def" => Sort::Def,
        "level" | "livello" | "lv" => Sort::Level,
        "popular" | "popolari" | "views" => Sort::Popular,
        "new" | "nuove" | "recenti" => Sort::New,
        "relevance" | "rilevanza" => Sort::Relevance,
        _ => return None,
    })
}

/// Splits `key<op>value` into its parts; `:` and `=` both mean "equals".
fn split_key(token: &str) -> Option<(String, &'static str, String)> {
    for op in [">=", "<=", "!=", ">", "<", "=", ":"] {
        if let Some(at) = token.find(op) {
            if at == 0 || token.starts_with('"') {
                return None;
            }
            let op = match op {
                ":" | "=" => "=",
                ">=" => ">=",
                "<=" => "<=",
                ">" => ">",
                "<" => "<",
                _ => return None,
            };
            let (key, rest) = token.split_at(at);
            let value = rest.trim_start_matches(['>', '<', '=', ':', '!']);
            return Some((key.to_lowercase(), op, unquote(value)));
        }
    }
    None
}

fn parse_range(op: &str, value: &str) -> Option<Range> {
    if op == "=" {
        if let Some((a, b)) = value.split_once('-') {
            return Some(Range { min: Some(a.trim().parse().ok()?), max: Some(b.trim().parse().ok()?) });
        }
        let n: i32 = value.parse().ok()?;
        return Some(Range { min: Some(n), max: Some(n) });
    }
    let n: i32 = value.parse().ok()?;
    Some(match op {
        ">=" => Range { min: Some(n), max: None },
        ">" => Range { min: Some(n + 1), max: None },
        "<=" => Range { min: None, max: Some(n) },
        "<" => Range { min: None, max: Some(n - 1) },
        _ => return None,
    })
}

fn range_label(name: &str, r: Range) -> String {
    match (r.min, r.max) {
        (Some(a), Some(b)) if a == b => format!("{name} = {a}"),
        (Some(a), Some(b)) => format!("{name} {a}–{b}"),
        (Some(a), None) => format!("{name} ≥ {a}"),
        (None, Some(b)) => format!("{name} ≤ {b}"),
        (None, None) => name.to_owned(),
    }
}

fn set_range(slot: &mut Range, op: &str, value: &str, name: &str) -> Option<String> {
    let range = parse_range(op, value)?;
    *slot = range;
    Some(range_label(name, range))
}

pub fn parse(input: &str) -> Parsed {
    let mut out = Parsed::default();
    for token in tokenize(input) {
        if token.starts_with('"') {
            let phrase = unquote(&token);
            if !phrase.is_empty() {
                out.query.phrases.push(phrase.clone());
                out.chips.push(Chip { token, label: format!("«{phrase}»") });
            }
            continue;
        }
        let Some((key, op, value)) = split_key(&token) else {
            out.query.words.push(token);
            continue;
        };
        let v = value.to_lowercase();
        let label = match key.as_str() {
            "t" | "type" | "tipo" => {
                if let Some(kind) = kind_alias(&v) {
                    out.query.kinds.push(kind);
                    let name = match kind {
                        CardKind::Monster => "Mostro",
                        CardKind::Spell => "Magia",
                        CardKind::Trap => "Trappola",
                        CardKind::Skill => "Abilità",
                        CardKind::Token => "Token",
                    };
                    Some(format!("Tipo: {name}"))
                } else if let Some((_, sub)) = SUBTYPES.iter().find(|(alias, _)| *alias == v) {
                    out.query.subtypes.push((*sub).to_owned());
                    Some(format!("Tipo: {sub}"))
                } else {
                    None
                }
            }
            "a" | "attr" | "attribute" | "attributo" => attribute_alias(&v).map(|a| {
                out.query.attributes.push(a.to_owned());
                format!("Attributo: {a}")
            }),
            "r" | "race" | "razza" => (!v.is_empty()).then(|| {
                out.query.races.push(value.clone());
                format!("Razza: {value}")
            }),
            "arch" | "archetype" | "archetipo" => (!v.is_empty()).then(|| {
                out.query.archetypes.push(value.clone());
                format!("Archetipo: {value}")
            }),
            "name" | "nome" => (!v.is_empty()).then(|| {
                out.query.name_words.extend(v.split_whitespace().map(str::to_owned));
                format!("Nome: {value}")
            }),
            "lv" | "level" | "livello" | "rank" => set_range(&mut out.query.level, op, &v, "Livello"),
            "atk" => set_range(&mut out.query.atk, op, &v, "ATK"),
            "def" => set_range(&mut out.query.def, op, &v, "DEF"),
            "link" => set_range(&mut out.query.link, op, &v, "Link"),
            "scale" | "scala" => set_range(&mut out.query.scale, op, &v, "Scala"),
            "f" | "format" | "formato" => format_alias(&v).map(|f| {
                out.query.formats.push(f.to_owned());
                format!("Formato: {f}")
            }),
            "rarity" | "rarità" | "rarita" => rarity_alias(&v).map(|r| {
                out.query.rarities.push(r.to_owned());
                format!("Rarità MD: {r}")
            }),
            "ban" => ban_alias(&v).map(|b| {
                out.query.bans.push(b.to_owned());
                format!("TCG: {b}")
            }),
            "sort" | "ordina" => sort_alias(&v).map(|s| {
                out.query.sort = s;
                let label = match s {
                    Sort::Relevance => "rilevanza",
                    Sort::Name => "nome",
                    Sort::Atk => "ATK",
                    Sort::Def => "DEF",
                    Sort::Level => "livello",
                    Sort::Popular => "più cercate",
                    Sort::New => "più recenti",
                };
                format!("Ordina: {label}")
            }),
            _ => {
                // Not a filter: treat "word:with:colons" as plain text.
                out.query.words.push(token.clone());
                continue;
            }
        };
        match label {
            Some(label) => out.chips.push(Chip { token, label }),
            None => out.errors.push(token),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_phrases_and_filters() {
        let p = parse(r#"aleister "negate the activation" attr:dark type:fusion atk>=2500 lv:4-6"#);
        assert_eq!(p.query.words, ["aleister"]);
        assert_eq!(p.query.phrases, ["negate the activation"]);
        assert_eq!(p.query.attributes, ["DARK"]);
        assert_eq!(p.query.subtypes, ["Fusion"]);
        assert_eq!(p.query.atk, Range { min: Some(2500), max: None });
        assert_eq!(p.query.level, Range { min: Some(4), max: Some(6) });
        assert_eq!(p.chips.len(), 5);
        assert!(p.errors.is_empty());
    }

    #[test]
    fn italian_aliases() {
        let p = parse("tipo:magia attributo:luce formato:md rarità:ur ban:limitata ordina:atk");
        assert_eq!(p.query.kinds, [CardKind::Spell]);
        assert_eq!(p.query.attributes, ["LIGHT"]);
        assert_eq!(p.query.formats, ["Master Duel"]);
        assert_eq!(p.query.rarities, ["Ultra Rare"]);
        assert_eq!(p.query.bans, ["Limited"]);
        assert_eq!(p.query.sort, Sort::Atk);
    }

    #[test]
    fn quoted_values_and_strict_comparisons() {
        let p = parse(r#"race:"winged beast" def<1000 atk>2000"#);
        assert_eq!(p.query.races, ["winged beast"]);
        assert_eq!(p.query.def, Range { min: None, max: Some(999) });
        assert_eq!(p.query.atk, Range { min: Some(2001), max: None });
    }

    #[test]
    fn bad_filters_are_reported() {
        let p = parse("attr:purple atk>=lots");
        assert_eq!(p.errors, ["attr:purple", "atk>=lots"]);
    }
}
