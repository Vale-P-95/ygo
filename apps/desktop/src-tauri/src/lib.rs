use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;
use ygo_engine::Engine;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineStatus {
    api_version: String,
    data_dir: String,
    cards: Option<usize>,
    scripts: Option<usize>,
    error: Option<String>,
}

/// Game data (card scripts, card databases) lives in the app data folder, or
/// in `DUELFORGE_DATA` when set (handy during development).
fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    std::env::var_os("DUELFORGE_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from(".")).join("data"))
}

#[tauri::command(async)]
fn engine_status(app: tauri::AppHandle) -> EngineStatus {
    let (major, minor) = Engine::api_version();
    let dir = data_dir(&app);
    let loaded = Engine::load(&dir);
    EngineStatus {
        api_version: format!("{major}.{minor}"),
        data_dir: dir.display().to_string(),
        cards: loaded.as_ref().ok().map(|e| e.cards().len()),
        scripts: loaded.as_ref().ok().map(|e| e.scripts().len()),
        error: loaded.err().map(|e| e.to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![engine_status])
        .run(tauri::generate_context!())
        .expect("error while running DuelForge");
}
