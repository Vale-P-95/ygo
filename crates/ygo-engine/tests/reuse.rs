//! A duel over is emptied and used for the thread's next one (the base
//! scripts stay loaded): it must play exactly like a new duel. Random duels
//! are played one after the other in reused duels, then each again in a new
//! duel, and every step must be the same. Needs the downloaded game data.

use std::path::PathBuf;

use ygo_engine::{fresh_duels, Action, ActionKind, DeckList, Duel, Engine, Opponent, Step};

/// What a duel showed at each step: the step, the full state, the hand order.
fn play(engine: &Engine, deck: &DeckList, hand: &[u32], opponent: &Opponent, answers: &mut Vec<Vec<u8>>, rng: &mut u64) -> Vec<String> {
    let replay = !answers.is_empty();
    let mut duel = Duel::goldfish_vs(engine, deck, hand, 1, opponent).unwrap();
    let mut seen = Vec::new();
    for i in 0..150 {
        let step = duel.advance();
        seen.push(format!("{step:?}\n{:?}", duel.state()));
        let Ok(Step::Decision { decision, .. }) = step else { break };
        let answer = if replay {
            match answers.get(i) {
                Some(a) => a.clone(),
                None => break,
            }
        } else {
            let list = duel.actions(&decision);
            // Ending the turn now and then, so both players play.
            let playable: Vec<&Action> = list.iter().filter(|a| a.kind != ActionKind::EndTurn || next(rng, 6) == 0).collect();
            let Some(a) = playable.get(next(rng, playable.len().max(1))) else { break };
            answers.push(a.response.clone());
            a.response.clone()
        };
        duel.respond(&answer);
    }
    seen
}

fn next(rng: &mut u64, n: usize) -> usize {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    (*rng % n as u64) as usize
}

#[test]
fn reused_duels_play_like_new_ones() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let ydk = std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap();
    let deck = DeckList::from_ydk(&ydk).unwrap();
    let opponent =
        Opponent { hand: deck.main[..5].to_vec(), deck: deck.main[5..].to_vec(), extra: deck.extra.clone(), ..Opponent::default() };
    let mut rng = 2026u64;
    let mut duels = Vec::new();
    for k in 0..30 {
        let hand: Vec<u32> = (0..5).map(|i| deck.main[(k * 7 + i * 3) % deck.main.len()]).collect();
        let mut hand = hand;
        hand.sort_unstable();
        hand.dedup();
        let mut answers = Vec::new();
        let reused = play(&engine, &deck, &hand, &opponent, &mut answers, &mut rng);
        duels.push((hand, answers, reused));
    }
    fresh_duels(true);
    for (k, (hand, mut answers, reused)) in duels.into_iter().enumerate() {
        let new = play(&engine, &deck, &hand, &opponent, &mut answers, &mut rng);
        for (i, (a, b)) in reused.iter().zip(&new).enumerate() {
            assert_eq!(a, b, "duel {k}, step {i}: a reused duel plays differently from a new one");
        }
    }
    fresh_duels(false);
}
