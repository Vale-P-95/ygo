//! Card images, downloaded once from YGOPRODeck and then served from disk.
//! YGOPRODeck asks not to hotlink: every image is fetched at most once, with
//! a request rate well under their limit of 20 per second.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, Semaphore};

use crate::Error;

const BASE_URL: &str = "https://images.ygoprodeck.com/images";
/// About 14 requests per second at most.
const MIN_INTERVAL: Duration = Duration::from_millis(70);
const MAX_CONCURRENT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSize {
    /// 168×246, for grids.
    Small,
    /// 421×614, for the viewer.
    Full,
    /// The artwork alone, for banners and deck boxes.
    Cropped,
}

impl ImageSize {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "small" => ImageSize::Small,
            "full" => ImageSize::Full,
            "cropped" => ImageSize::Cropped,
            _ => return None,
        })
    }

    fn dir(self) -> &'static str {
        match self {
            ImageSize::Small => "small",
            ImageSize::Full => "full",
            ImageSize::Cropped => "cropped",
        }
    }

    fn remote_dir(self) -> &'static str {
        match self {
            ImageSize::Small => "cards_small",
            ImageSize::Full => "cards",
            ImageSize::Cropped => "cards_cropped",
        }
    }
}

pub struct ImageCache {
    dir: PathBuf,
    client: reqwest::Client,
    permits: Semaphore,
    next_slot: Mutex<Instant>,
}

impl ImageCache {
    pub fn new(dir: PathBuf, client: reqwest::Client) -> Self {
        Self { dir, client, permits: Semaphore::new(MAX_CONCURRENT), next_slot: Mutex::new(Instant::now()) }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn cached_path(&self, id: u32, size: ImageSize) -> PathBuf {
        self.dir.join(size.dir()).join(format!("{id}.jpg"))
    }

    /// Path of the image on disk, downloading it first if needed.
    pub async fn get(&self, id: u32, size: ImageSize) -> Result<PathBuf, Error> {
        let path = self.cached_path(id, size);
        if path.is_file() {
            return Ok(path);
        }
        let _permit = self.permits.acquire().await.map_err(|e| Error::Http(e.to_string()))?;
        if path.is_file() {
            return Ok(path);
        }
        {
            let mut next = self.next_slot.lock().await;
            let now = Instant::now();
            if *next > now {
                tokio::time::sleep(*next - now).await;
            }
            *next = Instant::now().max(*next) + MIN_INTERVAL;
        }
        let url = format!("{BASE_URL}/{}/{id}.jpg", size.remote_dir());
        let bytes = self.client.get(&url).send().await?.error_for_status()?.bytes().await?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Write then rename, so a half-written file is never served.
        let tmp = path.with_extension(format!("{}.part", std::process::id()));
        tokio::fs::write(&tmp, &bytes).await?;
        tokio::fs::rename(&tmp, &path).await?;
        Ok(path)
    }

    /// Number of images already on disk, per size.
    pub fn cached_count(&self, size: ImageSize) -> usize {
        std::fs::read_dir(self.dir.join(size.dir())).map(|d| d.count()).unwrap_or(0)
    }
}
