//! The user's own data (`user.db`): saved decks. Kept apart from the card
//! data, which can be re-downloaded at any time.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::deck::{Deck, Format};
use crate::Error;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS decks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    main TEXT NOT NULL,      -- JSON array of passcodes
    extra TEXT NOT NULL,
    side TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckSummary {
    pub id: i64,
    pub name: String,
    pub format: Format,
    pub main_count: usize,
    pub extra_count: usize,
    pub side_count: usize,
    pub updated_at: String,
    /// First card of the Main Deck, for a thumbnail.
    pub cover: Option<u32>,
}

pub struct UserStore {
    conn: Connection,
}

fn ids(json: &str) -> Vec<u32> {
    serde_json::from_str(json).unwrap_or_default()
}

impl UserStore {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn list(&self) -> Result<Vec<DeckSummary>, Error> {
        let mut stmt =
            self.conn.prepare("SELECT id, name, format, main, extra, side, updated_at FROM decks ORDER BY updated_at DESC, id DESC")?;
        let rows = stmt.query_map([], |r| {
            let main = ids(&r.get::<_, String>(3)?);
            Ok(DeckSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                format: Format::parse(&r.get::<_, String>(2)?).unwrap_or_default(),
                main_count: main.len(),
                extra_count: ids(&r.get::<_, String>(4)?).len(),
                side_count: ids(&r.get::<_, String>(5)?).len(),
                updated_at: r.get(6)?,
                cover: main.first().copied(),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get(&self, id: i64) -> Result<Option<Deck>, Error> {
        Ok(self
            .conn
            .query_row("SELECT id, name, format, main, extra, side, updated_at FROM decks WHERE id = ?", [id], |r| {
                Ok(Deck {
                    id: Some(r.get(0)?),
                    name: r.get(1)?,
                    format: Format::parse(&r.get::<_, String>(2)?).unwrap_or_default(),
                    main: ids(&r.get::<_, String>(3)?),
                    extra: ids(&r.get::<_, String>(4)?),
                    side: ids(&r.get::<_, String>(5)?),
                    updated_at: Some(r.get(6)?),
                })
            })
            .optional()?)
    }

    /// Inserts a new deck or updates an existing one; returns it as stored.
    pub fn save(&self, deck: &Deck) -> Result<Deck, Error> {
        let (main, extra, side) =
            (serde_json::to_string(&deck.main)?, serde_json::to_string(&deck.extra)?, serde_json::to_string(&deck.side)?);
        let name = if deck.name.trim().is_empty() { "Nuovo deck" } else { deck.name.trim() };
        let id = match deck.id {
            Some(id) => {
                let changed = self.conn.execute(
                    "UPDATE decks SET name = ?2, format = ?3, main = ?4, extra = ?5, side = ?6, updated_at = datetime('now') WHERE id = ?1",
                    params![id, name, deck.format.as_str(), main, extra, side],
                )?;
                if changed == 0 {
                    return Err(Error::Data(format!("il deck {id} non esiste più")));
                }
                id
            }
            None => {
                self.conn.execute(
                    "INSERT INTO decks (name, format, main, extra, side) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![name, deck.format.as_str(), main, extra, side],
                )?;
                self.conn.last_insert_rowid()
            }
        };
        self.get(id)?.ok_or_else(|| Error::Data("deck salvato ma non trovato".into()))
    }

    pub fn delete(&self, id: i64) -> Result<(), Error> {
        self.conn.execute("DELETE FROM decks WHERE id = ?", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_list_update_delete() {
        let store = UserStore::open_in_memory().unwrap();
        let saved = store.save(&Deck { name: "Invoked".into(), main: vec![1, 2, 2], extra: vec![3], ..Default::default() }).unwrap();
        let id = saved.id.unwrap();
        assert_eq!(saved.main, [1, 2, 2]);
        assert_eq!(saved.format, Format::Md);

        let updated = store.save(&Deck { name: "Invoked MD".into(), format: Format::Tcg, ..saved }).unwrap();
        assert_eq!(updated.id, Some(id));
        assert_eq!(updated.format, Format::Tcg);

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].name.as_str(), list[0].main_count, list[0].extra_count, list[0].cover), ("Invoked MD", 3, 1, Some(1)));

        store.delete(id).unwrap();
        assert!(store.get(id).unwrap().is_none());
        assert!(store.save(&Deck { id: Some(id), ..Default::default() }).is_err(), "updating a deleted deck fails");
    }

    #[test]
    fn blank_names_get_a_default() {
        let store = UserStore::open_in_memory().unwrap();
        assert_eq!(store.save(&Deck { name: "  ".into(), ..Default::default() }).unwrap().name, "Nuovo deck");
    }
}
