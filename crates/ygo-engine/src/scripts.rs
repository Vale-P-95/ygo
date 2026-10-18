//! Card scripts (Lua) from Project Ignis CardScripts.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Error;

/// Index of script files by file name. Base scripts (`constant.lua`,
/// `utility.lua`, `proc_*.lua`) live in the root; card scripts (`c<code>.lua`)
/// in sub-folders, searched in priority order.
#[derive(Debug, Default)]
pub struct ScriptStore {
    files: HashMap<String, PathBuf>,
}

const CARD_SCRIPT_DIRS: &[&str] = &["official", "pre-release", "pre-errata", "unofficial"];

impl ScriptStore {
    pub fn load_dir(root: &Path) -> Result<Self, Error> {
        if !root.join("constant.lua").is_file() || !root.join("utility.lua").is_file() {
            return Err(Error::Data(format!("{} does not look like a CardScripts checkout", root.display())));
        }
        let mut store = ScriptStore::default();
        store.index(root);
        for dir in CARD_SCRIPT_DIRS {
            store.index(&root.join(dir));
        }
        Ok(store)
    }

    fn index(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for path in entries.filter_map(|e| e.ok().map(|e| e.path())) {
            if path.extension().is_some_and(|x| x == "lua") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    self.files.entry(name.to_owned()).or_insert(path);
                }
            }
        }
    }

    pub fn path(&self, name: &str) -> Option<&Path> {
        // The engine may ask for "./script/c123.lua"-style names.
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        self.files.get(base).map(PathBuf::as_path)
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(name)?).ok()
    }

    pub fn has_card_script(&self, code: u32) -> bool {
        self.files.contains_key(&format!("c{code}.lua"))
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
