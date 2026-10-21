// The engine's C++ objects (cards, effects, groups, the containers of the
// field) are allocated through mimalloc too, like Lua's: a duel makes and
// frees millions of small ones, and the C runtime's malloc and free were an
// eighth of a combo search. mimalloc's own replacements of the global new
// and delete operators, in this one file of the program.
#include "mimalloc-new-delete.h"
