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
        /// Goal: cards wanted on the field at the end (semicolon-separated;
        /// "Name@extra" or "Name@main" for a monster's zone).
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
    },
    /// The handtraps of a deck, read from the engine: cards it can use from
    /// the hand during the opponent's turn.
    HandTraps {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
    },
    /// What each card of a deck is worth alone on the field, measured by the
    /// probe turn (what the search steers by).
    CardValues {
        /// Deck in .ydk format.
        #[arg(long)]
        deck: PathBuf,
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
                probe,
            },
        ),
        Cmd::HandTraps { deck } => hand_traps(&cli.data, &deck),
        Cmd::CardValues { deck } => card_values(&cli.data, &deck),
        Cmd::Roles { deck } => roles(&cli.data, &deck),
        Cmd::Coverage { deck, nodes, threads, seed, all } => coverage(&cli.data, &deck, nodes, threads, seed, all),
        Cmd::Sweep { duels, max_decisions, max_turns, seed, threads, script_errors } => {
            let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
            sweep(&cli.data, &SweepConfig { duels, max_decisions, max_turns, seed, threads }, script_errors)
        }
    }
}

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
}

fn combos(dir: &Path, setup: &Setup, options: CombosOptions) -> Result<()> {
    let CombosOptions { seconds, nodes, threads, top, show_lines, use_handtraps, forbid, keep, card, hands, goal, goal_any, first, probe } =
        options;
    let (engine, deck, hand) = start(dir, setup)?;
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
    let kit = ygo_solver::Kit::standard(&engine);
    config.opponent = Some(kit.opponent().clone());
    // Handtraps, read from the engine: kept in hand they are worth more.
    config.hand_traps = ygo_solver::hand_traps(&engine, &config.deck, config.seed, &kit);
    // What each card stops alone on the field: the search steers by it,
    // and learns on the way what cards stop together.
    config.values =
        std::sync::Arc::new(ygo_solver::card_values(&engine, &config.deck, config.seed, &kit, &config.hand_traps, config.threads));
    config.learn = Some(std::sync::Arc::new(kit.clone()));
    if !use_handtraps {
        // Not played from the hand on your own turn: the well-known handtraps
        // (an archetype's quick effects from the hand are often combo pieces too).
        config.policy.passive =
            ygo_data::Tag::Handtrap.names().iter().filter_map(|n| engine.cards().find_by_name(n)).map(|c| c.code).collect();
    }
    config.policy.forbid = resolve_hand(&engine, &forbid)?;
    config.keep_in_hand = resolve_hand(&engine, &keep)?;
    // "Name@extra" or "Name@main" asks for a Monster Zone.
    let mut on_field = Vec::new();
    for item in goal.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let (name, zone) = match item.rsplit_once('@') {
            Some((name, "extra")) => (name, ygo_solver::GoalZone::Extra),
            Some((name, "main")) => (name, ygo_solver::GoalZone::Main),
            _ => (item, ygo_solver::GoalZone::Any),
        };
        let code = *resolve_hand(&engine, name)?.first().context("empty goal card")?;
        on_field.push(ygo_solver::GoalCard { code, zone });
    }
    config.goal = ygo_solver::Goal { on_field, all: !goal_any, first_only: first };
    let goal_status = |reached: Option<bool>| match reached {
        Some(true) => "reached",
        Some(false) => "impossible: no line reaches it",
        None => "not reached in the time given",
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    if let Some(card) = card {
        let card = *resolve_hand(&engine, &card)?.first().context("--card names no card")?;
        let hands = hands.max(1);
        let what = ygo_solver::CardSearch { card, hands, per_hand: config.time_limit / hands as u32, keep: top.max(1) };
        let results = ygo_solver::search_card(&engine, &config, &what, &cancel, |_, _, _| {});
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
    let mut prober = ygo_solver::Prober::new(&engine, &config.deck, &config.hand, config.seed, &kit);
    // Boards with the same field and hand (other cards in the GY, other
    // zones) are variants of one: listed once, as the Combo Lab does.
    let mut looks: Vec<(u64, usize)> = Vec::new();
    let mut listed: Vec<&ygo_solver::Board> = Vec::new();
    for board in &result.boards {
        let look = ygo_solver::look_key(&board.state);
        match looks.iter_mut().find(|(l, _)| *l == look) {
            Some((_, n)) => *n += 1,
            None if listed.len() < top => {
                looks.push((look, 0));
                listed.push(board);
            }
            None => {}
        }
    }
    for (i, board) in listed.into_iter().enumerate() {
        let variants = looks[i].1;
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
        println!(
            "\n#{}{}{} score {:.1} · {} cards used · {} choices · found at {:.1} s\n   field: {}\n   hand: {}",
            i + 1,
            if board.exact { "" } else { " (End Phase not played)" },
            if variants > 0 { format!(" (+{variants} variants)") } else { String::new() },
            board.quick_score,
            board.cards_used,
            board.line.choices.len(),
            board.found_after.as_secs_f64(),
            field.join(", "),
            hand.join(", ")
        );
        if probe {
            let started = std::time::Instant::now();
            match prober.measure(&board.line) {
                Ok(m) => print_measure(&engine, &m, started.elapsed()),
                Err(e) => println!("   probe turn failed: {e}"),
            }
            if let Ok(x) = ygo_solver::exposure(&engine, &config.deck, &config.hand, config.seed, config.opponent.as_ref(), &board.line) {
                println!(
                    "   exposure: {} choke points · Ash {} · on-field negation {} · summons {} (special {}) · searches {}",
                    x.choke_points(),
                    x.ash,
                    x.on_field,
                    x.summons,
                    x.special_summons,
                    x.searches
                );
            }
        }
        if show_lines {
            let (_, steps) = ygo_solver::explain(&engine, &config.deck, &config.hand, config.seed, config.opponent.as_ref(), &board.line)?;
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
        let dead = matches!(spot, ygo_solver::Spot::Hand | ygo_solver::Spot::HandWithField) && !values.usable(&engine, code, spot);
        println!("{value:.2}\t{spot:?}\t{}{}", engine.cards().name(code), if dead { "\t(cannot be used: a free discard)" } else { "" });
    }
    println!("measured in {:.1} s", started.elapsed().as_secs_f64());
    Ok(())
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
    let guesses = ygo_solver::guess_roles(&engine, &deck, &kit, &hand_traps, values, threads, |_, _| {});
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
    base.policy.passive = base.hand_traps.clone();
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
    println!("\noffered / left out by policy / as a return / cut / kept / chosen / activated or summoned / negated by your own card / negated one of your own");
    for &code in &listed {
        let mine: Vec<&(ygo_solver::Mechanic, ygo_solver::Counts)> = entries.iter().filter(|(m, _)| m.code == code).collect();
        let flagged: Vec<&&(ygo_solver::Mechanic, ygo_solver::Counts)> = mine
            .iter()
            .filter(|(m, c)| {
                all || c.chosen + c.activated + c.summoned == 0
                    || (m.kind == ygo_solver::MechanicKind::Activate && c.activated == 0)
                    || c.self_negated + c.negated_own > 0
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
                "  {:<17} {:<48} {:<16} {:>5} {:>5} {:>5} {:>4} {:>5} {:>5} {:>5} {:>4} {:>4}",
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
