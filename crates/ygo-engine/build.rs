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
    // lauxlib.c is compiled from a patched copy (see `patch_lauxlib`), which
    // includes mimalloc.h from libmimalloc-sys.
    lua_build.include(std::env::var("DEP_MIMALLOC_INCLUDE_DIR").expect("mimalloc include dir from libmimalloc-sys"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    for file in sources(&lua_src, "c") {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if name == "lauxlib.c" {
            let patched = out_dir.join(name);
            std::fs::write(&patched, patch_lauxlib(&std::fs::read_to_string(&file).expect("lauxlib.c"))).expect("patched lauxlib.c");
            lua_build.file(patched);
        } else if !LUA_EXCLUDED.contains(&name) {
            lua_build.file(file);
        }
    }
    // Our Lua helpers, built with Lua itself (same flags, same library).
    let shim = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shim");
    println!("cargo:rerun-if-changed={}", shim.display());
    lua_build.file(shim.join("compile.cpp"));
    lua_build.compile("lua");

    let mut core_build = cc::Build::new();
    core_build.cpp(true).std("c++17").include(&core).include(&lua_src).include(&shim).warnings(false);
    if msvc {
        core_build.flag("/GR-").flag("/EHsc").define("NOMINMAX", None);
    } else {
        core_build.flag("-fno-rtti");
    }
    // field.cpp is compiled from a patched copy (see `patch_field`).
    let field = core.join("field.cpp");
    let patched = out_dir.join("field.cpp");
    let text = std::fs::read_to_string(&field).expect("field.cpp");
    std::fs::write(&patched, patch_field(&text)).expect("patched field.cpp");
    // duel.cpp too (see `patch_duel`), and the shim that reuses duels.
    let duel = core.join("duel.cpp");
    let patched_duel = out_dir.join("duel.cpp");
    std::fs::write(&patched_duel, patch_duel(&std::fs::read_to_string(&duel).expect("duel.cpp"))).expect("patched duel.cpp");
    core_build
        .files(sources(&core, "cpp").into_iter().filter(|p| *p != field && *p != duel))
        .file(&patched)
        .file(&patched_duel)
        .file(shim.join("reuse.cpp"));
    core_build.files(sources(&core.join("RNG"), "cpp"));
    core_build.compile("ocgcore");
}

/// Lua allocates through mimalloc instead of the C runtime's malloc: a duel
/// lives a few milliseconds and makes and frees many small objects (with
/// glibc, about 40% of a combo search's time went to malloc and free).
/// Every Lua state comes from `luaL_newstate` (the core's interpreter and
/// shim/compile.cpp) and nothing else frees Lua's blocks, so its memory never
/// meets another allocator. mimalloc.h comes with the linked library, so the
/// compiler checks the calls.
/// Panics if upstream changed the code, so the patch is looked at again.
fn patch_lauxlib(text: &str) -> String {
    const ALLOC: &str = "  if (nsize == 0) {
    free(ptr);
    return NULL;
  }
  else
    return realloc(ptr, nsize);";
    const ALLOC_FIXED: &str = "  if (nsize == 0) {
    mi_free(ptr);
    return NULL;
  }
  else
    return mi_realloc(ptr, nsize);";
    const FUNCTION: &str = "static void *l_alloc (void *ud, void *ptr, size_t osize, size_t nsize) {";
    const INCLUDE: &str = "#include \"mimalloc.h\"\n";
    let text = text.replace("\r\n", "\n");
    for old in [ALLOC, FUNCTION] {
        assert_eq!(text.matches(old).count(), 1, "vendor/ygopro-core/lua/src/lauxlib.c changed: check the allocator patch in build.rs");
    }
    text.replace(ALLOC, ALLOC_FIXED).replace(FUNCTION, &format!("{INCLUDE}{FUNCTION}"))
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

/// `duel::clear`, which upstream never calls, empties a duel for the next
/// one (shim/reuse.cpp): the new field gets the next duel's options instead
/// of fixed defaults, the random numbers are seeded again like in a new
/// duel, the messages not read are dropped, and the effects the base
/// scripts created are kept (the shim puts them back). Panics if upstream
/// changed the code, so the patch is looked at again.
fn patch_duel(text: &str) -> String {
    const CLEAR: &str = "void duel::clear() {
	static constexpr OCG_DuelOptions default_options{ {},0,{8000,5,1},{8000,5,1} };
	for(auto& pcard : cards)
		delete pcard;
	for(auto& peffect : effects) {
		lua->unregister_effect(peffect);
		delete peffect;
	}
	delete game_field;
	//force full garbage collection to clean the groups
	lua->collect(true);
	cards.clear();
";
    const CLEAR_FIXED: &str = "thread_local const df_reset_info* df_reset = nullptr;
void duel::clear() {
	const OCG_DuelOptions& default_options = *df_reset->options;
	const auto& seed = default_options.seed;
	random = RNG::Xoshiro256StarStar({ seed[0], seed[1], seed[2], seed[3] });
	messages.clear();
	for(auto& pcard : cards)
		delete pcard;
	for(auto& peffect : effects) {
		if(df_reset->keep->count(peffect))
			continue;
		lua->unregister_effect(peffect);
		delete peffect;
	}
	delete game_field;
	if(df_reset->collect)
		lua->collect(true);
	cards.clear();
";
    const EFFECTS: &str = "	effects.clear();
	game_field = new field(this, default_options);";
    const EFFECTS_FIXED: &str = "	effects = { df_reset->keep->begin(), df_reset->keep->end() };
	game_field = new field(this, default_options);";
    const INCLUDE: &str = "#include \"interpreter.h\"\n";
    let text = text.replace("\r\n", "\n");
    for old in [CLEAR, EFFECTS, INCLUDE] {
        assert_eq!(text.matches(old).count(), 1, "vendor/ygopro-core/duel.cpp changed: check the duel::clear patch in build.rs");
    }
    text.replace(CLEAR, CLEAR_FIXED).replace(EFFECTS, EFFECTS_FIXED).replace(INCLUDE, &format!("{INCLUDE}#include \"reuse.h\"\n"))
}
