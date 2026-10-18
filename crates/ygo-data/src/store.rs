//! Local card database (`cards.db`, SQLite with an FTS5 full-text index).

use std::path::Path;

use rusqlite::types::Value;
use rusqlite::{params, Connection, OptionalExtension};

use crate::banlist::{self, Banlist};
use crate::model::{Card, CardKind, Facets};
use crate::query::{Range, SearchQuery, Sort};
use crate::tags::Tag;
use crate::Error;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS cards (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    frame TEXT NOT NULL,
    human_type TEXT NOT NULL,
    type_line TEXT NOT NULL,      -- '|Machine|Fusion|Effect|' for LIKE matching
    desc TEXT NOT NULL,
    pend_desc TEXT,
    race TEXT NOT NULL,
    attribute TEXT,
    archetype TEXT,
    level INTEGER,
    atk INTEGER,
    def INTEGER,
    scale INTEGER,
    link_rating INTEGER,
    link_markers TEXT NOT NULL,   -- JSON array
    md_rarity TEXT,
    formats TEXT NOT NULL,        -- '|TCG|OCG|Master Duel|'
    tcg_date TEXT,
    ocg_date TEXT,
    ban_tcg TEXT,
    ban_ocg TEXT,
    image_ids TEXT NOT NULL,      -- JSON array
    views INTEGER NOT NULL
);
-- Lists not in the card data (Master Duel); kept when the cards are replaced.
CREATE TABLE IF NOT EXISTS banlists (
    format TEXT NOT NULL,
    id INTEGER NOT NULL,
    status TEXT NOT NULL,
    PRIMARY KEY (format, id)
);
CREATE VIRTUAL TABLE IF NOT EXISTS cards_fts USING fts5(
    name, desc, content='cards', content_rowid='id', tokenize='unicode61 remove_diacritics 2'
);
"#;

const COLUMNS: &str = "id, name, kind, frame, human_type, type_line, desc, pend_desc, race, attribute, archetype, level, atk, def, \
                       scale, link_rating, link_markers, md_rarity, formats, tcg_date, ocg_date, ban_tcg, ban_ocg, image_ids, views";

pub struct CardStore {
    conn: Connection,
}

fn bar_list(items: &[String]) -> String {
    if items.is_empty() {
        String::new()
    } else {
        format!("|{}|", items.join("|"))
    }
}

fn split_bars(s: &str) -> Vec<String> {
    s.split('|').filter(|x| !x.is_empty()).map(str::to_owned).collect()
}

/// Quotes a term for an FTS5 query.
fn fts_term(word: &str, prefix: bool) -> String {
    format!("\"{}\"{}", word.replace('"', "\"\""), if prefix { "*" } else { "" })
}

fn fts_expression(q: &SearchQuery) -> Option<String> {
    let mut parts: Vec<String> = q.words.iter().map(|w| fts_term(w, true)).collect();
    parts.extend(q.phrases.iter().map(|p| fts_term(p, false)));
    parts.extend(q.name_words.iter().map(|w| format!("name : {}", fts_term(w, true))));
    (!parts.is_empty()).then(|| parts.join(" AND "))
}

fn push_range(column: &str, range: Range, clauses: &mut Vec<String>, args: &mut Vec<Value>) {
    if let Some(min) = range.min {
        clauses.push(format!("c.{column} >= ?"));
        args.push(Value::Integer(min.into()));
    }
    if let Some(max) = range.max {
        clauses.push(format!("c.{column} <= ?"));
        args.push(Value::Integer(max.into()));
    }
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

impl CardStore {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, Error> {
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Replaces the whole card list and records the source version.
    pub fn replace_all(&mut self, cards: &[Card], version: &str) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM cards", [])?;
        {
            let mut insert = tx.prepare(&format!("INSERT INTO cards ({COLUMNS}) VALUES ({})", placeholders(25)))?;
            for c in cards {
                insert.execute(params![
                    c.id,
                    c.name,
                    c.kind.as_str(),
                    c.frame,
                    c.human_type,
                    bar_list(&c.type_line),
                    c.desc,
                    c.pend_desc,
                    c.race,
                    c.attribute,
                    c.archetype,
                    c.level,
                    c.atk,
                    c.def,
                    c.scale,
                    c.link_rating,
                    serde_json::to_string(&c.link_markers)?,
                    c.md_rarity,
                    bar_list(&c.formats),
                    c.tcg_date,
                    c.ocg_date,
                    c.ban_tcg,
                    c.ban_ocg,
                    serde_json::to_string(&c.image_ids)?,
                    c.views as i64,
                ])?;
            }
        }
        tx.execute("INSERT INTO cards_fts(cards_fts) VALUES ('rebuild')", [])?;
        tx.execute("INSERT OR REPLACE INTO meta (key, value) VALUES ('version', ?)", [version])?;
        tx.commit()?;
        Ok(())
    }

    /// Replaces the stored list for `list.format`.
    pub fn set_banlist(&mut self, list: &Banlist) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM banlists WHERE format = ?", [&list.format])?;
        {
            let mut insert = tx.prepare("INSERT OR REPLACE INTO banlists (format, id, status) VALUES (?1, ?2, ?3)")?;
            for e in &list.cards {
                insert.execute(params![list.format, e.id, banlist::status_name(e.limit)])?;
            }
        }
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            [format!("banlist_{}", list.format), list.effective.clone()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Effective date of the stored list for `format` (`md`).
    pub fn banlist_date(&self, format: &str) -> Option<String> {
        self.conn.query_row("SELECT value FROM meta WHERE key = ?", [format!("banlist_{format}")], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn version(&self) -> Option<String> {
        self.conn.query_row("SELECT value FROM meta WHERE key = 'version'", [], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn count(&self) -> Result<usize, Error> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM cards", [], |r| r.get::<_, i64>(0))? as usize)
    }

    /// Ids of the matching cards, in display order.
    pub fn search(&self, q: &SearchQuery) -> Result<Vec<u32>, Error> {
        let mut clauses: Vec<String> = Vec::new();
        let mut args: Vec<Value> = Vec::new();
        let fts = fts_expression(q);
        let mut sql = String::from("SELECT c.id FROM cards c");
        if let Some(expr) = &fts {
            sql.push_str(" JOIN cards_fts ON cards_fts.rowid = c.id");
            clauses.push("cards_fts MATCH ?".into());
            args.push(Value::Text(expr.clone()));
        }
        if q.kinds.is_empty() {
            // Skills (Speed Duel) and tokens are not deck cards.
            clauses.push("c.kind NOT IN ('skill', 'token')".into());
        } else {
            clauses.push(format!("c.kind IN ({})", placeholders(q.kinds.len())));
            args.extend(q.kinds.iter().map(|k| Value::Text(k.as_str().into())));
        }
        for sub in &q.subtypes {
            clauses.push("(c.type_line LIKE ? OR (c.kind IN ('spell', 'trap') AND c.race = ?))".into());
            args.push(Value::Text(format!("%|{sub}|%")));
            args.push(Value::Text(sub.clone()));
        }
        if !q.attributes.is_empty() {
            clauses.push(format!("c.attribute IN ({})", placeholders(q.attributes.len())));
            args.extend(q.attributes.iter().map(|a| Value::Text(a.clone())));
        }
        if !q.races.is_empty() {
            clauses.push(format!("lower(c.race) IN ({}) AND c.kind = 'monster'", placeholders(q.races.len())));
            args.extend(q.races.iter().map(|r| Value::Text(r.to_lowercase())));
        }
        if !q.archetypes.is_empty() {
            let any = vec!["c.archetype LIKE ?"; q.archetypes.len()].join(" OR ");
            clauses.push(format!("({any})"));
            args.extend(q.archetypes.iter().map(|a| Value::Text(format!("%{a}%"))));
        }
        push_range("level", q.level, &mut clauses, &mut args);
        push_range("atk", q.atk, &mut clauses, &mut args);
        push_range("def", q.def, &mut clauses, &mut args);
        push_range("link_rating", q.link, &mut clauses, &mut args);
        push_range("scale", q.scale, &mut clauses, &mut args);
        for format in &q.formats {
            clauses.push("c.formats LIKE ?".into());
            args.push(Value::Text(format!("%|{format}|%")));
        }
        if !q.rarities.is_empty() {
            clauses.push(format!("c.md_rarity IN ({})", placeholders(q.rarities.len())));
            args.extend(q.rarities.iter().map(|r| Value::Text(r.clone())));
        }
        for tag in &q.tags {
            if *tag == Tag::Favorite {
                if q.favorites.is_empty() {
                    clauses.push("0".into());
                } else {
                    clauses.push(format!("c.id IN ({})", placeholders(q.favorites.len())));
                    args.extend(q.favorites.iter().map(|id| Value::Integer(*id as i64)));
                }
            } else {
                clauses.push(format!("c.name IN ({})", placeholders(tag.names().len())));
                args.extend(tag.names().iter().map(|n| Value::Text((*n).to_owned())));
            }
        }
        if !q.bans.is_empty() {
            if q.formats.iter().any(|f| f == "Master Duel") {
                // Master Duel cards follow Master Duel's own list.
                clauses
                    .push(format!("c.id IN (SELECT id FROM banlists WHERE format = 'md' AND status IN ({}))", placeholders(q.bans.len())));
                args.extend(q.bans.iter().map(|b| Value::Text(if b == "Banned" { "Forbidden".into() } else { b.clone() })));
            } else {
                clauses.push(format!("c.ban_tcg IN ({})", placeholders(q.bans.len())));
                args.extend(q.bans.iter().map(|b| Value::Text(b.clone())));
            }
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(match (q.sort, fts.is_some()) {
            (Sort::Relevance, true) => " ORDER BY bm25(cards_fts, 10.0, 1.0), c.name",
            (Sort::Relevance | Sort::Name, _) => " ORDER BY c.name",
            (Sort::Atk, _) => " ORDER BY c.atk DESC NULLS LAST, c.name",
            (Sort::Def, _) => " ORDER BY c.def DESC NULLS LAST, c.name",
            (Sort::Level, _) => " ORDER BY COALESCE(c.level, c.link_rating) DESC NULLS LAST, c.name",
            (Sort::Popular, _) => " ORDER BY c.views DESC, c.name",
            (Sort::New, _) => " ORDER BY COALESCE(c.tcg_date, c.ocg_date) DESC NULLS LAST, c.name",
        });
        let mut stmt = self.conn.prepare(&sql)?;
        let ids = stmt.query_map(rusqlite::params_from_iter(args), |r| r.get::<_, u32>(0))?.collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Cards for `ids`, in the same order (unknown ids are skipped).
    pub fn get_many(&self, ids: &[u32]) -> Result<Vec<Card>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {COLUMNS}, (SELECT status FROM banlists b WHERE b.format = 'md' AND b.id = cards.id) FROM cards WHERE id IN ({})",
            placeholders(ids.len())
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut found = std::collections::HashMap::with_capacity(ids.len());
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |r| {
            let kind: String = r.get(2)?;
            Ok(Card {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: CardKind::parse(&kind).unwrap_or(CardKind::Monster),
                frame: r.get(3)?,
                human_type: r.get(4)?,
                type_line: split_bars(&r.get::<_, String>(5)?),
                desc: r.get(6)?,
                pend_desc: r.get(7)?,
                race: r.get(8)?,
                attribute: r.get(9)?,
                archetype: r.get(10)?,
                level: r.get(11)?,
                atk: r.get(12)?,
                def: r.get(13)?,
                scale: r.get(14)?,
                link_rating: r.get(15)?,
                link_markers: serde_json::from_str(&r.get::<_, String>(16)?).unwrap_or_default(),
                md_rarity: r.get(17)?,
                formats: split_bars(&r.get::<_, String>(18)?),
                tcg_date: r.get(19)?,
                ocg_date: r.get(20)?,
                ban_tcg: r.get(21)?,
                ban_ocg: r.get(22)?,
                image_ids: serde_json::from_str(&r.get::<_, String>(23)?).unwrap_or_default(),
                views: r.get::<_, i64>(24)? as u64,
                ban_md: r.get(25)?,
            })
        })?;
        for card in rows {
            let card = card?;
            found.insert(card.id, card);
        }
        Ok(ids.iter().filter_map(|id| found.remove(id)).collect())
    }

    /// The card a passcode belongs to. Decks may use the passcode of an
    /// alternate artwork, which YGOPRODeck lists among a card's images.
    pub fn resolve_id(&self, id: u32) -> Result<Option<u32>, Error> {
        let sql = "SELECT id FROM cards WHERE id = ?1 \
                   UNION ALL \
                   SELECT id FROM cards WHERE instr(',' || replace(replace(image_ids, '[', ''), ']', '') || ',', ',' || ?1 || ',') > 0 \
                   LIMIT 1";
        Ok(self.conn.query_row(sql, [id], |r| r.get(0)).optional()?)
    }

    /// Values offered by the filter menus.
    pub fn facets(&self) -> Result<Facets, Error> {
        let list = |sql: &str| -> Result<Vec<String>, Error> {
            let mut stmt = self.conn.prepare(sql)?;
            let values = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
            Ok(values)
        };
        let mut formats: Vec<String> = list("SELECT DISTINCT formats FROM cards")?.iter().flat_map(|f| split_bars(f)).collect();
        formats.sort();
        formats.dedup();
        Ok(Facets {
            attributes: list("SELECT DISTINCT attribute FROM cards WHERE attribute IS NOT NULL ORDER BY 1")?,
            races: list("SELECT DISTINCT race FROM cards WHERE kind = 'monster' ORDER BY 1")?,
            archetypes: list("SELECT DISTINCT archetype FROM cards WHERE archetype IS NOT NULL ORDER BY 1")?,
            formats,
        })
    }
}
