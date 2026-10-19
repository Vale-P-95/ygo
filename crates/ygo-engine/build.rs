//! Compiles the EDOPro duel engine (vendor/ygopro-core) and its embedded Lua
//! into static libraries, mirroring the upstream premake configuration.

use std::path::{Path, PathBuf};

const LUA_EXCLUDED: &[&str] =
    &["lbitlib.c", "lcorolib.c", "ldblib.c", "linit.c", "loadlib.c", "loslib.c", "ltests.c", "lua.c", "luac.c", "lutf8lib.c", "onelua.c"];

fn sources(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (did you run `git submodule update --init --recursive`?)", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    files.sort();
    files
}

fn main() {
    let core = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/ygopro-core");
    let lua = core.join("lua");
    let lua_src = lua.join("src");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    println!("cargo:rerun-if-changed={}", core.display());

    // Lua is compiled as C++, like upstream: Lua errors then unwind as C++
    // exceptions through the engine instead of using longjmp.
    let customize = lua.join("luaconf-customize.h");
    let mut lua_build = cc::Build::new();
    lua_build.cpp(true).include(&lua).include(&lua_src).warnings(false);
    if msvc {
        lua_build.flag("/TP").flag(format!("/FI{}", customize.display()));
    } else {
        lua_build.flag("-xc++").flag("-include").flag(customize.to_str().expect("utf-8 path"));
    }
    for file in sources(&lua_src, "c") {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !LUA_EXCLUDED.contains(&name) {
            lua_build.file(file);
        }
    }
    // Our Lua helpers, built with Lua itself (same flags, same library).
    let shim = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shim");
    println!("cargo:rerun-if-changed={}", shim.display());
    lua_build.file(shim.join("compile.cpp"));
    lua_build.compile("lua");

    let mut core_build = cc::Build::new();
    core_build.cpp(true).std("c++17").include(&core).include(&lua_src).warnings(false);
    if msvc {
        core_build.flag("/GR-").flag("/EHsc").define("NOMINMAX", None);
    } else {
        core_build.flag("-fno-rtti");
    }
    core_build.files(sources(&core, "cpp")).files(sources(&core.join("RNG"), "cpp"));
    core_build.compile("ocgcore");
}
