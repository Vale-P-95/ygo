//! Going second (docs/plans/idee-nuove.md §5): the opponent's best board
//! from a few of its opening hands is put on its field, then your hand of 6
//! plays its turn against it with the Combo Lab's search. A hand is worth
//! the share of the opponent's board value left after your turn: the board
//! is measured as the probe turn measures yours (its field and floodgates
//! against the standard kit), from the opponent's side, before and after.
//! A measure of its own, apart from the first turn's. On your turn the
//! board answers with the effects its measure found as interruptions, aimed
//! at your cards, and nothing else. Your turn comes after its turn, with a
//! Battle Phase: the Life Points it loses count too ([`LP_WEIGHT`]).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use ygo_engine::consts::location;
use ygo_engine::{DeckList, DuelState, Engine, Error, Opponent, Placement, STARTING_LP};

use crate::card::Rng;
use crate::probe::{hand_traps, Kit, Measure, Prober};
use crate::search::{search, SearchConfig};

/// Weight of the floodgates in a board's value, as in the score's
/// "Bilanciato" profile (the field counts 1).
const FLOODGATE_WEIGHT: f64 = 0.4;

/// What taking away a card of the board is worth to the search at least,
/// in interruptions: a body to attack with or to use as material.
const BODY: f64 = 0.3;

/// What the opponent's Life Points lost are worth against its board: all
/// of them count as the whole board (a lethal leaves nothing), half of them
/// as half of it.
const LP_WEIGHT: f64 = 1.0;

/// The opponent's board you play against.
#[derive(Debug, Clone)]
pub struct RivalBoard {
    /// Its opening hand of 5.
    pub hand: Vec<u32>,
    /// Its cards on the field, in the GY and banished when its turn ends.
    pub placed: Vec<Placement>,
    /// What it keeps for your turn: its hand, Deck and Extra Deck left.
    pub left: Opponent,
    /// The board's value, in interruptions ([`board_value`]).
    pub value: f64,
    /// For each card on its field, what the board loses without it (the
    /// measure without that card), at least [`BODY`].
    pub worth: Vec<(u32, f64)>,
    /// The effects it answers your turn with (code, description): the
    /// interruptions of its measure, the one worth the most first.
    pub responders: Vec<(u32, u64)>,
}

/// One hand of yours against the board.
#[derive(Debug, Clone)]
pub struct BrokenBoard {
    pub hand: Vec<u32>,
    /// Your best board for this: the one that leaves the opponent the
    /// least (`None`: the search found no line).
    pub line: Option<Vec<String>>,
    /// The opponent's cards left on its field after your turn.
    pub kept: Vec<u32>,
    /// The board's value left after your turn, and the share of the value
    /// before that is left, less the opponent's Life Points lost at
    /// [`LP_WEIGHT`] (1 when your hand does nothing, 0 for a lethal).
    pub left: f64,
    pub share: f64,
    /// The Life Points the opponent lost.
    pub damage: i32,
    /// Nodes the search used.
    pub nodes: u64,
}

/// The value of a measured board: its field, and its floodgates at
/// [`FLOODGATE_WEIGHT`] (what the opponent's hand holds is not the board).
pub fn board_value(m: &Measure) -> f64 {
    m.field_value + FLOODGATE_WEIGHT * m.floodgate_value
}

/// `player`'s cards on the field, in the GY and banished in `state`, to
/// put back in another duel. What happened to them this turn (effects used,
/// materials) is not kept: an Extra Deck monster gets materials of its kind.
pub fn placements(state: &DuelState, player: usize) -> Vec<Placement> {
    let p = &state.players[player];
    let mut out = Vec::new();
    for (zone, c) in p.monsters.iter().enumerate() {
        if let Some(c) = c {
            out.push(Placement { overlay: c.overlay.clone(), ..Placement::new(c.code, location::MZONE, zone as u32, c.position) });
        }
    }
    for (zone, c) in p.spells.iter().enumerate().take(6) {
        if let Some(c) = c {
            let equip_to = c.equipped_to.filter(|e| e.0 as usize == player && e.1 == location::MZONE).map(|e| e.2);
            out.push(Placement { equip_to, ..Placement::new(c.code, location::SZONE, zone as u32, c.position) });
        }
    }
    for (i, &code) in p.grave.iter().enumerate() {
        out.push(Placement::new(code, location::GRAVE, i as u32, ygo_engine::consts::position::FACEUP_ATTACK));
    }
    for (i, &(code, up)) in p.banished.iter().enumerate() {
        let position = if up { ygo_engine::consts::position::FACEUP_ATTACK } else { ygo_engine::consts::position::FACEDOWN_DEFENSE };
        out.push(Placement::new(code, location::REMOVED, i as u32, position));
    }
    out
}

/// Measures `player`'s board in `state` (without its `without`-th
/// placement), with its hand there: some effects cost a card from it.
fn measure(
    engine: &Engine,
    deck: &DeckList,
    state: &DuelState,
    player: usize,
    without: Option<usize>,
    seed: u64,
    kit: &Kit,
) -> Result<Measure, Error> {
    let mut placed = placements(state, player);
    if let Some(i) = without {
        placed.remove(i);
    }
    Prober::new(engine, deck, &state.players[player].hand, seed, kit).measure_placed_unbroken(&placed)
}

fn on_field(p: &Placement) -> bool {
    matches!(p.location, location::MZONE | location::SZONE)
}

/// Random hands of `size` from `main`, the same for the same seed.
pub fn random_hands(main: &[u32], size: usize, count: usize, seed: u64) -> Vec<Vec<u32>> {
    let mut rng = Rng(seed ^ 0x5ec0_17d5);
    (0..count)
        .map(|_| {
            let mut pool = main.to_vec();
            let mut hand: Vec<u32> = (0..size.min(pool.len())).map(|_| pool.swap_remove(rng.below(pool.len()))).collect();
            hand.sort_unstable();
            hand
        })
        .collect()
}

/// The opponent's best board: `deck`'s best line from each of `hands`
/// random opening hands (searched with `nodes` nodes, as the Combo Lab
/// searches against the standard kit), measured; the one worth the most.
/// `None` when no hand makes a board.
pub fn rival_board(engine: &Engine, deck: &DeckList, seed: u64, hands: usize, nodes: u64, threads: usize, kit: &Kit) -> Option<RivalBoard> {
    let traps = hand_traps(engine, deck, seed, kit);
    let mut best: Option<(f64, Vec<u32>, Arc<DuelState>, Measure)> = None;
    for hand in random_hands(&deck.main, 5, hands, seed) {
        let mut config = SearchConfig::new(deck.clone(), hand.clone());
        config.seed = seed;
        config.threads = threads;
        config.max_nodes = nodes;
        config.time_limit = Duration::from_secs(120);
        config.opponent = Some(kit.opponent().clone());
        config.hand_traps = traps.clone();
        let result = search(engine, &config, &AtomicBool::new(false), |_| {});
        let Some(board) = result.boards.first() else { continue };
        let Ok(m) = measure(engine, deck, &board.state, 0, None, seed, kit) else { continue };
        let value = board_value(&m);
        if best.as_ref().is_none_or(|b| value > b.0) {
            best = Some((value, hand, board.state.clone(), m));
        }
    }
    let (value, hand, state, m) = best?;
    let mut interruptions: Vec<_> = m.interruptions.iter().filter(|i| i.value > 0.0).collect();
    interruptions.sort_by(|a, b| b.value.total_cmp(&a.value));
    let mut responders = Vec::new();
    for i in interruptions {
        let id = (i.responder.code, i.responder.description);
        if !responders.contains(&id) {
            responders.push(id);
        }
    }
    let placed = placements(&state, 0);
    let mut worth = Vec::new();
    for (i, p) in placed.iter().enumerate().filter(|(_, p)| on_field(p)) {
        let lost = measure(engine, deck, &state, 0, Some(i), seed, kit).map_or(0.0, |m| value - board_value(&m));
        worth.push((p.code, lost.max(BODY)));
    }
    let them = &state.players[0];
    let left = Opponent {
        hand: them.hand.clone(),
        deck: them.deck.clone(),
        extra: them.extra.iter().map(|e| e.0).collect(),
        placed: placed.clone(),
        went_first: true,
    };
    Some(RivalBoard { hand, placed, left, value, worth, responders })
}

/// How to play your turn against the board.
#[derive(Debug, Clone)]
pub struct BreakConfig {
    pub seed: u64,
    pub nodes: u64,
    pub threads: usize,
    /// Your boards measured, the best by the search's estimate: the one
    /// that leaves the opponent the least counts.
    pub top: usize,
    /// The board answers your turn with its responders (else it does
    /// nothing, only its lasting effects apply).
    pub answers: bool,
}

/// Plays `hand` (yours, 6 cards) against `rival` and measures what is left
/// of its board ([`BrokenBoard`]).
pub fn break_board(
    engine: &Engine,
    deck: &DeckList,
    rival_deck: &DeckList,
    hand: &[u32],
    rival: &RivalBoard,
    kit: &Kit,
    config: &BreakConfig,
) -> Result<BrokenBoard, Error> {
    let mut search_config = SearchConfig::new(deck.clone(), hand.to_vec());
    search_config.seed = config.seed;
    search_config.threads = config.threads;
    search_config.max_nodes = config.nodes;
    search_config.time_limit = Duration::from_secs(120);
    search_config.opponent = Some(rival.left.clone());
    search_config.opponent_field = rival.worth.clone();
    // To the search, taking all its Life Points is worth taking all its cards.
    search_config.opponent_lp = LP_WEIGHT * rival.worth.iter().map(|w| w.1).sum::<f64>() / STARTING_LP as f64;
    search_config.policy.skip_battle = false;
    if config.answers {
        search_config.policy.opponent_responders = rival.responders.clone();
    }
    search_config.hand_traps = hand_traps(engine, deck, config.seed, kit);
    let result = search(engine, &search_config, &AtomicBool::new(false), |_| {});
    let all: Vec<u32> = rival.placed.iter().filter(|p| on_field(p)).map(|p| p.code).collect();
    let mut out =
        BrokenBoard { hand: hand.to_vec(), line: None, kept: all, left: rival.value, share: 1.0, damage: 0, nodes: result.progress.nodes };
    let damage = |state: &DuelState| (STARTING_LP - state.players[1].lp).clamp(0, STARTING_LP);
    let lp_share = |damage: i32| LP_WEIGHT * damage as f64 / STARTING_LP as f64;
    // The best by the estimate, and any lethal: the board left does not
    // matter then.
    let lethal = result.boards.iter().skip(config.top.max(1)).find(|b| damage(&b.state) >= STARTING_LP);
    for board in result.boards.iter().take(config.top.max(1)).chain(lethal) {
        let placed = placements(&board.state, 1);
        let left = board_value(&measure(engine, rival_deck, &board.state, 1, None, config.seed, kit)?);
        let lost = damage(&board.state);
        let share = (if rival.value > 0.0 { left / rival.value } else { 1.0 } - lp_share(lost)).max(0.0);
        if out.line.is_none() || share < out.share {
            out.line = Some(board.line.choices.clone());
            out.kept = placed.iter().filter(|p| on_field(p)).map(|p| p.code).collect();
            out.left = left;
            out.share = share;
            out.damage = lost;
        }
    }
    Ok(out)
}
