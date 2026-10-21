use std::collections::{BTreeMap, HashMap, HashSet};
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
    /// Duel engine data, loaded on the first combo search.
    engine: Mutex<Option<Engine>>,
    /// The last combo search, to explain its boards.
    combo: Mutex<Option<ComboJob>>,
    /// Set to stop the running combo search.
    combo_cancel: AtomicBool,
    /// The last map of links of each deck.
    links: Mutex<HashMap<i64, LinksCache>>,
}

/// A map of links and the cards (Main and Extra Deck) it was read for.
struct LinksCache {
    cards: Vec<u32>,
    view: DeckLinksView,
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

    /// The duel engine with its card data and scripts (loaded once).
    fn engine(&self) -> Result<Engine, String> {
        let mut guard = self.engine.lock().map_err(|e| e.to_string())?;
        if let Some(engine) = guard.as_ref() {
            return Ok(engine.clone());
        }
        let engine =
            Engine::load(&self.data_dir).map_err(|e| format!("Dati del motore non disponibili ({e}): aggiornali dalla Panoramica"))?;
        *guard = Some(engine.clone());
        Ok(engine)
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RoleGuessView {
    code: u32,
    roles: Vec<&'static str>,
    reasons: Vec<String>,
}

#[derive(Serialize, Clone)]
struct RolesProgress {
    done: usize,
    total: usize,
}

/// Not played from the hand on your own turn: the well-known handtraps (an
/// archetype's quick effects from the hand are often combo pieces too).
fn known_hand_traps(engine: &Engine) -> Vec<u32> {
    Tag::Handtrap.names().iter().filter_map(|n| engine.cards().find_by_name(n)).map(|c| c.code).collect()
}

/// The role of each Main Deck card, found by playing it with the engine
/// (alone in hand, next to the best starter, from the hand on the
/// opponent's turn); sends `lab-roles-progress` events on the way.
#[tauri::command(async)]
fn guess_deck_roles(app: tauri::AppHandle, state: State<'_, AppState>, id: i64) -> Result<Vec<RoleGuessView>, String> {
    let engine = state.engine()?;
    let deck = state.with_user(|u| u.get(id))?.ok_or("Il deck non esiste più")?;
    if deck.main.is_empty() {
        return Ok(Vec::new());
    }
    let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
    let kit = ygo_solver::Kit::standard(&engine);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let hand_traps = ygo_solver::hand_traps(&engine, &list, 1, &kit);
    let values = std::sync::Arc::new(ygo_solver::card_values(&engine, &list, 1, &kit, &hand_traps, threads));
    let guesses = ygo_solver::guess_roles(&engine, &list, &kit, &hand_traps, &known_hand_traps(&engine), values, threads, |done, total| {
        let _ = app.emit("lab-roles-progress", RolesProgress { done, total });
    });
    // The deck's own copies (any artwork) get the guess of their card.
    let db = engine.cards();
    let mut out = Vec::new();
    for code in deck.main.iter().copied().collect::<std::collections::BTreeSet<_>>() {
        if let Some(g) = guesses.iter().find(|g| g.code == db.canonical_code(code)) {
            out.push(RoleGuessView { code, roles: g.roles.clone(), reasons: g.reasons.clone() });
        }
    }
    Ok(out)
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

/// The last combo search: what is needed to go on with it and to play its
/// lines again. A precise hand is a "card + rest" search with one hand.
struct ComboJob {
    deck_id: i64,
    /// The search settings; the hand is set for each hand searched.
    config: ygo_solver::SearchConfig,
    /// Each hand with its weight and where its search stopped.
    hands: Vec<(Vec<u32>, f64, ygo_solver::Frontier)>,
    /// Boards kept per hand.
    keep: usize,
    /// `(hand index, board)`.
    boards: Vec<(usize, ygo_solver::Board)>,
    /// Per hand: the goal reached, impossible or unknown, and every line tried.
    status: Vec<(Option<bool>, bool)>,
    /// The deck's card roles (starters and extenders left in hand count for follow-up).
    roles: BTreeMap<String, Vec<u32>>,
    /// Boards already put to the probe turn, by `(hand, key)`.
    scores: HashMap<(usize, u64), ScoreView>,
    /// The deck they were also measured against.
    vs: Option<i64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InterruptionView {
    code: u32,
    /// "mano", "campo", "Cimitero", "bandita".
    place: String,
    effect: String,
    targeted: bool,
    uses: u32,
    coverage: f64,
    /// With your other effects; `alone` with none of them used.
    value: f64,
    alone: f64,
    /// What it stops, in Italian.
    covers: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct BreakerView {
    name: String,
    not_applicable: bool,
    stopped_by: Option<u32>,
    remaining: f64,
}

/// What the probe turn measured on a board, metric by metric (the page
/// weighs them); all in interruptions but resources, follow-up and cost.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ScoreView {
    hand: usize,
    key: String,
    interruptions: f64,
    hand_traps: f64,
    floodgates: f64,
    resilience: f64,
    resources: f64,
    followup: f64,
    cost: f64,
    details: Vec<InterruptionView>,
    floodgate_list: Vec<String>,
    breakers: Vec<BreakerView>,
    resources_note: String,
    followup_note: String,
    /// Moments of the line where a handtrap stops it ([`ygo_solver::Exposure::choke_points`]).
    exposure: f64,
    exposure_note: String,
    exposure_detail: Option<ExposureView>,
    /// Against the deck chosen in [`combo_score`].
    matchup: Option<MatchupView>,
    /// The probe turn could not be played (the board is left out of the ranking).
    error: Option<String>,
}

/// The board and its line against a deck of the Bottega
/// ([`ygo_solver::Matchup`]).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MatchupView {
    deck_id: i64,
    deck_name: String,
    /// Interruption value of the field expected after the most harmful
    /// breaker it has (all of it when it has none).
    resilience: f64,
    /// It has no breaker the probe turn can play: the resilience is the standard one.
    standard_breakers: bool,
    breaker_chance: f64,
    breakers: Vec<RivalBreakerView>,
    /// Chance it has a handtrap that stops something in the line.
    hand_trap_chance: f64,
    hand_traps: Vec<RivalHandTrapView>,
    error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RivalBreakerView {
    code: u32,
    /// Chance it is among the 6 cards the opponent has in its turn.
    chance: f64,
    remaining: f64,
    stopped_by: Option<u32>,
    not_applicable: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RivalHandTrapView {
    code: u32,
    /// Chance it is among the 5 cards the opponent holds during your turn.
    chance: f64,
    /// Moments of the line it can be used, and those where it changes the
    /// field or hand you end with.
    windows: u32,
    stops: u32,
    /// Your cards being played at those moments.
    at: Vec<u32>,
}

fn matchup_view(deck_id: i64, deck_name: &str, matchup: Result<ygo_solver::Matchup, String>) -> MatchupView {
    let mut view = MatchupView {
        deck_id,
        deck_name: deck_name.to_owned(),
        resilience: 0.0,
        standard_breakers: false,
        breaker_chance: 0.0,
        breakers: Vec::new(),
        hand_trap_chance: 0.0,
        hand_traps: Vec::new(),
        error: None,
    };
    match matchup {
        Ok(x) => {
            view.resilience = x.resilience;
            view.standard_breakers = x.standard_breakers;
            view.breaker_chance = x.breaker_chance;
            view.breakers = x
                .breakers
                .iter()
                .map(|b| RivalBreakerView {
                    code: b.code,
                    chance: b.chance,
                    remaining: b.remaining,
                    stopped_by: b.stopped_by.map(|r| r.code),
                    not_applicable: b.not_applicable,
                })
                .collect();
            view.hand_trap_chance = x.hand_trap_chance;
            view.hand_traps = x
                .hand_traps
                .iter()
                .map(|h| RivalHandTrapView { code: h.code, chance: h.chance, windows: h.windows, stops: h.stops, at: h.at.clone() })
                .collect();
        }
        Err(e) => view.error = Some(e),
    }
    view
}

/// What the line does that the handtraps played most can stop.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ExposureView {
    ash: u32,
    on_field: u32,
    summons: u32,
    special_summons: u32,
    searches: u32,
}

fn place_name(loc: u8) -> &'static str {
    match loc {
        ygo_engine::consts::location::HAND => "mano",
        ygo_engine::consts::location::GRAVE => "Cimitero",
        ygo_engine::consts::location::REMOVED => "bandita",
        _ => "campo",
    }
}

/// The cards in hand that count as resources, in words.
fn resources_note(others: usize, grave: usize, dead: usize) -> String {
    let hand = match others {
        0 => "nessun'altra carta in mano".to_owned(),
        1 => "1 altra carta in mano".to_owned(),
        n => format!("{n} altre carte in mano"),
    };
    let grave = if grave == 1 { "1 carta nel Cimitero".to_owned() } else { format!("{grave} carte nel Cimitero") };
    match dead {
        0 => format!("{hand}, {grave}"),
        1 => format!("{hand}, {grave}; 1 handtrap che con questa board non si può usare (solo a campo vuoto): è la carta da scartare"),
        n => format!(
            "{hand}, {grave}; {n} handtrap che con questa board non si possono usare (solo a campo vuoto): sono le carte da scartare"
        ),
    }
}

/// The metrics of a board from its measure and its state.
fn score_view(
    engine: &ygo_engine::Engine,
    hand: usize,
    board: &ygo_solver::Board,
    measured: Result<ygo_solver::Measure, String>,
    exposure: Option<ygo_solver::Exposure>,
    hand_traps: &[u32],
    roles: &BTreeMap<String, Vec<u32>>,
) -> ScoreView {
    let me = &board.state.players[0];
    let db = engine.cards();
    // Each card in hand counts once: a handtrap the probe turn measured in
    // its metric, a starter or extender in the follow-up, any other card in
    // the resources. A handtrap that could not even be used with this board
    // (the Mulcharmy while you control cards) counts nowhere: going first
    // it is a dead card, the one to discard.
    let mut rest = me.hand.clone();
    let mut dead = 0;
    if let Ok(m) = &measured {
        for i in m.interruptions.iter().filter(|i| i.location == ygo_engine::consts::location::HAND) {
            if let Some(at) = rest.iter().position(|&c| db.canonical_code(c) == i.responder.code) {
                rest.remove(at);
            }
        }
        let usable = |c: u32| {
            m.offered.iter().any(|&(o, loc)| db.canonical_code(o) == db.canonical_code(c) && loc == ygo_engine::consts::location::HAND)
        };
        let hand_trap = |c: u32| hand_traps.iter().any(|&h| db.canonical_code(h) == db.canonical_code(c));
        rest.retain(|&c| {
            let unusable = hand_trap(c) && !usable(c);
            dead += usize::from(unusable);
            !unusable
        });
    }
    let has_role = |c: u32, role: &str| roles.get(role).is_some_and(|ids| ids.contains(&c));
    let (starters, extenders) =
        (rest.iter().filter(|&&c| has_role(c, "starter")).count(), rest.iter().filter(|&&c| has_role(c, "extender")).count());
    let followup = starters as f64 + 0.5 * extenders as f64;
    let others = rest.iter().filter(|&&c| !has_role(c, "starter") && !has_role(c, "extender")).count();
    let resources = others as f64 + 0.1 * me.grave.len() as f64;
    let mut view = ScoreView {
        hand,
        key: format!("{:016x}", board.key),
        interruptions: 0.0,
        hand_traps: 0.0,
        floodgates: 0.0,
        resilience: 0.0,
        resources,
        followup,
        cost: board.cards_used.saturating_sub(1) as f64,
        details: Vec::new(),
        floodgate_list: Vec::new(),
        breakers: Vec::new(),
        resources_note: resources_note(others, me.grave.len(), dead),
        followup_note: match (starters, extenders) {
            (0, 0) => "nessuno starter o estensore in mano".to_owned(),
            (s, 0) => format!("{s} starter in mano"),
            (0, 1) => "1 estensore in mano".to_owned(),
            (0, e) => format!("{e} estensori in mano"),
            (s, 1) => format!("{s} starter e 1 estensore in mano"),
            (s, e) => format!("{s} starter e {e} estensori in mano"),
        },
        exposure: exposure.map_or(0.0, |x| x.choke_points() as f64),
        exposure_detail: exposure.map(|x| ExposureView {
            ash: x.ash,
            on_field: x.on_field,
            summons: x.summons,
            special_summons: x.special_summons,
            searches: x.searches,
        }),
        exposure_note: exposure.map_or_else(String::new, |x| {
            let mut parts = vec![
                format!("{} effetti che Ash Blossom nega", x.ash),
                format!("{} effetti di mostri sul campo (Veiler, Impermanence)", x.on_field),
            ];
            parts.push(if x.summons >= 5 {
                format!("{} Evocazioni: Nibiru dalla quinta", x.summons)
            } else {
                format!("{} Evocazioni: Nibiru non arriva", x.summons)
            });
            if x.searches >= 2 {
                parts.push(format!("{} carte aggiunte dal Deck: Droll ferma le successive", x.searches));
            }
            parts.push(format!("{} Evocazioni Speciali (carte per Maxx \"C\")", x.special_summons));
            parts.join(", ")
        }),
        matchup: None,
        error: None,
    };
    match measured {
        Ok(m) => {
            view.interruptions = m.field_value;
            view.hand_traps = m.hand_value;
            view.floodgates = m.floodgate_value;
            view.resilience = m.resilience;
            view.details = m
                .interruptions
                .iter()
                .map(|i| InterruptionView {
                    code: i.responder.code,
                    place: place_name(i.location).to_owned(),
                    effect: i.effect.label().to_owned(),
                    targeted: i.targeted,
                    uses: i.uses,
                    coverage: i.coverage,
                    value: i.value,
                    alone: i.alone,
                    covers: i.covers.iter().map(|p| p.label().to_owned()).collect(),
                })
                .collect();
            view.floodgate_list = m.floodgates.iter().map(|p| p.label().to_owned()).collect();
            view.breakers = m
                .breakers
                .iter()
                .map(|b| BreakerView {
                    name: b.name.clone(),
                    not_applicable: b.not_applicable,
                    stopped_by: b.stopped_by.map(|r| r.code),
                    remaining: b.remaining,
                })
                .collect();
        }
        Err(e) => view.error = Some(e),
    }
    view
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ZoneCard {
    code: u32,
    face_up: bool,
    defense: bool,
    /// Link rating and arrows.
    link: u32,
    arrows: u32,
    level: u32,
    rank: u32,
    attack: i32,
    def: i32,
    overlay: Vec<u32>,
    counters: u32,
    negated: bool,
}

impl ZoneCard {
    fn of(c: &ygo_engine::FieldCard) -> Self {
        ZoneCard {
            code: c.code,
            face_up: c.is_face_up(),
            defense: c.position & 0xc != 0,
            link: c.link.0,
            arrows: c.link.1,
            level: c.level,
            rank: c.rank,
            attack: c.attack,
            def: c.defense,
            overlay: c.overlay.clone(),
            counters: c.counters.iter().map(|&(_, n)| n as u32).sum(),
            negated: c.is_negated(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct BoardView {
    /// Hex of the board's key; with `hand`, names the board for `combo_explain`.
    key: String,
    hand: usize,
    quick_score: f64,
    cards_used: usize,
    choices: usize,
    found_after_ms: u64,
    /// Played to the opponent's turn (else read when the turn was ended).
    exact: bool,
    goal_hits: usize,
    #[serde(flatten)]
    field: FieldView,
}

/// Your side of the duel: the board's, or the one a step of its line left.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FieldView {
    monsters: Vec<Option<ZoneCard>>,
    spells: Vec<Option<ZoneCard>>,
    hand_cards: Vec<u32>,
    grave: Vec<u32>,
    banished: Vec<u32>,
    deck_count: usize,
    extra_count: usize,
    /// Face-up Pendulum monsters in the Extra Deck.
    extra_face_up: Vec<u32>,
}

fn field_view(state: &ygo_engine::DuelState) -> FieldView {
    let me = &state.players[0];
    let zone = |z: &Option<ygo_engine::FieldCard>| z.as_ref().map(ZoneCard::of);
    FieldView {
        monsters: me.monsters.iter().map(zone).collect(),
        spells: me.spells.iter().map(zone).collect(),
        hand_cards: me.hand.clone(),
        grave: me.grave.clone(),
        banished: me.banished.iter().map(|&(c, _)| c).collect(),
        deck_count: me.deck.len(),
        extra_count: me.extra.len(),
        extra_face_up: me.extra.iter().filter(|&&(_, up)| up).map(|&(c, _)| c).collect(),
    }
}

fn board_view(hand: usize, b: &ygo_solver::Board) -> BoardView {
    BoardView {
        key: format!("{:016x}", b.key),
        hand,
        quick_score: b.quick_score,
        cards_used: b.cards_used,
        choices: b.line.choices.len(),
        found_after_ms: b.found_after.as_millis() as u64,
        exact: b.exact,
        goal_hits: b.goal_hits,
        field: field_view(&b.state),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HandView {
    cards: Vec<u32>,
    weight: f64,
    /// The goal is reached, impossible (`false`), or unknown (`null`).
    goal: Option<bool>,
    /// Every line from this hand was tried.
    complete: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ComboProgress {
    elapsed_ms: u64,
    nodes: u64,
    replays: u64,
    duplicates: u64,
    queued: usize,
    boards: usize,
    /// Hand being searched ("card + rest"), with all the hands.
    hand_index: usize,
    hands: Vec<HandView>,
    top: Vec<BoardView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComboOutcome {
    stop: String,
    elapsed_ms: u64,
    nodes: u64,
    hands: Vec<HandView>,
    /// Best first; in "card + rest" the best few of each hand.
    boards: Vec<BoardView>,
    /// Every line of every hand was tried: going on finds nothing new.
    complete: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoalCardRequest {
    code: u32,
    /// "any", "extra" (Extra Monster Zone) or "main".
    zone: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComboGoal {
    cards: Vec<GoalCardRequest>,
    /// All of them (else one is enough).
    all: bool,
    /// Stop at the first board that reaches it.
    first: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComboRequest {
    deck_id: i64,
    /// A precise hand, or empty with `card` set for "card + rest".
    hand: Vec<u32>,
    card: Option<u32>,
    seconds: u64,
    /// Hands to try in "card + rest".
    hands: Option<usize>,
    keep: Vec<u32>,
    forbid: Vec<u32>,
    use_handtraps: bool,
    goal: Option<ComboGoal>,
}

fn hand_views(job: &ComboJob) -> Vec<HandView> {
    job.hands
        .iter()
        .enumerate()
        .map(|(i, (cards, weight, _))| {
            let (goal, complete) = job.status.get(i).copied().unwrap_or((None, false));
            HandView { cards: cards.clone(), weight: *weight, goal, complete }
        })
        .collect()
}

/// Searches (or goes on searching) every hand of `job` for `seconds` in
/// all, sending `combo-progress` events.
fn run_combo(app: &tauri::AppHandle, engine: &Engine, job: &mut ComboJob, seconds: u64, cancel: &AtomicBool) -> ComboOutcome {
    let started = std::time::Instant::now();
    let count = job.hands.len().max(1) as u32;
    let what = ygo_solver::CardSearch {
        card: 0,
        hands: job.hands.len(),
        per_hand: std::time::Duration::from_secs(seconds.clamp(5, 3600)) / count,
        keep: job.keep,
    };
    let views = hand_views(job);
    let emit = |hand_index: usize, _: &[(Vec<u32>, f64)], p: &ygo_solver::Progress| {
        let _ = app.emit(
            "combo-progress",
            ComboProgress {
                elapsed_ms: p.elapsed.as_millis() as u64,
                nodes: p.nodes,
                replays: p.replays,
                duplicates: p.duplicates,
                queued: p.queued,
                boards: p.boards,
                hand_index,
                hands: views.clone(),
                top: p.top.iter().map(|b| board_view(hand_index, b)).collect(),
            },
        );
    };
    let previous = std::mem::take(&mut job.hands);
    let results = ygo_solver::resume_card(engine, &job.config, &what, previous, cancel, emit);
    let nodes = results.iter().map(|r| r.nodes).sum();
    let stop = match results.as_slice() {
        [one] => format!("{:?}", one.stop),
        _ if cancel.load(Ordering::Relaxed) => "Cancelled".to_owned(),
        _ => "Finished".to_owned(),
    };
    job.status = results.iter().map(|r| (r.goal, r.frontier.is_exhausted())).collect();
    job.boards = results.iter().enumerate().flat_map(|(i, r)| r.boards.iter().map(move |b| (i, b.clone()))).collect();
    job.hands = results.into_iter().map(|r| (r.hand, r.weight, r.frontier)).collect();
    ComboOutcome {
        stop,
        elapsed_ms: started.elapsed().as_millis() as u64,
        nodes,
        hands: hand_views(job),
        boards: job.boards.iter().map(|(h, b)| board_view(*h, b)).collect(),
        complete: job.status.iter().all(|&(_, done)| done),
    }
}

/// Combo Lab: the engine plays every line from a hand (or from many hands
/// with a card), sending `combo-progress` events, and returns the boards.
#[tauri::command(async)]
fn combo_search(app: tauri::AppHandle, state: State<'_, AppState>, request: ComboRequest) -> Result<ComboOutcome, String> {
    // Reset before the slow setup (the card values): a Stop pressed during
    // it still stops the search.
    state.combo_cancel.store(false, Ordering::Relaxed);
    let engine = state.engine()?;
    let deck = state.with_user(|u| u.get(request.deck_id))?.ok_or("Il deck non esiste più")?;
    let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
    let mut config = ygo_solver::SearchConfig::new(list.clone(), Vec::new());
    config.keep_in_hand = request.keep;
    config.policy.forbid = request.forbid;
    // The search plays against the probe turn's opponent: its lines play the same there.
    let kit = ygo_solver::Kit::standard(&engine);
    config.opponent = Some(kit.opponent().clone());
    // Handtraps, read from the engine: kept in hand they are worth more.
    config.hand_traps = ygo_solver::hand_traps(&engine, &list, config.seed, &kit);
    // What each card stops alone on the field: the search steers by it,
    // and learns on the way what cards stop together.
    config.values = std::sync::Arc::new(ygo_solver::card_values(&engine, &list, config.seed, &kit, &config.hand_traps, config.threads));
    config.learn = Some(std::sync::Arc::new(kit.clone()));
    if !request.use_handtraps {
        config.policy.passive = known_hand_traps(&engine);
    }
    if let Some(goal) = request.goal.filter(|g| !g.cards.is_empty()) {
        let db = engine.cards();
        let in_deck = |code: u32| list.main.iter().chain(&list.extra).any(|&c| db.canonical_code(c) == db.canonical_code(code));
        if let Some(missing) = goal.cards.iter().find(|c| !in_deck(c.code)) {
            return Err(format!("{} non è nel deck: l'obiettivo è impossibile", db.name(missing.code)));
        }
        let zone = |z: &str| match z {
            "extra" => ygo_solver::GoalZone::Extra,
            "main" => ygo_solver::GoalZone::Main,
            _ => ygo_solver::GoalZone::Any,
        };
        let on_field = goal.cards.iter().map(|c| ygo_solver::GoalCard { code: c.code, zone: zone(&c.zone) }).collect();
        config.goal = ygo_solver::Goal { on_field, all: goal.all, first_only: goal.first };
    }
    let (hands, keep) = match request.card {
        Some(card) if request.hand.is_empty() => {
            let count = request.hands.unwrap_or(12).clamp(1, 200);
            let sampled = ygo_solver::sample_hands(&list.main, card, count, config.seed);
            if sampled.is_empty() {
                return Err("La carta non è nel Main Deck".into());
            }
            (sampled, 5)
        }
        _ if request.hand.is_empty() => return Err("Scegli le carte della mano".into()),
        _ => {
            // A hand the engine cannot deal would look like a search with no boards.
            ygo_engine::Duel::goldfish(&engine, &list, &request.hand, config.seed).map_err(|e| format!("Mano non valida: {e}"))?;
            (vec![(request.hand.clone(), 1.0)], 200)
        }
    };
    let mut job = ComboJob {
        deck_id: request.deck_id,
        config,
        hands: hands.into_iter().map(|(h, w)| (h, w, ygo_solver::Frontier::default())).collect(),
        keep,
        boards: Vec::new(),
        status: Vec::new(),
        roles: deck.roles.clone(),
        scores: HashMap::new(),
        vs: None,
    };
    if let Ok(mut slot) = state.combo.lock() {
        *slot = None;
    }
    let outcome = run_combo(&app, &engine, &mut job, request.seconds, &state.combo_cancel);
    if let Ok(mut slot) = state.combo.lock() {
        *slot = Some(job);
    }
    Ok(outcome)
}

/// Goes on with the last search for `seconds` more, from where it stopped.
#[tauri::command(async)]
fn combo_continue(app: tauri::AppHandle, state: State<'_, AppState>, deck_id: i64, seconds: u64) -> Result<ComboOutcome, String> {
    let engine = state.engine()?;
    let mut job = state.combo.lock().map_err(|e| e.to_string())?.take().ok_or("Nessuna ricerca da continuare")?;
    if job.deck_id != deck_id {
        return Err("La ricerca da continuare è di un altro deck".into());
    }
    // Going on means looking for better lines, not stopping at the next one.
    job.config.goal.first_only = false;
    state.combo_cancel.store(false, Ordering::Relaxed);
    let outcome = run_combo(&app, &engine, &mut job, seconds, &state.combo_cancel);
    if let Ok(mut slot) = state.combo.lock() {
        *slot = Some(job);
    }
    Ok(outcome)
}

/// How one card of a deck leads to another (`docs/plans/mappa-collegamenti.md`).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CardLinkView {
    from: u32,
    to: u32,
    /// "search" (from the Deck to the hand), "summon" (Special Summoned),
    /// "toGrave" (from the Deck to the GY), "recover" (from the GY or
    /// banished back to the hand or Deck), "material" (`from` is a material of `to`).
    kind: String,
    /// Where `to` comes from: "deck", "extra", "grave", "banished", "hand".
    zone: String,
    /// The engine offered it: the effect can pick that card.
    possible: bool,
    /// Share of the Combo Lab's best lines that use it, 0 to 1.
    used: f64,
}

/// A card of the map.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LinkCardView {
    code: u32,
    copies: u32,
    extra: bool,
    /// Cards it leads to in one step; with those brought into play from there
    /// in any number of steps ([`ygo_solver::LinkCard`]).
    reach: u32,
    reach_all: u32,
    /// Chance that an opening hand of 5 holds it or a card that brings it into play.
    chance: f64,
}

/// The map of the links between the cards of a deck.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeckLinksView {
    deck_id: i64,
    cards: Vec<LinkCardView>,
    links: Vec<CardLinkView>,
    /// Hands played to read the links, and best lines counted for `used`.
    hands: u32,
    lines: u32,
}

/// Laboratorio, "Collegamenti": the map of the links between the cards of a
/// deck, read by the engine (lines searched from hands holding each card,
/// about half a minute), sending `deck-links-progress` events. Kept until
/// the deck's cards change.
#[tauri::command(async)]
fn deck_links(app: tauri::AppHandle, state: State<'_, AppState>, deck_id: i64) -> Result<DeckLinksView, String> {
    let engine = state.engine()?;
    let deck = state.with_user(|u| u.get(deck_id))?.ok_or("Il deck non esiste più")?;
    let cards: Vec<u32> = deck.main.iter().chain(&deck.extra).copied().collect();
    if let Some(cached) = state.links.lock().map_err(|e| e.to_string())?.get(&deck_id).filter(|c| c.cards == cards) {
        return Ok(cached.view.clone());
    }
    let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: Vec::new() };
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let budget = ygo_solver::LinksBudget { threads, ..ygo_solver::LinksBudget::default() };
    let map = ygo_solver::deck_links(&engine, &list, 1, budget, &AtomicBool::new(false), |done, total| {
        let _ = app.emit("deck-links-progress", RolesProgress { done, total });
    });
    let view = DeckLinksView {
        deck_id,
        cards: map
            .cards
            .iter()
            .map(|c| LinkCardView {
                code: c.code,
                copies: c.copies,
                extra: c.extra,
                reach: c.reach,
                reach_all: c.reach_all,
                chance: c.chance,
            })
            .collect(),
        links: map
            .links
            .iter()
            .map(|l| CardLinkView {
                from: l.from,
                to: l.to,
                kind: l.kind.key().to_owned(),
                zone: l.zone.key().to_owned(),
                possible: l.possible,
                used: l.used,
            })
            .collect(),
        hands: map.hands,
        lines: map.lines,
    };
    state.links.lock().map_err(|e| e.to_string())?.insert(deck_id, LinksCache { cards, view: view.clone() });
    Ok(view)
}

/// Boards put to the probe turn after a search: the best of a precise hand,
/// all those kept in "card + rest".
const SCORED_BOARDS: usize = 30;

/// Puts the best boards of the last search to the probe turn, on all cores,
/// sending each result as a `combo-score` event; boards already measured
/// are not measured again. With `vs_deck_id` (a deck of the Bottega) each
/// board is also measured against that deck's breakers and handtraps
/// (another deck than last time measures all boards again). Returns how
/// many boards have a score.
#[tauri::command(async)]
fn combo_score(app: tauri::AppHandle, state: State<'_, AppState>, vs_deck_id: Option<i64>) -> Result<usize, String> {
    let engine = state.engine()?;
    // What is needed, copied out so the search state stays free meanwhile.
    let (config, hands, todo, roles) = {
        let mut job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_mut().ok_or("Nessuna ricerca da valutare")?;
        if job.vs != vs_deck_id {
            job.scores.clear();
            job.vs = vs_deck_id;
        }
        let limit = if job.hands.len() == 1 { SCORED_BOARDS } else { usize::MAX };
        // One board per look: boards that differ only in the Graveyard or the banished cards score the same.
        let mut looks: HashSet<(usize, u64)> = HashSet::new();
        for (h, b) in &job.boards {
            if job.scores.contains_key(&(*h, b.key)) {
                looks.insert((*h, ygo_solver::look_key(&b.state)));
            }
        }
        let todo: Vec<(usize, ygo_solver::Board)> = job
            .boards
            .iter()
            .filter(|(h, b)| !job.scores.contains_key(&(*h, b.key)) && looks.insert((*h, ygo_solver::look_key(&b.state))))
            .take(limit)
            .cloned()
            .collect();
        (job.config.clone(), job.hands.iter().map(|(h, _, _)| h.clone()).collect::<Vec<_>>(), todo, job.roles.clone())
    };
    state.combo_cancel.store(false, Ordering::Relaxed);
    let kit = ygo_solver::Kit::standard(&engine);
    let rival = match vs_deck_id {
        Some(id) => {
            let deck = state.with_user(|u| u.get(id))?.ok_or("Il deck avversario non esiste più")?;
            let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
            Some((id, deck.name.clone(), ygo_solver::Rival::new(&engine, &list, config.seed, &kit)))
        }
        None => None,
    };
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                // One prober per hand: it keeps the empty board of that hand.
                let mut probers: HashMap<usize, ygo_solver::Prober> = HashMap::new();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((hand, board)) = todo.get(i) else { break };
                    if state.combo_cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let prober = probers
                        .entry(*hand)
                        .or_insert_with(|| ygo_solver::Prober::new(&engine, &config.deck, &hands[*hand], config.seed, &kit));
                    let exposure =
                        ygo_solver::exposure(&engine, &config.deck, &hands[*hand], config.seed, config.opponent.as_ref(), &board.line).ok();
                    let measured = prober.measure(&board.line).map_err(|e| e.to_string());
                    let matchup = rival.as_ref().map(|(id, name, rival)| {
                        let m = measured.as_ref().map_err(String::clone).and_then(|m| {
                            ygo_solver::matchup(&engine, &config.deck, &hands[*hand], config.seed, &board.line, m, rival)
                                .map_err(|e| e.to_string())
                        });
                        matchup_view(*id, name, m)
                    });
                    let mut view = score_view(&engine, *hand, board, measured, exposure, &config.hand_traps, &roles);
                    view.matchup = matchup;
                    let _ = app.emit("combo-score", view.clone());
                    if let Ok(mut r) = results.lock() {
                        r.push(((*hand, board.key), view));
                    }
                }
            });
        }
    });
    let mut job = state.combo.lock().map_err(|e| e.to_string())?;
    let job = job.as_mut().ok_or("La ricerca è cambiata")?;
    job.scores.extend(results.into_inner().unwrap_or_default());
    Ok(job.scores.len())
}

/// The scores already measured for the last search.
#[tauri::command]
fn combo_scores(state: State<'_, AppState>) -> Result<Vec<ScoreView>, String> {
    let job = state.combo.lock().map_err(|e| e.to_string())?;
    Ok(job.as_ref().map(|j| j.scores.values().cloned().collect()).unwrap_or_default())
}

#[tauri::command]
fn combo_cancel(state: State<'_, AppState>) {
    state.combo_cancel.store(true, Ordering::Relaxed);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExplainedStepView {
    choice: Option<String>,
    events: Vec<String>,
    /// What the choice does, for its icon: "normalSummon", "specialSummon",
    /// "activate", "set", "choose", "place", "pass", "endTurn", "other".
    kind: Option<&'static str>,
    card: Option<u32>,
    happened: Vec<HappeningView>,
    /// Your side once the step is done, in the board's shape.
    field: Option<FieldView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HappeningView {
    /// "phase", "draw", "summon", "activate", "negate", "set", "toHand",
    /// "toGrave", "banish", "toDeck", "toExtra", "material", "toField", "other".
    kind: &'static str,
    code: u32,
    chain_link: u32,
    mine: bool,
    text: String,
}

fn action_kind_name(kind: ygo_engine::ActionKind) -> &'static str {
    use ygo_engine::ActionKind as K;
    match kind {
        K::NormalSummon => "normalSummon",
        K::SpecialSummon => "specialSummon",
        K::Activate => "activate",
        K::SetMonster | K::SetSpellTrap => "set",
        // Ending a selection is a choice too (the cards picked so far).
        K::Choose | K::Yes | K::Announce | K::Order | K::Position | K::Finish => "choose",
        K::Place => "place",
        K::Pass | K::No | K::Cancel => "pass",
        K::EndTurn => "endTurn",
        _ => "other",
    }
}

fn happening_view(h: ygo_solver::Happening) -> HappeningView {
    use ygo_solver::HappeningKind as H;
    let kind = match h.kind {
        H::Phase => "phase",
        H::Draw => "draw",
        H::Summon => "summon",
        H::Activate => "activate",
        H::Negate => "negate",
        H::Set => "set",
        H::ToHand => "toHand",
        H::ToGrave => "toGrave",
        H::Banish => "banish",
        H::ToDeck => "toDeck",
        H::ToExtra => "toExtra",
        H::Material => "material",
        H::ToField => "toField",
        H::Other => "other",
    };
    HappeningView { kind, code: h.code, chain_link: h.chain_link, mine: h.mine, text: h.text }
}

/// The line of a board of the last search, played again and told in Italian.
#[tauri::command(async)]
fn combo_explain(state: State<'_, AppState>, hand: usize, key: String) -> Result<Vec<ExplainedStepView>, String> {
    let engine = state.engine()?;
    let job = state.combo.lock().map_err(|e| e.to_string())?;
    let job = job.as_ref().ok_or("Nessuna ricerca da spiegare")?;
    let board =
        job.boards.iter().find(|(h, b)| *h == hand && format!("{:016x}", b.key) == key).map(|(_, b)| b).ok_or("Board non trovata")?;
    let (cards, _, _) = job.hands.get(hand).ok_or("Mano non trovata")?;
    let (_, steps) = ygo_solver::explain(&engine, &job.config.deck, cards, job.config.seed, job.config.opponent.as_ref(), &board.line)
        .map_err(|e| e.to_string())?;
    Ok(steps
        .into_iter()
        .map(|s| ExplainedStepView {
            choice: s.choice,
            events: s.events,
            kind: s.kind.map(action_kind_name),
            card: s.card,
            happened: s.happened.into_iter().map(happening_view).collect(),
            field: s.state.as_ref().map(field_view),
        })
        .collect())
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
                engine: Mutex::new(None),
                combo: Mutex::new(None),
                combo_cancel: AtomicBool::new(false),
                links: Mutex::new(HashMap::new()),
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
            guess_deck_roles,
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
            combo_search,
            combo_continue,
            combo_score,
            combo_scores,
            deck_links,
            combo_cancel,
            combo_explain,
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
