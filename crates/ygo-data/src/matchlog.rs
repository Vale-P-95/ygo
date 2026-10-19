//! The match log: matches played with a deck, each with a snapshot of the
//! list it was played with, so that versions of the deck can be compared.
//! Also reads and writes the log as CSV.

use std::collections::BTreeMap;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::deck::Deck;
use crate::userdb::UserStore;
use crate::Error;

/// One logged match.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchEntry {
    #[serde(default)]
    pub id: Option<i64>,
    pub deck_id: Option<i64>,
    /// Name of the deck when the match was logged.
    #[serde(default)]
    pub deck_name: String,
    /// Fingerprint of the list played (see [`DeckList::version`]).
    #[serde(default)]
    pub version: String,
    /// Local date and time, "YYYY-MM-DD HH:MM"; now when empty.
    #[serde(default)]
    pub played_at: String,
    #[serde(default)]
    pub opponent: String,
    /// Went first in game 1 (`None` when not recorded).
    pub first: Option<bool>,
    /// Each game in order, "W", "L" or "D": "WLW".
    #[serde(default)]
    pub games: String,
    /// The match, "W", "L" or "D"; worked out from the games when empty.
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// The cards a match was played with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckList {
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
    pub side: Vec<u32>,
}

impl DeckList {
    pub fn of(deck: &Deck) -> Self {
        Self { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() }
    }

    pub fn is_empty(&self) -> bool {
        self.main.is_empty() && self.extra.is_empty() && self.side.is_empty()
    }

    /// Short fingerprint of the list: the same cards give the same version,
    /// whatever their order. FNV-1a, so it never changes between builds.
    pub fn version(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for zone in [&self.main, &self.extra, &self.side] {
            let mut ids = zone.clone();
            ids.sort_unstable();
            for byte in ids.iter().flat_map(|id| id.to_le_bytes()).chain([0xff; 4]) {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
        format!("{:08x}", (hash >> 32) as u32 ^ hash as u32)
    }

    /// Cards added (positive) and taken out (negative) going from `self` to `newer`, by zone.
    pub fn changes(&self, newer: &DeckList) -> Vec<CardChange> {
        let count = |ids: &[u32]| {
            let mut out = BTreeMap::<u32, i32>::new();
            ids.iter().for_each(|id| *out.entry(*id).or_default() += 1);
            out
        };
        let mut out = Vec::new();
        for (zone, old, new) in [("main", &self.main, &newer.main), ("extra", &self.extra, &newer.extra), ("side", &self.side, &newer.side)]
        {
            let (old, new) = (count(old), count(new));
            let ids: std::collections::BTreeSet<u32> = old.keys().chain(new.keys()).copied().collect();
            for id in ids {
                let delta = new.get(&id).copied().unwrap_or(0) - old.get(&id).copied().unwrap_or(0);
                if delta != 0 {
                    out.push(CardChange { id, zone: zone.to_owned(), delta });
                }
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CardChange {
    pub id: u32,
    pub zone: String,
    pub delta: i32,
}

/// The outcome of a match from its games: more wins than losses is a win.
pub fn match_result(games: &str) -> char {
    let wins = games.chars().filter(|&c| c == 'W').count();
    let losses = games.chars().filter(|&c| c == 'L').count();
    match wins.cmp(&losses) {
        std::cmp::Ordering::Greater => 'W',
        std::cmp::Ordering::Less => 'L',
        std::cmp::Ordering::Equal => 'D',
    }
}

/// "W", "L" or "D" from the ways a result can be written (English or Italian).
fn outcome(s: &str) -> Option<char> {
    match s.trim().to_lowercase().as_str() {
        "w" | "win" | "won" | "v" | "vittoria" | "vinta" | "1" => Some('W'),
        "l" | "loss" | "lost" | "s" | "sconfitta" | "persa" | "0" => Some('L'),
        "d" | "draw" | "p" | "pareggio" | "pari" => Some('D'),
        _ => None,
    }
}

/// Games as "WLW", from "WLW", "VSV", "W L W" or "W-L-W".
fn parse_games(s: &str) -> Result<String, Error> {
    let s = s.trim();
    let parts: Vec<&str> = if s.contains([' ', '-', '/', ',']) {
        s.split([' ', '-', '/', ',']).filter(|p| !p.is_empty()).collect()
    } else {
        s.split("").filter(|p| !p.is_empty()).collect()
    };
    let games: Option<String> = parts.iter().map(|p| outcome(p)).collect();
    match games {
        Some(g) if g.len() <= 3 => Ok(g),
        _ => Err(Error::Data(format!("game non validi: «{s}» (usa W, L, D)"))),
    }
}

/// Checks a match and fills in its result.
fn normalized(entry: &MatchEntry) -> Result<MatchEntry, Error> {
    let games = parse_games(&entry.games)?;
    let result = match entry.result.trim() {
        "" if games.is_empty() => return Err(Error::Data("indica il risultato della partita".into())),
        "" => match_result(&games),
        r => outcome(r).ok_or_else(|| Error::Data(format!("risultato non valido: «{r}»")))?,
    };
    let mut tags: Vec<String> = Vec::new();
    for t in entry.tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        if !tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            tags.push(t.to_owned());
        }
    }
    Ok(MatchEntry {
        games,
        result: result.to_string(),
        opponent: entry.opponent.trim().to_owned(),
        played_at: entry.played_at.trim().to_owned(),
        notes: entry.notes.trim().to_owned(),
        tags,
        ..entry.clone()
    })
}

const COLUMNS: &str = "id, deck_id, deck_name, deck_hash, played_at, opponent, first, games, result, tags, notes";

fn row(r: &rusqlite::Row) -> rusqlite::Result<MatchEntry> {
    Ok(MatchEntry {
        id: r.get(0)?,
        deck_id: r.get(1)?,
        deck_name: r.get(2)?,
        version: r.get(3)?,
        played_at: r.get(4)?,
        opponent: r.get(5)?,
        first: r.get(6)?,
        games: r.get(7)?,
        result: r.get(8)?,
        tags: serde_json::from_str(&r.get::<_, String>(9)?).unwrap_or_default(),
        notes: r.get(10)?,
    })
}

impl UserStore {
    /// Logs a match played with the deck as it is saved now.
    pub fn log_match(&self, entry: &MatchEntry) -> Result<MatchEntry, Error> {
        let deck_id = entry.deck_id.ok_or_else(|| Error::Data("scegli il deck della partita".into()))?;
        let deck = self.get(deck_id)?.ok_or_else(|| Error::Data(format!("il deck {deck_id} non esiste più")))?;
        let list = DeckList::of(&deck);
        self.insert_match(&MatchEntry { deck_name: deck.name, version: list.version(), ..entry.clone() }, Some(&list))
    }

    /// Adds matches read from a CSV file to a deck. Matches without a
    /// version count as played with the deck as it is now.
    pub fn import_matches(&self, deck_id: i64, entries: &[MatchEntry]) -> Result<usize, Error> {
        let deck = self.get(deck_id)?.ok_or_else(|| Error::Data(format!("il deck {deck_id} non esiste più")))?;
        let list = DeckList::of(&deck);
        let current = list.version();
        for e in entries {
            let version = if e.version.trim().is_empty() { current.clone() } else { e.version.trim().to_owned() };
            let snapshot = (version == current).then_some(&list);
            self.insert_match(
                &MatchEntry { id: None, deck_id: Some(deck_id), deck_name: deck.name.clone(), version, ..e.clone() },
                snapshot,
            )?;
        }
        Ok(entries.len())
    }

    fn insert_match(&self, entry: &MatchEntry, list: Option<&DeckList>) -> Result<MatchEntry, Error> {
        let e = normalized(entry)?;
        let snapshot = list.map(serde_json::to_string).transpose()?.unwrap_or_else(|| "{}".into());
        self.conn.execute(
            "INSERT INTO matches (deck_id, deck_name, deck_hash, deck_snapshot, played_at, opponent, first, games, result, tags, notes)
             VALUES (?1, ?2, ?3, ?4, COALESCE(NULLIF(?5, ''), strftime('%Y-%m-%d %H:%M', 'now', 'localtime')), ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                e.deck_id,
                e.deck_name,
                e.version,
                snapshot,
                e.played_at,
                e.opponent,
                e.first,
                e.games,
                e.result,
                serde_json::to_string(&e.tags)?,
                e.notes
            ],
        )?;
        self.get_match(self.conn.last_insert_rowid())
    }

    /// Changes what was recorded about a match; the deck and its version stay.
    pub fn update_match(&self, entry: &MatchEntry) -> Result<MatchEntry, Error> {
        let id = entry.id.ok_or_else(|| Error::Data("partita senza id".into()))?;
        let e = normalized(entry)?;
        let changed = self.conn.execute(
            "UPDATE matches SET played_at = COALESCE(NULLIF(?2, ''), played_at), opponent = ?3, first = ?4, games = ?5, result = ?6, tags = ?7, notes = ?8 WHERE id = ?1",
            params![id, e.played_at, e.opponent, e.first, e.games, e.result, serde_json::to_string(&e.tags)?, e.notes],
        )?;
        if changed == 0 {
            return Err(Error::Data(format!("la partita {id} non esiste più")));
        }
        self.get_match(id)
    }

    fn get_match(&self, id: i64) -> Result<MatchEntry, Error> {
        Ok(self.conn.query_row(&format!("SELECT {COLUMNS} FROM matches WHERE id = ?"), [id], row)?)
    }

    pub fn delete_match(&self, id: i64) -> Result<(), Error> {
        self.conn.execute("DELETE FROM matches WHERE id = ?", [id])?;
        Ok(())
    }

    /// Matches of a deck, or of every deck, oldest first.
    pub fn matches(&self, deck_id: Option<i64>) -> Result<Vec<MatchEntry>, Error> {
        let mut stmt =
            self.conn.prepare(&format!("SELECT {COLUMNS} FROM matches WHERE ?1 IS NULL OR deck_id = ?1 ORDER BY played_at, id"))?;
        let rows = stmt.query_map([deck_id], row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The versions a deck was played with, oldest first, with their lists
    /// (empty when unknown, e.g. imported from CSV).
    pub fn match_versions(&self, deck_id: i64) -> Result<Vec<(String, DeckList)>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT deck_hash, MAX(CASE deck_snapshot WHEN '{}' THEN '' ELSE deck_snapshot END), MIN(played_at) AS since FROM matches WHERE deck_id = ? GROUP BY deck_hash ORDER BY since, MIN(id)",
        )?;
        let rows =
            stmt.query_map([deck_id], |r| Ok((r.get::<_, String>(0)?, serde_json::from_str(&r.get::<_, String>(1)?).unwrap_or_default())))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Opponents and tags used so far, most frequent first, to suggest while logging.
    pub fn match_suggestions(&self) -> Result<(Vec<String>, Vec<String>), Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT opponent FROM matches WHERE opponent != '' GROUP BY opponent ORDER BY COUNT(*) DESC, MAX(id) DESC")?;
        let opponents = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        let mut counts = BTreeMap::<String, usize>::new();
        let mut stmt = self.conn.prepare("SELECT tags FROM matches")?;
        for tags in stmt.query_map([], |r| r.get::<_, String>(0))? {
            for t in serde_json::from_str::<Vec<String>>(&tags?).unwrap_or_default() {
                *counts.entry(t).or_default() += 1;
            }
        }
        let mut tags: Vec<(String, usize)> = counts.into_iter().collect();
        tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        Ok((opponents, tags.into_iter().map(|(t, _)| t).collect()))
    }
}

const CSV_HEAD: [&str; 9] = ["data", "deck", "versione", "avversario", "turno", "game", "risultato", "tag", "note"];

fn csv_field(s: &str) -> String {
    if s.contains([',', ';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// The log as CSV (comma separated, one match per line).
pub fn matches_to_csv(entries: &[MatchEntry]) -> String {
    let mut out = CSV_HEAD.join(",") + "\n";
    for e in entries {
        let turn = match e.first {
            Some(true) => "primo",
            Some(false) => "secondo",
            None => "",
        };
        let fields = [&e.played_at, &e.deck_name, &e.version, &e.opponent, turn, &e.games, &e.result, &e.tags.join(";"), &e.notes];
        out += &fields.map(csv_field).join(",");
        out.push('\n');
    }
    out
}

/// Splits CSV text into records, with quoted fields.
fn csv_records(text: &str, sep: char) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let (mut record, mut field, mut quoted) = (Vec::new(), String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => record.push(std::mem::take(&mut field)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            c => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records.retain(|r| r.iter().any(|f| !f.trim().is_empty()));
    records
}

/// Reads matches from CSV written by DuelForge or by hand: columns are
/// found by name (Italian or English), separated by commas, semicolons or tabs.
pub fn matches_from_csv(text: &str) -> Result<Vec<MatchEntry>, Error> {
    let text = text.trim_start_matches('\u{feff}');
    let head = text.lines().next().unwrap_or_default();
    let sep = [';', '\t', ','].into_iter().max_by_key(|&s| head.matches(s).count()).unwrap_or(',');
    let mut records = csv_records(text, sep).into_iter();
    let head: Vec<String> =
        records.next().ok_or_else(|| Error::Data("il file CSV è vuoto".into()))?.iter().map(|h| h.trim().to_lowercase()).collect();
    let col = |names: &[&str]| head.iter().position(|h| names.contains(&h.as_str()));
    let date = col(&["data", "date", "played_at"]);
    let version = col(&["versione", "version"]);
    let opponent = col(&["avversario", "opponent", "matchup"]);
    let turn = col(&["turno", "turn", "first", "primo"]);
    let games = col(&["game", "games", "partite"]);
    let result = col(&["risultato", "result", "esito"]);
    let tags = col(&["tag", "tags"]);
    let notes = col(&["note", "notes"]);
    if games.is_none() && result.is_none() {
        return Err(Error::Data("nel CSV manca la colonna «risultato» o «game»".into()));
    }
    let mut out = Vec::new();
    for (line, r) in records.enumerate() {
        let get = |c: Option<usize>| c.and_then(|c| r.get(c)).map(|s| s.trim().to_owned()).unwrap_or_default();
        let first = match get(turn).to_lowercase().as_str() {
            "primo" | "first" | "1st" | "1" | "sì" | "si" | "yes" | "true" => Some(true),
            "secondo" | "second" | "2nd" | "2" | "no" | "false" => Some(false),
            _ => None,
        };
        let entry = MatchEntry {
            played_at: get(date),
            version: get(version),
            opponent: get(opponent),
            first,
            games: get(games),
            result: get(result),
            tags: get(tags).split([';', ',', '|']).map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect(),
            notes: get(notes),
            ..Default::default()
        };
        out.push(normalized(&entry).map_err(|e| Error::Data(format!("riga {}: {e}", line + 2)))?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(deck_id: i64, opponent: &str, first: bool, games: &str) -> MatchEntry {
        MatchEntry { deck_id: Some(deck_id), opponent: opponent.into(), first: Some(first), games: games.into(), ..Default::default() }
    }

    #[test]
    fn versions_ignore_order_and_see_every_zone() {
        let a = DeckList { main: vec![1, 2, 2], extra: vec![9], side: vec![] };
        let b = DeckList { main: vec![2, 1, 2], extra: vec![9], side: vec![] };
        assert_eq!(a.version(), b.version());
        assert_eq!(a.version(), "06cb21a9", "the fingerprint never changes between builds");
        let moved = DeckList { main: vec![1, 2], extra: vec![9], side: vec![2] };
        assert_ne!(a.version(), moved.version(), "moving a card to the side is a new version");
        assert_eq!(
            a.changes(&moved),
            [CardChange { id: 2, zone: "main".into(), delta: -1 }, CardChange { id: 2, zone: "side".into(), delta: 1 }]
        );
    }

    #[test]
    fn results_from_games() {
        assert_eq!((match_result("WLW"), match_result("LL"), match_result("WL"), match_result("D")), ('W', 'L', 'D', 'D'));
        assert_eq!(parse_games("V S V").unwrap(), "WLW");
        assert_eq!(parse_games("w-l").unwrap(), "WL");
        assert!(parse_games("WLWW").is_err());
        assert!(parse_games("X").is_err());
    }

    #[test]
    fn log_versions_and_suggestions() {
        let store = UserStore::open_in_memory().unwrap();
        let deck = store.save(&Deck { name: "Invoked".into(), main: vec![1, 2, 2], ..Default::default() }).unwrap();
        let id = deck.id.unwrap();
        let m = store
            .log_match(&MatchEntry {
                played_at: "2026-10-01 20:00".into(),
                tags: vec!["ranked".into(), "Ranked".into()],
                ..entry(id, " Snake-Eye ", true, "W")
            })
            .unwrap();
        assert_eq!((m.deck_name.as_str(), m.result.as_str(), m.opponent.as_str()), ("Invoked", "W", "Snake-Eye"));
        assert_eq!(m.tags, ["ranked"]);
        let v1 = m.version.clone();

        store.save(&Deck { main: vec![1, 2, 3], ..deck }).unwrap();
        let m2 = store.log_match(&MatchEntry { played_at: "2026-10-02 20:00".into(), ..entry(id, "Tenpai", false, "LWL") }).unwrap();
        assert_eq!(m2.result, "L");
        assert_ne!(m2.version, v1);
        let now = store.log_match(&entry(id, "Snake-Eye", false, "WW")).unwrap();
        assert_eq!(now.played_at.len(), 16, "defaults to now: {}", now.played_at);
        assert!(store.log_match(&entry(id, "x", true, "")).is_err(), "a result is needed");
        assert!(store.log_match(&MatchEntry { deck_id: None, ..entry(id, "x", true, "W") }).is_err());

        let versions = store.match_versions(id).unwrap();
        assert_eq!(versions.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(), [v1.as_str(), m2.version.as_str()]);
        assert_eq!(
            versions[0].1.changes(&versions[1].1),
            [CardChange { id: 2, zone: "main".into(), delta: -1 }, CardChange { id: 3, zone: "main".into(), delta: 1 }]
        );

        let (opponents, tags) = store.match_suggestions().unwrap();
        assert_eq!(
            (opponents.as_slice(), tags.as_slice()),
            (&["Snake-Eye".to_owned(), "Tenpai".to_owned()][..], &["ranked".to_owned()][..])
        );

        let edited =
            store.update_match(&MatchEntry { opponent: "Yubel".into(), games: "WLW".into(), result: String::new(), ..m2.clone() }).unwrap();
        assert_eq!((edited.opponent.as_str(), edited.result.as_str(), edited.version.as_str()), ("Yubel", "W", m2.version.as_str()));
        store.delete_match(m.id.unwrap()).unwrap();
        assert_eq!(store.matches(Some(id)).unwrap().len(), 2);
        assert_eq!(store.matches(None).unwrap().len(), 2);
        assert_eq!(store.matches(Some(999)).unwrap().len(), 0);
    }

    #[test]
    fn csv_round_trip() {
        let store = UserStore::open_in_memory().unwrap();
        let id = store.save(&Deck { name: "Invoked, \"Siracusa\"".into(), main: vec![1], ..Default::default() }).unwrap().id.unwrap();
        store
            .log_match(&MatchEntry {
                played_at: "2026-10-01 20:00".into(),
                notes: "brick; poi Ash".into(),
                tags: vec!["evento".into(), "bo3".into()],
                ..entry(id, "Snake-Eye", true, "WLW")
            })
            .unwrap();
        store.log_match(&MatchEntry { played_at: "2026-10-02 21:00".into(), first: None, ..entry(id, "", false, "L") }).unwrap();
        let csv = matches_to_csv(&store.matches(Some(id)).unwrap());
        assert!(
            csv.starts_with("data,deck,versione,avversario,turno,game,risultato,tag,note\n2026-10-01 20:00,\"Invoked, \"\"Siracusa\"\"\",")
        );
        let read = matches_from_csv(&csv).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(
            (read[0].games.as_str(), read[0].result.as_str(), read[0].first, read[0].notes.as_str()),
            ("WLW", "W", Some(true), "brick; poi Ash")
        );
        assert_eq!(read[0].tags, ["evento", "bo3"]);
        assert_eq!((read[1].first, read[1].result.as_str()), (None, "L"));

        let other = store.save(&Deck { name: "Altro".into(), main: vec![5], ..Default::default() }).unwrap().id.unwrap();
        assert_eq!(store.import_matches(other, &read).unwrap(), 2);
        let imported = store.matches(Some(other)).unwrap();
        assert_eq!(imported[0].deck_name, "Altro");
        assert_eq!(imported[0].version, read[0].version, "the version in the file is kept");

        // A hand-made sheet: semicolons, Italian words, no version.
        let sheet = "Data;Avversario;Turno;Risultato\n2026-10-03;Yubel;secondo;vittoria\n\n2026-10-04;Tenpai;primo;S\n";
        let read = matches_from_csv(sheet).unwrap();
        assert_eq!(
            read.iter().map(|e| (e.opponent.as_str(), e.first, e.result.as_str(), e.games.as_str())).collect::<Vec<_>>(),
            [("Yubel", Some(false), "W", ""), ("Tenpai", Some(true), "L", "")]
        );
        store.import_matches(other, &read).unwrap();
        let versions = store.match_versions(other).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[1].1.main, [5], "matches without a version get the current list");
        assert!(matches_from_csv("data;avversario\n2026-10-03;Yubel\n").is_err());
        assert!(matches_from_csv("risultato\nboh\n").unwrap_err().to_string().contains("riga 2"));
    }
}
