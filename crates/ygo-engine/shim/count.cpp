// Counting the Lua work of each engine request, and stopping a request that
// does too much: a few positions make a card script loop for seconds, and
// the search replays them again and again (see duel.rs).
//
// The limit counts the times the core enters Lua (interpreter::call_lua,
// patched in build.rs): one increment each. Lua's own hooks cost too much to
// keep on (a count hook is checked before every instruction, a fifth of the
// search; a call hook a tenth), so they count calls and instructions only
// when measuring.
#include <cstdint>

#include "duel.h"
#include "interpreter.h"
#include "lua.h"
#include "ocgapi.h"

namespace {

// Lua runs on the thread that calls the engine, so the counts are per
// thread: each duel apart.
thread_local uint64_t entries = 0;
thread_local uint64_t calls = 0;
thread_local uint64_t instructions = 0;
// Entries a request may make; 0 for no limit.
thread_local uint64_t limit = 0;
thread_local bool stopped = false;

void hook(lua_State* L, lua_Debug* ar) {
	if(ar->event == LUA_HOOKCOUNT)
		instructions += static_cast<uint64_t>(lua_gethookcount(L));
	else
		++calls;
}

} // namespace

extern "C" {

// Whether the core may enter Lua once more in this request.
int df_lua_enter() {
	++entries;
	if(limit != 0 && entries > limit) {
		stopped = true;
		return 0;
	}
	return 1;
}

// Counts the calls and instructions (1000 at a time) of `ocg_duel` from
// now on. The coroutines the engine creates later inherit the hook.
void df_lua_hook_install(OCG_Duel ocg_duel) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	lua_sethook(pduel->lua->lua_state, hook, LUA_MASKCALL | LUA_MASKCOUNT, 1000);
}

// Starts a request on this thread with at most `max` entries into Lua (0: no limit).
void df_lua_request_start(uint64_t max) {
	entries = calls = instructions = 0;
	limit = max;
	stopped = false;
}

// Ends the request: its entries, calls and instructions, and whether the
// limit stopped it.
int df_lua_request_end(uint64_t* entry_count, uint64_t* call_count, uint64_t* instruction_count) {
	*entry_count = entries;
	*call_count = calls;
	*instruction_count = instructions;
	const int was_stopped = stopped ? 1 : 0;
	entries = calls = instructions = limit = 0;
	stopped = false;
	return was_stopped;
}

} // extern "C"
