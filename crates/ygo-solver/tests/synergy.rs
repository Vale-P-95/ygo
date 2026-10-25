//! The reading deck of the synergy plan on small decks: what an effect
//! offers is what its script accepts, and a bigger Deck offers the same
//! cards plus its own. Needs the downloaded game data, like `invoked.rs`.

use std::collections::HashSet;
use std::path::PathBuf;

use ygo_engine::{DeckList, Engine};
use ygo_solver::synergy::{compare, read_card, Pool, ReadBudget};

fn engine() -> Option<Engine> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    Some(Engine::load(&data).expect("game data loads"))
}

fn code(engine: &Engine, name: &str) -> u32 {
    engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name}")).code
}

/// Fillers of no Type the cards read look for.
fn deck(engine: &Engine, main: &[&str], extra: &[&str]) -> DeckList {
    let mut deck = DeckList::default();
    deck.main.extend(main.iter().map(|n| code(engine, n)));
    deck.main.extend([code(engine, "Pot of Greed"); 20]);
    deck.extra.extend(extra.iter().map(|n| code(engine, n)));
    deck
}

#[test]
fn an_effect_offers_what_its_script_accepts() {
    let Some(engine) = engine() else { return };
    let none = HashSet::new();
    // Reinforcement of the Army: a Level 4 or lower Warrior.
    let warriors = deck(&engine, &["Marauding Captain", "Goblindbergh", "Gilford the Lightning", "Dark Magician"], &[]);
    let reading = read_card(&engine, &warriors, code(&engine, "Reinforcement of the Army"), &none, ReadBudget::default(), 1);
    assert_eq!(reading.error, None);
    let added: Vec<u32> = reading.offers.iter().filter(|o| o.hint == 506).flat_map(|o| o.cards.clone()).collect();
    let mut expected = vec![code(&engine, "Marauding Captain"), code(&engine, "Goblindbergh")];
    expected.sort_unstable();
    assert_eq!(added, expected, "{reading:?}");

    // Instant Fusion: a Level 5 or lower Fusion Monster from the Extra Deck.
    let fusions = deck(&engine, &[], &["Thousand-Eyes Restrict", "Dark Paladin"]);
    let reading = read_card(&engine, &fusions, code(&engine, "Instant Fusion"), &none, ReadBudget::default(), 1);
    let summoned: Vec<u32> = reading.offers.iter().filter(|o| o.hint == 509).flat_map(|o| o.cards.clone()).collect();
    assert_eq!(summoned, vec![code(&engine, "Thousand-Eyes Restrict")], "{reading:?}");
}

#[test]
fn a_bigger_deck_offers_the_same_cards_of_the_small_one() {
    let Some(engine) = engine() else { return };
    let small = deck(&engine, &["Marauding Captain", "Goblindbergh", "Gilford the Lightning", "Dark Magician"], &[]);
    // The small deck and 300 more cards of the pool, Warriors among them.
    let pool = Pool::new(&engine);
    let mut big = small.clone();
    big.main.extend(pool.main.iter().step_by(pool.main.len() / 300).take(300));
    let card = code(&engine, "Reinforcement of the Army");
    let cards: HashSet<u32> = small.main.iter().map(|&c| engine.cards().canonical_code(c)).collect();
    let in_small = read_card(&engine, &small, card, &cards, ReadBudget::default(), 1);
    let in_big = read_card(&engine, &big, card, &cards, ReadBudget::default(), 1);
    let agreement = compare(&in_big, &in_small, &cards);
    assert!(agreement.agrees(), "{agreement:?}");
    let offered = |r: &ygo_solver::synergy::Reading| r.offers.iter().map(|o| o.cards.len()).sum::<usize>();
    assert!(offered(&in_big) > offered(&in_small), "the big deck has more Warriors to offer");
}
