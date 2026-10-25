//! Card data for DuelForge: the local card database built from YGOPRODeck,
//! the search language, the image cache and the data downloads.

pub mod banlist;
pub mod deck;
pub mod images;
pub mod import;
pub mod matchlog;
pub mod meta;
pub mod model;
pub mod psct;
pub mod query;
pub mod store;
pub mod sync;
pub mod tags;
pub mod userdb;

pub use banlist::Banlist;
pub use deck::{Deck, DeckBox, Format, Issue, Legality, Severity};
pub use images::{ImageCache, ImageSize};
pub use matchlog::{CardChange, DeckList, MatchEntry};
pub use model::{BanChange, Card, CardKind, Facets};
pub use query::{parse as parse_query, Chip, Parsed, SearchQuery, Sort};
pub use store::CardStore;
pub use tags::Tag;
pub use userdb::{BoardChoice, DeckSummary, Recipe, SavedSearch, UserStore};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("network error: {0}")]
    Http(String),
    #[error("archive error: {0}")]
    Zip(String),
    #[error("{0}")]
    Data(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e.to_string())
    }
}
