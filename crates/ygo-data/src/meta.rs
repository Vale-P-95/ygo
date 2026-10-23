//! The decks of the Master Duel meta and how much each is played, read
//! from Master Duel Meta when asked (nothing is kept between sessions): the
//! deck types by `popRank`, the share of the ranked top decks, each with
//! its most recent top decklist (a tournament one first).

use serde::Deserialize;

use crate::store::CardStore;
use crate::Error;

const TYPES: &str = "https://www.masterduelmeta.com/api/v1/deck-types?sort=-popRank&limit=60";
const TOP_DECKS: &str = "https://www.masterduelmeta.com/api/v1/top-decks?sort=-created&limit=150";

/// A deck type of the meta with a list.
#[derive(Debug, Clone, PartialEq)]
pub struct MetaList {
    pub name: String,
    /// `popRank`: the share of the ranked top decks, in %.
    pub weight: f64,
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
    /// Card names with no passcode in the card database (left out).
    pub missing: Vec<String>,
}

#[derive(Deserialize)]
struct DeckType {
    name: String,
    #[serde(rename = "popRank", default)]
    pop_rank: Option<f64>,
}

#[derive(Deserialize)]
struct TopDeck {
    #[serde(rename = "deckType", default)]
    deck_type: Option<Named>,
    #[serde(rename = "tournamentType", default)]
    tournament_type: Option<serde_json::Value>,
    #[serde(default)]
    main: Vec<Entry>,
    #[serde(default)]
    extra: Vec<Entry>,
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

#[derive(Deserialize)]
struct Entry {
    card: Named,
    amount: u32,
}

/// Downloads the deck types and the recent top decks of Master Duel Meta.
pub async fn fetch(client: &reqwest::Client) -> Result<(String, String), Error> {
    let types = client.get(TYPES).send().await?.error_for_status()?.text().await?;
    let decks = client.get(TOP_DECKS).send().await?.error_for_status()?.text().await?;
    Ok((types, decks))
}

/// The deck types played (`popRank` above 0), the most played first, each
/// with its most recent top decklist; a type with no recent list is left
/// out. `types` and `decks` are what [`fetch`] downloads.
pub fn meta_lists(types: &str, decks: &str, store: &CardStore) -> Result<Vec<MetaList>, Error> {
    let types: Vec<DeckType> = serde_json::from_str(types)?;
    let mut decks: Vec<TopDeck> = serde_json::from_str(decks)?;
    // The most recent first (as listed); tournament lists before the others.
    decks.sort_by_key(|d| d.tournament_type.as_ref().is_none_or(|t| t.is_null()));
    let mut out = Vec::new();
    for t in types.into_iter().filter(|t| t.pop_rank.unwrap_or(0.0) > 0.0) {
        let Some(deck) = decks.iter().find(|d| d.deck_type.as_ref().is_some_and(|k| k.name == t.name)) else { continue };
        let mut missing = Vec::new();
        let mut cards = |entries: &[Entry]| -> Result<Vec<u32>, Error> {
            let mut out = Vec::new();
            for e in entries {
                match store.id_by_name(&e.card.name)? {
                    Some(id) => out.extend(std::iter::repeat_n(id, e.amount as usize)),
                    None => missing.push(e.card.name.clone()),
                }
            }
            Ok(out)
        };
        let (main, extra) = (cards(&deck.main)?, cards(&deck.extra)?);
        out.push(MetaList { name: t.name, weight: t.pop_rank.unwrap_or(0.0), main, extra, missing });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_follow_the_types_by_play() {
        let store = CardStore::open_in_memory().unwrap();
        let types = r#"[{"name":"B","popRank":20.5},{"name":"A","popRank":10},{"name":"None","popRank":0},{"name":"C","popRank":5}]"#;
        let decks = r#"[
            {"deckType":{"name":"A"},"main":[{"card":{"name":"x"},"amount":3}],"extra":[]},
            {"deckType":{"name":"B"},"tournamentType":null,"main":[],"extra":[]},
            {"deckType":{"name":"B"},"tournamentType":{"name":"Cup"},"main":[{"card":{"name":"y"},"amount":1}],"extra":[]}
        ]"#;
        let lists = meta_lists(types, decks, &store).unwrap();
        let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
        // C has no list; None is not played.
        assert_eq!(names, ["B", "A"]);
        assert_eq!(lists[0].weight, 20.5);
        // The tournament list of B, whose card is unknown to an empty database.
        assert_eq!(lists[0].missing, ["y"]);
        assert_eq!(lists[1].missing, ["x"]);
    }
}
