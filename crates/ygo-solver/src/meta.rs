//! The last step of the board ranking (L2-3, `docs/plans/ricerca-esperimenti.md`
//! §6b): the best boards measured again at level 2 against the most played
//! decks of the meta, each weighed by how much it is played. Against three
//! decks this orders the boards as the whole meta does far better than the
//! probe turn's measure (94% of the pairs against 80%).

use ygo_engine::{DeckList, Engine, Error};

use crate::matchup::{against_moves, line_stops, none_drawn, rival_lines, LineStop, RivalLine};
use crate::probe::{hand_traps, Kit};
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
#[derive(Clone)]
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

/// A deck of the meta in the tech table.
#[derive(Debug, Clone, PartialEq)]
pub struct TechDeck {
    pub name: String,
    pub weight: f64,
    /// Lines found, and those that play as they were found with no handtrap
    /// used (only these are counted).
    pub lines: usize,
    pub measured: usize,
}

/// One of your handtraps against a deck of the meta.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TechAgainst {
    /// Lines measured where the engine lets you use it, and those it stops.
    pub usable: usize,
    pub stopped: usize,
    /// Share of the field and hand its lines end with that it takes, at its
    /// best moment of each, averaged over the lines measured.
    pub lost: f64,
}

/// One of your handtraps against the meta's lines.
#[derive(Debug, Clone, PartialEq)]
pub struct TechRow {
    pub code: u32,
    /// Copies in your Main and Side Deck.
    pub main: u32,
    pub side: u32,
    /// Chance of at least one among the 5 cards you hold in the meta deck's
    /// turn when you go second, with your Main Deck as it is.
    pub chance: f64,
    /// Σ weight · share of a deck's lines it stops (changes the field or
    /// hand they end with), weighed among the decks with lines measured.
    pub value: f64,
    /// The same weighed share of the field and hand the lines end with that
    /// it takes ([`TechAgainst::lost`]): what the table is ordered by.
    pub lost: f64,
    /// Against each deck of [`TechTable::decks`], in its order.
    pub decks: Vec<TechAgainst>,
    /// Moves of a line played before it stops it (its best moment),
    /// averaged over the lines it stops.
    pub step: Option<f64>,
    /// The cards of the lines it answers at that moment, with how many
    /// lines, the most often first.
    pub at: Vec<(u32, u32)>,
}

/// Your handtraps against the best lines of the meta's decks, the one that
/// takes the most first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TechTable {
    pub decks: Vec<TechDeck>,
    pub rows: Vec<TechRow>,
}

/// Plays each line of `rivals` again going first, with you holding all the
/// handtraps of your Main and Side Deck (those the engine lets you use from
/// the hand in the opponent's turn, [`hand_traps`]), and counts the lines
/// each one stops. Read from the engine, with no card named.
pub fn tech_table(engine: &Engine, deck: &DeckList, rivals: &[MetaRival], seed: u64, kit: &Kit, threads: usize) -> TechTable {
    let db = engine.cards();
    let all = DeckList { main: deck.main.iter().chain(&deck.side).copied().collect(), extra: deck.extra.clone(), side: Vec::new() };
    let candidates = hand_traps(engine, &all, seed, kit);
    let jobs: Vec<(usize, &RivalLine)> = rivals.iter().enumerate().flat_map(|(d, r)| r.lines.iter().map(move |l| (d, l))).collect();
    let at = std::sync::atomic::AtomicUsize::new(0);
    let found: std::sync::Mutex<Vec<(usize, Vec<LineStop>)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, jobs.len().max(1)) {
            s.spawn(|| loop {
                let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(&(d, line)) = jobs.get(i) else { break };
                if let Ok(Some(stops)) = line_stops(engine, &rivals[d].deck.list, line, seed, deck, &candidates, kit) {
                    if let Ok(mut all) = found.lock() {
                        all.push((d, stops));
                    }
                }
            });
        }
    });
    // The threads finish in any order: what follows only adds up.
    let found = found.into_inner().unwrap_or_default();
    let decks: Vec<TechDeck> = rivals
        .iter()
        .enumerate()
        .map(|(d, r)| TechDeck {
            name: r.deck.name.clone(),
            weight: r.deck.weight,
            lines: r.lines.len(),
            measured: found.iter().filter(|(k, _)| *k == d).count(),
        })
        .collect();
    let copies = |cards: &[u32], code: u32| cards.iter().filter(|&&c| db.canonical_code(c) == code).count() as u32;
    let rows = candidates
        .iter()
        .map(|&code| {
            let (main, side) = (copies(&deck.main, code), copies(&deck.side, code));
            let chance = 1.0 - none_drawn(deck.main.len().max(1), main, 5);
            tech_row(code, main, side, chance, &decks, &found)
        })
        .collect();
    let mut table = TechTable { decks, rows };
    table
        .rows
        .sort_by(|a, b| b.lost.total_cmp(&a.lost).then(b.value.total_cmp(&a.value)).then(b.main.cmp(&a.main)).then(a.code.cmp(&b.code)));
    table
}

/// The row of `code` from the lines measured (`found`: the deck each
/// belongs to and what each handtrap does against it).
fn tech_row(code: u32, main: u32, side: u32, chance: f64, decks: &[TechDeck], found: &[(usize, Vec<LineStop>)]) -> TechRow {
    let mut against = vec![TechAgainst::default(); decks.len()];
    let mut steps = Vec::new();
    let mut at: Vec<(u32, u32)> = Vec::new();
    for (d, stops) in found {
        let Some(s) = stops.iter().find(|s| s.code == code) else { continue };
        against[*d].usable += 1;
        if s.stops == 0 {
            continue;
        }
        against[*d].stopped += 1;
        against[*d].lost += s.lost;
        steps.extend(s.step);
        if let Some(card) = s.playing {
            match at.iter_mut().find(|(c, _)| *c == card) {
                Some(entry) => entry.1 += 1,
                None => at.push((card, 1)),
            }
        }
    }
    at.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (a, d) in against.iter_mut().zip(decks) {
        a.lost /= d.measured.max(1) as f64;
    }
    let weighed = |of: &dyn Fn(&TechDeck, &TechAgainst) -> f64| {
        let counted: Vec<(f64, f64)> =
            decks.iter().zip(&against).filter(|(d, _)| d.measured > 0).map(|(d, a)| (d.weight, of(d, a))).collect();
        let total: f64 = counted.iter().map(|(w, _)| w).sum();
        if total > 0.0 {
            counted.iter().map(|(w, v)| w * v).sum::<f64>() / total
        } else {
            0.0
        }
    };
    let value = weighed(&|d, a| a.stopped as f64 / d.measured as f64);
    let lost = weighed(&|_, a| a.lost);
    let step = (!steps.is_empty()).then(|| steps.iter().sum::<usize>() as f64 / steps.len() as f64);
    TechRow { code, main, side, chance, value, lost, decks: against, step, at }
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

    fn stop(code: u32, stops: u32, step: usize, playing: u32) -> LineStop {
        let lost = if stops > 0 { 0.5 } else { 0.0 };
        LineStop { code, windows: 3, stops, lost, step: (stops > 0).then_some(step), playing: (stops > 0).then_some(playing) }
    }

    #[test]
    fn a_row_weighs_each_deck_by_how_much_it_is_played() {
        let decks = vec![
            TechDeck { name: "a".into(), weight: 3.0, lines: 2, measured: 2 },
            TechDeck { name: "b".into(), weight: 1.0, lines: 2, measured: 1 },
            TechDeck { name: "c".into(), weight: 9.0, lines: 1, measured: 0 },
        ];
        let found = vec![(0, vec![stop(7, 2, 1, 100), stop(8, 0, 0, 0)]), (0, vec![stop(7, 0, 0, 0)]), (1, vec![stop(7, 1, 3, 200)])];
        let row = tech_row(7, 3, 0, 0.5, &decks, &found);
        // a: 1 of 2 lines, b: 1 of 1; the unmeasured deck c does not count.
        assert!((row.value - (3.0 * 0.5 + 1.0) / 4.0).abs() < 1e-9);
        assert!((row.lost - (3.0 * 0.25 + 0.5) / 4.0).abs() < 1e-9);
        assert_eq!(row.decks[0], TechAgainst { usable: 2, stopped: 1, lost: 0.25 });
        assert_eq!(row.step, Some(2.0));
        assert_eq!(row.at, vec![(100, 1), (200, 1)]);
        let never = tech_row(9, 1, 0, 0.1, &decks, &found);
        assert_eq!((never.value, never.step), (0.0, None));
    }
}
