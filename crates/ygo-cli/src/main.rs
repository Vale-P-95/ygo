//! `duelforge-cli`: headless tools around the duel engine.
//!
//! - `fetch-data`: download card scripts and card databases (Project Ignis).
//! - `info`: engine version and loaded data.
//! - `play`: play a starting hand from a deck, interactively or by script.
//! - `bench`: measure duel creation, playout and replay speed.
//! - `combos`: search every line from a hand and list the end boards.
//! - `engine-mc`: P(board ≥ threshold) from many hands played by the engine,
//!   and the swaps of a copy that raise it.
//! - `sweep`: random duels between random decks, reporting any decision
//!   that cannot be answered or any answer the engine rejects.
//! - `synergy-census`: the census of the synergy plan and the reading deck.
//! - `synergy-read`, `accepts`, `archetype`: what the pool accepts, kept in `data/synergy/`.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ygo_data::{sync, CardStore};
use ygo_engine::consts::{location, phase, position};
use ygo_engine::sweep::SweepConfig;
use ygo_engine::{actions, describe_card, run_script, Action, ActionKind, DeckList, Duel, Engine, Event, ScriptEnd, Step, Trace};
use ygo_solver::mc_run::{cheap_composite, composite, fields, STAGED_CHEAP, STAGED_FULL, STAGED_ROUNDS, STAGED_WINDOWS};
use ygo_solver::InteractionProfile;

mod engine_mc;
mod synergy;

#[derive(Parser)]
#[command(name = "duelforge-cli", version, about)]
struct Cli {
    /// Folder with `scripts/` (CardScripts) and `cdb/` (BabelCDB).
    #[arg(long, global = true, default_value = "data")]
    data: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}

// Parsed once: the size of the largest command's options does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum Cmd {
    /// Download or update card scripts and card databases with git.
    FetchData,
    /// Show engine version and loaded data.
    Info,
    /// Download the card list from YGOPRODeck into `<data>/cards.db`.
    UpdateCards {
        /// Rebuild even if the local version is current.
        #[arg(long)]
        force: bool,
    },
    /// Record the backend's answers for a sample deck as JSON, for the mock mode of the app (no card texts).
    RecordFixtures {
        /// Sample deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
        /// Where to write the file.
        #[arg(long, default_value = "apps/desktop/src/lib/fixtures/recorded.json")]
        out: PathBuf,
        /// Extra search queries to record, besides the built-in ones and one per archetype of the deck.
        #[arg(long)]
        query: Vec<String>,
        /// Deck roles for the probabilities, e.g. `starter=14558127,23434538`; handtraps come from the handtrap tag.
        #[arg(long)]
        role: Vec<String>,
    },
    /// Search the card database with the same language as the app.
    Search {
        /// e.g. `attr:dark type:fusion atk>=2500 "negate"`
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Play a starting hand. Without --steps, asks for each decision.
    Play {
        #[command(flatten)]
        setup: Setup,
        /// Semicolon-separated choices, each matched (case-insensitive
        /// substring) against the labels of the current decision's actions.
        #[arg(long)]
        steps: Option<String>,
        /// Print every legal action at each decision, even in --steps mode.
        #[arg(long)]
        verbose: bool,
    },
    /// Benchmark random playouts and replays of the first turn.
    Bench {
        #[command(flatten)]
        setup: Setup,
        /// Number of random playouts.
        #[arg(long, default_value_t = 200)]
        playouts: u32,
        /// Maximum decisions per playout.
        #[arg(long, default_value_t = 60)]
        max_decisions: u32,
    },
    /// Let the engine play every line from a hand and list the end boards.
    Combos {
        #[command(flatten)]
        setup: Setup,
        /// Time limit in seconds.
        #[arg(long, default_value_t = 60)]
        seconds: u64,
        /// Stop after this many choices explored instead (with --threads 1 the
        /// result does not depend on the machine's load).
        #[arg(long)]
        nodes: Option<u64>,
        /// Worker threads (default: all cores).
        #[arg(long)]
        threads: Option<usize>,
        /// Boards to list.
        #[arg(long, default_value_t = 10)]
        top: usize,
        /// Print the line of each board.
        #[arg(long)]
        lines: bool,
        /// Also try activating handtraps on your own turn.
        #[arg(long)]
        use_handtraps: bool,
        /// Cards never to use (semicolon-separated names or passcodes).
        #[arg(long, default_value = "")]
        forbid: String,
        /// Cards to keep in hand (semicolon-separated names or passcodes).
        #[arg(long, default_value = "")]
        keep: String,
        /// "Card + rest": instead of one hand, many hands with this card
        /// (the time is split among them).
        #[arg(long)]
        card: Option<String>,
        /// Hands to try with `--card`.
        #[arg(long, default_value_t = 12)]
        hands: usize,
        /// Goal: cards wanted at the end of the turn (semicolon-separated).
        /// "Name@place" says where: extra, main, monster, faceup, set,
        /// fieldzone, gy, banished, hand (default: field, anywhere on it);
        /// "~Name" makes the card soft (preferred, not required).
        #[arg(long, default_value = "")]
        goal: String,
        /// The goal needs only one of its cards.
        #[arg(long)]
        goal_any: bool,
        /// Stop at the first board that reaches the goal.
        #[arg(long)]
        first: bool,
        /// Put the best boards listed to the probe turn and print what it measures.
        #[arg(long)]
        probe: bool,
        /// Write the boards listed, measured, to this JSON file (implies --probe).
        #[arg(long)]
        json: Option<PathBuf>,
        /// Write the lines of the boards listed to this JSON file (for probe-line).
        #[arg(long)]
        save_lines: Option<PathBuf>,
        /// Measure without the board breakers (and the follow-up after them).
        #[arg(long)]
        no_breakers: bool,
        /// List the boards only by what they stop at the end of the turn: the
        /// interruptions on the field, in the GY, banished and in hand, and the
        /// floodgates, with a summary by kind (implies --probe and
        /// --no-breakers; no exposure). With --staged the cut to the full
        /// measure and the variant kept still go by the usual value.
        #[arg(long)]
        interaction: bool,
        /// Rank the boards by what you can use on the opponent's turn, read
        /// from your cards' effects with no opponent playing: how many
        /// separate interruptions, with the limits and costs they share
        /// (docs/plans/linee-interazione.md). Needs no probe turn.
        #[arg(long)]
        capacity: bool,
        /// Also take the quick value of each board listed (two runs), with its time.
        #[arg(long)]
        quick_value: bool,
        /// Measure in steps, as the Combo Lab does: a cheap measure of the
        /// first boards listed, then the full one of the best of them by it,
        /// going on from the cheap one
        /// (implies --probe).
        #[arg(long)]
        staged: bool,
        /// With --staged: fields given the cheap measure.
        #[arg(long, default_value_t = STAGED_CHEAP)]
        staged_cheap: usize,
        /// With --staged: of those, the best by it given the full measure.
        #[arg(long, default_value_t = STAGED_FULL)]
        staged_full: usize,
        /// With --staged: variants of each field given the cheap measure (another
        /// hand, Graveyard or banishment), the best by it standing for the field.
        #[arg(long, default_value_t = STAGED_VARIANTS)]
        staged_variants: usize,
        /// With --staged: windows each response is tried at in the cheap measure.
        #[arg(long, default_value_t = STAGED_WINDOWS)]
        staged_windows: usize,
        /// With --staged: runs counting the uses of a response in the cheap measure.
        #[arg(long, default_value_t = STAGED_ROUNDS)]
        staged_rounds: usize,
        /// With --nodes: the best boards at 500, 1000, 2000... nodes on the
        /// way and at the budget are measured too (each line once), so with
        /// one thread more nodes keep what fewer measured.
        #[arg(long)]
        checkpoints: bool,
        /// With --checkpoints and --staged: fields of each checkpoint given the cheap measure.
        #[arg(long, default_value_t = CHECKPOINT_CHEAP)]
        checkpoint_cheap: usize,
        /// With --checkpoints: fields of each checkpoint given the full measure
        /// (with --staged, the best by the cheap one).
        #[arg(long, default_value_t = CHECKPOINT_FULL)]
        checkpoint_full: usize,
        /// Measure the best N boards again at level 2 against the 3 most
        /// played decks of the meta, and list them by it (L2-3).
        #[arg(long, default_value_t = 0)]
        meta_final: usize,
        /// The meta for --meta-final from a file (`objective_bench.py`'s
        /// meta.json: opponents with deck and weight) instead of Master Duel
        /// Meta now; the lists are <deck>.ydk in --meta-decks.
        #[arg(long)]
        meta_file: Option<PathBuf>,
        #[arg(long)]
        meta_decks: Option<PathBuf>,
        /// Several hands of the deck in one run, one per line of this file:
        /// id, seed, hand, JSON file and lines file (tab-separated, the last
        /// two may be empty). The engine is loaded once, and the card values
        /// once per seed.
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// The Monte Carlo played by the engine: hands drawn from the deck, each
    /// searched and measured, and P(best board ≥ threshold) with its 95%
    /// interval. Swaps of a copy are measured on the same draws, with the
    /// hands already searched taken from a cache; `--climb` lets the
    /// optimizer pick the swaps. Writes `summary.txt` and `hands.json` to `--out`.
    EngineMc {
        /// Deck in .ydk format (its Side Deck: cards that may enter).
        #[arg(long)]
        deck: PathBuf,
        /// Hands drawn (5 cards, going first).
        #[arg(long, default_value_t = 200)]
        hands: usize,
        /// Choices explored per hand.
        #[arg(long, default_value_t = 2000)]
        nodes: u64,
        /// Measured value a hand's best board must reach (default: the
        /// median of the deck's hands, then kept for every variant).
        #[arg(long)]
        threshold: Option<f64>,
        /// Fields of each hand given the cheap measure.
        #[arg(long, default_value_t = 10)]
        cheap: usize,
        /// Of those, the best by it given the full measure.
        #[arg(long, default_value_t = 3)]
        full: usize,
        /// Swaps to measure: "Out>In", semicolon-separated names or passcodes.
        #[arg(long, default_value = "")]
        swap: String,
        /// Let the optimizer swap copies: decks measured at most (0: none).
        #[arg(long, default_value_t = 0)]
        climb: usize,
        /// Cards that may enter besides the Side Deck (semicolon-separated).
        #[arg(long, default_value = "")]
        candidates: String,
        /// Hands searched at once (each on one thread).
        #[arg(long)]
        threads: Option<usize>,
        /// Seed of the draws, the deck order and the engine.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value = "out/engine-mc")]
        out: PathBuf,
        /// Also find the roles by playing and compare: the exact odds of a
        /// Starter, and the share of the hands with and without one that
        /// reached the threshold.
        #[arg(long)]
        roles: bool,
    },
    /// Measure lines saved by `combos --save-lines` with the probe turn,
    /// without searching (the same boards before and after a change). The
    /// lines are played with the seed saved with them.
    ProbeLine {
        #[command(flatten)]
        setup: Setup,
        /// File written by `combos --save-lines`.
        #[arg(long)]
        lines: PathBuf,
        /// Only the line at this index.
        #[arg(long)]
        index: Option<usize>,
        /// Write the measures to this JSON file (as `combos --json`).
        #[arg(long)]
        json: Option<PathBuf>,
        /// Also against this deck (.ydk): its board breakers and handtraps.
        #[arg(long)]
        vs: Option<PathBuf>,
        /// With --vs: where to stop the line when the deck's Maxx "C" or
        /// Mulcharmy makes it draw (a few seconds a line).
        #[arg(long)]
        stops: bool,
        /// With --vs: the deck's best lines (from 12 hands, read once) are
        /// played again in its turn after yours, and each of your effects is
        /// measured against their moves (level 2 of the probe turn against a
        /// deck).
        #[arg(long)]
        moves: bool,
        /// What each card of the board that interrupts holds against:
        /// removals aimed at it (destroy, banish, face-down, negate), with
        /// only its own effects to answer (a run each).
        #[arg(long)]
        protection: bool,
    },
    /// The map of the links between the cards of a deck, read from the
    /// engine: which card adds, summons, sends or gets back which.
    Links {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
        /// Search nodes for each hand.
        #[arg(long, default_value_t = 1000)]
        nodes: u64,
        /// Hands searched for each card of the Main Deck.
        #[arg(long, default_value_t = 3)]
        hands: usize,
        #[arg(long)]
        threads: Option<usize>,
        /// Write the map to this JSON file.
        #[arg(long)]
        json: Option<PathBuf>,
        /// An earlier version of the deck (.ydk): its map is read first,
        /// then the deck's reusing what was searched for it.
        #[arg(long)]
        after: Option<PathBuf>,
        /// With --after: print what changes from the earlier version
        /// instead of the whole map.
        #[arg(long, requires = "after")]
        diff: bool,
        /// Instead of the map: search this many opening hands drawn at
        /// random and write to --json what their lines bring into play
        /// (the measure the chances are checked against).
        #[arg(long, requires = "json")]
        truth: Option<usize>,
        /// Seed of the hands drawn with --truth.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Also write each hand searched for the map and what its lines
        /// bring into play to this JSON file.
        #[arg(long)]
        hands_json: Option<PathBuf>,
    },
    /// A deck's typical opening: the moves of its best lines from random
    /// hands, the most played first (the probe turn against a deck, level 2).
    RivalPlays {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
        #[arg(long, default_value_t = 12)]
        hands: usize,
        #[arg(long, default_value_t = 1500)]
        nodes: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Going second: the best board of `--vs` from random hands of 5, then
    /// random hands of 6 of `--deck` play their turn against it; each hand
    /// is worth the share of the board's value left after its turn, less
    /// the share of the opponent's Life Points it took with its Battle Phase
    /// (a measure apart from the first turn's).
    BreakBoard {
        /// Your deck, in .ydk format.
        #[arg(long)]
        deck: PathBuf,
        /// The opponent's deck, in .ydk format.
        #[arg(long)]
        vs: PathBuf,
        /// Your hands of 6.
        #[arg(long, default_value_t = 10)]
        hands: usize,
        /// The opponent's hands of 5 whose best board is kept.
        #[arg(long, default_value_t = 12)]
        rival_hands: usize,
        #[arg(long, default_value_t = 4000)]
        nodes: u64,
        #[arg(long, default_value_t = 4000)]
        rival_nodes: u64,
        /// Your boards measured per hand, the best by the search's estimate.
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// The opponent's board does not answer your turn (as before it did).
        #[arg(long)]
        passive: bool,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// The handtraps of a deck's Main and Side against the meta: the share of
    /// the best lines of the decks played the most that each stops, weighed
    /// by how much each deck is played, and the move it stops them at.
    TechTable {
        /// Deck in .ydk format (its Side counts).
        #[arg(long)]
        deck: PathBuf,
        /// Decks of the meta played the most to measure against.
        #[arg(long, default_value_t = TECH_DECKS)]
        decks: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// The meta from a file (`objective_bench.py`'s meta.json: opponents
        /// with deck and weight) instead of Master Duel Meta now; the lists
        /// are <deck>.ydk in --meta-decks.
        #[arg(long)]
        meta_file: Option<PathBuf>,
        #[arg(long)]
        meta_decks: Option<PathBuf>,
        /// Also write the table to this JSON file.
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// The handtraps of a deck, read from the engine: cards it can use from
    /// the hand during the opponent's turn.
    HandTraps {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
    },
    /// How often each kind of play the probe turn's kit tests happens in
    /// lines found by the search (files of `combos --save-lines`), and each
    /// kind of activation, touching the Deck or not: what the weights of the
    /// probes are calibrated on.
    KitMix {
        /// Files written by `combos --save-lines`.
        lines: Vec<PathBuf>,
        /// Lines of each file read, the best boards first.
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// Seed the lines were found with.
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// What each card of a deck is worth alone on the field, measured by the
    /// probe turn (what the search steers by).
    CardValues {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
    },
    /// The anatomy of card effects read from their scripts: for each effect
    /// its kind, event, zone, once-per-turn limit, categories and whether it
    /// has a cost, a condition and targets (`docs/plans/anatomia-effetti.md`).
    Anatomy {
        /// Cards by name or passcode.
        #[arg(long)]
        card: Vec<String>,
        /// Every card of a deck in .ydk format.
        #[arg(long)]
        deck: Option<PathBuf>,
        /// Every card of the database (alternate artworks left out).
        #[arg(long)]
        all: bool,
        /// With --deck: random turns played from the deck, to count the
        /// activations linked to their effect by their description.
        #[arg(long, default_value_t = 0)]
        link: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Write the anatomy as JSON to this file.
        #[arg(long)]
        json: Option<PathBuf>,
        /// Write `summary.txt` (coverage) to this folder.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// The role of each card of a deck (starter, extender, handtrap,
    /// board breaker, brick), found by playing it.
    Roles {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
    },
    /// What the search does with each action of a deck's cards, over one
    /// hand per card of the Main Deck (the card and four others): how often
    /// each summon, Set and effect is offered, left out (and why), chosen
    /// and really activated.
    Coverage {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
        /// Choices explored per hand.
        #[arg(long, default_value_t = 2000)]
        nodes: u64,
        /// Hands searched at once (each on one thread).
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Print every mechanic, not only the ones never chosen or activated.
        #[arg(long)]
        all: bool,
    },
    /// Random duels between random decks; fails on unanswerable or rejected decisions.
    Sweep {
        #[arg(long, default_value_t = 200)]
        duels: u32,
        /// Decisions per duel (both players).
        #[arg(long, default_value_t = 400)]
        max_decisions: u32,
        #[arg(long, default_value_t = 4)]
        max_turns: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Worker threads (default: all cores).
        #[arg(long)]
        threads: Option<usize>,
        /// Also list the script errors logged by the engine.
        #[arg(long)]
        script_errors: bool,
    },
    /// The census of the synergy plan from the engine's data, and the
    /// reading deck tried on a few cards: a duel holding the whole pool,
    /// where each card's effects offer what they accept
    /// (`docs/plans/sinergie-archetipi.md` §2-§3). Writes `summary.txt`
    /// and `readings.tsv` to `--out`.
    SynergyCensus {
        /// Cards to read, by name or passcode (default: 30 test cards).
        #[arg(long)]
        card: Vec<String>,
        /// Folder of .ydk lists (sub-folders included) searched for a real
        /// deck holding each card; a deck is built when none does.
        #[arg(long, default_value = "tests/fixtures/banco-motore/v1/decks")]
        decks: PathBuf,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value = "out/synergy-census")]
        out: PathBuf,
    },
    /// Read what every card of the pool accepts (from the hand, the field
    /// and the GY) and the materials of the Extra Deck into
    /// `data/synergy/`, going on from what is already there
    /// (`docs/plans/sinergie-archetipi.md` §3.3). Writes `summary.txt`.
    SynergyRead {
        /// Cards to read, by name or passcode (default: the whole pool).
        #[arg(long)]
        card: Vec<String>,
        /// Cards of these archetypes, by name or set code.
        #[arg(long)]
        archetype: Vec<String>,
        /// Read only this many cards with effects that move cards, spread
        /// over the pool.
        #[arg(long)]
        sample: Option<usize>,
        /// Places to read from: hand, field, GY (default: all three).
        #[arg(long, value_delimiter = ',')]
        places: Vec<String>,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value = "out/synergy-data")]
        out: PathBuf,
        /// Forget what the store has and read again.
        #[arg(long)]
        fresh: bool,
    },
    /// What a card's effects accept, from the readings in `data/synergy/`
    /// (read now if missing), with the rules read from them.
    Accepts {
        #[arg(long)]
        card: String,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// The profile of an archetype from the readings in `data/synergy/`.
    Archetype {
        /// Name ("Blue-Eyes") or set code ("0xdd").
        #[arg(long)]
        profile: String,
    },
    /// Write decks of random archetypes of the card pool as .ydk, for the
    /// robustness sweep of the search and the probe turn
    /// (scripts/robustness.py).
    RandomDecks {
        #[arg(long, default_value_t = 100)]
        count: usize,
        /// Main Deck cards an archetype needs.
        #[arg(long, default_value_t = 10)]
        min_cards: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(clap::Args)]
struct Setup {
    /// Deck in .ydk format.
    #[arg(long)]
    deck: PathBuf,
    /// Starting hand: semicolon-separated card names or passcodes.
    #[arg(long, default_value = "")]
    hand: String,
    /// Seed for the deck order and the engine's random number generator.
    #[arg(long, default_value_t = 1)]
    seed: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::FetchData => fetch_data(&cli.data),
        Cmd::Info => info(&cli.data),
        Cmd::UpdateCards { force } => update_cards(&cli.data, force),
        Cmd::RecordFixtures { deck, out, query, role } => record_fixtures(&cli.data, &deck, &out, &query, &role),
        Cmd::Search { query, limit } => search(&cli.data, &query, limit),
        Cmd::Play { setup, steps, verbose } => play(&cli.data, &setup, steps.as_deref(), verbose),
        Cmd::Bench { setup, playouts, max_decisions } => bench(&cli.data, &setup, playouts, max_decisions),
        Cmd::Combos {
            setup,
            seconds,
            nodes,
            threads,
            top,
            lines,
            use_handtraps,
            forbid,
            keep,
            card,
            hands,
            goal,
            goal_any,
            first,
            probe,
            json,
            save_lines,
            no_breakers,
            interaction,
            capacity,
            quick_value,
            staged,
            staged_cheap,
            staged_full,
            staged_variants,
            staged_windows,
            staged_rounds,
            checkpoints,
            checkpoint_cheap,
            checkpoint_full,
            meta_final,
            meta_file,
            meta_decks,
            cases,
        } => combos(
            &cli.data,
            &setup,
            CombosOptions {
                seconds,
                nodes,
                threads,
                top,
                show_lines: lines,
                use_handtraps,
                forbid,
                keep,
                card,
                hands,
                goal,
                goal_any,
                first,
                probe: probe || (json.is_some() && !capacity) || staged || interaction,
                json,
                save_lines,
                no_breakers: no_breakers || interaction,
                interaction,
                capacity,
                quick_value,
                staged: staged.then_some(Staged {
                    cheap: staged_cheap,
                    full: staged_full,
                    variants: staged_variants.max(1),
                    measure: ygo_solver::probe::Cheap { windows: staged_windows, use_rounds: staged_rounds },
                }),
                checkpoints,
                checkpoint_cheap,
                checkpoint_full,
                meta: (meta_final > 0).then_some(MetaFinal { boards: meta_final, file: meta_file, decks: meta_decks }),
            },
            cases.as_deref(),
        ),
        Cmd::ProbeLine { setup, lines, index, json, vs, stops, moves, protection } => {
            probe_line(&cli.data, &setup, &lines, index, json.as_deref(), vs.as_deref(), stops, moves, protection)
        }
        Cmd::EngineMc { deck, hands, nodes, threshold, cheap, full, swap, climb, candidates, threads, seed, out, roles } => {
            engine_mc::engine_mc(
                &cli.data,
                engine_mc::Options {
                    deck,
                    hands,
                    nodes,
                    threshold,
                    cheap,
                    full,
                    swaps: swap,
                    climb,
                    candidates,
                    threads,
                    seed,
                    out,
                    roles,
                },
            )
        }
        Cmd::TechTable { deck, decks, seed, meta_file, meta_decks, json } => {
            tech_table(&cli.data, &deck, decks, seed, MetaFinal { boards: 0, file: meta_file, decks: meta_decks }, json.as_deref())
        }
        Cmd::HandTraps { deck } => hand_traps(&cli.data, &deck),
        Cmd::BreakBoard { deck, vs, hands, rival_hands, nodes, rival_nodes, top, passive, seed, json } => {
            let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
            let config = ygo_solver::BreakConfig { seed, nodes, threads, top, answers: !passive };
            break_board(&cli.data, &deck, &vs, hands, rival_hands, rival_nodes, &config, json.as_deref())
        }
        Cmd::RivalPlays { deck, hands, nodes, seed } => rival_plays(&cli.data, &deck, hands, nodes, seed),
        Cmd::Links { deck, nodes, hands, threads, json, after, diff, truth, seed, hands_json } => {
            let out = LinksOut { json: json.as_deref(), hands_json: hands_json.as_deref() };
            match truth {
                Some(count) => links_truth(&cli.data, &deck, nodes, count, seed, threads, &json.unwrap_or_default()),
                None => links(&cli.data, &deck, nodes, hands, threads, out, after.as_deref(), diff),
            }
        }
        Cmd::CardValues { deck } => card_values(&cli.data, &deck),
        Cmd::KitMix { lines, top, seed } => kit_mix(&cli.data, &lines, top, seed),
        Cmd::Anatomy { card, deck, all, link, seed, json, out } => {
            anatomy(&cli.data, &card, deck.as_deref(), all, link, seed, json.as_deref(), out.as_deref())
        }
        Cmd::Roles { deck } => roles(&cli.data, &deck),
        Cmd::Coverage { deck, nodes, threads, seed, all } => coverage(&cli.data, &deck, nodes, threads, seed, all),
        Cmd::Sweep { duels, max_decisions, max_turns, seed, threads, script_errors } => {
            let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
            sweep(&cli.data, &SweepConfig { duels, max_decisions, max_turns, seed, threads }, script_errors)
        }
        Cmd::RandomDecks { count, min_cards, seed, out } => random_decks(&cli.data, count, min_cards, seed, &out),
        Cmd::SynergyCensus { card, decks, threads, seed, out } => synergy_census(&cli.data, &card, &decks, threads, seed, &out),
        Cmd::SynergyRead { card, archetype, sample, places, threads, seed, out, fresh } => synergy::read(
            &cli.data,
            synergy::ReadArgs { cards: &card, archetypes: &archetype, sample, places: &places, threads, seed, out: &out, fresh },
        ),
        Cmd::Accepts { card, seed } => synergy::accepts(&cli.data, &card, seed),
        Cmd::Archetype { profile } => synergy::archetype(&cli.data, &profile),
    }
}

fn random_decks(dir: &Path, count: usize, min_cards: usize, seed: u64, out: &Path) -> Result<()> {
    let engine = load_engine(dir)?;
    let pool = ygo_engine::sweep::CardPool::new(&engine);
    std::fs::create_dir_all(out)?;
    let decks = pool.archetype_decks(count, min_cards, seed);
    for (k, (setcode, deck)) in decks.iter().enumerate() {
        // Named by the archetype's set code and its first card, for reading.
        let first = deck.main.first().map(|&c| engine.cards().name(c)).unwrap_or_default();
        let slug: String = first.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let name = format!("{k:03}-{setcode:03x}-{}", slug.trim_matches('_'));
        let text = format!(
            "#main\n{}\n#extra\n{}\n!side\n",
            deck.main.iter().map(u32::to_string).collect::<Vec<_>>().join("\n"),
            deck.extra.iter().map(u32::to_string).collect::<Vec<_>>().join("\n")
        );
        std::fs::write(out.join(format!("{name}.ydk")), text)?;
    }
    println!("{} decks of {} archetypes with {min_cards}+ Main Deck cards in {}", decks.len(), pool.archetypes(), out.display());
    Ok(())
}

/// The cards `synergy-census` reads by default: searches and summons from
/// the Deck by Attribute, Type, Level or kind of card, by archetype or by
/// name, Small World, and the Extra Deck (Instant Fusion, Gale Dogra).
const SYNERGY_CARDS: [&str; 30] = [
    "Reinforcement of the Army",
    "Fossil Dig",
    "Bonfire",
    "Rescue Cat",
    "Tour Guide From the Underworld",
    "Sage with Eyes of Blue",
    "Armageddon Knight",
    "Summoner Monk",
    "Unexpected Dai",
    "One for One",
    "Lonefire Blossom",
    "Dragon Shrine",
    "Foolish Burial",
    "Foolish Burial Goods",
    "Gold Sarcophagus",
    "Terraforming",
    "Small World",
    "Instant Fusion",
    "Gale Dogra",
    "Fiendsmith's Tract",
    "Spellbook of Secrets",
    "E - Emergency Call",
    "Elemental HERO Stratos",
    "Sky Striker Mobilize - Engage!",
    "Cyber Dragon Core",
    "Speedroid Terrortop",
    "Branded Opening",
    "Quick Launch",
    "Magician's Rod",
    "Charge of the Light Brigade",
];

/// Cards of a deck built for a card no list holds: some of those its
/// reading offers, and the rest drawn from the pool.
const BUILT_OFFERED: usize = 15;
const BUILT_OFFERED_EXTRA: usize = 8;

fn synergy_census(dir: &Path, cards: &[String], decks: &Path, threads: Option<usize>, seed: u64, out: &Path) -> Result<()> {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use ygo_solver::synergy::{self, OfferShape, Pool, ReadBudget, Reading};

    let engine = load_engine(dir)?;
    let db = engine.cards();
    let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    std::fs::create_dir_all(out)?;
    let started = Instant::now();
    let pool = Pool::new(&engine);
    let census = synergy::census(&engine, &pool);
    let census_time = started.elapsed().as_secs_f64();
    let names = synergy::archetype_names(&engine);
    let archetype = |code: u16| names.get(&code).cloned().unwrap_or_else(|| format!("0x{code:x}"));

    let wanted: Vec<String> = if cards.is_empty() { SYNERGY_CARDS.iter().map(|s| s.to_string()).collect() } else { cards.to_vec() };
    let mut codes = Vec::new();
    for name in &wanted {
        let code = match name.parse::<u32>() {
            Ok(code) => code,
            Err(_) => db.find_by_name(name).with_context(|| format!("no card named {name}"))?.code,
        };
        codes.push(db.canonical_code(code));
    }

    // The lists a card is looked for in, by file name.
    let mut lists: Vec<(String, DeckList)> = Vec::new();
    let mut folders = vec![decks.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for path in entries.filter_map(|e| e.ok().map(|e| e.path())) {
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|x| x == "ydk") {
                let list = DeckList::from_ydk(&std::fs::read_to_string(&path)?)?;
                lists.push((path.file_stem().unwrap_or_default().to_string_lossy().into_owned(), list));
            }
        }
    }
    lists.sort_by(|a, b| a.0.cmp(&b.0));

    // Each card read in the whole pool with nothing preferred, the reading
    // a later phase keeps; then in a real deck holding it, and in the pool
    // again preferring that deck's cards, to compare the two.
    let reading_deck = pool.deck();
    let budget = ReadBudget::default();
    let parallel = |jobs: &[(u32, DeckList, HashSet<u32>)]| -> Vec<Reading> {
        let next = AtomicUsize::new(0);
        let results: std::sync::Mutex<Vec<Option<Reading>>> = std::sync::Mutex::new(vec![None; jobs.len()]);
        std::thread::scope(|s| {
            for _ in 0..threads.clamp(1, jobs.len().max(1)) {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((code, deck, prefer)) = jobs.get(i) else { break };
                    let reading = synergy::read_card(&engine, deck, *code, prefer, budget, seed);
                    if let Ok(mut all) = results.lock() {
                        all[i] = Some(reading);
                    }
                });
            }
        });
        results.into_inner().unwrap_or_default().into_iter().map(Option::unwrap_or_default).collect()
    };
    let read_started = Instant::now();
    let plain = parallel(&codes.iter().map(|&c| (c, reading_deck.clone(), HashSet::new())).collect::<Vec<_>>());
    let read_time = read_started.elapsed().as_secs_f64();

    let mut rng = seed ^ 0x5e1e_c7ed;
    let mut draw = |n: usize| {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng % n.max(1) as u64) as usize
    };
    let mut real: Vec<(String, DeckList)> = Vec::new();
    for (&code, reading) in codes.iter().zip(&plain) {
        if let Some((name, list)) = lists.iter().find(|(_, l)| l.main.iter().any(|&c| db.canonical_code(c) == code)) {
            real.push((name.clone(), list.clone()));
            continue;
        }
        // Evenly spread over the cards offered, the first one included.
        let offered = |extra: bool| -> Vec<u32> {
            let mut all: Vec<u32> = reading
                .offers
                .iter()
                .flat_map(|o| o.cards.iter().copied())
                .filter(|&c| db.get(c).is_some_and(|d| d.is_extra_deck_monster() == extra) && c != code)
                .collect();
            all.sort_unstable();
            all.dedup();
            all
        };
        let spread = |all: Vec<u32>, n: usize| -> Vec<u32> {
            if all.len() <= n {
                return all;
            }
            (0..n).map(|k| all[k * all.len() / n]).collect()
        };
        let mut deck = DeckList { main: vec![code], ..DeckList::default() };
        deck.main.extend(spread(offered(false), BUILT_OFFERED));
        while deck.main.len() < 40 {
            deck.main.push(pool.main[draw(pool.main.len())]);
        }
        deck.extra = spread(offered(true), BUILT_OFFERED_EXTRA);
        while deck.extra.len() < 15 {
            deck.extra.push(pool.extra[draw(pool.extra.len())]);
        }
        real.push(("built".to_owned(), deck));
    }
    let deck_cards: Vec<HashSet<u32>> =
        real.iter().map(|(_, d)| d.main.iter().chain(&d.extra).map(|&c| db.canonical_code(c)).collect()).collect();
    let in_deck =
        parallel(&codes.iter().zip(&real).zip(&deck_cards).map(|((&c, (_, d)), k)| (c, d.clone(), k.clone())).collect::<Vec<_>>());
    let in_pool = parallel(&codes.iter().zip(&deck_cards).map(|(&c, k)| (c, reading_deck.clone(), k.clone())).collect::<Vec<_>>());

    let mut tsv = String::from("card\tcode\tdeck\tpurpose\thint\tlocation\toffered\tin_deck\tagrees\tshape\n");
    let mut agreeing = 0;
    let mut disagreements = Vec::new();
    let mut errors = Vec::new();
    let (mut offers, mut generic, mut with_offers) = (0, 0, 0);
    for (k, &code) in codes.iter().enumerate() {
        let name = db.name(code);
        let agreement = synergy::compare(&in_pool[k], &in_deck[k], &deck_cards[k]);
        if agreement.agrees() {
            agreeing += 1;
        } else {
            let names = |codes: &[u32]| codes.iter().take(4).map(|&c| db.name(c)).collect::<Vec<_>>().join(", ");
            disagreements.push(format!(
                "{name} in {}: {} of {} offers the same; only in the deck: {}; only in the pool: {}",
                real[k].0,
                agreement.same,
                agreement.offers,
                names(&agreement.only_deck),
                names(&agreement.only_pool)
            ));
        }
        for (r, which) in [(&plain[k], "pool"), (&in_deck[k], "deck")] {
            if let Some(e) = &r.error {
                errors.push(format!("{name} ({which}): {e}"));
            }
        }
        with_offers += usize::from(!plain[k].offers.is_empty());
        let mut parts = Vec::new();
        for o in &plain[k].offers {
            let shape = OfferShape::of(&engine, o);
            offers += 1;
            // Generic: the cards span several archetypes, or have none.
            let is_generic = shape.archetypes >= 3 || shape.plain > 0;
            generic += usize::from(is_generic);
            let in_real = o.cards.iter().filter(|c| deck_cards[k].contains(c)).count();
            let agrees = in_deck[k].offers.iter().find(|d| d.hint == o.hint && d.location == o.location).map_or(in_real == 0, |d| {
                let pool_cards: HashSet<u32> = o.cards.iter().copied().filter(|c| deck_cards[k].contains(c)).collect();
                d.cards.iter().copied().collect::<HashSet<u32>>() == pool_cards
            });
            tsv.push_str(&format!(
                "{name}\t{code}\t{}\t{}\t{}\t{}\t{}\t{in_real}\t{agrees}\t{}\n",
                real[k].0,
                o.purpose(),
                o.hint,
                ygo_engine::consts::location::name(o.location),
                o.cards.len(),
                shape.describe()
            ));
            parts.push(format!(
                "{} from {} {} ({})",
                o.purpose(),
                ygo_engine::consts::location::name(o.location),
                o.cards.len(),
                shape.describe()
            ));
        }
        println!(
            "{name}: {} turns, {:.2} s{}{}",
            plain[k].walks,
            plain[k].time.as_secs_f64(),
            if parts.is_empty() { ", nothing offered".to_owned() } else { format!("; {}", parts.join("; ")) },
            if agreement.agrees() { String::new() } else { "; DISAGREES".to_owned() }
        );
    }
    std::fs::write(out.join("readings.tsv"), tsv)?;

    let mut times: Vec<f64> = plain.iter().map(|r| r.time.as_secs_f64()).collect();
    times.sort_by(f64::total_cmp);
    let mean = times.iter().sum::<f64>() / times.len().max(1) as f64;
    let setup = plain.iter().map(|r| r.setup.as_secs_f64()).sum::<f64>() / plain.iter().map(|r| r.walks).sum::<usize>().max(1) as f64;
    let median = times.get(times.len() / 2).copied().unwrap_or(0.0);
    let peak = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmHWM")).and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok()))
        .map_or_else(|| "not measured".to_owned(), |kb| format!("{:.0} MB", kb / 1024.0));
    let built = real.iter().filter(|(n, _)| n == "built").count();
    let top: Vec<String> = census.top_pairs.iter().map(|&(a, b, n)| format!("{}–{} {n}", archetype(a), archetype(b))).collect();
    let at_least: Vec<String> = census.at_least.iter().map(|(n, k)| format!("{n}+: {k}")).collect();
    let mut lines = vec![
        format!("pool: {} cards with a script ({} Main Deck, {} Extra Deck), alternate artworks and tokens left out", pool.len(), census.main, census.extra),
        format!("with an archetype: {}; in 2+ base archetypes: {}", census.with_archetype, census.in_two),
        format!("base archetypes: {} ({}); pairs sharing a card: {}", census.archetypes, at_least.join(", "), census.pairs),
        format!("most shared: {}", top.join(", ")),
        format!(
            "cards with an effect that searches, adds to the hand, Special Summons or sends to the GY: {}; Extra Deck monsters with a summon procedure: {} of {}; scripts not set up: {} ({census_time:.1} s)",
            census.movers, census.extra_with_procedure, census.extra, census.errors
        ),
        format!(
            "reading deck: {} Main + {} Extra; {} cards read on {threads} threads in {read_time:.1} s: per card mean {mean:.2} s, median {median:.2} s, max {:.2} s; a duel made in {setup:.2} s; peak memory {peak}",
            pool.main.len(),
            pool.extra.len(),
            codes.len(),
            times.last().copied().unwrap_or(0.0)
        ),
        format!(
            "whole Main Deck pool at this pace: about {:.0} min on {threads} threads",
            mean * pool.main.len() as f64 / threads as f64 / 60.0
        ),
        format!("cards with offers: {with_offers} of {}; offers: {offers}, generic (3+ archetypes or cards of none): {generic}", codes.len()),
        format!(
            "same offers as in a real deck: {agreeing} of {} cards ({} in a list of {}, {built} in a deck built from the pool)",
            codes.len(),
            codes.len() - built,
            decks.display()
        ),
    ];
    lines.extend(disagreements.iter().map(|d| format!("  disagrees: {d}")));
    lines.push(format!("errors: {}", errors.len()));
    lines.extend(errors.iter().map(|e| format!("  {e}")));
    let summary = lines.join("\n") + "\n";
    print!("{summary}");
    std::fs::write(out.join("summary.txt"), summary)?;
    Ok(())
}

#[derive(Clone)]
struct CombosOptions {
    seconds: u64,
    nodes: Option<u64>,
    threads: Option<usize>,
    top: usize,
    show_lines: bool,
    use_handtraps: bool,
    forbid: String,
    keep: String,
    card: Option<String>,
    hands: usize,
    goal: String,
    goal_any: bool,
    first: bool,
    probe: bool,
    json: Option<PathBuf>,
    save_lines: Option<PathBuf>,
    no_breakers: bool,
    interaction: bool,
    capacity: bool,
    quick_value: bool,
    staged: Option<Staged>,
    checkpoints: bool,
    checkpoint_cheap: usize,
    checkpoint_full: usize,
    meta: Option<MetaFinal>,
}

/// `combos --meta-final`: the best `boards` measured again against the
/// meta, read from `file` (lists in `decks`) or from Master Duel Meta.
#[derive(Clone)]
struct MetaFinal {
    boards: usize,
    file: Option<PathBuf>,
    decks: Option<PathBuf>,
}

/// Decks of the meta the last step measures the boards against.
const META_DECKS: usize = 3;

/// The meta for `combos --meta-final`.
fn load_meta(dir: &Path, meta: &MetaFinal) -> Result<Vec<ygo_solver::MetaDeck>> {
    if let Some(file) = &meta.file {
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
        let decks = meta.decks.as_ref().context("--meta-file needs --meta-decks")?;
        let mut out = Vec::new();
        for o in doc["opponents"].as_array().context("no opponents in the meta file")? {
            let deck = o["deck"].as_str().context("an opponent with no deck")?;
            let path = decks.join(format!("{deck}.ydk"));
            let list = DeckList::from_ydk(&std::fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?)?;
            out.push(ygo_solver::MetaDeck { name: deck.to_owned(), weight: o["weight"].as_f64().unwrap_or(0.0), list });
        }
        return Ok(out);
    }
    let store = CardStore::open(&dir.join("cards.db"))?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (types, decks) = runtime.block_on(async { ygo_data::meta::fetch(&ygo_data::sync::http_client()?).await })?;
    Ok(ygo_data::meta::meta_lists(&types, &decks, &store)?
        .into_iter()
        .map(|m| ygo_solver::MetaDeck { name: m.name, weight: m.weight, list: DeckList { main: m.main, extra: m.extra, side: Vec::new() } })
        .collect())
}

/// The first checkpoint of `combos --checkpoints`, doubled up to the budget.
const CHECKPOINT_FIRST: u64 = 500;
/// Best boards the search copies at a checkpoint, for its first fields.
const CHECKPOINT_BOARDS: usize = 1000;
/// Fields of a checkpoint given the cheap measure, and of those the best
/// by it given the full one (`--staged`).
const CHECKPOINT_CHEAP: usize = 30;
const CHECKPOINT_FULL: usize = 15;

/// The checkpoints of `combos --checkpoints` for a budget of `nodes`:
/// 500·2^k nodes below it, then `nodes`. Absolute, not shares of the
/// budget: a search with 4000 nodes passes all those of one with 2000.
fn checkpoint_stops(nodes: u64) -> Vec<u64> {
    let mut stops: Vec<u64> = std::iter::successors(Some(CHECKPOINT_FIRST), |n| n.checked_mul(2)).take_while(|&n| n < nodes).collect();
    stops.push(nodes);
    stops
}

/// The variants of a field in the search's order, the first of each hand,
/// Graveyard and banishment: those differ in what the probe turn can still
/// use, while other zones alone are the zone phase's work.
fn variants(group: &[ygo_solver::Board], top: usize) -> Vec<&ygo_solver::Board> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for board in group {
        let p = &board.state.players[0];
        let mut rest = (p.hand.clone(), p.grave.clone(), p.banished.clone());
        rest.0.sort_unstable();
        rest.1.sort_unstable();
        rest.2.sort_unstable();
        if out.len() < top && seen.insert(rest) {
            out.push(board);
        }
    }
    out
}

/// `combos --staged`: the cheap measure of the first `cheap` fields, then
/// the full one of the best `full` by it. Each field stands for its first
/// `variants` variants with another hand, Graveyard or banishment, and the
/// best of them by the cheap measure stands for it.
#[derive(Clone, Copy)]
struct Staged {
    cheap: usize,
    full: usize,
    variants: usize,
    measure: ygo_solver::probe::Cheap,
}

fn combos(dir: &Path, setup: &Setup, options: CombosOptions, cases: Option<&Path>) -> Result<()> {
    let Some(cases) = cases else {
        let (engine, deck, hand) = start(dir, setup)?;
        return combos_with(dir, &engine, setup, deck, hand, options);
    };
    // The first case's time also counts the engine's loading.
    let mut started = std::time::Instant::now();
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(cases).with_context(|| format!("cannot read {}", cases.display()))?;
    let mut failed = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [id, seed, hand, json, save_lines] = fields[..] else { bail!("a case needs 5 tab-separated fields: {line}") };
        let path = |p: &str| (!p.is_empty()).then(|| PathBuf::from(p));
        let setup =
            Setup { deck: setup.deck.clone(), hand: hand.to_owned(), seed: seed.parse().with_context(|| format!("bad seed in {line}"))? };
        let json = path(json);
        // A JSON file implies the probe turn, as with --json.
        let options = CombosOptions {
            probe: options.probe || (json.is_some() && !options.capacity),
            json,
            save_lines: path(save_lines),
            ..options.clone()
        };
        // Each case's output between its markers, with its own time (the
        // first one also pays for the engine).
        println!("=== case {id}");
        let done = prepare(&engine, &setup).and_then(|(deck, hand)| combos_with(dir, &engine, &setup, deck, hand, options));
        match done {
            Ok(()) => println!("=== case {id} done in {:.2} s", started.elapsed().as_secs_f64()),
            Err(e) => {
                failed += 1;
                println!("=== case {id} failed in {:.2} s: {e:#}", started.elapsed().as_secs_f64());
            }
        }
        started = std::time::Instant::now();
    }
    if failed > 0 {
        bail!("{failed} cases failed");
    }
    Ok(())
}

fn combos_with(dir: &Path, engine: &Engine, setup: &Setup, deck: DeckList, hand: Vec<u32>, options: CombosOptions) -> Result<()> {
    let CombosOptions {
        seconds,
        nodes,
        threads,
        top,
        show_lines,
        use_handtraps,
        forbid,
        keep,
        card,
        hands,
        goal,
        goal_any,
        first,
        probe,
        json,
        save_lines,
        no_breakers,
        interaction,
        capacity,
        quick_value,
        staged,
        checkpoints,
        checkpoint_cheap,
        checkpoint_full,
        meta,
    } = options;
    let mut config = ygo_solver::SearchConfig::new(deck, hand);
    config.seed = setup.seed;
    config.time_limit = std::time::Duration::from_secs(seconds);
    if let Some(nodes) = nodes {
        config.max_nodes = nodes;
    }
    if let Some(t) = threads {
        config.threads = t;
    }
    // The search plays against the probe turn's opponent: its lines play the same there.
    let kit = ygo_solver::Kit::standard(engine);
    config.opponent = Some(kit.opponent().clone());
    // Handtraps, read from the engine: kept in hand they are worth more.
    config.hand_traps = ygo_solver::hand_traps(engine, &config.deck, config.seed, &kit);
    // What each card stops alone on the field: the search steers by it,
    // and learns on the way what cards stop together.
    config.values =
        std::sync::Arc::new(ygo_solver::card_values(engine, &config.deck, config.seed, &kit, &config.hand_traps, config.threads));
    config.learn = Some(std::sync::Arc::new(kit.clone()));
    if !use_handtraps {
        config.policy.passive = known_hand_traps(engine);
    }
    config.policy.forbid = resolve_hand(engine, &forbid)?;
    config.keep_in_hand = resolve_hand(engine, &keep)?;
    // "Name@place" asks for a place, "~Name" for a soft card.
    let mut cards = Vec::new();
    for item in goal.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let (item, hard) = match item.strip_prefix('~') {
            Some(rest) => (rest.trim(), false),
            None => (item, true),
        };
        let (name, zone) = match item.rsplit_once('@') {
            Some((name, place)) => (name, goal_zone(place).with_context(|| format!("unknown goal place: {place}"))?),
            None => (item, ygo_solver::GoalZone::Any),
        };
        let code = *resolve_hand(engine, name)?.first().context("empty goal card")?;
        cards.push(ygo_solver::GoalCard { code, zone, hard });
    }
    config.goal = ygo_solver::Goal { cards, all: !goal_any, first_only: first };
    let goal_status = |reached: Option<bool>| match reached {
        Some(true) => "reached",
        Some(false) => "impossible: no line reaches it",
        None => "not reached in the time given",
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    if let Some(card) = card {
        let card = *resolve_hand(engine, &card)?.first().context("--card names no card")?;
        // The deck's own copy: a name finds the original artwork, the deck
        // may hold another.
        let db = engine.cards();
        let card = config.deck.main.iter().copied().find(|&c| db.canonical_code(c) == db.canonical_code(card)).unwrap_or(card);
        let hands = hands.max(1);
        let what = ygo_solver::CardSearch { card, hands, per_hand: config.time_limit / hands as u32, keep: top.max(1) };
        let results = ygo_solver::search_card(engine, &config, &what, &cancel, |_, _, _| {});
        if results.is_empty() {
            bail!("{} is not in the Main Deck", engine.cards().name(card));
        }
        let db = engine.cards();
        let mut mean = 0.0;
        for (i, r) in results.iter().enumerate() {
            let best = r.boards.first().map_or(0.0, |b| b.quick_score);
            mean += r.weight * best;
            let names: Vec<String> = r.hand.iter().map(|&c| db.name(c)).collect();
            let goal = if config.goal.is_set() { format!(" · goal {}", goal_status(r.goal)) } else { String::new() };
            println!("hand {:>2} ({:>4.1}%) best {best:.1} · {} nodes{goal} · {}", i + 1, r.weight * 100.0, r.nodes, names.join(", "));
        }
        if config.goal.is_set() {
            let reached: f64 = results.iter().filter(|r| r.goal == Some(true)).map(|r| r.weight).sum();
            println!("goal reached from {:.1}% of the hands (weighted)", 100.0 * reached / total_weight(&results));
        }
        println!("weighted mean of the best boards: {:.2}", mean / total_weight(&results));
        return Ok(());
    }
    if config.hand.is_empty() {
        bail!("give --hand, or --card for \"card + rest\"");
    }
    // With --checkpoints the search copies its best boards at 500·2^k nodes
    // on the way, and the fields of every copy are measured too, each line
    // once (docs/plans/monotonia-budget.md). With one thread a search with
    // more nodes takes the same copies first: the best board measured gets
    // worse with more nodes only through the boards of the phases after
    // the search, which a longer search may not have.
    let stops = match nodes {
        Some(n) if checkpoints => checkpoint_stops(n),
        _ => Vec::new(),
    };
    config.checkpoint_top = if stops.is_empty() { 0 } else { CHECKPOINT_BOARDS };
    config.checkpoints = stops.clone();
    let mut last = 0;
    let result = ygo_solver::search(engine, &config, &cancel, |p| {
        if p.elapsed.as_secs() >= last + 5 {
            last = p.elapsed.as_secs();
            eprintln!(
                "{:>4} s  {} nodes, {} replays, {} duplicates, {} queued, {} boards (best {:.1})",
                p.elapsed.as_secs(),
                p.nodes,
                p.replays,
                p.duplicates,
                p.queued,
                p.boards,
                p.best_quick_score
            );
        }
    });
    if config.goal.is_set() {
        println!("goal: {}", goal_status(result.goal_reached(&config.goal)));
    }
    let p = &result.progress;
    println!(
        "{:?} after {:.1} s: {} nodes ({:.0}/s), {} replays, {} duplicates, {} boards, {} interrupted lines",
        result.stop,
        p.elapsed.as_secs_f64(),
        p.nodes,
        p.nodes as f64 / p.elapsed.as_secs_f64().max(1e-9),
        p.replays,
        p.duplicates,
        p.boards,
        result.interrupted
    );
    let db = engine.cards();
    let mut prober = ygo_solver::Prober::new(engine, &config.deck, &config.hand, config.seed, &kit);
    // Measured first: the boards are then printed best measured first (the
    // search's estimate next to it), as the Combo Lab ranks them; the JSON
    // and the saved lines keep the search's order, with the printed rank.
    struct Listed {
        board: ygo_solver::Board,
        variants: usize,
        measured: Option<Result<ygo_solver::Measure, ygo_engine::Error>>,
        took: std::time::Duration,
        exposure: Option<ygo_solver::Exposure>,
        loss: Option<(ygo_solver::probe::ExpectedLoss, std::time::Duration)>,
        /// The quick value and its time (`--quick-value`).
        quick: Option<(f64, std::time::Duration)>,
        /// `measured` is the cheap measure only (`--staged`).
        cheap: bool,
        /// The value of the field's cheap measure (`--staged`), for scripts.
        cheap_value: Option<f64>,
    }
    impl Listed {
        fn new(board: &ygo_solver::Board, variants: usize) -> Self {
            Listed {
                board: board.clone(),
                variants,
                measured: None,
                took: std::time::Duration::ZERO,
                exposure: None,
                loss: None,
                quick: None,
                cheap: false,
                cheap_value: None,
            }
        }

        /// The full measure's value, then the cheap one's: the best item of a field.
        fn rank(&self) -> (f64, f64) {
            let ok = self.measured.as_ref().and_then(|m| m.as_ref().ok());
            let full = ok.filter(|_| !self.cheap).map(composite);
            (full.unwrap_or(f64::NEG_INFINITY), ok.filter(|_| self.cheap).map(cheap_composite).unwrap_or(f64::NEG_INFINITY))
        }
    }
    type Answers = Vec<Vec<u8>>;
    type Kept = Result<ygo_solver::Measure, String>;
    /// A full measure, its time, the quick value and its time, and whether it went on from the cheap one.
    type FullKept = (Kept, std::time::Duration, Option<(f64, std::time::Duration)>, bool);
    /// The lines measured at the stops so far, by their answers: a line
    /// listed again at a later stop is not measured again. A cheap measure
    /// keeps what the full one goes on from.
    struct Measurer<'p, 'e> {
        prober: &'p mut ygo_solver::Prober<'e>,
        /// `probe`, without the breakers, the quick value too.
        how: (bool, bool, bool),
        cheap: HashMap<Answers, (Kept, Option<ygo_solver::probe::Partial>, std::time::Duration)>,
        full: HashMap<Answers, FullKept>,
        again: usize,
    }
    fn unkept(kept: &Kept) -> Result<ygo_solver::Measure, ygo_engine::Error> {
        kept.clone().map_err(ygo_engine::Error::Engine)
    }
    impl Measurer<'_, '_> {
        fn cheap(&mut self, item: &mut Listed, how: ygo_solver::probe::Cheap) {
            let key = &item.board.line.answers;
            if !self.cheap.contains_key(key) {
                let started = std::time::Instant::now();
                let (measured, partial) = match self.prober.measure_cheap(&item.board.line, how) {
                    Ok((m, p)) => (Ok(m), Some(p)),
                    Err(e) => (Err(e.to_string()), None),
                };
                self.cheap.insert(key.clone(), (measured, partial, started.elapsed()));
            } else {
                self.again += 1;
            }
            let (measured, _, took) = &self.cheap[key];
            item.measured = Some(unkept(measured));
            item.took = *took;
            item.cheap = true;
            item.cheap_value = measured.as_ref().ok().map(cheap_composite);
        }

        /// The full measure (with `probe`), from the cheap one's runs when it was taken.
        fn full(&mut self, item: &mut Listed) {
            let (probe, no_breakers, quick_value) = self.how;
            if !probe {
                return;
            }
            let key = &item.board.line.answers;
            if !self.full.contains_key(key) {
                let line = &item.board.line;
                let partial = self.cheap.get_mut(key).and_then(|c| c.1.take());
                let started = std::time::Instant::now();
                let (measured, resumed) = match partial {
                    Some(partial) if !no_breakers => (self.prober.measure_resumed(line, partial), true),
                    _ if no_breakers => (self.prober.measure_unbroken(line), false),
                    _ => (self.prober.measure(line), false),
                };
                let took = started.elapsed();
                let quick = quick_value
                    .then(|| {
                        let started = std::time::Instant::now();
                        self.prober.quick_measure(line).ok().map(|v| (v, started.elapsed()))
                    })
                    .flatten();
                self.full.insert(key.clone(), (measured.map_err(|e| e.to_string()), took, quick, resumed));
            } else {
                self.again += 1;
            }
            let (measured, took, quick, resumed) = &self.full[key];
            item.measured = Some(unkept(measured));
            // Going on from the cheap measure, the time is both's.
            item.took = if *resumed && item.cheap { item.took + *took } else { *took };
            item.quick = *quick;
            item.cheap = false;
        }

        /// The fields of one stop, measured as `combos` measures the fields
        /// of a search: with `staged`, the cheap measure of the first fields
        /// and the full one of the best by it; else the full one of all.
        /// With `probe`, the other variants of the best fields too.
        fn stop(&mut self, groups: &[(u64, Vec<ygo_solver::Board>)], staged: Option<Staged>) -> Vec<Listed> {
            let mut items: Vec<Listed> = groups.iter().map(|(_, group)| Listed::new(&group[0], group.len() - 1)).collect();
            match staged {
                Some(staged) => {
                    // The first variant found need not be the best: the field
                    // keeps the best of its variants by the cheap measure.
                    for (i, item) in items.iter_mut().enumerate().take(staged.cheap) {
                        let group = &groups[i].1;
                        for board in variants(group, staged.variants) {
                            let mut variant = Listed::new(board, group.len() - 1);
                            self.cheap(&mut variant, staged.measure);
                            if item.measured.is_none() || variant.rank().1 > item.rank().1 {
                                *item = variant;
                            }
                        }
                    }
                    let mut order: Vec<usize> = (0..items.len()).filter(|&i| items[i].cheap_value.is_some()).collect();
                    order.sort_by(|&a, &b| {
                        items[b].cheap_value.unwrap_or(0.0).total_cmp(&items[a].cheap_value.unwrap_or(0.0)).then(a.cmp(&b))
                    });
                    for &i in order.iter().take(staged.full) {
                        self.full(&mut items[i]);
                    }
                }
                None => {
                    for item in &mut items {
                        self.full(item);
                    }
                }
            }
            if self.how.0 {
                // The other variants only for the fields printed first: they decide
                // the top of the list, and each variant is a probe turn more. A field
                // only gains from them, so these fields stay first.
                let mut first: Vec<usize> = (0..items.len()).collect();
                first.sort_by(|&a, &b| items[b].rank().0.total_cmp(&items[a].rank().0).then(a.cmp(&b)));
                for &i in first.iter().take(FIELDS_WITH_VARIANTS) {
                    if items[i].cheap || items[i].measured.is_none() {
                        continue;
                    }
                    // The variants measured cheap first, best first, then the
                    // others in the search's order.
                    let group = &groups[i].1;
                    let shown = &items[i].board.line.answers;
                    let mut others: Vec<(f64, &ygo_solver::Board)> = group
                        .iter()
                        .filter(|b| &b.line.answers != shown)
                        .map(|b| {
                            let cheap = self.cheap.get(&b.line.answers).and_then(|c| c.0.as_ref().ok()).map(cheap_composite);
                            (cheap.unwrap_or(f64::NEG_INFINITY), b)
                        })
                        .collect();
                    others.sort_by(|a, b| b.0.total_cmp(&a.0));
                    let others: Vec<&ygo_solver::Board> = others.into_iter().map(|(_, b)| b).take(MEASURED_VARIANTS - 1).collect();
                    for board in others {
                        let mut item = Listed::new(board, group.len() - 1);
                        self.full(&mut item);
                        if item.rank().0 > items[i].rank().0 {
                            item.cheap_value = items[i].cheap_value;
                            items[i] = item;
                        }
                    }
                }
            }
            items
        }
    }
    let probe_started = std::time::Instant::now();
    let mut measurer =
        Measurer { prober: &mut prober, how: (probe, no_breakers, quick_value), cheap: HashMap::new(), full: HashMap::new(), again: 0 };
    // The checkpoints with their own numbers of fields measured: those of a
    // checkpoint do not depend on the budget.
    let at_checkpoint = staged.map(|s| Staged { cheap: checkpoint_cheap, full: checkpoint_full, ..s });
    let checkpoint_fields = if staged.is_some() { top.max(checkpoint_cheap) } else { checkpoint_full };
    let at_stops: Vec<Vec<Listed>> =
        result.checkpoints.iter().map(|(_, boards)| measurer.stop(&fields(boards, checkpoint_fields), at_checkpoint)).collect();
    let mut groups = fields(&result.boards, top);
    let mut boards = measurer.stop(&groups, staged);
    // The search's estimate does not see the opponent's turn, and the best
    // board measured is often low in its order (docs/plans/sprechi-ricerca.md
    // §7): while it is among the last fields measured, the next ones are
    // measured too.
    if probe && staged.is_none() && !interaction && top > 0 {
        loop {
            let best = (0..boards.len()).max_by(|&a, &b| boards[a].rank().0.total_cmp(&boards[b].rank().0).then(b.cmp(&a)));
            let Some(best) = best else { break };
            if best + LAST_FINALISTS < boards.len() || boards.len() >= MAX_FINALISTS {
                break;
            }
            let more = fields(&result.boards, (boards.len() + top).min(MAX_FINALISTS));
            if more.len() <= groups.len() {
                break;
            }
            boards.extend(measurer.stop(&more[groups.len()..], None));
            groups = more;
        }
    }
    let measured_again = measurer.again;
    // The fields of the search's end in its order, then those measured
    // only at a checkpoint; a field measured better at a checkpoint
    // (another line, another variant) keeps that measure.
    for item in at_stops.into_iter().flatten().filter(|item| item.measured.is_some()) {
        let key = ygo_solver::search::field_key(&item.board.state);
        match boards.iter().position(|b| ygo_solver::search::field_key(&b.board.state) == key) {
            Some(j) if item.rank() > boards[j].rank() => boards[j] = item,
            Some(_) => {}
            None => boards.push(item),
        }
    }
    // Ranked by interaction, the opponent's handtraps on your turn do not count.
    if probe && !interaction {
        for item in boards.iter_mut().filter(|item| !item.cheap && item.measured.is_some()) {
            let points = ygo_solver::explain::exposure_points(
                engine,
                &config.deck,
                &config.hand,
                config.seed,
                config.opponent.as_ref(),
                &item.board.line,
            )
            .ok();
            item.exposure = points.as_ref().map(|(x, _)| *x);
            item.loss = points.and_then(|(x, points)| expected_loss(&mut prober, &item.board.line, &x, &points));
        }
    }
    if !stops.is_empty() {
        println!(
            "checkpoints: {} of {} nodes passed, {} measures taken from another list",
            result.checkpoints.len(),
            stops.iter().map(u64::to_string).collect::<Vec<_>>().join(", "),
            measured_again
        );
    }
    let probe_seconds = probe_started.elapsed().as_secs_f64();
    if probe {
        let cheap = boards.iter().filter(|item| item.cheap).count();
        let full = boards.iter().filter(|item| !item.cheap && item.measured.is_some()).count();
        println!("probe turns: {full} full, {cheap} cheap only, in {probe_seconds:.1} s");
    }
    type Value = fn(&ygo_solver::Measure) -> f64;
    let (full_value, cheap_value_of): (Value, Value) =
        if interaction { (ygo_solver::interaction_value, ygo_solver::interaction_value) } else { (composite, cheap_composite) };
    let measured_value = |item: &Listed| item.measured.as_ref().and_then(|m| m.as_ref().ok()).filter(|_| !item.cheap).map(full_value);
    let cheap_value = |item: &Listed| item.measured.as_ref().and_then(|m| m.as_ref().ok()).filter(|_| item.cheap).map(cheap_value_of);
    let mut printed: Vec<usize> = (0..boards.len()).collect();
    // A board whose probe turn failed goes after the measured ones, and one
    // with the cheap measure only after those with the full one.
    printed.sort_by(|&a, &b| {
        let (va, vb) = (measured_value(&boards[a]), measured_value(&boards[b]));
        let (ca, cb) = (cheap_value(&boards[a]), cheap_value(&boards[b]));
        vb.unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&va.unwrap_or(f64::NEG_INFINITY))
            .then(cb.unwrap_or(f64::NEG_INFINITY).total_cmp(&ca.unwrap_or(f64::NEG_INFINITY)))
            .then(a.cmp(&b))
    });
    // By capacity: what your cards can stop on the opponent's turn, read
    // from their effects; the order before breaks ties.
    let started = std::time::Instant::now();
    let capacities: Vec<Option<Result<Checked, ygo_engine::Error>>> =
        boards.iter().map(|item| capacity.then(|| ygo_solver::board_capacity(engine, &config, &item.board.line))).collect();
    if capacity {
        let key = |i: usize| match &capacities[i] {
            Some(Ok((_, c))) => (c.count, c.quality, c.without_opponent, c.off_field(), c.floodgates.len()),
            _ => (0, f64::NEG_INFINITY, 0, 0, 0),
        };
        printed.sort_by(|&a, &b| {
            let (ka, kb) = (key(a), key(b));
            kb.0.cmp(&ka.0)
                .then(kb.1.total_cmp(&ka.1))
                .then(kb.2.cmp(&ka.2))
                .then(kb.3.cmp(&ka.3))
                .then(kb.4.cmp(&ka.4))
                .then(boards[a].board.line.choices.len().cmp(&boards[b].board.line.choices.len()))
        });
        println!("capacity: {} boards checked in {:.2} s", boards.len(), started.elapsed().as_secs_f64());
    }
    // The last step: the best boards by the measure against the meta's
    // most played decks, listed first by it.
    let mut by_measure = vec![0; boards.len()];
    for (rank, &i) in printed.iter().enumerate() {
        by_measure[i] = rank;
    }
    let mut metas: Vec<Option<ygo_solver::MetaScore>> = vec![None; boards.len()];
    // The time to read the meta decks' lines, and to measure the boards.
    let mut meta_seconds: Option<(f64, f64)> = None;
    if let Some(meta) = meta.as_ref().filter(|_| probe) {
        let started = std::time::Instant::now();
        let decks = ygo_solver::most_played(&load_meta(dir, meta)?, META_DECKS);
        let rivals = ygo_solver::meta_rivals(engine, &decks, config.seed, config.threads);
        let lines_took = started.elapsed();
        let started = std::time::Instant::now();
        let head: Vec<usize> = printed.iter().copied().filter(|&i| measured_value(&boards[i]).is_some()).take(meta.boards).collect();
        for &i in &head {
            metas[i] = ygo_solver::meta_score(engine, &config.deck, &config.hand, config.seed, &boards[i].board.line, &rivals, &kit).ok();
        }
        let value = |i: usize| metas[i].as_ref().and_then(|m| m.value).unwrap_or(f64::NEG_INFINITY);
        let mut first = head.clone();
        first.sort_by(|&a, &b| value(b).total_cmp(&value(a)).then(by_measure[a].cmp(&by_measure[b])));
        printed.retain(|i| !head.contains(i));
        printed.splice(0..0, first);
        meta_seconds = Some((lines_took.as_secs_f64(), started.elapsed().as_secs_f64()));
        println!(
            "meta final: {} boards against {} (lines {:.1} s, boards {:.1} s)",
            head.len(),
            decks.iter().map(|d| format!("{} {:.2}", d.name, d.weight)).collect::<Vec<_>>().join(", "),
            lines_took.as_secs_f64(),
            started.elapsed().as_secs_f64()
        );
    }
    let mut rank_of = vec![0; boards.len()];
    for (rank, &i) in printed.iter().enumerate() {
        rank_of[i] = rank;
    }
    for (rank, &i) in printed.iter().enumerate() {
        let Listed { board, variants, measured, took, exposure, loss, quick, cheap, .. } = &boards[i];
        let variants = *variants;
        let me = &board.state.players[0];
        let zone = |(z, c): (usize, &ygo_engine::FieldCard)| {
            if (5..=6).contains(&z) {
                format!("{} [EMZ]", db.name(c.code))
            } else {
                db.name(c.code)
            }
        };
        let field: Vec<String> = me
            .monsters
            .iter()
            .enumerate()
            .filter_map(|(z, c)| c.as_ref().map(|c| (z, c)))
            .map(zone)
            .chain(me.spells.iter().flatten().map(|c| db.name(c.code)))
            .collect();
        let hand: Vec<String> = me.hand.iter().map(|&c| db.name(c)).collect();
        let value = match (measured_value(&boards[i]), cheap_value(&boards[i])) {
            (Some(v), _) => format!(" · measured {v:.2}"),
            (None, Some(v)) => format!(" · cheap {v:.2}"),
            _ => String::new(),
        };
        println!(
            "\n#{}{}{} score {:.1}{value} · search #{} · {} cards used · {} choices · found at {:.1} s\n   field: {}\n   hand: {}",
            rank + 1,
            if board.exact { "" } else { " (End Phase not played)" },
            if variants > 0 { format!(" (+{variants} variants)") } else { String::new() },
            board.quick_score,
            i + 1,
            board.cards_used,
            board.line.choices.len(),
            board.found_after.as_secs_f64(),
            field.join(", "),
            hand.join(", ")
        );
        match measured {
            // Not the probe turn's line: scripts read that one as a full measure.
            Some(Ok(m)) if *cheap => println!(
                "   cheap measure ({:.1} s): field {:.2} · hand {:.2} · floodgates {:.2}",
                took.as_secs_f64(),
                m.field_value,
                m.hand_value,
                m.floodgate_value
            ),
            Some(Ok(m)) => print_measure(engine, m, *took),
            Some(Err(e)) => println!("   probe turn failed: {e}"),
            None => {}
        }
        if let Some(Ok(m)) = measured.as_ref().filter(|_| interaction) {
            print_interaction(&profile_of(m, &board.state));
        }
        match &capacities[i] {
            Some(Ok((_, c))) => print_capacity(db, c),
            Some(Err(e)) => println!("   capacity: dry check failed: {e}"),
            None => {}
        }
        if let Some(x) = exposure {
            println!(
                "   exposure: {} choke points · Ash {} · on-field negation {} · summons {} (special {}) · searches {} · Deck cost {} · LP paid {}",
                x.choke_points(),
                x.ash,
                x.on_field,
                x.summons,
                x.special_summons,
                x.searches,
                x.deck_cost,
                x.lp_paid
            );
        }
        if let Some((l, took)) = loss {
            print_loss(l, *took);
        }
        if let Some((v, took)) = quick {
            println!("   quick value ({:.1} s): {v:.2}", took.as_secs_f64());
        }
        if let Some(m) = &metas[i] {
            let decks: Vec<String> = m
                .decks
                .iter()
                .map(|d| match d.left {
                    Some(left) => format!("{} {:.0}% left", d.name, left * 100.0),
                    None => format!("{} not measured ({} lines, {:.0}% made)", d.name, d.lines, d.made * 100.0),
                })
                .collect();
            let value = m.value.map_or("-".to_owned(), |v| format!("{v:.2}"));
            println!("   meta (L2-3): {value} · {} · by the measure #{}", decks.join(" · "), by_measure[i] + 1);
        }
        if show_lines {
            let (_, steps) = ygo_solver::explain(engine, &config.deck, &config.hand, config.seed, config.opponent.as_ref(), &board.line)?;
            for (k, step) in steps.iter().enumerate() {
                if let Some(choice) = &step.choice {
                    println!("     {:>2}. {choice}", k);
                }
                for line in &step.events {
                    println!("           {line}");
                }
            }
        }
    }
    let mut measured_boards = Vec::new();
    let mut saved_lines = Vec::new();
    for (i, item) in boards.iter().enumerate() {
        if probe || capacity {
            let measured = item.measured.as_ref().and_then(|m| m.as_ref().ok());
            let mut b = board_json(engine, &config, &item.board, i, item.variants, measured, item.took, item.exposure.as_ref());
            b["printed"] = serde_json::json!(rank_of[i]);
            match &capacities[i] {
                Some(Ok((effects, c))) => b["capacity"] = capacity_json(engine.cards(), c, effects),
                Some(Err(e)) => b["capacity"] = serde_json::json!({ "error": e.to_string() }),
                None => {}
            }
            if staged.is_some() {
                b["stage"] = serde_json::json!(match (&item.measured, item.cheap) {
                    (None, _) => "none",
                    (Some(_), true) => "cheap",
                    (Some(_), false) => "full",
                });
                b["cheap_value"] = serde_json::json!(item.cheap_value);
            }
            if let Some(m) = &metas[i] {
                b["printed_by_measure"] = serde_json::json!(by_measure[i]);
                b["meta"] = serde_json::json!({
                    "value": m.value,
                    "left": m.left,
                    "decks": m.decks.iter().map(|d| serde_json::json!({
                        "name": d.name, "weight": d.weight, "lines": d.lines, "made": d.made, "left": d.left,
                    })).collect::<Vec<_>>(),
                });
            }
            if let Some((l, took)) = &item.loss {
                b["expected_loss"] = loss_json(l, *took);
            }
            if let Some((v, took)) = item.quick {
                b["quick_value"] = serde_json::json!({ "value": v, "seconds": took.as_secs_f64() });
            }
            measured_boards.push(b);
        }
        saved_lines.push(line_json(&item.board.line));
    }
    let header = |kind: &str, items: Vec<serde_json::Value>| {
        serde_json::json!({
            "kind": kind,
            "deck": setup.deck.display().to_string(),
            "hand": config.hand,
            "seed": config.seed,
            "hand_traps": config.hand_traps,
            "nodes": result.progress.nodes,
            "boards_found": result.progress.boards,
            "probe_seconds": probe_seconds,
            "meta_seconds": meta_seconds.map(|(lines, boards)| serde_json::json!({ "lines": lines, "boards": boards })),
            "items": items,
        })
    };
    if let Some(path) = &json {
        std::fs::write(path, serde_json::to_string_pretty(&header("boards", measured_boards))?)?;
    }
    if let Some(path) = &save_lines {
        std::fs::write(path, serde_json::to_string_pretty(&header("lines", saved_lines))?)?;
    }
    Ok(())
}

/// A board told for scripts: an id from the cards (the same board, whatever
/// the line), the cards, and what the probe turn measured.
#[allow(clippy::too_many_arguments)]
fn board_json(
    engine: &Engine,
    config: &ygo_solver::SearchConfig,
    board: &ygo_solver::Board,
    index: usize,
    variants: usize,
    measured: Option<&ygo_solver::Measure>,
    took: std::time::Duration,
    exposure: Option<&ygo_solver::Exposure>,
) -> serde_json::Value {
    state_json(
        engine,
        &config.hand_traps,
        &board.state,
        measured,
        took,
        exposure,
        serde_json::json!({
            "index": index,
            "variants": variants,
            "exact": board.exact,
            "quick_score": board.quick_score,
            "cards_used": board.cards_used,
            "choices": board.line.choices.len(),
        }),
    )
}

fn state_json(
    engine: &Engine,
    hand_traps: &[u32],
    state: &ygo_engine::DuelState,
    measured: Option<&ygo_solver::Measure>,
    took: std::time::Duration,
    exposure: Option<&ygo_solver::Exposure>,
    mut out: serde_json::Value,
) -> serde_json::Value {
    let db = engine.cards();
    let me = &state.players[0];
    let mut field: Vec<String> = Vec::new();
    for (z, c) in me.monsters.iter().enumerate() {
        if let Some(c) = c {
            // A monster Set face-down cannot use its effects: it must show.
            let face = if c.is_face_up() { "" } else { " (set)" };
            field.push(format!("{}{face}{}", db.name(c.code), if z >= 5 { " [EMZ]" } else { "" }));
        }
    }
    for c in me.spells.iter().flatten() {
        let target = c.equipped_to.and_then(|(ctrl, loc, seq)| {
            (loc == location::MZONE)
                .then(|| state.players.get(ctrl as usize)?.monsters.get(seq as usize)?.as_ref().map(|m| db.name(m.code)))
                .flatten()
        });
        let face = if c.is_face_up() { "" } else { " (set)" };
        field.push(match target {
            Some(t) => format!("{}{face} -> {t}", db.name(c.code)),
            None => format!("{}{face}", db.name(c.code)),
        });
    }
    field.sort();
    let mut hand: Vec<String> = me.hand.iter().map(|&c| db.name(c)).collect();
    hand.sort();
    let signature = format!("{} | hand: {}", field.join(", "), hand.join(", "));
    let canon = |c: u32| db.canonical_code(c);
    let hand_trap = |c: u32| hand_traps.iter().any(|&h| canon(h) == canon(c));
    let mut value = serde_json::json!({
        "look_key": format!("{:016x}", ygo_solver::look_key(state)),
        "signature": signature,
        "field": field,
        "hand": me.hand,
        "hand_names": hand,
        "hand_traps_in_hand": me.hand.iter().filter(|&&c| hand_trap(c)).count(),
        "grave": me.grave.len(),
        "banished": me.banished.len(),
        "grave_names": me.grave.iter().map(|&c| db.name(c)).collect::<Vec<_>>(),
        "banished_names": me.banished.iter().map(|&(c, _)| db.name(c)).collect::<Vec<_>>(),
    });
    if let Some(m) = measured {
        value["interaction"] = interaction_json(&profile_of(m, state));
        value["measure"] = serde_json::json!({
            "seconds": took.as_secs_f64(),
            "field": m.field_value,
            "hand": m.hand_value,
            "floodgates": m.floodgate_value,
            "resilience": m.resilience,
            "follow_up": m.follow_up.map(|f| serde_json::json!({ "rebuild": f.rebuild, "gained": f.gained })),
            "seconds_by_part": {
                "responses": m.seconds.responses,
                "together": m.seconds.together,
                "breakers": m.seconds.breakers,
                "follow_up": m.seconds.follow_up,
            },
            "attrition": m.attrition,
            "taken": m.taken.iter().map(|&(c, l)| serde_json::json!([db.name(c), location::name(l)])).collect::<Vec<_>>(),
            "countered": m.countered,
            "floodgate_list": m.floodgates.iter().map(|p| p.label()).collect::<Vec<_>>(),
            "offered": m.offered.iter().map(|&(c, l)| serde_json::json!([c, l])).collect::<Vec<_>>(),
            "interruptions": m.interruptions.iter().map(|i| serde_json::json!({
                "card": db.name(i.responder.code),
                "code": i.responder.code,
                "location": location::name(i.location),
                "effect": i.effect.label(),
                "targeted": i.targeted,
                "uses": i.uses,
                "coverage": i.coverage,
                "value": i.value,
                "alone": i.alone,
                "covers": i.covers.iter().map(|p| p.label()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "breakers": m.breakers.iter().map(|b| serde_json::json!({
                "name": b.name,
                "weight": b.weight,
                "not_applicable": b.not_applicable,
                "stopped_by": b.stopped_by.map(|r| db.name(r.code)),
                "remaining": b.remaining,
            })).collect::<Vec<_>>(),
        });
    }
    if let Some(x) = exposure {
        value["exposure"] = serde_json::json!({
            "choke_points": x.choke_points(),
            "ash": x.ash,
            "on_field": x.on_field,
            "summons": x.summons,
            "special_summons": x.special_summons,
            "searches": x.searches,
            "deck_cost": x.deck_cost,
            "lp_paid": x.lp_paid,
        });
    }
    if let (Some(o), Some(v)) = (out.as_object_mut(), value.as_object()) {
        o.extend(v.clone());
    }
    out
}

fn line_json(line: &ygo_solver::Line) -> serde_json::Value {
    serde_json::json!({ "answers": line.answers, "choices": line.choices, "choice_at": line.choice_at })
}

fn line_from_json(v: &serde_json::Value) -> Result<ygo_solver::Line> {
    let answers = v["answers"]
        .as_array()
        .context("line without answers")?
        .iter()
        .map(|a| a.as_array().map(|b| b.iter().filter_map(|x| x.as_u64().map(|x| x as u8)).collect()).context("bad answer"))
        .collect::<Result<Vec<Vec<u8>>>>()?;
    let choices = v["choices"].as_array().map(|c| c.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default();
    let choice_at =
        v["choice_at"].as_array().map(|c| c.iter().filter_map(|x| x.as_u64().map(|x| x as usize)).collect()).unwrap_or_default();
    Ok(ygo_solver::Line { answers, choices, choice_at })
}

/// Plays saved lines again and measures their boards (no search).
#[allow(clippy::too_many_arguments)]
fn probe_line(
    dir: &Path,
    setup: &Setup,
    lines: &Path,
    index: Option<usize>,
    json: Option<&Path>,
    vs: Option<&Path>,
    stops: bool,
    moves: bool,
    protection: bool,
) -> Result<()> {
    let (engine, deck, mut hand) = start(dir, setup)?;
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(lines)?)?;
    if hand.is_empty() {
        hand = saved["hand"].as_array().map(|h| h.iter().filter_map(|c| c.as_u64().map(|c| c as u32)).collect()).unwrap_or_default();
    }
    // The lines are played in the duel they were found in, with the seed saved
    // with them: with another one a shuffle or a mill differs and the line breaks.
    let seed = saved["seed"].as_u64().unwrap_or(setup.seed);
    let kit = ygo_solver::Kit::standard(&engine);
    let opponent = Some(kit.opponent().clone());
    let hand_traps = ygo_solver::hand_traps(&engine, &deck, seed, &kit);
    let mut prober = ygo_solver::Prober::new(&engine, &deck, &hand, seed, &kit);
    let rival = match vs {
        Some(path) => {
            let text = std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
            let rival_deck = DeckList::from_ydk(&text)?;
            let rival = ygo_solver::Rival::new(&engine, &rival_deck, seed, &kit);
            // Once each: a card the kit plays two ways (Lightning Storm) is one card.
            let names = |codes: &[u32]| {
                let mut names: Vec<String> = Vec::new();
                for &c in codes {
                    let name = engine.cards().name(c);
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                names.join(", ")
            };
            println!("against {}", path.display());
            println!(
                "   breakers: {}",
                if rival.breakers().is_empty() { "none it can play (standard ones)".to_owned() } else { names(rival.breakers()) }
            );
            println!("   handtraps: {}", names(&rival.hand_traps));
            // Its best lines, read once.
            let lines = if moves {
                let started = std::time::Instant::now();
                let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
                let lines = ygo_solver::rival_lines(&engine, &rival_deck, seed, 12, 1000, threads);
                println!("   its lines ({:.1} s): {} from 12 hands", started.elapsed().as_secs_f64(), lines.len());
                for (k, l) in lines.iter().enumerate() {
                    println!(
                        "     line {}: {} · {}",
                        k + 1,
                        names(&l.hand),
                        l.moves.iter().map(|&(c, kind, at)| play_label(&engine, c, kind, at)).collect::<Vec<_>>().join(" > ")
                    );
                }
                lines
            } else {
                Vec::new()
            };
            Some((rival, rival_deck, lines))
        }
        None => None,
    };
    let items = saved["items"].as_array().context("no lines in the file")?;
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate().filter(|(i, _)| index.is_none_or(|k| k == *i)) {
        let line = line_from_json(item)?;
        let (state, _) = ygo_solver::explain(&engine, &deck, &hand, seed, opponent.as_ref(), &line)?;
        let started = std::time::Instant::now();
        let measured = prober.measure(&line);
        let took = started.elapsed();
        let points = ygo_solver::explain::exposure_points(&engine, &deck, &hand, seed, opponent.as_ref(), &line).ok();
        let exposure = points.as_ref().map(|(x, _)| *x);
        let loss = points.and_then(|(x, points)| expected_loss(&mut prober, &line, &x, &points));
        let me = &state.players[0];
        println!(
            "#{} field: {}",
            i + 1,
            me.monsters.iter().chain(&me.spells).flatten().map(|c| engine.cards().name(c.code)).collect::<Vec<_>>().join(", ")
        );
        match &measured {
            Ok(m) => print_measure(&engine, m, took),
            Err(e) => println!("   probe turn failed: {e}"),
        }
        if let Some((l, took)) = &loss {
            print_loss(l, *took);
        }
        let mut extra = serde_json::json!({ "index": i });
        if let Some((l, took)) = &loss {
            extra["expected_loss"] = loss_json(l, *took);
        }
        if let (true, Ok(m)) = (protection, &measured) {
            let started = std::time::Instant::now();
            let pieces = prober.protection(&line, &m.interruptions);
            println!("   protection ({:.1} s):", started.elapsed().as_secs_f64());
            let labels = |rs: &[ygo_solver::Removal]| rs.iter().map(|r| r.label()).collect::<Vec<_>>().join(", ");
            for p in &pieces {
                println!(
                    "     {}: holds against {} · falls to {}",
                    engine.cards().name(p.responder.code),
                    if p.holds.is_empty() { "nothing".to_owned() } else { labels(&p.holds) },
                    if p.falls.is_empty() { "nothing".to_owned() } else { labels(&p.falls) }
                );
            }
            extra["protection"] = pieces
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "card": engine.cards().name(p.responder.code),
                        "code": p.responder.code,
                        "location": location::name(p.location),
                        "holds": p.holds.iter().map(|r| r.label()).collect::<Vec<_>>(),
                        "falls": p.falls.iter().map(|r| r.label()).collect::<Vec<_>>(),
                    })
                })
                .collect();
        }
        if let (Some((rival, rival_deck, lines)), Ok(m)) = (&rival, &measured) {
            let started = std::time::Instant::now();
            match ygo_solver::matchup(&engine, &deck, &hand, seed, &line, m, rival) {
                Ok(x) => {
                    let took = started.elapsed();
                    print_matchup(&engine, &x, took);
                    extra["matchup"] = matchup_json(&x);
                    extra["matchup"]["seconds"] = took.as_secs_f64().into();
                }
                Err(e) => println!("   matchup failed: {e}"),
            }
            if stops {
                let started = std::time::Instant::now();
                match ygo_solver::draw_trade_off(&engine, &deck, &hand, seed, &line, rival) {
                    Ok(offs) => {
                        print_trade_off(&engine, &offs, started.elapsed());
                        extra["stops"] = trade_off_json(&offs);
                    }
                    Err(e) => println!("   stops failed: {e}"),
                }
            }
            if moves {
                let started = std::time::Instant::now();
                match ygo_solver::against_moves(&engine, &deck, &hand, seed, &line, rival_deck, lines, &kit) {
                    Ok(x) => print_moves(&engine, &x, started.elapsed()),
                    Err(e) => println!("   moves failed: {e}"),
                }
            }
        }
        out.push(state_json(&engine, &hand_traps, &state, measured.as_ref().ok(), took, exposure.as_ref(), extra));
    }
    if let Some(path) = json {
        let doc = serde_json::json!({
            "kind": "boards",
            "deck": setup.deck.display().to_string(),
            "hand": hand,
            "seed": seed,
            "hand_traps": hand_traps,
            "items": out,
        });
        std::fs::write(path, serde_json::to_string_pretty(&doc)?)?;
    }
    Ok(())
}

/// Where a card is, in a few words.
fn place_name(from: u8) -> &'static str {
    match from {
        location::HAND => "hand",
        location::MZONE => "field",
        location::SZONE => "S/T zone",
        location::GRAVE => "GY",
        location::REMOVED => "banished",
        location::EXTRA => "Extra Deck",
        location::DECK => "Deck",
        _ => "?",
    }
}

/// "Normal Summon of X", "Special Summon of X (hand)", "X (field)".
fn play_label(engine: &Engine, code: u32, kind: ActionKind, from: u8) -> String {
    let name = engine.cards().name(code);
    match kind {
        ActionKind::NormalSummon => format!("Normal Summon of {name}"),
        ActionKind::SpecialSummon => format!("Special Summon of {name} ({})", place_name(from)),
        _ => format!("{name} ({})", place_name(from)),
    }
}

fn print_moves(engine: &Engine, x: &ygo_solver::MovesMatchup, took: std::time::Duration) {
    let db = engine.cards();
    // No line of the opponent could be played against the board: no
    // measure, not a measure of 0 (the scripts read it as missing).
    if x.lines == 0 {
        println!(
            "   against its lines ({:.1} s): no line of the opponent could be played against this board: not measured",
            took.as_secs_f64()
        );
        return;
    }
    let possible = x.moves.iter().filter(|m| m.possible).map(|m| m.weight).sum::<f64>();
    println!(
        "   against its lines ({:.1} s): {} lines · {:.0}% of their moves made on an empty board · value {:.2} · floodgates stop {:.0}% · {:.0}% of the rest left with all your answers",
        took.as_secs_f64(),
        x.lines,
        possible * 100.0,
        x.value,
        x.floodgate_share * 100.0,
        x.left * 100.0
    );
    for (i, m) in x.moves.iter().enumerate() {
        let state = match (m.possible, m.here, x.floodgated.contains(&i)) {
            (false, _, _) => "not made even on an empty board",
            (true, _, true) => "stopped by the board",
            (true, true, _) => "made",
            (true, false, _) => "lost with an earlier one",
        };
        println!("     {}. line {}: {} · {state}", i + 1, m.line + 1, play_label(engine, m.code, m.kind, m.location));
    }
    for s in &x.stops {
        let stops: Vec<String> = s.stops.iter().map(|&i| (i + 1).to_string()).collect();
        println!(
            "     {} ({}): {} moves {} · coverage {:.0}% · value {:.2}",
            db.name(s.responder.code),
            place_name(s.location),
            s.effect.label(),
            stops.join(", "),
            s.coverage * 100.0,
            s.value
        );
    }
}

fn print_matchup(engine: &Engine, x: &ygo_solver::Matchup, took: std::time::Duration) {
    let db = engine.cards();
    println!(
        "   against the deck ({:.1} s): resilience {:.2}{} · breaker in hand {:.0}% · a handtrap that stops the line {:.0}%",
        took.as_secs_f64(),
        x.resilience,
        if x.standard_breakers { " (standard breakers)" } else { "" },
        x.breaker_chance * 100.0,
        x.hand_trap_chance * 100.0
    );
    for b in &x.breakers {
        let how = match (b.not_applicable, b.stopped_by) {
            (true, _) => "not applicable".to_owned(),
            (_, Some(r)) => format!("negated by {}", db.name(r.code)),
            _ => String::new(),
        };
        println!("     breaker {} ({:.0}%): left {:.2} {how}", db.name(b.code), b.chance * 100.0, b.remaining);
    }
    for h in &x.hand_traps {
        let at: Vec<String> = h.at.iter().map(|&c| db.name(c)).collect();
        println!(
            "     handtrap {} ({:.0}%): stops {} of {} moments [{}]{}",
            db.name(h.code),
            h.chance * 100.0,
            h.stops,
            h.windows,
            at.join(", "),
            if h.drawn > 0 { format!(" · draws up to {}", h.drawn) } else { String::new() }
        );
        if h.drawn > 0 {
            // The cards it draws at each moment, with your card then.
            let moments: Vec<String> = h
                .moments
                .iter()
                .map(|m| format!("{}{} {}", if m.stops { "*" } else { "" }, m.drawn, m.playing.map(|c| db.name(c)).unwrap_or_default()))
                .collect();
            println!("       drawn by moment: {}", moments.join(" · "));
        }
    }
}

fn print_trade_off(engine: &Engine, offs: &[ygo_solver::DrawTradeOff], took: std::time::Duration) {
    let db = engine.cards();
    if offs.is_empty() {
        println!("   stops ({:.1} s): no handtrap of the deck makes it draw", took.as_secs_f64());
    }
    for o in offs {
        let best = &o.stops[o.best];
        let whole = o.stops.last().expect("the whole line");
        println!(
            "   against {} ({:.1} s{}): best stop after {} answers ({}), {:.2} expected and {} cards drawn; whole line {:.2} and {}",
            db.name(o.code),
            took.as_secs_f64(),
            if o.standard_breakers { ", standard breakers" } else { "" },
            best.answers,
            best.playing.map(|c| db.name(c)).unwrap_or_default(),
            best.expected,
            best.drawn,
            whole.expected,
            whole.drawn
        );
        for (i, p) in o.stops.iter().enumerate() {
            println!(
                "     {}{:>3} {:<40} field {:.2} · floodgates {:.2} · drawn {:>2} · expected {:.2}",
                if i == o.best { "*" } else { " " },
                p.answers,
                p.playing.map(|c| db.name(c)).unwrap_or_default(),
                p.field_value,
                p.floodgate_value,
                p.drawn,
                p.expected
            );
        }
    }
}

fn trade_off_json(offs: &[ygo_solver::DrawTradeOff]) -> serde_json::Value {
    serde_json::json!(offs
        .iter()
        .map(|o| serde_json::json!({
            "code": o.code, "best": o.best, "standard_breakers": o.standard_breakers,
            "stops": o.stops.iter().map(|p| serde_json::json!({
                "answers": p.answers, "playing": p.playing, "field_value": p.field_value,
                "floodgate_value": p.floodgate_value, "drawn": p.drawn, "expected": p.expected,
            })).collect::<Vec<_>>(),
        }))
        .collect::<Vec<_>>())
}

fn matchup_json(x: &ygo_solver::Matchup) -> serde_json::Value {
    serde_json::json!({
        "resilience": x.resilience,
        "standard_breakers": x.standard_breakers,
        "breaker_chance": x.breaker_chance,
        "hand_trap_chance": x.hand_trap_chance,
        "rejected": x.rejected,
        "diverged": x.diverged,
        "breakers": x.breakers.iter().map(|b| serde_json::json!({
            "code": b.code, "chance": b.chance, "remaining": b.remaining,
            "stopped_by": b.stopped_by.map(|r| r.code), "not_applicable": b.not_applicable,
        })).collect::<Vec<_>>(),
        "hand_traps": x.hand_traps.iter().map(|h| serde_json::json!({
            "code": h.code, "chance": h.chance, "windows": h.windows, "stops": h.stops, "at": h.at, "drawn": h.drawn,
            "moments": h.moments.iter().map(|m| serde_json::json!({
                "playing": m.playing, "stops": m.stops, "drawn": m.drawn, "rejected": m.rejected,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// What the handtraps are expected to take away from `line` (B6), with
/// the default chances of the app, and how long it took.
fn expected_loss(
    prober: &mut ygo_solver::Prober,
    line: &ygo_solver::Line,
    exposure: &ygo_solver::Exposure,
    points: &[ygo_solver::explain::ChokePoint],
) -> Option<(ygo_solver::probe::ExpectedLoss, std::time::Duration)> {
    let started = std::time::Instant::now();
    let rates = ygo_solver::probe::HandTrapRates::default();
    let loss = prober.expected_loss(line, points, exposure.special_summons, &rates).ok()?;
    Some((loss, started.elapsed()))
}

fn loss_label(kind: ygo_solver::explain::ChokeKind) -> &'static str {
    use ygo_solver::explain::ChokeKind;
    match kind {
        ChokeKind::DeckTouch => "Ash",
        ChokeKind::FieldEffect => "Veiler/Impermanence",
        ChokeKind::FifthSummon => "Nibiru",
        ChokeKind::SecondSearch => "Droll",
    }
}

fn print_loss(l: &ygo_solver::probe::ExpectedLoss, took: std::time::Duration) {
    let kinds: Vec<String> = l.losses.iter().map(|&(k, v)| format!("{} {v:.2}", loss_label(k))).collect();
    println!(
        "   expected loss to handtraps ({:.1} s, {} cuts): {:.2} · lost at worst: {} · Maxx \"C\" {:.2}",
        took.as_secs_f64(),
        l.cuts,
        l.expected,
        if kinds.is_empty() { "nothing".to_owned() } else { kinds.join(", ") },
        l.maxx
    );
}

fn loss_json(l: &ygo_solver::probe::ExpectedLoss, took: std::time::Duration) -> serde_json::Value {
    serde_json::json!({
        "expected": l.expected,
        "losses": l.losses.iter().map(|&(k, v)| serde_json::json!([loss_label(k), v])).collect::<Vec<_>>(),
        "maxx": l.maxx,
        "cuts": l.cuts,
        "seconds": took.as_secs_f64(),
    })
}

/// Variants of a field measured in `combos --probe`, the first ones the
/// search found: the best measured is shown. Only for the first
/// `FIELDS_WITH_VARIANTS` fields printed; the others are measured once.
const MEASURED_VARIANTS: usize = 3;
/// The best board measured among this many last finalists: the next ones
/// are measured too (`combos --probe`), up to [`MAX_FINALISTS`].
const LAST_FINALISTS: usize = 4;
const MAX_FINALISTS: usize = 40;
const FIELDS_WITH_VARIANTS: usize = 3;

/// Variants of each field given the cheap measure (`docs/plans/piano-motore-impatto.md` §4).
const STAGED_VARIANTS: usize = 3;

fn profile_of(m: &ygo_solver::Measure, state: &ygo_engine::DuelState) -> InteractionProfile {
    InteractionProfile::of(m, state.players[0].hand.len(), state.players[0].grave.len())
}

/// A board's dry check and its capacity.
type Checked = (Vec<ygo_engine::DryEffect>, ygo_solver::Capacity);

const PLACES: [&str; 4] = ["field", "gy", "banished", "hand"];

fn capacity_json(db: &ygo_engine::CardDb, c: &ygo_solver::Capacity, effects: &[ygo_engine::DryEffect]) -> serde_json::Value {
    let by_class: serde_json::Map<String, serde_json::Value> = ygo_solver::InterruptionClass::ALL
        .iter()
        .zip(c.by_class)
        .filter(|(_, n)| *n > 0)
        .map(|(class, n)| (class.label().to_owned(), serde_json::json!(n)))
        .collect();
    serde_json::json!({
        "count": c.count,
        "quality": c.quality,
        "negations": c.negations(),
        "removals": c.removals(),
        "by_class": by_class,
        "by_place": { "field": c.by_place[0], "gy": c.by_place[1], "banished": c.by_place[2], "hand": c.by_place[3] },
        "without_opponent": c.without_opponent,
        "floodgates": c.floodgates.iter().map(|(card, code)| serde_json::json!({ "card": card, "effect": code })).collect::<Vec<_>>(),
        "responses": c.responses,
        "unchecked": c.unchecked,
        "exact": c.exact,
        "interruptions": c.interruptions.iter().zip(&c.plan).map(|(it, &n)| serde_json::json!({
            "card": it.code,
            "name": db.name(it.code),
            "place": PLACES[it.place()],
            "class": it.class.label(),
            "kind": it.kind.name(),
            "uses": it.uses,
            "in_plan": n,
            "assumes": assumption_names(it.assumes),
            "blocked": it.blocked.map(|b| b.label()),
        })).collect::<Vec<_>>(),
        "effects_checked": effects.len(),
    })
}

fn assumption_names(bits: u32) -> Vec<&'static str> {
    use ygo_engine::assume;
    [(assume::OPPONENT, "opponent's cards"), (assume::EVENT, "the event"), (assume::PHASE, "a phase"), (assume::ERROR, "unchecked")]
        .into_iter()
        .filter(|(bit, _)| bits & bit != 0)
        .map(|(_, name)| name)
        .collect()
}

fn print_capacity(db: &ygo_engine::CardDb, c: &ygo_solver::Capacity) {
    let classes: Vec<String> = ygo_solver::InterruptionClass::ALL
        .iter()
        .zip(c.by_class)
        .filter(|(_, n)| *n > 0)
        .map(|(class, n)| format!("{} {n}", class.label()))
        .collect();
    println!(
        "   capacity {} ({:.2} weighed{}): {} · from field {}, GY {}, banished {}, hand {} · {} without assuming the opponent's cards · {} floodgates · {} other quick effects",
        c.count,
        c.quality,
        if c.exact { "" } else { ", greedy" },
        if classes.is_empty() { "nothing".to_owned() } else { classes.join(", ") },
        c.by_place[0],
        c.by_place[1],
        c.by_place[2],
        c.by_place[3],
        c.without_opponent,
        c.floodgates.len(),
        c.responses.len()
    );
    for (it, &n) in c.interruptions.iter().zip(&c.plan) {
        let state = match (it.blocked, n) {
            (Some(b), _) => format!("not usable: {}", b.label()),
            (None, 0) => "left out (shared limit or cost)".to_owned(),
            (None, n) => format!("{n} use{}", if n > 1 { "s" } else { "" }),
        };
        let assumes = assumption_names(it.assumes);
        println!(
            "     {} ({}, {}): {} · {}{}",
            db.name(it.code),
            ["field", "GY", "banished", "hand"][it.place()],
            it.kind.name(),
            it.class.label(),
            state,
            if assumes.is_empty() { String::new() } else { format!(" · assumes {}", assumes.join(", ")) }
        );
    }
    for (card, code) in &c.floodgates {
        println!("     {}: floodgate (effect {code})", db.name(*card));
    }
}

fn interaction_json(p: &InteractionProfile) -> serde_json::Value {
    let [field, grave, banished, hand] = p.by_place;
    serde_json::json!({
        "value": p.value,
        "interruptions": p.total(),
        "negations": p.negations,
        "removals": p.removals,
        "others": p.others,
        "from": { "field": field, "grave": grave, "banished": banished, "hand": hand },
        "only_alone": p.shared,
        "activatable": p.activatable,
        "floodgates": p.floodgates,
        "cards_in_hand": p.hand,
        "cards_in_grave": p.grave,
    })
}

fn print_interaction(p: &InteractionProfile) {
    let [field, grave, banished, hand] = p.by_place;
    println!(
        "   interaction {:.2}: {} interruptions (negations {} · removals {} · other {}) · from field {field}, GY {grave}, banished {banished}, hand {hand}{} · {} cards usable on the opponent's turn · {} kinds of moves floodgated · left: {} cards in hand, {} in the GY",
        p.value,
        p.total(),
        p.negations,
        p.removals,
        p.others,
        if p.shared > 0 { format!(" · {} more only alone", p.shared) } else { String::new() },
        p.activatable,
        p.floodgates,
        p.hand,
        p.grave
    );
}

fn print_measure(engine: &Engine, m: &ygo_solver::Measure, took: std::time::Duration) {
    let db = engine.cards();
    println!(
        "   probe turn ({:.1} s): field {:.2} · hand {:.2} · floodgates {:.2} · resilience {:.2}",
        took.as_secs_f64(),
        m.field_value,
        m.hand_value,
        m.floodgate_value,
        m.resilience
    );
    if let Some(f) = m.follow_up {
        println!("   your next turn, after the worst breaker: {} effects to rebuild with · {} cards gained", f.rebuild, f.gained);
    }
    if !m.taken.is_empty() {
        let pile = |l: u8| m.taken.iter().filter(|t| t.1 == l).count();
        println!(
            "   taken from the opponent: {} cards (Extra Deck {}, Deck {}, hand {}) · attrition {:.2}",
            m.taken.len(),
            pile(location::EXTRA),
            pile(location::DECK),
            pile(location::HAND),
            m.attrition
        );
    }
    if let Some(kept) = m.countered {
        println!("   with an answer to your first interruption: {:.0}% of what your effects stop is still stopped", kept * 100.0);
    }
    for i in &m.interruptions {
        let covers: Vec<&str> = i.covers.iter().map(|p| p.label()).collect();
        let shared = if i.value < i.alone { format!(" (alone {:.2})", i.alone) } else { String::new() };
        println!(
            "     {} ({}): {}{} ×{} · coverage {:.0}% · {:.2}{shared}  [{}]",
            db.name(i.responder.code),
            ygo_engine::consts::location::name(i.location),
            i.effect.label(),
            if i.targeted { ", with a target" } else { "" },
            i.uses,
            i.coverage * 100.0,
            i.value,
            covers.join(", ")
        );
    }
    for p in &m.floodgates {
        println!("     floodgate: the opponent cannot {}", p.label());
    }
    for b in &m.breakers {
        let how = if b.not_applicable {
            "not applicable".to_owned()
        } else if let Some(r) = b.stopped_by {
            format!("negated by {} · {:.2} left", db.name(r.code), b.remaining)
        } else {
            format!("{:.2} left", b.remaining)
        };
        println!("     {}: {how}", b.name);
    }
}

fn total_weight(results: &[ygo_solver::HandResult]) -> f64 {
    results.iter().map(|r| r.weight).sum::<f64>().max(f64::EPSILON)
}

fn sweep(dir: &Path, config: &SweepConfig, show_script_errors: bool) -> Result<()> {
    let engine = load_engine(dir)?;
    let report = ygo_engine::sweep::sweep(&engine, config);
    let secs = report.elapsed.as_secs_f64();
    println!(
        "{} duels, {} decisions in {:.1} s ({:.0} decisions/s), {} threads",
        report.duels,
        report.decisions,
        secs,
        report.decisions as f64 / secs.max(1e-9),
        config.threads
    );
    for (kind, n) in &report.by_decision {
        println!("  {kind:<22} {n}");
    }
    println!("script errors logged by the engine: {} distinct", report.script_errors.len());
    if show_script_errors {
        for (line, n) in &report.script_errors {
            println!("  ×{n} {line}");
        }
    }
    for p in &report.problems {
        println!("{:?} in duel {} at decision {}: {}\n    {}", p.kind, p.duel, p.step, p.decision, p.detail);
    }
    if !report.problems.is_empty() {
        bail!("{} problems", report.problems.len());
    }
    println!("no problems");
    Ok(())
}

const SOURCES: &[(&str, &str)] =
    &[("scripts", "https://github.com/ProjectIgnis/CardScripts.git"), ("cdb", "https://github.com/ProjectIgnis/BabelCDB.git")];

fn fetch_data(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for (name, url) in SOURCES {
        let target = dir.join(name);
        let status = if target.join(".git").is_dir() {
            // Shallow clones cannot always fast-forward: fetch the latest
            // commit and move to it (the folder only holds downloaded data).
            println!("Updating {} ...", target.display());
            let fetched = Command::new("git").arg("-C").arg(&target).args(["fetch", "--depth", "1", "origin", "HEAD"]).status();
            match fetched {
                Ok(s) if s.success() => {
                    Command::new("git").arg("-C").arg(&target).args(["reset", "--hard", "--quiet", "FETCH_HEAD"]).status()
                }
                other => other,
            }
        } else {
            println!("Downloading {url} ...");
            Command::new("git").args(["clone", "--depth", "1", url]).arg(&target).status()
        }
        .context("cannot run git (is it installed?)")?;
        if !status.success() {
            bail!("git failed for {url}");
        }
    }
    info(dir)
}

fn update_cards(dir: &Path, force: bool) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let db_path = dir.join("cards.db");
    let current = if force { None } else { CardStore::open(&db_path).ok().and_then(|s| s.version()) };
    let new_db = dir.join("cards.db.new");
    let runtime = tokio::runtime::Runtime::new()?;
    let installed = runtime.block_on(async {
        let client = sync::http_client()?;
        sync::update_cards(&client, current.as_deref(), Some(&db_path), &new_db, &|p| {
            if !matches!(p.stage, sync::Stage::Downloading) || p.total == Some(p.done) {
                println!("{:?} {}", p.stage, p.item);
            }
        })
        .await
    })?;
    match installed {
        Some(version) => {
            sync::install_file(&new_db, &db_path)?;
            println!("Card database {version}: {} cards", CardStore::open(&db_path)?.count()?);
        }
        None => println!("Card database already up to date ({})", current.unwrap_or_default()),
    }
    Ok(())
}

fn search(dir: &Path, text: &str, limit: usize) -> Result<()> {
    let store = CardStore::open(&dir.join("cards.db"))?;
    let parsed = ygo_data::parse_query(text);
    for chip in &parsed.chips {
        println!("filter: {}", chip.label);
    }
    for bad in &parsed.errors {
        println!("not understood: {bad}");
    }
    let started = Instant::now();
    let ids = store.search(&parsed.query)?;
    let elapsed = started.elapsed();
    for card in store.get_many(&ids[..ids.len().min(limit)])? {
        let stats = match (card.atk, card.def, card.link_rating) {
            (atk, _, Some(link)) => format!(" ATK {} LINK-{link}", atk.map_or("?".into(), |a| a.to_string())),
            (Some(atk), def, None) => format!(" ATK {atk} DEF {}", def.map_or("?".into(), |d| d.to_string())),
            _ => String::new(),
        };
        println!("{:>9}  {} [{}]{stats}", card.id, card.name, card.human_type);
    }
    println!("{} results in {:.1} ms", ids.len(), elapsed.as_secs_f64() * 1000.0);
    Ok(())
}

/// Searches every app screen starts from; the deck's archetypes are added to them.
const FIXTURE_QUERIES: &[&str] =
    &["", "format:md sort:popular", "type:monster", "type:spell", "type:trap", "attr:dark", "atk>=2500", "tag:handtrap", "\"negate\""];

/// Results kept per recorded search: the mock catalog needs every card it shows, and the file stays small.
const FIXTURE_RESULTS: usize = 48;

/// Parses `role=id,id,...` arguments into the deck's roles.
fn parse_roles(args: &[String]) -> Result<std::collections::BTreeMap<String, Vec<u32>>> {
    let mut roles = std::collections::BTreeMap::<String, Vec<u32>>::new();
    for arg in args {
        let (name, ids) = arg.split_once('=').with_context(|| format!("role {arg:?}: expected name=id,id,..."))?;
        let ids = ids
            .split(',')
            .map(|id| id.trim().parse::<u32>().with_context(|| format!("role {name}: bad id {id:?}")))
            .collect::<Result<Vec<_>>>()?;
        roles.entry(name.trim().to_owned()).or_default().extend(ids);
    }
    Ok(roles)
}

/// What the backend answers for a deck, in the shapes of `api.ts` and without any card text (Konami's, never committed).
fn recorded_fixtures(
    store: &CardStore,
    deck: &ygo_data::deck::Sections,
    extra_queries: &[String],
    roles: &std::collections::BTreeMap<String, Vec<u32>>,
) -> Result<serde_json::Value> {
    let (main, extra, side) = deck;
    let all: Vec<u32> = main.iter().chain(extra).chain(side).copied().collect();
    let deck_cards = store.get_many(&all)?;
    let mut archetypes: Vec<String> = deck_cards.iter().filter_map(|c| c.archetype.clone()).collect();
    archetypes.sort();
    archetypes.dedup();
    let mut queries: Vec<String> = FIXTURE_QUERIES.iter().map(|q| q.to_string()).collect();
    queries.extend(archetypes.iter().map(|a| format!("arch:\"{a}\"")));
    queries.extend(extra_queries.iter().cloned());
    let mut seen = std::collections::HashSet::new();
    queries.retain(|q| seen.insert(q.clone()));
    let mut searches = serde_json::Map::new();
    let mut shown: Vec<u32> = all.clone();
    for text in queries {
        let parsed = ygo_data::parse_query(&text);
        let mut ids = store.search(&parsed.query)?;
        ids.truncate(FIXTURE_RESULTS);
        shown.extend(&ids);
        searches.insert(text, serde_json::json!({ "ids": ids, "chips": parsed.chips, "errors": parsed.errors }));
    }
    shown.sort_unstable();
    shown.dedup();
    // Names, types and statuses only: the texts stay out of the repository.
    let cards: Vec<ygo_data::Card> = store
        .get_many(&shown)?
        .into_iter()
        .map(|mut c| {
            c.desc.clear();
            c.pend_desc = None;
            c
        })
        .collect();
    let format = ygo_data::Format::Md;
    let sample = ygo_data::Deck {
        name: "Deck registrato".into(),
        format,
        main: main.clone(),
        extra: extra.clone(),
        side: side.clone(),
        ..Default::default()
    };
    let by_id: HashMap<u32, ygo_data::Card> = deck_cards.iter().map(|c| (c.id, c.clone())).collect();
    let mut legality = ygo_data::deck::validate(&sample, &by_id);
    legality.banlist = store.banlist_date("md");
    // The handtrap role as the app's tag knows it, plus the roles given.
    let mut roles = roles.clone();
    let handtraps: Vec<u32> =
        deck_cards.iter().filter(|c| ygo_data::Tag::Handtrap.names().contains(&c.name.as_str())).map(|c| c.id).collect();
    roles.entry(ygo_stats::roles::HANDTRAP.to_owned()).or_default().extend(handtraps);
    for ids in roles.values_mut() {
        ids.sort_unstable();
        ids.dedup();
    }
    let overview = ygo_stats::overview(&ygo_stats::DeckModel::from_ids(main, &roles));
    Ok(serde_json::json!({
        "deck": { "main": main, "extra": extra, "side": side, "roles": roles },
        "searches": searches,
        "cards": cards,
        "legality": legality,
        "overview": overview,
        "facets": store.facets()?,
        "banChanges": store.ban_changes(Some("md"))?,
    }))
}

fn record_fixtures(dir: &Path, deck: &Path, out: &Path, extra_queries: &[String], roles: &[String]) -> Result<()> {
    let store = CardStore::open(&dir.join("cards.db"))?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("reading {}", deck.display()))?;
    let sections = ygo_data::deck::parse_any(&text)?;
    let json = recorded_fixtures(&store, &sections, extra_queries, &parse_roles(roles)?)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, serde_json::to_string_pretty(&json)? + "\n")?;
    println!(
        "wrote {} ({} searches, {} cards)",
        out.display(),
        json["searches"].as_object().map_or(0, |m| m.len()),
        json["cards"].as_array().map_or(0, |a| a.len())
    );
    Ok(())
}

/// Where `links` writes the map, and the hands searched for it.
struct LinksOut<'a> {
    json: Option<&'a Path>,
    hands_json: Option<&'a Path>,
}

/// The hands and what each of their lines brings into play, as JSON.
fn hands_reach_json(db: &ygo_engine::CardDb, reached: &[ygo_solver::HandReach]) -> serde_json::Value {
    let hand = |h: &ygo_solver::HandReach| h.hand.iter().map(|&c| db.canonical_code(c)).collect::<Vec<_>>();
    serde_json::json!(reached.iter().map(|h| serde_json::json!({ "hand": hand(h), "lines": h.lines })).collect::<Vec<_>>())
}

/// Searches `count` random opening hands and writes what their lines
/// bring into play.
fn links_truth(dir: &Path, deck: &Path, nodes: u64, count: usize, seed: u64, threads: Option<usize>, json: &Path) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let budget = ygo_solver::LinksBudget { nodes_per_hand: nodes, threads, ..ygo_solver::LinksBudget::default() };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let started = std::time::Instant::now();
    let hands = ygo_solver::random_hands(&deck.main, count, seed);
    let reached = ygo_solver::hands_reach(&engine, &deck, &hands, 1, budget, &cancel, |done, total| eprint!("\r{done}/{total} hands"));
    eprintln!();
    let db = engine.cards();
    let doc = serde_json::json!({
        "main": deck.main.iter().map(|&c| db.canonical_code(c)).collect::<Vec<_>>(),
        "extra": deck.extra.iter().map(|&c| db.canonical_code(c)).collect::<Vec<_>>(),
        "seconds": started.elapsed().as_secs_f64(),
        "hands": hands_reach_json(db, &reached),
    });
    std::fs::write(json, serde_json::to_string(&doc)?)?;
    println!("{} hands in {:.1} s", reached.len(), started.elapsed().as_secs_f64());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn links(
    dir: &Path,
    deck: &Path,
    nodes: u64,
    hands: usize,
    threads: Option<usize>,
    out: LinksOut,
    after: Option<&Path>,
    diff: bool,
) -> Result<()> {
    let engine = load_engine(dir)?;
    let read = |path: &Path| -> Result<DeckList> {
        let text = std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        Ok(DeckList::from_ydk(&text)?)
    };
    let deck = read(deck)?;
    let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let budget = ygo_solver::LinksBudget { hands_per_card: hands, nodes_per_hand: nodes, threads, ..ygo_solver::LinksBudget::default() };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    // The earlier version first, its work kept for the deck's.
    let earlier = match after {
        Some(path) => {
            let started = std::time::Instant::now();
            let read = ygo_solver::deck_links_again(&engine, &read(path)?, 1, budget, &cancel, |_, _, _| {}, None);
            eprintln!("earlier version read in {:.1} s", started.elapsed().as_secs_f64());
            Some(read)
        }
        None => None,
    };
    let started = std::time::Instant::now();
    let progress = |done, total, map: &ygo_solver::DeckLinks| {
        if done == 0 {
            eprintln!(
                "from the text, at once: {} links, {total} hands to search ({:.1} s)",
                map.links.len(),
                started.elapsed().as_secs_f64()
            );
        }
        eprint!("\r{done}/{total} hands");
    };
    let (map, work) = ygo_solver::deck_links_again(&engine, &deck, 1, budget, &cancel, progress, earlier.as_ref().map(|(_, work)| work));
    eprintln!();
    if let Some(path) = out.hands_json {
        std::fs::write(path, serde_json::to_string(&hands_reach_json(engine.cards(), &work.reached()))?)?;
    }
    let db = engine.cards();
    if let (true, Some((old, _))) = (diff, &earlier) {
        print_links_diff(db, &ygo_solver::compare_links(old, &map), started.elapsed().as_secs_f64());
        return Ok(());
    }
    println!(
        "{} cards, {} links ({} used) from {} hands and {} lines in {:.1} s",
        map.cards.len(),
        map.links.len(),
        map.links.iter().filter(|l| l.used > 0.0).count(),
        map.hands,
        map.lines,
        started.elapsed().as_secs_f64()
    );
    let mut cards = map.cards.clone();
    cards.sort_by(|a, b| b.reach_all.cmp(&a.reach_all).then(b.reach.cmp(&a.reach)));
    for c in &cards {
        println!(
            "{:<45} x{} {} reach {:>2} / {:>2} · reached by an opening hand {:>3.0}%{} · negated, {:.2} cards fewer reached",
            db.name(c.code),
            c.copies,
            if c.extra { "extra" } else { "main " },
            c.reach,
            c.reach_all,
            c.chance * 100.0,
            if c.measured { " (measured)" } else { "" },
            c.key
        );
        for l in map.links.iter().filter(|l| l.from == c.code) {
            let used = if l.used > 0.0 { format!(" used {:.0}%", l.used * 100.0) } else { String::new() };
            let how = match (l.possible, l.text) {
                (true, true) => "",
                (true, false) => " (not in the text)",
                (false, true) => " (text only)",
                (false, false) => " (seen used only)",
            };
            println!("    {:<9} {:<8} {}{how}{used}", l.kind.key(), l.zone.key(), db.name(l.to));
        }
    }
    if let Some(path) = out.json {
        let doc = serde_json::json!({
            "hands": map.hands,
            "lines": map.lines,
            "cards": map.cards.iter().map(|c| serde_json::json!({
                "code": c.code, "name": db.name(c.code), "copies": c.copies, "extra": c.extra,
                "reach": c.reach, "reachAll": c.reach_all, "chance": c.chance, "measured": c.measured, "key": c.key,
            })).collect::<Vec<_>>(),
            "links": map.links.iter().map(|l| serde_json::json!({
                "from": l.from, "to": l.to, "kind": l.kind.key(), "zone": l.zone.key(), "possible": l.possible, "used": l.used,
                "text": l.text,
            })).collect::<Vec<_>>(),
        });
        std::fs::write(path, serde_json::to_string_pretty(&doc)?)?;
    }
    Ok(())
}

fn print_links_diff(db: &ygo_engine::CardDb, diff: &ygo_solver::LinksDiff, seconds: f64) {
    println!("read in {seconds:.1} s; an opening hand reaches {:.2} cards on average, {:.2} before", diff.reached.1, diff.reached.0);
    let names = |codes: &[u32]| codes.iter().map(|&c| db.name(c)).collect::<Vec<_>>().join(", ");
    println!("added: {}", names(&diff.added));
    println!("removed: {}", names(&diff.removed));
    for (title, links) in [("gained", &diff.gained), ("lost", &diff.lost)] {
        println!("{} links {title}", links.len());
        for l in links.iter() {
            println!("    {:<40} {:<9} {:<8} {}", db.name(l.from), l.kind.key(), l.zone.key(), db.name(l.to));
        }
    }
    println!("{} cards changed", diff.changed.len());
    for c in &diff.changed {
        println!(
            "    {:<45} reached {:>3.0}% -> {:>3.0}% · key {:.2} -> {:.2} · reach {} -> {}",
            db.name(c.code),
            c.chance.0 * 100.0,
            c.chance.1 * 100.0,
            c.key.0,
            c.key.1,
            c.reach_all.0,
            c.reach_all.1
        );
    }
}

fn rival_plays(dir: &Path, deck: &Path, hands: usize, nodes: u64, seed: u64) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let started = std::time::Instant::now();
    let plays = ygo_solver::rival_plays(&engine, &deck, seed, hands, nodes, threads);
    println!("{} moves from {hands} hands in {:.1} s", plays.len(), started.elapsed().as_secs_f64());
    for p in plays.iter().take(40) {
        println!(
            "{:>4.0}% of lines ({:>3.0}% first, at {:>4.1}) {:?} {} from {}",
            p.share * 100.0,
            p.first * 100.0,
            p.order,
            p.kind,
            engine.cards().name(p.code),
            ygo_engine::consts::location::name(p.location)
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn break_board(
    dir: &Path,
    deck: &Path,
    vs: &Path,
    hands: usize,
    rival_hands: usize,
    rival_nodes: u64,
    config: &ygo_solver::BreakConfig,
    json: Option<&Path>,
) -> Result<()> {
    let engine = load_engine(dir)?;
    let read = |path: &Path| -> Result<DeckList> {
        let text = std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        Ok(DeckList::from_ydk(&text)?)
    };
    let (mine, theirs) = (read(deck)?, read(vs)?);
    let db = engine.cards();
    let names = |codes: &[u32]| codes.iter().map(|&c| db.name(c)).collect::<Vec<_>>().join(", ");
    let kit = ygo_solver::Kit::standard(&engine);
    let started = std::time::Instant::now();
    let rival = ygo_solver::rival_board(&engine, &theirs, config.seed, rival_hands, rival_nodes, config.threads, &kit)
        .context("no hand of the opponent's deck makes a board")?;
    println!("opponent's board ({:.1} s), value {:.2}, from {}", started.elapsed().as_secs_f64(), rival.value, names(&rival.hand));
    for (code, worth) in &rival.worth {
        println!("   {:<40} worth {worth:.2}", db.name(*code));
    }
    let answers: Vec<u32> = if config.answers { rival.responders.iter().map(|r| r.0).collect() } else { Vec::new() };
    println!("it answers with: {}", if answers.is_empty() { "nothing".to_owned() } else { names(&answers) });
    // The breakers of your deck the engine can play, to tell the hands apart.
    let breakers = ygo_solver::Rival::new(&engine, &mine, config.seed, &kit).breakers().to_vec();
    println!("your breakers: {}", if breakers.is_empty() { "none".to_owned() } else { names(&breakers) });
    let mut rows = Vec::new();
    let (mut with, mut without) = (Vec::new(), Vec::new());
    for hand in ygo_solver::second::random_hands(&mine.main, 6, hands, config.seed) {
        let at = std::time::Instant::now();
        let broken = ygo_solver::break_board(&engine, &mine, &theirs, &hand, &rival, &kit, config)?;
        let count = hand.iter().filter(|c| breakers.iter().any(|b| db.canonical_code(*b) == db.canonical_code(**c))).count();
        if count > 0 { &mut with } else { &mut without }.push(broken.share);
        println!(
            "{:>4.0}% left  {:>4} LP  {} breaker(s)  {:>5.1} s  {:>6} nodes  {}\n      kept: {}",
            broken.share * 100.0,
            -broken.damage,
            count,
            at.elapsed().as_secs_f64(),
            broken.nodes,
            names(&hand),
            names(&broken.kept)
        );
        rows.push(serde_json::json!({
            "hand": hand, "breakers": count, "share": broken.share, "left": broken.left,
            "kept": broken.kept, "damage": broken.damage, "line": broken.line, "nodes": broken.nodes,
        }));
    }
    let mean = |v: &[f64]| if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 };
    println!(
        "left on average: {:.0}% with a breaker ({} hands), {:.0}% without ({} hands); {:.1} s in all",
        mean(&with) * 100.0,
        with.len(),
        mean(&without) * 100.0,
        without.len(),
        started.elapsed().as_secs_f64()
    );
    if let Some(path) = json {
        let out = serde_json::json!({
            "rival": { "hand": rival.hand, "value": rival.value, "worth": rival.worth, "responders": answers },
            "hands": rows,
        });
        std::fs::write(path, serde_json::to_string_pretty(&out)?)?;
    }
    Ok(())
}

/// Decks of the meta `tech-table` measures against by default.
const TECH_DECKS: usize = 6;

fn tech_table(dir: &Path, deck: &Path, count: usize, seed: u64, meta: MetaFinal, json: Option<&Path>) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let decks = ygo_solver::most_played(&load_meta(dir, &meta)?, count);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let kit = ygo_solver::Kit::standard(&engine);
    let started = std::time::Instant::now();
    let rivals = ygo_solver::meta_rivals(&engine, &decks, seed, threads);
    let read = started.elapsed().as_secs_f64();
    let table = ygo_solver::tech_table(&engine, &deck, &rivals, seed, &kit, threads);
    let db = engine.cards();
    println!("lines read in {read:.1} s, played again in {:.1} s", started.elapsed().as_secs_f64() - read);
    for d in &table.decks {
        println!("  {:<28} weight {:>5.1}  {} lines, {} measured", d.name, d.weight, d.lines, d.measured);
    }
    println!();
    for r in &table.rows {
        let per: Vec<String> = r.decks.iter().map(|a| format!("{}/{}", a.stopped, a.usable)).collect();
        let at: Vec<String> = r.at.iter().take(3).map(|(c, n)| format!("{} ×{n}", db.name(*c))).collect();
        println!(
            "{:>4.0}% taken {:>4.0}% of lines  {:<32} {}+{}  in hand {:>3.0}%  step {:<4}  [{}]  at {}",
            r.lost * 100.0,
            r.value * 100.0,
            db.name(r.code),
            r.main,
            r.side,
            r.chance * 100.0,
            r.step.map_or("-".to_owned(), |s| format!("{:.1}", s)),
            per.join(" "),
            at.join(", ")
        );
    }
    if let Some(path) = json {
        let rows: Vec<serde_json::Value> = table
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "code": r.code, "name": db.name(r.code), "main": r.main, "side": r.side, "chance": r.chance,
                    "value": r.value, "lost": r.lost, "step": r.step,
                    "decks": r.decks.iter().map(|a| serde_json::json!({ "usable": a.usable, "stopped": a.stopped, "lost": a.lost })).collect::<Vec<_>>(),
                    "at": r.at.iter().map(|(c, n)| serde_json::json!({ "code": c, "name": db.name(*c), "lines": n })).collect::<Vec<_>>(),
                })
            })
            .collect();
        let decks: Vec<serde_json::Value> = table
            .decks
            .iter()
            .map(|d| serde_json::json!({ "name": d.name, "weight": d.weight, "lines": d.lines, "measured": d.measured }))
            .collect();
        std::fs::write(path, serde_json::to_string_pretty(&serde_json::json!({ "seed": seed, "decks": decks, "rows": rows }))?)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn anatomy(
    dir: &Path,
    cards: &[String],
    deck: Option<&Path>,
    all: bool,
    link: u32,
    seed: u64,
    json: Option<&Path>,
    out: Option<&Path>,
) -> Result<()> {
    use ygo_engine::consts::event;
    let engine = load_engine(dir)?;
    let db = engine.cards();
    let mut codes = Vec::new();
    for name in cards {
        let code = match name.parse::<u32>() {
            Ok(code) => code,
            Err(_) => db.find_by_name(name).with_context(|| format!("no card named {name}"))?.code,
        };
        codes.push(code);
    }
    let list = match deck {
        Some(path) => Some(DeckList::from_ydk(&std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?)?),
        None => None,
    };
    if let Some(list) = &list {
        codes.extend(list.main.iter().chain(&list.extra).map(|&c| db.canonical_code(c)));
    }
    if all {
        codes.extend(db.iter().filter(|c| c.alias == 0 && c.type_ & ygo_engine::consts::card_type::TOKEN == 0).map(|c| c.code));
        codes.sort_unstable();
    }
    let mut seen = std::collections::HashSet::new();
    codes.retain(|c| seen.insert(*c));
    let started = std::time::Instant::now();
    let read = ygo_engine::read_anatomy(&engine, &codes);
    let elapsed = started.elapsed().as_secs_f64();
    let verbose = !all;
    let mut lines = Vec::new();
    let (mut effects, mut activated, mut with_category, mut errors) = (0, 0, 0, 0);
    for card in &read {
        effects += card.effects.len();
        activated += card.effects.iter().filter(|e| e.kind.activated()).count();
        with_category += card.effects.iter().filter(|e| e.kind.activated() && e.category != 0).count();
        if card.error.is_some() {
            errors += 1;
        }
        if !verbose {
            continue;
        }
        println!("{} ({})", db.name(card.code), card.code);
        if let Some(error) = &card.error {
            println!("  error: {error}");
        }
        for (k, e) in card.effects.iter().enumerate() {
            let mut parts = vec![e.kind.name().to_owned()];
            if e.kind.activated() {
                if let Some(when) = event::name(e.code) {
                    parts.push(format!("when {when}"));
                }
            }
            let from = ygo_engine::consts::location::names(e.range);
            if !from.is_empty() {
                parts.push(format!("from {from}"));
            }
            if let Some(limit) = e.count_limit {
                let shared: Vec<_> = card
                    .effects
                    .iter()
                    .enumerate()
                    .filter(|&(j, o)| j != k && e.shares_limit(o))
                    .map(|(j, _)| (j + 1).to_string())
                    .collect();
                let shared = if shared.is_empty() { String::new() } else { format!(", shared with {}", shared.join(", ")) };
                parts.push(format!("{}× per turn{shared}", limit.count));
            }
            if !e.categories().is_empty() {
                parts.push(e.categories().join(", "));
            }
            let steps: Vec<_> = [(e.has_condition, "condition"), (e.has_cost, "cost"), (e.targets(), "targets")]
                .iter()
                .filter(|(on, _)| *on)
                .map(|(_, name)| *name)
                .collect();
            if !steps.is_empty() {
                parts.push(steps.join(" + "));
            }
            let text = if e.description != 0 { db.describe(e.description) } else { String::new() };
            println!("  {}. {}{}", k + 1, parts.join(" · "), if text.is_empty() { String::new() } else { format!("  «{text}»") });
        }
    }
    let share = |n: usize, of: usize| if of == 0 { 0.0 } else { 100.0 * n as f64 / of as f64 };
    lines.push(format!(
        "cards: {}, read without errors: {} ({:.1}%)",
        read.len(),
        read.len() - errors,
        share(read.len() - errors, read.len())
    ));
    lines.push(format!(
        "effects: {effects}, activated: {activated}, activated with a category: {with_category} ({:.1}%)",
        share(with_category, activated)
    ));
    lines.push(format!("time: {elapsed:.1} s ({:.2} ms per card)", 1000.0 * elapsed / read.len().max(1) as f64));
    let mut by_error: std::collections::BTreeMap<String, usize> = Default::default();
    for card in read.iter().filter_map(|c| c.error.as_ref()) {
        let kind: String = card.split(':').next().unwrap_or_default().chars().take(60).collect();
        *by_error.entry(kind).or_default() += 1;
    }
    let mut by_error: Vec<_> = by_error.into_iter().collect();
    by_error.sort_by_key(|e| std::cmp::Reverse(e.1));
    for (kind, n) in by_error.iter().take(3) {
        lines.push(format!("error ×{n}: {kind}"));
    }
    if let (Some(list), true) = (&list, link > 0) {
        let (offered, linked, undescribed) = link_activations(&engine, list, &read, link, seed);
        lines.push(format!(
            "activations offered in {link} random turns: {offered}, linked to their effect: {linked} ({:.1}%), without a description: {undescribed}",
            share(linked, offered)
        ));
    }
    for line in &lines {
        println!("{line}");
    }
    if let Some(out) = out {
        std::fs::create_dir_all(out)?;
        std::fs::write(out.join("summary.txt"), lines.join("\n") + "\n")?;
    }
    if let Some(path) = json {
        let cards: Vec<_> = read
            .iter()
            .map(|c| {
                serde_json::json!({
                    "code": c.code,
                    "name": db.name(c.code),
                    "error": c.error,
                    "effects": c.effects.iter().map(|e| serde_json::json!({
                        "kind": e.kind.name(),
                        "type": e.type_,
                        "code": e.code,
                        "event": if e.kind.activated() { event::name(e.code) } else { None },
                        "categories": e.categories(),
                        "range": e.range,
                        "targetRange": [e.target_range.0, e.target_range.1],
                        "countLimit": e.count_limit.map(|l| serde_json::json!({"count": l.count, "code": l.code, "flag": l.flag, "index": l.index})),
                        "targets": e.targets(),
                        "delayed": e.delayed(),
                        "description": e.description,
                        "text": if e.description != 0 { db.describe(e.description) } else { String::new() },
                        "value": e.value,
                        "cost": e.has_cost,
                        "condition": e.has_condition,
                        "target": e.has_target,
                        "operation": e.has_operation,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        std::fs::write(path, serde_json::to_string_pretty(&cards)?)?;
    }
    Ok(())
}

/// Random turns from `deck`: the activations offered (with a description of
/// one of the deck's cards), those whose effect the anatomy finds, and those
/// without a description.
fn link_activations(engine: &Engine, deck: &DeckList, read: &[ygo_engine::CardAnatomy], turns: u32, seed: u64) -> (usize, usize, usize) {
    let by_code: HashMap<u32, &ygo_engine::CardAnatomy> = read.iter().map(|c| (c.code, c)).collect();
    let (mut offered, mut linked, mut undescribed) = (0, 0, 0);
    let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    let mut below = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n.max(1) as u64) as usize
    };
    for turn in 0..turns {
        let mut main = deck.main.clone();
        ygo_engine::shuffle(&mut main, seed ^ turn as u64);
        let Ok(mut duel) = Duel::goldfish(engine, deck, &main[..5.min(main.len())], seed ^ turn as u64) else { continue };
        for _ in 0..200 {
            let Ok(Step::Decision { decision, .. }) = duel.advance() else { break };
            let list = duel.actions(&decision);
            if list.is_empty() {
                break;
            }
            for a in list
                .iter()
                .filter(|a| matches!(a.kind, ActionKind::Activate | ActionKind::Yes) && a.card.is_some_and(|c| c.controller == 0))
            {
                let owner = engine.cards().canonical_code(a.card.map_or(0, |c| c.code));
                let Some(card) = by_code.get(&owner) else { continue };
                offered += 1;
                if a.description == 0 || (a.description >> 20) as u32 != owner {
                    undescribed += 1;
                }
                let described = by_code.get(&((a.description >> 20) as u32)).copied().unwrap_or(card);
                if described.effect_of(a.description).is_some() {
                    linked += 1;
                }
            }
            let rest: Vec<_> = list.iter().filter(|a| a.kind != ActionKind::EndTurn).collect();
            let action = if rest.is_empty() { &list[0] } else { rest[below(rest.len())] };
            duel.respond(&action.response);
        }
    }
    (offered, linked, undescribed)
}

fn hand_traps(dir: &Path, deck: &Path) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let kit = ygo_solver::Kit::standard(&engine);
    for code in ygo_solver::hand_traps(&engine, &deck, 1, &kit) {
        println!("{code}\t{}", engine.cards().name(code));
    }
    Ok(())
}

/// The kinds of play of the probe turn's kit in one batch of events of your
/// turn. `landed`: your monsters summoned since your last free moment (an
/// effect of theirs is a summon trigger, not one of a monster already there).
fn plays(db: &ygo_engine::CardDb, events: &[Event], landed: &mut Vec<u32>, mut count: impl FnMut(ygo_solver::Probe)) {
    use ygo_solver::Probe;
    let monster = |code: u32| db.get(code).is_some_and(|c| c.type_ & 0x1 != 0);
    for e in events {
        match e {
            Event::Chaining { card, .. } if card.controller == 0 => match card.location {
                location::HAND if monster(card.code) => count(Probe::HandEffect),
                location::HAND | location::SZONE => count(Probe::Spell),
                location::GRAVE | location::REMOVED => count(Probe::GraveEffect),
                location::MZONE if landed.contains(&db.canonical_code(card.code)) => count(Probe::FieldEffect),
                location::MZONE => count(Probe::FieldIgnition),
                _ => {}
            },
            Event::NormalSummoning { card } if card.controller == 0 => {
                count(Probe::NormalSummon);
                landed.push(db.canonical_code(card.code));
            }
            Event::Move { code, from, to, reason } if from.controller == 0 && to.controller == 0 => match (from.location, to.location) {
                (location::DECK, location::HAND) => count(Probe::Search),
                (location::DECK, location::GRAVE) if monster(*code) => count(Probe::SendToGrave),
                (from, location::MZONE) if reason & 0x800 != 0 => {
                    landed.push(db.canonical_code(*code));
                    match from {
                        location::HAND => count(Probe::SpecialFromHand),
                        location::DECK => count(Probe::DeckSummon),
                        location::EXTRA => count(Probe::ExtraSummon),
                        _ => {}
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// Where an activation of yours comes from, and whether its resolution
/// touched your Main Deck (added, drew, summoned or sent from it): what
/// "stop anything that touches the Deck" handtraps can answer.
#[derive(Default)]
struct DeckTouch {
    /// Chain link -> kind of activation and whether it touched the Deck.
    links: std::collections::HashMap<u32, (&'static str, bool)>,
    solving: Option<u32>,
    counts: std::collections::BTreeMap<(&'static str, bool), u64>,
}

impl DeckTouch {
    fn read(&mut self, db: &ygo_engine::CardDb, events: &[Event], landed: &[u32]) {
        let monster = |code: u32| db.get(code).is_some_and(|c| c.type_ & 0x1 != 0);
        for e in events {
            match e {
                Event::Chaining { card, chain_link, .. } if card.controller == 0 => {
                    let kind = match card.location {
                        location::HAND if monster(card.code) => "monster from the hand",
                        location::HAND | location::SZONE => "Spell or Trap",
                        location::GRAVE | location::REMOVED => "from the GY or banished",
                        location::MZONE if landed.contains(&db.canonical_code(card.code)) => "trigger of a summoned monster",
                        location::MZONE => "monster on the field",
                        _ => continue,
                    };
                    self.links.insert(*chain_link, (kind, false));
                }
                Event::ChainSolving { chain_link } => self.solving = Some(u32::from(*chain_link)),
                Event::Move { from, .. } if from.controller == 0 && from.location == location::DECK => self.touch(),
                Event::Draw { player: 0, .. } => self.touch(),
                Event::ChainSolved { chain_link } => {
                    if let Some(link) = self.links.remove(&u32::from(*chain_link)) {
                        *self.counts.entry(link).or_default() += 1;
                    }
                    self.solving = None;
                }
                Event::ChainNegated { chain_link } | Event::ChainDisabled { chain_link } => {
                    self.links.remove(&u32::from(*chain_link));
                }
                Event::ChainEnd => {
                    self.links.clear();
                    self.solving = None;
                }
                _ => {}
            }
        }
    }

    fn touch(&mut self) {
        if let Some(link) = self.solving.and_then(|l| self.links.get_mut(&l)) {
            link.1 = true;
        }
    }
}

fn kit_mix(dir: &Path, files: &[PathBuf], top: usize, seed: u64) -> Result<()> {
    use ygo_solver::Probe;
    let engine = Engine::load(dir)?;
    let db = engine.cards();
    let kit = ygo_solver::Kit::standard(&engine);
    let mut counts: std::collections::BTreeMap<Probe, u64> = Probe::ALL.iter().map(|&p| (p, 0)).collect();
    let mut lines = 0;
    let mut touch = DeckTouch::default();
    for file in files {
        let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
        let deck = DeckList::from_ydk(&std::fs::read_to_string(saved["deck"].as_str().context("no deck in the file")?)?)?;
        let hand: Vec<u32> =
            saved["hand"].as_array().map(|h| h.iter().filter_map(|c| c.as_u64().map(|c| c as u32)).collect()).unwrap_or_default();
        for item in saved["items"].as_array().context("no lines in the file")?.iter().take(top) {
            let line = line_from_json(item)?;
            let mut duel = Duel::goldfish_against(&engine, &deck, &hand, seed, Some(kit.opponent()))?;
            let mut landed = Vec::new();
            for answer in &line.answers {
                let Step::Decision { decision, events } = duel.advance()? else { break };
                plays(db, &events, &mut landed, |p| *counts.entry(p).or_default() += 1);
                touch.read(db, &events, &landed);
                if matches!(decision, ygo_engine::Decision::IdleCmd { player: 0, .. }) {
                    landed.clear();
                }
                duel.respond(answer);
            }
            lines += 1;
        }
    }
    let total: u64 = counts.values().sum();
    println!("{lines} lines, {total} plays");
    // How often a line summons from the Extra Deck (the kit does it once).
    let extra = counts.get(&Probe::ExtraSummon).copied().unwrap_or(0);
    println!("summons from the Extra Deck: {:.2} a line", extra as f64 / lines.max(1) as f64);
    println!("{:<62} {:>6} {:>6} {:>7}", "probe", "plays", "share", "weight");
    for (p, n) in &counts {
        let share = *n as f64 / total.max(1) as f64;
        println!("{:<62} {:>6} {:>6.3} {:>7.2}", p.label(), n, share, p.weight());
    }
    let activations: u64 = touch.counts.values().sum();
    println!("\n{activations} activations resolved");
    println!("{:<32} {:>8} {:>8} {:>6}", "activation", "Deck", "no Deck", "share");
    let kinds: Vec<&str> = touch.counts.keys().map(|(k, _)| *k).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    for kind in kinds {
        let with = touch.counts.get(&(kind, true)).copied().unwrap_or(0);
        let without = touch.counts.get(&(kind, false)).copied().unwrap_or(0);
        println!("{kind:<32} {with:>8} {without:>8} {:>6.3}", (with + without) as f64 / activations.max(1) as f64);
    }
    let with: u64 = touch.counts.iter().filter(|((_, t), _)| *t).map(|(_, n)| n).sum();
    println!("touching the Main Deck: {:.3}", with as f64 / activations.max(1) as f64);
    Ok(())
}

fn card_values(dir: &Path, deck: &Path) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let kit = ygo_solver::Kit::standard(&engine);
    let started = std::time::Instant::now();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let hand_traps = ygo_solver::hand_traps(&engine, &deck, 1, &kit);
    let values = ygo_solver::card_values(&engine, &deck, 1, &kit, &hand_traps, threads);
    for (code, spot, value) in values.list() {
        // Dead: usable with an empty field, not after a combo (the Mulcharmy).
        let dead = spot == ygo_solver::Spot::HandWithField
            && values.usable(&engine, code, ygo_solver::Spot::Hand)
            && !values.usable(&engine, code, spot);
        println!("{value:.2}\t{spot:?}\t{}{}", engine.cards().name(code), if dead { "\t(cannot be used: a free discard)" } else { "" });
    }
    println!("measured in {:.1} s", started.elapsed().as_secs_f64());
    Ok(())
}

/// Not played from the hand on your own turn: the well-known handtraps (an
/// archetype's quick effects from the hand are often combo pieces too).
fn known_hand_traps(engine: &Engine) -> Vec<u32> {
    ygo_data::Tag::Handtrap.names().iter().filter_map(|n| engine.cards().find_by_name(n)).map(|c| c.code).collect()
}

fn roles(dir: &Path, deck: &Path) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let kit = ygo_solver::Kit::standard(&engine);
    let started = std::time::Instant::now();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let hand_traps = ygo_solver::hand_traps(&engine, &deck, 1, &kit);
    let values = std::sync::Arc::new(ygo_solver::card_values(&engine, &deck, 1, &kit, &hand_traps, threads));
    let guesses = ygo_solver::guess_roles(&engine, &deck, &kit, &hand_traps, &known_hand_traps(&engine), values, threads, |_, _| {});
    for g in &guesses {
        println!("{:<12} {:<45} {}", g.roles.join(","), engine.cards().name(g.code), g.reasons.join("; "));
    }
    println!("found in {:.1} s", started.elapsed().as_secs_f64());
    Ok(())
}

fn coverage(dir: &Path, deck: &Path, nodes: u64, threads: Option<usize>, seed: u64, all: bool) -> Result<()> {
    let engine = load_engine(dir)?;
    let text = std::fs::read_to_string(deck).with_context(|| format!("cannot read {}", deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let db = engine.cards();
    let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let started = Instant::now();
    // The search as the Combo Lab sets it up.
    let kit = ygo_solver::Kit::standard(&engine);
    let mut base = ygo_solver::SearchConfig::new(deck.clone(), Vec::new());
    base.seed = seed;
    base.threads = 1;
    base.max_nodes = nodes;
    base.time_limit = std::time::Duration::from_secs(600);
    base.opponent = Some(kit.opponent().clone());
    base.hand_traps = ygo_solver::hand_traps(&engine, &deck, seed, &kit);
    base.policy.passive = known_hand_traps(&engine);
    base.values = std::sync::Arc::new(ygo_solver::card_values(&engine, &deck, seed, &kit, &base.hand_traps, threads));
    base.learn = Some(std::sync::Arc::new(kit.clone()));
    base.coverage = true;
    // One hand per card of the Main Deck: that card and four others.
    let mut cards: Vec<u32> = Vec::new();
    for &c in &deck.main {
        if !cards.contains(&c) {
            cards.push(c);
        }
    }
    // A seed of its own for each hand: the other four cards differ.
    let hands: Vec<Vec<u32>> = cards
        .iter()
        .enumerate()
        .filter_map(|(i, &c)| {
            ygo_solver::sample_hands(&deck.main, c, 1, seed.wrapping_add(7919 * i as u64)).into_iter().next().map(|(h, _)| h)
        })
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, hands.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(hand) = hands.get(i) else { break };
                let mut config = base.clone();
                config.hand = hand.clone();
                let result = ygo_solver::search(&engine, &config, &std::sync::atomic::AtomicBool::new(false), |_| {});
                if let Ok(mut r) = results.lock() {
                    r.push((i, result));
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap_or_default();
    results.sort_by_key(|(i, _)| *i);
    let mut total = ygo_solver::Coverage::default();
    for (i, r) in &results {
        let names: Vec<String> = hands[*i].iter().map(|&c| db.name(c)).collect();
        println!(
            "hand {:>2}: {} nodes, {} boards, best {:.1}, {} interrupted · {}",
            i + 1,
            r.progress.nodes,
            r.progress.boards,
            r.boards.first().map_or(0.0, |b| b.quick_score),
            r.interrupted,
            names.join(", ")
        );
        if let Some(c) = &r.coverage {
            total.merge(c);
        }
    }
    let place = |bits: u8| -> String {
        let names = [
            (location::DECK, "Deck"),
            (location::HAND, "hand"),
            (location::MZONE, "M.Zone"),
            (location::SZONE, "S/T Zone"),
            (location::GRAVE, "GY"),
            (location::REMOVED, "banished"),
            (location::EXTRA, "Extra"),
        ];
        names.iter().filter(|(b, _)| bits & *b != 0).map(|(_, n)| *n).collect::<Vec<_>>().join("+")
    };
    let entries = total.sorted();
    let mut listed: Vec<u32> = Vec::new();
    for &c in deck.main.iter().chain(&deck.extra) {
        let c = db.canonical_code(c);
        if !listed.contains(&c) {
            listed.push(c);
        }
    }
    println!("\noffered / left out by policy / as a return / cut / kept / chosen / activated or summoned / negated by your own effect / disabled when resolving / negated one of your own");
    for &code in &listed {
        let mine: Vec<&(ygo_solver::Mechanic, ygo_solver::Counts)> = entries.iter().filter(|(m, _)| m.code == code).collect();
        let flagged: Vec<&&(ygo_solver::Mechanic, ygo_solver::Counts)> = mine
            .iter()
            .filter(|(m, c)| {
                all || c.chosen + c.activated + c.summoned == 0
                    || (m.kind == ygo_solver::MechanicKind::Activate && c.activated == 0)
                    || c.self_negated + c.negated_own + c.disabled > 0
            })
            .collect();
        if mine.is_empty() {
            println!("\n{} ({code}): never offered, summoned nor activated", db.name(code));
            continue;
        }
        if flagged.is_empty() {
            continue;
        }
        println!("\n{} ({code})", db.name(code));
        for (m, c) in flagged {
            let text = db.describe(m.description);
            println!(
                "  {:<17} {:<48} {:<16} {:>5} {:>5} {:>5} {:>4} {:>5} {:>5} {:>5} {:>4} {:>4} {:>4}",
                m.kind.label(),
                if text.is_empty() { format!("#{}", m.description & 0xf_ffff) } else { text.chars().take(48).collect() },
                place(c.locations),
                c.offered,
                c.policy,
                c.returns,
                c.cut,
                c.kept,
                c.chosen,
                c.activated + c.summoned,
                c.self_negated,
                c.disabled,
                c.negated_own
            );
        }
    }
    let others: Vec<&(ygo_solver::Mechanic, ygo_solver::Counts)> = entries.iter().filter(|(m, _)| !listed.contains(&m.code)).collect();
    if !others.is_empty() {
        println!("\nOther cards (tokens, cards of another code):");
        for (m, c) in others {
            println!(
                "  {:<40} {:<17} offered {} chosen {} activated {}",
                db.name(m.code),
                m.kind.label(),
                c.offered,
                c.chosen,
                c.activated
            );
        }
    }
    println!("\n{} hands in {:.1} s", results.len(), started.elapsed().as_secs_f64());
    Ok(())
}

fn load_engine(dir: &Path) -> Result<Engine> {
    Engine::load(dir).with_context(|| format!("cannot load game data from {} (run `duelforge-cli fetch-data` first)", dir.display()))
}

fn info(dir: &Path) -> Result<()> {
    let (major, minor) = Engine::api_version();
    println!("Engine API: {major}.{minor}");
    let engine = load_engine(dir)?;
    println!("Cards: {}", engine.cards().len());
    println!("Scripts: {}", engine.scripts().len());
    Ok(())
}

/// A place of `--goal` ("Name@place").
fn goal_zone(place: &str) -> Option<ygo_solver::GoalZone> {
    use ygo_solver::GoalZone;
    Some(match place {
        "field" => GoalZone::Any,
        "extra" => GoalZone::Extra,
        "main" => GoalZone::Main,
        "monster" => GoalZone::Monster,
        "faceup" => GoalZone::FaceUp,
        "set" => GoalZone::Set,
        "fieldzone" => GoalZone::FieldZone,
        "gy" | "grave" => GoalZone::Grave,
        "banished" => GoalZone::Banished,
        "hand" => GoalZone::Hand,
        _ => return None,
    })
}

fn resolve_hand(engine: &Engine, hand: &str) -> Result<Vec<u32>> {
    hand.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|item| match item.parse::<u32>() {
            Ok(code) => Ok(code),
            Err(_) => engine.cards().find_by_name(item).map(|c| c.code).with_context(|| format!("unknown card {item:?}")),
        })
        .collect()
}

fn start(dir: &Path, setup: &Setup) -> Result<(Engine, DeckList, Vec<u32>)> {
    let engine = load_engine(dir)?;
    let (deck, hand) = prepare(&engine, setup)?;
    Ok((engine, deck, hand))
}

/// The deck and the hand of `setup`, with a warning for each card without a script.
fn prepare(engine: &Engine, setup: &Setup) -> Result<(DeckList, Vec<u32>)> {
    let text = std::fs::read_to_string(&setup.deck).with_context(|| format!("cannot read {}", setup.deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let hand = resolve_hand(engine, &setup.hand)?;
    let mut unscripted: Vec<u32> = deck
        .main
        .iter()
        .chain(&deck.extra)
        .map(|&c| engine.cards().canonical_code(c))
        .filter(|&c| !engine.scripts().has_card_script(c))
        .collect();
    unscripted.dedup();
    for code in unscripted {
        eprintln!("warning: no script for {} ({code})", engine.cards().name(code));
    }
    Ok((deck, hand))
}

fn print_events(engine: &Engine, events: &[Event]) {
    let db = engine.cards();
    for event in events {
        let line = match event {
            Event::NewTurn { player } => format!("── Turn of player {player} ──"),
            Event::NewPhase { phase: p } => format!("· {}", phase::name(*p)),
            Event::Draw { player, codes } => format!("  player {player} draws {} card(s)", codes.len()),
            Event::Move { code, from, to, .. } if from.location != to.location => {
                let name = if *code == 0 { "a card".to_owned() } else { db.name(*code) };
                format!("  {name}: {} → {}", location::name(from.location), location::name(to.location))
            }
            Event::NormalSummoning { card } => format!("  Normal Summon: {}", describe_card(db, card)),
            Event::SpecialSummoning { card } => format!("  Special Summon: {}", describe_card(db, card)),
            Event::Chaining { card, description, chain_link } => {
                format!("  Chain Link {chain_link}: {} — {}", db.name(card.code), db.describe(*description))
            }
            Event::Win { player, reason } => format!("  Player {player} wins (reason {reason})"),
            _ => continue,
        };
        println!("{line}");
    }
}

fn print_board(engine: &Engine, duel: &Duel) {
    let db = engine.cards();
    let names = |loc: u8| -> Vec<String> {
        duel.query_location(0, loc)
            .into_iter()
            .flatten()
            .map(|(code, pos)| {
                let pos =
                    if loc == location::MZONE || loc == location::SZONE { format!(" [{}]", position::name(pos)) } else { String::new() };
                format!("{}{pos}", db.name(code))
            })
            .collect()
    };
    println!("\n=== Board (player 0) ===");
    for (label, loc) in [
        ("Monster Zones", location::MZONE),
        ("Spell/Trap Zones", location::SZONE),
        ("Hand", location::HAND),
        ("GY", location::GRAVE),
        ("Banished", location::REMOVED),
    ] {
        let list = names(loc);
        println!("{label} ({}): {}", list.len(), if list.is_empty() { "-".to_owned() } else { list.join(", ") });
    }
    println!("Deck: {} cards, Extra Deck: {} cards", duel.count(0, location::DECK), duel.count(0, location::EXTRA));
    let missing = duel.missing_scripts();
    if !missing.is_empty() {
        println!("Missing scripts: {}", missing.join(", "));
    }
}

fn show_options(list: &[Action]) {
    for (i, a) in list.iter().enumerate() {
        println!("  [{i}] {}", a.label);
    }
}

fn play(dir: &Path, setup: &Setup, steps: Option<&str>, verbose: bool) -> Result<()> {
    let (engine, deck, hand) = start(dir, setup)?;
    let mut duel = Duel::goldfish(&engine, &deck, &hand, setup.seed)?;
    println!("Hand: {}", hand.iter().map(|&c| engine.cards().name(c)).collect::<Vec<_>>().join(", "));
    match steps {
        Some(steps) => play_script(&engine, &mut duel, steps, verbose),
        None => play_interactive(&engine, &mut duel),
    }
}

fn play_script(engine: &Engine, duel: &mut Duel, steps: &str, verbose: bool) -> Result<()> {
    let steps: Vec<&str> = steps.split(';').collect();
    let end = run_script(duel, &steps, |trace| match trace {
        Trace::Events(events) => print_events(engine, events),
        Trace::Chose(action) => println!("> {}", action.label),
    })?;
    match end {
        ScriptEnd::Finished { actions, .. } => {
            println!("\nScript finished. Next decision:");
            show_options(&actions);
        }
        ScriptEnd::NoMatch { step, actions, decision } => {
            println!("\nNo action matches {step:?}. Options:");
            show_options(&actions);
            if verbose {
                println!("{decision:?}");
            }
            print_board(engine, duel);
            bail!("script stopped");
        }
        ScriptEnd::TurnEnded => println!("\nEnd of turn."),
        ScriptEnd::DuelEnded => println!("\nDuel over."),
    }
    print_board(engine, duel);
    Ok(())
}

fn play_interactive(engine: &Engine, duel: &mut Duel) -> Result<()> {
    let stdin = std::io::stdin();
    loop {
        let (decision, events) = match duel.advance()? {
            Step::End { events } => {
                print_events(engine, &events);
                println!("Duel over.");
                return Ok(());
            }
            Step::Decision { decision, events } => (decision, events),
        };
        print_events(engine, &events);
        let list = actions(&decision, engine.cards());
        if list.is_empty() {
            bail!("decision not supported yet: {decision:?}");
        }
        println!();
        show_options(&list);
        let index = loop {
            print!("choice (b = board, q = quit)> ");
            std::io::stdout().flush()?;
            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                return Ok(());
            }
            match line.trim() {
                "q" => return Ok(()),
                "b" => print_board(engine, duel),
                n => match n.parse::<usize>() {
                    Ok(i) if i < list.len() => break i,
                    _ => println!("enter a number between 0 and {}", list.len() - 1),
                },
            }
        };
        if list[index].kind == ActionKind::EndTurn {
            print_board(engine, duel);
            return Ok(());
        }
        duel.respond(&list[index].response);
    }
}

/// SplitMix64, enough for picking random actions.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}

/// Plays random legal actions until the end of the first turn; returns the
/// responses given.
fn random_playout(engine: &Engine, duel: &mut Duel, rng: &mut Rng, max_decisions: u32) -> Result<Vec<Vec<u8>>> {
    let mut responses = Vec::new();
    for _ in 0..max_decisions {
        let Step::Decision { decision, .. } = duel.advance()? else { break };
        let list = actions(&decision, engine.cards());
        if list.is_empty() {
            break;
        }
        // Prefer doing something over ending the turn early.
        let playable: Vec<&Action> = list.iter().filter(|a| !matches!(a.kind, ActionKind::EndTurn | ActionKind::ToBattlePhase)).collect();
        if playable.is_empty() {
            break;
        }
        let action = playable[rng.below(playable.len())];
        duel.respond(&action.response);
        responses.push(action.response.clone());
    }
    Ok(responses)
}

fn replay(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, line: &[Vec<u8>]) -> Result<()> {
    let mut duel = Duel::goldfish(engine, deck, hand, seed)?;
    for response in line {
        duel.advance()?;
        duel.respond(response);
    }
    duel.advance()?;
    Ok(())
}

fn bench(dir: &Path, setup: &Setup, playouts: u32, max_decisions: u32) -> Result<()> {
    let (engine, deck, hand) = start(dir, setup)?;

    let n_create = 50;
    let t = Instant::now();
    for _ in 0..n_create {
        Duel::new(&engine, &ygo_engine::DuelConfig::default())?;
    }
    println!("Empty duel (base scripts only): {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / n_create as f64);

    let t = Instant::now();
    for i in 0..n_create {
        Duel::goldfish(&engine, &deck, &hand, setup.seed + i)?.advance()?;
    }
    println!("Duel creation + first decision: {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / n_create as f64);

    let mut rng = Rng(setup.seed);
    let mut lines = Vec::new();
    let t = Instant::now();
    for _ in 0..playouts {
        let mut duel = Duel::goldfish(&engine, &deck, &hand, setup.seed)?;
        lines.push(random_playout(&engine, &mut duel, &mut rng, max_decisions)?);
    }
    let secs = t.elapsed().as_secs_f64();
    let decisions: usize = lines.iter().map(Vec::len).sum();
    let longest = lines.iter().map(Vec::len).max().unwrap_or(0);
    println!(
        "Random playouts: {playouts} in {secs:.2} s ({:.0} decisions/s, avg {:.1} decisions per playout, longest {longest})",
        decisions as f64 / secs,
        decisions as f64 / playouts as f64
    );

    // Replaying a line from scratch is how the search will restore a node.
    let t = Instant::now();
    for line in &lines {
        replay(&engine, &deck, &hand, setup.seed, line)?;
    }
    let secs = t.elapsed().as_secs_f64();
    println!("Replays: {:.2} ms per line ({:.0} decisions/s)", secs * 1000.0 / lines.len() as f64, decisions as f64 / secs);

    // The same replays on every core, each thread owning its duels.
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let t = Instant::now();
    std::thread::scope(|scope| -> Result<()> {
        let handles: Vec<_> = (0..threads)
            .map(|k| {
                let (engine, deck, hand, lines) = (&engine, &deck, &hand, &lines);
                scope.spawn(move || -> Result<()> {
                    for line in lines.iter().skip(k).step_by(threads) {
                        replay(engine, deck, hand, setup.seed, line)?;
                    }
                    Ok(())
                })
            })
            .collect();
        handles.into_iter().try_for_each(|h| h.join().expect("bench thread panicked"))
    })?;
    let secs = t.elapsed().as_secs_f64();
    println!("Parallel replays on {threads} threads: {:.0} decisions/s", decisions as f64 / secs);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_fixtures_cover_each_query_once() {
        let store = CardStore::open_in_memory().unwrap();
        let json =
            recorded_fixtures(&store, &(vec![1], vec![], vec![]), &["type:monster".into(), "x".into()], &Default::default()).unwrap();
        let searches = json["searches"].as_object().unwrap();
        assert_eq!(searches.len(), FIXTURE_QUERIES.len() + 1);
        assert_eq!(searches["x"]["ids"], serde_json::json!([]));
        assert_eq!(json["deck"]["main"], serde_json::json!([1]));
        assert_eq!(json["legality"]["legal"], serde_json::json!(false));
        assert_eq!(json["overview"]["size"], serde_json::json!(1));
    }

    #[test]
    fn roles_parse_from_arguments() {
        let roles = parse_roles(&["starter=1,2".into(), "starter=3".into(), "brick=4".into()]).unwrap();
        assert_eq!(roles["starter"], vec![1, 2, 3]);
        assert_eq!(roles["brick"], vec![4]);
        assert!(parse_roles(&["starter".into()]).is_err());
    }
}
