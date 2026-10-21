// Reusing a duel for the next one. Every new duel loads the engine's base
// scripts (constant.lua, utility.lua and the files it loads), which is about
// half the cost of a short duel. A duel that is over can instead be emptied
// and set up again with the base scripts kept:
// - df_duel_snapshot, right after the base scripts are loaded, copies every
//   table reachable from the globals (and the upvalues of every function),
//   and the effects the base scripts created (some register global effects,
//   like proc_workaround.lua's hand shuffle) with the field's counters;
// - df_duel_reset puts those tables and upvalues back as they were (the card
//   scripts' classes and anything they changed go away, so each card script
//   runs again like in a new duel), drops your cards, other effects,
//   coroutines and field, starts a new field with the options and random
//   seed given (duel::clear, compiled from a patched copy of duel.cpp, see
//   build.rs), and registers the base effects again as they were.
#include <algorithm>
#include <array>
#include <cstring>
#include <memory>
#include <unordered_set>
#include <vector>

#include "card.h"
#include "duel.h"
#include "effect.h"
#include "field.h"
#include "interpreter.h"
#include "lauxlib.h"
#include "lua.h"
#include "ocgapi.h"
#include "reuse.h"

namespace {
const char* const SNAPSHOT = "duelforge.snapshot";

// The snapshot table: [1] table -> copy of its pairs, [2] table -> pair
// count, [3] table -> metatable (or false), [4] upvalues (see
// visit_function), [5] what visit_function has seen (dropped at the end),
// [6] the Lua side of each base effect (see `Snapshot`).
enum { COPIES = 1, COUNTS, METAS, UPVALUES, SEEN, EFFECTS };

// The C++ side of the snapshot: the effects the base scripts created, as
// they were, and the field's counters then. The Lua side keeps, for the
// k-th effect, its userdata and the values of its Lua references:
// snapshot[EFFECTS][k] = { userdata, condition, cost, target, value,
// operation, label object } (see `ref_fields`).
struct BaseEffect {
	effect* peffect;
	std::unique_ptr<effect> pristine;
	bool registered;
	// Deleted during the duel (told by its userdata, not by its address,
	// which another effect may have taken since).
	bool gone;
};
struct Snapshot {
	std::vector<BaseEffect> effects;
	std::unordered_set<effect*> keep;
	field_info infos;
	uint32_t global_flag;
};
const char* const SNAPSHOT_CPP = "duelforge.snapshot.cpp";

Snapshot* cpp_snapshot(lua_State* L) {
	lua_getfield(L, LUA_REGISTRYINDEX, SNAPSHOT_CPP);
	auto* snap = static_cast<Snapshot*>(lua_touserdata(L, -1));
	lua_pop(L, 1);
	return snap;
}

// The registry references of an effect (0: none).
constexpr int REF_FIELDS = 6;
std::array<int32_t*, REF_FIELDS> ref_fields(effect& e) {
	return { &e.condition, &e.cost, &e.target, e.is_flag(EFFECT_FLAG_FUNC_VALUE) ? &e.value : nullptr, &e.operation, &e.label_object };
}
std::array<int32_t, REF_FIELDS> refs(effect& e) {
	std::array<int32_t, REF_FIELDS> out{};
	auto fields = ref_fields(e);
	for(int i = 0; i < REF_FIELDS; ++i)
		out[i] = fields[i] ? *fields[i] : 0;
	return out;
}

void visit(lua_State* L, int value, int snap);

// Copies the table at `t` (absolute index) unless already copied.
void visit_table(lua_State* L, int t, int snap) {
	lua_rawgeti(L, snap, COPIES);
	const int copies = lua_gettop(L);
	lua_pushvalue(L, t);
	if(lua_rawget(L, copies) != LUA_TNIL) {
		lua_pop(L, 2);
		return;
	}
	lua_pop(L, 1);
	lua_newtable(L);
	const int copy = lua_gettop(L);
	lua_pushvalue(L, t);
	lua_pushvalue(L, copy);
	lua_rawset(L, copies);
	lua_Integer count = 0;
	lua_pushnil(L);
	while(lua_next(L, t) != 0) {
		const int value = lua_gettop(L);
		const int key = value - 1;
		lua_pushvalue(L, key);
		lua_pushvalue(L, value);
		lua_rawset(L, copy);
		++count;
		visit(L, key, snap);
		visit(L, value, snap);
		lua_pop(L, 1);
	}
	lua_rawgeti(L, snap, COUNTS);
	lua_pushvalue(L, t);
	lua_pushinteger(L, count);
	lua_rawset(L, -3);
	lua_pop(L, 1);
	lua_rawgeti(L, snap, METAS);
	lua_pushvalue(L, t);
	if(!lua_getmetatable(L, t))
		lua_pushboolean(L, 0);
	lua_rawset(L, -3);
	lua_pop(L, 1);
	if(lua_getmetatable(L, t)) {
		visit(L, lua_gettop(L), snap);
		lua_pop(L, 1);
	}
	lua_pop(L, 2);
}

// Records the upvalues of the Lua function at `f`, each upvalue once (the
// functions of a script share its _ENV): [UPVALUES] holds (function, index,
// value) triples, [SEEN] the functions and upvalues already recorded.
void visit_function(lua_State* L, int f, int snap) {
	if(lua_iscfunction(L, f))
		return;
	lua_rawgeti(L, snap, SEEN);
	const int seen = lua_gettop(L);
	lua_pushvalue(L, f);
	if(lua_rawget(L, seen) != LUA_TNIL) {
		lua_pop(L, 2);
		return;
	}
	lua_pop(L, 1);
	lua_pushvalue(L, f);
	lua_pushboolean(L, 1);
	lua_rawset(L, seen);
	lua_rawgeti(L, snap, UPVALUES);
	const int list = lua_gettop(L);
	for(int i = 1; lua_getupvalue(L, f, i) != nullptr; ++i) {
		const int value = lua_gettop(L);
		lua_pushlightuserdata(L, lua_upvalueid(L, f, i));
		if(lua_rawget(L, seen) != LUA_TNIL) {
			lua_pop(L, 2);
			continue;
		}
		lua_pop(L, 1);
		lua_pushlightuserdata(L, lua_upvalueid(L, f, i));
		lua_pushboolean(L, 1);
		lua_rawset(L, seen);
		// Counted apart: a value can be nil.
		lua_getfield(L, list, "n");
		const auto n = lua_tointeger(L, -1);
		lua_pop(L, 1);
		lua_pushinteger(L, n + 3);
		lua_setfield(L, list, "n");
		lua_pushvalue(L, f);
		lua_rawseti(L, list, n + 1);
		lua_pushinteger(L, i);
		lua_rawseti(L, list, n + 2);
		lua_pushvalue(L, value);
		lua_rawseti(L, list, n + 3);
		visit(L, value, snap);
		lua_pop(L, 1);
	}
	lua_pop(L, 2);
}

void visit(lua_State* L, int value, int snap) {
	luaL_checkstack(L, 8, nullptr);
	switch(lua_type(L, value)) {
	case LUA_TTABLE:
		visit_table(L, lua_absindex(L, value), snap);
		break;
	case LUA_TFUNCTION:
		visit_function(L, lua_absindex(L, value), snap);
		break;
	default:
		break;
	}
}

// Puts the table at `t` back as its copy at `copy`, changing only what changed.
void restore_table(lua_State* L, int t, int copy, lua_Integer count) {
	lua_Integer seen = 0;
	bool same = true;
	lua_pushnil(L);
	while(lua_next(L, t) != 0) {
		++seen;
		lua_pushvalue(L, -2);
		lua_rawget(L, copy);
		if(!lua_rawequal(L, -1, -2)) {
			same = false;
			// Assigning an existing field (or nil) is allowed during lua_next.
			lua_pushvalue(L, -3);
			lua_pushvalue(L, -2);
			lua_rawset(L, t);
		}
		lua_pop(L, 2);
	}
	if(same && seen == count)
		return;
	// Fields taken out since: put back.
	lua_pushnil(L);
	while(lua_next(L, copy) != 0) {
		lua_pushvalue(L, -2);
		if(lua_rawget(L, t) == LUA_TNIL) {
			lua_pushvalue(L, -3);
			lua_pushvalue(L, -3);
			lua_rawset(L, t);
		}
		lua_pop(L, 2);
	}
}

void restore(lua_State* L, int snap) {
	luaL_checkstack(L, 12, nullptr);
	lua_rawgeti(L, snap, COPIES);
	const int copies = lua_gettop(L);
	lua_rawgeti(L, snap, COUNTS);
	const int counts = lua_gettop(L);
	lua_rawgeti(L, snap, METAS);
	const int metas = lua_gettop(L);
	lua_pushnil(L);
	while(lua_next(L, copies) != 0) {
		const int copy = lua_gettop(L);
		const int t = copy - 1;
		lua_pushvalue(L, t);
		lua_rawget(L, counts);
		const auto count = lua_tointeger(L, -1);
		lua_pop(L, 1);
		restore_table(L, t, copy, count);
		lua_pushvalue(L, t);
		lua_rawget(L, metas);
		const int meta = lua_gettop(L);
		const bool had = lua_toboolean(L, meta);
		if(lua_getmetatable(L, t)) {
			if(!had || !lua_rawequal(L, -1, meta)) {
				if(had)
					lua_pushvalue(L, meta);
				else
					lua_pushnil(L);
				lua_setmetatable(L, t);
			}
			lua_pop(L, 1);
		} else if(had) {
			lua_pushvalue(L, meta);
			lua_setmetatable(L, t);
		}
		lua_pop(L, 2);
	}
	lua_pop(L, 3);
	lua_rawgeti(L, snap, UPVALUES);
	const int list = lua_gettop(L);
	lua_getfield(L, list, "n");
	const auto n = lua_tointeger(L, -1);
	lua_pop(L, 1);
	for(lua_Integer k = 1; k <= n; k += 3) {
		lua_rawgeti(L, list, k);
		const int f = lua_gettop(L);
		const auto i = static_cast<int>((lua_rawgeti(L, list, k + 1), lua_tointeger(L, -1)));
		lua_pop(L, 1);
		lua_getupvalue(L, f, i);
		lua_rawgeti(L, list, k + 2);
		if(!lua_rawequal(L, -1, -2)) {
			lua_setupvalue(L, f, i);
			lua_pop(L, 2);
		} else {
			lua_pop(L, 3);
		}
	}
	lua_pop(L, 1);
}

// The Lua object of `obj` no longer points to it, and the registry lets it go.
void forget(interpreter* lua, lua_obj* obj) {
	if(!obj->ref_handle)
		return;
	lua_State* L = lua->lua_state;
	lua_rawgeti(L, LUA_REGISTRYINDEX, obj->ref_handle);
	if(auto** lobj = static_cast<lua_obj**>(lua_touserdata(L, -1)))
		*lobj = &lua->deleted;
	lua_pop(L, 1);
	luaL_unref(L, LUA_REGISTRYINDEX, obj->ref_handle);
	obj->ref_handle = 0;
}
}

extern "C" {
// Copies the Lua state's tables as they are now (call it once the base
// scripts are loaded). Returns 0 on a Lua error.
int df_duel_snapshot(OCG_Duel ocg_duel) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	lua_State* L = pduel->lua->lua_state;
	const int top = lua_gettop(L);
	auto cpp = std::make_unique<Snapshot>();
	auto& field = *pduel->game_field;
	for(auto* peffect : pduel->effects) {
		cpp->effects.push_back({ peffect, std::make_unique<effect>(*peffect), field.effects.indexer.count(peffect) > 0, false });
		cpp->keep.insert(peffect);
	}
	// Registered again in the order they were.
	std::sort(cpp->effects.begin(), cpp->effects.end(), [](const auto& a, const auto& b) { return a.pristine->id < b.pristine->id; });
	cpp->infos = field.infos;
	cpp->global_flag = field.core.global_flag;
	try {
		luaL_checkstack(L, 16, nullptr);
		lua_createtable(L, 6, 0);
		const int snap = lua_gettop(L);
		for(int i = COPIES; i <= EFFECTS; ++i) {
			lua_newtable(L);
			lua_rawseti(L, snap, i);
		}
		// The base effects' userdata and Lua references; what they reach is
		// put back too (proc_workaround.lua's hand shuffle keeps, in an
		// upvalue of its operation, whose hand was shuffled in the chain).
		lua_rawgeti(L, snap, EFFECTS);
		const int saved = lua_gettop(L);
		for(size_t k = 0; k < cpp->effects.size(); ++k) {
			auto& base = cpp->effects[k];
			lua_createtable(L, REF_FIELDS + 1, 0);
			lua_rawgeti(L, LUA_REGISTRYINDEX, base.peffect->ref_handle);
			lua_rawseti(L, -2, 1);
			auto r = refs(*base.peffect);
			for(int i = 0; i < REF_FIELDS; ++i) {
				if(r[i]) {
					lua_rawgeti(L, LUA_REGISTRYINDEX, r[i]);
					visit(L, lua_gettop(L), snap);
					lua_rawseti(L, -2, i + 2);
				}
			}
			lua_rawseti(L, saved, static_cast<lua_Integer>(k + 1));
		}
		lua_pop(L, 1);
		lua_rawgeti(L, LUA_REGISTRYINDEX, LUA_RIDX_GLOBALS);
		visit(L, lua_gettop(L), snap);
		lua_pop(L, 1);
		lua_rawgeti(L, LUA_REGISTRYINDEX, pduel->lua->loaded_scripts_table);
		visit(L, lua_gettop(L), snap);
		lua_pop(L, 1);
		lua_pushnil(L);
		lua_rawseti(L, snap, SEEN);
		lua_setfield(L, LUA_REGISTRYINDEX, SNAPSHOT);
	} catch(...) {
		lua_settop(L, top);
		return 0;
	}
	lua_settop(L, top);
	lua_pushlightuserdata(L, cpp.release());
	lua_setfield(L, LUA_REGISTRYINDEX, SNAPSHOT_CPP);
	return 1;
}

// Loads the card scripts of `codes` not loaded yet, in that order, before
// any card is added, and takes the snapshot again: the next duels keep
// these scripts as they are once loaded, and only run each card's
// initial_effect. Returns 0 when loading them changed more than Lua tables
// (the duel then differs from a new one and must not be used).
int df_duel_preload(OCG_Duel ocg_duel, const uint32_t* codes, size_t count) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	auto* lua = pduel->lua;
	lua_State* L = lua->lua_state;
	auto* cpp = cpp_snapshot(L);
	if(!cpp)
		return 1;
	std::vector<uint32_t> scripts;
	for(size_t i = 0; i < count; ++i) {
		// The script a card runs (interpreter::register_card).
		const auto& data = pduel->read_card(codes[i]);
		uint32_t code = codes[i];
		if(data.alias && data.alias < code + 10 && code < data.alias + 10)
			code = data.alias;
		if(std::find(scripts.begin(), scripts.end(), code) != scripts.end())
			continue;
		char name[32];
		lua_getglobal(L, interpreter::format_to(name, "c%u", code));
		if(lua_isnil(L, -1))
			scripts.push_back(code);
		lua_pop(L, 1);
	}
	if(scripts.empty())
		return 1;
	auto& field = *pduel->game_field;
	const auto effects = pduel->effects.size();
	const auto cards = pduel->cards.size();
	const auto infos = field.infos;
	const auto global_flag = field.core.global_flag;
	const int top = lua_gettop(L);
	try {
		for(auto code : scripts) {
			lua->load_card_script(code);
			lua_settop(L, top);
		}
	} catch(...) {
		lua_settop(L, top);
		return 0;
	}
	const auto& now = field.infos;
	if(pduel->effects.size() != effects || pduel->cards.size() != cards || now.event_id != infos.event_id || now.field_id != infos.field_id
	   || now.copy_id != infos.copy_id || now.card_id != infos.card_id || field.core.global_flag != global_flag)
		return 0;
	delete cpp;
	lua_pushnil(L);
	lua_setfield(L, LUA_REGISTRYINDEX, SNAPSHOT_CPP);
	return df_duel_snapshot(ocg_duel);
}

// Destroys a duel and its snapshot.
void df_duel_destroy(OCG_Duel ocg_duel) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	if(!pduel)
		return;
	std::unique_ptr<Snapshot> snap{ cpp_snapshot(pduel->lua->lua_state) };
	OCG_DestroyDuel(ocg_duel);
}

// Empties the duel for a new one with `options` (see the top of the file).
// Returns 0 when the duel cannot be reused (no snapshot, a Lua error, a base
// effect changed in a way that cannot be put back).
int df_duel_reset(OCG_Duel ocg_duel, const OCG_DuelOptions* options) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	auto* lua = pduel->lua;
	lua_State* L = lua->lua_state;
	const int top = lua_gettop(L);
	auto* cpp = cpp_snapshot(L);
	if(!cpp)
		return 0;
	card* old_temp = pduel->game_field->temp_card;
	for(const auto& base : cpp->effects) {
		const auto& p = *base.pristine;
		for(card* c : { p.owner, p.handler, p.active_handler }) {
			if(c && c != old_temp)
				return 0;
		}
	}
	try {
		luaL_checkstack(L, 4, nullptr);
		if(lua_getfield(L, LUA_REGISTRYINDEX, SNAPSHOT) != LUA_TTABLE) {
			lua_settop(L, top);
			return 0;
		}
		restore(L, lua_gettop(L));
		lua_settop(L, top);
		lua_getfield(L, LUA_REGISTRYINDEX, SNAPSHOT);
		lua_rawgeti(L, -1, EFFECTS);
		for(size_t k = 0; k < cpp->effects.size(); ++k) {
			auto& base = cpp->effects[k];
			lua_rawgeti(L, -1, static_cast<lua_Integer>(k + 1));
			lua_rawgeti(L, -1, 1);
			base.gone = *static_cast<lua_obj**>(lua_touserdata(L, -1)) != base.peffect;
			if(base.gone)
				cpp->keep.erase(base.peffect);
			lua_pop(L, 2);
		}
		lua_settop(L, top);
		// Coroutines left waiting for an answer.
		for(auto& [function, thread] : lua->coroutines)
			luaL_unref(L, LUA_REGISTRYINDEX, thread.second);
		lua->coroutines.clear();
		lua->params.clear();
		lua->call_depth = 0;
		lua->no_action = 0;
		lua->current_state = L;
		for(auto* pcard : pduel->cards)
			forget(lua, pcard);
		pduel->assumes.clear();
		pduel->uncopy.clear();
		const df_reset_info info{ options, &cpp->keep };
		df_reset = &info;
		pduel->clear();
		df_reset = nullptr;
		pduel->buff.clear();
		auto& field = *pduel->game_field;
		card* temp = field.temp_card;
		lua_getfield(L, LUA_REGISTRYINDEX, SNAPSHOT);
		lua_rawgeti(L, -1, EFFECTS);
		const int saved = lua_gettop(L);
		for(size_t k = 0; k < cpp->effects.size(); ++k) {
			auto& base = cpp->effects[k];
			auto& pristine = *base.pristine;
			for(card** c : { &pristine.owner, &pristine.handler, &pristine.active_handler }) {
				if(*c)
					*c = temp;
			}
			lua_rawgeti(L, saved, static_cast<lua_Integer>(k + 1));
			const int lua_side = lua_gettop(L);
			// Reset during the duel (EVENT_STARTUP's effect resets itself):
			// made again, behind its old userdata. Its references went with it.
			const bool gone = base.gone;
			if(gone) {
				base.peffect = new effect(pduel);
				pduel->effects.insert(base.peffect);
				cpp->keep.insert(base.peffect);
				lua_rawgeti(L, lua_side, 1);
				*static_cast<lua_obj**>(lua_touserdata(L, -1)) = base.peffect;
				pristine.ref_handle = luaL_ref(L, LUA_REGISTRYINDEX);
			}
			const auto now = refs(*base.peffect);
			const auto fields = ref_fields(pristine);
			for(int i = 0; i < REF_FIELDS; ++i) {
				const bool same = !gone && fields[i] && now[i] == *fields[i];
				if(same)
					continue;
				// Changed during the duel: the snapshot's value again.
				if(!gone && now[i])
					luaL_unref(L, LUA_REGISTRYINDEX, now[i]);
				if(!fields[i])
					continue;
				lua_rawgeti(L, lua_side, i + 2);
				if(lua_isnil(L, -1)) {
					lua_pop(L, 1);
					*fields[i] = 0;
				} else {
					*fields[i] = luaL_ref(L, LUA_REGISTRYINDEX);
				}
			}
			lua_pop(L, 1);
			*base.peffect = pristine;
			if(base.registered)
				field.add_effect(base.peffect, pristine.effect_owner);
		}
		lua_settop(L, top);
		field.infos = cpp->infos;
		field.core.global_flag = cpp->global_flag;
	} catch(...) {
		df_reset = nullptr;
		lua_settop(L, top);
		return 0;
	}
	return 1;
}
}
