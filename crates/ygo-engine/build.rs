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
    // field.cpp is compiled from a patched copy (see `patch_field`).
    let field = core.join("field.cpp");
    let patched = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("field.cpp");
    let text = std::fs::read_to_string(&field).expect("field.cpp");
    std::fs::write(&patched, patch_field(&text)).expect("patched field.cpp");
    core_build.files(sources(&core, "cpp").into_iter().filter(|p| *p != field)).file(&patched);
    core_build.files(sources(&core.join("RNG"), "cpp"));
    core_build.compile("ocgcore");
}

/// Upstream bug: `reset_phase` and `reset_chain` remove effects while
/// iterating over the set that holds them, and removing a grant effect also
/// removes the effects it granted, which can be the next ones in the set:
/// the loop then reads freed memory (a crash at the End Phase, seen with
/// D/D/D lines). The fixed loops collect the effects first and remove those
/// still there. Panics if upstream changed the code, so the patch is
/// looked at again.
fn patch_field(text: &str) -> String {
    const RESET_PHASE: &str = "void field::reset_phase(uint32_t phase) {
\tfor(auto eit = effects.pheff.begin(); eit != effects.pheff.end();) {
\t\tauto rm = eit++;
\t\tif((*rm)->reset(phase, RESET_PHASE)) {
\t\t\tif((*rm)->is_flag(EFFECT_FLAG_FIELD_ONLY))
\t\t\t\tremove_effect((*rm));
\t\t\telse
\t\t\t\t(*rm)->handler->remove_effect((*rm));
\t\t}
\t}
}";
    const RESET_PHASE_FIXED: &str = "void field::reset_phase(uint32_t phase) {
\tstd::vector<effect*> expired;
\tfor(auto* peffect : effects.pheff)
\t\tif(peffect->reset(phase, RESET_PHASE))
\t\t\texpired.push_back(peffect);
\tfor(auto* peffect : expired) {
\t\tif(effects.pheff.find(peffect) == effects.pheff.end())
\t\t\tcontinue;
\t\tif(peffect->is_flag(EFFECT_FLAG_FIELD_ONLY))
\t\t\tremove_effect(peffect);
\t\telse
\t\t\tpeffect->handler->remove_effect(peffect);
\t}
}";
    const RESET_CHAIN: &str = "\tfor(auto eit = effects.cheff.begin(); eit != effects.cheff.end();) {
\t\tauto rm = eit++;
\t\tif((*rm)->is_flag(EFFECT_FLAG_FIELD_ONLY))
\t\t\tremove_effect((*rm));
\t\telse
\t\t\t(*rm)->handler->remove_effect((*rm));
\t}";
    const RESET_CHAIN_FIXED: &str = "\tstd::vector<effect*> expired(effects.cheff.begin(), effects.cheff.end());
\tfor(auto* peffect : expired) {
\t\tif(effects.cheff.find(peffect) == effects.cheff.end())
\t\t\tcontinue;
\t\tif(peffect->is_flag(EFFECT_FLAG_FIELD_ONLY))
\t\t\tremove_effect(peffect);
\t\telse
\t\t\tpeffect->handler->remove_effect(peffect);
\t}";
    let text = text.replace("\r\n", "\n");
    for old in [RESET_PHASE, RESET_CHAIN] {
        assert_eq!(text.matches(old).count(), 1, "vendor/ygopro-core/field.cpp changed: check the reset loops patch in build.rs");
    }
    text.replace(RESET_PHASE, RESET_PHASE_FIXED).replace(RESET_CHAIN, RESET_CHAIN_FIXED)
}
