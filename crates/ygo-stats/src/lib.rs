//! Statistics for DuelForge's lab (Il Laboratorio): exact draw
//! probabilities over card roles, Monte Carlo simulation, deck optimization
//! and match-log statistics. No game engine involved: cards are counted by
//! the roles the player gives them.

pub mod cond;
pub mod exact;
pub mod overview;

pub use cond::{Cond, Op, Target};
pub use exact::{probability, CardEntry, DeckModel, ExactError};
pub use overview::{overview, Overview};

/// Role keys the app knows by name; any other key is a custom role.
pub mod roles {
    pub const STARTER: &str = "starter";
    pub const EXTENDER: &str = "extender";
    pub const HANDTRAP: &str = "handtrap";
    pub const BREAKER: &str = "breaker";
    pub const BRICK: &str = "brick";
}
