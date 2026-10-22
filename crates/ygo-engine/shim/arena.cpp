// A duel's memory in an arena, so the whole duel can be saved and put back
// by copying memory, Lua coroutines included (plan B13 in
// docs/plans/velocita-ricerca.md). While an arena is active on a thread,
// Lua's allocator (lauxlib.c's l_alloc, patched in build.rs) and the C++
// new/delete go to it; elsewhere they go to mimalloc. The allocator keeps
// its state in the arena's first bytes, so a copy of the used part is a save
// and copying it back at the same address puts the duel back: every pointer
// stays valid.
//
// Arenas live in fixed slots of one address range, so a pointer tells its
// arena by its address alone (a free goes back to its arena whichever one is
// active). Address space is reserved, not memory: pages are committed as the
// arena grows (made writable on Linux, so a system that does not overcommit
// counts only those).
//
// A Lua state allocates in the arena that was active when it was made (its
// allocator's `ud`), whichever one is active when it runs.
#if defined(_WIN32)
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#else
#include <sys/mman.h>
#ifndef MAP_FIXED_NOREPLACE
// Older systems: a hint only, checked below.
#define MAP_FIXED_NOREPLACE 0
#endif
#endif

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <new>

#include "mimalloc.h"

namespace {

constexpr std::uintptr_t RANGE_BASE = std::uintptr_t(0x5000) << 32;
constexpr std::uintptr_t SLOT_SIZE = std::uintptr_t(4) << 30;
constexpr size_t SLOTS = 256;
// Committed in steps of this much.
constexpr size_t COMMIT_STEP = size_t(1) << 20;

constexpr size_t SMALL = 2048;
constexpr size_t SMALL_CLASSES = SMALL / 16;
constexpr size_t CLASSES = SMALL_CLASSES + 21;
constexpr size_t HEADER = 16;

struct State {
	size_t top;   // bytes used from the base, this header included
	size_t live;  // bytes in blocks not freed
	void* free[CLASSES];
};

struct Arena {
	unsigned char* base;
	size_t committed;
	size_t slot;
	// The arena takes nothing past this (SLOT_SIZE; less in tests).
	size_t limit;
	// An allocation that did not fit, or that the arena cannot take, went to
	// mimalloc: what it holds is not in a save any more. Never cleared, as
	// the block stays in the duel.
	bool overflowed;
	State* state() const { return reinterpret_cast<State*>(base); }
};

std::atomic<Arena*> slots[SLOTS];
thread_local Arena* active = nullptr;

Arena* arena_of(const void* p) {
	const auto a = reinterpret_cast<std::uintptr_t>(p);
	if(a < RANGE_BASE || a >= RANGE_BASE + SLOT_SIZE * SLOTS)
		return nullptr;
	return slots[(a - RANGE_BASE) / SLOT_SIZE].load(std::memory_order_acquire);
}

bool commit(Arena* a, size_t upto) {
	if(upto > a->limit)
		return false;
	if(upto <= a->committed)
		return true;
	size_t want = (upto + COMMIT_STEP - 1) / COMMIT_STEP * COMMIT_STEP;
	if(want > a->limit)
		want = a->limit;
#if defined(_WIN32)
	if(!VirtualAlloc(a->base + a->committed, want - a->committed, MEM_COMMIT, PAGE_READWRITE))
		return false;
#else
	if(mprotect(a->base + a->committed, want - a->committed, PROT_READ | PROT_WRITE) != 0)
		return false;
#endif
	a->committed = want;
	return true;
}

// The size class of a block of n bytes; CLASSES when no slot can hold it.
size_t class_of(size_t n, size_t* block) {
	if(n > SLOT_SIZE) {
		*block = 0;
		return CLASSES;
	}
	if(n <= SMALL) {
		const size_t c = n == 0 ? 1 : (n + 15) / 16;
		*block = c * 16;
		return c - 1;
	}
	size_t c = 0, b = SMALL * 2;
	while(b < n) {
		b <<= 1;
		++c;
	}
	*block = b;
	return SMALL_CLASSES + c;
}

size_t block_size(size_t c) {
	return c < SMALL_CLASSES ? (c + 1) * 16 : (SMALL * 2) << (c - SMALL_CLASSES);
}

void* arena_alloc(Arena* a, size_t n) {
	size_t block;
	const size_t c = class_of(n, &block);
	if(c >= CLASSES)
		return nullptr;
	State* s = a->state();
	void* p = s->free[c];
	if(p) {
		s->free[c] = *static_cast<void**>(p);
	} else {
		const size_t end = s->top + HEADER + block;
		if(!commit(a, end))
			return nullptr;
		unsigned char* at = a->base + s->top;
		s->top = end;
		*reinterpret_cast<size_t*>(at) = c;
		p = at + HEADER;
	}
	s->live += block;
	return p;
}

size_t class_at(const void* p) {
	return *reinterpret_cast<const size_t*>(static_cast<const unsigned char*>(p) - HEADER);
}

void arena_free(Arena* a, void* p) {
	const size_t c = class_at(p);
	State* s = a->state();
	*static_cast<void**>(p) = s->free[c];
	s->free[c] = p;
	s->live -= block_size(c);
}

// From arena `a`; when it is full, from mimalloc, and the arena no longer
// holds the whole duel. Null only for `nothrow`, else std::bad_alloc.
void* allocate(Arena* a, size_t n, bool nothrow) {
	if(void* p = arena_alloc(a, n))
		return p;
	a->overflowed = true;
	return nothrow ? mi_malloc(n) : mi_new(n);
}

void release(void* p) {
	if(Arena* a = arena_of(p))
		arena_free(a, p);
	else
		mi_free(p);
}

} // namespace

extern "C" {

// A new arena, or null when no slot can be reserved.
void* df_arena_new() {
	for(size_t i = 0; i < SLOTS; ++i) {
		Arena* expected = nullptr;
		// Claimed first, so two threads do not reserve the same slot.
		auto* a = static_cast<Arena*>(mi_malloc(sizeof(Arena)));
		if(!slots[i].compare_exchange_strong(expected, a)) {
			mi_free(a);
			continue;
		}
		void* want = reinterpret_cast<void*>(RANGE_BASE + i * SLOT_SIZE);
#if defined(_WIN32)
		void* got = VirtualAlloc(want, SLOT_SIZE, MEM_RESERVE, PAGE_READWRITE);
#else
		void* got = mmap(want, SLOT_SIZE, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE | MAP_FIXED_NOREPLACE, -1, 0);
		if(got == MAP_FAILED)
			got = nullptr;
		else if(got != want) {
			munmap(got, SLOT_SIZE);
			got = nullptr;
		}
#endif
		if(!got) {
			slots[i].store(nullptr);
			mi_free(a);
			continue;
		}
		*a = Arena{ static_cast<unsigned char*>(got), 0, i, SLOT_SIZE, false };
		const size_t header = (sizeof(State) + 15) & ~size_t(15);
		if(!commit(a, header)) {
			slots[i].store(nullptr);
#if defined(_WIN32)
			VirtualFree(got, 0, MEM_RELEASE);
#else
			munmap(got, SLOT_SIZE);
#endif
			mi_free(a);
			return nullptr;
		}
		std::memset(a->base, 0, sizeof(State));
		a->state()->top = header;
		return a;
	}
	return nullptr;
}

// Gives the arena's memory back. Nothing allocated in it may be used after.
void df_arena_free(void* arena) {
	auto* a = static_cast<Arena*>(arena);
	if(!a)
		return;
	if(active == a)
		active = nullptr;
#if defined(_WIN32)
	VirtualFree(a->base, 0, MEM_RELEASE);
#else
	munmap(a->base, SLOT_SIZE);
#endif
	slots[a->slot].store(nullptr, std::memory_order_release);
	mi_free(a);
}

// Allocations on this thread go to `arena` (null: to mimalloc), and Lua
// states made now go to it for good. Returns the arena active before, to put
// back.
void* df_arena_enter(void* arena) {
	Arena* before = active;
	active = static_cast<Arena*>(arena);
	return before;
}

size_t df_arena_used(void* arena) {
	return static_cast<Arena*>(arena)->state()->top;
}

size_t df_arena_live(void* arena) {
	return static_cast<Arena*>(arena)->state()->live;
}

int df_arena_overflowed(void* arena) {
	return static_cast<Arena*>(arena)->overflowed ? 1 : 0;
}

// For tests: the arena takes nothing new past `bytes`, as if it were full.
void df_arena_limit(void* arena, size_t bytes) {
	static_cast<Arena*>(arena)->limit = bytes;
}

// Copies the used part of the arena into `out` (df_arena_used bytes).
void df_arena_save(void* arena, void* out) {
	auto* a = static_cast<Arena*>(arena);
	std::memcpy(out, a->base, a->state()->top);
}

// Puts back `size` bytes saved by df_arena_save from the same arena. Returns
// 0, with nothing changed, if the arena cannot hold them.
int df_arena_restore(void* arena, const void* in, size_t size) {
	auto* a = static_cast<Arena*>(arena);
	if(size > a->committed && !commit(a, size))
		return 0;
	std::memcpy(a->base, in, size);
	return 1;
}

// The `ud` of a Lua state's allocator: the arena active when it is made.
void* df_lua_arena() {
	return active;
}

// Lua's allocator: the state's arena (`ud`) or, without one, mimalloc. On
// failure the old block stays, as Lua wants.
void* df_lua_alloc(void* ud, void* ptr, size_t osize, size_t nsize) {
	if(nsize == 0) {
		if(ptr)
			release(ptr);
		return nullptr;
	}
	auto* home = static_cast<Arena*>(ud);
	if(!home)
		return mi_realloc(ptr, nsize);
	void* p = allocate(home, nsize, true);
	if(p && ptr) {
		std::memcpy(p, ptr, osize < nsize ? osize : nsize);
		release(ptr);
	}
	return p;
}

// Checks the allocator without a duel: blocks in an arena, a save, changes,
// a restore; a full arena; every slot taken. Returns 1 when everything is as saved. Addresses are kept as
// integers: a block freed and then restored is live again, which the
// compiler cannot know (it takes a block from new for one nothing else
// writes to).
int df_arena_selftest() {
	void* arena = df_arena_new();
	if(!arena)
		return 0;
	void* before = df_arena_enter(arena);
	auto* made = new int[1000];
	for(int i = 0; i < 1000; ++i)
		made[i] = i;
	auto* text = new char[64];
	std::strcpy(text, "saved");
	df_arena_enter(before);
	const auto numbers = reinterpret_cast<std::uintptr_t>(made);
	const auto words = reinterpret_cast<std::uintptr_t>(text);
	bool same = arena_of(made) == arena && arena_of(text) == arena;
	const size_t used = df_arena_used(arena);
	auto* copy = static_cast<unsigned char*>(mi_malloc(used));
	df_arena_save(arena, copy);
	before = df_arena_enter(arena);
	delete[] made;
	auto* other = new long long[500];
	for(int i = 0; i < 500; ++i)
		other[i] = -1;
	std::strcpy(text, "changed");
	df_arena_enter(before);
	// The freed block was taken again.
	same = same && reinterpret_cast<std::uintptr_t>(other) == numbers;
	df_arena_restore(arena, copy, used);
	auto* restored = reinterpret_cast<volatile int*>(numbers);
	auto* saved_text = reinterpret_cast<volatile char*>(words);
	same = same && saved_text[0] == 's' && saved_text[4] == 'd' && saved_text[5] == 0;
	for(int i = 0; i < 1000 && same; ++i)
		same = restored[i] == i;
	// The free lists are back as saved: a block of that size now comes
	// from the top.
	before = df_arena_enter(arena);
	auto* again = new long long[500];
	df_arena_enter(before);
	same = same && df_arena_used(arena) == used + HEADER + 4096 && arena_of(again) == arena;
	mi_free(copy);
	df_arena_free(arena);

	// A full arena: the block comes from mimalloc and the arena says so.
	// Explicit operator calls, which the compiler does not leave out.
	arena = df_arena_new();
	auto* full = static_cast<Arena*>(arena);
	full->limit = full->committed;
	before = df_arena_enter(arena);
	void* big = ::operator new(size_t(2) << 20);
	df_arena_enter(before);
	same = same && !arena_of(big) && df_arena_overflowed(arena) == 1;
	::operator delete(big);
	// Lua's allocator keeps the old block when it cannot grow it.
	void* block = df_lua_alloc(arena, nullptr, 0, 32);
	same = same && arena_of(block) == arena;
	same = same && !df_lua_alloc(arena, block, 32, SIZE_MAX - 4096) && class_at(block) == 1;
	df_lua_alloc(arena, block, 32, 0);
	df_arena_free(arena);

	// Every slot taken: no arena, until one is freed.
	static void* all[SLOTS];
	size_t taken = 0;
	while(taken < SLOTS && (all[taken] = df_arena_new()))
		++taken;
	same = same && taken > 0 && !df_arena_new();
	df_arena_free(all[--taken]);
	all[taken] = df_arena_new();
	same = same && all[taken];
	for(size_t i = 0; i <= taken; ++i)
		df_arena_free(all[i]);
	return same ? 1 : 0;
}
}

// The C++ objects of the core (in place of mimalloc-new-delete.h).
void* operator new(size_t n) {
	return active ? allocate(active, n, false) : mi_new(n);
}
void* operator new[](size_t n) {
	return operator new(n);
}
void* operator new(size_t n, const std::nothrow_t&) noexcept {
	return active ? allocate(active, n, true) : mi_new_nothrow(n);
}
void* operator new[](size_t n, const std::nothrow_t& t) noexcept {
	return operator new(n, t);
}
void operator delete(void* p) noexcept {
	if(p)
		release(p);
}
void operator delete[](void* p) noexcept {
	operator delete(p);
}
void operator delete(void* p, size_t) noexcept {
	operator delete(p);
}
void operator delete[](void* p, size_t) noexcept {
	operator delete(p);
}
void operator delete(void* p, const std::nothrow_t&) noexcept {
	operator delete(p);
}
void operator delete[](void* p, const std::nothrow_t&) noexcept {
	operator delete(p);
}
// Over-aligned objects: the core has none; mimalloc's, outside any arena,
// so an arena active then can no longer be saved.
void* operator new(size_t n, std::align_val_t al) {
	if(active)
		active->overflowed = true;
	return mi_new_aligned(n, static_cast<size_t>(al));
}
void* operator new[](size_t n, std::align_val_t al) {
	return operator new(n, al);
}
void operator delete(void* p, std::align_val_t) noexcept {
	mi_free(p);
}
void operator delete[](void* p, std::align_val_t) noexcept {
	mi_free(p);
}
void operator delete(void* p, size_t, std::align_val_t) noexcept {
	mi_free(p);
}
void operator delete[](void* p, size_t, std::align_val_t) noexcept {
	mi_free(p);
}
