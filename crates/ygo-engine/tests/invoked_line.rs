//! End-to-end checks of the engine with the Invoked test deck. They need the
//! downloaded game data (`duelforge-cli fetch-data`, or `DUELFORGE_DATA`);
//! without it they are skipped.

use std::path::PathBuf;

use ygo_engine::consts::location;
use ygo_engine::{run_script, DeckList, Duel, Engine, ScriptEnd, Trace};

const ALEISTER: u32 = 86120751;
const CHALICE: u32 = 76334960;
const MELTDOWN: u32 = 47679935;
const ASH_ALT_ART: u32 = 14558128;
const FUWALOS: u32 = 42141493;

fn setup() -> Option<(Engine, DeckList)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let ydk = std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap();
    Some((engine, DeckList::from_ydk(&ydk).unwrap()))
}

fn names(duel: &Duel, loc: u8) -> Vec<String> {
    let mut names: Vec<_> = duel.query_location(0, loc).into_iter().flatten().map(|(code, _)| duel.cards().name(code)).collect();
    names.sort();
    names
}

#[test]
fn starting_hand_accepts_alternate_artworks() {
    let Some((engine, deck)) = setup() else { return };
    // The deck lists Ash Blossom's alternate artwork; asking for the
    // original passcode must find it.
    let duel = Duel::goldfish(&engine, &deck, &[ALEISTER, CHALICE, MELTDOWN, 14558127, FUWALOS], 1).unwrap();
    assert_eq!(duel.count(0, location::HAND), 5);
    assert_eq!(duel.count(0, location::DECK), 35);
    assert_eq!(duel.count(0, location::EXTRA), 15);
}

#[test]
fn meltdown_aleister_chalice_into_baybarron() {
    let Some((engine, deck)) = setup() else { return };
    let mut duel = Duel::goldfish(&engine, &deck, &[ALEISTER, CHALICE, MELTDOWN, ASH_ALT_ART, FUWALOS], 1).unwrap();
    let mut chosen = Vec::new();
    let end = run_script(
        &mut duel,
        &[
            "activate magical meltdown",
            "yes",
            "select aleister the invoker (deck)",
            "normal summon aleister the invoker",
            "yes",
            "select invocation (deck)",
            "activate invocation - \"chalice\"",
            "option: fusion summon",
            "select invoked baybarron",
            "select aleister the invoker (monster zone 1)",
            "select virakam",
            "end turn",
        ],
        |trace| {
            if let Trace::Chose(action) = trace {
                chosen.push(action.label.clone());
            }
        },
    )
    .unwrap();
    assert!(matches!(end, ScriptEnd::TurnEnded), "line did not complete: {end:?}\nchosen: {chosen:#?}");

    assert_eq!(names(&duel, location::MZONE), ["Invoked Baybarron"]);
    assert_eq!(names(&duel, location::SZONE), ["Magical Meltdown"]);
    assert_eq!(names(&duel, location::GRAVE), ["Invocation - \"Chalice\""]);
    assert_eq!(names(&duel, location::REMOVED), ["Aleister the Invoker", "Virakam, the Divine Spirit of Artifice"]);
    assert_eq!(names(&duel, location::HAND), ["Aleister the Invoker", "Ash Blossom & Joyous Spring", "Invocation", "Mulcharmy Fuwalos"]);
    // Ruling check: Baybarron's search needs Virakam or Gamma Summon in the
    // Deck. The only Virakam was banished as material and Gamma Summon is in
    // the Side Deck, so the trigger was never offered.
    assert!(!chosen.iter().any(|label| label.contains("Invoked Baybarron (Monster Zone")), "{chosen:#?}");
    assert_eq!(duel.count(0, location::DECK), 32);
}
