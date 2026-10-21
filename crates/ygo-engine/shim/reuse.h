// What duel::clear (compiled from a patched copy of duel.cpp, see build.rs)
// needs to empty a duel for the next one (shim/reuse.cpp).
#ifndef DF_REUSE_H_
#define DF_REUSE_H_

#include <unordered_set>
#include "ocgapi_types.h"

class effect;

struct df_reset_info {
	// The next duel's options: the new field's, and the random seed.
	const OCG_DuelOptions* options;
	// Effects created by the base scripts: kept, put back by the shim.
	const std::unordered_set<effect*>* keep;
};

extern thread_local const df_reset_info* df_reset;

#endif /* DF_REUSE_H_ */
