//! Downloads that keep the local data current: the card list from YGOPRODeck
//! and the engine data (card scripts and databases) from Project Ignis.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use futures_util::StreamExt;
use serde::Serialize;

use crate::import;
use crate::store::CardStore;
use crate::Error;

const CARDINFO_URL: &str = "https://db.ygoprodeck.com/api/v7/cardinfo.php?misc=yes";
const DB_VERSION_URL: &str = "https://db.ygoprodeck.com/api/v7/checkDBVer.php";

/// Engine data repositories and the folder each one is extracted to.
pub const ENGINE_SOURCES: &[(&str, &str)] = &[("scripts", "ProjectIgnis/CardScripts"), ("cdb", "ProjectIgnis/BabelCDB")];
/// Marker file with the commit an extracted folder comes from.
const VERSION_FILE: &str = ".duelforge-version";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Checking,
    Downloading,
    Extracting,
    Indexing,
    Done,
    UpToDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// `cards`, `scripts` or `cdb`.
    pub item: String,
    pub stage: Stage,
    pub done: u64,
    pub total: Option<u64>,
}

impl Progress {
    fn new(item: &str, stage: Stage) -> Self {
        Self { item: item.to_owned(), stage, done: 0, total: None }
    }
}

pub fn http_client() -> Result<reqwest::Client, Error> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("DuelForge/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Vale-P-95/ygo)"))
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()?)
}

async fn download(client: &reqwest::Client, url: &str, item: &str, progress: &(dyn Fn(Progress) + Sync)) -> Result<Vec<u8>, Error> {
    let response = client.get(url).send().await?.error_for_status()?;
    let total = response.content_length();
    let mut body = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut stream = response.bytes_stream();
    let mut last_report = 0usize;
    while let Some(chunk) = stream.next().await {
        body.extend_from_slice(&chunk?);
        if body.len() - last_report >= 256 * 1024 {
            last_report = body.len();
            progress(Progress { item: item.into(), stage: Stage::Downloading, done: body.len() as u64, total });
        }
    }
    progress(Progress { item: item.into(), stage: Stage::Downloading, done: body.len() as u64, total: Some(body.len() as u64) });
    Ok(body)
}

/// Downloads the card list if YGOPRODeck has a newer version than `current`
/// and builds a new database at `new_db`. Returns the version installed, or
/// `None` when already up to date. The caller swaps `new_db` in place (see
/// [`install_file`]) once it has closed the old database.
pub async fn update_cards(
    client: &reqwest::Client,
    current: Option<&str>,
    new_db: &Path,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Option<String>, Error> {
    progress(Progress::new("cards", Stage::Checking));
    let remote = import::parse_db_version(&client.get(DB_VERSION_URL).send().await?.error_for_status()?.bytes().await?)?;
    if current == Some(remote.as_str()) {
        progress(Progress::new("cards", Stage::UpToDate));
        return Ok(None);
    }
    let json = download(client, CARDINFO_URL, "cards", progress).await?;
    progress(Progress::new("cards", Stage::Indexing));
    let new_db = new_db.to_owned();
    let version = remote.clone();
    tokio::task::spawn_blocking(move || -> Result<(), Error> {
        let cards = import::parse_cardinfo(&json)?;
        if new_db.exists() {
            std::fs::remove_file(&new_db)?;
        }
        CardStore::open(&new_db)?.replace_all(&cards, &version)
    })
    .await
    .map_err(|e| Error::Data(e.to_string()))??;
    progress(Progress::new("cards", Stage::Done));
    Ok(Some(remote))
}

/// Replaces `target` with `new` (removing `target` first: Windows does not
/// rename over an existing file).
pub fn install_file(new: &Path, target: &Path) -> Result<(), Error> {
    if target.exists() {
        std::fs::remove_file(target)?;
    }
    std::fs::rename(new, target)?;
    Ok(())
}

async fn latest_commit(client: &reqwest::Client, repo: &str) -> Result<String, Error> {
    let url = format!("https://api.github.com/repos/{repo}/commits/HEAD");
    let sha = client.get(&url).header("Accept", "application/vnd.github.sha").send().await?.error_for_status()?.text().await?;
    let sha = sha.trim().to_owned();
    if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Data(format!("unexpected commit id from {repo}")));
    }
    Ok(sha)
}

pub fn installed_engine_version(data_dir: &Path, folder: &str) -> Option<String> {
    std::fs::read_to_string(data_dir.join(folder).join(VERSION_FILE)).ok().map(|s| s.trim().to_owned())
}

/// Brings `data_dir/scripts` and `data_dir/cdb` to the latest commit of
/// Project Ignis' repositories. Returns whether anything changed.
pub async fn update_engine_data(client: &reqwest::Client, data_dir: &Path, progress: &(dyn Fn(Progress) + Sync)) -> Result<bool, Error> {
    let mut changed = false;
    for (folder, repo) in ENGINE_SOURCES {
        progress(Progress::new(folder, Stage::Checking));
        let sha = latest_commit(client, repo).await?;
        if installed_engine_version(data_dir, folder).as_deref() == Some(sha.as_str()) {
            progress(Progress::new(folder, Stage::UpToDate));
            continue;
        }
        let archive = download(client, &format!("https://codeload.github.com/{repo}/zip/{sha}"), folder, progress).await?;
        progress(Progress::new(folder, Stage::Extracting));
        let target = data_dir.join(folder);
        let staging = data_dir.join(format!("{folder}.new"));
        tokio::task::spawn_blocking(move || -> Result<(), Error> {
            if staging.exists() {
                std::fs::remove_dir_all(&staging)?;
            }
            extract_zip(&archive, &staging, true)?;
            std::fs::write(staging.join(VERSION_FILE), &sha)?;
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
            std::fs::rename(&staging, &target)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::Data(e.to_string()))??;
        progress(Progress::new(folder, Stage::Done));
        changed = true;
    }
    Ok(changed)
}

/// Extracts a zip archive into `dest`, optionally dropping the top-level
/// folder GitHub archives wrap everything in. Entries that would escape
/// `dest` are rejected.
pub fn extract_zip(bytes: &[u8], dest: &Path, strip_top_level: bool) -> Result<usize, Error> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| Error::Zip(e.to_string()))?;
    let mut files = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| Error::Zip(e.to_string()))?;
        let Some(path) = entry.enclosed_name() else {
            return Err(Error::Zip(format!("unsafe path in archive: {}", entry.name())));
        };
        let relative: PathBuf = path.components().skip(usize::from(strip_top_level)).collect();
        if relative.as_os_str().is_empty() || relative.components().any(|c| !matches!(c, Component::Normal(_))) {
            continue;
        }
        let out = dest.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data)?;
        std::fs::write(&out, data)?;
        files += 1;
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_with(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buf);
        let options = zip::write::SimpleFileOptions::default();
        for (name, content) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn extracts_and_strips_github_top_folder() {
        let dir = std::env::temp_dir().join(format!("duelforge-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let zip = zip_with(&[("CardScripts-abc/constant.lua", "-- c"), ("CardScripts-abc/official/c1.lua", "-- 1")]);
        assert_eq!(extract_zip(&zip, &dir, true).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(dir.join("constant.lua")).unwrap(), "-- c");
        assert!(dir.join("official/c1.lua").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_paths_escaping_destination() {
        let dir = std::env::temp_dir().join(format!("duelforge-zip-evil-{}", std::process::id()));
        let zip = zip_with(&[("../evil.lua", "x")]);
        assert!(extract_zip(&zip, &dir, false).is_err());
        assert!(!dir.join("../evil.lua").exists());
    }
}
