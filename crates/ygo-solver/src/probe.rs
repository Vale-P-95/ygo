//! The probe turn (Phase 5C): a board is put to the test by an opponent
//! with a kit of plain cards, and the engine measures what the board does
//! about it. The turn of the board is played again with the same answers
//! in a duel where the opponent has the kit; then, on the opponent's turn:
//!
//! - **probes**: the opponent activates a Spell, searches, uses a monster
//!   effect from the hand, sends a card to the GY, Special Summons from the
//!   hand, from the GY and from the Extra Deck, Normal Summons. Each one has
//!   goals (a card added, a monster on the field) that tell if it worked;
//! - **responders**: every effect of yours the engine offers in response is
//!   tried in a run of its own, used whenever it is offered; the probes that
//!   fail in that run (and worked without it) are what it stops;
//! - **floodgates**: probes that work on an empty board but not on this one
//!   without any response;
//! - **board breakers**: real cards (Dark Hole, Raigeki, Kaiju...) each in a
//!   run of its own: what is left of the responders after them, or whether
//!   a responder stops them.
//!
//! Everything is measured in "interruptions": 1.0 is negating the
//! activation of any card and destroying it, once, without targeting.

use std::collections::{HashMap, HashSet};

use ygo_engine::auto::passive_answer;
use ygo_engine::consts::{location, reason};
use ygo_engine::{Action, ActionKind, CardRef, Decision, DeckList, Duel, DuelState, Engine, Error, Event, Opponent, Step};

use crate::search::Line;

/// What the opponent tries; each probe has a weight (how much the action
/// matters in a typical deck).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Probe {
    Spell,
    Search,
    HandEffect,
    SendToGrave,
    SpecialFromHand,
    FieldEffect,
    GraveEffect,
    NormalSummon,
    ExtraSummon,
}

impl Probe {
    pub const ALL: [Probe; 9] = [
        Probe::Spell,
        Probe::Search,
        Probe::HandEffect,
        Probe::SendToGrave,
        Probe::SpecialFromHand,
        Probe::FieldEffect,
        Probe::GraveEffect,
        Probe::NormalSummon,
        Probe::ExtraSummon,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Probe::Spell => "attivare una Magia",
            Probe::Search => "aggiungere una carta dal Deck alla mano",
            Probe::HandEffect => "attivare un effetto di mostro dalla mano",
            Probe::SendToGrave => "mandare un mostro dal Deck al Cimitero",
            Probe::SpecialFromHand => "Evocare Specialmente dalla mano",
            Probe::FieldEffect => "attivare l'effetto di un mostro sul campo",
            Probe::GraveEffect => "attivare un effetto dal Cimitero",
            Probe::NormalSummon => "Evocare Normalmente",
            Probe::ExtraSummon => "Evocare dall'Extra Deck",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Probe::Spell => 0.4,
            Probe::Search => 0.5,
            Probe::HandEffect | Probe::SendToGrave | Probe::SpecialFromHand | Probe::FieldEffect | Probe::GraveEffect => 0.3,
            Probe::NormalSummon => 0.2,
            Probe::ExtraSummon => 0.6,
        }
    }

    /// An activation (the reference of coverage: an effect that can stop
    /// every activation covers 100%).
    fn is_activation(self) -> bool {
        !matches!(self, Probe::SpecialFromHand | Probe::NormalSummon | Probe::ExtraSummon)
    }
}

/// Sum of the weights of the activation probes.
fn activation_weight() -> f64 {
    Probe::ALL.iter().filter(|p| p.is_activation()).map(|p| p.weight()).sum()
}

/// What a response did to the opponent's card, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    NegateAndRemove,
    NegateSummon,
    NegateActivation,
    Banish,
    ToDeck,
    NegateEffect,
    Bounce,
    Destroy,
    ToGrave,
    /// Nothing happened to the card, but later actions failed (as with Droll & Lock Bird).
    Lock,
    Position,
}

impl Effect {
    pub fn value(self) -> f64 {
        match self {
            Effect::NegateAndRemove => 1.0,
            Effect::NegateSummon => 0.95,
            Effect::NegateActivation => 0.9,
            Effect::Banish | Effect::ToDeck => 0.85,
            Effect::NegateEffect => 0.8,
            Effect::Bounce | Effect::Destroy | Effect::ToGrave | Effect::Lock => 0.7,
            Effect::Position => 0.4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Effect::NegateAndRemove => "nega l'attivazione e toglie la carta",
            Effect::NegateSummon => "nega l'Evocazione",
            Effect::NegateActivation => "nega l'attivazione",
            Effect::Banish => "bandisce",
            Effect::ToDeck => "rimanda nel Deck",
            Effect::NegateEffect => "nega l'effetto",
            Effect::Bounce => "rimanda in mano",
            Effect::Destroy => "distrugge",
            Effect::ToGrave => "manda al Cimitero",
            Effect::Lock => "blocca le azioni dopo",
            Effect::Position => "cambia posizione o controllo",
        }
    }

    /// Stops a card being activated (what can answer a board breaker).
    fn negates(self) -> bool {
        matches!(self, Effect::NegateAndRemove | Effect::NegateActivation | Effect::NegateEffect)
    }
}

/// One of your effects: the card (original code) and which of its effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResponderId {
    pub code: u32,
    pub description: u64,
}

/// What the opponent does in one step, and how to tell it worked.
#[derive(Debug, Clone)]
struct KitStep {
    act: Act,
    /// Cards to pick when the step asks to choose (search targets, materials).
    picks: Vec<u32>,
    /// Pick as many cards as allowed (costs of Forbidden Droplet).
    take_max: bool,
    /// The option to choose (Lightning Storm's two effects).
    option: usize,
    /// Its own triggers to activate (Speedroid Terrortop's search).
    triggers: Vec<u32>,
    /// Then attack with every monster that can, at the `picks` first.
    attack: bool,
    goals: Vec<(Probe, Want)>,
    /// A probe that must have worked for this step to make sense.
    needs: Option<Probe>,
}

#[derive(Debug, Clone, Copy)]
enum Act {
    Activate(u32, u8),
    NormalSummon(u32),
    SpecialSummon(u32),
}

/// The opponent has `delta` more copies of `code` (any card if `None`) in
/// `location` after the step.
#[derive(Debug, Clone, Copy)]
struct Want {
    code: Option<u32>,
    location: u8,
    delta: i32,
}

/// A board breaker of the kit.
#[derive(Debug, Clone)]
pub struct Breaker {
    pub name: String,
    /// How often it matters: the average over breakers is weighted by it.
    pub weight: f64,
    steps: Vec<KitStep>,
}

/// The opponent's cards and what it does with them.
#[derive(Debug, Clone)]
pub struct Kit {
    opponent: Opponent,
    steps: Vec<KitStep>,
    pub breakers: Vec<Breaker>,
}

impl Kit {
    /// The standard kit: plain old cards that do one thing each, and the
    /// board breakers most played. Cards missing from the data are left out
    /// with what they test.
    pub fn standard(engine: &Engine) -> Kit {
        let code = |name: &str| engine.cards().find_by_name(name).map(|c| c.code);
        let pot = code("Pot of Greed");
        let reinforce = code("Reinforcement of the Army");
        let celtic = code("Celtic Guardian");
        let thunder = code("Thunder Dragon");
        let foolish = code("Foolish Burial");
        let bulb = code("Glow-Up Bulb");
        let terrortop = code("Speedroid Terrortop");
        let taketomborg = code("Speedroid Taketomborg");
        let spider = code("Link Spider");
        let filler = code("Alexandrite Dragon");

        let step = |act: Act, picks: Vec<u32>, goals: Vec<(Probe, Want)>, needs: Option<Probe>| KitStep {
            act,
            picks,
            take_max: false,
            option: 0,
            triggers: Vec::new(),
            attack: false,
            goals,
            needs,
        };
        let want = |code: Option<u32>, location: u8, delta: i32| Want { code, location, delta };
        let mut steps = Vec::new();
        let mut hand = Vec::new();
        let mut deck = Vec::new();
        let mut extra = Vec::new();
        if let Some(pot) = pot {
            hand.push(pot);
            steps.push(step(Act::Activate(pot, location::HAND), vec![], vec![(Probe::Spell, want(None, location::HAND, 1))], None));
        }
        if let (Some(reinforce), Some(celtic)) = (reinforce, celtic) {
            hand.push(reinforce);
            deck.push(celtic);
            steps.push(step(
                Act::Activate(reinforce, location::HAND),
                vec![celtic],
                vec![(Probe::Search, want(Some(celtic), location::HAND, 1))],
                None,
            ));
        }
        if let Some(thunder) = thunder {
            hand.push(thunder);
            deck.extend([thunder, thunder]);
            // Discards itself and adds up to two: one more in the hand at least.
            steps.push(step(
                Act::Activate(thunder, location::HAND),
                vec![thunder],
                vec![(Probe::HandEffect, want(Some(thunder), location::HAND, 0))],
                None,
            ));
        }
        if let (Some(foolish), Some(bulb)) = (foolish, bulb) {
            hand.push(foolish);
            deck.push(bulb);
            steps.push(step(
                Act::Activate(foolish, location::HAND),
                vec![bulb],
                vec![(Probe::SendToGrave, want(Some(bulb), location::GRAVE, 1))],
                None,
            ));
        }
        if let (Some(terrortop), Some(taketomborg)) = (terrortop, taketomborg) {
            hand.push(terrortop);
            deck.push(taketomborg);
            let mut s = step(
                Act::SpecialSummon(terrortop),
                vec![taketomborg],
                vec![
                    (Probe::SpecialFromHand, want(Some(terrortop), location::MZONE, 1)),
                    (Probe::FieldEffect, want(Some(taketomborg), location::HAND, 1)),
                ],
                None,
            );
            s.triggers = vec![terrortop];
            steps.push(s);
        }
        if let Some(bulb) = bulb {
            steps.push(step(
                Act::Activate(bulb, location::GRAVE),
                vec![],
                vec![(Probe::GraveEffect, want(Some(bulb), location::MZONE, 1))],
                Some(Probe::SendToGrave),
            ));
        }
        if let Some(celtic) = celtic {
            hand.push(celtic);
            steps.push(step(Act::NormalSummon(celtic), vec![], vec![(Probe::NormalSummon, want(Some(celtic), location::MZONE, 1))], None));
            if let Some(spider) = spider {
                extra.push(spider);
                steps.push(step(
                    Act::SpecialSummon(spider),
                    vec![celtic],
                    vec![(Probe::ExtraSummon, want(Some(spider), location::MZONE, 1))],
                    Some(Probe::NormalSummon),
                ));
            }
        }

        let mut breakers = Vec::new();
        let mut breaker = |name: &str, label: &str, weight: f64, summon: bool, option: usize, take_max: bool| {
            let Some(c) = code(name) else { return };
            if !hand.contains(&c) {
                hand.push(c);
            }
            let act = if summon { Act::SpecialSummon(c) } else { Act::Activate(c, location::HAND) };
            let mut s = step(act, vec![], vec![], None);
            s.option = option;
            s.take_max = take_max;
            breakers.push(Breaker { name: label.to_owned(), weight, steps: vec![s] });
        };
        breaker("Dark Hole", "Dark Hole", 1.0, false, 0, false);
        breaker("Raigeki", "Raigeki", 1.0, false, 0, false);
        breaker("Harpie's Feather Duster", "Harpie's Feather Duster", 1.0, false, 0, false);
        breaker("Lightning Storm", "Lightning Storm (mostri in Attacco)", 0.7, false, 0, false);
        breaker("Lightning Storm", "Lightning Storm (Magie e Trappole)", 0.5, false, 1, false);
        breaker("Forbidden Droplet", "Forbidden Droplet", 1.0, false, 0, true);
        breaker("Dark Ruler No More", "Dark Ruler No More", 1.0, false, 0, false);
        breaker("Book of Eclipse", "Book of Eclipse", 0.6, false, 0, false);
        breaker("Gameciel, the Sea Turtle Kaiju", "Kaiju (Gameciel)", 0.8, true, 0, false);
        // An attack: Cyber Dragon (it comes from the hand while only you
        // control monsters) with Limiter Removal, 4200 ATK, at the monster
        // worth the most it may attack. Battle protections count here (Invoked
        // Oceanus in a Main Monster Zone draws the attack).
        if let (Some(cyber), Some(limiter)) = (code("Cyber Dragon"), code("Limiter Removal")) {
            hand.extend([cyber, limiter]);
            let mut attack = step(Act::Activate(limiter, location::HAND), vec![], vec![], None);
            attack.attack = true;
            breakers.push(Breaker {
                name: "Attacco da 4200 ATK".to_owned(),
                weight: 0.6,
                steps: vec![step(Act::SpecialSummon(cyber), vec![], vec![], None), attack],
            });
        }

        // The last cards of the Deck are drawn first (at the start of the
        // turn and by Pot of Greed): plain ones, so the cards to search stay.
        deck.extend([filler; 3].into_iter().flatten());
        Kit { opponent: Opponent { hand, deck, extra }, steps, breakers }
    }

    /// The opponent's cards: give them to the search too
    /// ([`crate::SearchConfig::opponent`]), so the lines it finds play the
    /// same in the probe turn.
    pub fn opponent(&self) -> &Opponent {
        &self.opponent
    }

    /// The probes this kit can measure.
    pub fn probes(&self) -> Vec<Probe> {
        let mut out: Vec<Probe> = self.steps.iter().flat_map(|s| s.goals.iter().map(|(p, _)| *p)).collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Where your turn starts from in a run: the line of a board, or ending
/// the turn at once (the empty board the floodgates are compared with).
#[derive(Clone, Copy)]
enum Prelude<'a> {
    Line(&'a Line),
    EndAtOnce,
}

/// A moment to respond in a run: `(step, window in the step)`.
type WindowAt = (usize, usize);

/// How you answer the opponent's actions in a run.
#[derive(Clone, Copy)]
enum Policy<'a> {
    Pass,
    /// Use this effect whenever it is offered, except at the windows listed
    /// (`(step, window in the step)`).
    Use(ResponderId, &'a HashSet<WindowAt>),
    /// Use any of these whenever offered, the first listed first.
    UseAll(&'a [ResponderId]),
    /// Use this effect only at one window: `(step, window in the step)`.
    UseAt(ResponderId, WindowAt),
}

/// A moment you could respond, and with what.
#[derive(Debug, Clone)]
struct Window {
    step: usize,
    options: Vec<(ResponderId, u8)>,
}

/// One use of the responder of a run.
#[derive(Debug, Clone)]
struct Use {
    responder: ResponderId,
    step: usize,
    window: usize,
    effect: Option<Effect>,
    targeted: bool,
}

#[derive(Debug, Clone, Default)]
struct StepRun {
    available: bool,
    /// For each goal of the step.
    worked: Vec<bool>,
    /// The step's own card had its activation negated.
    negated: bool,
}

#[derive(Debug, Clone)]
struct RunTrace {
    steps: Vec<StepRun>,
    windows: Vec<Window>,
    uses: Vec<Use>,
    state: DuelState,
}

/// Follows the events of a run to tell what each use of the responder did.
#[derive(Default)]
struct Observer {
    /// Chain link -> controller and code.
    links: HashMap<u32, (u8, u32)>,
    /// A use waiting for its chain link.
    waiting: Option<usize>,
    use_of_link: HashMap<u32, usize>,
    targeting: Option<usize>,
    solving: Option<usize>,
    /// Uses already resolved in the current chain: a card whose effects
    /// they negated (Effect Veiler, Infinite Impermanence) only shows it
    /// when its own link resolves later.
    resolved: Vec<usize>,
    /// The use resolving negated this card of the opponent.
    negated_code: Option<u32>,
    /// The step's opponent chain links (to tell if its card was negated).
    step_links: Vec<u32>,
    step_negated: bool,
    /// The opponent's monster being summoned in this step, before any chain.
    summoning: Option<u32>,
}

impl Observer {
    fn see(&mut self, engine: &Engine, event: &Event, uses: &mut [Use]) {
        let db = engine.cards();
        // Keeps the strongest effect seen for a use.
        fn better(uses: &mut [Use], u: usize, e: Effect) {
            let slot = &mut uses[u].effect;
            if slot.is_none_or(|old| e < old) {
                *slot = Some(e);
            }
        }
        match event {
            Event::Chaining { card, chain_link, .. } => {
                self.links.insert(*chain_link, (card.controller, card.code));
                if card.controller == 1 {
                    self.step_links.push(*chain_link);
                    self.summoning = None;
                }
                if let Some(u) = self.waiting {
                    if card.controller == 0 {
                        self.use_of_link.insert(*chain_link, u);
                        self.waiting = None;
                        self.targeting = Some(u);
                    }
                }
            }
            Event::BecomeTarget { cards } => {
                if let Some(u) = self.targeting {
                    if cards.iter().any(|c| c.controller == 1) {
                        uses[u].targeted = true;
                    }
                }
            }
            Event::ChainSolving { chain_link } => {
                self.targeting = None;
                self.solving = self.use_of_link.get(&(*chain_link as u32)).copied();
                self.negated_code = None;
            }
            Event::ChainSolved { .. } => {
                if let Some(u) = self.solving.take() {
                    self.resolved.push(u);
                }
            }
            Event::ChainEnd => {
                self.links.clear();
                self.use_of_link.clear();
                self.solving = None;
                self.resolved.clear();
            }
            Event::ChainNegated { chain_link } | Event::ChainDisabled { chain_link } => {
                let negated = matches!(event, Event::ChainNegated { .. });
                let link = *chain_link as u32;
                if self.step_links.contains(&link) {
                    self.step_negated = true;
                }
                if let (Some(u), Some(&(1, code))) = (self.solving, self.links.get(&link)) {
                    better(uses, u, if negated { Effect::NegateActivation } else { Effect::NegateEffect });
                    self.negated_code = Some(db.canonical_code(code));
                } else if let (None, Some(&u), Some((1, _))) = (self.solving, self.resolved.last(), self.links.get(&link)) {
                    // Negated by one of your effects resolved before it in this chain.
                    better(uses, u, Effect::NegateEffect);
                }
            }
            Event::Move { code, from, to, reason: why } => {
                let Some(u) = self.solving else { return };
                if from.controller != 1 || from.location & (location::MZONE | location::SZONE | location::HAND) == 0 {
                    return;
                }
                let code = db.canonical_code(*code);
                if self.negated_code == Some(code) {
                    better(uses, u, Effect::NegateAndRemove);
                    return;
                }
                if self.summoning == Some(code) && from.location == location::MZONE {
                    better(uses, u, Effect::NegateSummon);
                    return;
                }
                let effect = match to.location & !location::OVERLAY {
                    _ if to.controller == 0 && to.location & (location::MZONE | location::SZONE) != 0 => Effect::Position,
                    location::REMOVED => Effect::Banish,
                    location::DECK | location::EXTRA => Effect::ToDeck,
                    location::HAND => Effect::Bounce,
                    location::GRAVE if why & reason::DESTROY != 0 => Effect::Destroy,
                    location::GRAVE => Effect::ToGrave,
                    _ => return,
                };
                better(uses, u, effect);
            }
            Event::PositionChange { card, .. } => {
                if let Some(u) = self.solving {
                    if card.controller == 1 {
                        better(uses, u, Effect::Position);
                    }
                }
            }
            Event::NormalSummoning { card } | Event::SpecialSummoning { card } if card.controller == 1 && self.step_links.is_empty() => {
                self.summoning = Some(db.canonical_code(card.code));
            }
            _ => {}
        }
    }

    fn new_step(&mut self) {
        self.step_links.clear();
        self.step_negated = false;
        self.summoning = None;
    }
}

/// Puts boards to the test for one deck and hand.
pub struct Prober<'a> {
    engine: &'a Engine,
    deck: &'a DeckList,
    hand: &'a [u32],
    seed: u64,
    kit: &'a Kit,
    /// The empty board: what each probe does with no answer at all.
    baseline: Option<RunTrace>,
}

/// One of your effects that stops what the opponent does.
#[derive(Debug, Clone)]
pub struct Interruption {
    pub responder: ResponderId,
    /// Where the card is: hand, field, GY, banished.
    pub location: u8,
    pub effect: Effect,
    pub targeted: bool,
    /// Times it stopped something in a row.
    pub uses: u32,
    /// Probes it can answer (offered) or stopped.
    pub covers: Vec<Probe>,
    pub coverage: f64,
    /// Counted with your other effects: less than `alone` when they share
    /// what it needs (Invoked Mechaba sends a card from your hand, and that
    /// card may be the handtrap counted on its own).
    pub value: f64,
    /// Measured with no other effect used.
    pub alone: f64,
}

#[derive(Debug, Clone)]
pub struct BreakerResult {
    pub name: String,
    pub weight: f64,
    /// The breaker could not be used against this board.
    pub not_applicable: bool,
    /// One of your effects negated it.
    pub stopped_by: Option<ResponderId>,
    /// Interruption value left after it.
    pub remaining: f64,
}

/// What the probe turn measured on a board.
#[derive(Debug, Clone, Default)]
pub struct Measure {
    pub interruptions: Vec<Interruption>,
    /// Probes that work on an empty board and not on this one.
    pub floodgates: Vec<Probe>,
    pub breakers: Vec<BreakerResult>,
    /// Interruption value from the field, GY and banishment.
    pub field_value: f64,
    /// Interruption value from the hand.
    pub hand_value: f64,
    pub floodgate_value: f64,
    /// Interruption value left on average after a board breaker.
    pub resilience: f64,
}

impl<'a> Prober<'a> {
    pub fn new(engine: &'a Engine, deck: &'a DeckList, hand: &'a [u32], seed: u64, kit: &'a Kit) -> Self {
        Prober { engine, deck, hand, seed, kit, baseline: None }
    }

    /// Measures the board reached by `line`.
    pub fn measure(&mut self, line: &Line) -> Result<Measure, Error> {
        let all_steps: Vec<&KitStep> = self.kit.steps.iter().collect();
        if self.baseline.is_none() {
            self.baseline = Some(self.run(Prelude::EndAtOnce, &all_steps, Policy::Pass)?);
        }
        let base = self.baseline.clone().expect("baseline");
        let all = self.run(Prelude::Line(line), &all_steps, Policy::Pass)?;
        if std::env::var_os("DUELFORGE_DEBUG_PROBE").is_some() {
            let db = self.engine.cards();
            for (label, t) in [("empty board", &base), ("this board", &all)] {
                eprintln!("-- {label}");
                for (i, (step, run)) in self.kit.steps.iter().zip(&t.steps).enumerate() {
                    let offered: Vec<String> =
                        t.windows.iter().filter(|w| w.step == i).flat_map(|w| w.options.iter().map(|(o, _)| db.name(o.code))).collect();
                    eprintln!(
                        "   {:?}: available {} worked {:?} offered {:?}",
                        step.goals.iter().map(|g| g.0).collect::<Vec<_>>(),
                        run.available,
                        run.worked,
                        offered
                    );
                }
            }
        }
        let worked = |trace: &RunTrace| -> HashMap<Probe, bool> {
            let mut out = HashMap::new();
            for (step, run) in self.kit.steps.iter().zip(&trace.steps) {
                for ((probe, _), ok) in step.goals.iter().zip(&run.worked) {
                    out.insert(*probe, run.available && *ok);
                }
            }
            out
        };
        let all_worked = worked(&all);

        // Floodgates: what works on an empty board but not here, with no answer.
        let mut floodgates = Vec::new();
        for (step, (run, base_run)) in self.kit.steps.iter().zip(all.steps.iter().zip(&base.steps)) {
            let prerequisite = step.needs.is_none_or(|p| all_worked.get(&p) == Some(&true) || floodgates.contains(&p));
            if !prerequisite {
                continue;
            }
            for (i, (probe, _)) in step.goals.iter().enumerate() {
                let possible = base_run.available && base_run.worked.get(i) == Some(&true);
                let here = run.available && run.worked.get(i) == Some(&true);
                if possible && !here {
                    floodgates.push(*probe);
                }
            }
        }
        // Coverage is of what the opponent can still do: what a floodgate
        // already stops is counted there, not against your other effects.
        let reference = activation_weight() - floodgates.iter().filter(|p| p.is_activation()).map(|p| p.weight()).sum::<f64>();

        // Responders: every effect offered while nothing is answered, and
        // the windows (step, window in the step) where it is offered.
        let mut offered: Vec<(ResponderId, u8, Vec<WindowAt>)> = Vec::new();
        let mut in_step: HashMap<usize, usize> = HashMap::new();
        for w in &all.windows {
            let at = in_step.entry(w.step).or_default();
            for &(id, loc) in &w.options {
                match offered.iter_mut().find(|(o, _, _)| *o == id) {
                    Some((_, _, windows)) => windows.push((w.step, *at)),
                    None => offered.push((id, loc, vec![(w.step, *at)])),
                }
            }
            *at += 1;
        }
        let debug = std::env::var_os("DUELFORGE_DEBUG_PROBE").is_some();
        let stopped_by = |run: &RunTrace| -> Vec<Probe> {
            let now = worked(run);
            all_worked.iter().filter(|(p, ok)| **ok && now.get(p) != Some(&true)).map(|(p, _)| *p).collect()
        };
        let mut interruptions = Vec::new();
        for (id, loc, windows) in &offered {
            // What it stops, tried at each window where it is offered (being
            // offered is not being able to stop that action: Effect Veiler is
            // offered while a monster is summoned, but cannot stop the summon).
            let mut covers: Vec<Probe> = Vec::new();
            let mut effect: Option<Effect> = None;
            let mut targeted = false;
            for &window in windows.iter().take(12) {
                let Ok(run) = self.run(Prelude::Line(line), &all_steps, Policy::UseAt(*id, window)) else { continue };
                let stopped = stopped_by(&run);
                if debug {
                    eprintln!("-- {} at {window:?}: stops {stopped:?}", self.engine.cards().name(id.code));
                }
                if stopped.is_empty() {
                    continue;
                }
                covers.extend(stopped);
                for u in &run.uses {
                    targeted |= u.targeted;
                    effect = match (effect, u.effect) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                }
            }
            if covers.is_empty() {
                continue;
            }
            covers.sort();
            covers.dedup();
            // How many times in a row: used whenever offered, keeping it for
            // later where it does nothing.
            let mut uses = 1;
            let mut skip = HashSet::new();
            for _ in 0..3 {
                let Ok(run) = self.run(Prelude::Line(line), &all_steps, Policy::Use(*id, &skip)) else { break };
                uses = uses.max(run.uses.iter().filter(|u| u.effect.is_some()).count() as u32);
                let wasted: Vec<(usize, usize)> = run.uses.iter().filter(|u| u.effect.is_none()).map(|u| (u.step, u.window)).collect();
                if wasted.is_empty() || wasted.iter().all(|w| skip.contains(w)) {
                    break;
                }
                skip.extend(wasted);
            }
            let effect = effect.unwrap_or(Effect::Lock);
            let coverage = if reference > 0.0 { (covers.iter().map(|p| p.weight()).sum::<f64>() / reference).min(1.0) } else { 0.0 };
            let counter = self.engine.cards().get(id.code).is_some_and(|c| c.type_ & 0x10_0000 != 0);
            let value = effect.value() * if targeted { 0.85 } else { 1.0 } * coverage * uses as f64 * if counter { 1.05 } else { 1.0 };
            interruptions.push(Interruption {
                responder: *id,
                location: *loc,
                effect,
                targeted,
                uses,
                covers,
                coverage,
                value,
                alone: value,
            });
        }
        interruptions.sort_by(|a, b| b.value.total_cmp(&a.value));

        // All together, the strongest first: an effect the engine no longer
        // offers once the others are used (its cost is gone) counts less.
        if interruptions.len() > 1 {
            let order: Vec<ResponderId> = interruptions.iter().map(|i| i.responder).collect();
            if let Ok(run) = self.run(Prelude::Line(line), &all_steps, Policy::UseAll(&order)) {
                for i in &mut interruptions {
                    let used = run.uses.iter().filter(|u| u.responder == i.responder).count() as u32;
                    if used < i.uses {
                        i.value = i.alone * used as f64 / i.uses as f64;
                    }
                }
                interruptions.sort_by(|a, b| b.value.total_cmp(&a.value));
            }
        }

        // Board breakers, each in a run of its own.
        let total = interruptions.iter().map(|i| i.value).sum::<f64>() + 0.0;
        let mut breakers = Vec::new();
        // The breakers go for the monster worth the most first (the attack,
        // the monster a Kaiju takes).
        let targets: Vec<u32> = interruptions.iter().filter(|i| i.location == location::MZONE).map(|i| i.responder.code).collect();
        for breaker in &self.kit.breakers {
            let mut owned = breaker.steps.clone();
            for s in &mut owned {
                s.picks = targets.clone();
            }
            let steps: Vec<&KitStep> = owned.iter().collect();
            let run = match self.run(Prelude::Line(line), &steps, Policy::Pass) {
                Ok(run) if run.steps.iter().all(|s| s.available) => run,
                _ => {
                    breakers.push(BreakerResult {
                        name: breaker.name.clone(),
                        weight: breaker.weight,
                        not_applicable: true,
                        stopped_by: None,
                        remaining: total,
                    });
                    continue;
                }
            };
            let mut result = BreakerResult {
                name: breaker.name.clone(),
                weight: breaker.weight,
                not_applicable: false,
                stopped_by: None,
                remaining: self.left(&run.state, &interruptions),
            };
            // A negation offered against it: try it (it spends one use).
            let negators: Vec<&Interruption> = interruptions
                .iter()
                .filter(|i| i.effect.negates() && run.windows.iter().any(|w| w.options.iter().any(|(o, _)| *o == i.responder)))
                .collect();
            for negator in negators {
                let none = HashSet::new();
                let Ok(tried) = self.run(Prelude::Line(line), &steps, Policy::Use(negator.responder, &none)) else { continue };
                if tried.steps.iter().any(|s| s.negated) {
                    let left = self.left(&tried.state, &interruptions) - negator.value / negator.uses as f64;
                    if left > result.remaining {
                        result.remaining = left;
                        result.stopped_by = Some(negator.responder);
                    }
                    break;
                }
            }
            breakers.push(result);
        }
        let weights: f64 = breakers.iter().map(|b| b.weight).sum();
        let resilience = if weights > 0.0 { breakers.iter().map(|b| b.weight * b.remaining).sum::<f64>() / weights } else { total };

        let hand_value = interruptions.iter().filter(|i| i.location == location::HAND).map(|i| i.value).sum::<f64>() + 0.0;
        Ok(Measure {
            field_value: total - hand_value,
            hand_value,
            floodgate_value: floodgates.iter().map(|p| p.weight()).sum::<f64>() + 0.0,
            interruptions,
            floodgates,
            breakers,
            resilience,
        })
    }

    /// Interruption value still usable in `state`: each responder's card is
    /// still where it was (face-up and not negated on the field).
    fn left(&self, state: &DuelState, interruptions: &[Interruption]) -> f64 {
        let db = self.engine.cards();
        let me = &state.players[0];
        let same = |c: u32, code: u32| db.canonical_code(c) == code;
        interruptions
            .iter()
            .filter(|i| {
                let code = i.responder.code;
                match i.location {
                    location::HAND => me.hand.iter().any(|&c| same(c, code)),
                    location::GRAVE => me.grave.iter().any(|&c| same(c, code)),
                    location::REMOVED => me.banished.iter().any(|&(c, _)| same(c, code)),
                    _ => me
                        .monsters
                        .iter()
                        .chain(&me.spells)
                        .flatten()
                        .any(|c| same(c.code, code) && (c.is_face_up() || i.location == location::SZONE) && !c.is_negated()),
                }
            })
            .map(|i| i.value)
            .sum::<f64>()
            + 0.0
    }

    /// Plays your turn, then the opponent's steps, answering as `policy` says.
    fn run(&self, prelude: Prelude, steps: &[&KitStep], policy: Policy) -> Result<RunTrace, Error> {
        let engine = self.engine;
        let mut duel = Duel::goldfish_vs(engine, self.deck, self.hand, self.seed, &self.kit.opponent)?;
        const NO_ANSWERS: &[Vec<u8>] = &[];
        let mut answers = match prelude {
            Prelude::Line(line) => line.answers.iter(),
            Prelude::EndAtOnce => NO_ANSWERS.iter(),
        };
        let mut trace =
            RunTrace { steps: vec![StepRun::default(); steps.len()], windows: Vec::new(), uses: Vec::new(), state: DuelState::default() };
        let mut observer = Observer::default();
        let mut next = 0usize;
        let mut current: Option<(usize, DuelState)> = None;
        let mut windows_in_step = 0usize;
        // The step being played went to the Battle Phase already.
        let mut battled = false;
        // The opponent got back to choose after its last step.
        let mut complete = false;
        let verbose = std::env::var("DUELFORGE_DEBUG_PROBE").is_ok_and(|v| v == "2");
        let mut shown = false;
        let label =
            |actions: &[Action], answer: &[u8]| actions.iter().find(|a| a.response == answer).map_or("?".to_owned(), |a| a.label.clone());
        for _ in 0..4000 {
            let (decision, events) = match duel.advance()? {
                Step::Decision { decision, events } => (decision, events),
                Step::End { .. } => break,
            };
            for e in &events {
                observer.see(engine, e, &mut trace.uses);
                if verbose {
                    if matches!(
                        e,
                        Event::Chaining { .. }
                            | Event::ChainSolving { .. }
                            | Event::ChainSolved { .. }
                            | Event::ChainNegated { .. }
                            | Event::ChainDisabled { .. }
                            | Event::BecomeTarget { .. }
                    ) {
                        eprintln!("       {e:?}");
                    }
                    if let Event::Move { code, from, to, reason: why } = e {
                        eprintln!(
                            "       move {} {:?}/{} -> {:?}/{} reason {why:#x}",
                            engine.cards().name(*code),
                            from.location,
                            from.controller,
                            to.location,
                            to.controller
                        );
                    }
                }
            }
            let (turn, _, _) = duel.turn_info();
            let actions = duel.actions(&decision);
            // Your turn: the line's answers; the opponent passes.
            if turn <= 1 {
                // The line has every answer of your turn, the opponent's passes
                // included. A provisional board's line stops at "end the turn":
                // its End Phase is played like the search would (doing something first).
                match prelude {
                    Prelude::Line(_) => match answers.next() {
                        Some(a) => duel.respond(a),
                        None if decision.player() == Some(1) => duel.respond(&passive_answer(&decision, &actions)),
                        None => duel.respond(&end_phase_answer(&decision, &actions)),
                    },
                    Prelude::EndAtOnce if decision.player() == Some(1) => duel.respond(&passive_answer(&decision, &actions)),
                    Prelude::EndAtOnce => duel.respond(&end_turn(&decision, &actions)),
                }
                continue;
            }
            if verbose && !shown {
                shown = true;
                let st = duel.state();
                let db = engine.cards();
                let names = |v: &[u32]| v.iter().map(|&c| db.name(c)).collect::<Vec<_>>().join(", ");
                let zones: Vec<String> = st.players[0]
                    .monsters
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.as_ref().map(|c| format!("{}@{i}", db.name(c.code))))
                    .collect();
                eprintln!("   turn 2, your monsters: {} · grave {:?}", zones.join(", "), st.players[1].grave);
                if let Ok(name) = std::env::var("DUELFORGE_DEBUG_LUA") {
                    duel.run_lua(&name);
                }
                for line in duel.log() {
                    eprintln!("   engine log: {line}");
                }
                eprintln!("   your hand: {} · opponent hand: {}", names(&st.players[0].hand), names(&st.players[1].hand));
            }
            if verbose {
                eprintln!(
                    "     p{:?} {:?}: {}",
                    decision.player(),
                    std::mem::discriminant(&decision),
                    actions.iter().map(|a| a.label.as_str()).collect::<Vec<_>>().join(" | ")
                );
            }
            let _ = &label;
            match decision.player() {
                Some(1) => {
                    if let Decision::IdleCmd { .. } = decision {
                        // A step that attacks is over after the Battle Phase.
                        if current.as_ref().is_some_and(|(i, _)| steps[*i].attack) && !battled {
                            battled = true;
                            if let Some(a) = actions.iter().find(|a| a.kind == ActionKind::ToBattlePhase) {
                                duel.respond(&a.response);
                                continue;
                            }
                        }
                        if let Some((i, before)) = current.take() {
                            let after = duel.state();
                            if verbose {
                                let db = engine.cards();
                                let names = |v: &[u32]| v.iter().map(|&c| db.name(c)).collect::<Vec<_>>().join(", ");
                                eprintln!(
                                    "   after step {i}: opponent GY [{}] banished [{}]",
                                    names(&after.players[1].grave),
                                    names(&after.players[1].banished.iter().map(|&(c, _)| c).collect::<Vec<_>>())
                                );
                            }
                            self.finish(&mut trace.steps[i], steps[i], &before, &after, &observer);
                        }
                        // The next step that can be played now.
                        let mut chosen = None;
                        while next < steps.len() {
                            let i = next;
                            next += 1;
                            if let Some(a) = find_act(engine, &actions, steps[i].act) {
                                trace.steps[i].available = true;
                                chosen = Some((i, a));
                                break;
                            }
                        }
                        match chosen {
                            Some((i, answer)) => {
                                current = Some((i, duel.state()));
                                observer.new_step();
                                windows_in_step = 0;
                                battled = false;
                                duel.respond(&answer);
                            }
                            None => {
                                complete = true;
                                break;
                            }
                        }
                    } else {
                        let step = current.as_ref().map(|(i, _)| steps[*i]);
                        duel.respond(&opponent_answer(engine, &decision, &actions, step));
                    }
                }
                _ => {
                    let window = match &decision {
                        Decision::SelectChain { forced: false, chains, .. } if !chains.is_empty() => Some(
                            chains
                                .iter()
                                .map(|a| {
                                    (
                                        ResponderId { code: engine.cards().canonical_code(a.card.code), description: a.description },
                                        a.card.location,
                                    )
                                })
                                .collect::<Vec<_>>(),
                        ),
                        Decision::EffectYesNo { card, description, .. } => Some(vec![(
                            ResponderId { code: engine.cards().canonical_code(card.code), description: *description },
                            card.location,
                        )]),
                        _ => None,
                    };
                    match (window, current.as_ref()) {
                        (Some(options), Some((step, _))) => {
                            let at = windows_in_step;
                            windows_in_step += 1;
                            let pick = match policy {
                                Policy::Use(id, skip) if !skip.contains(&(*step, at)) => options.iter().position(|(o, _)| *o == id),
                                Policy::UseAll(ids) => ids.iter().find_map(|id| options.iter().position(|(o, _)| o == id)),
                                Policy::UseAt(id, window) if window == (*step, at) => options.iter().position(|(o, _)| *o == id),
                                _ => None,
                            };
                            trace.windows.push(Window { step: *step, options: options.clone() });
                            match pick {
                                Some(i) => {
                                    trace.uses.push(Use {
                                        responder: options[i].0,
                                        step: *step,
                                        window: at,
                                        effect: None,
                                        targeted: false,
                                    });
                                    observer.waiting = Some(trace.uses.len() - 1);
                                    duel.respond(&window_answer(&decision, Some(i)));
                                }
                                None => duel.respond(&window_answer(&decision, None)),
                            }
                        }
                        (Some(_), None) => duel.respond(&window_answer(&decision, None)),
                        (None, _) => duel.respond(&your_answer(&decision, &actions)),
                    }
                }
            }
        }
        if verbose {
            for line in duel.log() {
                eprintln!("   engine log (end): {line}");
            }
        }
        if !complete {
            return Err(Error::Engine("the probe turn did not get through its steps".into()));
        }
        trace.state = duel.state();
        Ok(trace)
    }

    /// Whether each goal of a finished step was met.
    fn finish(&self, run: &mut StepRun, step: &KitStep, before: &DuelState, after: &DuelState, observer: &Observer) {
        let db = self.engine.cards();
        let count = |state: &DuelState, want: &Want| -> i32 {
            let p = &state.players[1];
            let matches = |c: u32| want.code.is_none_or(|w| db.canonical_code(c) == db.canonical_code(w));
            (match want.location {
                location::HAND => p.hand.iter().filter(|&&c| matches(c)).count(),
                location::GRAVE => p.grave.iter().filter(|&&c| matches(c)).count(),
                location::MZONE => p.monsters.iter().flatten().filter(|c| c.is_face_up() && matches(c.code)).count(),
                _ => 0,
            }) as i32
        };
        run.worked = step.goals.iter().map(|(_, want)| count(after, want) - count(before, want) >= want.delta).collect();
        run.negated = observer.step_negated;
    }
}

/// Your End Phase after a provisional line: optional effects are used,
/// lists are picked forward.
fn end_phase_answer(decision: &Decision, actions: &[Action]) -> Vec<u8> {
    if let Some(answer) = forward(decision, actions, |_| true) {
        return answer;
    }
    let act = actions.iter().find(|a| !matches!(a.kind, ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish));
    act.or(actions.first()).map(|a| a.response.clone()).unwrap_or_else(|| window_answer(decision, None))
}

/// Ends your turn at once (the empty board).
fn end_turn(decision: &Decision, actions: &[Action]) -> Vec<u8> {
    match actions.iter().find(|a| a.kind == ActionKind::EndTurn) {
        Some(a) => a.response.clone(),
        None => passive_answer(decision, actions),
    }
}

/// Your answer to a chance to respond: pass, or the effect at `index`.
fn window_answer(decision: &Decision, index: Option<usize>) -> Vec<u8> {
    match decision {
        Decision::EffectYesNo { .. } | Decision::YesNo { .. } => (index.is_some() as i32).to_le_bytes().to_vec(),
        _ => index.map_or(-1, |i| i as i32).to_le_bytes().to_vec(),
    }
}

/// In a list where cards are picked one at a time, picking goes forward:
/// finish when allowed, never take a card back (that would go round in circles).
fn forward(decision: &Decision, actions: &[Action], prefer: impl Fn(&Action) -> bool) -> Option<Vec<u8>> {
    let Decision::SelectUnselectCard { finishable, selectable, .. } = decision else { return None };
    if *finishable {
        if let Some(a) = actions.iter().find(|a| a.kind == ActionKind::Finish) {
            return Some(a.response.clone());
        }
    }
    // Picks of the selectable cards come first in the list of answers.
    let picks = &actions[..selectable.len().min(actions.len())];
    picks.iter().find(|a| prefer(a)).or(picks.first()).map(|a| a.response.clone())
}

/// Your other decisions while answering: aim at the opponent's cards.
fn your_answer(decision: &Decision, actions: &[Action]) -> Vec<u8> {
    if let Some(answer) = forward(decision, actions, |a| a.card.is_some_and(|c| c.controller == 1)) {
        return answer;
    }
    let theirs = actions.iter().find(|a| a.card.is_some_and(|c| c.controller == 1) && a.kind != ActionKind::Cancel);
    let forced = matches!(decision, Decision::SelectChain { forced: true, .. });
    let pick = theirs
        .or_else(|| if forced { actions.iter().find(|a| a.kind == ActionKind::Activate) } else { None })
        .or_else(|| actions.iter().find(|a| matches!(a.kind, ActionKind::Yes | ActionKind::Choose)))
        .or_else(|| actions.iter().find(|a| a.kind != ActionKind::Cancel))
        .or(actions.first());
    pick.map(|a| a.response.clone()).unwrap_or_else(|| window_answer(decision, None))
}

/// The opponent plays its step: its picks, its triggers, its option.
fn opponent_answer(engine: &Engine, decision: &Decision, actions: &[Action], step: Option<&KitStep>) -> Vec<u8> {
    let db = engine.cards();
    let listed = |list: &[u32], code: u32| list.iter().any(|&c| db.canonical_code(c) == db.canonical_code(code));
    let picks: &[u32] = step.map_or(&[], |s| &s.picks);
    let triggers: &[u32] = step.map_or(&[], |s| &s.triggers);
    if let Some(answer) = forward(decision, actions, |a| a.card.is_some_and(|c| listed(picks, c.code))) {
        return answer;
    }
    let first = |kinds: &[ActionKind]| actions.iter().find(|a| kinds.contains(&a.kind));
    let chosen = match decision {
        Decision::SelectChain { forced, .. } => {
            let own = actions.iter().find(|a| a.kind == ActionKind::Activate && a.card.is_some_and(|c| listed(triggers, c.code)));
            own.or_else(|| if *forced { first(&[ActionKind::Activate]) } else { first(&[ActionKind::Pass]) })
        }
        Decision::BattleCmd { .. } if step.is_some_and(|s| s.attack) => {
            first(&[ActionKind::Attack, ActionKind::ToMainPhase2, ActionKind::EndTurn])
        }
        Decision::BattleCmd { .. } => first(&[ActionKind::ToMainPhase2, ActionKind::EndTurn]),
        Decision::Option { .. } => actions.get(step.map_or(0, |s| s.option)).or(actions.first()),
        Decision::EffectYesNo { .. } | Decision::YesNo { .. } => first(&[ActionKind::Yes]),
        _ => {
            let picked = actions.iter().find(|a| a.card.is_some_and(|c| listed(picks, c.code)));
            let most = if step.is_some_and(|s| s.take_max) {
                actions
                    .iter()
                    .filter(|a| a.kind == ActionKind::Choose)
                    .max_by_key(|a| a.response.get(4..8).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
            } else {
                None
            };
            most.or(picked)
                .or_else(|| actions.iter().find(|a| !matches!(a.kind, ActionKind::Cancel | ActionKind::Finish)))
                .or(actions.first())
        }
    };
    chosen.or(actions.first()).map(|a| a.response.clone()).unwrap_or_else(|| window_answer(decision, None))
}

/// The idle answer that plays `act`, if the engine offers it.
fn find_act(engine: &Engine, actions: &[Action], act: Act) -> Option<Vec<u8>> {
    let db = engine.cards();
    let is = |c: &CardRef, code: u32| db.canonical_code(c.code) == db.canonical_code(code);
    actions
        .iter()
        .find(|a| {
            let Some(c) = &a.card else { return false };
            match act {
                Act::Activate(code, loc) => a.kind == ActionKind::Activate && c.location == loc && is(c, code),
                Act::NormalSummon(code) => a.kind == ActionKind::NormalSummon && is(c, code),
                Act::SpecialSummon(code) => a.kind == ActionKind::SpecialSummon && is(c, code),
            }
        })
        .map(|a| a.response.clone())
}
