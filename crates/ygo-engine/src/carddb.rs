//! Card data loaded from EDOPro card databases (`.cdb` SQLite files from
//! Project Ignis BabelCDB).

use std::collections::HashMap;
use std::path::Path;

use crate::consts::card_type;
use crate::Error;

#[derive(Debug, Clone, Default)]
pub struct Card {
    pub code: u32,
    pub alias: u32,
    pub setcodes: Vec<u16>,
    pub type_: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u64,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
    pub name: String,
    pub desc: String,
    /// Effect descriptions referenced by `desc` values (`str1`..`str16`).
    pub strings: Vec<String>,
}

impl Card {
    pub fn is_extra_deck_monster(&self) -> bool {
        self.type_ & card_type::EXTRA != 0
    }
}

#[derive(Debug, Default)]
pub struct CardDb {
    cards: HashMap<u32, Card>,
}

/// Databases that are not part of the official TCG/OCG card pool.
fn is_official_pool(file_name: &str) -> bool {
    !(file_name.contains("rush") || file_name.contains("skill") || file_name.contains("unofficial") || file_name.contains("goat"))
}

impl CardDb {
    /// Loads `cards.cdb`, then `release-*.cdb`, then `prerelease-*.cdb` from
    /// `dir`; later files override earlier ones.
    pub fn load_dir(dir: &Path) -> Result<Self, Error> {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| Error::Data(format!("cannot read {}: {e}", dir.display())))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "cdb"))
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(is_official_pool))
            .collect();
        let rank = |p: &std::path::PathBuf| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            (
                if name == "cards.cdb" {
                    0
                } else if name.starts_with("release") {
                    1
                } else {
                    2
                },
                name.to_owned(),
            )
        };
        files.sort_by_key(rank);
        if files.is_empty() {
            return Err(Error::Data(format!("no .cdb files in {}", dir.display())));
        }
        let mut db = CardDb::default();
        for file in files {
            db.load_file(&file)?;
        }
        Ok(db)
    }

    pub fn load_file(&mut self, path: &Path) -> Result<(), Error> {
        let conn = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut stmt = conn.prepare(
            "SELECT d.id, d.alias, d.setcode, d.type, d.atk, d.def, d.level, d.race, d.attribute,
                    t.name, t.desc, t.str1, t.str2, t.str3, t.str4, t.str5, t.str6, t.str7, t.str8,
                    t.str9, t.str10, t.str11, t.str12, t.str13, t.str14, t.str15, t.str16
             FROM datas d JOIN texts t ON t.id = d.id",
        )?;
        let rows = stmt.query_map([], |row| {
            let setcode: i64 = row.get(2)?;
            let type_: u32 = row.get::<_, i64>(3)? as u32;
            let def: i32 = row.get(5)?;
            let level: i64 = row.get(6)?;
            let mut card = Card {
                code: row.get::<_, i64>(0)? as u32,
                alias: row.get::<_, i64>(1)? as u32,
                setcodes: (0..4).map(|i| ((setcode as u64 >> (i * 16)) & 0xffff) as u16).filter(|&s| s != 0).collect(),
                type_,
                level: (level & 0xff) as u32,
                attribute: row.get::<_, i64>(8)? as u32,
                race: row.get::<_, i64>(7)? as u64,
                attack: row.get(4)?,
                defense: def,
                lscale: ((level >> 24) & 0xff) as u32,
                rscale: ((level >> 16) & 0xff) as u32,
                link_marker: 0,
                name: row.get(9)?,
                desc: row.get(10)?,
                strings: (11..27).map(|i| row.get::<_, Option<String>>(i).map(|s| s.unwrap_or_default())).collect::<Result<_, _>>()?,
            };
            if type_ & card_type::LINK != 0 {
                card.link_marker = def as u32;
                card.defense = 0;
            }
            Ok(card)
        })?;
        for card in rows {
            let card = card?;
            self.cards.insert(card.code, card);
        }
        Ok(())
    }

    pub fn get(&self, code: u32) -> Option<&Card> {
        self.cards.get(&code)
    }

    /// The code whose script a card uses: alternate artworks point to the
    /// original through `alias` (the engine's rule: codes less than 10 apart).
    pub fn canonical_code(&self, code: u32) -> u32 {
        match self.get(code) {
            Some(card) if card.alias != 0 && card.alias.abs_diff(code) < 10 => card.alias,
            _ => code,
        }
    }

    pub fn name(&self, code: u32) -> String {
        self.get(code).map_or_else(|| format!("#{code}"), |c| c.name.clone())
    }

    /// Finds a card by exact name (case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Option<&Card> {
        let wanted = name.trim().to_lowercase();
        self.cards.values().filter(|c| c.name.to_lowercase() == wanted).min_by_key(|c| (c.alias != 0, c.code))
    }

    /// Human-readable text for an effect description id (`code << 20 | index`).
    pub fn describe(&self, description: u64) -> String {
        let code = (description >> 20) as u32;
        let index = (description & 0xfffff) as usize;
        match self.get(code) {
            Some(card) => match card.strings.get(index).filter(|s| !s.is_empty()) {
                Some(text) => text.clone(),
                None => format!("effect of {}", card.name),
            },
            None if description == 0 => String::new(),
            None => format!("system string {description}"),
        }
    }

    /// Every card, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &Card> {
        self.cards.values()
    }

    /// Codes the engine accepts for a card declaration with filter `opcodes`,
    /// sorted by code.
    pub fn declarable(&self, opcodes: &[u64]) -> Vec<u32> {
        let mut codes: Vec<u32> = self.cards.values().filter(|c| crate::announce::is_declarable(c, opcodes)).map(|c| c.code).collect();
        codes.sort_unstable();
        codes
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}
