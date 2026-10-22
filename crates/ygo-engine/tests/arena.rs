//! A duel made in a memory arena, saved in the middle of a turn and put
//! back, plays the rest exactly as before and exactly like a duel replayed
//! from the start without an arena (plan B13). Random lines of play on the
//! test deck. Needs the downloaded game data.

use std::path::PathBuf;

use ygo_engine::{arena_duels, Action, ActionKind, CardDb, DeckList, Duel, DuelConfig, Engine, Opponent, ScriptStore, Step};

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

fn next(rng: &mut u64, n: usize) -> usize {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    (*rng % n.max(1) as u64) as usize
}

/// What the duel shows at each answer of `answers`: the step and the state.
fn play(duel: &mut Duel, answers: &[Vec<u8>]) -> Vec<String> {
    let mut seen = Vec::new();
    for a in answers {
        let step = duel.advance();
        seen.push(format!("{:?}", step.map(|s| matches!(s, Step::Decision { .. }))));
        seen.push(format!("{:?}", duel.state()));
        duel.respond(a);
    }
    seen
}

/// Plays a random line for each seed and checks restores against a replay;
/// returns how many lines were long enough to check.
fn check_lines(engine: &Engine, deck: &DeckList, seeds: std::ops::RangeInclusive<u64>) -> usize {
    let hand: Vec<u32> = deck.main.iter().copied().take(5).collect();
    let mut rng = 0x9e37_79b9_u64 ^ *seeds.start();
    let mut checked = 0;
    for seed in seeds {
        // A random line to the end of the turn (at most 60 answers).
        arena_duels(false);
        let mut duel = Duel::goldfish_vs(engine, deck, &hand, seed, &Opponent::default()).unwrap();
        let mut answers: Vec<Vec<u8>> = Vec::new();
        for _ in 0..60 {
            let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
            let list = duel.actions(&decision);
            let playable: Vec<&Action> = list.iter().filter(|a| a.kind != ActionKind::EndTurn || next(&mut rng, 8) == 0).collect();
            let Some(a) = playable.get(next(&mut rng, playable.len())) else { break };
            answers.push(a.response.clone());
            duel.respond(&a.response);
        }
        drop(duel);
        if answers.len() < 4 {
            continue;
        }
        let k = answers.len() / 2;
        // Replayed from the start, as the search does without an arena.
        let mut replayed = Duel::goldfish_vs(engine, deck, &hand, seed, &Opponent::default()).unwrap();
        play(&mut replayed, &answers[..k]);
        let expected = play(&mut replayed, &answers[k..]);

        arena_duels(true);
        let mut duel = Duel::goldfish_vs(engine, deck, &hand, seed, &Opponent::default()).unwrap();
        play(&mut duel, &answers[..k]);
        let saved = duel.save().unwrap();
        assert!(saved.size() > 0);
        assert_eq!(play(&mut duel, &answers[k..]), expected, "seed {seed}: the duel in an arena plays differently");
        for _ in 0..3 {
            duel.restore(&saved).unwrap();
            assert_eq!(play(&mut duel, &answers[k..]), expected, "seed {seed}: a restored duel plays differently");
        }
        // Another duel's save does not go in.
        let other = Duel::goldfish_vs(engine, deck, &hand, seed, &Opponent::default()).unwrap();
        assert!(duel.restore(&other.save().unwrap()).is_err());
        checked += 1;
    }
    arena_duels(false);
    checked
}

#[test]
fn a_restored_duel_plays_like_a_replayed_one() {
    let Some((engine, deck)) = setup() else { return };
    let checked = check_lines(&engine, &deck, 1..=12);
    assert!(checked >= 6, "too few lines long enough: {checked}");
}

#[test]
fn duels_in_arenas_on_several_threads_do_not_mix() {
    let Some((engine, deck)) = setup() else { return };
    let checked: usize = std::thread::scope(|s| {
        let threads: Vec<_> = (0..4u64)
            .map(|t| {
                let (engine, deck) = (&engine, &deck);
                s.spawn(move || check_lines(engine, deck, 101 + t * 6..=106 + t * 6))
            })
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).sum()
    });
    assert!(checked >= 8, "too few lines long enough: {checked}");
}

#[test]
fn a_save_does_not_go_into_a_later_duel() {
    let Some((engine, _)) = setup() else { return };
    arena_duels(true);
    let first = Duel::new(&engine, &DuelConfig::default()).unwrap();
    let saved = first.save().unwrap();
    drop(first);
    // Later duels take its slot again and, sooner or later, its address.
    for i in 0..300 {
        let mut duel = Duel::new(&engine, &DuelConfig::default()).unwrap();
        assert!(duel.restore(&saved).is_err(), "duel {i} took the save of a dropped one");
    }
    arena_duels(false);
}

#[test]
fn a_duel_that_cannot_load_its_base_scripts_gives_its_arena_back_once() {
    // No game data needed: base scripts that fail on purpose.
    let dir = std::env::temp_dir().join(format!("duelforge-broken-scripts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("constant.lua"), "").unwrap();
    std::fs::write(dir.join("utility.lua"), "error('broken on purpose')").unwrap();
    let engine = Engine::new(CardDb::default(), ScriptStore::load_dir(&dir).unwrap());
    arena_duels(true);
    // More failures than arenas: each arena is freed, once.
    for _ in 0..300 {
        let err = Duel::new(&engine, &DuelConfig::default()).err().expect("the base scripts fail");
        assert!(err.to_string().contains("utility.lua"), "{err}");
    }
    arena_duels(false);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_duel_without_arena_cannot_be_saved() {
    let Some((engine, deck)) = setup() else { return };
    arena_duels(false);
    let hand: Vec<u32> = deck.main.iter().copied().take(5).collect();
    let duel = Duel::goldfish_vs(&engine, &deck, &hand, 1, &Opponent::default()).unwrap();
    assert!(duel.save().is_err());
}
