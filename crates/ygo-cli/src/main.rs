//! `duelforge-cli`: headless tools around the duel engine.
//!
//! - `fetch-data`: download card scripts and card databases (Project Ignis).
//! - `info`: engine version and loaded data.
//! - `play`: play a starting hand from a deck, interactively or by script.
//! - `bench`: measure duel creation, playout and replay speed.
//! - `combos`: search every line from a hand and list the end boards.
//! - `sweep`: random duels between random decks, reporting any decision
//!   that cannot be answered or any answer the engine rejects.

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

#[derive(Parser)]
#[command(name = "duelforge-cli", version, about)]
struct Cli {
    /// Folder with `scripts/` (CardScripts) and `cdb/` (BabelCDB).
    #[arg(long, global = true, default_value = "data")]
    data: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}

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
}

#[derive(clap::Args)]
struct Setup {
    /// Deck in .ydk format.
    #[arg(long)]
    deck: PathBuf,
    /// Starting hand: semicolon-separated card names or passcodes.
    #[arg(long)]
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
        Cmd::Search { query, limit } => search(&cli.data, &query, limit),
        Cmd::Play { setup, steps, verbose } => play(&cli.data, &setup, steps.as_deref(), verbose),
        Cmd::Bench { setup, playouts, max_decisions } => bench(&cli.data, &setup, playouts, max_decisions),
        Cmd::Combos { setup, seconds, threads, top, lines, use_handtraps, forbid, keep } => {
            combos(&cli.data, &setup, CombosOptions { seconds, threads, top, show_lines: lines, use_handtraps, forbid, keep })
        }
        Cmd::Sweep { duels, max_decisions, max_turns, seed, threads, script_errors } => {
            let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
            sweep(&cli.data, &SweepConfig { duels, max_decisions, max_turns, seed, threads }, script_errors)
        }
    }
}

struct CombosOptions {
    seconds: u64,
    threads: Option<usize>,
    top: usize,
    show_lines: bool,
    use_handtraps: bool,
    forbid: String,
    keep: String,
}

fn combos(dir: &Path, setup: &Setup, options: CombosOptions) -> Result<()> {
    let CombosOptions { seconds, threads, top, show_lines, use_handtraps, forbid, keep } = options;
    let (engine, deck, hand) = start(dir, setup)?;
    let mut config = ygo_solver::SearchConfig::new(deck, hand);
    config.seed = setup.seed;
    config.time_limit = std::time::Duration::from_secs(seconds);
    if let Some(t) = threads {
        config.threads = t;
    }
    if !use_handtraps {
        config.policy.passive =
            ygo_data::Tag::Handtrap.names().iter().filter_map(|n| engine.cards().find_by_name(n)).map(|c| c.code).collect();
    }
    config.policy.forbid = resolve_hand(&engine, &forbid)?;
    config.keep_in_hand = resolve_hand(&engine, &keep)?;
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut last = 0;
    let result = ygo_solver::search(&engine, &config, &cancel, |p| {
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
    for (i, board) in result.boards.iter().take(top).enumerate() {
        let me = &board.state.players[0];
        let field: Vec<String> = me.monsters.iter().chain(&me.spells).flatten().map(|c| db.name(c.code)).collect();
        let hand: Vec<String> = me.hand.iter().map(|&c| db.name(c)).collect();
        println!(
            "\n#{} score {:.1} · {} cards used · {} choices · found at {:.1} s\n   field: {}\n   hand: {}",
            i + 1,
            board.quick_score,
            board.cards_used,
            board.line.choices.len(),
            board.found_after.as_secs_f64(),
            field.join(", "),
            hand.join(", ")
        );
        if show_lines {
            let (_, steps) = ygo_solver::explain(&engine, &config.deck, &config.hand, config.seed, &board.line)?;
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
    Ok(())
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
        sync::update_cards(&client, current.as_deref(), &new_db, &|p| {
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
    let text = std::fs::read_to_string(&setup.deck).with_context(|| format!("cannot read {}", setup.deck.display()))?;
    let deck = DeckList::from_ydk(&text)?;
    let hand = resolve_hand(&engine, &setup.hand)?;
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
    Ok((engine, deck, hand))
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
