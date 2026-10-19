//! The search on the Invoked test hand. Needs the downloaded game data.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ygo_engine::{DeckList, Engine};
use ygo_solver::{explain, search, SearchConfig, Stop};

const ALEISTER: u32 = 86120751;
const CHALICE: u32 = 76334960;
const MELTDOWN: u32 = 47679935;
const ASH: u32 = 14558127;
const FUWALOS: u32 = 42141493;

#[test]
fn finds_invoked_boards_by_itself() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let hand = vec![ALEISTER, CHALICE, MELTDOWN, ASH, FUWALOS];
    let mut config = SearchConfig::new(deck.clone(), hand.clone());
    config.threads = 2;
    config.time_limit = Duration::from_secs(25);
    // Handtraps stay in the hand on your own turn.
    config.policy.passive = vec![ASH, FUWALOS];
    let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
    assert!(matches!(result.stop, Stop::TimeLimit | Stop::Exhausted));
    assert_eq!(result.interrupted, 0);
    assert!(!result.boards.is_empty());

    // An Invoked Fusion Monster on the field with Magical Meltdown, as in the known line.
    let invoked_fusion = |code: u32| engine.cards().get(code).is_some_and(|c| c.name.starts_with("Invoked ") && c.type_ & 0x40 != 0);
    let board = result
        .boards
        .iter()
        .find(|b| {
            let me = &b.state.players[0];
            me.monsters.iter().flatten().any(|c| invoked_fusion(c.code)) && me.spells.iter().flatten().any(|c| c.code == MELTDOWN)
        })
        .expect("an Invoked Fusion board");

    // The line plays again to the same board, told in Italian.
    let (state, steps) = explain(&engine, &deck, &hand, config.seed, &board.line).unwrap();
    assert_eq!(state.players[0].monsters, board.state.players[0].monsters);
    assert_eq!(steps.iter().filter(|s| s.choice.is_some()).count(), board.line.choices.len());
    let text: Vec<&String> = steps.iter().flat_map(|s| &s.events).collect();
    assert!(text.iter().any(|l| l.contains("tramite Evocazione Fusion")), "{text:#?}");
}
