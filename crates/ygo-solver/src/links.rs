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
use std::time::Duration;

use ygo_engine::consts::{location, reason};
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
    /// Gets it back from the GY or banishment to the hand or the Deck.
    Recover,
    /// `from` is a material named by `to` (an Extra Deck monster).
    Material,
}

impl LinkKind {
    pub fn key(self) -> &'static str {
        match self {
            LinkKind::Search => "search",
            LinkKind::Summon => "summon",
            LinkKind::ToGrave => "toGrave",
            LinkKind::Recover => "recover",
            LinkKind::Material => "material",
        }
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
    /// into play in any number of steps. 0 for an Extra Deck monster no
    /// card names: it is summoned with any materials.
    pub chance: f64,
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
}

impl Default for LinksBudget {
    fn default() -> Self {
        LinksBudget { hands_per_card: 2, nodes_per_hand: 1000, lines_per_hand: 20, threads: 4 }
    }
}

/// A link seen in a line: from, to, kind, where `to` came from.
type Key = (u32, u32, LinkKind, Zone);

/// A link over all lines: the first zone seen, offered, the lines using it.
struct Found {
    zone: Zone,
    offered: bool,
    lines: HashSet<usize>,
}

/// The links of one line: those offered and those used.
#[derive(Default)]
struct Seen {
    possible: HashSet<Key>,
    used: HashSet<Key>,
}

/// Reads the map of `deck`, calling `progress(done, total)` after each hand.
pub fn deck_links(
    engine: &Engine,
    deck: &DeckList,
    seed: u64,
    budget: LinksBudget,
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize) + Sync,
) -> DeckLinks {
    let db = engine.cards();
    let mut cards: Vec<LinkCard> = Vec::new();
    for (codes, extra) in [(&deck.main, false), (&deck.extra, true)] {
        for &c in codes {
            let code = db.canonical_code(c);
            match cards.iter_mut().find(|k| k.code == code) {
                Some(k) => k.copies += 1,
                None => cards.push(LinkCard { code, copies: 1, extra, reach: 0, reach_all: 0, chance: 0.0 }),
            }
        }
    }

    // A few hands holding each card of the Main Deck.
    let kit = Kit::standard(engine);
    let traps = hand_traps(engine, deck, seed, &kit);
    let mut hands: Vec<Vec<u32>> = Vec::new();
    for card in cards.iter().filter(|c| !c.extra) {
        let Some(&copy) = deck.main.iter().find(|&&c| db.canonical_code(c) == card.code) else { continue };
        for (hand, _) in sample_hands(&deck.main, copy, budget.hands_per_card, seed) {
            if !hands.contains(&hand) {
                hands.push(hand);
            }
        }
    }

    let named = Names::new(engine, &cards);
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let seen: Mutex<Vec<Seen>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..budget.threads.clamp(1, hands.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(hand) = hands.get(i) else { break };
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let mut config = SearchConfig::new(deck.clone(), hand.clone());
                config.seed = seed;
                config.threads = 1;
                config.max_nodes = budget.nodes_per_hand;
                config.time_limit = Duration::from_secs(60);
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
                    all.extend(lines);
                }
                progress(done.fetch_add(1, Ordering::Relaxed) + 1, hands.len());
            });
        }
    });
    let seen = seen.into_inner().unwrap_or_default();

    // Links between cards of the deck, one per card and kind, from the
    // first zone seen; used ones counted once per line.
    let in_deck: HashSet<u32> = cards.iter().map(|c| c.code).collect();
    let mut found: HashMap<(u32, u32, LinkKind), Found> = HashMap::new();
    for (i, line) in seen.iter().enumerate() {
        for (&(from, to, kind, zone), used) in line.possible.iter().map(|k| (k, false)).chain(line.used.iter().map(|k| (k, true))) {
            let entry = found.entry((from, to, kind)).or_insert(Found { zone, offered: false, lines: HashSet::new() });
            entry.zone = entry.zone.min(zone);
            entry.offered |= !used;
            if used {
                entry.lines.insert(i);
            }
        }
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
        })
        .collect();
    links.sort_by_key(|l| (l.from, l.kind, l.to));

    // Reach in one step; the cards brought into play in any number of
    // steps, and the chance of an opening hand holding one that leads there.
    let next_of = |code: u32| -> HashSet<u32> { links.iter().filter(|l| l.from == code).map(|l| l.to).collect() };
    let brings = |code: u32| -> HashSet<u32> {
        links.iter().filter(|l| l.from == code && matches!(l.zone, Zone::Deck | Zone::Extra)).map(|l| l.to).collect()
    };
    let closure = |code: u32| -> HashSet<u32> {
        let mut seen: HashSet<u32> = HashSet::new();
        let mut todo = vec![code];
        while let Some(c) = todo.pop() {
            for n in brings(c) {
                if n != code && seen.insert(n) {
                    todo.push(n);
                }
            }
        }
        seen
    };
    let closures: HashMap<u32, HashSet<u32>> = cards.iter().map(|c| (c.code, closure(c.code))).collect();
    let main_size = deck.main.len();
    let copies: HashMap<u32, u32> = cards.iter().filter(|c| !c.extra).map(|c| (c.code, c.copies)).collect();
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
        let leading: u32 = copies.iter().filter(|(&c, _)| c == card.code || closures[&c].contains(&card.code)).map(|(_, &n)| n).sum();
        card.chance = 1.0 - none_drawn(main_size, leading, 5);
    }
    DeckLinks { cards, links, hands: hands.len() as u32, lines: seen.len() as u32 }
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
                // The materials come first, before the first sentence ends.
                let materials = text.split(['.', '\n']).next().unwrap_or("").to_owned();
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
                        (location::GRAVE | location::REMOVED, location::HAND | location::DECK) => Some(LinkKind::Recover),
                        _ => None,
                    };
                    if why & reason::MATERIAL != 0 {
                        self.used_materials.push(code);
                    } else if let (Some(kind), Some(cause), Some(zone)) = (kind, cause, Zone::of(from.location)) {
                        self.out.used.insert((cause, code, kind, zone));
                    }
                }
                Event::SpecialSummoning { card } if card.controller == 0 => {
                    let code = db.canonical_code(card.code);
                    let from = self.moved_from.get(&code).and_then(|&l| Zone::of(l));
                    if from == Some(Zone::Extra) {
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
                (505..=507, location::GRAVE | location::REMOVED) => Some(LinkKind::Recover),
                (504, location::DECK) => Some(LinkKind::ToGrave),
                (509, l) if l & (location::MZONE | location::SZONE) == 0 => Some(LinkKind::Summon),
                (511..=513 | 533, _) => {
                    self.offered_materials.push(code);
                    None
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
            // The card of an action you choose: its costs and its summon are its own.
            if matches!(decision, Decision::IdleCmd { .. } | Decision::SelectChain { .. }) {
                let actions = duel.actions(&decision);
                if let Some(card) = actions.iter().find(|a| &a.response == answer).and_then(|a| a.card) {
                    reader.acting = Some(engine.cards().canonical_code(card.code));
                }
            }
        }
        duel.respond(answer);
    }
    if let Ok(Step::Decision { events, .. } | Step::End { events }) = duel.advance() {
        reader.see(engine, &events, named);
    }
    Some(reader.out)
}
