//! Rust bindings to the EDOPro duel engine (ygopro-core) and the data it
//! needs: card databases, card scripts and deck lists.
//!
//! ```no_run
//! use ygo_engine::{actions, DeckList, Duel, Engine, Step};
//!
//! let engine = Engine::load("data".as_ref())?;
//! let deck = DeckList::from_ydk(&std::fs::read_to_string("deck.ydk")?)?;
//! let mut duel = Duel::goldfish(&engine, &deck, &deck.main[..5], 42)?;
//! if let Step::Decision { decision, .. } = duel.advance()? {
//!     for action in actions(&decision, engine.cards()) {
//!         println!("{}", action.label);
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

// Lua's allocator (build.rs) is called only from C++: without this use,
// rustc would not link the crate and mi_free would be undefined.
use libmimalloc_sys as _;

mod actions;
mod announce;
pub mod auto;
mod carddb;
pub mod consts;
mod deck;
mod duel;
mod ffi;
pub mod italian;
mod message;
mod script;
mod scripts;
pub mod state;
pub mod sweep;

pub use actions::{actions, actions_with, describe_card, without_labels, Action, ActionKind, MAX_ANNOUNCE};
pub use announce::is_declarable;
pub use carddb::{Card, CardDb};
pub use deck::DeckList;
pub use duel::{
    arena_duels, fresh_duels, generic_materials, shuffle, ChainLink, Duel, DuelConfig, Engine, Opponent, Placement, Prelude, SavedDuel,
    Step, STARTING_LP,
};
pub use message::{Activatable, CardRef, Decision, Event};
pub use script::{run_script, ScriptEnd, Trace};
pub use scripts::ScriptStore;
pub use state::{canonical_fingerprint, fingerprint, DuelState, FieldCard, PlayerState, TurnActivity};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("data error: {0}")]
    Data(String),
    #[error("card database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("engine error: {0}")]
    Engine(String),
    #[error("the engine rejected the response: {0}")]
    Rejected(String),
}
