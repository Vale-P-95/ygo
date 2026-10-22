// Counting the Lua instructions of each engine request, and stopping a
// request that runs too many: a few positions make a card script loop for
// seconds, and the search replays them again and again (see duel.rs).
#include <cstdint>

#include "duel.h"
#include "interpreter.h"
#include "lauxlib.h"
#include "lua.h"
#include "ocgapi.h"

namespace {

// Lua runs on the thread that calls the engine, so the counts are per
// thread: each duel apart.
thread_local uint64_t done = 0;
// Instructions a request may run; 0 for no limit.
thread_local uint64_t limit = 0;
thread_local bool stopped = false;

void count(lua_State* L, lua_Debug*) {
	done += static_cast<uint64_t>(lua_gethookcount(L));
	if(limit != 0 && done > limit) {
		// Again at every hook until the request ends: the scripts still
		// running fail at once and the engine gets back to us.
		stopped = true;
		luaL_error(L, "more than %I Lua instructions in one request", static_cast<lua_Integer>(limit));
	}
}

} // namespace

extern "C" {

// Counts the instructions of `ocg_duel` from now on, `step` at a time. The
// coroutines the engine creates later inherit the hook from the main state.
void df_lua_hook_install(OCG_Duel ocg_duel, int step) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	lua_sethook(pduel->lua->lua_state, count, LUA_MASKCOUNT, step);
}

// Starts a request on this thread with at most `max` instructions (0: no limit).
void df_lua_request_start(uint64_t max) {
	done = 0;
	limit = max;
	stopped = false;
}

// Ends the request: the instructions it ran, and whether the limit stopped it.
uint64_t df_lua_request_end(int* was_stopped) {
	*was_stopped = stopped ? 1 : 0;
	limit = 0;
	stopped = false;
	const uint64_t d = done;
	done = 0;
	return d;
}

} // extern "C"
