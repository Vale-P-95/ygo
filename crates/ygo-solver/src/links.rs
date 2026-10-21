//! The map of the links between the cards of a deck (plan
//! `docs/plans/mappa-collegamenti.md`): which card adds which from the
//! Deck, Special Summons it, sends it to the GY, gets it back, or is one of
//! its materials. Read from the engine, not from the text: lines are
//! searched from hands holding each card, and played again; whenever an
//! effect asks to pick cards, the cards it offers are links "possible", and
//! the cards the line moves are links "used".

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ygo_engine::consts::{card_type, location, reason};
use ygo_engine::{CardRef, Decision, DeckList, Duel, Engine, Event, Opponent, Step};

use crate::card::sample_hands;
use crate::matchup::none_drawn;
use crate::probe::{hand_traps, Kit};
use crate::search::{search, Line, SearchConfig};

/// How one card leads to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LinkKind {
    /// Adds it from the Deck to the hand.
    Search,
    /// Special Summons it (from anywhere but the field).
    Summon,
    /// Sends it from the Deck to the GY.
    ToGrave,
    /// Gets it back from the GY or banishment to the hand.
    Recover,
    /// `from` is a material named by `to` (an Extra Deck monster).
    Material,
    /// Puts it from the Deck on the field: Set, or placed face-up in a
    /// Spell & Trap Zone or the Field Zone.
    Set,
}

impl LinkKind {
    pub fn key(self) -> &'static str {
        match self {
            LinkKind::Search => "search",
            LinkKind::Summon => "summon",
            LinkKind::ToGrave => "toGrave",
            LinkKind::Recover => "recover",
            LinkKind::Material => "material",
            LinkKind::Set => "set",
        }
    }

    /// It brings a new card into play (to the hand or the field), so the
    /// links of that card count from there: not sending it to the GY or
    /// getting it back.
    fn brings(self) -> bool {
        matches!(self, LinkKind::Search | LinkKind::Summon | LinkKind::Material | LinkKind::Set)
    }
}

/// Where the card a link leads to comes from, the zones that bring a new
/// card into play first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Zone {
    Deck,
    Extra,
    Grave,
    Banished,
    Hand,
}

impl Zone {
    fn of(location: u8) -> Option<Zone> {
        match location {
            location::DECK => Some(Zone::Deck),
            location::EXTRA => Some(Zone::Extra),
            location::GRAVE => Some(Zone::Grave),
            location::REMOVED => Some(Zone::Banished),
            location::HAND => Some(Zone::Hand),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Zone::Deck => "deck",
            Zone::Extra => "extra",
            Zone::Grave => "grave",
            Zone::Banished => "banished",
            Zone::Hand => "hand",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CardLink {
    pub from: u32,
    pub to: u32,
    pub kind: LinkKind,
    /// Where `to` comes from (the first of the zones seen).
    pub zone: Zone,
    /// The engine offered it: the effect can pick that card.
    pub possible: bool,
    /// Share of the lines played that use it, 0 to 1.
    pub used: f64,
    /// The card's text names it (an effect the lines may never use).
    pub text: bool,
}

#[derive(Debug, Clone)]
pub struct LinkCard {
    pub code: u32,
    pub copies: u32,
    pub extra: bool,
    /// Cards it leads to in one step (any link); with the cards brought
    /// into play from there in any number of steps (links from the Deck or
    /// the Extra Deck: getting back a card already used brings no new one).
    pub reach: u32,
    pub reach_all: u32,
    /// Chance that an opening hand of 5 holds it or a card that brings it
    /// into play in any number of steps. For an Extra Deck monster no card
    /// names (summoned with any materials), the share of the hands searched
    /// whose best line summons it.
    pub chance: f64,
    /// `chance` is that share, measured and not exact.
    pub measured: bool,
    /// Cards an opening hand reaches on average (the sum of the chances)
    /// that are lost when this card's effects are negated: how much the
    /// deck leans on it.
    pub key: f64,
}

#[derive(Debug, Clone, Default)]
pub struct DeckLinks {
    pub cards: Vec<LinkCard>,
    pub links: Vec<CardLink>,
    /// Hands searched and lines played.
    pub hands: u32,
    pub lines: u32,
}

/// How much is searched to read the links.
#[derive(Debug, Clone, Copy)]
pub struct LinksBudget {
    /// Hands searched for each card of the Main Deck.
    pub hands_per_card: usize,
    pub nodes_per_hand: u64,
    /// Best lines played again for each hand.
    pub lines_per_hand: usize,
    pub threads: usize,
    /// The longest search of a hand, and of the whole map: past it the
    /// hands left are not searched (every card has had its first hand by
    /// then, unless the deck is very slow).
    pub time_per_hand: Duration,
    pub time: Duration,
}

impl Default for LinksBudget {
    fn default() -> Self {
        LinksBudget {
            hands_per_card: 2,
            nodes_per_hand: 1000,
            lines_per_hand: 20,
            threads: 4,
            time_per_hand: Duration::from_secs(15),
            time: Duration::from_secs(60),
        }
    }
}

/// A link seen in a line: from, to, kind, where `to` came from.
type Key = (u32, u32, LinkKind, Zone);

/// A link over all lines: the first zone seen, offered, named by the text,
/// the lines using it.
struct Found {
    zone: Zone,
    offered: bool,
    text: bool,
    lines: HashSet<usize>,
}

/// The links of one line: those offered and those used; the Extra Deck
/// monsters it summons.
#[derive(Default)]
struct Seen {
    possible: HashSet<Key>,
    used: HashSet<Key>,
    summoned: HashSet<u32>,
}

/// Reads the map of `deck`. `progress(done, total, map)` comes first with
/// the map read from the cards' text alone (`done` 0), then after each
/// hand searched with the map so far: a draft at once, filled in as the
/// engine plays.
pub fn deck_links(
    engine: &Engine,
    deck: &DeckList,
    seed: u64,
    budget: LinksBudget,
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize, &DeckLinks) + Sync,
) -> DeckLinks {
    let db = engine.cards();
    let mut cards: Vec<LinkCard> = Vec::new();
    for (codes, extra) in [(&deck.main, false), (&deck.extra, true)] {
        for &c in codes {
            let code = db.canonical_code(c);
            match cards.iter_mut().find(|k| k.code == code) {
                Some(k) => k.copies += 1,
                None => cards.push(LinkCard { code, copies: 1, extra, reach: 0, reach_all: 0, chance: 0.0, measured: false, key: 0.0 }),
            }
        }
    }
    let named = Names::new(engine, &cards);
    let text = text_links(engine, &cards, &named);

    // A few hands holding each card of the Main Deck. The first hand of
    // every card, then the second ones: a map cut short by the time limit
    // still has every card.
    let kit = Kit::standard(engine);
    let traps = hand_traps(engine, deck, seed, &kit);
    let mut per_card: Vec<Vec<Vec<u32>>> = Vec::new();
    for card in cards.iter().filter(|c| !c.extra) {
        let Some(&copy) = deck.main.iter().find(|&&c| db.canonical_code(c) == card.code) else { continue };
        per_card.push(sample_hands(&deck.main, copy, budget.hands_per_card, seed).into_iter().map(|(hand, _)| hand).collect());
    }
    let mut hands: Vec<Vec<u32>> = Vec::new();
    for round in 0..budget.hands_per_card {
        for hand in per_card.iter().filter_map(|h| h.get(round)) {
            if !hands.contains(hand) {
                hands.push(hand.clone());
            }
        }
    }
    progress(0, hands.len(), &assemble(deck, &cards, &[], &text));
    let deadline = Instant::now() + budget.time;

    let next = AtomicUsize::new(0);
    // The lines played of each hand, the best first.
    let seen: Mutex<Vec<Vec<Seen>>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..budget.threads.clamp(1, hands.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(hand) = hands.get(i) else { break };
                let left = deadline.saturating_duration_since(Instant::now());
                if cancel.load(Ordering::Relaxed) || left.is_zero() {
                    break;
                }
                let mut config = SearchConfig::new(deck.clone(), hand.clone());
                config.seed = seed;
                config.threads = 1;
                config.max_nodes = budget.nodes_per_hand;
                config.time_limit = budget.time_per_hand.min(left);
                config.opponent = Some(kit.opponent().clone());
                config.hand_traps = traps.clone();
                let result = search(engine, &config, cancel, |_| {});
                let lines: Vec<Seen> = result
                    .boards
                    .iter()
                    .take(budget.lines_per_hand)
                    .filter_map(|b| play(engine, deck, hand, seed, config.opponent.as_ref(), &b.line, &named))
                    .collect();
                if let Ok(mut all) = seen.lock() {
                    all.push(lines);
                    progress(all.len(), hands.len(), &assemble(deck, &cards, &all, &text));
                }
            });
        }
    });
    let by_hand = seen.into_inner().unwrap_or_default();
    assemble(deck, &cards, &by_hand, &text)
}

/// The map from the lines played of each hand and the links the text names.
fn assemble(deck: &DeckList, cards: &[LinkCard], by_hand: &[Vec<Seen>], text: &HashSet<Key>) -> DeckLinks {
    let mut cards = cards.to_vec();
    let seen: Vec<&Seen> = by_hand.iter().flatten().collect();

    // Links between cards of the deck, one per card and kind, from the
    // first zone seen; used ones counted once per line.
    let in_deck: HashSet<u32> = cards.iter().map(|c| c.code).collect();
    let mut found: HashMap<(u32, u32, LinkKind), Found> = HashMap::new();
    let mut add = |&(from, to, kind, zone): &Key, offered: bool, named: bool, line: Option<usize>| {
        let entry = found.entry((from, to, kind)).or_insert(Found { zone, offered: false, text: false, lines: HashSet::new() });
        entry.zone = entry.zone.min(zone);
        entry.offered |= offered;
        entry.text |= named;
        entry.lines.extend(line);
    };
    for (i, line) in seen.iter().enumerate() {
        for k in &line.possible {
            add(k, true, false, None);
        }
        for k in &line.used {
            add(k, false, false, Some(i));
        }
    }
    for k in text {
        add(k, false, true, None);
    }
    let mut links: Vec<CardLink> = found
        .into_iter()
        .filter(|((from, to, _), _)| from != to && in_deck.contains(from) && in_deck.contains(to))
        .map(|((from, to, kind), f)| CardLink {
            from,
            to,
            kind,
            zone: f.zone,
            possible: f.offered || !f.lines.is_empty(),
            used: f.lines.len() as f64 / seen.len().max(1) as f64,
            text: f.text,
        })
        .collect();
    links.sort_by_key(|l| (l.from, l.kind, l.to));

    // Reach in one step; the cards brought into play in any number of
    // steps, and the chance of an opening hand holding one that leads there.
    let next_of = |code: u32| -> HashSet<u32> { links.iter().filter(|l| l.from == code).map(|l| l.to).collect() };
    let brings = |code: u32| -> HashSet<u32> {
        links.iter().filter(|l| l.from == code && l.kind.brings() && matches!(l.zone, Zone::Deck | Zone::Extra)).map(|l| l.to).collect()
    };
    // What `code` brings into play, without the links of `negated`.
    let closure = |code: u32, negated: Option<u32>| -> HashSet<u32> {
        let mut seen: HashSet<u32> = HashSet::new();
        let mut todo = vec![code];
        while let Some(c) = todo.pop() {
            if Some(c) == negated {
                continue;
            }
            for n in brings(c) {
                if n != code && seen.insert(n) {
                    todo.push(n);
                }
            }
        }
        seen
    };
    let codes: Vec<u32> = cards.iter().map(|c| c.code).collect();
    let closures_without =
        |negated: Option<u32>| -> HashMap<u32, HashSet<u32>> { codes.iter().map(|&c| (c, closure(c, negated))).collect() };
    let main_size = deck.main.len();
    let copies: HashMap<u32, u32> = cards.iter().filter(|c| !c.extra).map(|c| (c.code, c.copies)).collect();
    // The chance an opening hand of 5 holds `target` or a card that brings it.
    let chance = |target: u32, closures: &HashMap<u32, HashSet<u32>>| -> f64 {
        let leading: u32 = copies.iter().filter(|(&c, _)| c == target || closures[&c].contains(&target)).map(|(_, &n)| n).sum();
        1.0 - none_drawn(main_size, leading, 5)
    };
    let closures = closures_without(None);
    let reached = |closures: &HashMap<u32, HashSet<u32>>| -> f64 { codes.iter().map(|&c| chance(c, closures)).sum() };
    let total = reached(&closures);
    let keys: HashMap<u32, f64> = codes.iter().map(|&c| (c, (total - reached(&closures_without(Some(c)))).max(0.0))).collect();
    for card in &mut cards {
        let direct = next_of(card.code);
        let mut all = closures[&card.code].clone();
        for d in &direct {
            all.insert(*d);
            all.extend(&closures[d]);
        }
        all.remove(&card.code);
        card.reach = direct.len() as u32;
        card.reach_all = all.len() as u32;
        card.chance = chance(card.code, &closures);
        card.key = keys[&card.code];
    }
    // An Extra Deck monster no card leads to: how often the best line of a
    // hand summons it.
    let best: Vec<&Seen> = by_hand.iter().filter_map(|lines| lines.first()).collect();
    for card in cards.iter_mut().filter(|c| c.extra && c.chance == 0.0 && !best.is_empty()) {
        card.chance = best.iter().filter(|s| s.summoned.contains(&card.code)).count() as f64 / best.len() as f64;
        card.measured = true;
    }
    DeckLinks { cards, links, hands: by_hand.len() as u32, lines: seen.len() as u32 }
}

/// The materials each Extra Deck monster of the deck names in its text: a
/// card named there, or whose name holds an archetype quoted there.
struct Names {
    names: HashMap<u32, String>,
    quoted: HashMap<u32, Vec<String>>,
}

impl Names {
    fn new(engine: &Engine, cards: &[LinkCard]) -> Self {
        let db = engine.cards();
        let names = cards.iter().map(|c| (c.code, db.name(c.code).to_lowercase())).collect();
        let quoted = cards
            .iter()
            .filter(|c| c.extra)
            .filter_map(|c| {
                let text = db.get(c.code)?.desc.to_lowercase();
                // The materials are the first line (of the monster's text,
                // after a Pendulum Effect): not cut at a dot, which names
                // like "R.B." hold.
                let text = text.rsplit("[ monster effect ]").next().unwrap_or("").trim_start();
                let materials = text.split(['\r', '\n']).next().unwrap_or("").to_owned();
                let mut quoted: Vec<String> = materials.split('"').skip(1).step_by(2).map(str::to_owned).collect();
                quoted.push(materials);
                Some((c.code, quoted))
            })
            .collect();
        Names { names, quoted }
    }

    /// `material` is named among the materials of `monster`.
    fn named(&self, material: u32, monster: u32) -> bool {
        let (Some(name), Some(quoted)) = (self.names.get(&material), self.quoted.get(&monster)) else { return false };
        // The last entry is the whole materials line: the card's full name in it.
        quoted.iter().enumerate().any(|(i, q)| if i + 1 == quoted.len() { q.contains(name.as_str()) } else { name.contains(q.as_str()) })
    }
}

/// The links the cards' text names: an effect that adds, summons, sends,
/// sets or places cards named in quotes (a card's name, or an archetype in
/// it) from a zone, and the materials an Extra Deck monster names. A draft
/// read at once, and the effects the lines found never use.
fn text_links(engine: &Engine, cards: &[LinkCard], named: &Names) -> HashSet<Key> {
    let db = engine.cards();
    let mut out = HashSet::new();
    for monster in cards.iter().filter(|c| c.extra) {
        for m in cards.iter().filter(|m| named.named(m.code, monster.code)) {
            out.insert((m.code, monster.code, LinkKind::Material, Zone::Extra));
        }
    }
    for card in cards {
        let Some(data) = db.get(card.code) else { continue };
        let text = data.desc.to_lowercase();
        for clause in clauses(&text) {
            let Some((kind, zone)) = clause_kind(clause) else { continue };
            for (name, what) in quoted(clause) {
                for target in cards.iter().filter(|t| fits(engine, t, &name, what, zone, kind, named)) {
                    out.insert((card.code, target.code, kind, zone));
                }
            }
        }
    }
    out
}

/// The clauses of a text: split at the end of sentences, at ";" and ":",
/// never inside quotes (names like "R.B.").
fn clauses(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0;
    let mut quoted = false;
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '.' | ';' | ':' | '\n' | '\r' if !quoted => {
                out.push(&text[from..i]);
                from = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&text[from..]);
    out
}

/// What a clause does with the cards it names, and from where: the zones
/// after "from your", the Deck first (it brings a new card).
fn clause_kind(clause: &str) -> Option<(LinkKind, Zone)> {
    let at = clause.find("from your ")?;
    let place = &clause[at + 10..];
    let place = &place[..place.find(" to ").unwrap_or(place.len())];
    let zone = if place.contains("extra deck") {
        Zone::Extra
    } else if place.contains("deck") {
        Zone::Deck
    } else if place.contains("gy") || place.contains("graveyard") {
        Zone::Grave
    } else if place.contains("banish") {
        Zone::Banished
    } else if place.contains("hand") {
        Zone::Hand
    } else {
        return None;
    };
    let summons = ["special summon", "fusion summon", "ritual summon", "synchro summon", "xyz summon", "link summon"];
    let kind = if clause.contains("add") && clause.contains("to your hand") {
        if zone == Zone::Deck {
            LinkKind::Search
        } else if matches!(zone, Zone::Grave | Zone::Banished) {
            LinkKind::Recover
        } else {
            return None;
        }
    } else if summons.iter().any(|w| clause.contains(w)) {
        LinkKind::Summon
    } else if clause.contains("send") && clause.contains("to the gy") && zone == Zone::Deck {
        LinkKind::ToGrave
    } else if (clause.contains("set ") || clause.contains("place ")) && zone == Zone::Deck {
        LinkKind::Set
    } else {
        return None;
    };
    Some((kind, zone))
}

/// The kind of card a quoted name stands for, from the word after it.
#[derive(Clone, Copy, PartialEq)]
enum What {
    Any,
    Monster,
    Spell,
    Trap,
    SpellTrap,
}

/// The names in quotes in a clause, with the kind of card each stands for.
fn quoted(clause: &str) -> Vec<(String, What)> {
    let parts: Vec<&str> = clause.split('"').collect();
    (1..parts.len())
        .step_by(2)
        .filter(|&i| !parts[i].trim().is_empty())
        .map(|i| {
            let after = parts.get(i + 1).map_or("", |a| a.trim_start());
            let what = if after.starts_with("spell/trap") || after.starts_with("spell or trap") {
                What::SpellTrap
            } else if after.starts_with("spell") {
                What::Spell
            } else if after.starts_with("trap") {
                What::Trap
            } else if after.starts_with("monster") || after.split_whitespace().nth(1).is_some_and(|w| w.starts_with("monster")) {
                What::Monster
            } else {
                What::Any
            };
            (parts[i].trim().to_owned(), what)
        })
        .collect()
}

/// `target` is a card `name` (in quotes) stands for, of kind `what`, that
/// can come from `zone`: the card of that name, or one whose name holds it
/// when no card has it as its name (an archetype).
fn fits(engine: &Engine, target: &LinkCard, name: &str, what: What, zone: Zone, kind: LinkKind, named: &Names) -> bool {
    let Some(card) = engine.cards().get(target.code) else { return false };
    let Some(own) = named.names.get(&target.code) else { return false };
    let exact = named.names.values().any(|n| n == name);
    let by_name = if exact { own == name } else { own.contains(name) };
    let kind_ok = match what {
        What::Any => true,
        What::Monster => card.type_ & card_type::MONSTER != 0,
        What::Spell => card.type_ & card_type::SPELL != 0,
        What::Trap => card.type_ & card_type::TRAP != 0,
        What::SpellTrap => card.type_ & (card_type::SPELL | card_type::TRAP) != 0,
    };
    // An Extra Deck monster comes from the Extra Deck, and is summoned.
    let zone_ok = match zone {
        Zone::Extra => target.extra,
        Zone::Deck | Zone::Hand => !target.extra,
        Zone::Grave | Zone::Banished => true,
    };
    let kind_fits = kind != LinkKind::Summon || card.type_ & card_type::MONSTER != 0;
    by_name && kind_ok && zone_ok && kind_fits
}

/// Follows a line played again: what causes each move, and its links.
#[derive(Default)]
struct Reader {
    /// Chain link -> card; the links resolving; the card of your last action.
    chain: HashMap<u32, u32>,
    solving: Vec<u32>,
    acting: Option<u32>,
    /// Materials offered and used, waiting for the monster they summon.
    offered_materials: Vec<u32>,
    used_materials: Vec<u32>,
    /// Where each card last moved from.
    moved_from: HashMap<u32, u8>,
    out: Seen,
}

impl Reader {
    /// The card whose effect is resolving, else the one you are using.
    fn cause(&self) -> Option<u32> {
        self.solving.last().and_then(|l| self.chain.get(l)).copied().or(self.acting)
    }

    fn see(&mut self, engine: &Engine, events: &[Event], named: &Names) {
        let db = engine.cards();
        for e in events {
            let cause = self.cause();
            match e {
                Event::Chaining { card, chain_link, .. } => {
                    self.chain.insert(*chain_link, db.canonical_code(card.code));
                }
                Event::ChainSolving { chain_link } => self.solving.push(u32::from(*chain_link)),
                Event::ChainSolved { .. } => {
                    self.solving.pop();
                }
                Event::ChainEnd => {
                    self.chain.clear();
                    self.solving.clear();
                }
                Event::Move { code, from, to, reason: why } if to.controller == 0 => {
                    let code = db.canonical_code(*code);
                    self.moved_from.insert(code, from.location);
                    let kind = match (from.location, to.location) {
                        (location::DECK, location::HAND) => Some(LinkKind::Search),
                        (location::DECK, location::GRAVE) => Some(LinkKind::ToGrave),
                        (location::DECK, location::SZONE) => Some(LinkKind::Set),
                        (location::GRAVE | location::REMOVED, location::HAND) => Some(LinkKind::Recover),
                        _ => None,
                    };
                    if why & reason::MATERIAL != 0 {
                        self.used_materials.push(code);
                        // A material sent from the Deck (Branded Fusion) is sent to the GY.
                        if let (Some(LinkKind::ToGrave), Some(cause)) = (kind, cause) {
                            self.out.used.insert((cause, code, LinkKind::ToGrave, Zone::Deck));
                        }
                    } else if let (Some(kind), Some(cause), Some(zone)) = (kind, cause, Zone::of(from.location)) {
                        self.out.used.insert((cause, code, kind, zone));
                    }
                }
                Event::SpecialSummoning { card } if card.controller == 0 => {
                    let code = db.canonical_code(card.code);
                    let from = self.moved_from.get(&code).and_then(|&l| Zone::of(l));
                    if from == Some(Zone::Extra) {
                        self.out.summoned.insert(code);
                        for &m in self.offered_materials.iter().filter(|&&m| named.named(m, code)) {
                            self.out.possible.insert((m, code, LinkKind::Material, Zone::Extra));
                        }
                        for &m in self.used_materials.iter().filter(|&&m| named.named(m, code)) {
                            self.out.used.insert((m, code, LinkKind::Material, Zone::Extra));
                        }
                    }
                    self.offered_materials.clear();
                    self.used_materials.clear();
                    if let (Some(cause), Some(zone)) = (cause.filter(|&c| c != code), from) {
                        self.out.used.insert((cause, code, LinkKind::Summon, zone));
                    }
                }
                _ => {}
            }
        }
    }

    /// The cards a decision of yours offers to pick.
    fn offered(&mut self, engine: &Engine, decision: &Decision) {
        let db = engine.cards();
        let (cards, hint): (Vec<&CardRef>, u64) = match decision {
            Decision::SelectCard { cards, hint, .. } => (cards.iter().collect(), *hint),
            Decision::SelectUnselectCard { selectable, hint, .. } => (selectable.iter().collect(), *hint),
            Decision::SelectTribute { cards, hint, .. } => (cards.iter().map(|(c, _)| c).collect(), *hint),
            Decision::SelectSum { must, cards, hint, .. } => (must.iter().chain(cards).map(|(c, _)| c).collect(), *hint),
            _ => return,
        };
        let cause = self.cause();
        for c in cards.into_iter().filter(|c| c.controller == 0) {
            let code = db.canonical_code(c.code);
            // The select hints of the scripts (HINTMSG_*).
            let kind = match (hint, c.location) {
                (506, location::DECK) => Some(LinkKind::Search),
                (505 | 506, location::GRAVE | location::REMOVED) => Some(LinkKind::Recover),
                (504, location::DECK) => Some(LinkKind::ToGrave),
                (509, l) if l & (location::MZONE | location::SZONE) == 0 => Some(LinkKind::Summon),
                (510 | 527, location::DECK) => Some(LinkKind::Set),
                (511..=513 | 533, l) => {
                    self.offered_materials.push(code);
                    // Materials from the Deck go to the GY.
                    (l == location::DECK).then_some(LinkKind::ToGrave)
                }
                _ => None,
            };
            if let (Some(kind), Some(cause), Some(zone)) = (kind, cause, Zone::of(c.location)) {
                self.out.possible.insert((cause, code, kind, zone));
            }
        }
    }
}

/// Plays `line` again and reads its links. `None` when it does not play.
fn play(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    opponent: Option<&Opponent>,
    line: &Line,
    named: &Names,
) -> Option<Seen> {
    let mut duel = Duel::goldfish_against(engine, deck, hand, seed, opponent).ok()?;
    let mut reader = Reader::default();
    for answer in &line.answers {
        let Ok(Step::Decision { decision, events }) = duel.advance() else { return None };
        reader.see(engine, &events, named);
        if decision.player() == Some(0) {
            reader.offered(engine, &decision);
            // The card of an action you choose, a trigger you say yes to
            // included: its costs and its summon are its own.
            match &decision {
                Decision::IdleCmd { .. } | Decision::SelectChain { .. } => {
                    let actions = duel.actions(&decision);
                    if let Some(card) = actions.iter().find(|a| &a.response == answer).and_then(|a| a.card) {
                        reader.acting = Some(engine.cards().canonical_code(card.code));
                    }
                }
                Decision::EffectYesNo { card, .. } if answer.first().is_some_and(|&b| b != 0) => {
                    reader.acting = Some(engine.cards().canonical_code(card.code));
                }
                _ => {}
            }
        }
        duel.respond(answer);
    }
    if let Ok(Step::Decision { events, .. } | Step::End { events }) = duel.advance() {
        reader.see(engine, &events, named);
    }
    Some(reader.out)
}
