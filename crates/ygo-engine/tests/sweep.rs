//! A short randomized sweep (the CLI's `sweep` runs the long one): random
//! duels between random decks must never meet a decision without answers
//! or an answer the engine rejects. Needs the downloaded game data.

use std::path::PathBuf;

use ygo_engine::sweep::{sweep, SweepConfig};
use ygo_engine::Engine;

#[test]
fn random_duels_never_get_stuck() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let config = SweepConfig { duels: 80, max_decisions: 300, max_turns: 6, seed: 2026, threads: 4 };
    let report = sweep(&engine, &config);
    assert_eq!(report.duels, 80);
    assert!(report.decisions > 5_000, "only {} decisions", report.decisions);
    let problems: Vec<_> =
        report.problems.iter().map(|p| format!("{:?} in duel {}: {} {}", p.kind, p.duel, p.decision, p.detail)).collect();
    assert!(problems.is_empty(), "{problems:#?}");
}
