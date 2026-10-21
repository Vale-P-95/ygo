//! The map of the links between the cards of a deck, read from the engine.
//! Skipped without the game data, like `invoked.rs`.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use ygo_engine::{DeckList, Engine};
use ygo_solver::{deck_links, LinkKind, LinkZone, LinksBudget};

#[test]
fn the_invoked_deck_links_aleister_invocation_and_the_invoked() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let code = |name: &str| engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name}")).code;
    let budget = LinksBudget { hands_per_card: 1, nodes_per_hand: 500, lines_per_hand: 10, threads: 2 };
    let map = deck_links(&engine, &deck, 1, budget, &AtomicBool::new(false), |_, _| {});
    let link = |from: &str, to: &str, kind: LinkKind| map.links.iter().find(|l| l.from == code(from) && l.to == code(to) && l.kind == kind);

    // Aleister searches Invocation from the Deck and is a material Invoked
    // Mechaba names; Invocation Fusion Summons the Invoked from the Extra Deck.
    let search = link("Aleister the Invoker", "Invocation", LinkKind::Search).expect("Aleister adds Invocation");
    assert!(search.possible && search.used > 0.0 && search.zone == LinkZone::Deck, "{search:?}");
    assert!(link("Aleister the Invoker", "Invoked Mechaba", LinkKind::Material).is_some(), "{:?}", map.links);
    let invoked = |c: u32| engine.cards().name(c).starts_with("Invoked ");
    let fusions = map.links.iter().filter(|l| l.from == code("Invocation") && l.kind == LinkKind::Summon && invoked(l.to)).count();
    assert!(fusions >= 3, "{:?}", map.links);
    let fusion = map.links.iter().find(|l| l.from == code("Invocation") && l.kind == LinkKind::Summon && invoked(l.to));
    assert_eq!(fusion.map(|l| l.zone), Some(LinkZone::Extra));

    // A handtrap leads nowhere: reached only by holding it (3 in 40).
    let ash = map.cards.iter().find(|c| c.code == code("Ash Blossom & Joyous Spring")).expect("Ash");
    assert_eq!((ash.reach, ash.reach_all), (0, 0));
    assert!(ash.chance > 0.3 && ash.chance < 0.4, "{ash:?}");
    assert!(map.cards.iter().all(|c| c.reach_all >= c.reach), "{:?}", map.cards);
}
