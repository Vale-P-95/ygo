//! Statistics for DuelForge's lab (Il Laboratorio): exact draw
//! probabilities over card roles, Monte Carlo simulation, deck optimization
//! and match-log statistics. No game engine involved: cards are counted by
//! the roles the player gives them.

pub mod cond;
pub mod exact;
pub mod matches;
pub mod optimize;
pub mod overview;
pub mod prefs;
pub mod sim;
pub mod special;

pub use cond::{Cond, Op, Target};
pub use exact::{probability, CardEntry, DeckModel, ExactError};
pub use matches::{report as match_report, sample_size, MatchInput, Report as MatchReport};
pub use optimize::{
    climb, heatmap, optimize, sensitivity, size_curve, ClimbConfig, ClimbResult, Heatmap, Judged, OptimizeConfig, OptimizeError,
    OptimizeResult, Sensitivity, Tried,
};
pub use overview::{overview, Overview};
pub use prefs::{learn_weights, Choice, Learned};
pub use sim::{simulate, CardRule, Checkpoint, Effect, SimConfig, SimResult};
pub use special::{special, Facts, Mechanism, SpecialInput, SpecialReport};

/// Role keys the app knows by name; any other key is a custom role.
pub mod roles {
    pub const STARTER: &str = "starter";
    pub const EXTENDER: &str = "extender";
    pub const HANDTRAP: &str = "handtrap";
    pub const BREAKER: &str = "breaker";
    pub const BRICK: &str = "brick";
}
