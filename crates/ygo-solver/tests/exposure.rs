//! The exposure of a line as the value expected lost to handtraps (B6 V1,
//! docs/plans/esposizione.md), on a line of the Invoked test deck. Release
//! only; skipped when the game data is missing.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use ygo_engine::{Decision, DeckList, Duel, Engine, Step};
use ygo_solver::explain::exposure_points;
use ygo_solver::probe::HandTrapRates;
use ygo_solver::{card_values, search, Kit, Prober, SearchConfig};

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn a_line_loses_where_the_handtraps_stop_it() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let code = |name: &str| engine.cards().find_by_name(name).expect(name).code;
    let hand: Vec<u32> = [
        "Aleister the Reminiscent",
        "Aleister the Invoker",
        "Invocation - \"Sword\"",
        "Ash Blossom & Joyous Spring",
        "Crowley, the Gifted Magistus",
    ]
    .iter()
    .map(|n| code(n))
    .collect();
    let kit = Kit::standard(&engine);
    let mut config = SearchConfig::new(deck.clone(), hand.clone());
    config.threads = 1;
    config.max_nodes = 1500;
    config.time_limit = Duration::from_secs(900);
    config.opponent = Some(kit.opponent().clone());
    config.hand_traps = ygo_solver::hand_traps(&engine, &deck, config.seed, &kit);
    config.values = Arc::new(card_values(&engine, &deck, config.seed, &kit, &config.hand_traps, 1));
    config.policy.passive = config.hand_traps.clone();
    let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
    let line = &result.boards.first().expect("a board").line;

    let (exposure, points) = exposure_points(&engine, &deck, &hand, config.seed, config.opponent.as_ref(), line).unwrap();
    assert_eq!(points.len() as u32, exposure.choke_points(), "{points:?}");
    // Each cut is a free moment of yours, where the turn can end.
    for p in &points {
        let mut duel = Duel::goldfish_against(&engine, &deck, &hand, config.seed, config.opponent.as_ref()).unwrap();
        for answer in &line.answers[..p.answer] {
            duel.advance().unwrap();
            duel.respond(answer);
        }
        let Step::Decision { decision, .. } = duel.advance().unwrap() else { panic!("the turn goes on") };
        assert!(matches!(decision, Decision::IdleCmd { player: 0, .. }), "{p:?}: {decision:?}");
    }

    let mut prober = Prober::new(&engine, &deck, &hand, config.seed, &kit);
    let loss = prober.expected_loss(line, &points, exposure.special_summons, &HandTrapRates::default()).unwrap();
    assert!(!loss.losses.is_empty());
    assert!(loss.losses.iter().all(|&(_, l)| l >= 0.0));
    assert!((loss.maxx - 0.1 * exposure.special_summons.saturating_sub(1) as f64).abs() < 1e-9);
    assert!(loss.expected >= HandTrapRates::default().special_summons * loss.maxx - 1e-9);
    // Without handtraps in the opponent's decks, nothing is lost.
    let none = HandTrapRates { deck_touch: 0.0, field_effect: 0.0, fifth_summon: 0.0, second_search: 0.0, special_summons: 0.0 };
    assert_eq!(prober.expected_loss(line, &points, exposure.special_summons, &none).unwrap().expected, 0.0);
}
