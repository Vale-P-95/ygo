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
use ygo_engine::{CardDb, CardRef, Decision, DeckList, Duel, Engine, Event, Opponent, Step};

use crate::card::{sample_hands, Rng};
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

/// What changes from the map of one version of a deck to another's.
#[derive(Debug, Clone, Default)]
pub struct LinksDiff {
    /// Links only in the new version, and only in the old one (by cards and kind).
    pub gained: Vec<CardLink>,
    pub lost: Vec<CardLink>,
    /// Cards in both whose chance, key or reach changed, the most changed first.
    pub changed: Vec<CardChange>,
    /// Cards put in and taken out.
    pub added: Vec<u32>,
    pub removed: Vec<u32>,
    /// Cards an opening hand reaches on average (the sum of the chances), before and after.
    pub reached: (f64, f64),
}

/// A card in both versions: its numbers before and after.
#[derive(Debug, Clone)]
pub struct CardChange {
    pub code: u32,
    pub chance: (f64, f64),
    pub key: (f64, f64),
    pub reach_all: (u32, u32),
}

/// Smaller changes of a card's chance and key are noise of the hands searched.
const CHANGE: f64 = 0.02;

/// Compares the map of a deck (`old`) with that of another version (`new`).
pub fn compare_links(old: &DeckLinks, new: &DeckLinks) -> LinksDiff {
    let id = |l: &CardLink| (l.from, l.to, l.kind);
    let before: HashSet<(u32, u32, LinkKind)> = old.links.iter().map(id).collect();
    let after: HashSet<(u32, u32, LinkKind)> = new.links.iter().map(id).collect();
    let card = |map: &DeckLinks, code: u32| map.cards.iter().find(|c| c.code == code).cloned();
    let mut changed: Vec<CardChange> = new
        .cards
        .iter()
        .filter_map(|n| {
            let o = card(old, n.code)?;
            let moved = (n.chance - o.chance).abs() >= CHANGE || (n.key - o.key).abs() >= CHANGE || n.reach_all != o.reach_all;
            moved.then_some(CardChange {
                code: n.code,
                chance: (o.chance, n.chance),
                key: (o.key, n.key),
                reach_all: (o.reach_all, n.reach_all),
            })
        })
        .collect();
    let size = |c: &CardChange| (c.chance.1 - c.chance.0).abs() + (c.key.1 - c.key.0).abs();
    changed.sort_by(|a, b| size(b).total_cmp(&size(a)).then(a.code.cmp(&b.code)));
    let reached = |map: &DeckLinks| map.cards.iter().map(|c| c.chance).sum::<f64>();
    LinksDiff {
        gained: new.links.iter().filter(|l| !before.contains(&id(l))).cloned().collect(),
        lost: old.links.iter().filter(|l| !after.contains(&id(l))).cloned().collect(),
        changed,
        added: new.cards.iter().filter(|c| card(old, c.code).is_none()).map(|c| c.code).collect(),
        removed: old.cards.iter().filter(|c| card(new, c.code).is_none()).map(|c| c.code).collect(),
        reached: (reached(old), reached(new)),
    }
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
            hands_per_card: 3,
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

/// The links of one line: those offered and those used; the cards of
/// yours it brings into play (held in the hand, added to it, or on the
/// field at some point).
#[derive(Default, Clone)]
struct Seen {
    possible: HashSet<Key>,
    used: HashSet<Key>,
    touched: HashSet<u32>,
}

/// The hands searched for a map and the lines played from each: kept to
/// read the map again after the deck changes, searching only the hands
/// the change needs ([`deck_links_again`]).
#[derive(Debug, Clone, Default)]
pub struct LinksWork {
    hands: Vec<(Vec<u32>, Vec<Seen>)>,
}

impl LinksWork {
    /// Each hand searched with the cards each of its lines brings into
    /// play, the best line first.
    pub fn reached(&self) -> Vec<HandReach> {
        self.hands.iter().map(|(hand, lines)| HandReach::new(hand, lines)).collect()
    }
}

/// A hand and the cards each of its lines (the best first) brings into
/// play: in the hand, added to it, on the field.
#[derive(Debug, Clone)]
pub struct HandReach {
    pub hand: Vec<u32>,
    pub lines: Vec<Vec<u32>>,
}

impl HandReach {
    fn new(hand: &[u32], lines: &[Seen]) -> Self {
        let lines = lines
            .iter()
            .map(|l| {
                let mut codes: Vec<u32> = l.touched.iter().copied().collect();
                codes.sort_unstable();
                codes
            })
            .collect();
        HandReach { hand: hand.to_vec(), lines }
    }
}

impl std::fmt::Debug for Seen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} links used", self.used.len())
    }
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
    deck_links_again(engine, deck, seed, budget, cancel, progress, None).0
}

/// [`deck_links`], reusing what was read for an earlier version of the
/// deck: the hands whose cards are all still in it and whose lines use no
/// card taken out are kept, and only the hands needed to give each card
/// its hands again are searched (a card or two changed: a few seconds).
/// The lines of a kept hand do not see the cards added; those get their
/// own hands, and the text names what they lead to.
pub fn deck_links_again(
    engine: &Engine,
    deck: &DeckList,
    seed: u64,
    budget: LinksBudget,
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize, &DeckLinks) + Sync,
    previous: Option<&LinksWork>,
) -> (DeckLinks, LinksWork) {
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
        // A seed of its own for each card: with one seed for all, the
        // other four cards were drawn at the same places of the deck, and
        // most hands of the map were the same hand but one card.
        let own = seed.wrapping_add(u64::from(card.code).wrapping_mul(7919));
        per_card.push(sample_hands(&deck.main, copy, budget.hands_per_card, own).into_iter().map(|(hand, _)| hand).collect());
    }
    // The hands kept from before: their cards still in the deck, their
    // lines using none taken out.
    let in_deck: HashSet<u32> = cards.iter().map(|c| c.code).collect();
    let copies: HashMap<u32, u32> = cards.iter().map(|c| (c.code, c.copies)).collect();
    let fits = |hand: &[u32]| {
        let mut held: HashMap<u32, u32> = HashMap::new();
        for &c in hand {
            *held.entry(db.canonical_code(c)).or_default() += 1;
        }
        held.iter().all(|(c, n)| copies.get(c).is_some_and(|k| k >= n))
    };
    let kept: Vec<(Vec<u32>, Vec<Seen>)> = previous
        .map(|w| {
            w.hands
                .iter()
                .filter(|(hand, lines)| {
                    fits(hand) && lines.iter().all(|l| l.used.iter().all(|(from, to, _, _)| in_deck.contains(from) && in_deck.contains(to)))
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let holds = |hand: &[u32], code: u32| hand.iter().any(|&c| db.canonical_code(c) == code);
    let mut hands: Vec<Vec<u32>> = Vec::new();
    for round in 0..budget.hands_per_card {
        for (card, sampled) in cards.iter().filter(|c| !c.extra).zip(&per_card) {
            // A card with its hands among those kept needs no new one.
            let have = kept.iter().filter(|(h, _)| holds(h, card.code)).count() + hands.iter().filter(|h| holds(h, card.code)).count();
            if have > round {
                continue;
            }
            if let Some(hand) = sampled.get(round).filter(|h| !hands.contains(h) && !kept.iter().any(|(k, _)| k == *h)) {
                hands.push(hand.clone());
            }
        }
    }
    progress(0, hands.len(), &assemble(db, deck, &cards, &kept, &text));
    let deadline = Instant::now() + budget.time;

    let next = AtomicUsize::new(0);
    // The lines played of each hand, the best first: those kept, then each
    // hand searched.
    let kept_count = kept.len();
    let seen: Mutex<Vec<(Vec<u32>, Vec<Seen>)>> = Mutex::new(kept);
    std::thread::scope(|s| {
        for _ in 0..budget.threads.clamp(1, hands.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(hand) = hands.get(i) else { break };
                let left = deadline.saturating_duration_since(Instant::now());
                if cancel.load(Ordering::Relaxed) || left.is_zero() {
                    break;
                }
                let lines = search_hand(engine, deck, hand, seed, &budget, left, &kit, &traps, &named, cancel);
                if let Ok(mut all) = seen.lock() {
                    all.push((hand.clone(), lines));
                    progress(all.len() - kept_count, hands.len(), &assemble(db, deck, &cards, &all, &text));
                }
            });
        }
    });
    let work = LinksWork { hands: seen.into_inner().unwrap_or_default() };
    (assemble(db, deck, &cards, &work.hands, &text), work)
}

/// Searches `hand` for at most `left` and plays its best lines again.
#[allow(clippy::too_many_arguments)]
fn search_hand(
    engine: &Engine,
    deck: &DeckList,
    hand: &[u32],
    seed: u64,
    budget: &LinksBudget,
    left: Duration,
    kit: &Kit,
    traps: &[u32],
    named: &Names,
    cancel: &AtomicBool,
) -> Vec<Seen> {
    let mut config = SearchConfig::new(deck.clone(), hand.to_vec());
    config.seed = seed;
    config.threads = 1;
    config.max_nodes = budget.nodes_per_hand;
    config.time_limit = budget.time_per_hand.min(left);
    config.opponent = Some(kit.opponent().clone());
    config.hand_traps = traps.to_vec();
    let result = search(engine, &config, cancel, |_| {});
    result
        .boards
        .iter()
        .take(budget.lines_per_hand)
        .filter_map(|b| play(engine, deck, hand, seed, config.opponent.as_ref(), &b.line, named))
        .collect()
}

/// `count` opening hands of 5 drawn at random from `main`: each hand as
/// likely as in a real duel.
pub fn random_hands(main: &[u32], count: usize, seed: u64) -> Vec<Vec<u32>> {
    let mut rng = Rng(seed ^ 0x0ba5_e11e);
    (0..count)
        .map(|_| {
            let mut pool = main.to_vec();
            let mut hand: Vec<u32> = (0..5.min(pool.len())).map(|_| pool.swap_remove(rng.below(pool.len()))).collect();
            hand.sort_unstable();
            hand
        })
        .collect()
}

/// Searches each hand as the map does (same budget per hand, no limit on
/// the whole) and reads what its lines bring into play: the measure the
/// chances of the map are checked against, from hands drawn at random.
pub fn hands_reach(
    engine: &Engine,
    deck: &DeckList,
    hands: &[Vec<u32>],
    seed: u64,
    budget: LinksBudget,
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize) + Sync,
) -> Vec<HandReach> {
    let db = engine.cards();
    let mut cards: Vec<LinkCard> = Vec::new();
    for (codes, extra) in [(&deck.main, false), (&deck.extra, true)] {
        for &c in codes {
            let code = db.canonical_code(c);
            if !cards.iter().any(|k| k.code == code) {
                cards.push(LinkCard { code, copies: 1, extra, reach: 0, reach_all: 0, chance: 0.0, measured: false, key: 0.0 });
            }
        }
    }
    let named = Names::new(engine, &cards);
    let kit = Kit::standard(engine);
    let traps = hand_traps(engine, deck, seed, &kit);
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<HandReach>>> = Mutex::new(vec![None; hands.len()]);
    let done = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..budget.threads.clamp(1, hands.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(hand) = hands.get(i) else { break };
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let lines = search_hand(engine, deck, hand, seed, &budget, budget.time_per_hand, &kit, &traps, &named, cancel);
                if let Ok(mut all) = out.lock() {
                    all[i] = Some(HandReach::new(hand, &lines));
                }
                progress(done.fetch_add(1, Ordering::Relaxed) + 1, hands.len());
            });
        }
    });
    out.into_inner().unwrap_or_default().into_iter().flatten().collect()
}

/// The map from the hands searched, the lines played of each, and the
/// links the text names.
fn assemble(db: &CardDb, deck: &DeckList, cards: &[LinkCard], by_hand: &[(Vec<u32>, Vec<Seen>)], text: &HashSet<Key>) -> DeckLinks {
    let mut cards = cards.to_vec();
    let seen: Vec<&Seen> = by_hand.iter().flat_map(|(_, lines)| lines).collect();

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
    // The draft's chance, before any hand is searched: a hand holding the
    // card or one card that brings it in one step. Following the links in
    // more steps reaches almost the whole deck, which one turn cannot play.
    let one_step: HashMap<u32, HashSet<u32>> = codes.iter().map(|&c| (c, brings(c))).collect();
    let measure = Measure::new(db, &copies, by_hand);
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
        let held = if card.extra { 0.0 } else { 1.0 - none_drawn(main_size, card.copies, 5) };
        let draft = chance(card.code, &one_step);
        match measure.share(card.code) {
            Some(share) => {
                card.chance = held + (1.0 - held) * share;
                card.measured = true;
            }
            None => card.chance = draft,
        }
        card.key = keys[&card.code];
    }
    DeckLinks { cards, links, hands: by_hand.len() as u32, lines: seen.len() as u32 }
}

/// What the best lines of the hands searched bring into play, each hand
/// weighted back to its chance in a real duel. The map draws hands holding
/// a given card (that card and 4 at random): such a hand comes up as often
/// as it holds copies of the card, over the card's copies in the deck. So
/// over all the cards, a hand counts 1 / (sum over its cards of
/// 1 / copies), and the weighted shares are those of hands drawn at random.
struct Measure {
    /// Weight, cards held, cards the best line brings into play.
    hands: Vec<(f64, HashSet<u32>, HashSet<u32>)>,
}

impl Measure {
    fn new(db: &CardDb, copies: &HashMap<u32, u32>, by_hand: &[(Vec<u32>, Vec<Seen>)]) -> Self {
        let hands = by_hand
            .iter()
            .map(|(hand, lines)| {
                let held: Vec<u32> = hand.iter().map(|&c| db.canonical_code(c)).collect();
                let weight = 1.0 / held.iter().map(|c| 1.0 / f64::from(copies.get(c).copied().unwrap_or(1).max(1))).sum::<f64>();
                let mut brought: HashSet<u32> = lines.first().map(|l| l.touched.clone()).unwrap_or_default();
                brought.extend(&held);
                (weight, held.into_iter().collect(), brought)
            })
            .collect();
        Measure { hands }
    }

    /// The share of the hands without `code` whose best line brings it into
    /// play (holding it is counted exactly apart). `None` with no such hand.
    /// Pulling it toward the draft's share while few hands are searched
    /// helps the Main Deck a little and hurts the Extra Deck, where the
    /// draft is poor (checked against hands drawn at random,
    /// `scripts/links_chance.py`): left out.
    fn share(&self, code: u32) -> Option<f64> {
        let (total, got) = self
            .hands
            .iter()
            .filter(|(_, held, _)| !held.contains(&code))
            .fold((0.0, 0.0), |(t, g), (w, _, brought)| (t + w, g + if brought.contains(&code) { *w } else { 0.0 }));
        (total > 0.0).then(|| got / total)
    }
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
    let is_monster = |code: u32| db.get(code).is_some_and(|c| c.type_ & card_type::MONSTER != 0);
    for monster in cards.iter().filter(|c| c.extra) {
        // A material is a monster ("R.B. Stage Landing" holds the archetype too).
        for m in cards.iter().filter(|m| is_monster(m.code) && named.named(m.code, monster.code)) {
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
    // A restriction ("you cannot Special Summon monsters from your Extra
    // Deck, except ...") brings no card.
    if clause[..at].contains("cannot") || clause[..at].contains("can't") {
        return None;
    }
    // The zones listed right after "from your" ("hand, Deck, or GY"), not
    // the words of the rest of the clause (a restriction on the Extra Deck
    // after a summon from the Deck).
    const ZONE_WORDS: [&str; 13] =
        ["hand", "deck", "main", "extra", "gy", "graveyard", "banishment", "banished", "face-up", "and/or", "or", "and", "your"];
    let place: Vec<&str> = clause[at + 10..]
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '/'))
        .take_while(|w| ZONE_WORDS.contains(w))
        .collect();
    let place = place.join(" ");
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
                // An effect applied later (in the End Phase) belongs to no
                // action of this phase: better no cause than the last card.
                Event::NewPhase { .. } => self.acting = None,
                Event::Move { code, from, to, reason: why } if to.controller == 0 => {
                    let code = db.canonical_code(*code);
                    self.moved_from.insert(code, from.location);
                    if to.location & (location::HAND | location::MZONE | location::SZONE) != 0 {
                        self.out.touched.insert(code);
                    }
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
                    self.out.touched.insert(code);
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
    reader.out.touched.extend(hand.iter().map(|&c| engine.cards().canonical_code(c)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_drawn_for_each_card_weigh_back_to_random_hands() {
        // 3 copies of 1, 2 of 2, one of 3, and 34 other cards.
        let mut main = vec![1, 1, 1, 2, 2, 3];
        main.extend(10..44);
        let copies: HashMap<u32, u32> = main.iter().fold(HashMap::new(), |mut m, &c| {
            *m.entry(c).or_default() += 1;
            m
        });
        // Hands drawn as the map draws them, many for each card; a line
        // brings card 3 in whenever the hand holds card 1.
        let mut by_hand = Vec::new();
        let mut kinds: Vec<u32> = copies.keys().copied().collect();
        kinds.sort_unstable();
        for &card in &kinds {
            for (hand, _) in sample_hands(&main, card, 300, u64::from(card)) {
                let mut line = Seen::default();
                line.touched.extend(&hand);
                if hand.contains(&1) {
                    line.touched.insert(3);
                }
                by_hand.push((hand, vec![line]));
            }
        }
        // Not a real card database: the codes are their own canonical codes.
        let db = CardDb::default();
        let measure = Measure::new(&db, &copies, &by_hand);
        // Among random hands without card 3, those holding card 1.
        let truth = 1.0 - none_drawn(39, 3, 5);
        let share = measure.share(3).expect("hands without card 3");
        assert!((share - truth).abs() < 0.03, "{share} against {truth}");
        // Unweighted, the hands drawn for the 37 cards overweigh the rare ones.
        let plain = by_hand.iter().filter(|(h, _)| !h.contains(&3)).collect::<Vec<_>>();
        let plain = plain.iter().filter(|(h, _)| h.contains(&1)).count() as f64 / plain.len() as f64;
        assert!((plain - truth).abs() > (share - truth).abs(), "{plain} {share} {truth}");
    }

    #[test]
    fn a_clause_reads_the_zones_right_after_from_your() {
        // The zones listed after "from your", not the Extra Deck of a
        // restriction later in the clause.
        let summon = "you can special summon 2 \"d/d\" monsters from your hand and/or deck in defense position, \
                      also you cannot special summon monsters from the extra deck for the rest of this turn";
        assert!(matches!(clause_kind(summon), Some((LinkKind::Summon, Zone::Deck))));
        assert!(matches!(clause_kind("add 1 \"x\" from your gy or banishment to your hand"), Some((LinkKind::Recover, Zone::Grave))));
        assert!(matches!(clause_kind("special summon 1 \"x\" from your main deck"), Some((LinkKind::Summon, Zone::Deck))));
        // A restriction brings no card.
        assert!(clause_kind("you cannot special summon monsters from your extra deck, except \"d/d\" monsters").is_none());
    }
}
