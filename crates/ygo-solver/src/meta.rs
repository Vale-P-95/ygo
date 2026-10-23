//! The last step of the board ranking (L2-3, `docs/plans/ricerca-esperimenti.md`
//! §6b): the best boards measured again at level 2 against the most played
//! decks of the meta, each weighed by how much it is played. Against three
//! decks this orders the boards as the whole meta does far better than the
//! probe turn's measure (94% of the pairs against 80%).

use ygo_engine::{DeckList, Engine, Error};

use crate::matchup::{against_moves, rival_lines, RivalLine};
use crate::probe::Kit;
use crate::search::Line;

/// A deck of the meta and how much it is played (any scale: the decks
/// used are weighed among themselves).
#[derive(Debug, Clone)]
pub struct MetaDeck {
    pub name: String,
    pub weight: f64,
    pub list: DeckList,
}

/// The `count` decks played the most, by their weight alone.
pub fn most_played(decks: &[MetaDeck], count: usize) -> Vec<MetaDeck> {
    let mut decks: Vec<MetaDeck> = decks.iter().filter(|d| d.weight > 0.0).cloned().collect();
    decks.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    decks.truncate(count);
    decks
}

/// A deck of the meta with its best lines, read once for all the boards.
pub struct MetaRival {
    pub deck: MetaDeck,
    pub lines: Vec<RivalLine>,
}

/// Opening hands each deck's lines are read from, and the nodes of each
/// search: as the Combo Lab reads a deck's lines (`combo_moves`).
const HANDS: usize = 12;
const NODES: u64 = 1000;

/// Reads the best lines of each deck, with this `seed` (computed now, never
/// kept from another session: a change to the engine changes them).
pub fn meta_rivals(engine: &Engine, decks: &[MetaDeck], seed: u64, threads: usize) -> Vec<MetaRival> {
    decks.iter().map(|d| MetaRival { deck: d.clone(), lines: rival_lines(engine, &d.list, seed, HANDS, NODES, threads) }).collect()
}

/// A board against one deck of the meta.
#[derive(Debug, Clone, PartialEq)]
pub struct MetaDeckScore {
    pub name: String,
    pub weight: f64,
    /// Lines of the deck played against the board.
    pub lines: usize,
    /// Share of the deck's moves made on an empty board (a run that strays
    /// from its line makes fewer; under [`MADE`] the deck is not counted).
    pub made: f64,
    /// Share of the moves the board lets the deck make that it still makes
    /// with all your answers used; none when not measured.
    pub left: Option<f64>,
}

/// A board's level 2 against the meta's decks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetaScore {
    /// Σ weight · (1 − left) over the decks measured, weighed among them;
    /// none when no deck could be measured.
    pub value: Option<f64>,
    /// The same weighed `left`.
    pub left: Option<f64>,
    pub decks: Vec<MetaDeckScore>,
}

/// Below this share of its moves made on an empty board a deck's lines
/// did not play as they were found, and the deck is left out (as in the
/// objective bench).
pub const MADE: f64 = 0.8;

/// The board reached by `line` against each deck of `rivals`.
#[allow(clippy::too_many_arguments)]
pub fn meta_score(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    line: &Line,
    rivals: &[MetaRival],
    kit: &Kit,
) -> Result<MetaScore, Error> {
    let mut decks = Vec::new();
    for rival in rivals {
        let x = against_moves(engine, deck, hand, seed, line, &rival.deck.list, &rival.lines, kit)?;
        let made = x.moves.iter().filter(|m| m.possible).map(|m| m.weight).sum::<f64>();
        decks.push(MetaDeckScore {
            name: rival.deck.name.clone(),
            weight: rival.deck.weight,
            lines: x.lines,
            made,
            left: (x.lines > 0 && made >= MADE).then_some(x.left),
        });
    }
    let counted: Vec<(f64, f64)> = decks.iter().filter_map(|d| d.left.map(|l| (d.weight, l))).collect();
    let total: f64 = counted.iter().map(|(w, _)| w).sum();
    let left = (total > 0.0).then(|| counted.iter().map(|(w, l)| w * l).sum::<f64>() / total);
    Ok(MetaScore { value: left.map(|l| 1.0 - l), left, decks })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(name: &str, weight: f64) -> MetaDeck {
        MetaDeck { name: name.to_owned(), weight, list: DeckList::default() }
    }

    #[test]
    fn the_most_played_go_by_weight() {
        let decks = [deck("a", 3.0), deck("b", 9.0), deck("c", 0.0), deck("d", 5.0), deck("e", 1.0)];
        let names: Vec<String> = most_played(&decks, 3).into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["b", "d", "a"]);
    }
}
