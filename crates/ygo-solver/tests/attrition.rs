//! What your turn takes from the opponent (B5a): the search sees it.
//! Release only; skipped when the game data is missing.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use ygo_engine::consts::{location, position};
use ygo_engine::{DeckList, Engine, Placement};
use ygo_solver::{card_values, search, Kit, SearchConfig};

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn aeon_banishing_from_the_opponents_extra_deck_comes_first() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let mut deck =
        DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let code = |name: &str| engine.cards().find_by_name(name).expect(name).code;
    // A hand that does nothing: the board is made from the GY, where Invoked
    // Sorath fuses three Fusion Monsters with different Attributes into
    // Hyperinvoked Aeon (with 3 materials it may banish from the opponent's
    // Extra Deck too).
    let blank = code("Black Luster Ritual");
    deck.main.extend([blank; 5]);
    let kit = Kit::standard(&engine);
    let mut config = SearchConfig::new(deck.clone(), vec![blank; 5]);
    config.threads = 1;
    config.max_nodes = 600;
    config.time_limit = Duration::from_secs(600);
    config.opponent = Some(kit.opponent().clone());
    config.hand_traps = ygo_solver::hand_traps(&engine, &deck, config.seed, &kit);
    config.values = Arc::new(card_values(&engine, &deck, config.seed, &kit, &config.hand_traps, 1));
    config.placed = ["Invoked Sorath", "Invoked Caliga", "Invoked Raidjin", "Alexandrite Dragon"]
        .iter()
        .enumerate()
        .map(|(i, n)| Placement::new(code(n), location::GRAVE, i as u32, position::FACEUP_ATTACK))
        .collect();
    let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
    let best = result.boards.first().expect("a board");
    let banished = best.state.players[1].banished.len();
    assert_eq!(banished, 3, "the best board banished {banished} cards of the opponent's Extra Deck: {:?}", best.line.choices);
    // The same Aeon without banishing them is estimated lower.
    let kept = result.boards.iter().find(|b| b.state.players[1].banished.is_empty()).expect("a board that does not banish");
    assert!(best.quick_score > kept.quick_score + 0.1, "{} against {}", best.quick_score, kept.quick_score);
}
