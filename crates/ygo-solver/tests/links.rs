//! The map of the links between the cards of a deck, read from the engine.
//! Skipped without the game data, like `invoked.rs`.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use ygo_engine::{DeckList, Engine};
use ygo_solver::{compare_links, deck_links_again, LinkKind, LinkZone, LinksBudget};

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
    let budget = LinksBudget { hands_per_card: 1, nodes_per_hand: 500, lines_per_hand: 10, threads: 2, ..LinksBudget::default() };
    // The text alone gives a draft at once, before any hand is searched.
    let draft = std::sync::Mutex::new(None);
    let (map, work) = deck_links_again(
        &engine,
        &deck,
        1,
        budget,
        &AtomicBool::new(false),
        |done, _, map| {
            if done == 0 {
                *draft.lock().unwrap() = Some(map.clone());
            }
        },
        None,
    );
    let draft = draft.into_inner().unwrap().expect("the draft comes first");
    let named = |from: &str, to: &str, kind: LinkKind| {
        draft.links.iter().any(|l| l.from == code(from) && l.to == code(to) && l.kind == kind && l.text && !l.possible)
    };
    assert!(named("Aleister the Invoker", "Invocation", LinkKind::Search), "{:?}", draft.links);
    assert!(named("Aleister the Invoker", "Invoked Mechaba", LinkKind::Material), "{:?}", draft.links);
    let link = |from: &str, to: &str, kind: LinkKind| map.links.iter().find(|l| l.from == code(from) && l.to == code(to) && l.kind == kind);

    // Aleister searches Invocation from the Deck and is a material Invoked
    // Mechaba names; Invocation Fusion Summons the Invoked from the Extra Deck.
    let search = link("Aleister the Invoker", "Invocation", LinkKind::Search).expect("Aleister adds Invocation");
    assert!(search.possible && search.text && search.used > 0.0 && search.zone == LinkZone::Deck, "{search:?}");
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
    // Negating Aleister loses cards reached; negating a handtrap loses none.
    let aleister = map.cards.iter().find(|c| c.code == code("Aleister the Invoker")).expect("Aleister");
    assert!(aleister.key > 0.0 && ash.key == 0.0, "{aleister:?} {ash:?}");

    // Read again with nothing changed, the work done is reused: no hand to
    // search, the same map.
    let searched = std::sync::atomic::AtomicUsize::new(usize::MAX);
    let (again, _) = deck_links_again(
        &engine,
        &deck,
        1,
        budget,
        &AtomicBool::new(false),
        |_, total, _| searched.store(total, std::sync::atomic::Ordering::Relaxed),
        Some(&work),
    );
    assert_eq!(searched.into_inner(), 0);
    assert_eq!(again.links.len(), map.links.len());
    let same = compare_links(&map, &again);
    assert!(same.gained.is_empty() && same.lost.is_empty() && same.changed.is_empty() && same.removed.is_empty(), "{same:?}");

    // A version without Magical Meltdown: its links are lost, and Aleister,
    // which it searched, is reached less often.
    let meltdown = code("Magical Meltdown");
    let mut lighter = deck.clone();
    lighter.main.retain(|&c| c != meltdown);
    let (after, _) = deck_links_again(&engine, &lighter, 1, budget, &AtomicBool::new(false), |_, _, _| {}, Some(&work));
    let diff = compare_links(&map, &after);
    assert_eq!((diff.removed.clone(), diff.added.len()), (vec![meltdown], 0));
    let aleister_code = code("Aleister the Invoker");
    assert!(diff.lost.iter().any(|l| l.from == meltdown && l.to == aleister_code && l.kind == LinkKind::Search), "{:?}", diff.lost);
    assert!(diff.gained.iter().all(|l| l.from != meltdown && l.to != meltdown), "{:?}", diff.gained);
    let change = diff.changed.iter().find(|c| c.code == aleister_code).expect("Aleister changes");
    assert!(change.chance.1 < change.chance.0, "{change:?}");
    assert!(diff.reached.1 < diff.reached.0, "{:?}", diff.reached);
}
