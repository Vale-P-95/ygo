// Raw images of Lua tables (shim/reuse.cpp): two images of a table are the
// same bytes only if nothing was stored in it in between, so a table the
// last duel did not touch is not gone over key by key to be put back.
// Built with Lua itself, which is the only code that knows its tables.
#include <cstring>

#include "lua.h"
#include "lobject.h"
#include "ltable.h"

namespace {
struct Header {
	const void* array;
	const void* node;
	const void* metatable;
	unsigned int array_size;
	unsigned int node_size;
};

Header header(const Table* t) {
	// Compared as bytes: no padding left unset.
	Header h;
	std::memset(&h, 0, sizeof h);
	h.array = t->array;
	h.node = t->node;
	h.metatable = t->metatable;
	h.array_size = luaH_realasize(t);
	h.node_size = static_cast<unsigned int>(allocsizenode(t));
	return h;
}
}

extern "C" {
// Bytes of the image of `table` (a pointer from lua_topointer).
size_t df_table_image_size(const void* table) {
	const Header h = header(static_cast<const Table*>(table));
	return sizeof(Header) + h.array_size * sizeof(TValue) + h.node_size * sizeof(Node);
}

void df_table_image(const void* table, unsigned char* out) {
	const auto* t = static_cast<const Table*>(table);
	const Header h = header(t);
	std::memcpy(out, &h, sizeof h);
	out += sizeof h;
	std::memcpy(out, t->array, h.array_size * sizeof(TValue));
	out += h.array_size * sizeof(TValue);
	if(h.node_size)
		std::memcpy(out, t->node, h.node_size * sizeof(Node));
}

int df_table_same(const void* table, const unsigned char* image, size_t size) {
	const auto* t = static_cast<const Table*>(table);
	const Header h = header(t);
	if(size != df_table_image_size(table) || std::memcmp(image, &h, sizeof h) != 0)
		return 0;
	image += sizeof h;
	if(std::memcmp(image, t->array, h.array_size * sizeof(TValue)) != 0)
		return 0;
	image += h.array_size * sizeof(TValue);
	return !h.node_size || std::memcmp(image, t->node, h.node_size * sizeof(Node)) == 0;
}
}
