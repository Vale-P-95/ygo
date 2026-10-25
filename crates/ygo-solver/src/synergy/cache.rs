//! What the reading deck read, kept in `data/synergy/<key>/` (plan
//! `docs/plans/sinergie-archetipi.md` §3.3 (e)): never in the repository,
//! read again after `fetch-data`. The key holds the version of the scripts,
//! of the card database, of the pool and of the reading, so a reading of
//! other data is never taken for this one.
//!
//! Plain text, one line per record, appended card after card: a reading cut
//! short goes on from where it was. Sets of cards are a list of codes, or a
//! bitmap over the pool (`pool.txt`) when they are many.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use ygo_engine::reading::{decode_bitmap, MaterialReading, MaterialSlot, PlayerLimit};

use super::read::{Offer, Place, Reading};

/// Raised whenever what a reading records or how it plays changes.
pub const READER_VERSION: u32 = 1;

/// Sets of more cards than this are kept as a bitmap.
const LIST_AT_MOST: usize = 400;

/// The folder of the readings of this data, and the pool its bitmaps
/// follow (every card of the pool once, sorted).
#[derive(Debug, Clone)]
pub struct Store {
    pub dir: PathBuf,
    pub order: Vec<u32>,
}

/// A short hash (FNV-1a, 64 bits).
fn fnv(bytes: &[u8], mut hash: u64) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// The key of the data in `data_dir` with this pool: the commit of the
/// scripts (or the size and time of their base files), the size and time
/// of each card database, the pool and the version of the reading.
pub fn key(data_dir: &Path, order: &[u32]) -> String {
    let mut hash = fnv(&READER_VERSION.to_le_bytes(), 0xcbf2_9ce4_8422_2325);
    let head = std::fs::read_to_string(data_dir.join("scripts/.git/HEAD")).unwrap_or_default();
    let commit = match head.trim().strip_prefix("ref: ") {
        Some(r) => std::fs::read_to_string(data_dir.join("scripts/.git").join(r)).unwrap_or_default(),
        None => head,
    };
    hash = fnv(commit.trim().as_bytes(), hash);
    let stamp = |path: &Path, hash: u64| -> u64 {
        let Ok(meta) = std::fs::metadata(path) else { return hash };
        let time = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
        fnv(&[meta.len().to_le_bytes(), time.to_le_bytes()].concat(), hash)
    };
    if commit.trim().is_empty() {
        for base in ["constant.lua", "utility.lua"] {
            hash = stamp(&data_dir.join("scripts").join(base), hash);
        }
    }
    let mut cdbs: Vec<PathBuf> = std::fs::read_dir(data_dir.join("cdb"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "cdb"))
        .collect();
    cdbs.sort();
    for cdb in cdbs {
        hash = stamp(&cdb, fnv(cdb.file_name().map(|n| n.as_encoded_bytes()).unwrap_or_default(), hash));
    }
    for code in order {
        hash = fnv(&code.to_le_bytes(), hash);
    }
    format!("{hash:016x}")
}

impl Store {
    /// The store of `data_dir` for the pool `order` (sorted, once each),
    /// with `pool.txt` written if it is new.
    pub fn open(data_dir: &Path, order: Vec<u32>) -> std::io::Result<Store> {
        let dir = data_dir.join("synergy").join(key(data_dir, &order));
        std::fs::create_dir_all(&dir)?;
        let pool = dir.join("pool.txt");
        if !pool.is_file() {
            let text: Vec<String> = order.iter().map(u32::to_string).collect();
            std::fs::write(&pool, text.join("\n") + "\n")?;
        }
        Ok(Store { dir, order })
    }

    pub fn readings_path(&self) -> PathBuf {
        self.dir.join("readings.txt")
    }

    pub fn materials_path(&self) -> PathBuf {
        self.dir.join("materials.txt")
    }

    fn encode(&self, cards: &[u32]) -> String {
        if cards.is_empty() {
            return "-".to_owned();
        }
        if cards.len() <= LIST_AT_MOST {
            return format!("l:{}", cards.iter().map(u32::to_string).collect::<Vec<_>>().join(","));
        }
        let mut nibbles = vec![0u8; self.order.len().div_ceil(4)];
        for c in cards {
            if let Ok(i) = self.order.binary_search(c) {
                nibbles[i / 4] |= 1 << (i % 4);
            }
        }
        let digits: String = nibbles.iter().map(|&n| char::from_digit(u32::from(n), 16).unwrap_or('0')).collect();
        format!("b:{digits}")
    }

    fn decode(&self, text: &str) -> Option<Vec<u32>> {
        if text == "-" {
            return Some(Vec::new());
        }
        if let Some(list) = text.strip_prefix("l:") {
            return list.split(',').map(|c| c.parse().ok()).collect();
        }
        decode_bitmap(text.strip_prefix("b:")?, self.order.len(), &self.order)
    }

    /// The lines of one card's reading in `place`.
    pub fn reading_lines(&self, place: Place, r: &Reading) -> String {
        let activated =
            if r.activated.is_empty() { "-".to_owned() } else { r.activated.iter().map(u64::to_string).collect::<Vec<_>>().join(",") };
        let p = place.name();
        let mut out = format!(
            "card {} {p} {} {} {} {} {} {activated} {}\n",
            r.code,
            r.walks,
            r.actions,
            r.summons,
            r.decisions,
            r.time.as_millis(),
            r.error.as_deref().unwrap_or("-").replace('\n', " ")
        );
        for o in &r.offers {
            out.push_str(&format!("offer {} {p} {} {} {} {}\n", r.code, o.effect, o.hint, o.location, self.encode(&o.cards)));
        }
        for l in &r.limits {
            out.push_str(&format!("limit {} {p} {} {} {} {}\n", r.code, l.owner, l.code, l.errors, self.encode(&l.forbidden)));
        }
        out
    }

    /// Appends readings to `readings.txt`.
    pub fn append_readings(&self, readings: &[(Place, &Reading)]) -> std::io::Result<()> {
        let text: String = readings.iter().map(|(p, r)| self.reading_lines(*p, r)).collect();
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(self.readings_path())?;
        file.write_all(text.as_bytes())
    }

    /// Every reading kept, by card and place. A line that cannot be read
    /// (a reading cut in the middle of a line) is skipped, with the card.
    pub fn load_readings(&self) -> BTreeMap<(u32, Place), Reading> {
        let mut out: BTreeMap<(u32, Place), Reading> = BTreeMap::new();
        let Ok(text) = std::fs::read_to_string(self.readings_path()) else { return out };
        for line in text.lines() {
            let fields: Vec<&str> = line.splitn(10, ' ').collect();
            let (Some(code), Some(place)) = (fields.get(1).and_then(|c| c.parse::<u32>().ok()), fields.get(2).and_then(|p| place_of(p)))
            else {
                continue;
            };
            let n = |i: usize| fields.get(i).and_then(|f| f.parse::<u64>().ok());
            match fields[0] {
                "card" if fields.len() == 10 => {
                    let (Some(walks), Some(actions), Some(summons), Some(decisions), Some(ms)) = (n(3), n(4), n(5), n(6), n(7)) else {
                        continue;
                    };
                    let activated =
                        if fields[8] == "-" { Vec::new() } else { fields[8].split(',').filter_map(|d| d.parse().ok()).collect() };
                    let error = (fields[9] != "-").then(|| fields[9].to_owned());
                    out.insert(
                        (code, place),
                        Reading {
                            code,
                            walks: walks as usize,
                            actions: actions as usize,
                            summons: summons as usize,
                            decisions: decisions as usize,
                            time: std::time::Duration::from_millis(ms),
                            activated,
                            error,
                            ..Reading::default()
                        },
                    );
                }
                "offer" if fields.len() == 7 => {
                    let (Some(effect), Some(hint), Some(location), Some(cards)) = (n(3), n(4), n(5), self.decode(fields[6])) else {
                        continue;
                    };
                    if let Some(r) = out.get_mut(&(code, place)) {
                        r.offers.push(Offer { effect, hint, location: location as u8, cards });
                    }
                }
                "limit" if fields.len() == 7 => {
                    let (Some(owner), Some(limit), Some(errors), Some(forbidden)) = (n(3), n(4), n(5), self.decode(fields[6])) else {
                        continue;
                    };
                    if let Some(r) = out.get_mut(&(code, place)) {
                        r.limits.push(PlayerLimit { owner: owner as u32, code: limit as u32, errors: errors as u32, forbidden });
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Writes `materials.txt` anew.
    pub fn write_materials(&self, materials: &[MaterialReading]) -> std::io::Result<()> {
        let mut text = String::new();
        for m in materials {
            if m.slots.is_empty() {
                text.push_str(&format!("nomat {} {}\n", m.code, m.error.as_deref().unwrap_or("-").replace('\n', " ")));
            }
            for s in &m.slots {
                text.push_str(&format!(
                    "mat {} {} {} {} {} {} {} {} {}\n",
                    m.code,
                    s.procedure,
                    s.slot,
                    s.kind.replace(' ', "_"),
                    s.min,
                    s.max,
                    u8::from(s.checked),
                    s.errors,
                    self.encode(&s.cards)
                ));
            }
        }
        std::fs::write(self.materials_path(), text)
    }

    /// The materials kept, by card (empty if not read yet).
    pub fn load_materials(&self) -> BTreeMap<u32, MaterialReading> {
        let mut out: BTreeMap<u32, MaterialReading> = BTreeMap::new();
        let Ok(text) = std::fs::read_to_string(self.materials_path()) else { return out };
        for line in text.lines() {
            let fields: Vec<&str> = line.splitn(10, ' ').collect();
            let Some(code) = fields.get(1).and_then(|c| c.parse::<u32>().ok()) else { continue };
            let entry = out.entry(code).or_insert_with(|| MaterialReading { code, ..MaterialReading::default() });
            match fields[0] {
                "nomat" => entry.error = fields.get(2).filter(|e| **e != "-").map(|e| e.to_string()),
                "mat" if fields.len() == 10 => {
                    let n = |i: usize| fields[i].parse::<u32>().ok();
                    let (Some(procedure), Some(slot), Some(min), Some(max), Some(checked), Some(errors), Some(cards)) =
                        (n(2), n(3), n(5), n(6), n(7), n(8), self.decode(fields[9]))
                    else {
                        continue;
                    };
                    entry.slots.push(MaterialSlot {
                        kind: fields[4].replace('_', " "),
                        procedure,
                        slot,
                        min,
                        max,
                        checked: checked != 0,
                        errors,
                        cards,
                    });
                }
                _ => {}
            }
        }
        out
    }
}

fn place_of(name: &str) -> Option<Place> {
    Place::from_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readings_and_materials_come_back_as_written() {
        let dir = std::env::temp_dir().join(format!("df-synergy-cache-{}", std::process::id()));
        let order: Vec<u32> = (1..=1000).collect();
        let store = Store { dir: dir.clone(), order: order.clone() };
        std::fs::create_dir_all(&dir).unwrap();
        let reading = Reading {
            code: 7,
            walks: 2,
            actions: 2,
            summons: 1,
            decisions: 30,
            activated: vec![112, 113],
            offers: vec![
                Offer { effect: 112, hint: 506, location: 1, cards: vec![3, 9] },
                // Many cards: a bitmap.
                Offer { effect: 113, hint: 509, location: 16, cards: (1..=900).collect() },
            ],
            limits: vec![PlayerLimit { owner: 7, code: 22, errors: 0, forbidden: (500..=1000).collect() }],
            error: Some("a Lua error here".to_owned()),
            time: std::time::Duration::from_millis(1500),
            ..Reading::default()
        };
        store.append_readings(&[(Place::Grave, &reading)]).unwrap();
        let back = store.load_readings();
        let got = &back[&(7, Place::Grave)];
        assert_eq!((got.walks, got.actions, got.summons, got.decisions), (2, 2, 1, 30));
        assert_eq!(got.activated, reading.activated);
        assert_eq!(got.offers, reading.offers);
        assert_eq!(got.limits, reading.limits);
        assert_eq!(got.error, reading.error);

        let slot = MaterialSlot {
            kind: "xyz on a monster".to_owned(),
            procedure: 2,
            slot: 1,
            min: 1,
            max: 1,
            checked: true,
            errors: 0,
            cards: order[..450].to_vec(),
        };
        store
            .write_materials(&[
                MaterialReading { code: 5, slots: vec![slot.clone()], error: None },
                MaterialReading { code: 6, ..Default::default() },
            ])
            .unwrap();
        let mats = store.load_materials();
        assert_eq!(mats[&5].slots, vec![slot]);
        assert!(mats[&6].slots.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
