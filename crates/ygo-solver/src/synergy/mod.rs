//! The synergies between archetypes, the section "Il Crogiolo" (plan
//! `docs/plans/sinergie-archetipi.md`): what the effects of an archetype
//! accept from another, read from the engine. Phase F0: the census of the
//! pool and the reading deck, a duel holding the whole pool. Phase F1: the
//! reading of every card from the hand, the field and the GY, effect by
//! effect, with a rule people can read; the materials of the Extra Deck;
//! the profiles of the archetypes; the store in `data/synergy/`.

pub mod accept;
pub mod cache;
pub mod census;
pub mod profile;
pub mod read;
pub mod rule;

pub use accept::{accepts, Accepted, Accepts, Coverage};
pub use cache::Store;
pub use census::{archetype_names, base_archetype, census, Census, OfferShape};
pub use profile::{profile, Profile};
pub use read::{blanks, compare, read_card, read_card_in, Agreement, GraveSample, Offer, Place, Pool, ReadBudget, Reading, Scene};
pub use rule::{learn, Condition, Rule};
