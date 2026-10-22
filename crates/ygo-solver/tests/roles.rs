//! The roles of the cards of the Invoked test deck. Needs the downloaded
//! game data, like `invoked.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use ygo_engine::{DeckList, Engine};
use ygo_solver::{card_values, guess_roles, hand_traps, Kit};

#[test]
fn the_invoked_deck_has_its_extenders_and_breakers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    let code = |name: &str| engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name}")).code;
    let kit = Kit::standard(&engine);
    let traps = hand_traps(&engine, &deck, 1, &kit);
    let values = Arc::new(card_values(&engine, &deck, 1, &kit, &traps, 4));
    let passive: Vec<u32> = ["Ash Blossom & Joyous Spring", "Mulcharmy Fuwalos"].iter().map(|n| code(n)).collect();
    let roles = guess_roles(&engine, &deck, &kit, &traps, &passive, values, 4, |_, _| {});
    let of = |name: &str| roles.iter().find(|r| r.code == code(name)).map(|r| r.roles.clone()).unwrap_or_default();

    // Super Polymerization takes the opponent's monsters as material.
    assert!(of("Super Polymerization").contains(&"breaker"), "{roles:?}");
    // Invocation - "Chalice" turns a monster the line is done with and one
    // from the Deck into an Invoked Fusion Monster: it extends, and it can
    // banish a monster of the opponent's.
    let chalice = of("Invocation - \"Chalice\"");
    assert!(chalice.contains(&"extender") && chalice.contains(&"breaker"), "{roles:?}");
    assert!(of("Magical Meltdown").contains(&"starter"), "{roles:?}");
}
