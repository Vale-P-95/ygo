use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::http::{Response, StatusCode};
use tauri::{Emitter, Manager, State};
use ygo_data::sync::{self, Progress};
use ygo_data::{
    banlist, deck, matchlog, BanChange, Banlist, Card, CardChange, CardKind, CardStore, Chip, Deck, DeckBox, DeckList, DeckSummary, Facets,
    Format, ImageCache, ImageSize, Legality, MatchEntry, Recipe, SavedSearch, Tag, UserStore,
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
    /// The best lines of each rival deck, read for level 2 of the probe turn.
    rival_lines: Mutex<HashMap<i64, RivalLinesCache>>,
    /// The meta read from Master Duel Meta for [`combo_meta`], and its
    /// decks' lines: kept for this session only.
    meta: Mutex<Option<MetaCache>>,
    /// The last tech table, and the cards (Main, Extra, Side) of its deck.
    tech: Mutex<Option<(Vec<u32>, TechTableView)>>,
    /// Set to stop the running "per secondo" measure.
    second_cancel: AtomicBool,
    /// Set to stop the running engine Monte Carlo.
    mc_cancel: AtomicBool,
}

/// The decks of the meta and, once read, the most played ones with their
/// lines for a seed (the first of them serve a call that wants fewer).
struct MetaCache {
    decks: Vec<ygo_solver::MetaDeck>,
    rivals: Option<(u64, std::sync::Arc<Vec<ygo_solver::MetaRival>>)>,
}

/// The best lines of a rival deck (level 2 of the probe turn against a
/// deck), read once for its cards and the search's seed.
struct RivalLinesCache {
    cards: Vec<u32>,
    seed: u64,
    lines: std::sync::Arc<Vec<ygo_solver::RivalLine>>,
}

/// A map of links and the cards (Main and Extra Deck) it was read for.
struct LinksCache {
    cards: Vec<u32>,
    map: ygo_solver::DeckLinks,
    view: DeckLinksView,
    /// What was searched for it: after a change of the deck, only the
    /// hands the change needs are searched again.
    work: ygo_solver::LinksWork,
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
    if sync::update_cards(&state.client, current.as_deref(), Some(&state.cards_db()), &new_db, &report)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
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

/// Cards whose Forbidden & Limited status changed with a list update,
/// newest first (Il Banditore); only `format`'s (`tcg`, `ocg`, `md`) when given.
#[tauri::command(async)]
fn ban_changes(state: State<'_, AppState>, format: Option<String>) -> Result<Vec<BanChange>, String> {
    state.with_store(|s| s.ban_changes(format.as_deref()))
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

fn lab_facts(card: &Card) -> ygo_stats::Facts {
    let kind = match card.kind {
        CardKind::Monster => "monster",
        CardKind::Spell => "spell",
        CardKind::Trap => "trap",
        _ => "other",
    };
    ygo_stats::Facts {
        id: card.id,
        name: card.name.clone(),
        kind: kind.to_owned(),
        race: Some(card.race.clone()).filter(|r| !r.is_empty()),
        attribute: card.attribute.clone(),
        level: card.level,
        atk: card.atk,
        def: card.def,
    }
}

/// A card the lab's "Carte speciali" knows, for trying it in a deck that lacks it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpecialKnown {
    id: u32,
    name: String,
    mechanism: ygo_stats::Mechanism,
}

/// Lab, "Carte speciali": the cards whose mechanism the lab knows, with their passcode.
#[tauri::command(async)]
fn lab_special_known(state: State<'_, AppState>) -> Result<Vec<SpecialKnown>, String> {
    state.with_store(|s| {
        let mut out = Vec::new();
        for k in ygo_stats::special::known() {
            if let Some(id) = s.id_by_name(&k.name)? {
                out.push(SpecialKnown { id, name: k.name.clone(), mechanism: k.mechanism.clone() });
            }
        }
        Ok(out)
    })
}

/// Lab, "Carte speciali": how the deck's draw spells, excavators, bridges
/// and searches change the chance of reaching `target`. The mechanism of a
/// card comes from its name (`special.json`), the rest from its data.
#[tauri::command(async)]
fn lab_special(
    state: State<'_, AppState>,
    main: Vec<u32>,
    extra: Vec<u32>,
    roles: BTreeMap<String, Vec<u32>>,
    target: ygo_stats::Target,
    hand: u32,
) -> Result<ygo_stats::SpecialReport, String> {
    let mut ids: Vec<u32> = main.iter().chain(&extra).copied().collect();
    ids.sort_unstable();
    ids.dedup();
    let cards: HashMap<u32, Card> = state.with_store(|s| s.get_many(&ids))?.into_iter().map(|c| (c.id, c)).collect();
    let deck = ygo_stats::DeckModel::from_ids(&main, &roles);
    let special = deck
        .cards
        .iter()
        .filter_map(|c| cards.get(&c.id).and_then(|card| ygo_stats::special::mechanism_of(&card.name)).map(|m| (c.id, m.clone())))
        .collect();
    let input = ygo_stats::SpecialInput {
        cards: deck.cards.iter().filter_map(|c| cards.get(&c.id)).map(lab_facts).collect(),
        extra: extra.iter().filter_map(|id| cards.get(id)).map(lab_facts).collect(),
        deck,
        hand,
        target,
        special,
    };
    ygo_stats::special(&input).map_err(|e| e.to_string())
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

/// Combo Lab: of two boards compared, the player preferred the one with
/// `chosen` metrics. Returns how many choices are recorded.
#[tauri::command]
fn record_board_choice(
    state: State<'_, AppState>,
    deck_id: Option<i64>,
    chosen: BTreeMap<String, f64>,
    other: BTreeMap<String, f64>,
) -> Result<usize, String> {
    state.with_user(|u| {
        u.add_board_choice(deck_id, &chosen, &other)?;
        Ok(u.board_choices()?.len())
    })
}

/// Score weights learned from the player's choices ([`ygo_stats::learn_weights`]).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LearnedWeightsView {
    weights: BTreeMap<String, f64>,
    choices: usize,
    /// Share of the choices the learned weights explain, and the prior's.
    agreement: f64,
    prior_agreement: f64,
}

/// The score weights that explain the boards the player preferred,
/// starting from `prior` (the profile in use) and pulled towards it.
#[tauri::command]
fn learned_weights(state: State<'_, AppState>, prior: BTreeMap<String, f64>) -> Result<LearnedWeightsView, String> {
    let choices: Vec<ygo_stats::Choice> =
        state.with_user(|u| u.board_choices())?.into_iter().map(|c| ygo_stats::Choice { chosen: c.chosen, other: c.other }).collect();
    let learned = ygo_stats::learn_weights(&choices, &prior);
    Ok(LearnedWeightsView {
        weights: learned.weights,
        choices: learned.choices,
        agreement: learned.agreement,
        prior_agreement: learned.prior_agreement,
    })
}

#[tauri::command]
fn clear_board_choices(state: State<'_, AppState>) -> Result<(), String> {
    state.with_user(|u| u.clear_board_choices())
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
    /// What the handtraps take away from each board's line, by `(hand,
    /// key)`: the same whatever deck the boards are measured against.
    losses: HashMap<(usize, u64), Option<ygo_solver::probe::ExpectedLoss>>,
    /// The cheap measure of each board given it, by `(hand, key)`: its value
    /// and its score (the same whatever deck).
    cheap: HashMap<(usize, u64), (f64, ScoreView)>,
    /// The deck they were also measured against.
    vs: Option<i64>,
    /// Each board measured against the meta ([`combo_meta`]), by `(hand,
    /// key)`: the same whatever deck the scores are against.
    metas: HashMap<(usize, u64), MetaView>,
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
    /// What the handtraps are expected to take away from the line, in
    /// interruptions ([`ygo_solver::probe::ExpectedLoss::expected`]); the
    /// choke points are counted in `exposure_detail`.
    exposure: f64,
    exposure_note: String,
    exposure_detail: Option<ExposureView>,
    /// Against the deck chosen in [`combo_score`].
    matchup: Option<MatchupView>,
    /// Share of what your effects stop together that they still stop when
    /// the opponent answers the first of them it can, with Infinite
    /// Impermanence ([`ygo_solver::Measure::countered`]): none if it can
    /// answer none, or they stop nothing. Shown, not weighed.
    countered: Option<f64>,
    /// What the board stops at the end of the turn, by kind and place; none
    /// when the probe turn failed.
    interaction: Option<InteractionView>,
    /// The probe turn could not be played (the board is left out of the ranking).
    error: Option<String>,
    /// Only the cheap measure ([`ygo_solver::Prober::measure_cheap`]): the
    /// board was not among the best by it, so it has no breakers (its
    /// `resilience` is all its field), follow-up, exposure nor matchup, and
    /// is ranked by the page after the boards with the full measure.
    cheap: bool,
    /// Level 2 against the meta's most played decks ([`combo_meta`]): the
    /// best boards are ranked by it, ahead of the measure.
    meta: Option<MetaView>,
}

/// What a board stops at the end of the turn, by kind and place
/// ([`ygo_solver::InteractionProfile`]); weights all 1, so `value` is
/// interruptions + hand traps + floodgates.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InteractionView {
    value: f64,
    /// Times your effects stop something, by what they do.
    negations: u32,
    removals: u32,
    others: u32,
    /// The same times, by where the card is.
    from_field: u32,
    from_grave: u32,
    from_banished: u32,
    from_hand: u32,
    /// Effects that stop something only alone, not counted above.
    only_alone: usize,
    /// Cards you can use on the opponent's turn.
    activatable: usize,
    floodgates: usize,
    cards_in_hand: usize,
    cards_in_grave: usize,
}

fn interaction_view(p: &ygo_solver::InteractionProfile) -> InteractionView {
    let [from_field, from_grave, from_banished, from_hand] = p.by_place;
    InteractionView {
        value: p.value,
        negations: p.negations,
        removals: p.removals,
        others: p.others,
        from_field,
        from_grave,
        from_banished,
        from_hand,
        only_alone: p.shared,
        activatable: p.activatable,
        floodgates: p.floodgates,
        cards_in_hand: p.hand,
        cards_in_grave: p.grave,
    }
}

/// A board against the 3 decks of the meta played the most (L2-3,
/// [`ygo_solver::MetaScore`]).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MetaView {
    /// Σ weight · (1 − left) over the decks measured, weighed among them
    /// (0-1, higher is better); none when no deck could be measured.
    value: Option<f64>,
    /// The same weighed share of the decks' moves still made.
    left: Option<f64>,
    per_deck: Vec<MetaDeckView>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MetaDeckView {
    name: String,
    /// Its `popRank` on Master Duel Meta (share of the ranked top decks, %).
    weight: f64,
    lines: usize,
    /// Share of its moves made on an empty board: under 0.8 its lines did
    /// not play as they were found, and the deck is not counted.
    made: f64,
    /// Share of its moves still made with all your answers; none when not counted.
    left: Option<f64>,
}

fn meta_view(m: &ygo_solver::MetaScore) -> MetaView {
    MetaView {
        value: m.value,
        left: m.left,
        per_deck: m
            .decks
            .iter()
            .map(|d| MetaDeckView { name: d.name.clone(), weight: d.weight, lines: d.lines, made: d.made, left: d.left })
            .collect(),
    }
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
    /// The most cards the opponent draws using it (Maxx "C", the Mulcharmy).
    drawn: u32,
    /// Each moment tried, in the order of the line.
    moments: Vec<HandTrapMomentView>,
}

/// A moment of the line the opponent uses a handtrap at.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HandTrapMomentView {
    /// Your card being played then.
    playing: Option<u32>,
    /// It changes the field or hand you end with.
    stops: bool,
    /// Cards the opponent draws from then to the end of your turn, less those
    /// it shuffles back into the Deck.
    drawn: u32,
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
                .map(|h| RivalHandTrapView {
                    code: h.code,
                    chance: h.chance,
                    windows: h.windows,
                    stops: h.stops,
                    at: h.at.clone(),
                    drawn: h.drawn,
                    moments: h.moments.iter().map(|m| HandTrapMomentView { playing: m.playing, stops: m.stops, drawn: m.drawn }).collect(),
                })
                .collect();
        }
        Err(e) => view.error = Some(e),
    }
    view
}

/// A point where a board's line can stop, while the opponent draws from
/// Maxx "C" or a Mulcharmy ([`ygo_solver::StopPoint`]).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StopPointView {
    /// Steps of the line played before ending the turn (as `combo_explain`
    /// lists them; all of them for the whole line), and your last card then.
    steps: usize,
    playing: Option<u32>,
    /// Interruption and floodgate value of the board you end on.
    field_value: f64,
    floodgate_value: f64,
    /// Cards the opponent has drawn by the end of your turn.
    drawn: u32,
    /// Value expected in its turn, holding 6 cards plus those drawn.
    expected: f64,
}

/// Where to stop the line against one handtrap that draws.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DrawTradeOffView {
    code: u32,
    /// In the order of the line, the whole line last.
    stops: Vec<StopPointView>,
    /// Index of the stop that keeps the most for its turn.
    best: usize,
    /// The deck has no breaker the probe turn can play: the standard ones
    /// are used, assuming about 5 in its 40 cards.
    standard_breakers: bool,
}

/// Combo Lab: where to stop a board's line when the opponent (a deck of the
/// Bottega) uses Maxx "C" or a Mulcharmy at the first moment: for each of
/// its handtraps that draw, the value the board keeps in its turn at each
/// free moment of the line, the cards it draws, and the best stop. A few
/// seconds (each stop is put to the probe turn).
#[tauri::command(async)]
fn combo_stops(state: State<'_, AppState>, hand: usize, key: String, vs_deck_id: i64) -> Result<Vec<DrawTradeOffView>, String> {
    let engine = state.engine()?;
    let (config, cards, line) = {
        let job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_ref().ok_or("Nessuna ricerca da valutare")?;
        let board =
            job.boards.iter().find(|(h, b)| *h == hand && format!("{:016x}", b.key) == key).map(|(_, b)| b).ok_or("Board non trovata")?;
        let (cards, _, _) = job.hands.get(hand).ok_or("Mano non trovata")?;
        (job.config.clone(), cards.clone(), board.line.clone())
    };
    let deck = state.with_user(|u| u.get(vs_deck_id))?.ok_or("Il deck avversario non esiste più")?;
    let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
    let kit = ygo_solver::Kit::standard(&engine);
    let rival = ygo_solver::Rival::new(&engine, &list, config.seed, &kit);
    let offs = ygo_solver::draw_trade_off(&engine, &config.deck, &cards, config.seed, &line, &rival).map_err(|e| e.to_string())?;
    Ok(offs
        .into_iter()
        .map(|o| DrawTradeOffView {
            code: o.code,
            stops: o
                .stops
                .iter()
                .map(|p| StopPointView {
                    steps: p.choices,
                    playing: p.playing,
                    field_value: p.field_value,
                    floodgate_value: p.floodgate_value,
                    drawn: p.drawn,
                    expected: p.expected,
                })
                .collect(),
            best: o.best,
            standard_breakers: o.standard_breakers,
        })
        .collect())
}

/// Where a card of the rival's moves was: its hand, the field, the GY,
/// banished, its Extra Deck or its Deck.
fn move_place(loc: u8) -> &'static str {
    use ygo_engine::consts::location;
    match loc {
        location::HAND => "mano",
        location::GRAVE => "Cimitero",
        location::REMOVED => "bandita",
        location::EXTRA => "Extra Deck",
        location::DECK => "Deck",
        _ => "campo",
    }
}

/// One move of the rival: a card, how it is played and from where.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RivalPlayView {
    code: u32,
    /// "activate", "normalSummon" or "specialSummon".
    kind: &'static str,
    /// "mano", "campo", "Cimitero", "bandita", "Extra Deck", "Deck".
    place: &'static str,
}

fn rival_play(code: u32, kind: ygo_engine::ActionKind, location: u8) -> RivalPlayView {
    let kind = match kind {
        ygo_engine::ActionKind::NormalSummon => "normalSummon",
        ygo_engine::ActionKind::SpecialSummon => "specialSummon",
        _ => "activate",
    };
    RivalPlayView { code, kind, place: move_place(location) }
}

/// One of the rival deck's best lines, from one opening hand.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RivalLineView {
    hand: Vec<u32>,
    /// Its moves in order; a line that only Sets cards has none.
    moves: Vec<RivalPlayView>,
}

/// A move of a line, measured against the board.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MeasuredMoveView {
    /// The line it belongs to (index in `lines`).
    line: usize,
    #[serde(flatten)]
    play: RivalPlayView,
    /// Its share of all the moves: every line weighs the same, and every
    /// move of a line the same.
    weight: f64,
    /// "made" (made against the board with no answer), "stopped" (the
    /// board stops it with no answer: a floodgate), "lost" (lost with an
    /// earlier move the board stopped) or "notMade" (not made even on an
    /// empty board).
    state: &'static str,
}

/// One of your effects against the rival's lines.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MoveStopView {
    code: u32,
    /// "mano", "campo", "Cimitero", "bandita".
    place: &'static str,
    effect: String,
    /// The moves it stops itself (indices in `moves`).
    moves: Vec<usize>,
    /// Share of the moves it takes away at its best moment, averaged over the lines.
    coverage: f64,
    value: f64,
}

/// The board against what a deck does in its turn: its best lines played
/// again after yours (level 2 of the probe turn against a deck).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MovesView {
    /// The deck's best lines from 12 random hands.
    lines: Vec<RivalLineView>,
    /// Lines measured: those with moves (a line whose run failed is left out).
    measured: usize,
    moves: Vec<MeasuredMoveView>,
    stops: Vec<MoveStopView>,
    /// Share of the moves the lines make on an empty board (the rest the
    /// probe turn could not play again).
    made_share: f64,
    /// Share of those moves your board stops with no answer.
    floodgate_share: f64,
    /// Interruption value against these lines.
    value: f64,
    /// Share of the moves the board lets the lines make that they still
    /// make with all your answers used together.
    left: f64,
}

/// Combo Lab: the board against what a deck of the Bottega does in its
/// turn (level 2 of the probe turn against a deck). The deck's best lines
/// from 12 hands are read once per deck (10-30 seconds) and played again
/// after your turn; each of your effects is measured against their moves
/// (a few seconds a board).
#[tauri::command(async)]
fn combo_moves(state: State<'_, AppState>, hand: usize, key: String, vs_deck_id: i64) -> Result<MovesView, String> {
    let engine = state.engine()?;
    let (config, cards, line) = {
        let job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_ref().ok_or("Nessuna ricerca da valutare")?;
        let board =
            job.boards.iter().find(|(h, b)| *h == hand && format!("{:016x}", b.key) == key).map(|(_, b)| b).ok_or("Board non trovata")?;
        let (cards, _, _) = job.hands.get(hand).ok_or("Mano non trovata")?;
        (job.config.clone(), cards.clone(), board.line.clone())
    };
    let deck = state.with_user(|u| u.get(vs_deck_id))?.ok_or("Il deck avversario non esiste più")?;
    let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
    let rival_cards: Vec<u32> = list.main.iter().chain(&list.extra).copied().collect();
    let cached = state
        .rival_lines
        .lock()
        .map_err(|e| e.to_string())?
        .get(&vs_deck_id)
        .filter(|c| c.cards == rival_cards && c.seed == config.seed)
        .map(|c| c.lines.clone());
    let lines = match cached {
        Some(lines) => lines,
        None => {
            let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
            let lines = std::sync::Arc::new(ygo_solver::rival_lines(&engine, &list, config.seed, 12, 1000, threads));
            state
                .rival_lines
                .lock()
                .map_err(|e| e.to_string())?
                .insert(vs_deck_id, RivalLinesCache { cards: rival_cards, seed: config.seed, lines: lines.clone() });
            lines
        }
    };
    let kit = ygo_solver::Kit::standard(&engine);
    let x = ygo_solver::against_moves(&engine, &config.deck, &cards, config.seed, &line, &list, &lines, &kit).map_err(|e| e.to_string())?;
    Ok(MovesView {
        lines: lines
            .iter()
            .map(|l| RivalLineView { hand: l.hand.clone(), moves: l.moves.iter().map(|&(c, k, at)| rival_play(c, k, at)).collect() })
            .collect(),
        measured: x.lines,
        made_share: x.moves.iter().filter(|m| m.possible).map(|m| m.weight).sum::<f64>() + 0.0,
        moves: x
            .moves
            .iter()
            .enumerate()
            .map(|(i, m)| MeasuredMoveView {
                line: m.line,
                play: rival_play(m.code, m.kind, m.location),
                weight: m.weight,
                state: match (m.possible, m.here, x.floodgated.contains(&i)) {
                    (false, _, _) => "notMade",
                    (true, _, true) => "stopped",
                    (true, true, _) => "made",
                    (true, false, _) => "lost",
                },
            })
            .collect(),
        stops: x
            .stops
            .iter()
            .map(|s| MoveStopView {
                code: s.responder.code,
                place: place_name(s.location),
                effect: s.effect.label().to_owned(),
                moves: s.stops.clone(),
                coverage: s.coverage,
                value: s.value,
            })
            .collect(),
        floodgate_share: x.floodgate_share,
        value: x.value,
        left: x.left,
    })
}

/// One card of the board that interrupts, against removals aimed at it
/// ([`ygo_solver::Protection`]).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PieceView {
    code: u32,
    /// "mano", "campo", "Cimitero", "bandita".
    place: &'static str,
    /// What it is still usable after, in Italian: "distruggere", "bandire",
    /// "girare coperto", "negare".
    holds: Vec<String>,
    falls: Vec<String>,
}

/// Combo Lab: what each card of a board that interrupts holds against,
/// for the board open (a few seconds): the probe turn is measured again,
/// then a removal aimed at each card plays alone, a run each. The card
/// answers with its own effects only; the board's lasting protections
/// (Stardust Sifr's) count.
#[tauri::command(async)]
fn board_protection(state: State<'_, AppState>, hand: usize, key: String) -> Result<Vec<PieceView>, String> {
    let engine = state.engine()?;
    let (config, cards, line) = {
        let job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_ref().ok_or("Nessuna ricerca da valutare")?;
        let board =
            job.boards.iter().find(|(h, b)| *h == hand && format!("{:016x}", b.key) == key).map(|(_, b)| b).ok_or("Board non trovata")?;
        let (cards, _, _) = job.hands.get(hand).ok_or("Mano non trovata")?;
        (job.config.clone(), cards.clone(), board.line.clone())
    };
    let kit = ygo_solver::Kit::standard(&engine);
    let mut prober = ygo_solver::Prober::new(&engine, &config.deck, &cards, config.seed, &kit);
    let measured = prober.measure(&line).map_err(|e| e.to_string())?;
    let labels = |rs: &[ygo_solver::Removal]| rs.iter().map(|r| r.label().to_owned()).collect();
    Ok(prober
        .protection(&line, &measured.interruptions)
        .iter()
        .map(|p| PieceView { code: p.responder.code, place: place_name(p.location), holds: labels(&p.holds), falls: labels(&p.falls) })
        .collect())
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
    /// Moments of the line where a handtrap stops it ([`ygo_solver::Exposure::choke_points`]).
    choke_points: u32,
    /// For each kind of handtrap the line meets: what it takes away at the
    /// worst moment and the chance the opponent holds one.
    losses: Vec<ExposureLossView>,
    /// What Maxx "C" or a Mulcharmy gives the opponent, in interruptions,
    /// and the chance it holds one.
    maxx: f64,
    maxx_chance: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ExposureLossView {
    /// "ash", "fieldEffect", "nibiru", "droll".
    kind: &'static str,
    loss: f64,
    chance: f64,
}

fn place_name(loc: u8) -> &'static str {
    match loc {
        ygo_engine::consts::location::HAND => "mano",
        ygo_engine::consts::location::GRAVE => "Cimitero",
        ygo_engine::consts::location::REMOVED => "bandita",
        _ => "campo",
    }
}

fn choke_kind_key(kind: ygo_solver::explain::ChokeKind) -> &'static str {
    use ygo_solver::explain::ChokeKind;
    match kind {
        ChokeKind::DeckTouch => "ash",
        ChokeKind::FieldEffect => "fieldEffect",
        ChokeKind::FifthSummon => "nibiru",
        ChokeKind::SecondSearch => "droll",
    }
}

/// The cards in hand that count as resources, in words.
fn resources_note(others: usize, dead: usize) -> String {
    let hand = match others {
        0 => "nessun'altra carta in mano".to_owned(),
        1 => "1 altra carta in mano".to_owned(),
        n => format!("{n} altre carte in mano"),
    };
    match dead {
        0 => hand,
        1 => format!("{hand}; 1 handtrap che con questa board non si può usare (solo a campo vuoto): è la carta da scartare"),
        n => format!("{hand}; {n} handtrap che con questa board non si possono usare (solo a campo vuoto): sono le carte da scartare"),
    }
}

/// The follow-up in words: the starters and extenders in hand, and what
/// comes back from the GY and banishment in your next turn.
fn followup_note(starters: usize, extenders: usize, follow_up: Option<ygo_solver::FollowUp>) -> String {
    let hand = match (starters, extenders) {
        (0, 0) => "nessuno starter o estensore in mano".to_owned(),
        (s, 0) => format!("{s} starter in mano"),
        (0, 1) => "1 estensore in mano".to_owned(),
        (0, e) => format!("{e} estensori in mano"),
        (s, 1) => format!("{s} starter e 1 estensore in mano"),
        (s, e) => format!("{s} starter e {e} estensori in mano"),
    };
    match follow_up {
        Some(f) => {
            let rebuild = match f.rebuild {
                0 => "nessun effetto da Cimitero o bandite".to_owned(),
                1 => "1 effetto da Cimitero o bandite".to_owned(),
                n => format!("{n} effetti da Cimitero o bandite"),
            };
            let gained = match f.gained {
                0 => String::new(),
                1 => " (1 carta presa o Evocata nella End Phase, non contata)".to_owned(),
                n => format!(" ({n} carte prese o Evocate nella End Phase, non contate)"),
            };
            format!("{hand}; nel tuo turno dopo, dopo il board breaker peggiore: {rebuild}{gained}")
        }
        None => hand,
    }
}

/// The metrics of a board from its measure and its state.
#[allow(clippy::too_many_arguments)]
fn score_view(
    engine: &ygo_engine::Engine,
    hand: usize,
    board: &ygo_solver::Board,
    measured: Result<ygo_solver::Measure, String>,
    exposure: Option<ygo_solver::Exposure>,
    loss: Option<&ygo_solver::probe::ExpectedLoss>,
    hand_traps: &[u32],
    roles: &BTreeMap<String, Vec<u32>>,
) -> ScoreView {
    let rates = ygo_solver::probe::HandTrapRates::default();
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
    // The next turn: an effect that comes back from the GY or banishment
    // after the worst breaker weighs 0.3 of a starter. The cards gained in
    // the End Phase are shown, not weighed: with them the board pairs the
    // judges agreed on came out worse (docs/plans/classificazione-board.md, C10).
    let follow_up = measured.as_ref().ok().and_then(|m| m.follow_up);
    let followup = starters as f64 + 0.5 * extenders as f64 + 0.3 * follow_up.map_or(0.0, |f| f.rebuild as f64);
    let others = rest.iter().filter(|&&c| !has_role(c, "starter") && !has_role(c, "extender")).count();
    // The GY is in the follow-up now, by what comes back from it.
    let resources = others as f64;
    // What the line spends besides the cards of the hand (those left are
    // already counted above): Deck cards paid as a cost, Extra Deck monsters
    // gone to the GY or banishment, Life Points.
    let extra = |c: u32| db.get(c).is_some_and(|c| c.type_ & ygo_engine::consts::card_type::EXTRA != 0);
    let extra_spent = me.grave.iter().filter(|&&c| extra(c)).count() + me.banished.iter().filter(|&&(c, _)| extra(c)).count();
    let cost = exposure.map_or(0.0, |x| 0.05 * x.deck_cost as f64 + x.lp_paid as f64 / 4000.0) + 0.1 * extra_spent as f64;
    let mut view = ScoreView {
        hand,
        key: format!("{:016x}", board.key),
        interruptions: 0.0,
        hand_traps: 0.0,
        floodgates: 0.0,
        resilience: 0.0,
        resources,
        followup,
        cost,
        details: Vec::new(),
        floodgate_list: Vec::new(),
        breakers: Vec::new(),
        resources_note: resources_note(others, dead),
        followup_note: followup_note(starters, extenders, follow_up),
        exposure: loss.map_or(0.0, |l| l.expected),
        exposure_detail: exposure.map(|x| ExposureView {
            ash: x.ash,
            on_field: x.on_field,
            summons: x.summons,
            special_summons: x.special_summons,
            searches: x.searches,
            choke_points: x.choke_points(),
            losses: loss.map_or_else(Vec::new, |l| {
                l.losses
                    .iter()
                    .map(|&(kind, value)| ExposureLossView { kind: choke_kind_key(kind), loss: value, chance: rates.chance(kind) })
                    .collect()
            }),
            maxx: loss.map_or(0.0, |l| l.maxx),
            maxx_chance: rates.special_summons,
        }),
        exposure_note: exposure.map_or_else(String::new, |x| {
            let mut parts = Vec::new();
            if let Some(l) = loss {
                parts.push(format!("perdita attesa per le handtrap {:.2} interruzioni", l.expected));
            }
            parts.push(format!("{} effetti che Ash Blossom nega", x.ash));
            parts.push(format!("{} effetti di mostri sul campo (Veiler, Impermanence)", x.on_field));
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
        countered: None,
        interaction: None,
        error: None,
        cheap: false,
        meta: None,
    };
    match measured {
        Ok(m) => {
            view.countered = m.countered;
            view.interaction = Some(interaction_view(&ygo_solver::InteractionProfile::of(&m, me.hand.len(), me.grave.len())));
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
    goal_soft: usize,
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
        goal_soft: b.goal_soft,
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
    /// "any" (your field), "extra" (Extra Monster Zone), "main", "monster",
    /// "faceup", "set", "fieldzone", "grave", "banished" or "hand".
    zone: String,
    /// Required (the default); false: only preferred. Goals saved before
    /// it have none.
    #[serde(default = "required")]
    hard: bool,
}

fn required() -> bool {
    true
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComboGoal {
    cards: Vec<GoalCardRequest>,
    /// All the hard ones (else one is enough).
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
    // A board already measured stays in the list when the search goes on,
    // even when it is no longer among the best kept by the estimate: more
    // time never loses the best board measured (docs/plans/monotonia-budget.md).
    let previous = std::mem::take(&mut job.boards);
    let measured = |h: usize, key: u64| job.scores.contains_key(&(h, key)) || job.cheap.contains_key(&(h, key));
    let mut boards = Vec::new();
    for (i, r) in results.iter().enumerate() {
        let listed: HashSet<u64> = r.boards.iter().map(|b| b.key).collect();
        boards.extend(r.boards.iter().map(|b| (i, b.clone())));
        boards.extend(previous.iter().filter(|(h, b)| *h == i && !listed.contains(&b.key) && measured(i, b.key)).cloned());
    }
    job.boards = boards;
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
            "monster" => ygo_solver::GoalZone::Monster,
            "faceup" => ygo_solver::GoalZone::FaceUp,
            "set" => ygo_solver::GoalZone::Set,
            "fieldzone" => ygo_solver::GoalZone::FieldZone,
            "grave" => ygo_solver::GoalZone::Grave,
            "banished" => ygo_solver::GoalZone::Banished,
            "hand" => ygo_solver::GoalZone::Hand,
            _ => ygo_solver::GoalZone::Any,
        };
        let cards = goal.cards.iter().map(|c| ygo_solver::GoalCard { code: c.code, zone: zone(&c.zone), hard: c.hard }).collect();
        config.goal = ygo_solver::Goal { cards, all: goal.all, first_only: goal.first };
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
        losses: HashMap::new(),
        cheap: HashMap::new(),
        vs: None,
        metas: HashMap::new(),
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
    /// "toGrave" (from the Deck to the GY, a Fusion Material included),
    /// "recover" (from the GY or banished back to the hand), "material"
    /// (`from` is a material of `to`), "set" (from the Deck to the field:
    /// Set, or placed in a Spell & Trap Zone or the Field Zone).
    kind: String,
    /// Where `to` comes from: "deck", "extra", "grave", "banished", "hand".
    zone: String,
    /// The engine offered it: the effect can pick that card.
    possible: bool,
    /// Share of the Combo Lab's best lines that use it, 0 to 1.
    used: f64,
    /// The card's text names it; alone (not `possible`), an effect the lines found never use.
    text: bool,
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
    /// Chance that an opening hand of 5 brings it into play: holds it, or its
    /// best line adds it to the hand or puts it on the field ([`ygo_solver::LinkCard`]).
    chance: f64,
    /// `chance` is measured on the hands searched, not the draft's.
    measured: bool,
    /// Cards an opening hand reaches on average that are lost when its effects are negated.
    key: f64,
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
    Ok(read_links(&app, &state, deck_id, None)?.1)
}

/// The map of `deck_id`, from the cache while its cards are the same. A
/// changed deck reuses what was searched for its earlier version, or else
/// for `like` (another version of it).
fn read_links(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
    deck_id: i64,
    like: Option<i64>,
) -> Result<(ygo_solver::DeckLinks, DeckLinksView), String> {
    let engine = state.engine()?;
    let deck = state.with_user(|u| u.get(deck_id))?.ok_or("Il deck non esiste più")?;
    let cards: Vec<u32> = deck.main.iter().chain(&deck.extra).copied().collect();
    let previous = {
        let cache = state.links.lock().map_err(|e| e.to_string())?;
        match cache.get(&deck_id) {
            Some(cached) if cached.cards == cards => return Ok((cached.map.clone(), cached.view.clone())),
            Some(cached) => Some(cached.work.clone()),
            None => like.and_then(|id| cache.get(&id)).map(|c| c.work.clone()),
        }
    };
    let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: Vec::new() };
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let budget = ygo_solver::LinksBudget { threads, ..ygo_solver::LinksBudget::default() };
    // The map so far with each step: from the text at once, then hand by hand.
    let progress = |done, total, map: &ygo_solver::DeckLinks| {
        let _ = app.emit("deck-links-progress", LinksProgressView { deck_id, done, total, map: links_view(deck_id, map) });
    };
    let (map, work) = ygo_solver::deck_links_again(&engine, &list, 1, budget, &AtomicBool::new(false), progress, previous.as_ref());
    let view = links_view(deck_id, &map);
    state.links.lock().map_err(|e| e.to_string())?.insert(deck_id, LinksCache { cards, map: map.clone(), view: view.clone(), work });
    Ok((map, view))
}

/// A card in both versions compared: its numbers before and after.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LinkCardChangeView {
    code: u32,
    chance_before: f64,
    chance_after: f64,
    key_before: f64,
    key_after: f64,
    reach_all_before: u32,
    reach_all_after: u32,
}

/// Two versions of a deck on the links map: both maps and what changes.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LinksComparisonView {
    before: DeckLinksView,
    after: DeckLinksView,
    /// Links only in the second deck, only in the first.
    gained: Vec<CardLinkView>,
    lost: Vec<CardLinkView>,
    /// Cards in both whose chance, key or reach changed, the most changed first.
    changed: Vec<LinkCardChangeView>,
    /// Cards put in and taken out.
    added: Vec<u32>,
    removed: Vec<u32>,
    /// Cards an opening hand reaches on average, with the first deck and with the second.
    reached_before: f64,
    reached_after: f64,
}

/// Laboratorio, "Collegamenti": what changes on the map from one deck to
/// another (two versions of a deck). The second is read reusing what was
/// searched for the first: a few cards changed, a few seconds more.
#[tauri::command(async)]
fn deck_links_compare(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    deck_id_a: i64,
    deck_id_b: i64,
) -> Result<LinksComparisonView, String> {
    let (old, before) = read_links(&app, &state, deck_id_a, None)?;
    let (new, after) = read_links(&app, &state, deck_id_b, Some(deck_id_a))?;
    let diff = ygo_solver::compare_links(&old, &new);
    Ok(LinksComparisonView {
        before,
        after,
        gained: diff.gained.iter().map(link_view).collect(),
        lost: diff.lost.iter().map(link_view).collect(),
        changed: diff
            .changed
            .iter()
            .map(|c| LinkCardChangeView {
                code: c.code,
                chance_before: c.chance.0,
                chance_after: c.chance.1,
                key_before: c.key.0,
                key_after: c.key.1,
                reach_all_before: c.reach_all.0,
                reach_all_after: c.reach_all.1,
            })
            .collect(),
        added: diff.added,
        removed: diff.removed,
        reached_before: diff.reached.0,
        reached_after: diff.reached.1,
    })
}

/// The map as far as it is read: `done` hands of `total` searched (0: the
/// draft from the cards' text).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LinksProgressView {
    deck_id: i64,
    done: usize,
    total: usize,
    map: DeckLinksView,
}

fn link_view(l: &ygo_solver::CardLink) -> CardLinkView {
    CardLinkView {
        from: l.from,
        to: l.to,
        kind: l.kind.key().to_owned(),
        zone: l.zone.key().to_owned(),
        possible: l.possible,
        used: l.used,
        text: l.text,
    }
}

fn links_view(deck_id: i64, map: &ygo_solver::DeckLinks) -> DeckLinksView {
    DeckLinksView {
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
                measured: c.measured,
                key: c.key,
            })
            .collect(),
        links: map.links.iter().map(link_view).collect(),
        hands: map.hands,
        lines: map.lines,
    }
}

/// Boards of a precise hand put to the cheap measure after a search, the
/// first in the search's order, and of those the best by it put to the full
/// one, which goes on from the cheap one (B14, `docs/plans/velocita-ricerca.md`):
/// the same best board as measuring the 30 in full, in less time. The cheap
/// measure tries each response at its first 8 windows and counts its uses
/// once (with 2 it misses responses that stop something only later), and
/// sees your next turn when nothing is broken. "Card + rest" puts all the
/// boards kept to the full measure.
const CHEAP_BOARDS: usize = 30;
const SCORED_BOARDS: usize = 15;
/// What the cheap measure tries.
const CHEAP_MEASURE: ygo_solver::probe::Cheap = ygo_solver::probe::Cheap { windows: 8, use_rounds: 1 };

/// The value a cheap measure ranks the boards by: field, hand, floodgates
/// and the effects to rebuild with on your next turn, with no breaker
/// played. The floodgates weigh 0.4, as in the "Bilanciato" profile, so the
/// cut to the full measure ranks by the weights of the score.
fn cheap_value(m: &ygo_solver::Measure) -> f64 {
    let rebuild = m.follow_up.map_or(0.0, |f| f.rebuild as f64);
    m.field_value + 0.9 * m.hand_value + 0.4 * m.floodgate_value + 0.4 * 0.3 * rebuild
}

/// `work` on each board, on all cores, until `cancel`; one prober per hand
/// and thread (it keeps the empty board of that hand).
fn on_all_cores<'a, R: Send>(
    boards: &[(usize, ygo_solver::Board)],
    cancel: &AtomicBool,
    prober: impl Fn(usize) -> ygo_solver::Prober<'a> + Sync,
    work: impl Fn(&mut ygo_solver::Prober<'a>, usize, &ygo_solver::Board) -> R + Sync,
) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut probers: HashMap<usize, ygo_solver::Prober> = HashMap::new();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((hand, board)) = boards.get(i) else { break };
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let prober = probers.entry(*hand).or_insert_with(|| prober(*hand));
                    let r = work(prober, *hand, board);
                    if let Ok(mut out) = results.lock() {
                        out.push(r);
                    }
                }
            });
        }
    });
    results.into_inner().unwrap_or_default()
}

/// Puts the best boards of the last search to the probe turn, on all cores,
/// sending each result as a `combo-score` event; boards already measured
/// are not measured again. A precise hand goes in steps: the cheap measure
/// of the first [`CHEAP_BOARDS`], then the full one of the best
/// [`SCORED_BOARDS`] by it; the others keep the cheap score
/// ([`ScoreView::cheap`]), which the page ranks after the full ones. With `vs_deck_id` (a deck of the Bottega) each
/// board with the full measure is also measured against that deck's
/// breakers and handtraps (another deck than last time measures all boards
/// again). Returns how many boards have a score.
#[tauri::command(async)]
fn combo_score(app: tauri::AppHandle, state: State<'_, AppState>, vs_deck_id: Option<i64>) -> Result<usize, String> {
    let engine = state.engine()?;
    // What is needed, copied out so the search state stays free meanwhile.
    let (config, hands, mut candidates, cheap, roles, losses) = {
        let mut job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_mut().ok_or("Nessuna ricerca da valutare")?;
        if job.vs != vs_deck_id {
            job.scores.clear();
            job.vs = vs_deck_id;
        }
        let staged = job.hands.len() == 1;
        // One board per look: boards that differ only in the Graveyard or the banished cards score the same.
        // A look already measured is that board.
        let full = |h: usize, key: u64| job.scores.get(&(h, key)).is_some_and(|v| !v.cheap);
        let mut measured: HashMap<(usize, u64), &ygo_solver::Board> = HashMap::new();
        for (h, b) in &job.boards {
            if full(*h, b.key) {
                measured.entry((*h, ygo_solver::look_key(&b.state))).or_insert(b);
            }
        }
        let mut looks: HashSet<(usize, u64)> = HashSet::new();
        let candidates: Vec<(usize, ygo_solver::Board)> = job
            .boards
            .iter()
            .filter_map(|(h, b)| {
                let look = (*h, ygo_solver::look_key(&b.state));
                // "Card + rest": only the looks not measured yet, as before.
                if !looks.insert(look) || !staged && measured.contains_key(&look) {
                    return None;
                }
                Some((*h, measured.get(&look).map_or(b, |m| *m).clone()))
            })
            .enumerate()
            // Past the first ones, the boards with the cheap measure already
            // (kept from before "Continua") cost nothing and stay in the
            // ranking: the full measure never leaves out one it chose before.
            .filter(|(i, (h, b))| !staged || *i < CHEAP_BOARDS || job.cheap.contains_key(&(*h, b.key)))
            .map(|(_, c)| c)
            .collect();
        let cheap: Option<HashMap<(usize, u64), f64>> = staged.then(|| job.cheap.iter().map(|(k, (v, _))| (*k, *v)).collect());
        (
            job.config.clone(),
            job.hands.iter().map(|(h, _, _)| h.clone()).collect::<Vec<_>>(),
            candidates,
            cheap,
            job.roles.clone(),
            job.losses.clone(),
        )
    };
    state.combo_cancel.store(false, Ordering::Relaxed);
    let kit = ygo_solver::Kit::standard(&engine);
    let new_prober = |hand: usize| ygo_solver::Prober::new(&engine, &config.deck, &hands[hand], config.seed, &kit);

    // The cheap measure of the candidates without one, then the best by it.
    let mut cheap_scores: Vec<((usize, u64), (f64, ScoreView))> = Vec::new();
    // What each cheap measure played, for the full one to go on from it.
    let partials: Mutex<HashMap<(usize, u64), ygo_solver::probe::Partial>> = Mutex::new(HashMap::new());
    let todo: Vec<(usize, ygo_solver::Board)> = match &cheap {
        Some(known) => {
            let missing: Vec<(usize, ygo_solver::Board)> =
                candidates.iter().filter(|(h, b)| !known.contains_key(&(*h, b.key))).cloned().collect();
            cheap_scores = on_all_cores(&missing, &state.combo_cancel, new_prober, |prober, hand, board| {
                let measured = prober.measure_cheap(&board.line, CHEAP_MEASURE).map_err(|e| e.to_string()).map(|(m, partial)| {
                    if let Ok(mut p) = partials.lock() {
                        p.insert((hand, board.key), partial);
                    }
                    m
                });
                let value = measured.as_ref().map_or(f64::NEG_INFINITY, cheap_value);
                let mut view = score_view(&engine, hand, board, measured, None, None, &config.hand_traps, &roles);
                view.cheap = true;
                let _ = app.emit("combo-score", view.clone());
                ((hand, board.key), (value, view))
            });
            if state.combo_cancel.load(Ordering::Relaxed) {
                // A ranking with boards missing would put the wrong ones to the full measure.
                Vec::new()
            } else {
                let value = |h: usize, key: u64| {
                    known.get(&(h, key)).or_else(|| cheap_scores.iter().find(|(k, _)| *k == (h, key)).map(|(_, (v, _))| v)).copied()
                };
                let mut ranked: Vec<&(usize, ygo_solver::Board)> = candidates.iter().collect();
                // Stable: at equal value the search's order.
                ranked.sort_by(|(ha, a), (hb, b)| {
                    value(*hb, b.key).unwrap_or(f64::NEG_INFINITY).total_cmp(&value(*ha, a.key).unwrap_or(f64::NEG_INFINITY))
                });
                ranked.into_iter().take(SCORED_BOARDS).cloned().collect()
            }
        }
        // Not needed after: "card + rest" keeps no cheap scores.
        None => std::mem::take(&mut candidates),
    };
    // Boards with the full measure already are not measured again.
    let todo: Vec<(usize, ygo_solver::Board)> = {
        let job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_ref().ok_or("La ricerca è cambiata")?;
        todo.into_iter().filter(|(h, b)| !job.scores.get(&(*h, b.key)).is_some_and(|v| !v.cheap)).collect()
    };

    let rival = match vs_deck_id {
        Some(id) => {
            let deck = state.with_user(|u| u.get(id))?.ok_or("Il deck avversario non esiste più")?;
            let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
            Some((id, deck.name.clone(), ygo_solver::Rival::new(&engine, &list, config.seed, &kit)))
        }
        None => None,
    };
    let results = on_all_cores(&todo, &state.combo_cancel, new_prober, |prober, hand, board| {
        let points =
            ygo_solver::explain::exposure_points(&engine, &config.deck, &hands[hand], config.seed, config.opponent.as_ref(), &board.line)
                .ok();
        let exposure = points.as_ref().map(|(x, _)| *x);
        // Every board ranked has it: one without would rank higher.
        let loss = match losses.get(&(hand, board.key)) {
            Some(known) => known.clone(),
            None => points.as_ref().and_then(|(x, points)| {
                let rates = ygo_solver::probe::HandTrapRates::default();
                prober.expected_loss(&board.line, points, x.special_summons, &rates).ok()
            }),
        };
        let partial = partials.lock().ok().and_then(|mut p| p.remove(&(hand, board.key)));
        let measured = match partial {
            Some(partial) => prober.measure_resumed(&board.line, partial),
            None => prober.measure(&board.line),
        }
        .map_err(|e| e.to_string());
        let matchup = rival.as_ref().map(|(id, name, rival)| {
            let m = measured.as_ref().map_err(String::clone).and_then(|m| {
                ygo_solver::matchup(&engine, &config.deck, &hands[hand], config.seed, &board.line, m, rival).map_err(|e| e.to_string())
            });
            matchup_view(*id, name, m)
        });
        let mut view = score_view(&engine, hand, board, measured, exposure, loss.as_ref(), &config.hand_traps, &roles);
        view.matchup = matchup;
        let _ = app.emit("combo-score", view.clone());
        ((hand, board.key), view, loss)
    });
    let mut job = state.combo.lock().map_err(|e| e.to_string())?;
    let job = job.as_mut().ok_or("La ricerca è cambiata")?;
    for (key, entry) in cheap_scores {
        job.cheap.insert(key, entry);
    }
    for (key, mut view, loss) in results {
        // A board already measured against the meta keeps it (sent again with it).
        if let Some(meta) = job.metas.get(&key) {
            view.meta = Some(meta.clone());
            let _ = app.emit("combo-score", view.clone());
        }
        job.scores.insert(key, view);
        job.losses.insert(key, loss);
    }
    // The candidates left out of the full measure keep the cheap score
    // (sent again after a change of deck, which clears the scores).
    if cheap.is_some() {
        for (h, b) in &candidates {
            if let (None, Some((_, view))) = (job.scores.get(&(*h, b.key)), job.cheap.get(&(*h, b.key))) {
                let view = view.clone();
                let _ = app.emit("combo-score", view.clone());
                job.scores.insert((*h, b.key), view);
            }
        }
    }
    Ok(job.scores.len())
}

/// Boards of the last search measured against the meta when none are named.
const META_BOARDS: usize = 4;
/// Decks of the meta they are measured against: the most played.
const META_DECKS: usize = 3;

/// A board of the last search.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoardRef {
    hand: usize,
    key: String,
}

/// The value of a score with the "Bilanciato" profile of the page. The
/// resilience weighs 0 there (docs/plans/ricerca-esperimenti.md, section 7).
fn balanced(v: &ScoreView) -> f64 {
    v.interruptions + 0.9 * v.hand_traps + 0.4 * v.floodgates + 0.4 * v.followup + 0.1 * v.resources - 0.1 * v.cost - 0.5 * v.exposure
}

/// The last step of the ranking (L2-3): the boards named, or the best
/// [`META_BOARDS`] with the full measure by the "Bilanciato" profile, played
/// at level 2 against the [`META_DECKS`] decks of the meta played the most
/// (read from Master Duel Meta, chosen by how much they are played), on all
/// cores. Each score is sent again as a `combo-score` event with its
/// [`ScoreView::meta`]. The meta and its decks' lines are read once per
/// session (the lines take 10-25 s a deck), then about 20 s a board.
/// Returns how many boards have it.
#[tauri::command]
async fn combo_meta(app: tauri::AppHandle, state: State<'_, AppState>, boards: Option<Vec<BoardRef>>) -> Result<usize, String> {
    let decks = meta_decks(&state).await?;
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app2.state::<AppState>();
        meta_final(&app2, &state, &decks, boards)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The decks of the meta, read from Master Duel Meta once per session.
async fn meta_decks(state: &State<'_, AppState>) -> Result<Vec<ygo_solver::MetaDeck>, String> {
    let known = state.meta.lock().map_err(|e| e.to_string())?.as_ref().map(|m| m.decks.clone());
    Ok(match known {
        Some(decks) => decks,
        None => {
            let (types, lists) =
                ygo_data::meta::fetch(&state.client).await.map_err(|e| format!("Impossibile leggere il meta da Master Duel Meta: {e}"))?;
            let lists = state.with_store(|s| ygo_data::meta::meta_lists(&types, &lists, s))?;
            let decks: Vec<ygo_solver::MetaDeck> = lists
                .into_iter()
                .map(|m| ygo_solver::MetaDeck {
                    name: m.name,
                    weight: m.weight,
                    list: ygo_engine::DeckList { main: m.main, extra: m.extra, side: Vec::new() },
                })
                .collect();
            if decks.is_empty() {
                return Err("Master Duel Meta non ha dato nessun deck del meta".into());
            }
            *state.meta.lock().map_err(|e| e.to_string())? = Some(MetaCache { decks: decks.clone(), rivals: None });
            decks
        }
    })
}

/// The best lines of the `count` decks of the meta played the most, for
/// `seed`: from the cache when it has as many for that seed, else read
/// (10-25 s a deck) and kept, the lines already read reused.
fn meta_rivals_for(
    state: &AppState,
    engine: &Engine,
    decks: &[ygo_solver::MetaDeck],
    count: usize,
    seed: u64,
    threads: usize,
) -> Result<Vec<ygo_solver::MetaRival>, String> {
    let wanted = ygo_solver::most_played(decks, count);
    let known = state.meta.lock().map_err(|e| e.to_string())?.as_ref().and_then(|m| m.rivals.clone());
    let known: Vec<ygo_solver::MetaRival> = known.filter(|(s, _)| *s == seed).map(|(_, r)| r.as_ref().clone()).unwrap_or_default();
    let mut rivals = Vec::new();
    for deck in &wanted {
        match known.iter().find(|r| r.deck.name == deck.name && r.deck.list.main == deck.list.main) {
            Some(r) => rivals.push(r.clone()),
            None => rivals.extend(ygo_solver::meta_rivals(engine, std::slice::from_ref(deck), seed, threads)),
        }
    }
    if rivals.len() > known.len() {
        if let Some(m) = state.meta.lock().map_err(|e| e.to_string())?.as_mut() {
            m.rivals = Some((seed, std::sync::Arc::new(rivals.clone())));
        }
    }
    Ok(rivals)
}

/// Decks of the meta the tech table measures against: the most played.
const TECH_DECKS: usize = 6;
/// Seed of the meta's lines for the tech table: the Combo Lab's default, so
/// that both reuse the same lines.
const TECH_SEED: u64 = 1;

/// The tech table of a deck ([`ygo_solver::TechTable`]).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TechTableView {
    deck_id: i64,
    decks: Vec<TechDeckView>,
    rows: Vec<TechRowView>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TechDeckView {
    name: String,
    /// Its `popRank` on Master Duel Meta (share of the ranked top decks, %).
    weight: f64,
    /// Lines found, and those that play as found with no handtrap (counted).
    lines: usize,
    measured: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TechRowView {
    code: u32,
    /// Copies in the Main and Side Deck.
    main: u32,
    side: u32,
    /// Chance of at least one among the 5 cards held going second.
    chance: f64,
    /// Share of the meta's lines it stops (changes how they end), each deck
    /// weighed by its weight (0-1).
    value: f64,
    /// The same weighed share of the field and hand the lines end with that
    /// it takes at its best moment (0-1): the rows are ordered by it.
    lost: f64,
    /// Against each deck of `decks`, in its order.
    per_deck: Vec<TechAgainstView>,
    /// Moves of a line played before it stops it, averaged; none when it stops none.
    step: Option<f64>,
    /// Cards of the lines it answers when it stops them, the most often first.
    at: Vec<TechAtView>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TechAgainstView {
    /// Lines counted where it can be used, and those it stops.
    usable: usize,
    stopped: usize,
    /// Share of the field and hand its lines end with that it takes, averaged.
    lost: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TechAtView {
    code: u32,
    lines: u32,
}

/// Laboratorio, "Contro il meta": each handtrap of the deck's Main and Side
/// against the best lines of the [`TECH_DECKS`] decks of the meta played the
/// most (read from Master Duel Meta), played again with it in hand: the
/// share of the lines it stops, weighed by how much each deck is played.
/// The meta's lines are read once per session (10-25 s a deck); then about
/// half a minute a deck of yours. The last table is kept while its deck's
/// cards are the same.
#[tauri::command]
async fn tech_table(app: tauri::AppHandle, state: State<'_, AppState>, deck_id: i64) -> Result<TechTableView, String> {
    let deck = state.with_user(|u| u.get(deck_id))?.ok_or("Il deck non esiste più")?;
    let cards: Vec<u32> = deck.main.iter().chain(&deck.extra).chain(&deck.side).copied().collect();
    if let Some((known, view)) = state.tech.lock().map_err(|e| e.to_string())?.as_ref() {
        if *known == cards {
            return Ok(TechTableView { deck_id, ..view.clone() });
        }
    }
    let decks = meta_decks(&state).await?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let engine = state.engine()?;
        let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let rivals = meta_rivals_for(&state, &engine, &decks, TECH_DECKS, TECH_SEED, threads)?;
        let kit = ygo_solver::Kit::standard(&engine);
        let table = ygo_solver::tech_table(&engine, &list, &rivals, TECH_SEED, &kit, threads);
        let view = TechTableView {
            deck_id,
            decks: table
                .decks
                .iter()
                .map(|d| TechDeckView { name: d.name.clone(), weight: d.weight, lines: d.lines, measured: d.measured })
                .collect(),
            rows: table
                .rows
                .iter()
                .map(|r| TechRowView {
                    code: r.code,
                    main: r.main,
                    side: r.side,
                    chance: r.chance,
                    value: r.value,
                    lost: r.lost,
                    per_deck: r.decks.iter().map(|a| TechAgainstView { usable: a.usable, stopped: a.stopped, lost: a.lost }).collect(),
                    step: r.step,
                    at: r.at.iter().map(|&(code, lines)| TechAtView { code, lines }).collect(),
                })
                .collect(),
        };
        *state.tech.lock().map_err(|e| e.to_string())? = Some((cards, view.clone()));
        Ok(view)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// A deck of the meta to go second against.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecondOpponentView {
    name: String,
    /// Its `popRank` on Master Duel Meta (share of the ranked top decks, %).
    weight: f64,
}

/// The most played decks of the meta, to pick the opponent of "Per secondo".
#[tauri::command]
async fn second_opponents(state: State<'_, AppState>) -> Result<Vec<SecondOpponentView>, String> {
    let decks = meta_decks(&state).await?;
    Ok(ygo_solver::most_played(&decks, SECOND_OPPONENTS)
        .into_iter()
        .map(|d| SecondOpponentView { name: d.name, weight: d.weight })
        .collect())
}

/// Decks offered as the opponent of "Per secondo".
const SECOND_OPPONENTS: usize = 12;
/// Seed of "Per secondo": the Combo Lab's default.
const SECOND_SEED: u64 = 1;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecondRequest {
    deck_id: i64,
    /// Name of the opponent's deck, among [`second_opponents`].
    vs: String,
    /// Your hands of 6, and the nodes the search spends on each.
    hands: usize,
    nodes: u64,
}

/// The opponent's board: the best of its hands, and what it answers with.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecondRivalView {
    hand: Vec<u32>,
    /// Cards on its field, each with what the board loses without it.
    worth: Vec<SecondWorthView>,
    /// Cards whose effects it answers your turn with.
    responders: Vec<u32>,
    value: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecondWorthView {
    code: u32,
    worth: f64,
}

/// One hand of yours against the board ([`ygo_solver::BrokenBoard`]).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecondHandView {
    hand: Vec<u32>,
    /// Breakers of your deck in the hand.
    breakers: usize,
    /// Share of the board's value left after your turn, less the Life Points taken (0-1; 1 when the hand does nothing).
    share: f64,
    /// Opponent's cards still on its field.
    kept: Vec<u32>,
    /// Life Points the opponent lost.
    damage: i32,
    /// The line that leaves the opponent the least, step by step; none when the search found no line.
    line: Option<Vec<String>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecondResultView {
    deck_id: i64,
    vs: String,
    rival: SecondRivalView,
    /// Cards of your deck the engine can play to break a board.
    breakers: Vec<u32>,
    hands: Vec<SecondHandView>,
    /// Hands asked for: less than that when stopped.
    asked: usize,
    seconds: f64,
    /// Whether the run was stopped before the last hand.
    stopped: bool,
}

/// Progress of [`second_run`]: the result so far, `rival` first.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecondProgressView {
    deck_id: i64,
    vs: String,
    result: Option<SecondResultView>,
}

/// "Per secondo": the best board of a deck of the meta from random hands of
/// 5, then random hands of 6 of yours play their turn against it, with its
/// answers and your Battle Phase ([`ygo_solver::break_board`]). Each hand is
/// sent as it ends (`second-progress`); `second_cancel` stops after the hand
/// being played.
#[tauri::command]
async fn second_run(app: tauri::AppHandle, state: State<'_, AppState>, request: SecondRequest) -> Result<SecondResultView, String> {
    let deck = state.with_user(|u| u.get(request.deck_id))?.ok_or("Il deck non esiste più")?;
    let decks = meta_decks(&state).await?;
    let theirs = decks.iter().find(|d| d.name == request.vs).ok_or("Il deck avversario non è più nel meta")?.list.clone();
    state.second_cancel.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let engine = state.engine()?;
        let mine = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let kit = ygo_solver::Kit::standard(&engine);
        let started = std::time::Instant::now();
        let rival = ygo_solver::rival_board(&engine, &theirs, SECOND_SEED, 8, request.nodes, threads, &kit)
            .ok_or("Nessuna mano del deck avversario fa una board")?;
        let breakers = ygo_solver::Rival::new(&engine, &mine, SECOND_SEED, &kit).breakers().to_vec();
        let db = engine.cards();
        let mut view = SecondResultView {
            deck_id: request.deck_id,
            vs: request.vs.clone(),
            rival: SecondRivalView {
                hand: rival.hand.clone(),
                worth: rival.worth.iter().map(|&(code, worth)| SecondWorthView { code, worth }).collect(),
                responders: rival.responders.iter().map(|r| r.0).collect(),
                value: rival.value,
            },
            breakers: breakers.clone(),
            hands: Vec::new(),
            asked: request.hands,
            seconds: started.elapsed().as_secs_f64(),
            stopped: false,
        };
        let progress = |view: &SecondResultView| {
            let _ =
                app.emit("second-progress", SecondProgressView { deck_id: view.deck_id, vs: view.vs.clone(), result: Some(view.clone()) });
        };
        progress(&view);
        let config = ygo_solver::BreakConfig { seed: SECOND_SEED, nodes: request.nodes, threads, top: 5, answers: true };
        for hand in ygo_solver::second::random_hands(&mine.main, 6, request.hands, SECOND_SEED) {
            if state.second_cancel.load(Ordering::Relaxed) {
                view.stopped = true;
                break;
            }
            let broken = ygo_solver::break_board(&engine, &mine, &theirs, &hand, &rival, &kit, &config).map_err(|e| e.to_string())?;
            let count = hand.iter().filter(|c| breakers.iter().any(|b| db.canonical_code(*b) == db.canonical_code(**c))).count();
            view.hands.push(SecondHandView {
                hand,
                breakers: count,
                share: broken.share,
                kept: broken.kept,
                damage: broken.damage,
                line: broken.line,
            });
            view.seconds = started.elapsed().as_secs_f64();
            progress(&view);
        }
        view.seconds = started.elapsed().as_secs_f64();
        Ok(view)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn second_cancel(state: State<'_, AppState>) {
    state.second_cancel.store(true, Ordering::Relaxed);
}

/// Seed of the engine Monte Carlo: the Combo Lab's default.
const MC_SEED: u64 = 1;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct McRequest {
    deck_id: i64,
    /// Echoed on every progress event, so the page drops those of an older run.
    request_id: u64,
    /// Hands of 5 drawn from the Main Deck, and the nodes the search spends on each.
    hands: usize,
    nodes: u64,
    /// One copy of `out` turned into `into`, measured on the same hands.
    swap: Option<McSwapRequest>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct McSwapRequest {
    out: u32,
    into: u32,
}

/// One hand played by the engine.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McHandView {
    hand: Vec<u32>,
    /// Value of the best board the hand reached (0 when none).
    value: f64,
    /// Cards of the hand the best line used.
    used: Vec<u32>,
}

/// A deck measured: P(board ≥ threshold) and what it cost.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McDeckView {
    /// Share of the hands that reached the threshold, corrected by the odds of the cards (0-1), with its 95% half width.
    p: f64,
    p_half: f64,
    /// The share of the sample alone, and its Wilson interval.
    p_sample: f64,
    low: f64,
    high: f64,
    mean: f64,
    mean_half: f64,
    hands: Vec<McHandView>,
    searched: usize,
    seconds: f64,
}

/// A swap of a copy measured on the same hands as the deck.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McSwapView {
    out: u32,
    into: u32,
    deck: McDeckView,
    /// Change of P with the half width of its 95% interval, on the same hands and corrected.
    dp: f64,
    dp_half: f64,
    dmean: f64,
    dmean_half: f64,
    /// Hands that changed with the swap.
    changed: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McResultView {
    deck_id: i64,
    request_id: u64,
    /// Value a board must reach: the median of the deck's hands.
    threshold: f64,
    deck: McDeckView,
    swap: Option<McSwapView>,
    /// Stopped before the last hand: the numbers use the hands done.
    stopped: bool,
    seconds: f64,
}

/// A hand finished, while `mc_run` goes on (`mc-progress`).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McProgressView {
    deck_id: i64,
    request_id: u64,
    /// "deck" or "swap".
    phase: &'static str,
    done: usize,
    total: usize,
    hand: McHandView,
}

fn mc_hand_view(h: &ygo_solver::mc_run::McHand) -> McHandView {
    McHandView { hand: h.hand.clone(), value: h.value, used: h.used.clone() }
}

fn mc_deck_view(d: &ygo_solver::mc_run::McDeck) -> McDeckView {
    let e = &d.estimate;
    McDeckView {
        p: e.p_adjusted,
        p_half: e.p_adjusted_half,
        p_sample: e.p,
        low: e.low,
        high: e.high,
        mean: e.mean,
        mean_half: e.mean_half,
        hands: d.hands.iter().map(mc_hand_view).collect(),
        searched: d.searched,
        seconds: d.seconds,
    }
}

/// The Monte Carlo played by the engine: hands of 5 of the deck, each
/// searched by the Combo Lab, and the share whose best board reaches the
/// median of the deck ([`ygo_solver::mc_run::run`]). Each hand is sent as it
/// ends (`mc-progress`); `mc_cancel` stops it and the call returns the hands done.
#[tauri::command]
async fn mc_run(app: tauri::AppHandle, state: State<'_, AppState>, request: McRequest) -> Result<McResultView, String> {
    let deck = state.with_user(|u| u.get(request.deck_id))?.ok_or("Il deck non esiste più")?;
    state.mc_cancel.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let engine = state.engine()?;
        let list = ygo_engine::DeckList { main: deck.main.clone(), extra: deck.extra.clone(), side: deck.side.clone() };
        if let Some(s) = &request.swap {
            let db = engine.cards();
            if !list.main.iter().any(|&c| db.canonical_code(c) == db.canonical_code(s.out)) {
                return Err("La carta da togliere non è nel Main Deck".to_string());
            }
        }
        let config = ygo_solver::mc_run::McConfig {
            hands: request.hands,
            nodes: request.nodes,
            threshold: None,
            cheap: ygo_solver::mc_run::STAGED_CHEAP,
            full: ygo_solver::mc_run::STAGED_FULL,
            seed: MC_SEED,
            threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
            passive: known_hand_traps(&engine),
            swap: request.swap.as_ref().map(|s| (s.out, s.into)),
        };
        let (deck_id, request_id) = (request.deck_id, request.request_id);
        let sent = |p: ygo_solver::mc_run::McProgress| {
            let phase = if p.phase == ygo_solver::mc_run::McPhase::Deck { "deck" } else { "swap" };
            let _ = app.emit(
                "mc-progress",
                McProgressView { deck_id, request_id, phase, done: p.done, total: p.total, hand: mc_hand_view(&p.played) },
            );
        };
        let report = ygo_solver::mc_run::run(&engine, &list, &config, &state.mc_cancel, &sent)?;
        Ok(McResultView {
            deck_id,
            request_id,
            threshold: report.threshold,
            deck: mc_deck_view(&report.deck),
            swap: report.swap.as_ref().map(|s| McSwapView {
                out: s.out,
                into: s.into,
                deck: mc_deck_view(&s.deck),
                dp: s.paired.dp_adjusted,
                dp_half: s.paired.dp_adjusted_half,
                dmean: s.paired.dmean,
                dmean_half: s.paired.dmean_half,
                changed: s.paired.changed,
            }),
            stopped: report.stopped,
            seconds: report.seconds,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn mc_cancel(state: State<'_, AppState>) {
    state.mc_cancel.store(true, Ordering::Relaxed);
}

fn meta_final(
    app: &tauri::AppHandle,
    state: &AppState,
    decks: &[ygo_solver::MetaDeck],
    boards: Option<Vec<BoardRef>>,
) -> Result<usize, String> {
    let engine = state.engine()?;
    let (config, hands, todo) = {
        let job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_ref().ok_or("Nessuna ricerca da valutare")?;
        let wanted: Vec<(usize, u64)> = match boards {
            Some(named) => named.iter().filter_map(|b| u64::from_str_radix(&b.key, 16).ok().map(|k| (b.hand, k))).collect(),
            None => {
                let mut full: Vec<(&(usize, u64), &ScoreView)> = job.scores.iter().filter(|(_, v)| !v.cheap && v.error.is_none()).collect();
                full.sort_by(|a, b| balanced(b.1).total_cmp(&balanced(a.1)));
                full.into_iter().take(META_BOARDS).map(|(k, _)| *k).collect()
            }
        };
        let todo: Vec<(usize, ygo_solver::Board)> = wanted
            .into_iter()
            .filter(|k| !job.metas.contains_key(k))
            .filter_map(|(h, k)| job.boards.iter().find(|(bh, b)| *bh == h && b.key == k).cloned())
            .collect();
        (job.config.clone(), job.hands.iter().map(|(h, _, _)| h.clone()).collect::<Vec<_>>(), todo)
    };
    state.combo_cancel.store(false, Ordering::Relaxed);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let kit = ygo_solver::Kit::standard(&engine);
    if !todo.is_empty() {
        let rivals = meta_rivals_for(state, &engine, decks, META_DECKS, config.seed, threads)?;
        let next = std::sync::atomic::AtomicUsize::new(0);
        let results = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            for _ in 0..threads.min(todo.len()) {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((hand, board)) = todo.get(i) else { break };
                    if state.combo_cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let m = ygo_solver::meta_score(&engine, &config.deck, &hands[*hand], config.seed, &board.line, &rivals, &kit);
                    if let (Ok(m), Ok(mut out)) = (m, results.lock()) {
                        out.push(((*hand, board.key), meta_view(&m)));
                    }
                });
            }
        });
        let mut job = state.combo.lock().map_err(|e| e.to_string())?;
        let job = job.as_mut().ok_or("La ricerca è cambiata")?;
        for (key, view) in results.into_inner().unwrap_or_default() {
            job.metas.insert(key, view);
        }
    }
    // Every score with a meta measure gets it, and is sent again.
    let mut job = state.combo.lock().map_err(|e| e.to_string())?;
    let job = job.as_mut().ok_or("La ricerca è cambiata")?;
    let metas = job.metas.clone();
    for (key, meta) in &metas {
        if let Some(view) = job.scores.get_mut(key) {
            view.meta = Some(meta.clone());
            let _ = app.emit("combo-score", view.clone());
        }
    }
    Ok(job.scores.values().filter(|v| v.meta.is_some()).count())
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
                rival_lines: Mutex::new(HashMap::new()),
                meta: Mutex::new(None),
                tech: Mutex::new(None),
                second_cancel: AtomicBool::new(false),
                mc_cancel: AtomicBool::new(false),
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
            ban_changes,
            list_favorites,
            set_favorite,
            list_saved_searches,
            save_search,
            delete_saved_search,
            list_decks,
            lab_overview,
            lab_probability,
            lab_special,
            lab_special_known,
            lab_simulate,
            lab_cancel_simulation,
            lab_analyze,
            lab_optimize,
            set_deck_roles,
            guess_deck_roles,
            list_recipes,
            save_recipe,
            delete_recipe,
            record_board_choice,
            learned_weights,
            clear_board_choices,
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
            combo_meta,
            tech_table,
            second_opponents,
            second_run,
            second_cancel,
            mc_run,
            mc_cancel,
            deck_links,
            deck_links_compare,
            combo_cancel,
            combo_explain,
            combo_stops,
            combo_moves,
            board_protection,
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
