//! Regression tests of the search, run by the CI on every pull request:
//! fixed hands of the Invoked test deck, one thread and a node budget, so
//! the boards do not depend on the machine's load. The thresholds are what
//! the search finds today with some room: a better search keeps passing,
//! one that loses the known lines or plays pointless moves fails.
//!
//! Release only (in debug each search takes minutes):
//! `cargo test --release -p ygo-solver --test search_regression`.
//! Skipped when the game data is missing.
//!
//! When one fails on a pull request that does not touch the search:
//! - The game data is not pinned: `fetch-data` takes the scripts and the
//!   card database as they are upstream that day, and a rewritten script or
//!   a renamed card can change the boards or break a lookup by name.
//! - The thresholds depend on the board estimate (`quick_score`): a change
//!   of its scale or weights moves them. To set them again, run the tests at
//!   2000 and 4000 nodes and keep each threshold below the 2000-node value.
//!
//! Not covered: dropping a line where one of your effects negates your own
//! chain link (only learning not to chain it is).

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use ygo_engine::{DeckList, DuelState, Engine};
use ygo_solver::{
    card_values, explain, search, Goal, GoalCard, GoalZone, HappeningKind, Kit, MechanicKind, SearchConfig, SearchResult, Stop,
};

/// Nodes per search: about a minute in release. The thresholds below hold
/// already with half of them, so a search that reaches the same boards a
/// little later keeps passing.
const NODES: u64 = 4000;
/// The best boards checked: the ranking among boards of the same strength
/// may change, the strength of the top may not.
const TOP: usize = 5;

/// Invocation - "Grail" is still under its prerelease name in the
/// engine's data ("Chalice"): by passcode.
const GRAIL: &str = "76334960";

/// What a hand must give.
struct Expect {
    hand: &'static [&'static str],
    /// The search may activate the handtraps on your own turn: it must
    /// learn by itself that they do nothing there.
    use_hand_traps: bool,
    /// Cards that one of the best boards has on the field, all together.
    pieces: &'static [&'static str],
    /// Monsters on the field of the best board, at least.
    monsters: usize,
    /// Cards of the starting hand still in the hand of the best board, at least.
    kept: usize,
    /// The best board's estimate, at least.
    min_score: f64,
    /// Cards whose effects the search activates (coverage).
    activated: &'static [&'static str],
    /// Monsters the search summons (coverage).
    summoned: &'static [&'static str],
}

struct Lab {
    engine: Engine,
    deck: DeckList,
    kit: Kit,
}

fn lab() -> Option<Lab> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let text = std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap();
    let deck = DeckList::from_ydk(&text).unwrap();
    let kit = Kit::standard(&engine);
    Some(Lab { engine, deck, kit })
}

impl Lab {
    /// A card by name or passcode.
    fn card(&self, name: &str) -> u32 {
        let db = self.engine.cards();
        name.parse()
            .ok()
            .filter(|&c| db.get(c).is_some())
            .unwrap_or_else(|| db.find_by_name(name).unwrap_or_else(|| panic!("no card {name}")).code)
    }

    /// Its canonical code (alternate artworks are the same card).
    fn code(&self, name: &str) -> u32 {
        self.engine.cards().canonical_code(self.card(name))
    }

    fn name(&self, code: u32) -> String {
        self.engine.cards().name(code)
    }

    /// The search as the Combo Lab sets it up, on one thread with a node budget.
    fn search(&self, hand: &[u32], use_hand_traps: bool) -> (SearchConfig, SearchResult) {
        self.search_with(hand, use_hand_traps, Goal::default())
    }

    /// The same with a goal, the handtraps kept.
    fn search_for(&self, hand: &[u32], goal: Goal) -> (SearchConfig, SearchResult) {
        self.search_with(hand, false, goal)
    }

    fn search_with(&self, hand: &[u32], use_hand_traps: bool, goal: Goal) -> (SearchConfig, SearchResult) {
        let config = self.config(hand, use_hand_traps, goal);
        let result = search(&self.engine, &config, &AtomicBool::new(false), |_| {});
        (config, result)
    }

    fn config(&self, hand: &[u32], use_hand_traps: bool, goal: Goal) -> SearchConfig {
        let engine = &self.engine;
        let mut config = SearchConfig::new(self.deck.clone(), hand.to_vec());
        config.goal = goal;
        config.threads = 1;
        config.max_nodes = NODES;
        // Far more than needed: a search cut by the clock would depend on the machine.
        config.time_limit = Duration::from_secs(900);
        config.opponent = Some(self.kit.opponent().clone());
        config.hand_traps = ygo_solver::hand_traps(engine, &self.deck, config.seed, &self.kit);
        config.values = Arc::new(card_values(engine, &self.deck, config.seed, &self.kit, &config.hand_traps, 1));
        config.learn = Some(Arc::new(self.kit.clone()));
        if !use_hand_traps {
            config.policy.passive = config.hand_traps.clone();
        }
        config.coverage = true;
        config
    }

    /// Your cards on the field (canonical codes).
    fn field(&self, state: &DuelState) -> Vec<u32> {
        let me = &state.players[0];
        me.monsters.iter().chain(&me.spells).flatten().map(|c| self.engine.cards().canonical_code(c.code)).collect()
    }

    fn describe(&self, result: &SearchResult) -> String {
        let mut text = format!(
            "{:?}: {} nodes, {} replays, {} boards, {} interrupted\n",
            result.stop, result.progress.nodes, result.progress.replays, result.progress.boards, result.interrupted
        );
        for (i, b) in result.boards.iter().take(TOP).enumerate() {
            let field: Vec<String> = self.field(&b.state).iter().map(|&c| self.name(c)).collect();
            let hand: Vec<String> = b.state.players[0].hand.iter().map(|&c| self.name(c)).collect();
            text += &format!(
                "#{} score {:.1}{} · {} choices · field: {} · hand: {}\n",
                i + 1,
                b.quick_score,
                if b.exact { "" } else { " (provisional)" },
                b.line.choices.len(),
                field.join(", "),
                hand.join(", ")
            );
        }
        text
    }
}

fn check(expect: Expect) {
    let Some(lab) = lab() else { return };
    let db = lab.engine.cards();
    let hand: Vec<u32> = expect.hand.iter().map(|n| lab.card(n)).collect();
    let (config, result) = lab.search(&hand, expect.use_hand_traps);
    let summary = lab.describe(&result);
    eprintln!("{summary}");

    // The node budget is what stops it, and every line plays to its end.
    assert!(matches!(result.stop, Stop::NodeLimit | Stop::Exhausted), "{summary}");
    assert_eq!(result.interrupted, 0, "interrupted lines\n{summary}");
    assert!(!result.boards.is_empty(), "no boards\n{summary}");
    let top = &result.boards[..result.boards.len().min(TOP)];

    // The best lines, played again: the same board, no chain link of yours
    // negated (the opponent does not answer on your turn: it was one of your
    // own cards), no handtrap activated on your own turn (it does nothing there).
    let hand_trap = |code: u32| config.hand_traps.iter().any(|&h| db.canonical_code(h) == db.canonical_code(code));
    for (i, b) in top.iter().enumerate() {
        let (state, steps) = explain(&lab.engine, &config.deck, &hand, config.seed, config.opponent.as_ref(), &b.line).unwrap();
        if b.exact {
            assert_eq!(state.players[0].monsters, b.state.players[0].monsters, "board #{} plays to another board\n{summary}", i + 1);
        }
        for s in &steps {
            for h in &s.happened {
                assert!(h.kind != HappeningKind::Negate, "board #{}: {} after {:?}\n{summary}", i + 1, h.text, s.choice);
                assert!(
                    !(h.kind == HappeningKind::Activate && h.mine && hand_trap(h.code)),
                    "board #{}: handtrap activated on your turn: {}\n{summary}",
                    i + 1,
                    h.text
                );
            }
        }
    }

    // The strength of the best board.
    let best = &result.boards[0];
    assert!(best.quick_score >= expect.min_score, "best score below {}\n{summary}", expect.min_score);
    let monsters = best.state.players[0].monsters.iter().flatten().count();
    assert!(monsters >= expect.monsters, "{monsters} monsters, expected at least {}\n{summary}", expect.monsters);
    let mut left: Vec<u32> = best.state.players[0].hand.iter().map(|&c| db.canonical_code(c)).collect();
    let kept = hand
        .iter()
        .filter(|&&c| {
            let i = left.iter().position(|&h| h == db.canonical_code(c));
            i.map(|i| left.swap_remove(i)).is_some()
        })
        .count();
    assert!(kept >= expect.kept, "{kept} cards of the hand kept, expected at least {}\n{summary}", expect.kept);
    let pieces: Vec<u32> = expect.pieces.iter().map(|n| lab.code(n)).collect();
    assert!(
        top.iter().any(|b| {
            let field = lab.field(&b.state);
            pieces.iter().all(|p| field.contains(p))
        }),
        "no board among the best {TOP} with {}\n{summary}",
        expect.pieces.join(", ")
    );

    // The main cards of the hand's lines are really played.
    let counts = result.coverage.as_ref().expect("coverage asked").sorted();
    for name in expect.activated {
        let code = lab.code(name);
        let n: u64 = counts.iter().filter(|(m, _)| m.code == code && m.kind == MechanicKind::Activate).map(|(_, c)| c.activated).sum();
        assert!(n > 0, "{name} never activated\n{summary}");
    }
    for name in expect.summoned {
        let code = lab.code(name);
        let n: u64 = counts.iter().filter(|(m, _)| m.code == code).map(|(_, c)| c.summoned).sum();
        assert!(n > 0, "{name} never summoned\n{summary}");
    }

    // What does nothing is learned, not tried on every line: a handtrap that
    // did nothing on your turn three times is not activated again (the
    // search's IDLE_STRIKES; kept passive, never). Lines queued before that
    // and ended as duplicates before the chain resolved count too: today at
    // most 11 times, without learning over 100.
    let limit = if expect.use_hand_traps { 20 } else { 0 };
    for &h in &config.hand_traps {
        let n: u64 = counts
            .iter()
            .filter(|(m, _)| m.code == db.canonical_code(h) && m.kind == MechanicKind::Activate)
            .map(|(_, c)| c.activated)
            .sum();
        assert!(n <= limit, "{} activated {n} times on your turn, at most {limit}\n{summary}", lab.name(h));
    }
    // An effect that negated your own chain link is not chained to yours
    // again (today Invoked Mechaba, at most 8 times in a search, while the
    // line goes on after it: without that, about 50).
    for (m, c) in &counts {
        assert!(c.negated_own <= 16, "{} negated your own chain links {} times\n{summary}", lab.name(m.code), c.negated_own);
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn aleister_grail_meltdown_reach_mechaba() {
    check(Expect {
        hand: &["Aleister the Invoker", GRAIL, "Magical Meltdown", "Ash Blossom & Joyous Spring", "Mulcharmy Fuwalos"],
        use_hand_traps: true,
        pieces: &["Invoked Mechaba", "Magical Meltdown"],
        monsters: 2,
        kept: 1,
        min_score: 13.0,
        activated: &["Aleister the Invoker", GRAIL, "Magical Meltdown", "Invocation", "Invoked Sorath"],
        summoned: &["Invoked Mechaba", "Invoked Caliga"],
    });
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn reminiscent_alone_keeps_the_handtraps() {
    // The two Drolls are kept; the Mulcharmies only work while you control
    // no cards, so one may be spent. Invoked Caliga lets each player try one
    // monster effect a turn: with the kit weighed on real lines, where two
    // activations in three are monster effects, it stops more than Invoked
    // Mechaba's one negation, and comes first with Hyperinvoked Aeon.
    // (The estimate counts a measured monster as a 0.3 body plus its value,
    // not +1 or +2: the same board as before is 9.9, not 15.0.)
    check(Expect {
        hand: &["Aleister the Reminiscent", "Droll & Lock Bird", "Droll & Lock Bird", "Mulcharmy Fuwalos", "Mulcharmy Purulia"],
        use_hand_traps: true,
        pieces: &["Invoked Caliga", "Hyperinvoked Aeon"],
        monsters: 3,
        kept: 2,
        min_score: 9.5,
        activated: &["Aleister the Reminiscent", "Invocation - \"Sword\"", "Invocation", "Invoked Sorath", "Hyperinvoked Aeon"],
        summoned: &["Invoked Sorath", "Invoked Caliga", "Hyperinvoked Aeon"],
    });
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn two_aleisters_and_sword_reach_mechaba() {
    // Mechaba and Caliga with Invocation - "Sword" face-up: measured 2.32,
    // more than the boards with a third monster found before (2.09 at best),
    // which the estimate's fixed +1 or +2 a monster put first.
    check(Expect {
        hand: &[
            "Aleister the Reminiscent",
            "Aleister the Invoker",
            "Invocation - \"Sword\"",
            "Ash Blossom & Joyous Spring",
            "Crowley, the Gifted Magistus",
        ],
        use_hand_traps: false,
        pieces: &["Invoked Mechaba"],
        monsters: 2,
        kept: 1,
        min_score: 15.0,
        activated: &["Aleister the Reminiscent", "Aleister the Invoker", "Invocation - \"Sword\"", "Invocation", "Invoked Sorath"],
        summoned: &["Invoked Mechaba", "Invoked Caliga"],
    });
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn reminiscent_aiwass_grail_reach_mechaba() {
    check(Expect {
        hand: &["Aleister the Reminiscent", "Mulcharmy Purulia", "Aiwass, Divine Spirit of the Law", "Ash Blossom & Joyous Spring", GRAIL],
        use_hand_traps: false,
        pieces: &["Invoked Mechaba", "Invoked Caliga"],
        monsters: 2,
        kept: 1,
        // The same board as before, 14.6 with a measured monster as a 0.3 body.
        min_score: 14.0,
        activated: &["Aleister the Reminiscent", "Aiwass, Divine Spirit of the Law", GRAIL, "Invocation - \"Sword\"", "Invoked Sorath"],
        summoned: &["Invoked Mechaba", "Invoked Caliga"],
    });
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn a_goal_card_is_not_sent_away_first() {
    // Invoked Oceanus and S:P Little Knight on the field (the hands of the
    // goals checked in docs/plans/ricerca-esperimenti.md, section 3). Aleister the Reminiscent's
    // cost banishes an Invoked from the Extra Deck, Invocation banishes its
    // Fusion Materials: the goal cards were picked first there too, as if
    // picking them brought them closer, and no line kept them (0 of 4 hands
    // in 60 s).
    let Some(lab) = lab() else { return };
    let hands: [&[&str]; 4] = [
        &[
            "Aleister the Reminiscent",
            "Aleister the Invoker",
            "Invocation - \"Sword\"",
            "Ash Blossom & Joyous Spring",
            "Crowley, the Gifted Magistus",
        ],
        &["Aleister the Reminiscent", "Mulcharmy Purulia", "Aiwass, Divine Spirit of the Law", "Ash Blossom & Joyous Spring", GRAIL],
        &["Aleister the Reminiscent", "Crowley, the Gifted Magistus", "Invocation", "Super Polymerization", "Droll & Lock Bird"],
        &[
            "Aleister the Reminiscent",
            "Invocation - \"Sword\"",
            "Ash Blossom & Joyous Spring",
            "Magical Meltdown",
            "Crowley, the Gifted Magistus",
        ],
    ];
    let goal = Goal::cards(&[lab.card("Invoked Oceanus"), lab.card("S:P Little Knight")], true, true);
    for hand in hands {
        let hand: Vec<u32> = hand.iter().map(|n| lab.card(n)).collect();
        let (_, result) = lab.search_for(&hand, goal.clone());
        assert_eq!(
            result.stop,
            Stop::GoalReached,
            "{:?}\n{}",
            hand.iter().map(|&c| lab.name(c)).collect::<Vec<_>>(),
            lab.describe(&result)
        );
    }
}

impl Lab {
    /// A goal of all its cards: "Name@place" (`GoalZone` in lower case, or
    /// none for anywhere on the field), "~Name" for a soft card.
    fn goal(&self, cards: &[&str]) -> Goal {
        let cards = cards
            .iter()
            .map(|item| {
                let (item, hard) = item.strip_prefix('~').map_or((*item, true), |rest| (rest, false));
                let (name, zone) = match item.rsplit_once('@') {
                    Some((name, place)) => {
                        let zone = [
                            GoalZone::Extra,
                            GoalZone::Main,
                            GoalZone::Monster,
                            GoalZone::FaceUp,
                            GoalZone::Set,
                            GoalZone::FieldZone,
                            GoalZone::Grave,
                            GoalZone::Banished,
                            GoalZone::Hand,
                        ]
                        .into_iter()
                        .find(|z| format!("{z:?}").to_lowercase() == place)
                        .unwrap_or_else(|| panic!("no place {place}"));
                        (name, zone)
                    }
                    None => (item, GoalZone::Any),
                };
                GoalCard { code: self.card(name), zone, hard }
            })
            .collect();
        Goal { cards, all: true, first_only: true }
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn goals_off_the_field_and_set_cards_are_reached_or_proved_impossible() {
    // Each place of a goal card, reached from a hand that can (the cards
    // wanted off the field on a board played through the End Phase), and
    // proved out of reach from one that cannot: every line tried.
    let Some(lab) = lab() else { return };
    let hand = |names: &[&str]| names.iter().map(|n| lab.card(n)).collect::<Vec<u32>>();
    let meltdown = hand(&["Aleister the Invoker", GRAIL, "Magical Meltdown", "Ash Blossom & Joyous Spring", "Mulcharmy Fuwalos"]);
    let super_poly =
        hand(&["Aleister the Reminiscent", "Crowley, the Gifted Magistus", "Invocation", "Super Polymerization", "Droll & Lock Bird"]);
    let dead = hand(&[
        "Ash Blossom & Joyous Spring",
        "Mulcharmy Fuwalos",
        "Ash Blossom & Joyous Spring",
        "Mulcharmy Fuwalos",
        "Ash Blossom & Joyous Spring",
    ]);
    let cases: [(&[u32], &[&str], Option<bool>); 9] = [
        (&meltdown, &["Invoked Mechaba@monster", "Aleister the Invoker@banished", "Magical Meltdown@fieldzone"], Some(true)),
        (&meltdown, &["Invoked Mechaba@main", "76334960@grave", "~Mulcharmy Fuwalos@hand"], Some(true)),
        (&super_poly, &["Super Polymerization@set", "Invoked Mechaba", "Invocation@grave"], Some(true)),
        (&dead, &["Ash Blossom & Joyous Spring@hand"], Some(true)),
        (&dead, &["Ash Blossom & Joyous Spring@grave"], Some(false)),
        (&dead, &["Mulcharmy Fuwalos@banished"], Some(false)),
        (&dead, &["Ash Blossom & Joyous Spring@faceup"], Some(false)),
        // A soft card does not make a goal reachable, nor one alone a goal.
        (&dead, &["Mulcharmy Fuwalos@banished", "~Ash Blossom & Joyous Spring@hand"], Some(false)),
        (&dead, &["~Ash Blossom & Joyous Spring@hand"], None),
    ];
    for (hand, cards, verdict) in cases {
        let goal = lab.goal(cards);
        let (_, result) = lab.search_for(hand, goal.clone());
        let summary = format!("{cards:?}\n{}", lab.describe(&result));
        assert_eq!(result.goal_reached(&goal), verdict, "{summary}");
        match verdict {
            Some(true) => {
                assert_eq!(result.stop, Stop::GoalReached, "{summary}");
                let board = &result.boards[0];
                assert!(board.exact && goal.reached(board.goal_hits), "{summary}");
                assert_eq!(goal.hits(&lab.engine, &board.state).hard, cards.iter().filter(|c| !c.starts_with('~')).count(), "{summary}");
            }
            Some(false) => assert_eq!(result.stop, Stop::Exhausted, "{summary}"),
            None => assert_eq!(result.stop, Stop::Exhausted, "{summary}"),
        }
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn lines_started_from_copies_play_like_replayed_ones() {
    // The same search replaying every line from the start of the duel, and
    // starting lines from copies of the duel (a small cap, so copies are
    // also thrown away and made again): the same boards by the same lines.
    let Some(lab) = lab() else { return };
    let hand: Vec<u32> = [
        "Aleister the Reminiscent",
        "Aleister the Invoker",
        "Invocation - \"Sword\"",
        "Ash Blossom & Joyous Spring",
        "Crowley, the Gifted Magistus",
    ]
    .iter()
    .map(|n| lab.card(n))
    .collect();
    let run = |copies_mb| {
        let mut config = lab.config(&hand, false, Goal::default());
        config.max_nodes = 1500;
        config.copies_mb = copies_mb;
        search(&lab.engine, &config, &AtomicBool::new(false), |_| {})
    };
    let (replayed, copied) = (run(0), run(8));
    let boards = |r: &SearchResult| r.boards.iter().map(|b| (b.key, b.line.answers.clone(), b.quick_score.to_bits())).collect::<Vec<_>>();
    assert_eq!((replayed.progress.nodes, replayed.interrupted), (copied.progress.nodes, copied.interrupted));
    assert!(boards(&replayed) == boards(&copied), "{}\n{}", lab.describe(&replayed), lab.describe(&copied));
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow in debug: run with --release")]
fn checkpoints_change_nothing_and_a_longer_search_takes_the_same() {
    // The copies of the best boards at the checkpoints leave the search as
    // it is, and a search with more nodes takes the copies of a shorter one
    // first (docs/plans/monotonia-budget.md).
    let Some(lab) = lab() else { return };
    let hand: Vec<u32> = ["Aleister the Invoker", GRAIL, "Magical Meltdown", "Ash Blossom & Joyous Spring", "Mulcharmy Fuwalos"]
        .iter()
        .map(|n| lab.card(n))
        .collect();
    let run = |nodes: u64, checkpoints: &[u64]| {
        let mut config = lab.config(&hand, false, Goal::default());
        config.max_nodes = nodes;
        config.checkpoints = checkpoints.to_vec();
        config.checkpoint_top = 50;
        search(&lab.engine, &config, &AtomicBool::new(false), |_| {})
    };
    let boards = |b: &[ygo_solver::Board]| b.iter().map(|b| (b.key, b.line.answers.clone(), b.quick_score.to_bits())).collect::<Vec<_>>();
    let (plain, short, long) = (run(1000, &[]), run(1000, &[250, 500, 1000]), run(2000, &[250, 500, 1000, 2000]));
    assert!(boards(&plain.boards) == boards(&short.boards), "{}\n{}", lab.describe(&plain), lab.describe(&short));
    assert_eq!(short.checkpoints.len(), 3);
    assert_eq!(long.checkpoints.len(), 4);
    for ((n_short, b_short), (n_long, b_long)) in short.checkpoints.iter().zip(&long.checkpoints) {
        assert_eq!(n_short, n_long);
        assert!(!b_short.is_empty() && boards(b_short) == boards(b_long), "checkpoint at {n_short} nodes");
    }
}
