//! The Combo Lab's search: from a hand, the duel engine plays every line by
//! itself and collects the boards reached at the end of the turn, each with
//! the shortest line that reaches it.

pub mod explain;
pub mod search;

pub use explain::{explain, ExplainedStep};
pub use search::{search, Board, Line, Progress, SearchConfig, SearchResult, Stop};
