//! The user's own data (`user.db`): saved decks, favourite cards and saved
//! searches. Kept apart from the card data, which can be re-downloaded at
//! any time.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::deck::{Deck, DeckBox, Format};
use crate::Error;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS decks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    main TEXT NOT NULL,      -- JSON array of passcodes
    extra TEXT NOT NULL,
    side TEXT NOT NULL,
    maybe TEXT NOT NULL DEFAULT '[]',
    roles TEXT NOT NULL DEFAULT '{}',  -- JSON object: role -> passcodes
    deck_box TEXT NOT NULL DEFAULT '{}',  -- JSON: shelf appearance
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS favorites (
    id INTEGER PRIMARY KEY,
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS saved_searches (
    name TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSearch {
    pub name: String,
    pub query: String,
}

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
    /// First showcase card, else the first card of the Main Deck, for a thumbnail.
    pub cover: Option<u32>,
    #[serde(rename = "box")]
    pub deck_box: DeckBox,
}

pub struct UserStore {
    conn: Connection,
}

/// Brings databases created by older versions up to the current schema.
fn migrate(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(SCHEMA)?;
    // Columns added after the first release, with their defaults.
    for (column, default) in [("maybe", "'[]'"), ("roles", "'{}'"), ("deck_box", "'{}'")] {
        let present: bool =
            conn.query_row("SELECT COUNT(*) FROM pragma_table_info('decks') WHERE name = ?", [column], |r| r.get::<_, i64>(0))? > 0;
        if !present {
            conn.execute_batch(&format!("ALTER TABLE decks ADD COLUMN {column} TEXT NOT NULL DEFAULT {default}"))?;
        }
    }
    Ok(())
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
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn list(&self) -> Result<Vec<DeckSummary>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, format, main, extra, side, updated_at, deck_box FROM decks ORDER BY updated_at DESC, id DESC")?;
        let rows = stmt.query_map([], |r| {
            let main = ids(&r.get::<_, String>(3)?);
            let deck_box: DeckBox = serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or_default();
            Ok(DeckSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                format: Format::parse(&r.get::<_, String>(2)?).unwrap_or_default(),
                main_count: main.len(),
                extra_count: ids(&r.get::<_, String>(4)?).len(),
                side_count: ids(&r.get::<_, String>(5)?).len(),
                updated_at: r.get(6)?,
                cover: deck_box.showcase.first().or(main.first()).copied(),
                deck_box,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get(&self, id: i64) -> Result<Option<Deck>, Error> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, format, main, extra, side, maybe, roles, updated_at, deck_box FROM decks WHERE id = ?",
                [id],
                |r| {
                    Ok(Deck {
                        id: Some(r.get(0)?),
                        name: r.get(1)?,
                        format: Format::parse(&r.get::<_, String>(2)?).unwrap_or_default(),
                        main: ids(&r.get::<_, String>(3)?),
                        extra: ids(&r.get::<_, String>(4)?),
                        side: ids(&r.get::<_, String>(5)?),
                        maybe: ids(&r.get::<_, String>(6)?),
                        roles: serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or_default(),
                        updated_at: Some(r.get(8)?),
                        deck_box: serde_json::from_str(&r.get::<_, String>(9)?).unwrap_or_default(),
                    })
                },
            )
            .optional()?)
    }

    /// Inserts a new deck or updates an existing one; returns it as stored.
    pub fn save(&self, deck: &Deck) -> Result<Deck, Error> {
        let json = |ids: &Vec<u32>| serde_json::to_string(ids);
        let (main, extra, side, maybe) = (json(&deck.main)?, json(&deck.extra)?, json(&deck.side)?, json(&deck.maybe)?);
        let roles = serde_json::to_string(&deck.roles)?;
        let deck_box = serde_json::to_string(&deck.deck_box)?;
        let name = if deck.name.trim().is_empty() { "Nuovo deck" } else { deck.name.trim() };
        let id = match deck.id {
            Some(id) => {
                let changed = self.conn.execute(
                    "UPDATE decks SET name = ?2, format = ?3, main = ?4, extra = ?5, side = ?6, maybe = ?7, roles = ?8, deck_box = ?9, updated_at = datetime('now') WHERE id = ?1",
                    params![id, name, deck.format.as_str(), main, extra, side, maybe, roles, deck_box],
                )?;
                if changed == 0 {
                    return Err(Error::Data(format!("il deck {id} non esiste più")));
                }
                id
            }
            None => {
                self.conn.execute(
                    "INSERT INTO decks (name, format, main, extra, side, maybe, roles, deck_box) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![name, deck.format.as_str(), main, extra, side, maybe, roles, deck_box],
                )?;
                self.conn.last_insert_rowid()
            }
        };
        self.get(id)?.ok_or_else(|| Error::Data("deck salvato ma non trovato".into()))
    }

    /// Changes how a deck looks on the shelf, without counting as an edit of the deck.
    pub fn set_box(&self, id: i64, deck_box: &DeckBox) -> Result<(), Error> {
        let changed = self.conn.execute("UPDATE decks SET deck_box = ?2 WHERE id = ?1", params![id, serde_json::to_string(deck_box)?])?;
        if changed == 0 {
            return Err(Error::Data(format!("il deck {id} non esiste più")));
        }
        Ok(())
    }

    /// Copies a deck under the name "<name> (copia)"; returns the copy.
    pub fn duplicate(&self, id: i64) -> Result<Deck, Error> {
        let deck = self.get(id)?.ok_or_else(|| Error::Data(format!("il deck {id} non esiste più")))?;
        self.save(&Deck { id: None, name: format!("{} (copia)", deck.name), ..deck })
    }

    pub fn delete(&self, id: i64) -> Result<(), Error> {
        self.conn.execute("DELETE FROM decks WHERE id = ?", [id])?;
        Ok(())
    }

    /// Favourite cards, by passcode.
    pub fn favorites(&self) -> Result<Vec<u32>, Error> {
        let mut stmt = self.conn.prepare("SELECT id FROM favorites ORDER BY id")?;
        let ids = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(ids)
    }

    pub fn set_favorite(&self, id: u32, favorite: bool) -> Result<(), Error> {
        if favorite {
            self.conn.execute("INSERT OR IGNORE INTO favorites (id) VALUES (?)", [id])?;
        } else {
            self.conn.execute("DELETE FROM favorites WHERE id = ?", [id])?;
        }
        Ok(())
    }

    pub fn saved_searches(&self) -> Result<Vec<SavedSearch>, Error> {
        let mut stmt = self.conn.prepare("SELECT name, query FROM saved_searches ORDER BY created_at, rowid")?;
        let rows = stmt.query_map([], |r| Ok(SavedSearch { name: r.get(0)?, query: r.get(1)? }))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Saves a search under `name`, replacing one with the same name.
    pub fn save_search(&self, name: &str, query: &str) -> Result<(), Error> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Data("dai un nome alla ricerca".into()));
        }
        self.conn.execute("INSERT OR REPLACE INTO saved_searches (name, query) VALUES (?1, ?2)", [name, query.trim()])?;
        Ok(())
    }

    pub fn delete_search(&self, name: &str) -> Result<(), Error> {
        self.conn.execute("DELETE FROM saved_searches WHERE name = ?", [name])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

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
    fn keeps_the_maybe_board_and_migrates_old_databases() {
        let dir = std::env::temp_dir().join(format!("duelforge-userdb-{}", std::process::id()));
        let path = dir.join("user.db");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            // A database from before the maybe board existed.
            let conn = Connection::open(&path).unwrap();
            let old_schema = SCHEMA
                .replace("    maybe TEXT NOT NULL DEFAULT '[]',\n", "")
                .replace("    roles TEXT NOT NULL DEFAULT '{}',  -- JSON object: role -> passcodes\n", "")
                .replace("    deck_box TEXT NOT NULL DEFAULT '{}',  -- JSON: shelf appearance\n", "");
            assert!(!old_schema.contains("maybe") && !old_schema.contains("roles") && !old_schema.contains("deck_box"));
            conn.execute_batch(&old_schema).unwrap();
            conn.execute("INSERT INTO decks (name, format, main, extra, side) VALUES ('Old', 'md', '[1]', '[]', '[]')", []).unwrap();
        }
        let store = UserStore::open(&path).unwrap();
        let old = store.get(1).unwrap().unwrap();
        assert_eq!((old.main.as_slice(), old.maybe.as_slice()), (&[1][..], &[][..]));
        let roles = BTreeMap::from([("starter".to_owned(), vec![1])]);
        let saved = store.save(&Deck { maybe: vec![7, 7], roles: roles.clone(), ..old }).unwrap();
        assert_eq!(saved.maybe, [7, 7]);
        assert_eq!(saved.roles, roles);
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deck_box_duplicate_and_cover() {
        let store = UserStore::open_in_memory().unwrap();
        let deck = store.save(&Deck { name: "Invoked".into(), main: vec![1, 2], ..Default::default() }).unwrap();
        let id = deck.id.unwrap();
        assert_eq!(store.list().unwrap()[0].cover, Some(1));
        let look = DeckBox { color: "#8b1e1e".into(), material: "leather".into(), shape: "tin".into(), showcase: vec![2, 1] };
        store.set_box(id, &look).unwrap();
        assert_eq!(store.get(id).unwrap().unwrap().deck_box, look);
        let summary = &store.list().unwrap()[0];
        assert_eq!((summary.cover, &summary.deck_box), (Some(2), &look));

        let copy = store.duplicate(id).unwrap();
        assert_ne!(copy.id, Some(id));
        assert_eq!((copy.name.as_str(), copy.main.as_slice(), &copy.deck_box), ("Invoked (copia)", &[1, 2][..], &look));
        assert!(store.set_box(999, &look).is_err());
    }

    #[test]
    fn favorites_and_saved_searches() {
        let store = UserStore::open_in_memory().unwrap();
        store.set_favorite(5, true).unwrap();
        store.set_favorite(3, true).unwrap();
        store.set_favorite(5, true).unwrap();
        assert_eq!(store.favorites().unwrap(), [3, 5]);
        store.set_favorite(5, false).unwrap();
        assert_eq!(store.favorites().unwrap(), [3]);

        store.save_search("Handtrap MD", "tag:handtrap format:md").unwrap();
        store.save_search("Invoked", "arch:invoked").unwrap();
        store.save_search("Handtrap MD", "tag:handtrap format:md sort:popular").unwrap();
        let saved = store.saved_searches().unwrap();
        assert_eq!(saved.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Invoked", "Handtrap MD"]);
        assert_eq!(saved[1].query, "tag:handtrap format:md sort:popular");
        assert!(store.save_search(" ", "x").is_err());
        store.delete_search("Invoked").unwrap();
        assert_eq!(store.saved_searches().unwrap().len(), 1);
    }

    #[test]
    fn blank_names_get_a_default() {
        let store = UserStore::open_in_memory().unwrap();
        assert_eq!(store.save(&Deck { name: "  ".into(), ..Default::default() }).unwrap().name, "Nuovo deck");
    }
}
