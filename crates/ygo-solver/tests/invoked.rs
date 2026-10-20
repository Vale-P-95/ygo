//! The search on the Invoked test hand. Needs the downloaded game data.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ygo_engine::{DeckList, Engine};
use ygo_solver::{card_values, explain, resume, search, Goal, GoalCard, GoalZone, Kit, Probe, Prober, SearchConfig, Spot, Stop};

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
    config.hand_traps = vec![ASH, FUWALOS];
    config.opponent = Some(Kit::standard(&engine).opponent().clone());
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
            b.exact
                && me.monsters.iter().flatten().any(|c| invoked_fusion(c.code))
                && me.spells.iter().flatten().any(|c| c.code == MELTDOWN)
        })
        .expect("an Invoked Fusion board");

    // The line plays again to the same board, told in Italian.
    let (state, steps) = explain(&engine, &deck, &hand, config.seed, config.opponent.as_ref(), &board.line).unwrap();
    assert_eq!(state.players[0].monsters, board.state.players[0].monsters);
    assert_eq!(steps.iter().filter(|s| s.choice.is_some()).count(), board.line.choices.len());
    let text: Vec<&String> = steps.iter().flat_map(|s| &s.events).collect();
    assert!(text.iter().any(|l| l.contains("tramite Evocazione Fusion")), "{text:#?}");
}

#[test]
fn a_goal_is_reached_or_proved_impossible() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let mechaba = engine.cards().find_by_name("Invoked Mechaba").expect("Mechaba").code;
    let goal = Goal::cards(&[mechaba], true, true);

    // From the Invoked hand the goal leads the search to Mechaba at once.
    let mut config = SearchConfig::new(deck.clone(), vec![ALEISTER, CHALICE, MELTDOWN, ASH, FUWALOS]);
    // One thread and a node budget: the same boards whatever the machine's load.
    config.threads = 1;
    config.max_nodes = 4000;
    config.time_limit = Duration::from_secs(300);
    config.policy.passive = vec![ASH, FUWALOS];
    config.hand_traps = vec![ASH, FUWALOS];
    config.opponent = Some(Kit::standard(&engine).opponent().clone());
    config.goal = goal.clone();
    let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
    assert_eq!(result.stop, Stop::GoalReached);
    assert_eq!(result.goal_reached(&goal), Some(true));
    let board = &result.boards[0];
    assert!(board.exact && goal.reached(board.goal_hits));
    let (state, _) = explain(&engine, &deck, &config.hand, config.seed, config.opponent.as_ref(), &board.line).unwrap();
    assert!(state.players[0].monsters.iter().flatten().any(|c| c.code == mechaba));

    // Without Aleister or a way to reach him every line is tried: impossible.
    let mut config = SearchConfig::new(deck, vec![ASH, FUWALOS, ASH, FUWALOS, ASH]);
    config.threads = 2;
    config.time_limit = Duration::from_secs(20);
    config.goal = goal.clone();
    let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
    assert_eq!(result.stop, Stop::Exhausted);
    assert_eq!(result.goal_reached(&goal), Some(false));
    assert!(result.frontier.is_exhausted());
}

#[test]
fn the_probe_turn_tells_the_zones_of_oceanus_apart() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let code = |name: &str| engine.cards().find_by_name(name).expect(name).code;
    let (oceanus, mechaba) = (code("Invoked Oceanus"), code("Invoked Mechaba"));
    let kit = Kit::standard(&engine);
    let hand = vec![ALEISTER, CHALICE, MELTDOWN, ASH, FUWALOS];

    // Invoked Mechaba with Invoked Oceanus in a Main Monster Zone, then in the Extra Monster Zone.
    let measure = |zone: GoalZone| {
        let mut config = SearchConfig::new(deck.clone(), hand.clone());
        // One thread and a node budget: the same boards whatever the machine's load.
        config.threads = 1;
        config.max_nodes = 4000;
        config.time_limit = Duration::from_secs(300);
        config.policy.passive = vec![ASH, FUWALOS];
        config.hand_traps = vec![ASH, FUWALOS];
        config.opponent = Some(Kit::standard(&engine).opponent().clone());
        config.goal = Goal {
            on_field: vec![GoalCard { code: oceanus, zone }, GoalCard { code: mechaba, zone: GoalZone::Any }],
            all: true,
            first_only: true,
        };
        let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
        assert_eq!(result.stop, Stop::GoalReached, "{zone:?}");
        let mut prober = Prober::new(&engine, &deck, &hand, config.seed, &kit);
        prober.measure(&result.boards[0].line).unwrap()
    };
    let main = measure(GoalZone::Main);
    let extra = measure(GoalZone::Extra);
    // Only from the Extra Monster Zone it banishes what goes to the opponent's GY.
    assert!(main.floodgates.is_empty(), "{:?}", main.floodgates);
    assert!(extra.floodgates.contains(&Probe::SendToGrave), "{:?}", extra.floodgates);
    // The opponent attacks both boards (Oceanus in a Main Monster Zone draws the attack).
    for m in [&main, &extra] {
        let attack = m.breakers.iter().find(|b| b.name.starts_with("Attacco")).expect("the attack");
        assert!(!attack.not_applicable);
    }
}

#[test]
fn cards_alone_on_the_field_are_worth_what_they_stop() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let code = |name: &str| engine.cards().find_by_name(name).expect(name).code;
    let kit = Kit::standard(&engine);
    let hand_traps = ygo_solver::hand_traps(&engine, &deck, 1, &kit);
    let values = card_values(&engine, &deck, 1, &kit, &hand_traps, 2);
    let value = |name: &str, spot: Spot| values.get(&engine, code(name), spot);
    // A negation that pays with a card of the same kind from the hand.
    assert!(value("Invoked Mechaba", Spot::Main) >= 0.9);
    // A floodgate only from the Extra Monster Zone.
    assert!(value("Invoked Oceanus", Spot::Extra) > 0.3);
    assert_eq!(value("Invoked Oceanus", Spot::Main), 0.0);
    // The searcher of the deck does nothing on the opponent's turn.
    assert_eq!(value("Aleister the Invoker", Spot::Main), 0.0);
    // A handtrap from the hand, with your field empty or not.
    assert!(value("Ash Blossom & Joyous Spring", Spot::Hand) > 0.5);
    assert!(value("Ash Blossom & Joyous Spring", Spot::HandWithField) > 0.5);
    // The Mulcharmy only while you control no cards: going first, a free discard.
    let fuwalos = code("Mulcharmy Fuwalos");
    assert!(values.usable(&engine, fuwalos, Spot::Hand));
    assert!(!values.usable(&engine, fuwalos, Spot::HandWithField));
}

#[test]
fn every_summon_is_tried_even_when_its_materials_are_chosen_alike() {
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
    // Aleister the Invoker and four cards that do nothing.
    let blank = code("Black Luster Ritual");
    deck.main.extend([blank; 4]);
    let mut config = SearchConfig::new(deck, vec![ALEISTER, blank, blank, blank, blank]);
    config.threads = 1;
    config.max_nodes = 3000;
    config.time_limit = Duration::from_secs(300);
    config.opponent = Some(Kit::standard(&engine).opponent().clone());
    let result = search(&engine, &config, &AtomicBool::new(false), |_| {});
    // Choosing Aleister as material for Salamangreat Almiraj or for Artemis
    // is the same choice of the same state: the second must not be taken
    // for the first (Artemis is the LIGHT monster Invoked Mechaba needs).
    let mechaba = code("Invoked Mechaba");
    let has =
        |b: &ygo_solver::Board| b.state.players[0].monsters.iter().flatten().any(|c| engine.cards().canonical_code(c.code) == mechaba);
    assert!(result.boards.iter().any(has), "{} boards, none with Invoked Mechaba", result.boards.len());
}

#[test]
fn a_search_stopped_and_resumed_finds_what_one_run_finds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    // A small search: one starter and handtraps.
    let hand = vec![74063034, 14558128, FUWALOS, FUWALOS, 14558128];
    let mut config = SearchConfig::new(deck, hand);
    config.threads = 1;
    config.time_limit = Duration::from_secs(60);
    let cancel = AtomicBool::new(false);
    let whole = search(&engine, &config, &cancel, |_| {});
    assert!(matches!(whole.stop, Stop::Exhausted));

    // A few nodes at a time, each run going on from where the last stopped.
    config.max_nodes = 3;
    let mut result = search(&engine, &config, &cancel, |_| {});
    for _ in 0..100 {
        if matches!(result.stop, Stop::Exhausted) {
            break;
        }
        config.max_nodes += 3;
        result = resume(&engine, &config, result.frontier, &cancel, |_| {});
    }
    assert!(matches!(result.stop, Stop::Exhausted));
    let keys = |r: &ygo_solver::SearchResult| {
        let mut k: Vec<u64> = r.boards.iter().map(|b| b.key).collect();
        k.sort_unstable();
        k
    };
    assert_eq!(keys(&result), keys(&whole));
}
