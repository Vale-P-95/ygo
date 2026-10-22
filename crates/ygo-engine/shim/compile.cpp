// Compiles Lua source into a binary chunk with the Lua embedded in the
// engine, so scripts are parsed once and every new duel loads bytecode.
// Debug information is stripped: loading is faster, error messages lose
// line numbers (set DUELFORGE_LUA_SOURCE to load the sources instead).
// Lua is compiled as C++ here, so its functions are only reachable from C++.
#include <cstdlib>
#include <cstring>
#include <string>

#include "lauxlib.h"
#include "lua.h"

extern "C" void* df_arena_enter(void* arena);

namespace {
// Outside any duel's arena while compiling (shim/arena.cpp): the state is
// thrown away at once, and would only fill the arena of the duel that asked.
struct NoArena {
	void* before = df_arena_enter(nullptr);
	~NoArena() {
		df_arena_enter(before);
	}
};

int append(lua_State*, const void* data, size_t size, void* out) {
	static_cast<std::string*>(out)->append(static_cast<const char*>(data), size);
	return 0;
}
}

extern "C" {
// Returns a malloc'd chunk (free it with df_free) or nullptr on a syntax error.
unsigned char* df_lua_compile(const char* source, size_t length, const char* name, size_t* out_length) {
	NoArena outside;
	lua_State* L = luaL_newstate();
	if(!L)
		return nullptr;
	std::string chunk;
	bool ok = false;
	try {
		ok = luaL_loadbufferx(L, source, length, name, "t") == LUA_OK && lua_dump(L, append, &chunk, 1) == 0;
	} catch(...) {
		ok = false;
	}
	lua_close(L);
	if(!ok || chunk.empty())
		return nullptr;
	auto* out = static_cast<unsigned char*>(std::malloc(chunk.size()));
	if(!out)
		return nullptr;
	std::memcpy(out, chunk.data(), chunk.size());
	*out_length = chunk.size();
	return out;
}

void df_free(void* p) {
	std::free(p);
}
}
