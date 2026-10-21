// Images of Lua tables (shim/reuse.cpp): two images of a table are the same
// only if nothing was stored in it in between, so a table the last duel did
// not touch is not gone over key by key to be put back. An image holds the
// table's parts and every slot's tags and values as Lua lays them out (the
// bytes no slot uses are left out: they are never written). Built with Lua
// itself, which is the only code that knows its tables.
#include <cstdint>
#include <cstring>
#include <vector>

#include "lua.h"
#include "lobject.h"
#include "ltable.h"

namespace {
using Image = std::vector<uint64_t>;

uint64_t bits(const Value& v) {
	uint64_t out = 0;
	std::memcpy(&out, &v, sizeof v < sizeof out ? sizeof v : sizeof out);
	return out;
}

// Nil and booleans are told by their tag alone; their value is never set.
bool has_value(lu_byte tag) {
	return novariant(tag) != LUA_TNIL && novariant(tag) != LUA_TBOOLEAN;
}

void image_of(const Table* t, Image& out) {
	out.clear();
	const unsigned int array_size = luaH_realasize(t);
	const unsigned int node_size = static_cast<unsigned int>(allocsizenode(t));
	out.push_back(reinterpret_cast<uintptr_t>(t->array));
	out.push_back(reinterpret_cast<uintptr_t>(t->node));
	out.push_back(reinterpret_cast<uintptr_t>(t->metatable));
	out.push_back((static_cast<uint64_t>(array_size) << 32) | node_size);
	for(unsigned int i = 0; i < array_size; ++i) {
		const TValue& v = t->array[i];
		out.push_back(v.tt_);
		if(has_value(v.tt_))
			out.push_back(bits(v.value_));
	}
	for(unsigned int i = 0; i < node_size; ++i) {
		const Node& n = t->node[i];
		out.push_back(n.u.tt_ | static_cast<uint64_t>(n.u.key_tt) << 8 | static_cast<uint64_t>(static_cast<uint32_t>(n.u.next)) << 32);
		if(has_value(n.u.tt_))
			out.push_back(bits(n.u.value_));
		if(has_value(n.u.key_tt))
			out.push_back(bits(n.u.key_val));
	}
}
}

extern "C" {
// The image of `table` (a pointer from lua_topointer), into `out`
// (a std::vector<uint64_t>).
void df_table_image(const void* table, void* out) {
	image_of(static_cast<const Table*>(table), *static_cast<Image*>(out));
}

// Whether `table` still has the image `image`.
int df_table_same(const void* table, const void* image) {
	thread_local Image now;
	image_of(static_cast<const Table*>(table), now);
	return now == *static_cast<const Image*>(image);
}
}
