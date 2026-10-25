//! The profile of an archetype (plan `docs/plans/sinergie-archetipi.md`
//! §3.3 (c)): its members, what its monsters are, what its effects accept
//! from other cards, what it gives others (bodies their effects accept,
//! materials of their Extra Deck monsters), the events it makes and
//! answers, the limits it puts on Summons. Everything from the database,
//! the anatomy and the readings kept in the store; nothing by name.

use std::collections::{BTreeMap, HashMap, HashSet};

use ygo_engine::consts::{card_type, event};
use ygo_engine::reading::MaterialReading;
use ygo_engine::{attribute_name, race_name, read_anatomy, CardDb, EffectKind, Engine};

use super::accept::{moving_effects, played, MOVES};
use super::read::{Place, Pool, Reading};
use super::rule::in_archetype;

/// `CATEGORY_TOGRAVE`, `CATEGORY_HANDES` (discard), `CATEGORY_DECKDES` (mill).
const TO_GRAVE: u64 = 0x20;
const DISCARD: u64 = 0x80;
const MILL: u64 = 0x40;

#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub setcode: u16,
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
    /// Of its monsters: Attribute, Type and Level → how many.
    pub attributes: BTreeMap<u32, usize>,
    pub races: BTreeMap<u32, usize>,
    pub levels: BTreeMap<u32, usize>,
    /// Members read, members with effects that move cards, and how many of
    /// those effects a reading played.
    pub read: usize,
    pub moving: (usize, usize),
    /// Cards out of the archetype its effects accept, and of them by
    /// archetype (base set code; 0 for no archetype).
    pub accepts_out: usize,
    pub accepts_by: BTreeMap<u16, usize>,
    /// Cards out of the archetype whose effects accept its members, by
    /// archetype of the card accepting.
    pub accepted_by: BTreeMap<u16, usize>,
    /// Extra Deck monsters out of the archetype that take its monsters as a
    /// material, by archetype; and its own Extra Deck monsters that take
    /// monsters out of it.
    pub material_of: BTreeMap<u16, usize>,
    pub material_of_count: usize,
    pub own_extra_open: usize,
    /// Its effects that send to the GY, banish, discard or mill, and that
    /// Special Summon (events another archetype may answer).
    pub makes: BTreeMap<&'static str, usize>,
    /// Its triggers, by the event they answer.
    pub answers: BTreeMap<&'static str, usize>,
    /// Members whose effects limit your Special Summons, and how many
    /// Extra Deck monsters out of the archetype the strictest still allows.
    pub limited: Vec<u32>,
    pub strictest_allows_out: Option<usize>,
    /// Monsters of Level 4 or lower whose Normal Summon triggers an effect
    /// that moves cards: the starters that want the Normal Summon.
    pub normal_starters: Vec<u32>,
}

fn base(setcode: u16) -> u16 {
    setcode & 0x0fff
}

/// The base archetypes of a card other than `setcode`'s, or 0 for none.
fn others(db: &CardDb, code: u32, setcode: u16) -> Vec<u16> {
    let Some(c) = db.get(code) else { return Vec::new() };
    let mut out: Vec<u16> = c.setcodes.iter().map(|&s| base(s)).filter(|&s| s != base(setcode)).collect();
    out.sort_unstable();
    out.dedup();
    if out.is_empty() && c.setcodes.is_empty() {
        out.push(0);
    }
    out
}

pub fn profile(
    engine: &Engine,
    pool: &Pool,
    setcode: u16,
    readings: &BTreeMap<(u32, Place), Reading>,
    materials: &BTreeMap<u32, MaterialReading>,
) -> Profile {
    let db = engine.cards();
    let member = |code: u32| db.get(code).is_some_and(|c| in_archetype(c, setcode));
    let mut p = Profile { setcode, ..Profile::default() };
    p.main = pool.main.iter().copied().filter(|&c| member(c)).collect();
    p.extra = pool.extra.iter().copied().filter(|&c| member(c)).collect();
    let members: HashSet<u32> = p.main.iter().chain(&p.extra).copied().collect();
    for &code in p.main.iter().chain(&p.extra) {
        let Some(c) = db.get(code).filter(|c| c.type_ & card_type::MONSTER != 0) else { continue };
        *p.attributes.entry(c.attribute).or_default() += 1;
        *p.races.entry(c.race as u32).or_default() += 1;
        *p.levels.entry(c.level & 0xff).or_default() += 1;
    }

    let codes: Vec<u32> = p.main.iter().chain(&p.extra).copied().collect();
    let mut accepted_out: HashSet<u32> = HashSet::new();
    let mut strictest: Option<usize> = None;
    for a in read_anatomy(engine, &codes) {
        let mine: Vec<&Reading> = Place::ALL.iter().filter_map(|&pl| readings.get(&(a.code, pl))).collect();
        p.read += usize::from(!mine.is_empty());
        let activated: HashSet<u64> = mine.iter().flat_map(|r| r.activated.iter().copied()).collect();
        let moving = moving_effects(&a);
        if !moving.is_empty() {
            p.moving.0 += moving.len();
            p.moving.1 += played(&a, &moving, &activated).iter().filter(|&&d| d).count();
        }
        for r in &mine {
            for o in &r.offers {
                accepted_out.extend(o.cards.iter().copied().filter(|c| !members.contains(c)));
            }
            for l in r.limits.iter().filter(|l| l.code == 22) {
                if !p.limited.contains(&a.code) {
                    p.limited.push(a.code);
                }
                let forbidden: HashSet<u32> = l.forbidden.iter().copied().collect();
                let allows = pool.extra.iter().filter(|c| !members.contains(c) && !forbidden.contains(c)).count();
                strictest = Some(strictest.map_or(allows, |s: usize| s.min(allows)));
            }
        }
        for e in a.effects.iter().filter(|e| e.kind.activated()) {
            for (bit, name) in [
                (TO_GRAVE, "send to the GY"),
                (0x4, "banish"),
                (DISCARD, "discard"),
                (MILL, "mill"),
                (ygo_engine::consts::category::SPECIAL_SUMMON, "Special Summon"),
            ] {
                if e.category & bit != 0 {
                    *p.makes.entry(name).or_default() += 1;
                }
            }
            if matches!(e.kind, EffectKind::Trigger { .. }) {
                if let Some(name) = event::name(e.code) {
                    *p.answers.entry(name).or_default() += 1;
                }
            }
        }
        let low_monster =
            db.get(a.code).is_some_and(|c| c.type_ & card_type::MONSTER != 0 && !c.is_extra_deck_monster() && (c.level & 0xff) <= 4);
        let on_normal_summon = a
            .effects
            .iter()
            .any(|e| matches!(e.kind, EffectKind::Trigger { .. }) && e.code == event::SUMMON_SUCCESS && e.category & MOVES != 0);
        if low_monster && on_normal_summon {
            p.normal_starters.push(a.code);
        }
    }
    p.strictest_allows_out = strictest;
    p.accepts_out = accepted_out.len();
    for &c in &accepted_out {
        for a in others(db, c, setcode) {
            *p.accepts_by.entry(a).or_default() += 1;
        }
    }
    // Who else accepts its members: every reading kept, of cards out of it.
    let mut by: HashMap<u32, Vec<u16>> = HashMap::new();
    for ((code, _), r) in readings {
        if members.contains(code) || !r.offers.iter().any(|o| o.cards.iter().any(|c| members.contains(c))) {
            continue;
        }
        by.entry(*code).or_insert_with(|| others(db, *code, setcode));
    }
    for archetypes in by.values() {
        for &a in archetypes {
            *p.accepted_by.entry(a).or_default() += 1;
        }
    }
    let monsters: HashSet<u32> =
        members.iter().copied().filter(|&c| db.get(c).is_some_and(|d| d.type_ & card_type::MONSTER != 0)).collect();
    for (code, m) in materials {
        let takes_member = m.slots.iter().any(|s| s.cards.iter().any(|c| monsters.contains(c)));
        if !members.contains(code) && takes_member {
            p.material_of_count += 1;
            for a in others(db, *code, setcode) {
                *p.material_of.entry(a).or_default() += 1;
            }
        }
        if members.contains(code) && m.slots.iter().any(|s| s.cards.iter().any(|c| !members.contains(c))) {
            p.own_extra_open += 1;
        }
    }
    p
}

impl Profile {
    pub fn describe(&self, db: &CardDb, names: &HashMap<u16, String>) -> String {
        let name = |s: u16| match s {
            0 => "no archetype".to_owned(),
            _ => names.get(&s).cloned().unwrap_or_else(|| format!("0x{s:x}")),
        };
        let top = |m: &BTreeMap<u16, usize>, n: usize| -> String {
            let mut v: Vec<(&u16, &usize)> = m.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let parts: Vec<String> = v.iter().take(n).map(|(s, k)| format!("{} {k}", name(**s))).collect();
            if parts.is_empty() {
                "-".to_owned()
            } else {
                parts.join(", ")
            }
        };
        let bits = |m: &BTreeMap<u32, usize>, label: &dyn Fn(u32) -> String| -> String {
            let mut v: Vec<(&u32, &usize)> = m.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1));
            v.iter().take(4).map(|(k, n)| format!("{} {n}", label(**k))).collect::<Vec<_>>().join(", ")
        };
        let one_bit = |v: u32, f: fn(u32) -> String| if v.is_power_of_two() { f(v.trailing_zeros()) } else { format!("0x{v:x}") };
        let counts = |m: &BTreeMap<&'static str, usize>| m.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", ");
        let cards = |c: &[u32]| -> String {
            let mut v: Vec<String> = c.iter().take(5).map(|&c| db.name(c)).collect();
            if c.len() > 5 {
                v.push(format!("and {} more", c.len() - 5));
            }
            if v.is_empty() {
                "none".to_owned()
            } else {
                v.join(", ")
            }
        };
        let mut lines = vec![
            format!("{} (0x{:x}): {} Main Deck cards, {} Extra Deck monsters", name(self.setcode), self.setcode, self.main.len(), self.extra.len()),
            format!(
                "  monsters: {}; {}; Levels {}",
                bits(&self.attributes, &|a| one_bit(a, attribute_name)),
                bits(&self.races, &|r| one_bit(r, race_name)),
                bits(&self.levels, &|l| l.to_string())
            ),
            format!(
                "  read: {} of {} members; effects that move cards played {} of {}",
                self.read,
                self.main.len() + self.extra.len(),
                self.moving.1,
                self.moving.0
            ),
            format!("  its effects accept {} cards out of it; by archetype: {}", self.accepts_out, top(&self.accepts_by, 8)),
            format!("  cards out of it whose effects accept its members, by archetype: {}", top(&self.accepted_by, 8)),
            format!(
                "  material of {} Extra Deck monsters out of it; by archetype: {}; its own Extra Deck monsters open to other monsters: {} of {}",
                self.material_of_count,
                top(&self.material_of, 6),
                self.own_extra_open,
                self.extra.len()
            ),
            format!("  its effects: {}", counts(&self.makes)),
            format!("  its triggers answer: {}", counts(&self.answers)),
            format!(
                "  limits on Special Summons: {}{}",
                cards(&self.limited),
                self.strictest_allows_out.map_or(String::new(), |n| format!("; the strictest still allows {n} Extra Deck monsters out of it"))
            ),
            format!("  starters that want the Normal Summon: {}", cards(&self.normal_starters)),
        ];
        lines.push(String::new());
        lines.join("\n")
    }
}
