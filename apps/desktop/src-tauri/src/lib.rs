use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::http::{Response, StatusCode};
use tauri::{Emitter, Manager, State};
use ygo_data::sync::{self, Progress};
use ygo_data::{
    banlist, deck, Banlist, Card, CardStore, Chip, Deck, DeckBox, DeckSummary, Facets, Format, ImageCache, ImageSize, Legality, Recipe,
    SavedSearch, Tag, UserStore,
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
    let dir = app.path().download_dir().or_else(|_| app.path().app_data_dir()).map_err(|e| e.to_string())?.join("DuelForge");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stem = file_stem(&name);
    let mut path = dir.join(format!("{stem}.png"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).png"));
        n += 1;
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
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
            list_favorites,
            set_favorite,
            list_saved_searches,
            save_search,
            delete_saved_search,
            list_decks,
            lab_overview,
            lab_probability,
            set_deck_roles,
            list_recipes,
            save_recipe,
            delete_recipe,
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
