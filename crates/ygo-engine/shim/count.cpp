// Counting the Lua instructions a duel runs, to measure what a limit per
// engine request would cut (DUELFORGE_LUA_COUNT, see duel.rs). Off unless
// installed: without the hook Lua runs as before.
#include <cstdint>

#include "duel.h"
#include "interpreter.h"
#include "lua.h"
#include "ocgapi.h"

namespace {

// Instructions between two calls of the hook.
constexpr int STEP = 1000;

// Lua runs on the thread that calls the engine, so a counter per thread
// counts each duel apart.
thread_local uint64_t ticks = 0;

void count(lua_State*, lua_Debug*) {
	++ticks;
}

} // namespace

extern "C" {

// Counts the instructions of `ocg_duel` from now on. The coroutines the
// engine creates later inherit the hook from the main state.
void df_lua_count_install(OCG_Duel ocg_duel) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	lua_sethook(pduel->lua->lua_state, count, LUA_MASKCOUNT, STEP);
}

// Thousands of instructions run on this thread since the last call.
uint64_t df_lua_count_take() {
	const uint64_t t = ticks;
	ticks = 0;
	return t;
}

} // extern "C"
