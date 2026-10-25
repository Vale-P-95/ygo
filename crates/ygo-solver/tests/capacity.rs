//! What a board can stop on the opponent's turn, read from its cards'
//! effects with no opponent playing (docs/plans/linee-interazione.md §3).
//! Skipped when the game data is missing.

use std::path::PathBuf;

use ygo_engine::consts::{location, position};
use ygo_engine::{fingerprint, Blocked, DeckList, Duel, Engine, Placement, Step};
use ygo_solver::{board_capacity, capacity, InterruptionClass, Line, SearchConfig};

fn engine() -> Option<(Engine, PathBuf)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    Some((Engine::load(&data).expect("game data loads"), root))
}

/// The Invoked deck with `main` and `extra` added, the hand `hand` and `placed` on the field.
fn config(engine: &Engine, root: &std::path::Path, main: &[&str], extra: &[&str], hand: &[&str], placed: &[(&str, u8)]) -> SearchConfig {
    let code = |name: &str| engine.cards().find_by_name(name).expect(name).code;
    let mut deck =
        DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    deck.main.extend(main.iter().map(|n| code(n)));
    deck.extra.extend(extra.iter().map(|n| code(n)));
    let mut config = SearchConfig::new(deck, hand.iter().map(|n| code(n)).collect());
    config.placed = placed.iter().enumerate().map(|(i, &(n, at))| Placement::new(code(n), at, i as u32, position::FACEUP_ATTACK)).collect();
    config
}

#[test]
fn copies_share_their_limit_and_costs_share_the_hand() {
    let Some((engine, root)) = engine() else { return };
    // Two Ash Blossom in the hand, once per turn by name; Invoked Mechaba
    // sends a card from the hand to negate: one Ash pays for it.
    let ash = "Ash Blossom & Joyous Spring";
    let config = config(&engine, &root, &[ash, ash], &[], &[ash, ash], &[("Invoked Mechaba", location::MZONE)]);
    let (effects, c) = board_capacity(&engine, &config, &Line::default()).expect("dry check");
    assert!(!effects.is_empty());
    let ash_uses: u32 = c.interruptions.iter().zip(&c.plan).filter(|(i, _)| i.location == location::HAND).map(|(_, &n)| n).sum();
    assert_eq!(ash_uses, 1, "{c:?}");
    assert_eq!(c.count, 2, "{c:?}");
    assert_eq!(c.by_class[InterruptionClass::ALL.iter().position(|&k| k == InterruptionClass::NegateActivation).unwrap()], 1);
    // With no card in the hand, Mechaba cannot pay.
    let alone = config_with_empty_hand(&engine, &root);
    let (_, c) = board_capacity(&engine, &alone, &Line::default()).expect("dry check");
    let mechaba = c.interruptions.iter().find(|i| i.location == location::MZONE).expect("Mechaba's negation");
    assert_eq!(mechaba.blocked, Some(Blocked::Cost), "{c:?}");
    assert_eq!(c.count, 0);
}

fn config_with_empty_hand(engine: &Engine, root: &std::path::Path) -> SearchConfig {
    config(engine, root, &[], &[], &[], &[("Invoked Mechaba", location::MZONE)])
}

#[test]
fn an_xyz_monster_without_materials_cannot_detach() {
    let Some((engine, root)) = engine() else { return };
    let config = config(&engine, &root, &[], &["Tornado Dragon"], &[], &[("Tornado Dragon", location::MZONE)]);
    let (_, c) = board_capacity(&engine, &config, &Line::default()).expect("dry check");
    let tornado = c.interruptions.iter().find(|i| i.class == InterruptionClass::Destroy).expect("Tornado Dragon's destruction");
    assert_eq!(tornado.blocked, Some(Blocked::Cost), "{c:?}");
}

#[test]
fn the_dry_check_changes_nothing() {
    let Some((engine, root)) = engine() else { return };
    let ash = "Ash Blossom & Joyous Spring";
    let config = config(&engine, &root, &[ash], &[], &[ash], &[("Invoked Mechaba", location::MZONE)]);
    let mut duel =
        Duel::goldfish_against_from(&engine, &config.deck, &config.hand, config.seed, None, &config.placed, &config.prelude).expect("duel");
    assert!(matches!(duel.advance().expect("first decision"), Step::Decision { .. }));
    let before = fingerprint(&duel.state());
    let effects = ygo_engine::dry_check(&mut duel, 0).expect("dry check");
    assert_eq!(fingerprint(&duel.state()), before);
    // Checked twice, the same answer: nothing the first check wrapped is left.
    assert_eq!(ygo_engine::dry_check(&mut duel, 0).expect("dry check"), effects);
    // One card in the hand: Ash discards itself or pays for Mechaba, not both.
    assert_eq!(capacity(&effects).count, 1);
}
