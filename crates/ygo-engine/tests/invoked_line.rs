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

#[test]
fn state_and_fingerprint_follow_the_line() {
    let Some((engine, deck)) = setup() else { return };
    let hand = [ALEISTER, CHALICE, MELTDOWN, ASH_ALT_ART, FUWALOS];
    let mut a = Duel::goldfish(&engine, &deck, &hand, 1).unwrap();
    let mut b = Duel::goldfish(&engine, &deck, &hand, 1).unwrap();
    a.advance().unwrap();
    b.advance().unwrap();
    let start = a.state();
    assert_eq!((start.turn, start.turn_player), (1, 0));
    assert_eq!(start.players[0].lp, 8000);
    assert_eq!(start.players[0].hand.len(), 5);
    assert_eq!(start.players[0].deck.len(), 35);
    assert_eq!(start.players[0].monsters.len(), 7);
    assert_eq!(start.players[0].spells.len(), 8);
    assert_eq!(a.fingerprint(), b.fingerprint(), "same state, same fingerprint");

    run_script(
        &mut a,
        &[
            "activate magical meltdown",
            "yes",
            "select aleister the invoker (deck)",
            "normal summon aleister the invoker",
            "yes",
            "select invocation (deck)",
        ],
        |_| {},
    )
    .unwrap();
    let after = a.state();
    assert_ne!(a.fingerprint(), b.fingerprint());
    let aleister = after.players[0].monsters.iter().flatten().find(|c| c.code == ALEISTER).expect("Aleister on the field");
    assert!(aleister.is_face_up() && aleister.attack > 0);
    assert!(after.players[0].spells.iter().flatten().any(|c| c.code == MELTDOWN));
    assert!(after.activity.normal_summon_used);
    assert!(after.activity.activations.iter().any(|&(code, _)| code == MELTDOWN));
    assert!(after.activity.activations.iter().any(|&(code, _)| code == ALEISTER));
}

#[test]
fn the_line_told_in_italian() {
    let Some((engine, deck)) = setup() else { return };
    let mut duel = Duel::goldfish(&engine, &deck, &[ALEISTER, CHALICE, MELTDOWN, ASH_ALT_ART, FUWALOS], 1).unwrap();
    let mut lines = Vec::new();
    run_script(
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
        ],
        |trace| {
            if let Trace::Events(events) = trace {
                lines.extend(events.iter().filter_map(|e| ygo_engine::italian::describe(engine.cards(), e)));
            }
        },
    )
    .unwrap();
    for expected in [
        "Catena 1: Magical Meltdown",
        "Aleister the Invoker viene aggiunta alla mano dal Deck",
        "Evochi normalmente Aleister the Invoker",
        "Invocation viene aggiunta alla mano dal Deck",
        "Invoked Baybarron viene evocato tramite Evocazione Fusion",
    ] {
        assert!(lines.iter().any(|l| l.starts_with(expected)), "missing {expected:?} in {lines:#?}");
    }
}

#[test]
fn trivial_answers_are_taken_automatically() {
    use ygo_engine::auto::AutoPolicy;
    use ygo_engine::Step;
    let Some((engine, deck)) = setup() else { return };
    let mut duel = Duel::goldfish(&engine, &deck, &[ALEISTER, CHALICE, MELTDOWN, ASH_ALT_ART, FUWALOS], 1).unwrap();
    let policy = AutoPolicy::default();
    let mut answers = Vec::new();
    let mut real = Vec::new();
    for step in [
        "activate magical meltdown",
        "yes",
        "normal summon aleister the invoker",
        "yes",
        "activate invocation - \"chalice\"",
        // The only mode Chalice can use here: chosen automatically.
        "select invoked baybarron",
        "select aleister the invoker (monster zone",
        "select virakam",
    ] {
        loop {
            let (Step::Decision { .. }, actions) = duel.advance_auto(&policy, &mut answers).unwrap() else { panic!("duel ended") };
            assert!(!actions.iter().any(|a| a.kind == ygo_engine::ActionKind::ToBattlePhase));
            // Mulcharmy Fuwalos may be chained at many points: the line does
            // not; zones are a real choice, the line takes the first.
            let action = match actions.iter().find(|a| a.label.to_lowercase().contains(step)) {
                Some(a) => a,
                None => actions
                    .iter()
                    .find(|a| a.label == "Don't chain" || a.kind == ygo_engine::ActionKind::Place)
                    .unwrap_or_else(|| panic!("no {step:?} in {:#?}", actions.iter().map(|a| &a.label).collect::<Vec<_>>())),
            };
            real.push(action.label.clone());
            answers.push(action.response.clone());
            duel.respond(&action.response);
            if action.label.to_lowercase().contains(step) {
                break;
            }
        }
    }
    // Finish the summon (its zone) until the turn is back to us.
    loop {
        let (Step::Decision { .. }, actions) = duel.advance_auto(&policy, &mut answers).unwrap() else { panic!("duel ended") };
        if actions.iter().any(|a| a.kind == ygo_engine::ActionKind::EndTurn) {
            break;
        }
        let action =
            actions.iter().find(|a| a.label == "Don't chain" || a.kind == ygo_engine::ActionKind::Place).expect("zone or chain prompt");
        answers.push(action.response.clone());
        duel.respond(&action.response);
    }
    let monsters: Vec<_> = duel.query_location(0, location::MZONE).into_iter().flatten().map(|(c, _)| engine.cards().name(c)).collect();
    assert_eq!(monsters, ["Invoked Baybarron"]);
    // Picking the only Aleister and the only Invocation in the Deck, the
    // chain prompts and the positions were answered without asking.
    assert!(answers.len() > real.len() + 4, "{} answers for {} choices", answers.len(), real.len());

    // The answers replay the same line on a new duel.
    let mut again = Duel::goldfish(&engine, &deck, &[ALEISTER, CHALICE, MELTDOWN, ASH_ALT_ART, FUWALOS], 1).unwrap();
    for a in &answers {
        again.advance().unwrap();
        again.respond(a);
    }
    again.advance().unwrap();
    assert_eq!(again.fingerprint(), duel.fingerprint());
}
