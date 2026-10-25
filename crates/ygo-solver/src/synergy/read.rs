//! The reading deck (plan `docs/plans/sinergie-archetipi.md` §3.3 (a)):
//! which cards of the whole pool an effect that picks cards accepts, as the
//! engine offers them. The Deck and the Extra Deck of the duel hold every
//! card of the pool; the card read is in the hand with cards that do
//! nothing, and its summons and effects are played one after the other.
//! Whenever a choice asks to pick cards, the cards offered are what the
//! effect accepts, with the script's conditions applied: the same rule as
//! the map of the links ("what the engine offers"), on the whole pool.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use ygo_engine::consts::{card_type, location, position};
use ygo_engine::reading::{add_helpers, take_limits, watch_limits, PlayerLimit, HELPER};
use ygo_engine::{ActionKind, CardRef, Decision, DeckList, Duel, Engine, Event, Placement, Step};

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

    /// The GY of the readings from the field and the GY
    /// ([`GraveSample`]). `effects` gives, for each card that may go there,
    /// how many effects it has: the fewest are taken (a Normal Monster has
    /// none); cards not in it are left out.
    pub fn grave_sample(&self, engine: &Engine, effects: &HashMap<u32, usize>) -> GraveSample {
        let db = engine.cards();
        let mut best: HashMap<(u8, u64), (usize, u32)> = HashMap::new();
        let mut members: HashMap<u16, Vec<(usize, u32)>> = HashMap::new();
        for &code in self.main.iter().chain(&self.extra) {
            let (Some(c), Some(&cost)) = (db.get(code), effects.get(&code)) else { continue };
            let mut keys = Vec::new();
            if c.type_ & card_type::MONSTER != 0 {
                keys.push((0, (u64::from(c.attribute) << 32) | c.race));
                keys.push((1, u64::from(c.level & 0xff)));
                keys.push((2, u64::from(c.type_ & (card_type::EXTRA | card_type::RITUAL | card_type::PENDULUM | 0x1000))));
            } else {
                keys.push((3, u64::from(c.type_)));
            }
            for key in keys {
                let entry = best.entry(key).or_insert((cost, code));
                if (cost, code) < *entry {
                    *entry = (cost, code);
                }
            }
            let mut bases: Vec<u16> = c.setcodes.iter().map(|&s| s & 0x0fff).collect();
            bases.dedup();
            for b in bases {
                members.entry(b).or_default().push((cost, code));
            }
        }
        let mut base: Vec<u32> = best.into_values().map(|(_, code)| code).collect();
        base.sort_unstable();
        base.dedup();
        let members = members
            .into_iter()
            .map(|(b, mut cards)| {
                cards.sort_unstable();
                (b, cards.into_iter().take(GRAVE_MEMBERS).map(|(_, code)| code).collect())
            })
            .collect();
        GraveSample { base, members }
    }
}

/// Members of an archetype put in the GY with a card of it.
pub const GRAVE_MEMBERS: usize = 8;

/// The cards in the GY when reading from the field and from the GY. The
/// whole pool there makes every step of the engine check thousands of
/// cards (minutes a reading, growing faster than the GY), so a sample: for
/// each Attribute and Type, each Level, each kind of monster (Extra Deck
/// summon, Ritual, Pendulum, Tuner) and each kind of Spell and Trap, the
/// card with the fewest effects; and with a card, a few members of its own
/// archetypes, which its effects most often take back.
#[derive(Debug, Clone, Default)]
pub struct GraveSample {
    pub base: Vec<u32>,
    pub members: HashMap<u16, Vec<u32>>,
}

impl GraveSample {
    /// The companions of `code` ([`Scene::hand`], [`Scene::field`]): up to
    /// two Main Deck members of its archetypes for the hand (monsters of
    /// Level 4 or lower first), and one monster member for the field; among
    /// the members of the sample, the ones with the fewest effects.
    pub fn companions(&self, db: &ygo_engine::CardDb, code: u32) -> (Vec<u32>, Vec<u32>) {
        let mut members: Vec<u32> = Vec::new();
        for s in db.get(code).map(|c| c.setcodes.as_slice()).unwrap_or_default() {
            for &m in self.members.get(&(s & 0x0fff)).into_iter().flatten() {
                if m != code && !members.contains(&m) {
                    members.push(m);
                }
            }
        }
        let card = |m: &u32| db.get(*m);
        // Monsters of Level 4 or lower first: "Special Summon 1 ... from your hand".
        let mut main: Vec<u32> = members.iter().copied().filter(|m| card(m).is_some_and(|c| !c.is_extra_deck_monster())).collect();
        main.sort_by_key(|m| card(m).map_or(2, |c| u8::from(c.type_ & card_type::MONSTER == 0) * 2 + u8::from(c.level & 0xff > 4)));
        let hand: Vec<u32> = main.into_iter().take(2).collect();
        let field: Vec<u32> = members
            .iter()
            .copied()
            .filter(|m| card(m).is_some_and(|c| c.type_ & card_type::MONSTER != 0) && !hand.contains(m))
            .take(1)
            .collect();
        (hand, field)
    }

    /// The GY of the readings of `code`, sorted.
    pub fn of(&self, db: &ygo_engine::CardDb, code: u32) -> Vec<u32> {
        let mut out = self.base.clone();
        for s in db.get(code).map(|c| c.setcodes.as_slice()).unwrap_or_default() {
            out.extend(self.members.get(&(s & 0x0fff)).into_iter().flatten());
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// One kind of choice met while reading a card: the effect of the card
/// that asked (its description, as its activations carry it), the select
/// hint of the script (`HINTMSG_*`, 506 "add to the hand", 509 "Special
/// Summon"...), where the cards offered are, and every card offered there
/// (canonical codes, sorted), over all the times it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// The chain link of the card being built or resolved; else the summon
    /// or effect the card used last (a summon procedure, a Tribute).
    pub effect: u64,
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
    /// The descriptions of the card's effects activated (its chain links),
    /// sorted: the effects the engine played, whatever they offered.
    pub activated: Vec<u64>,
    /// Summons of the card made (Normal, or by a summon procedure).
    pub summons: usize,
    /// The limits the card's effects put on you ("you cannot Special
    /// Summon, except...").
    pub limits: Vec<PlayerLimit>,
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

/// Where the card read starts, and what the GY holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Place {
    /// In the hand, the GY empty (the reading of F0).
    #[default]
    Hand,
    /// On the field as if properly Summoned or Set: a monster face-up, an
    /// Extra Deck monster Summoned by its own kind of summon, a Spell or
    /// Trap Set (face-up when it stays on the field).
    Field,
    /// In the GY.
    Grave,
}

impl Place {
    pub const ALL: [Place; 3] = [Place::Hand, Place::Field, Place::Grave];

    pub fn name(self) -> &'static str {
        match self {
            Place::Hand => "hand",
            Place::Field => "field",
            Place::Grave => "GY",
        }
    }

    pub fn from_name(name: &str) -> Option<Place> {
        Place::ALL.into_iter().find(|p| p.name().eq_ignore_ascii_case(name))
    }
}

/// How a card is read: where it starts, the cards in the GY besides (a
/// sample of the pool, so that what an effect takes from the GY is offered
/// too), the events helpers make happen to it after its own actions, for
/// its triggers (see [`ygo_engine::reading::add_helpers`]), and companions.
/// The default is the reading of F0: from the hand, with none of these.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scene<'a> {
    pub place: Place,
    pub grave: &'a [u32],
    pub events: &'a [u32],
    /// Companions: cards in the hand, and monsters face-up on the field
    /// (at most two), besides the card read: members of its archetypes, as
    /// many of its effects ask for one ("if you control...", "discard 1...").
    pub hand: &'a [u32],
    pub field: &'a [u32],
    /// A Spell or Trap read from the field is Set even when it stays on the
    /// field once activated (Continuous, Field): its activation is what
    /// needs reading.
    pub set: bool,
}

/// The events of the triggers and Flip effects of a card: what helpers
/// should make happen to it.
pub fn trigger_events(anatomy: &ygo_engine::CardAnatomy) -> Vec<u32> {
    let mut out: Vec<u32> = anatomy
        .effects
        .iter()
        .filter_map(|e| match e.kind {
            ygo_engine::EffectKind::Flip => Some(1001),
            ygo_engine::EffectKind::Trigger { .. } => Some(e.code),
            _ => None,
        })
        .filter(|e| ygo_engine::reading::HELPER_EVENTS.contains(e))
        .collect();
    // Being destroyed also sends to the GY and leaves the field: the
    // triggers that ask how it went there ("destroyed and sent to the GY").
    if out.iter().any(|e| matches!(e, 1014 | 1015)) {
        out.push(1029);
    }
    out.sort_unstable();
    out.dedup();
    out
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
    read_card_in(engine, deck, code, prefer, budget, seed, Scene::default())
}

/// Like [`read_card`], with the card starting where `scene` says.
pub fn read_card_in(
    engine: &Engine,
    deck: &DeckList,
    code: u32,
    prefer: &HashSet<u32>,
    budget: ReadBudget,
    seed: u64,
    scene: Scene,
) -> Reading {
    let started = Instant::now();
    let db = engine.cards();
    let probe = db.canonical_code(code);
    let blanks = blanks(engine);
    let mut deck = deck.clone();
    let Some(card) = db.get(probe) else {
        return Reading { code: probe, error: Some("not in the card database".to_owned()), ..Reading::default() };
    };
    let is_extra = card.is_extra_deck_monster();
    if is_extra && scene.place == Place::Hand {
        return Reading { code: probe, error: Some("an Extra Deck monster is not read from the hand".to_owned()), ..Reading::default() };
    }
    let mut hand = Vec::new();
    let mut placed = Vec::new();
    match scene.place {
        Place::Hand => {
            deck.main.push(probe);
            hand.push(probe);
        }
        Place::Field => {
            let mut p = field_placement(card);
            if scene.set && card.type_ & card_type::MONSTER == 0 {
                p.position = position::FACEDOWN_ATTACK | position::FACEDOWN_DEFENSE;
            }
            // A Flip effect wants the card face-down, to be flipped.
            if scene.events.contains(&1001) && card.type_ & card_type::MONSTER != 0 {
                p.position = position::FACEDOWN_DEFENSE;
            }
            placed.push(p)
        }
        Place::Grave => placed.push(Placement::new(probe, location::GRAVE, 0, position::FACEUP_ATTACK)),
    }
    if scene.place != Place::Hand {
        // The placed card is a copy of its own: the deck keeps the pool's.
        if is_extra {
            deck.extra.push(probe);
        } else {
            deck.main.push(probe);
        }
    }
    deck.main.extend(&blanks);
    hand.extend(&blanks);
    // Companions are copies of their own too.
    for &c in scene.hand {
        deck.main.push(c);
        hand.push(c);
    }
    for (i, &c) in scene.field.iter().enumerate() {
        let extra = db.get(c).is_some_and(|d| d.is_extra_deck_monster());
        if extra {
            deck.extra.push(c);
        } else {
            deck.main.push(c);
        }
        placed.push(Placement::new(c, location::MZONE, i as u32, position::FACEUP_ATTACK));
    }
    // Every card code of the duel once, sorted: the order of the limits' bitmaps.
    let mut order: Vec<u32> = deck.main.iter().chain(&deck.extra).chain(scene.grave).map(|&c| db.canonical_code(c)).collect();
    order.sort_unstable();
    order.dedup();

    let mut reading = Reading { code: probe, ..Reading::default() };
    let mut offers: BTreeMap<(u64, u64, u8), HashSet<u32>> = BTreeMap::new();
    let mut activated: HashSet<u64> = HashSet::new();
    let ignore: HashSet<u32> = blanks.iter().copied().chain([probe]).collect();
    let mut walk = 0;
    loop {
        let made = Instant::now();
        let made_duel = if placed.is_empty() && scene.grave.is_empty() {
            Duel::goldfish(engine, &deck, &hand, seed)
        } else {
            Duel::goldfish_reading(engine, &deck, &hand, seed, &placed, scene.grave)
        };
        let mut duel = match made_duel {
            Ok(duel) => duel,
            Err(e) => {
                reading.error = Some(e.to_string());
                break;
            }
        };
        watch_limits(&duel);
        if let Some(&holder) = blanks.first() {
            add_helpers(&duel, probe, holder, scene.events);
        }
        reading.setup += made.elapsed();
        let mut walker = Walker {
            engine,
            probe,
            prefer,
            ignore: &ignore,
            walk,
            first: None,
            own: 0,
            own_budget: budget.walks,
            holder: blanks.first().copied(),
            used: HashSet::new(),
            last: 0,
            offers: &mut offers,
            activated: &mut activated,
            summons: 0,
            limits: &mut reading.limits,
            order: &order,
        };
        let played = walker.play(&mut duel, budget.decisions);
        let (first, own) = (walker.first, walker.own);
        reading.summons += walker.summons;
        reading.walks += 1;
        match played {
            Ok(decisions) => reading.decisions += decisions,
            Err(e) => {
                reading.error = Some(e);
                break;
            }
        }
        // The card's own actions (at most the budget), then the helpers.
        reading.actions = own;
        walk += 1;
        if walk >= own.min(budget.walks) + first.unwrap_or(0).saturating_sub(own) {
            break;
        }
    }
    reading.offers = offers
        .into_iter()
        .filter(|(_, cards)| !cards.is_empty())
        .map(|((effect, hint, location), cards)| {
            let mut cards: Vec<u32> = cards.into_iter().collect();
            cards.sort_unstable();
            Offer { effect, hint, location, cards }
        })
        .collect();
    reading.activated = activated.into_iter().collect();
    reading.activated.sort_unstable();
    reading.limits.sort_by(|a, b| (a.owner, a.code, &a.forbidden).cmp(&(b.owner, b.code, &b.forbidden)));
    reading.limits.dedup();
    reading.time = started.elapsed();
    reading
}

/// Where a card read from the field is: a monster face-up in the middle
/// Monster Zone, a Field Spell in the Field Zone, a Continuous or Equip
/// Spell or Trap face-up, any other Spell or Trap Set.
fn field_placement(card: &ygo_engine::Card) -> Placement {
    if card.type_ & card_type::MONSTER != 0 {
        return Placement::new(card.code, location::MZONE, 2, position::FACEUP_ATTACK);
    }
    if card.type_ & card_type::FIELD != 0 {
        return Placement::new(card.code, location::SZONE, 5, position::FACEUP_ATTACK);
    }
    let stays = card.type_ & (card_type::CONTINUOUS | card_type::EQUIP) != 0;
    Placement::new(
        card.code,
        location::SZONE,
        2,
        if stays { position::FACEUP_ATTACK } else { position::FACEDOWN_ATTACK | position::FACEDOWN_DEFENSE },
    )
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
    /// The card's actions and the helpers in the first open moment, once
    /// it is met; and of them the card's own.
    first: Option<usize>,
    own: usize,
    /// Walks of the card's own actions, at most.
    own_budget: usize,
    /// The card of the hand the helpers belong to.
    holder: Option<u32>,
    /// The card's summons and effects used in this turn (kind, description).
    used: HashSet<(ActionKind, u64)>,
    /// The description of the card's summon or effect used last.
    last: u64,
    offers: &'a mut BTreeMap<(u64, u64, u8), HashSet<u32>>,
    activated: &'a mut HashSet<u64>,
    summons: usize,
    limits: &'a mut Vec<PlayerLimit>,
    order: &'a [u32],
}

impl Walker<'_> {
    fn play(&mut self, duel: &mut Duel, max: usize) -> Result<usize, String> {
        for decisions in 0..max {
            let step = duel.advance().map_err(|e| e.to_string())?;
            let (Step::End { events } | Step::Decision { events, .. }) = &step;
            self.note(events);
            self.limits.extend(take_limits(duel, self.order));
            let decision = match step {
                Step::End { .. } => return Ok(decisions),
                Step::Decision { decision, .. } => decision,
            };
            let Some(answer) = self.answer(duel, &decision) else { return Ok(decisions) };
            duel.respond(&answer);
        }
        Ok(max)
    }

    /// The card's activations and summons among `events`.
    fn note(&mut self, events: &[Event]) {
        for event in events {
            match event {
                Event::Chaining { card, description, .. } if card.controller == 0 && self.is_probe(card) => {
                    self.activated.insert(*description);
                }
                Event::NormalSummoning { card } | Event::SpecialSummoning { card } if card.controller == 0 && self.is_probe(card) => {
                    self.summons += 1;
                }
                _ => {}
            }
        }
    }

    fn is_probe(&self, card: &CardRef) -> bool {
        self.engine.cards().canonical_code(card.code) == self.probe
    }

    /// `None` ends the turn here: the walk is done.
    fn answer(&mut self, duel: &Duel, decision: &Decision) -> Option<Vec<u8>> {
        if decision.player() != Some(0) {
            return Some(ygo_engine::auto::passive_answer(decision, &duel.actions(decision)));
        }
        self.record(duel, decision);
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
                        let helper = |a: &&ygo_engine::Action| {
                            a.kind == ActionKind::Activate
                                && a.description >> 16 == HELPER >> 16
                                && a.card.is_some_and(|c| Some(self.engine.cards().canonical_code(c.code)) == self.holder)
                        };
                        let helpers: Vec<&ygo_engine::Action> = actions.iter().filter(helper).collect();
                        self.own = mine.len();
                        self.first = Some(mine.len() + helpers.len());
                        // Past the budget of its own actions, the helpers.
                        let own_walks = mine.len().min(self.own_budget);
                        if self.walk < own_walks {
                            mine.get(self.walk).copied()
                        } else {
                            helpers.get(self.walk - own_walks).copied()
                        }
                    }
                    Some(_) => mine.iter().find(|a| !self.used.contains(&(a.kind, a.description))).copied(),
                };
                match pick {
                    Some(a) => {
                        self.used.insert((a.kind, a.description));
                        self.last = a.description;
                        Some(a.response.clone())
                    }
                    None => None,
                }
            }
            Decision::SelectChain { .. } => {
                let actions = duel.actions(decision);
                let own = actions.iter().find(|a| a.kind == ActionKind::Activate && a.card.is_some_and(|c| self.is_probe(&c)));
                Some(match own {
                    Some(a) => {
                        self.last = a.description;
                        a.response.clone()
                    }
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

    /// The cards of yours a choice offers, off the field, under the effect
    /// of the card asking.
    fn record(&mut self, duel: &Duel, decision: &Decision) {
        let (cards, hint): (Vec<&CardRef>, u64) = match decision {
            Decision::SelectCard { cards, hint, .. } => (cards.iter().collect(), *hint),
            Decision::SelectUnselectCard { selectable, unselectable, hint, .. } => (selectable.iter().chain(unselectable).collect(), *hint),
            Decision::SelectTribute { cards, hint, .. } => (cards.iter().map(|(c, _)| c).collect(), *hint),
            Decision::SelectSum { must, cards, hint, .. } => (must.iter().chain(cards).map(|(c, _)| c).collect(), *hint),
            _ => return,
        };
        let db = self.engine.cards();
        let effect = match duel.chain().last() {
            Some(link) if link.controller == 0 && link.code == self.probe => link.description,
            _ => self.last,
        };
        for c in cards {
            let code = db.canonical_code(c.code);
            if c.controller != 0 || c.location & (location::MZONE | location::SZONE | location::OVERLAY) != 0 || self.ignore.contains(&code)
            {
                continue;
            }
            self.offers.entry((effect, hint, c.location)).or_default().insert(code);
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
/// each hint and location of the deck's reading, the pool's offers
/// restricted to the deck's cards must be the same.
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
        // Over the card's effects: which one asked may differ between the two.
        let find = |r: &Reading| -> HashSet<u32> {
            r.offers.iter().filter(|o| key(o) == k).flat_map(|o| o.cards.iter().copied().filter(|c| deck_cards.contains(c))).collect()
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
        let offers = offers.iter().map(|&(hint, location, cards)| Offer { effect: 0, hint, location, cards: cards.to_vec() }).collect();
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
