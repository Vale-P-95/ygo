//! The probe turn on boards put straight on the field (plan of the board
//! classification, C4). Skipped without the game data, like `invoked.rs`.

use std::path::PathBuf;

use ygo_engine::consts::{location, position};
use ygo_engine::{DeckList, Engine, Placement};
use ygo_solver::{Effect, Kit, Measure, Probe, Prober};

struct Lab {
    engine: Engine,
    deck: DeckList,
}

fn lab() -> Option<Lab> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let deck = DeckList::from_ydk(&std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap()).unwrap();
    Some(Lab { engine, deck })
}

/// Where a card is put.
enum At {
    Monster,
    Set,
    FaceUp,
}

impl Lab {
    fn code(&self, name: &str) -> u32 {
        self.engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name}")).code
    }

    /// Measures these cards on your field, with `hand` in your hand (cards
    /// that do nothing on the opponent's turn when empty).
    fn measure(&self, field: &[(&str, At)], hand: &[&str]) -> Measure {
        self.measure_with(field, hand, &[])
    }

    /// [`Lab::measure`], with these cards added to your Deck.
    fn measure_with(&self, field: &[(&str, At)], hand: &[&str], in_deck: &[&str]) -> Measure {
        let mut hand: Vec<u32> = hand.iter().map(|n| self.code(n)).collect();
        if hand.is_empty() {
            hand = ["Alexandrite Dragon", "Pot of Greed", "Trap Hole"].iter().map(|n| self.code(n)).collect();
        }
        let mut deck = self.deck.clone();
        deck.main.extend(&hand);
        deck.main.extend(in_deck.iter().map(|n| self.code(n)));
        let mut monsters = 0;
        let mut spells = 1;
        let placed: Vec<Placement> = field
            .iter()
            .map(|(name, at)| {
                let (location, sequence, position) = match at {
                    At::Monster => {
                        monsters += 1;
                        (location::MZONE, monsters - 1, position::FACEUP_ATTACK)
                    }
                    At::Set => {
                        spells += 1;
                        (location::SZONE, spells - 1, position::FACEDOWN_DEFENSE)
                    }
                    At::FaceUp => {
                        spells += 1;
                        (location::SZONE, spells - 1, position::FACEUP_ATTACK)
                    }
                };
                Placement { code: self.code(name), location, sequence, position, overlay: Vec::new() }
            })
            .collect();
        let kit = Kit::standard(&self.engine);
        let mut prober = Prober::new(&self.engine, &deck, &hand, 1, &kit);
        prober.measure_placed(&placed).expect("the probe turn is played")
    }
}

fn covers(m: &Measure, card: u32) -> Vec<Probe> {
    m.interruptions.iter().find(|i| i.responder.code == card).map(|i| i.covers.clone()).unwrap_or_default()
}

#[test]
fn a_step_lost_because_an_earlier_one_was_stopped_is_not_counted() {
    let Some(lab) = lab() else { return };
    // Trap Hole destroys Celtic Guardian when it is Normal Summoned: Link
    // Spider (Celtic is its material) and Link Spider's effect are lost
    // too, but Trap Hole cannot answer them. They are not what it stops.
    let m = lab.measure(&[("Trap Hole", At::Set)], &[]);
    let hole = covers(&m, lab.code("Trap Hole"));
    assert!(!hole.is_empty());
    assert!(!hole.contains(&Probe::FieldIgnition), "{hole:?}");
}

#[test]
fn a_floodgate_counts_what_it_stops_not_what_follows() {
    let Some(lab) = lab() else { return };
    // Monsters sent to the GY are banished: Glow-Up Bulb is not sent, and
    // its effect from the GY is a step that follows, not a second floodgate.
    let m = lab.measure(&[("Dimensional Fissure", At::FaceUp)], &[]);
    assert!(m.floodgates.contains(&Probe::SendToGrave), "{:?}", m.floodgates);
    assert!(!m.floodgates.contains(&Probe::GraveEffect), "{:?}", m.floodgates);
}

#[test]
fn infinite_impermanence_set_answers_the_monsters_already_on_the_field() {
    let Some(lab) = lab() else { return };
    // A monster of yours that does nothing, as on most boards.
    let m = lab.measure(&[("Alexandrite Dragon", At::Monster), ("Infinite Impermanence", At::Set)], &[]);
    let imperm = covers(&m, lab.code("Infinite Impermanence"));
    assert!(imperm.contains(&Probe::FieldIgnition) && imperm.contains(&Probe::DeckSummon), "{imperm:?}");
}

fn effect(m: &Measure, card: u32) -> Option<Effect> {
    m.interruptions.iter().find(|i| i.responder.code == card).map(|i| i.effect)
}

#[test]
fn a_monster_destroyed_after_landing_was_summoned() {
    let Some(lab) = lab() else { return };
    // Trap Hole destroys Celtic Guardian once its Normal Summon is done:
    // the summon worked, Link Spider (Celtic is its material) is what is lost.
    let m = lab.measure(&[("Trap Hole", At::Set)], &[]);
    let hole = lab.code("Trap Hole");
    assert_eq!(effect(&m, hole), Some(Effect::Destroy));
    let covers = covers(&m, hole);
    assert!(!covers.contains(&Probe::NormalSummon) && covers.contains(&Probe::ExtraSummon), "{covers:?}");
    // Torrential Tribute destroys every monster once a summon is done: it
    // negates no summon, and stops only what needed the monsters.
    let m = lab.measure(&[("Torrential Tribute", At::Set)], &[]);
    let torrential = m.interruptions.iter().find(|i| i.responder.code == lab.code("Torrential Tribute")).expect("Torrential");
    assert_eq!(torrential.effect, Effect::Destroy);
    assert!(torrential.coverage < 0.5, "{torrential:?}");
}

#[test]
fn a_summon_negated_never_lands() {
    let Some(lab) = lab() else { return };
    // Solemn Warning also negates an activation that summons (and then
    // shows that, the stronger effect).
    let m = lab.measure(&[("Solemn Warning", At::Set)], &[]);
    let solemn = lab.code("Solemn Warning");
    assert!(effect(&m, solemn).is_some_and(|e| e <= Effect::NegateSummon), "{:?}", effect(&m, solemn));
    let covers = covers(&m, solemn);
    assert!(covers.contains(&Probe::NormalSummon) || covers.contains(&Probe::ExtraSummon), "{covers:?}");
}

#[test]
fn a_monster_turned_face_down_has_its_effects_negated() {
    let Some(lab) = lab() else { return };
    // Destructive Daruma Karma Cannon turns the monsters face-down: they
    // lose their effects, more than a change of position.
    let m = lab.measure(&[("Destructive Daruma Karma Cannon", At::Set)], &[]);
    let daruma = lab.code("Destructive Daruma Karma Cannon");
    assert!(effect(&m, daruma).is_some_and(|e| e <= Effect::NegateEffect), "{:?}", effect(&m, daruma));
}

#[test]
fn a_response_is_worth_what_it_brings() {
    let Some(lab) = lab() else { return };
    // Big Welcome Labrynth stops nothing itself: it Special Summons a
    // Labrynth monster from the Deck (the strongest, Lovely Labrynth of the
    // Silver Castle), whose effect destroys a card when the Trap resolves.
    // It then returns one of your monsters to the hand: the weakest.
    let field = [("Alexandrite Dragon", At::Monster), ("Big Welcome Labrynth", At::Set)];
    let m = lab.measure_with(&field, &[], &["Lovely Labrynth of the Silver Castle", "Labrynth Cooclock"]);
    let welcome = covers(&m, lab.code("Big Welcome Labrynth"));
    assert!(!welcome.is_empty(), "{:?}", m.interruptions);
}

#[test]
fn a_card_that_comes_back_from_the_gy_is_follow_up() {
    let Some(lab) = lab() else { return };
    // The breaker destroys the monster: Glow-Up Bulb can come back from the
    // GY in your next turn, a Normal Monster cannot.
    let bulb = lab.measure(&[("Glow-Up Bulb", At::Monster)], &[]).follow_up.expect("your next turn is reached");
    let plain = lab.measure(&[("Alexandrite Dragon", At::Monster)], &[]).follow_up.expect("your next turn is reached");
    assert!(bulb.rebuild > plain.rebuild, "{bulb:?} against {plain:?}");
}
