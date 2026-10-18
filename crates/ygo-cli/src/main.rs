//! `duelforge-cli`: headless tools around the duel engine.
//!
//! - `fetch-data`: download card scripts and card databases (Project Ignis).
//! - `info`: engine version and loaded data.
//! - `play`: play a starting hand from a deck, interactively or by script.
//! - `bench`: measure duel creation, playout and replay speed.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ygo_engine::consts::{location, phase, position};
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
        Cmd::Play { setup, steps, verbose } => play(&cli.data, &setup, steps.as_deref(), verbose),
        Cmd::Bench { setup, playouts, max_decisions } => bench(&cli.data, &setup, playouts, max_decisions),
    }
}

const SOURCES: &[(&str, &str)] =
    &[("scripts", "https://github.com/ProjectIgnis/CardScripts.git"), ("cdb", "https://github.com/ProjectIgnis/BabelCDB.git")];

fn fetch_data(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for (name, url) in SOURCES {
        let target = dir.join(name);
        let status = if target.join(".git").is_dir() {
            println!("Updating {} ...", target.display());
            Command::new("git").arg("-C").arg(&target).args(["pull", "--ff-only", "--depth", "1"]).status()
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
