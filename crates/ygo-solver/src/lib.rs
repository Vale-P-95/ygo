//! The Combo Lab's search: from a hand, the duel engine plays every line by
//! itself and collects the boards reached at the end of the turn, each with
//! the shortest line that reaches it.

pub mod card;
pub mod explain;
pub mod probe;
pub mod roles;
pub mod search;
pub mod value;

pub use card::{resume_card, sample_hands, search_card, CardSearch, HandResult};
pub use explain::{explain, ExplainedStep, Happening, HappeningKind};
pub use probe::{hand_traps, BreakerResult, Effect, Interruption, Kit, Measure, Probe, Prober, ResponderId};
pub use roles::{guess_roles, RoleGuess};
pub use search::{
    look_key, quick_score, resume, search, Board, Frontier, Goal, GoalCard, GoalZone, Line, Progress, SearchConfig, SearchResult, Stop,
};
pub use value::{card_values, CardValues, Spot};
