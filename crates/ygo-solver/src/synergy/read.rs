//! The reading deck (plan `docs/plans/sinergie-archetipi.md` §3.3 (a)):
//! which cards of the whole pool an effect that picks cards accepts, as the
//! engine offers them. The Deck and the Extra Deck of the duel hold every
//! card of the pool; the card read is in the hand with cards that do
//! nothing, and its summons and effects are played one after the other.
//! Whenever a choice asks to pick cards, the cards offered are what the
//! effect accepts, with the script's conditions applied: the same rule as
//! the map of the links ("what the engine offers"), on the whole pool.

use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

use ygo_engine::consts::{card_type, location};
use ygo_engine::{ActionKind, CardRef, Decision, DeckList, Duel, Engine, Step};

/// Cards in the hand next to the one read. They do nothing by themselves
/// and give the costs something to take: Ritual Spells without their
/// Ritual Monster (as in the roles) to discard, and a Normal Monster of no
/// archetype to discard, Tribute or reveal (Small World).
const BLANK_SPELL: &str = "Black Luster Ritual";
const BLANK_MONSTER: &str = "Mystical Elf";

/// Every card a deck can hold: one copy of each, alternate artworks and
/// tokens left out. A card needs a script, unless it is a Normal Monster
/// (most have none). A card always treated as another ("Fallen of the
/// White Dragon") is a card of its own: its alias is far from its code.
#[derive(Debug, Clone, Default)]
pub struct Pool {
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
}

impl Pool {
    pub fn new(engine: &Engine) -> Self {
        let db = engine.cards();
        let mut cards: Vec<_> = db
            .iter()
            .filter(|c| db.canonical_code(c.code) == c.code && c.type_ & card_type::TOKEN == 0)
            .filter(|c| c.type_ & (card_type::MONSTER | card_type::SPELL | card_type::TRAP) != 0)
            .filter(|c| engine.scripts().has_card_script(c.code) || c.type_ & card_type::NORMAL != 0)
            .collect();
        cards.sort_by_key(|c| c.code);
        let (extra, main): (Vec<_>, Vec<_>) = cards.into_iter().partition(|c| c.is_extra_deck_monster());
        Pool { main: main.into_iter().map(|c| c.code).collect(), extra: extra.into_iter().map(|c| c.code).collect() }
    }

    pub fn len(&self) -> usize {
        self.main.len() + self.extra.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The reading deck: the whole pool.
    pub fn deck(&self) -> DeckList {
        DeckList { main: self.main.clone(), extra: self.extra.clone(), side: Vec::new() }
    }
}

/// One kind of choice met while reading a card: the select hint of the
/// script (`HINTMSG_*`, 506 "add to the hand", 509 "Special Summon"...),
/// where the cards offered are, and every card offered there (canonical
/// codes, sorted), over all the times it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub hint: u64,
    pub location: u8,
    pub cards: Vec<u32>,
}

impl Offer {
    /// What the hint says the choice is for.
    pub fn purpose(&self) -> &'static str {
        match self.hint {
            500 | 531 => "tribute",
            501 => "discard",
            503 => "banish",
            504 => "send to the GY",
            505 | 506 => "add to the hand",
            507 => "return to the Deck",
            509 => "Special Summon",
            510 | 527 => "set or place",
            511..=513 | 533 => "material",
            526 => "reveal",
            _ => "other",
        }
    }
}

/// What a card's effects accept, read in the reading deck.
#[derive(Debug, Clone, Default)]
pub struct Reading {
    pub code: u32,
    /// By hint and location; none when the card cannot act from the hand
    /// or its effects pick no card.
    pub offers: Vec<Offer>,
    /// Turns played: one for each summon or effect the card has in the
    /// first open moment, which it uses first.
    pub walks: usize,
    /// The summons and effects of the card offered in the first open moment.
    pub actions: usize,
    pub decisions: usize,
    /// Making the duels, and all the reading (making the duels included).
    pub setup: Duration,
    pub time: Duration,
    pub error: Option<String>,
}

/// How much is played to read a card.
#[derive(Debug, Clone, Copy)]
pub struct ReadBudget {
    /// Turns played, at most (one per action of the first open moment).
    pub walks: usize,
    /// Decisions of a turn, at most.
    pub decisions: usize,
}

impl Default for ReadBudget {
    fn default() -> Self {
        ReadBudget { walks: 4, decisions: 200 }
    }
}

/// The cards in the hand next to the card read: three Spells and a Normal
/// Monster, all doing nothing. Empty if the database has neither.
pub fn blanks(engine: &Engine) -> Vec<u32> {
    let db = engine.cards();
    let (Some(spell), Some(monster)) = (db.find_by_name(BLANK_SPELL), db.find_by_name(BLANK_MONSTER)) else { return Vec::new() };
    vec![spell.code, spell.code, spell.code, monster.code]
}

/// Reads `code` in `deck` (the card is added to it, with the blanks).
/// `prefer` are the cards picked first when an effect asks to pick: the
/// cards of a real deck, so that a reading in the whole pool makes the
/// same choices as the reading in that deck, and offers can be compared.
pub fn read_card(engine: &Engine, deck: &DeckList, code: u32, prefer: &HashSet<u32>, budget: ReadBudget, seed: u64) -> Reading {
    let started = Instant::now();
    let db = engine.cards();
    let probe = db.canonical_code(code);
    let blanks = blanks(engine);
    let mut deck = deck.clone();
    let is_extra = db.get(probe).is_some_and(|c| c.is_extra_deck_monster());
    if is_extra {
        return Reading { code: probe, error: Some("an Extra Deck monster is not read from the hand".to_owned()), ..Reading::default() };
    }
    deck.main.push(probe);
    deck.main.extend(&blanks);
    let mut hand = vec![probe];
    hand.extend(&blanks);

    let mut reading = Reading { code: probe, ..Reading::default() };
    let mut offers: BTreeMap<(u64, u8), HashSet<u32>> = BTreeMap::new();
    let ignore: HashSet<u32> = blanks.iter().copied().chain([probe]).collect();
    let mut walk = 0;
    loop {
        let made = Instant::now();
        let mut duel = match Duel::goldfish(engine, &deck, &hand, seed) {
            Ok(duel) => duel,
            Err(e) => {
                reading.error = Some(e.to_string());
                break;
            }
        };
        reading.setup += made.elapsed();
        let mut walker = Walker { engine, probe, prefer, ignore: &ignore, walk, first: None, used: HashSet::new(), offers: &mut offers };
        let played = walker.play(&mut duel, budget.decisions);
        let first = walker.first;
        reading.walks += 1;
        match played {
            Ok(decisions) => reading.decisions += decisions,
            Err(e) => {
                reading.error = Some(e);
                break;
            }
        }
        reading.actions = first.unwrap_or(0);
        walk += 1;
        if walk >= first.unwrap_or(0).min(budget.walks) {
            break;
        }
    }
    reading.offers = offers
        .into_iter()
        .filter(|(_, cards)| !cards.is_empty())
        .map(|((hint, location), cards)| {
            let mut cards: Vec<u32> = cards.into_iter().collect();
            cards.sort_unstable();
            Offer { hint, location, cards }
        })
        .collect();
    reading.time = started.elapsed();
    reading
}

/// Plays one turn of the card read: in the first open moment its `walk`-th
/// summon or effect, then any other of its own not used yet; its triggers
/// always, nothing else of the deck's. Picks the preferred cards first,
/// else the lowest codes; says yes to its questions.
struct Walker<'a> {
    engine: &'a Engine,
    probe: u32,
    prefer: &'a HashSet<u32>,
    ignore: &'a HashSet<u32>,
    walk: usize,
    /// The card's actions in the first open moment, once it is met.
    first: Option<usize>,
    /// The card's summons and effects used in this turn (kind, description).
    used: HashSet<(ActionKind, u64)>,
    offers: &'a mut BTreeMap<(u64, u8), HashSet<u32>>,
}

impl Walker<'_> {
    fn play(&mut self, duel: &mut Duel, max: usize) -> Result<usize, String> {
        for decisions in 0..max {
            let decision = match duel.advance().map_err(|e| e.to_string())? {
                Step::End { .. } => return Ok(decisions),
                Step::Decision { decision, .. } => decision,
            };
            let Some(answer) = self.answer(duel, &decision) else { return Ok(decisions) };
            duel.respond(&answer);
        }
        Ok(max)
    }

    fn is_probe(&self, card: &CardRef) -> bool {
        self.engine.cards().canonical_code(card.code) == self.probe
    }

    /// `None` ends the turn here: the walk is done.
    fn answer(&mut self, duel: &Duel, decision: &Decision) -> Option<Vec<u8>> {
        if decision.player() != Some(0) {
            return Some(ygo_engine::auto::passive_answer(decision, &duel.actions(decision)));
        }
        self.record(decision);
        match decision {
            Decision::SelectCard { min, cards, .. } => Some(indices(&self.picks(cards, (*min).max(1) as usize))),
            Decision::SelectUnselectCard { finishable, selectable, .. } => {
                if *finishable || selectable.is_empty() {
                    return Some((-1i32).to_le_bytes().to_vec());
                }
                let mut answer = 1i32.to_le_bytes().to_vec();
                answer.extend_from_slice(&(self.picks(selectable, 1)[0] as i32).to_le_bytes());
                Some(answer)
            }
            Decision::IdleCmd { .. } => {
                let actions = duel.actions(decision);
                let own = |a: &&ygo_engine::Action| {
                    a.card.is_some_and(|c| self.is_probe(&c))
                        && matches!(a.kind, ActionKind::Activate | ActionKind::NormalSummon | ActionKind::SpecialSummon)
                };
                let mine: Vec<&ygo_engine::Action> = actions.iter().filter(own).collect();
                let pick = match self.first {
                    None => {
                        self.first = Some(mine.len());
                        mine.get(self.walk).copied()
                    }
                    Some(_) => mine.iter().find(|a| !self.used.contains(&(a.kind, a.description))).copied(),
                };
                match pick {
                    Some(a) => {
                        self.used.insert((a.kind, a.description));
                        Some(a.response.clone())
                    }
                    None => None,
                }
            }
            Decision::SelectChain { .. } => {
                let actions = duel.actions(decision);
                let own = actions.iter().find(|a| a.kind == ActionKind::Activate && a.card.is_some_and(|c| self.is_probe(&c)));
                Some(match own {
                    Some(a) => a.response.clone(),
                    None => ygo_engine::auto::passive_answer(decision, &actions),
                })
            }
            Decision::EffectYesNo { card, .. } => Some(i32::from(self.is_probe(card)).to_le_bytes().to_vec()),
            Decision::YesNo { .. } => Some(1i32.to_le_bytes().to_vec()),
            _ => {
                let actions = duel.actions(decision);
                let forward = actions.iter().find(|a| !matches!(a.kind, ActionKind::Cancel | ActionKind::Pass | ActionKind::No));
                Some(match forward.or(actions.first()) {
                    Some(a) => a.response.clone(),
                    None => ygo_engine::auto::passive_answer(decision, &actions),
                })
            }
        }
    }

    /// The indices of the `count` cards to pick: the preferred ones first,
    /// then the lowest codes (the same choice in any deck holding them).
    fn picks(&self, cards: &[CardRef], count: usize) -> Vec<usize> {
        let db = self.engine.cards();
        let mut order: Vec<usize> = (0..cards.len()).collect();
        order.sort_by_key(|&i| {
            let code = db.canonical_code(cards[i].code);
            (!self.prefer.contains(&code), code, cards[i].location, cards[i].sequence)
        });
        order.truncate(count.min(cards.len()));
        order
    }

    /// The cards of yours a choice offers, off the field.
    fn record(&mut self, decision: &Decision) {
        let (cards, hint): (Vec<&CardRef>, u64) = match decision {
            Decision::SelectCard { cards, hint, .. } => (cards.iter().collect(), *hint),
            Decision::SelectUnselectCard { selectable, unselectable, hint, .. } => (selectable.iter().chain(unselectable).collect(), *hint),
            Decision::SelectTribute { cards, hint, .. } => (cards.iter().map(|(c, _)| c).collect(), *hint),
            Decision::SelectSum { must, cards, hint, .. } => (must.iter().chain(cards).map(|(c, _)| c).collect(), *hint),
            _ => return,
        };
        let db = self.engine.cards();
        for c in cards {
            let code = db.canonical_code(c.code);
            if c.controller != 0 || c.location & (location::MZONE | location::SZONE | location::OVERLAY) != 0 || self.ignore.contains(&code)
            {
                continue;
            }
            self.offers.entry((hint, c.location)).or_default().insert(code);
        }
    }
}

/// The answer that picks the cards at `picks` in a card choice.
fn indices(picks: &[usize]) -> Vec<u8> {
    let mut out = 0i32.to_le_bytes().to_vec();
    out.extend_from_slice(&(picks.len() as u32).to_le_bytes());
    for &i in picks {
        out.extend_from_slice(&(i as u32).to_le_bytes());
    }
    out
}

/// How a reading in the whole pool compares with one in a real deck: for
/// each offer of the deck's reading, the pool's offer restricted to the
/// deck's cards must be the same.
#[derive(Debug, Clone, Default)]
pub struct Agreement {
    /// Offers (hint and location) in either reading, and those the same.
    pub offers: usize,
    pub same: usize,
    /// Cards offered only in the deck, and only in the pool (of the deck's cards).
    pub only_deck: Vec<u32>,
    pub only_pool: Vec<u32>,
}

impl Agreement {
    pub fn agrees(&self) -> bool {
        self.offers == self.same
    }
}

pub fn compare(pool: &Reading, deck: &Reading, deck_cards: &HashSet<u32>) -> Agreement {
    let key = |o: &Offer| (o.hint, o.location);
    let mut keys: Vec<(u64, u8)> = pool.offers.iter().map(key).chain(deck.offers.iter().map(key)).collect();
    keys.sort_unstable();
    keys.dedup();
    let mut out = Agreement::default();
    for k in keys {
        let find = |r: &Reading| -> HashSet<u32> {
            r.offers
                .iter()
                .find(|o| key(o) == k)
                .map(|o| o.cards.iter().copied().filter(|c| deck_cards.contains(c)).collect())
                .unwrap_or_default()
        };
        let (p, d) = (find(pool), find(deck));
        if p.is_empty() && d.is_empty() {
            continue;
        }
        out.offers += 1;
        if p == d {
            out.same += 1;
        }
        out.only_deck.extend(d.difference(&p));
        out.only_pool.extend(p.difference(&d));
    }
    out.only_deck.sort_unstable();
    out.only_pool.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(offers: &[(u64, u8, &[u32])]) -> Reading {
        let offers = offers.iter().map(|&(hint, location, cards)| Offer { hint, location, cards: cards.to_vec() }).collect();
        Reading { offers, ..Reading::default() }
    }

    #[test]
    fn the_pool_agrees_when_its_offer_restricted_to_the_deck_is_the_deck_s() {
        let deck: HashSet<u32> = [1, 2, 3].into_iter().collect();
        // The pool offers 1, 2 and cards out of the deck; the deck 1 and 2.
        let pool = reading(&[(506, location::DECK, &[1, 2, 7, 9])]);
        assert!(compare(&pool, &reading(&[(506, location::DECK, &[1, 2])]), &deck).agrees());
        // The deck offers 3 too: the pool missed it.
        let missed = compare(&pool, &reading(&[(506, location::DECK, &[1, 2, 3])]), &deck);
        assert!(!missed.agrees());
        assert_eq!(missed.only_deck, vec![3]);
        // An offer of cards out of the deck only is no offer in the deck.
        let out = reading(&[(506, location::DECK, &[1, 2]), (504, location::DECK, &[8])]);
        assert!(compare(&out, &reading(&[(506, location::DECK, &[1, 2])]), &deck).agrees());
    }
}
