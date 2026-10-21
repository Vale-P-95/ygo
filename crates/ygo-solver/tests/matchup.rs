//! The probe turn against a chosen deck (#44): its breakers and handtraps,
//! read from the engine. Skipped without the game data, like `invoked.rs`.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ygo_engine::{DeckList, Engine};
use ygo_solver::{draw_trade_off, matchup, search, Goal, Kit, Prober, Rival, SearchConfig, Stop};

const ALEISTER: u32 = 86120751;
const CHALICE: u32 = 76334960;
const MELTDOWN: u32 = 47679935;
const ASH: u32 = 14558127;
const FUWALOS: u32 = 42141493;

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

impl Lab {
    fn code(&self, name: &str) -> u32 {
        self.engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name}")).code
    }

    /// The Invoked deck with these cards added: the opponent's deck.
    fn rival_deck(&self, cards: &[(&str, usize)]) -> DeckList {
        let mut deck = self.deck.clone();
        for &(name, copies) in cards {
            deck.main.extend(std::iter::repeat_n(self.code(name), copies));
        }
        deck
    }
}

#[test]
fn a_deck_brings_the_breakers_and_handtraps_it_can_play() {
    let Some(lab) = lab() else { return };
    let kit = Kit::standard(&lab.engine);
    let deck = lab.rival_deck(&[
        ("Dark Hole", 1),
        ("Gameciel, the Sea Turtle Kaiju", 1),
        ("Dogoran, the Mad Flame Kaiju", 1),
        ("Evenly Matched", 1),
        ("Clockwork Night", 1),
        ("Maxx \"C\"", 3),
    ]);
    let rival = Rival::new(&lab.engine, &deck, 1, &kit);
    let breakers = rival.breakers();
    // Dark Hole from the hand, the Kaiju by their own summon (Dogoran by
    // the text, its monster on your field in place of yours); Evenly Matched
    // reads like a breaker, but it is played at the end of the Battle Phase;
    // Clockwork Night only lowers ATK.
    assert!(breakers.contains(&lab.code("Dark Hole")), "{breakers:?}");
    assert!(breakers.contains(&lab.code("Gameciel, the Sea Turtle Kaiju")), "{breakers:?}");
    assert!(breakers.contains(&lab.code("Dogoran, the Mad Flame Kaiju")), "{breakers:?}");
    assert!(!breakers.contains(&lab.code("Evenly Matched")), "{breakers:?}");
    assert!(!breakers.contains(&lab.code("Clockwork Night")), "{breakers:?}");
    assert!(rival.hand_traps.contains(&ASH) && rival.hand_traps.contains(&lab.code("Maxx \"C\"")), "{:?}", rival.hand_traps);

    // The opponent's Deck is as big as during your first turn: plain cards
    // in the standard kit, its own cards against it (Aleister among them),
    // with its Extra Deck too.
    assert_eq!(kit.opponent().deck.len(), 35);
    let theirs = rival.kit().opponent();
    assert_eq!(theirs.deck.len(), 35);
    assert!(theirs.deck.iter().filter(|&&c| lab.deck.main.contains(&c)).count() > 15, "{:?}", theirs.deck);
    assert!(theirs.extra.contains(&lab.code("Invoked Mechaba")), "{:?}", theirs.extra);
}

#[test]
fn a_handtrap_counts_where_it_stops_the_line() {
    let Some(lab) = lab() else { return };
    let engine = &lab.engine;
    let kit = Kit::standard(engine);
    // An Invoked Mechaba line: Aleister searches, Magical Meltdown searches.
    let hand = vec![ALEISTER, CHALICE, MELTDOWN, ASH, FUWALOS];
    let mut config = SearchConfig::new(lab.deck.clone(), hand.clone());
    config.threads = 1;
    config.max_nodes = 4000;
    config.time_limit = Duration::from_secs(300);
    config.policy.passive = vec![ASH, FUWALOS];
    config.hand_traps = vec![ASH, FUWALOS];
    config.opponent = Some(kit.opponent().clone());
    config.goal = Goal::cards(&[lab.code("Invoked Mechaba")], true, true);
    let result = search(engine, &config, &AtomicBool::new(false), |_| {});
    assert_eq!(result.stop, Stop::GoalReached);
    let line = &result.boards[0].line;
    let measure = Prober::new(engine, &lab.deck, &hand, config.seed, &kit).measure(line).expect("the probe turn is played");

    let deck = lab.rival_deck(&[("Maxx \"C\"", 3), ("Dark Hole", 1), ("Ultimate Slayer", 1)]);
    let rival = Rival::new(engine, &deck, config.seed, &kit);
    let x = matchup(engine, &lab.deck, &hand, config.seed, line, &measure, &rival).expect("the matchup is played");
    let of = |code: u32| x.hand_traps.iter().find(|h| h.code == code);
    // Ash Blossom negates a search of the line; Maxx "C" makes the opponent
    // draw, but the line ends on the same board.
    let ash = of(ASH).expect("Ash is offered");
    assert!(ash.stops > 0 && ash.chance > 0.0, "{ash:?}");
    let maxx = of(lab.code("Maxx \"C\"")).expect("Maxx \"C\" is offered");
    assert!(maxx.windows > 0 && maxx.stops == 0, "{maxx:?}");
    // It draws a card for each summon left in the line: the most at the
    // first moment, fewer later on. Ash draws nothing.
    let first = maxx.moments.first().expect("moments").drawn;
    assert!(maxx.drawn > 0 && first == maxx.drawn, "{maxx:?}");
    assert!(maxx.moments.last().is_some_and(|m| m.drawn <= first), "{maxx:?}");
    assert_eq!(maxx.moments.len() as u32, maxx.windows.min(24));
    assert_eq!(ash.drawn, 0, "{ash:?}");

    // Where to stop the line against Maxx "C": the whole line comes last
    // and draws as much as the first moment; the best stop keeps at least
    // as much as the whole line, and no stop more than its board.
    let offs = draw_trade_off(engine, &lab.deck, &hand, config.seed, line, &rival).expect("the stops are measured");
    assert!(offs.iter().all(|o| o.code != ASH), "{offs:?}");
    let off = offs.iter().find(|o| o.code == lab.code("Maxx \"C\"")).expect("Maxx \"C\" draws");
    let whole = off.stops.last().expect("the whole line");
    assert_eq!(whole.drawn, maxx.drawn, "{off:?}");
    assert!(off.stops[off.best].expected >= whole.expected - 1e-9, "{off:?}");
    assert!(off.stops.iter().all(|p| p.expected <= p.field_value + p.floodgate_value + 1e-9), "{off:?}");
    assert!(off.stops.windows(2).all(|w| w[0].answers < w[1].answers && w[0].choices <= w[1].choices), "{off:?}");
    assert!(x.hand_trap_chance > 0.0 && x.hand_trap_chance < 1.0);
    // One Dark Hole: the field holds unless the opponent has it.
    let hole = x.breakers.iter().find(|b| b.code == lab.code("Dark Hole")).expect("Dark Hole");
    assert!(hole.chance > 0.0 && hole.chance < 0.5, "{hole:?}");
    assert!(x.resilience >= hole.remaining && x.resilience <= measure.field_value + 1e-9, "{x:?}");
    // Ultimate Slayer pays with a monster of its Extra Deck of the type of
    // the one it shuffles away (Invoked Mechaba is a Fusion): it has its
    // real Extra Deck, so it can be played.
    let slayer = x.breakers.iter().find(|b| b.code == lab.code("Ultimate Slayer")).expect("Ultimate Slayer");
    assert!(!slayer.not_applicable, "{slayer:?}");
}

#[test]
fn a_deck_opens_with_its_starters() {
    let Some(lab) = lab() else { return };
    // The Invoked deck's best lines play Aleister, Invocation or the cards
    // that bring them; none opens with a handtrap.
    let plays = ygo_solver::rival_plays(&lab.engine, &lab.deck, 1, 4, 400, 2);
    assert!(!plays.is_empty());
    let names: Vec<String> = plays.iter().map(|p| lab.engine.cards().name(p.code)).collect();
    assert!(names.iter().any(|n| n.contains("Aleister") || n.contains("Invocation") || n.contains("Aiwass")), "{names:?}");
    assert!(plays.iter().all(|p| p.share > 0.0 && p.share <= 1.0 && p.order >= 1.0), "{plays:?}");
}
