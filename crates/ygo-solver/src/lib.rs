//! The Combo Lab's search: from a hand, the duel engine plays every line by
//! itself and collects the boards reached at the end of the turn, each with
//! the shortest line that reaches it.

pub mod card;
pub mod explain;
pub mod search;

pub use card::{resume_card, sample_hands, search_card, CardSearch, HandResult};
pub use explain::{explain, ExplainedStep};
pub use search::{quick_score, resume, search, Board, Frontier, Goal, Line, Progress, SearchConfig, SearchResult, Stop};
