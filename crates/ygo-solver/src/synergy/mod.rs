//! The synergies between archetypes, the section "Il Crogiolo" (plan
//! `docs/plans/sinergie-archetipi.md`): what the effects of an archetype
//! accept from another, read from the engine. Phase F0: the census of the
//! pool and the reading deck, a duel holding the whole pool.

pub mod census;
pub mod read;

pub use census::{archetype_names, base_archetype, census, Census, OfferShape};
pub use read::{blanks, compare, read_card, Agreement, Offer, Pool, ReadBudget, Reading};
