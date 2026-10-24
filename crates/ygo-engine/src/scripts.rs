//! Card scripts (Lua) from Project Ignis CardScripts.

use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::Error;

/// Index of script files by file name. Base scripts (`constant.lua`,
/// `utility.lua`, `proc_*.lua`) live in the root; card scripts (`c<code>.lua`)
/// in sub-folders, searched in priority order.
#[derive(Debug, Default)]
pub struct ScriptStore {
    files: HashMap<String, PathBuf>,
    /// Scripts already read, compiled to Lua bytecode when possible: each
    /// new duel loads them without touching the disk or parsing again.
    loaded: Mutex<HashMap<String, Option<Arc<Vec<u8>>>>>,
    replacements: OnceLock<Vec<&'static Replacement>>,
}

/// A faster copy of a function of the base scripts, loaded after them. It
/// gives the same answers as the upstream function it was written for.
#[derive(Debug)]
pub struct Replacement {
    pub name: &'static str,
    /// The base script that defines the function.
    base: &'static str,
    /// The function as upstream wrote it.
    upstream: &'static str,
    pub source: &'static str,
}

/// The upstream functions of the selection loop by groups enter Lua from
/// the engine millions of times in one request on some decks
/// (`docs/plans/robustezza.md`, the Vendread case).
const REPLACEMENTS: &[Replacement] = &[
    Replacement {
        name: "select_unselect.lua",
        base: "utility.lua",
        upstream: include_str!("../lua/upstream/select_unselect_loop.lua"),
        source: include_str!("../lua/select_unselect.lua"),
    },
    Replacement {
        name: "ritual_check.lua",
        base: "proc_ritual.lua",
        upstream: include_str!("../lua/upstream/ritual_check.lua"),
        source: include_str!("../lua/ritual_check.lua"),
    },
];

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

    /// The script to give the engine: compiled once, then kept in memory.
    pub fn load(&self, name: &str) -> Option<Arc<Vec<u8>>> {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if let Some(found) = self.loaded.lock().ok()?.get(base) {
            return found.clone();
        }
        // DUELFORGE_LUA_SOURCE=1 keeps the sources, whose errors name the line.
        let keep_source = std::env::var_os("DUELFORGE_LUA_SOURCE").is_some();
        let chunk = self.read(base).map(|source| Arc::new(if keep_source { source } else { compile(&source, base).unwrap_or(source) }));
        self.loaded.lock().ok()?.insert(base.to_owned(), chunk.clone());
        chunk
    }

    /// The replacements to load after the base scripts: those whose upstream
    /// function is still the one in the base scripts. If CardScripts changes
    /// it, the replacement is left out rather than undoing the change.
    /// `DUELFORGE_UPSTREAM_LUA=1` leaves them all out (to compare).
    pub fn replacements(&self) -> &[&'static Replacement] {
        self.replacements.get_or_init(|| {
            if std::env::var_os("DUELFORGE_UPSTREAM_LUA").is_some() {
                return Vec::new();
            }
            REPLACEMENTS
                .iter()
                .filter(|r| {
                    // Checkouts on Windows may have CRLF line endings.
                    let base = self.read(r.base).map(|s| String::from_utf8_lossy(&s).replace('\r', "")).unwrap_or_default();
                    let same = base.contains(&r.upstream.replace('\r', ""));
                    if !same {
                        eprintln!("{} changed upstream: {} is left out", r.base, r.name);
                    }
                    same
                })
                .collect()
        })
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
/// Lua bytecode for `source`, compiled by the Lua inside the engine; `None`
/// on a syntax error (the engine then reports it when loading the source).
pub fn compile(source: &[u8], name: &str) -> Option<Vec<u8>> {
    let name = CString::new(format!("@{name}")).ok()?;
    let mut len = 0usize;
    let ptr = unsafe { crate::ffi::df_lua_compile(source.as_ptr() as *const _, source.len(), name.as_ptr(), &mut len) };
    if ptr.is_null() {
        return None;
    }
    let chunk = unsafe { std::slice::from_raw_parts(ptr, len).to_vec() };
    unsafe { crate::ffi::df_free(ptr as *mut _) };
    Some(chunk)
}
