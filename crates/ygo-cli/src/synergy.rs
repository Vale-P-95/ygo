//! The commands of the synergy plan's data (`docs/plans/sinergie-archetipi.md`
//! §3.3, phase F1): `synergy-read` reads the pool into `data/synergy/`,
//! `accepts` shows what a card accepts, `archetype` an archetype's profile.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{Context, Result};
use ygo_engine::consts::location;
use ygo_engine::{read_anatomy, Engine};
use ygo_solver::synergy::{self, accept, Coverage, GraveSample, Place, Pool, ReadBudget, Reading, Scene, Store};

use crate::load_engine;

/// The card of a name or passcode.
fn card_code(engine: &Engine, name: &str) -> Result<u32> {
    let db = engine.cards();
    let code = match name.parse::<u32>() {
        Ok(code) => code,
        Err(_) => db.find_by_name(name).with_context(|| format!("no card named {name}"))?.code,
    };
    Ok(db.canonical_code(code))
}

/// Every card of the pool once, sorted: the order of the store's bitmaps.
fn pool_order(pool: &Pool) -> Vec<u32> {
    let mut order: Vec<u32> = pool.main.iter().chain(&pool.extra).copied().collect();
    order.sort_unstable();
    order
}

pub struct ReadArgs<'a> {
    pub cards: &'a [String],
    pub archetypes: &'a [String],
    pub sample: Option<usize>,
    pub places: &'a [String],
    pub threads: Option<usize>,
    pub seed: u64,
    pub out: &'a Path,
    pub fresh: bool,
}

pub fn read(dir: &Path, args: ReadArgs) -> Result<()> {
    raise_lua_limit();
    let engine = load_engine(dir)?;
    let db = engine.cards();
    let threads = args.threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    std::fs::create_dir_all(args.out)?;
    let pool = Pool::new(&engine);
    let order = pool_order(&pool);
    let store = Store::open(dir, order.clone())?;
    if args.fresh {
        std::fs::remove_file(store.readings_path()).ok();
        std::fs::remove_file(store.materials_path()).ok();
    }
    let places: Vec<Place> = if args.places.is_empty() {
        Place::ALL.to_vec()
    } else {
        args.places.iter().map(|p| Place::from_name(p).with_context(|| format!("no place {p} (hand, field, GY)"))).collect::<Result<_>>()?
    };

    // The materials of the whole Extra Deck, in one duel holding the pool.
    let started = Instant::now();
    let materials_time = if store.materials_path().is_file() {
        None
    } else {
        let materials = ygo_engine::reading::read_materials(&engine, &pool.main, &pool.extra, &pool.extra)?;
        store.write_materials(&materials)?;
        Some(started.elapsed().as_secs_f64())
    };

    // The cards: those asked for, a sample of the cards that move cards,
    // or the whole pool.
    let anatomy_all: Option<Vec<ygo_engine::CardAnatomy>> = Some(read_anatomy(&engine, &order));
    let codes: Vec<u32> = if !args.cards.is_empty() || !args.archetypes.is_empty() {
        let names = synergy::archetype_names(&engine);
        let mut codes: Vec<u32> = args.cards.iter().map(|n| card_code(&engine, n)).collect::<Result<_>>()?;
        for a in args.archetypes {
            let setcode = archetype_code(&names, a)?;
            codes.extend(order.iter().copied().filter(|&c| db.get(c).is_some_and(|d| synergy::rule::in_archetype(d, setcode))));
        }
        codes.sort_unstable();
        codes.dedup();
        codes
    } else if let (Some(n), Some(all)) = (args.sample, anatomy_all.as_ref()) {
        let movers: Vec<u32> = all.iter().filter(|a| !accept::moving_effects(a).is_empty()).map(|a| a.code).collect();
        let step = (movers.len() / n.max(1)).max(1);
        movers.iter().step_by(step).take(n).copied().collect()
    } else {
        order.clone()
    };

    let kept = store.load_readings();
    let anatomy: HashMap<u32, ygo_engine::CardAnatomy> = match anatomy_all {
        Some(all) => all.into_iter().map(|a| (a.code, a)).collect(),
        None => read_anatomy(&engine, &order).into_iter().map(|a| (a.code, a)).collect(),
    };
    let graves = grave_sample(&engine, &pool, &anatomy);
    let deck = pool.deck();
    let budget = ReadBudget::default();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let readings_made = AtomicUsize::new(0);
    let failed = Mutex::new(Vec::new());
    let read_started = Instant::now();
    // Card by card: from the hand, then from the field and the GY when an
    // effect that moves cards is still not played (accept::wanted).
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, codes.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(&code) = codes.get(i) else { break };
                let extra = db.get(code).is_some_and(|c| c.is_extra_deck_monster());
                let grave = graves.of(db, code);
                let events = anatomy.get(&code).map(synergy::read::trigger_events).unwrap_or_default();
                let (hand, field) = graves.companions(db, code);
                let mut activated: HashSet<u64> = HashSet::new();
                for place in Place::ALL {
                    if !places.contains(&place) || (extra && place == Place::Hand) {
                        continue;
                    }
                    if place != Place::Hand {
                        let (field, gy) = anatomy.get(&code).map_or((false, false), |a| accept::wanted(a, extra, &activated));
                        if (place == Place::Field && !field) || (place == Place::Grave && !gy) {
                            continue;
                        }
                    }
                    if let Some(r) = kept.get(&(code, place)) {
                        activated.extend(r.activated.iter().copied());
                        continue;
                    }
                    let set = anatomy.get(&code).is_some_and(|a| accept::activation_wanted(a, &activated));
                    let scene = Scene {
                        place,
                        grave: if place == Place::Hand { &[] } else { &grave },
                        events: &events,
                        hand: &hand,
                        field: &field,
                        set,
                    };
                    let reading = synergy::read_card_in(&engine, &deck, code, &HashSet::new(), budget, args.seed, scene);
                    activated.extend(reading.activated.iter().copied());
                    readings_made.fetch_add(1, Ordering::Relaxed);
                    if let Err(e) = store.append_readings(&[(place, &reading)]) {
                        if let Ok(mut f) = failed.lock() {
                            f.push(format!("cannot keep {code}: {e}"));
                        }
                    }
                }
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n.is_multiple_of(200) || n == codes.len() {
                    let secs = read_started.elapsed().as_secs_f64();
                    println!(
                        "{n} of {} cards in {secs:.0} s, about {:.0} min left",
                        codes.len(),
                        secs / n as f64 * (codes.len() - n) as f64 / 60.0
                    );
                }
            });
        }
    });
    let read_time = read_started.elapsed().as_secs_f64();
    for f in failed.into_inner().unwrap_or_default() {
        eprintln!("{f}");
    }

    let (summary, untried) = summarize(
        &engine,
        &pool,
        &graves,
        &anatomy,
        &store,
        &codes,
        &places,
        threads,
        read_time,
        readings_made.into_inner(),
        materials_time,
    );
    print!("{summary}");
    std::fs::write(args.out.join("summary.txt"), &summary)?;
    std::fs::write(args.out.join("untried.tsv"), untried)?;
    Ok(())
}

/// The GY of the readings from the field and the GY ([`Pool::grave_sample`]).
fn grave_sample(engine: &Engine, pool: &Pool, anatomy: &HashMap<u32, ygo_engine::CardAnatomy>) -> GraveSample {
    // Cards with an effect that applies by itself, or that works from the
    // GY, make the engine check it at every step (some look through the
    // whole Deck each time): left out.
    let quiet = |a: &ygo_engine::CardAnatomy| {
        a.error.is_none()
            && a.effects.iter().all(|e| {
                !matches!(e.kind, ygo_engine::EffectKind::Continuous | ygo_engine::EffectKind::Granted)
                    && e.range & u32::from(location::GRAVE) == 0
            })
    };
    let effects: HashMap<u32, usize> = anatomy.values().filter(|a| quiet(a)).map(|a| (a.code, a.effects.len())).collect();
    pool.grave_sample(engine, &effects)
}

/// Reading duels whose GY holds a sample of the pool need more entries into
/// Lua than a duel of a real deck (the limit stops runaway scripts).
fn raise_lua_limit() {
    if std::env::var_os("DUELFORGE_LUA_LIMIT").is_none() {
        std::env::set_var("DUELFORGE_LUA_LIMIT", "5000000");
    }
}

/// The summary of the readings of `codes`: coverage of the effects that move
/// cards, the rules read, the limits, the materials, errors and time; and
/// the effects no reading played, one line each (`untried.tsv`).
#[allow(clippy::too_many_arguments)]
fn summarize(
    engine: &Engine,
    pool: &Pool,
    graves: &GraveSample,
    anatomy: &HashMap<u32, ygo_engine::CardAnatomy>,
    store: &Store,
    codes: &[u32],
    places: &[Place],
    threads: usize,
    read_time: f64,
    jobs: usize,
    materials_time: Option<f64>,
) -> (String, String) {
    let db = engine.cards();
    let readings = store.load_readings();
    let mut coverage = Coverage::default();
    let mut by_place: BTreeMap<Place, (usize, f64, usize)> = BTreeMap::new();
    let (mut offers, mut exact, mut loose, mut named, mut no_universe) = (0, 0, 0, 0, 0);
    let mut limits = 0;
    let mut errors: BTreeMap<String, usize> = BTreeMap::new();
    let mut untried_script: BTreeMap<&str, usize> = BTreeMap::new();
    let mut untried = 0;
    let mut untried_tsv = String::from("card\tcode\teffect\tkind\tevent\trange\tasks\tscript filters\n");
    for a in codes.iter().filter_map(|c| anatomy.get(c)) {
        let mut activated = HashSet::new();
        let mut mine = Vec::new();
        for &p in places {
            let Some(r) = readings.get(&(a.code, p)) else { continue };
            let entry = by_place.entry(p).or_default();
            entry.0 += 1;
            entry.1 += r.time.as_secs_f64();
            entry.2 += usize::from(!r.offers.is_empty());
            activated.extend(r.activated.iter().copied());
            limits += usize::from(!r.limits.is_empty());
            if let Some(e) = &r.error {
                // The kind of error, not the card: the first words.
                let kind: String = e.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
                *errors.entry(kind).or_default() += 1;
            }
            mine.push((p, r));
        }
        coverage.add(a, &activated);
        let script = engine.scripts().read(&format!("c{}.lua", a.code)).map(|s| String::from_utf8_lossy(&s).into_owned());
        let acc = synergy::accepts(db, pool, &graves.of(db, a.code), a, &mine, script.as_deref());
        for list in acc.effects.values() {
            for x in list {
                offers += 1;
                match &x.rule {
                    None => no_universe += 1,
                    Some(r) if matches!(r.conditions.first(), Some(synergy::Condition::Named(_))) => named += 1,
                    Some(r) if r.exact() => exact += 1,
                    Some(_) => loose += 1,
                }
            }
        }
        for (description, hints) in &acc.untried {
            if let Some(e) = a.effects.iter().find(|e| e.description == *description && e.kind.activated()) {
                untried_tsv.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{:#x}\t{}{}{}\t{}\n",
                    db.name(a.code),
                    a.code,
                    db.describe(*description),
                    e.kind.name(),
                    ygo_engine::consts::event::name(e.code).map_or_else(|| e.code.to_string(), str::to_owned),
                    e.range,
                    if e.has_condition { "condition " } else { "" },
                    if e.has_cost { "cost " } else { "" },
                    if e.targets() { "target" } else { "" },
                    hints.join(",")
                ));
            }
            untried += 1;
            for h in hints {
                *untried_script.entry(h).or_default() += 1;
            }
        }
    }
    let (moving, played) = coverage.effects();
    let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
    let kinds: Vec<String> = coverage.by_kind.iter().map(|(k, (n, d))| format!("{k} {d}/{n}")).collect();
    let missed: Vec<String> = coverage.missed_with.iter().map(|(k, n)| format!("{k} {n}")).collect();
    let places_line: Vec<String> =
        by_place.iter().map(|(p, (n, t, o))| format!("{} {n} ({o} with offers, {:.2} s each)", p.name(), t / (*n).max(1) as f64)).collect();
    let script_line: Vec<String> = untried_script.iter().map(|(k, n)| format!("{k} {n}")).collect();
    let materials = store.load_materials();
    let with_slots = materials.values().filter(|m| !m.slots.is_empty()).count();
    let mut mat_kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for m in materials.values() {
        let mut kinds: Vec<&str> = m.slots.iter().map(|s| s.kind.as_str()).collect();
        kinds.dedup();
        for k in kinds {
            *mat_kinds.entry(k).or_default() += 1;
        }
    }
    let mat_errors = materials.values().flat_map(|m| &m.slots).filter(|s| s.errors > 0).count();
    let mat_kinds: Vec<String> = mat_kinds.iter().map(|(k, n)| format!("{k} {n}")).collect();
    let mut errors: Vec<(String, usize)> = errors.into_iter().collect();
    errors.sort_by_key(|e| std::cmp::Reverse(e.1));
    let mut lines = vec![
        format!(
            "store: {} ({} cards in the pool; in the GY of the readings from the field and the GY {} cards, and up to {} of the card's archetypes)",
            store.dir.display(),
            store.order.len(),
            graves.base.len(),
            synergy::read::GRAVE_MEMBERS
        ),
        format!(
            "cards: {} read in {} as needed ({jobs} readings now, {read_time:.0} s on {threads} threads); by place: {}",
            codes.len(),
            places.iter().map(|p| p.name()).collect::<Vec<_>>().join(", "),
            places_line.join("; ")
        ),
        format!(
            "effects that move cards (search, add to the hand, Special Summon, send to the GY): {moving}, played by a reading {played} ({:.1}%); by kind: {}",
            pct(played, moving),
            kinds.join(", ")
        ),
        format!(
            "cards with such effects: {}, all played {} ({:.1}%); not played, asking first for: {}",
            coverage.cards,
            coverage.cards_all,
            pct(coverage.cards_all, coverage.cards),
            missed.join(", ")
        ),
        format!(
            "from the script, not tried: {untried} effects; their scripts' filters look at: {}",
            if script_line.is_empty() { "-".to_owned() } else { script_line.join(", ") }
        ),
        format!(
            "offers: {offers}; rules read: exact {exact}, with exceptions {loose}, by name {named}; no rule (hand or field) {no_universe}"
        ),
        format!("readings with a limit on Summons: {limits}"),
        format!(
            "materials: {} Extra Deck monsters, {with_slots} with materials read{}; by procedure: {}; slots with filter errors: {mat_errors}",
            pool.extra.len(),
            materials_time.map_or(String::new(), |t| format!(" in {t:.0} s")),
            mat_kinds.join(", ")
        ),
        format!("errors: {}", errors.iter().map(|(_, n)| n).sum::<usize>()),
    ];
    lines.extend(errors.iter().take(5).map(|(e, n)| format!("  {n}× {e}")));
    (lines.join("\n") + "\n", untried_tsv)
}

/// Where a choice took cards from, for people.
fn from(location: u8) -> &'static str {
    match location {
        location::DECK => "Deck",
        location::GRAVE => "GY",
        location::EXTRA => "Extra Deck",
        location::REMOVED => "banished",
        location::HAND => "hand",
        _ => "field",
    }
}

/// An effect for people: its text, or "the activation" for a Spell or Trap
/// activation without one.
fn effect_name(db: &ygo_engine::CardDb, description: u64) -> String {
    let text = db.describe(description);
    if description == 0 || text.trim().is_empty() {
        "the activation (no text)".to_owned()
    } else {
        text
    }
}

/// What a card accepts, from the store (reading it first if it is not there).
pub fn accepts(dir: &Path, card: &str, seed: u64) -> Result<()> {
    raise_lua_limit();
    let engine = load_engine(dir)?;
    let db = engine.cards();
    let pool = Pool::new(&engine);
    let order = pool_order(&pool);
    let store = Store::open(dir, order.clone())?;
    let code = card_code(&engine, card)?;
    let mut readings = store.load_readings();
    let extra = db.get(code).is_some_and(|c| c.is_extra_deck_monster());
    let all: HashMap<u32, ygo_engine::CardAnatomy> = read_anatomy(&engine, &order).into_iter().map(|a| (a.code, a)).collect();
    let graves = grave_sample(&engine, &pool, &all);
    let grave = graves.of(db, code);
    let (hand, field) = graves.companions(db, code);
    let events = all.get(&code).map(synergy::read::trigger_events).unwrap_or_default();
    let mut activated: HashSet<u64> = HashSet::new();
    let mut places = Vec::new();
    for p in Place::ALL {
        if extra && p == Place::Hand {
            continue;
        }
        if p != Place::Hand {
            let (field, gy) = all.get(&code).map_or((false, false), |a| accept::wanted(a, extra, &activated));
            if (p == Place::Field && !field) || (p == Place::Grave && !gy) {
                continue;
            }
        }
        let set = all.get(&code).is_some_and(|a| accept::activation_wanted(a, &activated));
        let r = match readings.entry((code, p)) {
            std::collections::btree_map::Entry::Occupied(kept) => kept.into_mut(),
            std::collections::btree_map::Entry::Vacant(slot) => {
                let scene = Scene {
                    place: p,
                    grave: if p == Place::Hand { &[] } else { &grave },
                    events: &events,
                    hand: &hand,
                    field: &field,
                    set,
                };
                let r = synergy::read_card_in(&engine, &pool.deck(), code, &HashSet::new(), ReadBudget::default(), seed, scene);
                store.append_readings(&[(p, &r)])?;
                slot.insert(r)
            }
        };
        activated.extend(r.activated.iter().copied());
        places.push(p);
    }
    let anatomy = all.get(&code).context("no anatomy")?;
    let mine: Vec<(Place, &Reading)> = places.iter().filter_map(|&p| readings.get(&(code, p)).map(|r| (p, r))).collect();
    let script = engine.scripts().read(&format!("c{code}.lua")).map(|s| String::from_utf8_lossy(&s).into_owned());
    let acc = synergy::accepts(db, &pool, &grave, anatomy, &mine, script.as_deref());
    let names = synergy::archetype_names(&engine);
    println!("{} ({code})", db.name(code));
    for (place, r) in &mine {
        let played: Vec<String> = r.activated.iter().map(|&d| effect_name(db, d)).collect();
        println!(
            "  from the {}: {} turns, {} summons, effects played: {}{}",
            place.name(),
            r.walks,
            r.summons,
            if played.is_empty() { "none".to_owned() } else { played.join(" | ") },
            r.error.as_ref().map_or(String::new(), |e| format!("; error: {e}"))
        );
    }
    for (effect, list) in &acc.effects {
        println!("  effect {}:", effect_name(db, *effect));
        for x in list {
            let o = &x.offer;
            let rule = x.rule.as_ref().map_or("no rule".to_owned(), |r| r.describe(db, &names));
            let sample: Vec<String> = o.cards.iter().take(3).map(|&c| db.name(c)).collect();
            println!(
                "    [{}] {} from the {}: {} cards ({}{}) — {rule}",
                x.place.name(),
                o.purpose(),
                from(o.location),
                o.cards.len(),
                sample.join(", "),
                if o.cards.len() > 3 { ", ..." } else { "" }
            );
        }
    }
    for (place, r) in &mine {
        for l in &r.limits {
            let what = if l.code == 20 { "Normal Summon" } else { "Special Summon" };
            let allowed = pool.extra.len() - l.forbidden.iter().filter(|c| pool.extra.binary_search(c).is_ok()).count();
            println!(
                "  limit [{}]: no {what} of {} cards; {allowed} of {} Extra Deck monsters still allowed",
                place.name(),
                l.forbidden.len(),
                pool.extra.len()
            );
        }
    }
    for (effect, hints) in &acc.untried {
        println!(
            "  effect {}: not played by any reading; from the script, not tried: filters by {}",
            effect_name(db, *effect),
            if hints.is_empty() { "nothing known".to_owned() } else { hints.join(", ") }
        );
    }
    if extra {
        if let Some(m) = store.load_materials().get(&code) {
            for s in &m.slots {
                let rule = synergy::learn(db, &s.cards, &pool_order(&pool));
                println!(
                    "  material {} ({}, {}–{}): {} cards — {}{}",
                    s.slot,
                    s.kind,
                    s.min,
                    s.max,
                    s.cards.len(),
                    rule.describe(db, &names),
                    if s.checked { "; the procedure checks more" } else { "" }
                );
            }
        }
    }
    Ok(())
}

/// An archetype by name ("Blue-Eyes", "blue eyes") or set code ("0xdd").
fn archetype_code(names: &HashMap<u16, String>, name: &str) -> Result<u16> {
    if let Some(hex) = name.strip_prefix("0x") {
        return u16::from_str_radix(hex, 16).context("a set code in hexadecimal");
    }
    let simple = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
    let wanted = simple(name);
    let mut found: Vec<(&u16, &String)> = names.iter().filter(|(_, n)| simple(n) == wanted).collect();
    found.sort();
    found.first().map(|(c, _)| **c).with_context(|| format!("no archetype named {name}"))
}

/// The profile of an archetype (plan §3.3 (c)).
pub fn archetype(dir: &Path, name: &str) -> Result<()> {
    let engine = load_engine(dir)?;
    let db = engine.cards();
    let pool = Pool::new(&engine);
    let store = Store::open(dir, pool_order(&pool))?;
    let names = synergy::archetype_names(&engine);
    let setcode = archetype_code(&names, name)?;
    let readings = store.load_readings();
    let materials = store.load_materials();
    let profile = synergy::profile(&engine, &pool, setcode, &readings, &materials);
    print!("{}", profile.describe(db, &names));
    Ok(())
}
