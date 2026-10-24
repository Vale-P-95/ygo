//! The Combo Lab's search: from a hand, the duel engine plays every line by
//! itself and collects the boards reached at the end of the turn, each with
//! the shortest line that reaches it.

pub mod card;
pub mod coverage;
pub mod engine_mc;
pub mod explain;
mod follow;
pub mod goal;
pub mod interaction;
pub mod links;
pub mod matchup;
pub mod meta;
pub mod probe;
pub mod roles;
pub mod search;
pub mod second;
pub mod value;

pub use card::{resume_card, sample_hands, search_card, CardSearch, HandResult};
pub use coverage::{Counts, Coverage, Kind as MechanicKind, Mechanic};
pub use explain::{explain, exposure, ExplainedStep, Exposure, Happening, HappeningKind};
pub use goal::{Goal, GoalCard, GoalHits, GoalZone};
pub use interaction::{interaction_value, InteractionProfile};
pub use links::{
    compare_links, deck_links, deck_links_again, hands_reach, random_hands, CardChange, CardLink, DeckLinks, HandReach, LinkCard, LinkKind,
    LinksBudget, LinksDiff, LinksWork, Zone as LinkZone,
};
pub use matchup::{
    against_moves, draw_trade_off, line_stops, matchup, plays_of, rival_line, rival_lines, rival_plays, DrawTradeOff, HandTrapMoment,
    LineStop, Matchup, MoveStop, MovesMatchup, Rival, RivalBreaker, RivalHandTrap, RivalLine, RivalMove, RivalPlay, StopPoint,
};
pub use meta::{
    meta_rivals, meta_score, most_played, tech_table, MetaDeck, MetaDeckScore, MetaRival, MetaScore, TechAgainst, TechDeck, TechRow,
    TechTable,
};
pub use probe::{hand_traps, BreakerResult, Effect, FollowUp, Interruption, Kit, Measure, Probe, Prober, Protection, Removal, ResponderId};
pub use roles::{guess_roles, RoleGuess};
pub use search::{look_key, quick_score, resume, search, Board, Frontier, Line, Progress, SearchConfig, SearchResult, Stop};
pub use second::{break_board, rival_board, BreakConfig, BrokenBoard, RivalBoard};
pub use value::{card_values, CardValues, InHand, Spot};
