use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::http::{Response, StatusCode};
use tauri::{Emitter, Manager, State};
use ygo_data::sync::{self, Progress};
use ygo_data::{deck, Card, CardStore, Chip, Deck, DeckSummary, Facets, ImageCache, ImageSize, Legality, UserStore};
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
}

impl AppState {
    fn cards_db(&self) -> PathBuf {
        self.data_dir.join("cards.db")
    }

    fn open_store(&self) {
        let path = self.cards_db();
        let store = path.is_file().then(|| CardStore::open(&path).ok()).flatten();
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
    let parsed = ygo_data::parse_query(&query);
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
fn list_decks(state: State<'_, AppState>) -> Result<Vec<DeckSummary>, String> {
    state.with_user(|u| u.list())
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
    let mut ids: Vec<u32> = deck.main.iter().chain(&deck.extra).chain(&deck.side).copied().collect();
    ids.sort_unstable();
    ids.dedup();
    let cards: HashMap<u32, Card> = state.with_store(|s| s.get_many(&ids))?.into_iter().map(|c| (c.id, c)).collect();
    Ok(deck::validate(&deck, &cards))
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
            list_decks,
            get_deck,
            save_deck,
            delete_deck,
            validate_deck,
            import_deck,
            export_deck
        ])
        .run(tauri::generate_context!())
        .expect("error while running DuelForge");
}
