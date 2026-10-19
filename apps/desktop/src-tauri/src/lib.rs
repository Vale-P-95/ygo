use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::http::{Response, StatusCode};
use tauri::{Emitter, Manager, State};
use ygo_data::sync::{self, Progress};
use ygo_data::{
    banlist, deck, matchlog, Banlist, Card, CardChange, CardStore, Chip, Deck, DeckBox, DeckList, DeckSummary, Facets, Format, ImageCache,
    ImageSize, Legality, MatchEntry, Recipe, SavedSearch, Tag, UserStore,
};
use ygo_engine::{Engine, ScriptStore};

/// Shared by every command. The card database connection is not `Sync`, so
/// it sits behind a mutex; it is `None` until the card list is downloaded.
struct AppState {
    data_dir: PathBuf,
    store: Mutex<Option<CardStore>>,
    images: ImageCache,
    client: reqwest::Client,
    updating: tokio::sync::Mutex<()>,
    /// The user's decks (`user.db`), kept outside the re-downloadable data.
    user: Mutex<UserStore>,
    /// Set to stop the lab's running simulation.
    sim_cancel: AtomicBool,
}

impl AppState {
    fn cards_db(&self) -> PathBuf {
        self.data_dir.join("cards.db")
    }

    /// A newer Master Duel list downloaded by `update_data` (the app also has one built in).
    fn md_banlist_path(&self) -> PathBuf {
        self.data_dir.join("banlists").join("md.json")
    }

    fn open_store(&self) {
        let path = self.cards_db();
        let mut store = path.is_file().then(|| CardStore::open(&path).ok()).flatten();
        if let Some(store) = store.as_mut() {
            if let Err(e) = store.set_banlist(&Banlist::newest_md(&self.md_banlist_path())) {
                eprintln!("Master Duel banlist not applied: {e}");
            }
        }
        *self.store.lock().expect("store lock") = store;
    }

    fn with_store<T>(&self, f: impl FnOnce(&CardStore) -> Result<T, ygo_data::Error>) -> Result<T, String> {
        let guard = self.store.lock().map_err(|e| e.to_string())?;
        let store = guard.as_ref().ok_or("Database delle carte non ancora scaricato")?;
        f(store).map_err(|e| e.to_string())
    }

    fn with_user<T>(&self, f: impl FnOnce(&UserStore) -> Result<T, ygo_data::Error>) -> Result<T, String> {
        let guard = self.user.lock().map_err(|e| e.to_string())?;
        f(&guard).map_err(|e| e.to_string())
    }
}

/// Game data lives in the app data folder, or in `DUELFORGE_DATA` when set
/// (handy during development).
fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    std::env::var_os("DUELFORGE_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from(".")).join("data"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardsStatus {
    version: String,
    count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineDataStatus {
    api_version: String,
    scripts: Option<usize>,
    scripts_version: Option<String>,
    databases: usize,
    cdb_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DataStatus {
    data_dir: String,
    cards: Option<CardsStatus>,
    engine: EngineDataStatus,
    images_cached: usize,
    /// Set by `update_data` when the card list updated but the engine data
    /// could not be downloaded (the card browser works without it).
    engine_error: Option<String>,
    /// Effective date of the Master Duel Forbidden & Limited list in use.
    banlist_md: Option<String>,
}

fn status(state: &AppState) -> DataStatus {
    let cards = state.with_store(|s| Ok(CardsStatus { version: s.version().unwrap_or_default(), count: s.count()? })).ok();
    let (major, minor) = Engine::api_version();
    let scripts = ScriptStore::load_dir(&state.data_dir.join("scripts")).ok().map(|s| s.len());
    let databases = std::fs::read_dir(state.data_dir.join("cdb"))
        .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "cdb")).count())
        .unwrap_or(0);
    DataStatus {
        data_dir: state.data_dir.display().to_string(),
        cards,
        engine: EngineDataStatus {
            api_version: format!("{major}.{minor}"),
            scripts,
            scripts_version: sync::installed_engine_version(&state.data_dir, "scripts"),
            databases,
            cdb_version: sync::installed_engine_version(&state.data_dir, "cdb"),
        },
        images_cached: state.images.cached_count(ImageSize::Small),
        engine_error: None,
        banlist_md: state.with_store(|s| Ok(s.banlist_date("md"))).ok().flatten(),
    }
}

#[tauri::command(async)]
fn data_status(state: State<'_, AppState>) -> DataStatus {
    status(&state)
}

/// Downloads what is missing or outdated, reporting `data-progress` events.
#[tauri::command]
async fn update_data(app: tauri::AppHandle, state: State<'_, AppState>, force: bool) -> Result<DataStatus, String> {
    let _busy = state.updating.try_lock().map_err(|_| "Aggiornamento già in corso")?;
    std::fs::create_dir_all(&state.data_dir).map_err(|e| e.to_string())?;
    let report = |p: Progress| {
        let _ = app.emit("data-progress", p);
    };
    let current = if force { None } else { state.with_store(|s| Ok(s.version())).ok().flatten() };
    let new_db = state.data_dir.join("cards.db.new");
    if sync::update_cards(&state.client, current.as_deref(), &new_db, &report).await.map_err(|e| e.to_string())?.is_some() {
        // Close the old database before replacing its file.
        *state.store.lock().map_err(|e| e.to_string())? = None;
        sync::install_file(&new_db, &state.cards_db()).map_err(|e| e.to_string())?;
        state.open_store();
    }
    // A newer Master Duel list, if the repository has one; the built-in list stays otherwise.
    let current_md = Banlist::newest_md(&state.md_banlist_path()).effective;
    match banlist::update_md(&state.client, &state.md_banlist_path(), &current_md).await {
        Ok(true) => state.open_store(),
        Ok(false) => {}
        Err(e) => eprintln!("Master Duel banlist not updated: {e}"),
    }
    let engine = sync::update_engine_data(&state.client, &state.data_dir, &report).await;
    Ok(DataStatus { engine_error: engine.err().map(|e| e.to_string()), ..status(&state) })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    ids: Vec<u32>,
    chips: Vec<Chip>,
    errors: Vec<String>,
}

#[tauri::command(async)]
fn search_cards(state: State<'_, AppState>, query: String) -> Result<SearchResult, String> {
    let mut parsed = ygo_data::parse_query(&query);
    if parsed.query.tags.contains(&Tag::Favorite) {
        parsed.query.favorites = state.with_user(|u| u.favorites())?;
    }
    let ids = state.with_store(|s| s.search(&parsed.query))?;
    Ok(SearchResult { ids, chips: parsed.chips, errors: parsed.errors })
}

#[tauri::command(async)]
fn get_cards(state: State<'_, AppState>, ids: Vec<u32>) -> Result<Vec<Card>, String> {
    state.with_store(|s| s.get_many(&ids))
}

#[tauri::command(async)]
fn card_facets(state: State<'_, AppState>) -> Result<Facets, String> {
    state.with_store(|s| s.facets())
}

#[tauri::command(async)]
fn list_favorites(state: State<'_, AppState>) -> Result<Vec<u32>, String> {
    state.with_user(|u| u.favorites())
}

#[tauri::command(async)]
fn set_favorite(state: State<'_, AppState>, id: u32, favorite: bool) -> Result<(), String> {
    state.with_user(|u| u.set_favorite(id, favorite))
}

#[tauri::command(async)]
fn list_saved_searches(state: State<'_, AppState>) -> Result<Vec<SavedSearch>, String> {
    state.with_user(|u| u.saved_searches())
}

#[tauri::command(async)]
fn save_search(state: State<'_, AppState>, name: String, query: String) -> Result<Vec<SavedSearch>, String> {
    state.with_user(|u| {
        u.save_search(&name, &query)?;
        u.saved_searches()
    })
}

#[tauri::command(async)]
fn delete_saved_search(state: State<'_, AppState>, name: String) -> Result<Vec<SavedSearch>, String> {
    state.with_user(|u| {
        u.delete_search(&name)?;
        u.saved_searches()
    })
}

/// Lab: the overview of a Main Deck with its roles.
#[tauri::command(async)]
fn lab_overview(main: Vec<u32>, roles: BTreeMap<String, Vec<u32>>) -> ygo_stats::Overview {
    ygo_stats::overview(&ygo_stats::DeckModel::from_ids(&main, &roles))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LabResult {
    /// `None` when the exact calculation is impossible (see `error`).
    p: Option<f64>,
    error: Option<String>,
}

/// Lab: exact probability that a hand of `hand` cards satisfies `cond`.
#[tauri::command(async)]
fn lab_probability(main: Vec<u32>, roles: BTreeMap<String, Vec<u32>>, cond: ygo_stats::Cond, hand: u32) -> LabResult {
    match ygo_stats::probability(&ygo_stats::DeckModel::from_ids(&main, &roles), &cond, hand) {
        Ok(p) => LabResult { p: Some(p), error: None },
        Err(e) => LabResult { p: None, error: Some(e.to_string()) },
    }
}

/// Lab: Monte Carlo simulation. Sends `lab-sim-progress` checkpoints while it runs.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
fn lab_simulate(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    main: Vec<u32>,
    roles: BTreeMap<String, Vec<u32>>,
    cond: ygo_stats::Cond,
    hand: u32,
    rules: Vec<ygo_stats::CardRule>,
    hands: u64,
    seed: u64,
) -> ygo_stats::SimResult {
    state.sim_cancel.store(false, Ordering::Relaxed);
    let config =
        ygo_stats::SimConfig { deck: ygo_stats::DeckModel::from_ids(&main, &roles), cond, hand, rules, hands: hands.min(20_000_000), seed };
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    ygo_stats::simulate(&config, threads, &state.sim_cancel, |cp| {
        let _ = app.emit("lab-sim-progress", cp);
    })
}

#[tauri::command]
fn lab_cancel_simulation(state: State<'_, AppState>) {
    state.sim_cancel.store(true, Ordering::Relaxed);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Optimizer {
    sensitivity: Option<ygo_stats::Sensitivity>,
    size: Vec<Option<f64>>,
    heatmap: Option<ygo_stats::Heatmap>,
    error: Option<String>,
}

/// Lab: value of each copy, deck size curve and the two-role map, in one call.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
fn lab_analyze(
    main: Vec<u32>,
    roles: BTreeMap<String, Vec<u32>>,
    cond: ygo_stats::Cond,
    hand: u32,
    role_keys: Vec<String>,
    limits: BTreeMap<u32, u32>,
    role_a: Option<String>,
    role_b: Option<String>,
) -> Optimizer {
    let deck = ygo_stats::DeckModel::from_ids(&main, &roles);
    let sensitivity = ygo_stats::sensitivity(&deck, &cond, hand, &role_keys, &limits);
    let heatmap = match (role_a, role_b) {
        (Some(a), Some(b)) if a != b => Some(ygo_stats::heatmap(&deck, &cond, hand, &a, &b, 4)),
        _ => None,
    };
    Optimizer {
        error: sensitivity.as_ref().err().map(|e| e.to_string()),
        sensitivity: sensitivity.ok(),
        size: ygo_stats::size_curve(&deck, &cond, hand, 5),
        heatmap,
    }
}

/// Lab: the optimizer. Cards of the Maybe board may enter the deck.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
fn lab_optimize(
    main: Vec<u32>,
    maybe: Vec<u32>,
    roles: BTreeMap<String, Vec<u32>>,
    limits: BTreeMap<u32, u32>,
    locked: Vec<u32>,
    cond: ygo_stats::Cond,
    weight_first: f64,
    min_size: u32,
    max_size: u32,
    seed: u64,
) -> Result<ygo_stats::OptimizeResult, String> {
    let deck = ygo_stats::DeckModel::from_ids(&main, &roles);
    let mut candidates = ygo_stats::DeckModel::from_ids(&maybe, &roles).cards;
    candidates.retain(|c| !deck.cards.iter().any(|d| d.id == c.id));
    let config =
        ygo_stats::OptimizeConfig { deck, candidates, limits, locked, cond, weight_first, min_size, max_size, iterations: 2500, seed };
    ygo_stats::optimize(&config).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn set_deck_roles(state: State<'_, AppState>, id: i64, roles: BTreeMap<String, Vec<u32>>) -> Result<(), String> {
    state.with_user(|u| u.set_roles(id, &roles))
}

#[tauri::command(async)]
fn list_recipes(state: State<'_, AppState>, deck_id: i64) -> Result<Vec<Recipe>, String> {
    state.with_user(|u| u.recipes(deck_id))
}

#[tauri::command(async)]
fn save_recipe(state: State<'_, AppState>, deck_id: Option<i64>, name: String, cond: serde_json::Value) -> Result<Recipe, String> {
    state.with_user(|u| u.save_recipe(deck_id, &name, &cond))
}

#[tauri::command(async)]
fn delete_recipe(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.with_user(|u| u.delete_recipe(id))
}

/// A version of the deck in the match log.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogVersion {
    version: String,
    /// "v1", "v2"... in the order they were first played.
    name: String,
    /// Changes from the version before; `None` when either list is unknown.
    changes: Option<Vec<CardChange>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchLog {
    /// Oldest first.
    matches: Vec<MatchEntry>,
    report: ygo_stats::MatchReport,
    /// Versions of the deck, oldest first (none in the log of every deck).
    versions: Vec<LogVersion>,
    /// The list as saved now (in the log of one deck).
    current: Option<LogVersion>,
    /// No match has been played with the current list yet.
    current_is_new: bool,
    /// Opponents and tags used so far, to suggest while logging.
    opponents: Vec<String>,
    tags: Vec<String>,
}

/// Lab: the matches of a deck (or of every deck) with their statistics.
/// For one deck the report compares its versions; for every deck, the decks.
#[tauri::command(async)]
fn match_log(state: State<'_, AppState>, deck_id: Option<i64>) -> Result<MatchLog, String> {
    state.with_user(|u| {
        let matches = u.matches(deck_id)?;
        let (opponents, tags) = u.match_suggestions()?;
        let (mut versions, mut current, mut current_is_new) = (Vec::new(), None, false);
        if let Some(id) = deck_id {
            let lists = u.match_versions(id)?;
            for (i, (version, list)) in lists.iter().enumerate() {
                let before = i.checked_sub(1).map(|j| &lists[j].1).filter(|l| !l.is_empty());
                let changes = before.filter(|_| !list.is_empty()).map(|b| b.changes(list));
                versions.push(LogVersion { version: version.clone(), name: format!("v{}", i + 1), changes });
            }
            if let Some(deck) = u.get(id)? {
                let list = DeckList::of(&deck);
                let version = list.version();
                current = Some(match versions.iter().find(|v| v.version == version) {
                    Some(played) => LogVersion { version, name: played.name.clone(), changes: None },
                    None => {
                        current_is_new = true;
                        let changes = lists.last().filter(|(_, l)| !l.is_empty()).map(|(_, l)| l.changes(&list));
                        LogVersion { version, name: format!("v{}", lists.len() + 1), changes }
                    }
                });
            }
        }
        let names: HashMap<String, String> = match deck_id {
            Some(_) => versions.iter().map(|v| (v.version.clone(), v.name.clone())).collect(),
            None => u.list()?.into_iter().map(|d| (d.id.to_string(), d.name)).collect(),
        };
        let inputs: Vec<ygo_stats::MatchInput> = matches
            .iter()
            .map(|m| {
                let version = match deck_id {
                    Some(_) => m.version.clone(),
                    None => m.deck_id.map(|d| d.to_string()).unwrap_or_default(),
                };
                ygo_stats::MatchInput {
                    opponent: m.opponent.clone(),
                    first: m.first,
                    games: m.games.chars().collect(),
                    result: m.result.chars().next().unwrap_or('D'),
                    tags: m.tags.clone(),
                    version_name: names.get(&version).cloned().unwrap_or_else(|| m.deck_name.clone()),
                    version,
                    played_at: m.played_at.clone(),
                }
            })
            .collect();
        Ok(MatchLog { report: ygo_stats::match_report(&inputs), matches, versions, current, current_is_new, opponents, tags })
    })
}

#[tauri::command(async)]
fn log_match(state: State<'_, AppState>, entry: MatchEntry) -> Result<MatchEntry, String> {
    state.with_user(|u| u.log_match(&entry))
}

#[tauri::command(async)]
fn update_match(state: State<'_, AppState>, entry: MatchEntry) -> Result<MatchEntry, String> {
    state.with_user(|u| u.update_match(&entry))
}

#[tauri::command(async)]
fn delete_match(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.with_user(|u| u.delete_match(id))
}

/// Saves the log as CSV in the downloads folder; returns the file's path.
#[tauri::command(async)]
fn export_matches(app: tauri::AppHandle, state: State<'_, AppState>, deck_id: Option<i64>, name: String) -> Result<String, String> {
    let csv = state.with_user(|u| u.matches(deck_id).map(|m| matchlog::matches_to_csv(&m)))?;
    let path = download_path(&app, &format!("{} - partite", file_stem(&name)), "csv")?;
    // The byte order mark tells spreadsheets the file is UTF-8.
    std::fs::write(&path, format!("\u{feff}{csv}")).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// Adds the matches of a CSV file to a deck; returns how many.
#[tauri::command(async)]
fn import_matches(state: State<'_, AppState>, deck_id: i64, text: String) -> Result<usize, String> {
    let entries = matchlog::matches_from_csv(&text).map_err(|e| e.to_string())?;
    state.with_user(|u| u.import_matches(deck_id, &entries))
}

/// Matches needed to tell `p1` from `p0`: against a fixed rate, and per version.
#[tauri::command]
fn lab_sample_size(p0: f64, p1: f64) -> (u32, u32) {
    ygo_stats::sample_size(p0, p1)
}

#[tauri::command(async)]
fn list_decks(state: State<'_, AppState>) -> Result<Vec<DeckSummary>, String> {
    state.with_user(|u| u.list())
}

/// A deck on the shelf, with its legality checked against the current lists.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShelfDeck {
    #[serde(flatten)]
    summary: DeckSummary,
    /// `None` when the card database is missing.
    legal: Option<bool>,
    errors: usize,
}

fn check(state: &AppState, deck: &Deck) -> Result<Legality, String> {
    let mut ids: Vec<u32> = deck.main.iter().chain(&deck.extra).chain(&deck.side).copied().collect();
    ids.sort_unstable();
    ids.dedup();
    let cards: HashMap<u32, Card> = state.with_store(|s| s.get_many(&ids))?.into_iter().map(|c| (c.id, c)).collect();
    Ok(deck::validate(deck, &cards))
}

#[tauri::command(async)]
fn shelf_decks(state: State<'_, AppState>) -> Result<Vec<ShelfDeck>, String> {
    let decks: Vec<(DeckSummary, Option<Deck>)> =
        state.with_user(|u| u.list()?.into_iter().map(|s| Ok((s.clone(), u.get(s.id)?))).collect())?;
    Ok(decks
        .into_iter()
        .map(|(summary, deck)| {
            let legality = deck.and_then(|d| check(&state, &d).ok());
            ShelfDeck {
                summary,
                legal: legality.as_ref().map(|l| l.legal),
                errors: legality.map_or(0, |l| l.issues.iter().filter(|i| i.severity == ygo_data::Severity::Error).count()),
            }
        })
        .collect())
}

#[tauri::command(async)]
fn set_deck_box(state: State<'_, AppState>, id: i64, deck_box: DeckBox) -> Result<(), String> {
    state.with_user(|u| u.set_box(id, &deck_box))
}

#[tauri::command(async)]
fn duplicate_deck(state: State<'_, AppState>, id: i64) -> Result<Deck, String> {
    state.with_user(|u| u.duplicate(id))
}

#[tauri::command(async)]
fn get_deck(state: State<'_, AppState>, id: i64) -> Result<Option<Deck>, String> {
    state.with_user(|u| u.get(id))
}

#[tauri::command(async)]
fn save_deck(state: State<'_, AppState>, deck: Deck) -> Result<Deck, String> {
    state.with_user(|u| u.save(&deck))
}

#[tauri::command(async)]
fn delete_deck(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.with_user(|u| u.delete(id))
}

#[tauri::command(async)]
fn validate_deck(state: State<'_, AppState>, deck: Deck) -> Result<Legality, String> {
    let mut legality = check(&state, &deck)?;
    if deck.format == Format::Md {
        legality.banlist = state.with_store(|s| Ok(s.banlist_date("md"))).ok().flatten();
    }
    Ok(legality)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Imported {
    main: Vec<u32>,
    extra: Vec<u32>,
    side: Vec<u32>,
    /// Passcodes that match no known card.
    unknown: Vec<u32>,
}

/// Reads a `.ydk` file or a `ydke://` link; alternate-artwork passcodes are
/// mapped to their card.
#[tauri::command(async)]
fn import_deck(state: State<'_, AppState>, text: String) -> Result<Imported, String> {
    let (main, extra, side) = deck::parse_any(&text).map_err(|e| e.to_string())?;
    state.with_store(|s| {
        let mut unknown = Vec::new();
        let mut resolve = |ids: Vec<u32>| -> Result<Vec<u32>, ygo_data::Error> {
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                match s.resolve_id(id)? {
                    Some(card) => out.push(card),
                    None => unknown.push(id),
                }
            }
            Ok(out)
        };
        let (main, extra, side) = (resolve(main)?, resolve(extra)?, resolve(side)?);
        Ok(Imported { main, extra, side, unknown })
    })
}

#[derive(Serialize)]
struct Exported {
    ydk: String,
    ydke: String,
}

#[tauri::command]
fn export_deck(deck: Deck) -> Exported {
    Exported { ydk: deck::to_ydk(&deck), ydke: deck::to_ydke(&deck) }
}

/// Raw bytes of a card image, for drawing on a canvas: an image loaded from
/// `cardimg:` comes from another origin and would block exporting the canvas.
#[tauri::command]
async fn card_image_data(state: State<'_, AppState>, id: u32, size: String) -> Result<tauri::ipc::Response, String> {
    let size = ImageSize::parse(&size).ok_or("formato immagine sconosciuto")?;
    let path = state.images.get(id, size).await.map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(std::fs::read(path).map_err(|e| e.to_string())?))
}

/// Decodes `%XX` escapes (the file name travels in an HTTP-style header).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (bytes[i], bytes.get(i + 1).and_then(|b| hex(*b)), bytes.get(i + 2).and_then(|b| hex(*b))) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (b, _, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A file name made of the deck name's letters, digits and a few signs.
fn file_stem(name: &str) -> String {
    let stem: String = name.chars().map(|c| if c.is_alphanumeric() || " -_()".contains(c) { c } else { '_' }).collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "deck".into()
    } else {
        stem.chars().take(80).collect()
    }
}

/// Saves a PNG sent as the raw request body into Downloads/DuelForge (the
/// deck name comes, percent-encoded, in the `x-file-name` header). Returns
/// the path of the new file.
#[tauri::command]
fn save_deck_image(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("immagine mancante".into());
    };
    let name = request.headers().get("x-file-name").and_then(|v| v.to_str().ok()).map(percent_decode).unwrap_or_default();
    let path = download_path(&app, &file_stem(&name), "png")?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// A new file in the "DuelForge" folder of the downloads, never overwriting one.
fn download_path(app: &tauri::AppHandle, stem: &str, ext: &str) -> Result<PathBuf, String> {
    let dir = app.path().download_dir().or_else(|_| app.path().app_data_dir()).map_err(|e| e.to_string())?.join("DuelForge");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut path = dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).{ext}"));
        n += 1;
    }
    Ok(path)
}

/// Path of a `cardimg://localhost/<size>/<id>` request.
fn parse_image_path(path: &str) -> Option<(ImageSize, u32)> {
    let mut parts = path.trim_start_matches('/').split('/');
    let size = ImageSize::parse(parts.next()?)?;
    let id = parts.next()?.trim_end_matches(".jpg").parse().ok()?;
    Some((size, id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = data_dir(app.handle());
            let client = sync::http_client()?;
            let user_db = std::env::var_os("DUELFORGE_USER_DB")
                .map(PathBuf::from)
                .unwrap_or_else(|| app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from(".")).join("user.db"));
            let state = AppState {
                user: Mutex::new(UserStore::open(&user_db)?),
                images: ImageCache::new(dir.join("images"), client.clone()),
                data_dir: dir,
                store: Mutex::new(None),
                client,
                updating: tokio::sync::Mutex::new(()),
                sim_cancel: AtomicBool::new(false),
            };
            state.open_store();
            app.manage(state);
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("cardimg", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                // Card images are downloaded on first use, then served from disk.
                let image = match parse_image_path(&path) {
                    Some((size, id)) => state.images.get(id, size).await.ok(),
                    None => None,
                };
                let response = match image.and_then(|p| std::fs::read(p).ok()) {
                    Some(bytes) => Response::builder()
                        .header("Content-Type", "image/jpeg")
                        .header("Cache-Control", "public, max-age=31536000, immutable")
                        .body(bytes),
                    None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
                };
                responder.respond(response.expect("valid response"));
            });
        })
        .invoke_handler(tauri::generate_handler![
            data_status,
            update_data,
            search_cards,
            get_cards,
            card_facets,
            list_favorites,
            set_favorite,
            list_saved_searches,
            save_search,
            delete_saved_search,
            list_decks,
            lab_overview,
            lab_probability,
            lab_simulate,
            lab_cancel_simulation,
            lab_analyze,
            lab_optimize,
            set_deck_roles,
            list_recipes,
            save_recipe,
            delete_recipe,
            match_log,
            log_match,
            update_match,
            delete_match,
            export_matches,
            import_matches,
            lab_sample_size,
            shelf_decks,
            set_deck_box,
            duplicate_deck,
            get_deck,
            save_deck,
            delete_deck,
            validate_deck,
            import_deck,
            export_deck,
            card_image_data,
            save_deck_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running DuelForge");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_from_deck_names() {
        assert_eq!(percent_decode("Invoked%20Siracusa%20%C3%A8%20%2F%20MD"), "Invoked Siracusa è / MD");
        assert_eq!(file_stem("Invoked Siracusa è / MD"), "Invoked Siracusa è _ MD");
        assert_eq!(file_stem("  ../  "), "___", "no way out of the folder");
        assert_eq!(file_stem(""), "deck");
    }
}
