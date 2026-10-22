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
use std::sync::Arc;

use ygo_engine::auto::passive_answer;
use ygo_engine::consts::{hint, location, position, reason};
use ygo_engine::{
    Action, ActionKind, CardDb, CardRef, Decision, DeckList, Duel, DuelState, Engine, Error, Event, Opponent, Placement, Step,
};

use crate::explain::{ChokeKind, ChokePoint};
use crate::follow::{line_picks, pick_again, Follower, Script};
use crate::matchup::RivalLine;
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
    /// The effect of a monster already on the field (not a trigger of its summon).
    FieldIgnition,
    DeckSummon,
    /// A Spell from the hand that does not touch the Deck.
    SpellNoDeck,
    /// A monster effect from the hand that does not touch the Deck.
    HandNoDeck,
}

impl Probe {
    pub const ALL: [Probe; 13] = [
        Probe::Spell,
        Probe::Search,
        Probe::HandEffect,
        Probe::SendToGrave,
        Probe::SpecialFromHand,
        Probe::FieldEffect,
        Probe::GraveEffect,
        Probe::NormalSummon,
        Probe::ExtraSummon,
        Probe::FieldIgnition,
        Probe::DeckSummon,
        Probe::SpellNoDeck,
        Probe::HandNoDeck,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Probe::Spell => "attivare una Magia",
            Probe::Search => "aggiungere una carta dal Deck alla mano",
            Probe::HandEffect => "attivare un effetto di mostro dalla mano",
            Probe::SendToGrave => "mandare un mostro dal Deck al Cimitero",
            Probe::SpecialFromHand => "Evocare Specialmente dalla mano",
            Probe::FieldEffect => "attivare l'effetto di un mostro appena evocato",
            Probe::GraveEffect => "attivare un effetto dal Cimitero",
            Probe::NormalSummon => "Evocare Normalmente",
            Probe::ExtraSummon => "Evocare dall'Extra Deck",
            Probe::FieldIgnition => "attivare l'effetto di un mostro già sul campo",
            Probe::DeckSummon => "Evocare Specialmente dal Deck",
            Probe::SpellNoDeck => "attivare una Magia senza toccare il Deck",
            Probe::HandNoDeck => "attivare un effetto di mostro dalla mano senza toccare il Deck",
        }
    }

    /// The activations weigh as often as their kind is played in real lines
    /// (`duelforge-cli kit-mix`, 1979 activations in the lines of the best 5
    /// boards of 45 decks, 22/10/2026): Spells and Traps 31% (half touch the
    /// Deck), effects from the GY 18%, monster effects from the hand 11% and
    /// on the field 11%, summon triggers 29% (two thirds touch the Deck). The
    /// kinds the kit has no card for (effects from the GY that touch the
    /// Deck, summon triggers that do not) are left out. Together they still
    /// weigh 2.7, as before: summons stopped keep their weight against them.
    pub fn weight(self) -> f64 {
        match self {
            Probe::Spell => 0.13,
            Probe::Search => 0.25,
            Probe::HandEffect => 0.12,
            Probe::SendToGrave => 0.1,
            Probe::FieldEffect => 0.63,
            Probe::GraveEffect => 0.41,
            Probe::FieldIgnition => 0.18,
            Probe::DeckSummon => 0.16,
            Probe::SpellNoDeck => 0.5,
            Probe::HandNoDeck => 0.22,
            Probe::SpecialFromHand => 0.3,
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

/// A way to take one card of your board out of the opponent's way, aimed
/// at it (the protection of each piece, D15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Removal {
    Destroy,
    Banish,
    FaceDown,
    Negate,
}

impl Removal {
    pub fn label(self) -> &'static str {
        match self {
            Removal::Destroy => "distruggere",
            Removal::Banish => "bandire",
            Removal::FaceDown => "girare coperto",
            Removal::Negate => "negare",
        }
    }
}

/// One card of your board against each removal aimed at it, with only its
/// own effects to answer: the removals it is still usable after, and those
/// it is not.
#[derive(Debug, Clone)]
pub struct Protection {
    pub responder: ResponderId,
    pub location: u8,
    pub holds: Vec<Removal>,
    pub falls: Vec<Removal>,
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
    /// A breaker is played without it when the engine does not offer it.
    optional: bool,
}

impl KitStep {
    /// A step that only picks `picks` wherever asked (the targets of the
    /// opponent's answer to one of your links).
    fn aiming(picks: Vec<u32>) -> KitStep {
        KitStep {
            act: Act::Activate(0, location::HAND),
            picks,
            take_max: false,
            option: 0,
            triggers: Vec::new(),
            attack: false,
            goals: Vec::new(),
            needs: None,
            optional: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Act {
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

/// The `location` of a goal met when the step's own activation resolves
/// without being negated: for an effect that moves no card.
const RESOLVED: u8 = 0;

/// A board breaker of the kit.
#[derive(Debug, Clone)]
pub struct Breaker {
    pub name: String,
    /// How often it matters: the average over breakers is weighted by it.
    pub weight: f64,
    steps: Vec<KitStep>,
    /// Its cards the opponent has only in its run (see [`Arsenal`]).
    arsenal: Arsenal,
}

impl Breaker {
    /// A card of the opponent's own deck played as a breaker: activated from
    /// the hand, or Special Summoned by its own procedure and then its effect
    /// on the field, when it has one (it may also have done its work being
    /// summoned, as the Kaiju do).
    pub(crate) fn card(name: String, weight: f64, code: u32, summon: bool) -> Breaker {
        let step = |act: Act, optional: bool| KitStep {
            act,
            picks: Vec::new(),
            take_max: false,
            option: 0,
            triggers: vec![code],
            attack: false,
            goals: Vec::new(),
            needs: None,
            optional,
        };
        let steps = if summon {
            vec![step(Act::SpecialSummon(code), false), step(Act::Activate(code, location::MZONE), true)]
        } else {
            vec![step(Act::Activate(code, location::HAND), false)]
        };
        Breaker { name, weight, steps, arsenal: Arsenal::default() }
    }

    /// The one card it plays, if it plays one.
    pub(crate) fn code(&self) -> Option<u32> {
        match self.steps.as_slice() {
            [KitStep { act: Act::Activate(code, _) | Act::SpecialSummon(code) | Act::NormalSummon(code), .. }] => Some(*code),
            _ => None,
        }
    }
}

/// The opponent's cards and what it does with them.
#[derive(Debug, Clone)]
pub struct Kit {
    opponent: Opponent,
    steps: Vec<KitStep>,
    pub breakers: Vec<Breaker>,
    /// What the opponent answers your first interruption with, when it can
    /// ([`Measure::countered`]).
    counters: Vec<u32>,
    /// The cards that remove one of yours ([`Prober::protection`]), and
    /// where the cards they can aim at are.
    removals: Vec<(Removal, u8, u32)>,
    /// The opponent plays a line of its deck again instead of the steps
    /// one by one (the probe turn against a deck, level 2).
    script: Option<Arc<LineScript>>,
}

/// Cards added to the opponent's hand and Extra Deck only in the runs that
/// play them: one card more in its hand changes the random cards your
/// effects take from it and the monster it summons when one lets it, in the
/// search as in the other runs of the measure.
#[derive(Debug, Clone, Default)]
struct Arsenal {
    hand: Vec<u32>,
    extra: Vec<u32>,
}

/// A line of the opponent's deck to play again in its turn: its answers,
/// and for each answer the step (one of its moves) it plays, if any.
#[derive(Debug)]
struct LineScript {
    script: Script,
    step_of: Vec<Option<usize>>,
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
        let dice = code("Speedroid Red-Eyed Dice");
        let filler = code("Alexandrite Dragon");
        let bystial = code("Bystial Magnamhut");
        let dian = code("Dian Keto the Cure Master");

        let step = |act: Act, picks: Vec<u32>, goals: Vec<(Probe, Want)>, needs: Option<Probe>| KitStep {
            act,
            picks,
            take_max: false,
            option: 0,
            triggers: Vec::new(),
            attack: false,
            goals,
            needs,
            optional: false,
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
        // A Spell that touches no Deck and needs nothing on the field.
        if let Some(dian) = dian {
            hand.push(dian);
            steps.push(step(Act::Activate(dian, location::HAND), vec![], vec![(Probe::SpellNoDeck, want(None, RESOLVED, 0))], None));
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
        // A monster effect from the hand that touches no Deck: it banishes the
        // Thunder Dragon discarded before and comes to the field (after
        // Terrortop, which comes only while its controller has no monsters).
        if let (Some(thunder), Some(bystial)) = (thunder, bystial) {
            hand.push(bystial);
            steps.push(step(
                Act::Activate(bystial, location::HAND),
                vec![thunder],
                vec![(Probe::HandNoDeck, want(Some(bystial), location::MZONE, 1))],
                Some(Probe::HandEffect),
            ));
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
                // Link Spider's own effect: a Normal Monster from the hand
                // (the plain cards drawn at the start of the turn).
                if let Some(filler) = filler {
                    steps.push(step(
                        Act::Activate(spider, location::MZONE),
                        vec![filler],
                        vec![(Probe::FieldIgnition, want(Some(filler), location::MZONE, 1))],
                        Some(Probe::ExtraSummon),
                    ));
                }
            }
        }
        // Taketomborg (searched by Terrortop) comes from the hand, then is
        // tributed for a Speedroid Tuner from the Deck. Last: after it only
        // WIND monsters can be Special Summoned.
        if let (Some(taketomborg), Some(dice)) = (taketomborg, dice) {
            deck.push(dice);
            steps.push(step(Act::SpecialSummon(taketomborg), vec![], vec![], Some(Probe::FieldEffect)));
            steps.push(step(
                Act::Activate(taketomborg, location::MZONE),
                vec![dice],
                vec![(Probe::DeckSummon, want(Some(dice), location::MZONE, 1))],
                Some(Probe::FieldEffect),
            ));
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
            breakers.push(Breaker { name: label.to_owned(), weight, steps: vec![s], arsenal: Arsenal::default() });
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
                arsenal: Arsenal::default(),
            });
        }

        // The cards added from D15 on are in the opponent's hand and Extra
        // Deck only in the runs that play them (an [`Arsenal`]).
        // The breakers the Master Duel Meta lists play in their Main Deck
        // (by axis): a targeted negation used before anything else (Infinite
        // Impermanence comes from the hand while its controller has no
        // cards), and a Fusion your negations cannot answer that takes your
        // monsters as materials (Super Polymerization, with the two Fusions
        // those lists play it for: any two monsters with the same Attribute
        // are materials for one of them).
        if let Some(imperm) = code("Infinite Impermanence") {
            breakers.push(Breaker {
                name: "Infinite Impermanence".to_owned(),
                weight: 1.0,
                steps: vec![step(Act::Activate(imperm, location::HAND), vec![], vec![], None)],
                arsenal: Arsenal { hand: vec![imperm], extra: Vec::new() },
            });
        }
        if let (Some(poly), Some(mudragon), Some(garura)) =
            (code("Super Polymerization"), code("Mudragon of the Swamp"), code("Garura, Wings of Resonant Life"))
        {
            breakers.push(Breaker {
                name: "Super Polymerization".to_owned(),
                weight: 1.0,
                steps: vec![step(Act::Activate(poly, location::HAND), vec![], vec![], None)],
                arsenal: Arsenal { hand: vec![poly], extra: vec![mudragon, garura] },
            });
        }
        // The opponent's answer to your first interruption it can answer:
        // Infinite Impermanence on a monster's effect while the opponent has
        // no cards. Ghost Ogre was tried too and left out for the reason above.
        let counters: Vec<u32> = [code("Infinite Impermanence")].into_iter().flatten().collect();

        // The last cards of the Deck are drawn or sent first (the draw of the
        // turn, Pot of Greed, the top card Glow-Up Bulb sends): plain ones,
        // so the cards to search stay.
        deck.extend([filler; 6].into_iter().flatten());
        // A Deck as big as the opponent's during your first turn (40 less
        // its 5 cards), plain cards at the bottom: effects that look at its
        // Deck see what they would (That Grass Looks Greener sends cards
        // until your Deck is no bigger).
        const DECK: usize = 35;
        if let Some(filler) = filler {
            let missing = DECK.saturating_sub(deck.len());
            deck.splice(0..0, std::iter::repeat_n(filler, missing));
        }
        // Removals aimed at one card, each the plainest card of its kind (the
        // protection of each piece), in the hand for that report only.
        let removals: Vec<(Removal, u8, u32)> = [
            (Removal::Destroy, location::MZONE, "Offerings to the Doomed"),
            (Removal::Banish, location::MZONE, "Spiritual Entanglement"),
            (Removal::FaceDown, location::MZONE, "Book of Moon"),
            (Removal::Negate, location::MZONE, "Infinite Impermanence"),
            (Removal::Destroy, location::SZONE, "Mystical Space Typhoon"),
            (Removal::Banish, location::SZONE, "Cosmic Cyclone"),
        ]
        .into_iter()
        .filter_map(|(r, at, name)| Some((r, at, code(name)?)))
        .collect();
        // An Extra Deck like a real one (15 cards): a
        // card taken from it is a small loss, not the end of its Extra
        // Deck summons (Link Spider alone was a floodgate when banished).
        // Monsters with no effect, so they change nothing while there.
        for (name, copies) in GENERIC_EXTRA {
            extra.extend(code(name).into_iter().flat_map(|c| std::iter::repeat_n(c, copies)));
        }
        Kit { opponent: Opponent { hand, deck, extra }, steps, breakers, counters, removals, script: None }
    }

    /// The opponent's cards: give them to the search too
    /// ([`crate::SearchConfig::opponent`]), so the lines it finds play the
    /// same in the probe turn.
    pub fn opponent(&self) -> &Opponent {
        &self.opponent
    }

    /// The opponent's cards with `arsenal` too.
    fn armed(&self, arsenal: &Arsenal) -> Opponent {
        let mut armed = self.opponent.clone();
        for &c in &arsenal.hand {
            if !armed.hand.contains(&c) {
                armed.hand.push(c);
            }
        }
        armed.extra.extend(&arsenal.extra);
        armed
    }

    /// This kit with the breakers of `rival`, their cards in the opponent's
    /// hand, and its real Deck: some breakers pay a cost from the Extra Deck
    /// (Ultimate Slayer) or take cards from the Deck (Interrupted Kaiju
    /// Slumber). Random cards of its Main Deck take the place of the plain
    /// ones at the bottom, so the Deck keeps the size the search played
    /// against; its Extra Deck joins the kit's.
    pub(crate) fn with_breakers(&self, breakers: Vec<Breaker>, rival: &DeckList, seed: u64) -> Kit {
        let mut kit = self.clone();
        let mut main = rival.main.clone();
        for b in &breakers {
            for s in &b.steps {
                let (Act::Activate(code, _) | Act::SpecialSummon(code) | Act::NormalSummon(code)) = s.act;
                if !kit.opponent.hand.contains(&code) {
                    kit.opponent.hand.push(code);
                    // That copy is in its hand, no more in its Deck.
                    if let Some(i) = main.iter().position(|&c| c == code) {
                        main.swap_remove(i);
                    }
                }
            }
        }
        shuffle(&mut main, seed);
        let deck = &mut kit.opponent.deck;
        let filler = deck.last().copied();
        let plain = deck.iter().take_while(|&&c| Some(c) == filler).count().min(main.len());
        deck.splice(0..plain, main.into_iter().take(plain));
        kit.opponent.extra.extend(&rival.extra);
        kit.breakers = breakers;
        // Its own breakers and nothing more: the kit's answer and removals
        // are not its cards.
        kit.counters.clear();
        kit.removals.clear();
        kit
    }

    /// The opponent of `rival` playing `line` again ([`crate::rival_lines`])
    /// instead of the kit's steps, with no breakers. It has the line's hand
    /// and its Deck in the order of the duel the line was found in, the
    /// plain card on top for the draw of its turn, and its Extra Deck. Each
    /// move of the line is a step: a summon works when the monster lands,
    /// an activation when it is not negated (judged by the measure).
    pub(crate) fn with_line(&self, engine: &Engine, rival: &DeckList, line: &RivalLine, seed: u64) -> Kit {
        let db = engine.cards();
        let mut main = rival.main.clone();
        // As the goldfish duel deals it: the hand out first, the first copy
        // of each card, then the rest shuffled.
        for &code in &line.hand {
            if let Some(i) = main.iter().position(|&c| db.canonical_code(c) == db.canonical_code(code)) {
                main.remove(i);
            }
        }
        ygo_engine::shuffle(&mut main, seed);
        main.extend(self.opponent.deck.last().copied());
        let steps = line
            .moves
            .iter()
            .map(|&(code, kind, location)| {
                let act = match kind {
                    ActionKind::NormalSummon => Act::NormalSummon(code),
                    ActionKind::SpecialSummon => Act::SpecialSummon(code),
                    _ => Act::Activate(code, location),
                };
                let goals = match act {
                    Act::NormalSummon(code) => vec![(Probe::NormalSummon, Want { code: Some(code), location: location::MZONE, delta: 1 })],
                    Act::SpecialSummon(code) => {
                        vec![(Probe::SpecialFromHand, Want { code: Some(code), location: location::MZONE, delta: 1 })]
                    }
                    Act::Activate(..) => Vec::new(),
                };
                KitStep {
                    act,
                    picks: Vec::new(),
                    take_max: false,
                    option: 0,
                    triggers: Vec::new(),
                    attack: false,
                    goals,
                    needs: None,
                    optional: false,
                }
            })
            .collect();
        Kit {
            opponent: Opponent { hand: line.hand.clone(), deck: main, extra: rival.extra.clone() },
            steps,
            breakers: Vec::new(),
            counters: Vec::new(),
            removals: Vec::new(),
            script: Some(Arc::new(LineScript { script: line.script.clone(), step_of: line.move_of.clone() })),
        }
    }

    /// The probes this kit can measure.
    pub fn probes(&self) -> Vec<Probe> {
        let mut out: Vec<Probe> = self.steps.iter().flat_map(|s| s.goals.iter().map(|(p, _)| *p)).collect();
        out.sort();
        out.dedup();
        out
    }
}

/// `cards` shuffled with `seed`: the same Deck for the same duel.
fn shuffle(cards: &mut [u32], seed: u64) {
    let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
    for i in (1..cards.len()).rev() {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        cards.swap(i, ((z ^ (z >> 31)) % (i as u64 + 1)) as usize);
    }
}

/// Where your turn starts from in a run: the line of a board, or ending
/// the turn at once (the empty board the floodgates are compared with).
#[derive(Clone, Copy)]
pub(crate) enum Prelude<'a> {
    Line(&'a Line),
    EndAtOnce,
    /// These cards on the field from the start, then the turn ends.
    Placed(&'a [Placement]),
    /// Your answers of a line (not the opponent's), played again by what
    /// they do: the opponent passes whenever it is asked, and its cards are
    /// another deck's, so a random effect on its hand (Elfnote Tinia) can
    /// change the questions after it.
    Own(&'a Script),
    /// A line played up to its answer at this index (not included), given
    /// at a free moment of yours: the turn ends there, as if a handtrap
    /// had stopped what came next ([`Prober::expected_loss`]).
    Cut(&'a Line, usize),
}

/// The chance that the opponent, going second, holds at least one handtrap
/// of each kind in its 6 cards: the mean copies in the Main Decks of the 41
/// Master Duel Meta lists of 22/10/2026, in a 40-card deck
/// (`scripts/handtrap_rates.py`, which also names the cards of each kind).
/// The app has no lists to count: it uses these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandTrapRates {
    /// Ash Blossom & Joyous Spring (2.46 copies).
    pub deck_touch: f64,
    /// Effect Veiler, Infinite Impermanence, Ghost Ogre & Snow Rabbit (2.71).
    pub field_effect: f64,
    /// Nibiru, the Primal Being (0.22).
    pub fifth_summon: f64,
    /// Droll & Lock Bird (1.12).
    pub second_search: f64,
    /// Maxx "C" and the Mulcharmy (3.76).
    pub special_summons: f64,
}

impl Default for HandTrapRates {
    fn default() -> Self {
        HandTrapRates { deck_touch: 0.335, field_effect: 0.363, fifth_summon: 0.035, second_search: 0.167, special_summons: 0.47 }
    }
}

impl HandTrapRates {
    pub fn chance(&self, kind: ChokeKind) -> f64 {
        match kind {
            ChokeKind::DeckTouch => self.deck_touch,
            ChokeKind::FieldEffect => self.field_effect,
            ChokeKind::FifthSummon => self.fifth_summon,
            ChokeKind::SecondSearch => self.second_search,
        }
    }
}

/// What a card given to the opponent by Maxx "C" or a Mulcharmy is worth,
/// in interruptions: a starting value, to calibrate (B6).
const MAXX_PER_SUMMON: f64 = 0.1;

/// What the handtraps are expected to take away from a line
/// ([`Prober::expected_loss`]), in interruptions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExpectedLoss {
    /// Σ chance × loss over the kinds of handtrap.
    pub expected: f64,
    /// The value lost at the worst choke point of each kind the line has.
    pub losses: Vec<(ChokeKind, f64)>,
    /// What Maxx "C" or a Mulcharmy gives the opponent.
    pub maxx: f64,
    /// Places the line was cut at (two runs each).
    pub cuts: usize,
}

/// What the opponent's cards your turn took away are worth, in
/// interruptions, by where they were (B5a; the weights are a start, to
/// calibrate with D17): a card of its 15-card Extra Deck about a fifteenth
/// of losing its Extra Deck summons (0.6), a card of its hand about one of
/// its six cards, a card of its Deck little, a card of its field like a
/// destroyed monster.
pub fn attrition(taken: &[(u32, u8)]) -> f64 {
    taken.iter().map(|&(_, from)| taken_weight(from)).sum::<f64>() + 0.0
}

/// What a card of the opponent taken from `from` is worth ([`attrition`]).
pub(crate) fn taken_weight(from: u8) -> f64 {
    match from {
        location::EXTRA => 0.04,
        location::HAND => 0.15,
        location::DECK => 0.01,
        _ => 0.3,
    }
}

/// The rest of the kit opponent's Extra Deck besides Link Spider: a Fusion
/// and a Link Monster with no effect, seven copies each. Copies of two
/// cards, not fourteen cards: what to take from it is then a choice of how
/// many of each, not one board for every set of cards taken.
const GENERIC_EXTRA: [(&str, usize); 2] = [("Flame Swordsman", 7), ("Gaia Saber, the Lightning Shadow", 7)];

/// What a measure plays besides each response alone: all of them together
/// (what they share), and the board breakers. Each costs runs (a duel each).
#[derive(Clone, Copy)]
struct Parts {
    together: bool,
    breakers: bool,
    /// Windows each response is tried at, to see what it stops.
    windows: usize,
    /// Runs with a response used whenever offered, to count its uses.
    use_rounds: usize,
}

impl Parts {
    const ALL: Parts = Parts { together: true, breakers: true, windows: 16, use_rounds: 3 };
}

/// How much a cheap measure ([`Prober::measure_cheap`]) tries: most of the
/// time of a full one goes in trying each response at every window and
/// counting its uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cheap {
    /// Windows each response is tried at (16 in a full measure).
    pub windows: usize,
    /// Runs counting the uses of a response (3 in a full measure; none
    /// counts it used once).
    pub use_rounds: usize,
}

/// What a cheap measure played on a board ([`Prober::measure_cheap`]), for
/// the full one to go on from it ([`Prober::measure_resumed`]) instead of
/// playing those runs again: the run with no answer, and for each response
/// the windows tried and the rounds counting its uses.
#[derive(Debug, Clone)]
pub struct Partial {
    all: RunTrace,
    responses: Vec<Response>,
}

/// One response as far as a measure tried it.
#[derive(Debug, Clone)]
struct Response {
    id: ResponderId,
    /// Windows tried (the first ones in the order of the measure).
    windows: usize,
    covers: Vec<Probe>,
    effect: Option<Effect>,
    targeted: bool,
    /// Rounds counting its uses played, and if they are over.
    rounds: usize,
    settled: bool,
    uses: u32,
    skip: HashSet<WindowAt>,
}

impl Response {
    fn new(id: ResponderId) -> Response {
        Response {
            id,
            windows: 0,
            covers: Vec::new(),
            effect: None,
            targeted: false,
            rounds: 0,
            settled: false,
            uses: 1,
            skip: HashSet::new(),
        }
    }
}

/// The opponent's hand and GY, what the goals of a step look at (cheaper
/// to read than the whole duel).
struct Piles {
    hand: Vec<u32>,
    grave: Vec<u32>,
}

impl Piles {
    fn of(duel: &Duel) -> Piles {
        let codes = |loc| duel.query_location(1, loc).into_iter().flatten().map(|(code, _)| code).collect();
        Piles { hand: codes(location::HAND), grave: codes(location::GRAVE) }
    }
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

/// What a run knows beyond its policy.
#[derive(Clone, Copy, Default)]
struct Context<'a> {
    /// Your effects offered with no response at all: the others come from
    /// one of your responses (a monster it summoned, its own effect from
    /// the GY). They answer too, whenever offered, and what they stop is
    /// the response's (Big Welcome Labrynth stops nothing itself: the
    /// Labrynth monster it summons does).
    known: Option<&'a HashSet<ResponderId>>,
    /// What each of your responders is worth alone (by code): of two that
    /// can pay a cost, the one worth less pays.
    worth: Option<&'a HashMap<u32, f64>>,
    /// Once the opponent is done, play on to your next Main Phase.
    turn_three: bool,
    /// The opponent answers your first link it can with one of the kit's
    /// counters, aimed at that link's card, else at these (your
    /// responders, the one worth the most first).
    counter: Option<&'a [u32]>,
    /// The opponent's cards, if not the kit's.
    opponent: Option<&'a Opponent>,
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
    follow_up: Option<FollowUp>,
    /// The opponent answered one of your links ([`Context::counter`]).
    countered: bool,
    /// The opponent's cards your turn took away (sent to the GY, banished
    /// or taken by you) and where they were: its code and location.
    taken: Vec<(u32, u8)>,
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
    /// The summons begun and not done yet, the last begun last, each with
    /// the opponent's monsters in it (yours can be summoned while its summon
    /// waits). Leaving the field before the summon is done is having it negated.
    summons: Vec<Vec<u32>>,
    /// The last event began a Special Summon: monsters summoned together
    /// come in a row, before the summon is done.
    special_now: bool,
    /// The opponent's monsters that landed in this step (a summon done,
    /// whatever happens to them after).
    landed: Vec<u32>,
    /// The links of the chain being built that have not resolved yet.
    open: Vec<CardRef>,
    /// The opponent's next link is its answer to yours, not its step's.
    answering: bool,
}

impl Observer {
    fn see(&mut self, engine: &Engine, event: &Event, uses: &mut [Use]) {
        let db = engine.cards();
        let special_before = std::mem::replace(&mut self.special_now, matches!(event, Event::SpecialSummoning { .. }));
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
                self.open.push(*card);
                if card.controller == 1 && !std::mem::take(&mut self.answering) {
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
                self.open.pop();
                if let Some(u) = self.solving.take() {
                    self.resolved.push(u);
                }
            }
            Event::ChainEnd => {
                self.open.clear();
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
                let code = db.canonical_code(*code);
                // Off the field before landing: the summon was negated.
                let mut not_landed = false;
                if from.controller == 1 && from.location == location::MZONE {
                    for summon in self.summons.iter_mut().rev() {
                        if let Some(i) = summon.iter().position(|&c| c == code) {
                            summon.swap_remove(i);
                            not_landed = true;
                            break;
                        }
                    }
                }
                let Some(u) = self.solving else { return };
                if from.controller != 1 || from.location & (location::MZONE | location::SZONE | location::HAND) == 0 {
                    return;
                }
                if self.negated_code == Some(code) {
                    better(uses, u, Effect::NegateAndRemove);
                    return;
                }
                if not_landed {
                    better(uses, u, Effect::NegateSummon);
                    return;
                }
                let effect = match to.location & !location::OVERLAY {
                    // Attached to an Xyz Monster: out of play, as banished.
                    _ if to.location & location::OVERLAY != 0 => Effect::Banish,
                    // Taken: the opponent loses it and you get it, at least
                    // as good as destroying it.
                    _ if to.controller == 0 && to.location & (location::MZONE | location::SZONE) != 0 => Effect::Destroy,
                    // A monster put in its owner's Spell & Trap Zone (as a
                    // Continuous Spell) is out of play like a banished one.
                    location::SZONE if from.location == location::MZONE => Effect::Banish,
                    location::REMOVED => Effect::Banish,
                    location::DECK | location::EXTRA => Effect::ToDeck,
                    location::HAND => Effect::Bounce,
                    location::GRAVE if why & reason::DESTROY != 0 => Effect::Destroy,
                    location::GRAVE => Effect::ToGrave,
                    _ => return,
                };
                better(uses, u, effect);
            }
            Event::PositionChange { card, to, .. } => {
                if let Some(u) = self.solving {
                    if card.controller == 1 {
                        // Face-down, a monster has no effects any more.
                        let face_down = u32::from(*to) & (position::FACEDOWN_ATTACK | position::FACEDOWN_DEFENSE) != 0;
                        better(uses, u, if face_down { Effect::NegateEffect } else { Effect::Position });
                    }
                }
            }
            Event::NormalSummoning { card } | Event::SpecialSummoning { card } | Event::FlipSummoning { card } => {
                let code = db.canonical_code(card.code);
                let theirs = (card.controller == 1).then_some(code);
                match self.summons.last_mut() {
                    Some(summon) if special_before && matches!(event, Event::SpecialSummoning { .. }) => summon.extend(theirs),
                    _ => self.summons.push(theirs.into_iter().collect()),
                }
                if card.controller == 1 && self.step_links.is_empty() && !matches!(event, Event::FlipSummoning { .. }) {
                    self.summoning = Some(code);
                }
            }
            Event::Summoned | Event::SpecialSummoned => {
                if let Some(mut summon) = self.summons.pop() {
                    self.landed.append(&mut summon);
                }
            }
            _ => {}
        }
    }

    /// How well an answer aims at what the opponent is doing: the card of
    /// its last link still on the chain first, then the monster it is
    /// summoning, then any of its cards (0 is best).
    fn aim(&self, db: &CardDb, a: &Action) -> u8 {
        let Some(c) = a.card.filter(|c| c.controller == 1) else { return 3 };
        let top = self.open.iter().rev().find(|l| l.controller == 1);
        if top
            .is_some_and(|l| l.location == c.location && l.sequence == c.sequence && db.canonical_code(l.code) == db.canonical_code(c.code))
        {
            0
        } else if self.summoning == Some(db.canonical_code(c.code)) {
            1
        } else {
            2
        }
    }

    fn new_step(&mut self) {
        self.step_links.clear();
        self.step_negated = false;
        self.summoning = None;
        self.summons.clear();
        self.landed.clear();
    }
}

/// The cards of `deck` that can be used from the hand during the
/// opponent's turn (handtraps): those the probe turn offers from your hand
/// while the kit's opponent plays against your empty board. Read from the
/// engine, so it holds for any card, old or new.
pub fn hand_traps(engine: &Engine, deck: &DeckList, seed: u64, kit: &Kit) -> Vec<u32> {
    let db = engine.cards();
    // One copy of each card (any artwork), five at a time.
    let mut cards: Vec<u32> = Vec::new();
    for &c in &deck.main {
        if !cards.iter().any(|&k| db.canonical_code(k) == db.canonical_code(c)) {
            cards.push(c);
        }
    }
    let steps: Vec<&KitStep> = kit.steps.iter().collect();
    let mut found: Vec<u32> = Vec::new();
    for hand in cards.chunks(5) {
        let prober = Prober::new(engine, deck, hand, seed, kit);
        let Ok(trace) = prober.run(Prelude::EndAtOnce, &steps, Policy::Pass) else { continue };
        for (id, loc) in trace.windows.iter().flat_map(|w| &w.options) {
            if *loc == location::HAND && !found.contains(&id.code) {
                found.push(id.code);
            }
        }
    }
    found.sort_unstable();
    found
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
    /// When the runs stop (a search's learning round).
    deadline: Option<std::time::Instant>,
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

/// One of your effects against the opponent's typical moves
/// ([`Kit::with_moves`]): the moves it stops, by index.
#[derive(Debug, Clone)]
pub struct MoveInterruption {
    pub responder: ResponderId,
    pub location: u8,
    pub effect: Effect,
    pub targeted: bool,
    pub stops: Vec<usize>,
    /// Share it takes away of the moves' weight your board lets the
    /// opponent make (those lost with the ones it stops included).
    pub coverage: f64,
    pub value: f64,
}

/// The board against the opponent's typical moves.
#[derive(Debug, Clone, Default)]
pub struct MovesMeasure {
    /// Each move: made on an empty board, and against this board with no answer.
    pub possible: Vec<bool>,
    pub here: Vec<bool>,
    /// Moves your board stops with no answer at all (by index).
    pub floodgated: Vec<usize>,
    pub interruptions: Vec<MoveInterruption>,
    /// Share of the moves' weight your board stops with no answer (those
    /// lost with them included).
    pub floodgate_share: f64,
    /// Interruption value against these moves (sum of `interruptions`).
    pub value: f64,
    /// Share of the moves' weight your board lets the opponent make that is
    /// still made with all your answers used, the strongest first.
    pub left: f64,
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
    /// Interruption value of the field (not the hand) left on average
    /// after a board breaker.
    pub resilience: f64,
    /// Your cards the engine let you use at least once during the
    /// opponent's turn, and where (a handtrap missing here cannot be used
    /// with this board: the Mulcharmy only while you control no cards).
    pub offered: Vec<(u32, u8)>,
    /// What you have for your next turn after the breaker that takes away
    /// the most (none if the run could not get there).
    pub follow_up: Option<FollowUp>,
    /// Share of what all your effects stop together that they still stop
    /// when the opponent answers the first of them it can (none if it could
    /// answer none, or they stop nothing).
    pub countered: Option<f64>,
    /// The opponent's cards your turn took away (sent to the GY, banished
    /// or taken), with where they were: its Extra Deck, Deck, hand or field.
    pub taken: Vec<(u32, u8)>,
    /// What they are worth, in interruptions ([`attrition`]).
    pub attrition: f64,
    /// How long each part of the measure took.
    pub seconds: MeasureSeconds,
}

/// The time of each part of a measure, in seconds.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MeasureSeconds {
    /// The empty board, the board with no answer, and each response alone.
    pub responses: f64,
    /// All the responses together, and the opponent's answer to the first.
    pub together: f64,
    /// The board breakers.
    pub breakers: f64,
    /// Your next turn after the worst breaker.
    pub follow_up: f64,
}

/// Your next turn, after the opponent's: the board breaker that takes away
/// the most played, then its End Phase and your Draw and Standby Phases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FollowUp {
    /// Effects you can activate at the start of your Main Phase from the GY
    /// or banishment, and summons from there (each card and effect once).
    pub rebuild: usize,
    /// Cards added to your hand and monsters summoned since the end of the
    /// opponent's Main Phase, your draw aside.
    pub gained: usize,
}

impl<'a> Prober<'a> {
    pub fn new(engine: &'a Engine, deck: &'a DeckList, hand: &'a [u32], seed: u64, kit: &'a Kit) -> Self {
        Prober { engine, deck, hand, seed, kit, baseline: None, deadline: None }
    }

    /// From `deadline` on, every run fails at its next decision: a measure
    /// with your effects checked at each chain window can take minutes
    /// (quick Synchro Summons with many materials), more than a search may
    /// wait.
    pub(crate) fn until(&mut self, deadline: std::time::Instant) {
        self.deadline = Some(deadline);
    }

    /// Past the deadline: a measure taken now may be missing runs.
    pub(crate) fn out_of_time(&self) -> bool {
        self.deadline.is_some_and(|d| std::time::Instant::now() >= d)
    }

    /// Measures the board reached by `line`.
    pub fn measure(&mut self, line: &Line) -> Result<Measure, Error> {
        self.measure_from(Prelude::Line(line), Parts::ALL)
    }

    /// Measures a board put straight on the field (with the hand of the
    /// prober), without playing the turn that makes it.
    pub fn measure_placed(&mut self, placed: &[Placement]) -> Result<Measure, Error> {
        self.measure_from(Prelude::Placed(placed), Parts::ALL)
    }

    /// [`Prober::measure`] without the board breakers: no
    /// [`Measure::breakers`] nor follow-up, and the [`Measure::resilience`]
    /// of a board none is tried on (to see what the measure costs by part).
    pub fn measure_unbroken(&mut self, line: &Line) -> Result<Measure, Error> {
        self.measure_from(Prelude::Line(line), Parts { together: true, breakers: false, ..Parts::ALL })
    }

    /// The quick value of the board `line` reaches ([`Prober::quick_value`]):
    /// two runs, in interruptions.
    pub fn quick_measure(&mut self, line: &Line) -> Result<f64, Error> {
        self.quick_value(Prelude::Line(line))
    }

    /// [`Prober::measure`] of each interruption alone ([`Interruption::alone`]):
    /// the runs that only change the rest are not played.
    pub(crate) fn measure_alone(&mut self, line: &Line) -> Result<Measure, Error> {
        self.measure_from(Prelude::Line(line), Parts { together: false, breakers: false, ..Parts::ALL })
    }

    /// A cheap [`Prober::measure`] of the board `line` reaches, to choose
    /// the boards worth a full one (B14): each response tried at fewer
    /// windows and its uses counted in fewer runs, as `cheap` says; no
    /// breakers nor answer to it. With what it played, for
    /// [`Prober::measure_resumed`].
    pub fn measure_cheap(&mut self, line: &Line, cheap: Cheap) -> Result<(Measure, Partial), Error> {
        let parts = Parts { together: true, breakers: false, windows: cheap.windows, use_rounds: cheap.use_rounds };
        let (measure, partial) = self.measure_partial(Prelude::Line(line), parts, None, true)?;
        Ok((measure, partial.expect("kept")))
    }

    /// [`Prober::measure`] of the board `line` reaches, going on from what
    /// its cheap measure played (`partial`, from a prober of the same hand):
    /// the same measure, without those runs.
    pub fn measure_resumed(&mut self, line: &Line, partial: Partial) -> Result<Measure, Error> {
        self.measure_partial(Prelude::Line(line), Parts::ALL, Some(partial), false).map(|(m, _)| m)
    }

    /// [`Prober::measure_placed`] without the board breakers: no
    /// [`Measure::breakers`], and the [`Measure::resilience`] of a board none
    /// is tried on.
    pub(crate) fn measure_placed_unbroken(&mut self, placed: &[Placement]) -> Result<Measure, Error> {
        self.measure_from(Prelude::Placed(placed), Parts { together: true, breakers: false, ..Parts::ALL })
    }

    fn measure_from(&mut self, prelude: Prelude, parts: Parts) -> Result<Measure, Error> {
        self.measure_partial(prelude, parts, None, false).map(|(m, _)| m)
    }

    /// [`Prober::measure_from`] going on from what `resume` already played
    /// (the same board, with fewer windows or use rounds), and with `keep`
    /// what it played, for a fuller measure to go on from.
    fn measure_partial(
        &mut self,
        prelude: Prelude,
        parts: Parts,
        resume: Option<Partial>,
        keep: bool,
    ) -> Result<(Measure, Option<Partial>), Error> {
        let started = std::time::Instant::now();
        let all_steps: Vec<&KitStep> = self.kit.steps.iter().collect();
        if self.baseline.is_none() {
            self.baseline = Some(self.run(Prelude::EndAtOnce, &all_steps, Policy::Pass)?);
        }
        let base = self.baseline.clone().expect("baseline");
        let (all, done) = match resume {
            Some(p) => (p.all, p.responses),
            None => (self.run(prelude, &all_steps, Policy::Pass)?, Vec::new()),
        };
        let mut played = Vec::new();
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

        let floodgates = self.floodgates(&base, &all, &all_worked);
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
        let known: HashSet<ResponderId> = offered.iter().map(|(id, _, _)| *id).collect();
        let follow = Context { known: Some(&known), ..Context::default() };
        // What a step needs: a probe lost only because the one it needs was
        // stopped is not stopped again (negating Foolish Burial is not also
        // stopping Glow-Up Bulb's effect from the GY).
        let needs: HashMap<Probe, Probe> =
            self.kit.steps.iter().flat_map(|s| s.goals.iter().filter_map(move |(p, _)| Some((*p, s.needs?)))).collect();
        let stopped_by = |run: &RunTrace| -> Vec<Probe> {
            let now = worked(run);
            all_worked
                .iter()
                .filter(|(p, ok)| **ok && now.get(p) != Some(&true) && needs.get(p).is_none_or(|n| now.get(n) == Some(&true)))
                .map(|(p, _)| *p)
                .collect()
        };
        let mut interruptions = Vec::new();
        for (id, loc, windows) in &offered {
            // What it stops, tried at each window where it is offered (being
            // offered is not being able to stop that action: Effect Veiler is
            // offered while a monster is summoned, but cannot stop the summon).
            // A measure resumed starts from what the one before found.
            let before = done.iter().find(|r| r.id == *id).cloned().unwrap_or(Response::new(*id));
            let Response { mut covers, mut effect, mut targeted, .. } = before.clone();
            // The first window of every step before the later ones of a step:
            // the cap leaves out no step whole.
            let mut tried = windows.clone();
            tried.sort_by_key(|&(step, at)| (at, step));
            for &window in tried.iter().take(parts.windows).skip(before.windows) {
                let Ok(run) = self.run_with(prelude, &all_steps, Policy::UseAt(*id, window), follow) else { continue };
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
            let mut response = Response { windows: before.windows.max(parts.windows), covers, effect, targeted, ..before };
            if response.covers.is_empty() {
                if keep {
                    played.push(response);
                }
                continue;
            }
            response.covers.sort();
            response.covers.dedup();
            // How many times in a row: used whenever offered, keeping it for
            // later where it does nothing.
            while !response.settled && response.rounds < parts.use_rounds {
                response.rounds += 1;
                let Ok(run) = self.run(prelude, &all_steps, Policy::Use(*id, &response.skip)) else {
                    response.settled = true;
                    break;
                };
                response.uses = response.uses.max(run.uses.iter().filter(|u| u.effect.is_some()).count() as u32);
                let wasted: Vec<(usize, usize)> = run.uses.iter().filter(|u| u.effect.is_none()).map(|u| (u.step, u.window)).collect();
                if wasted.is_empty() || wasted.iter().all(|w| response.skip.contains(w)) {
                    response.settled = true;
                    break;
                }
                response.skip.extend(wasted);
            }
            let (covers, effect, targeted, uses) = (response.covers.clone(), response.effect, response.targeted, response.uses);
            if keep {
                played.push(response);
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
        // With the breakers, the same once more with the opponent answering
        // your first link it can (report only).
        let answer = parts.breakers && !self.kit.counters.is_empty();
        let mut countered = None;
        let mut seconds = MeasureSeconds { responses: started.elapsed().as_secs_f64(), ..MeasureSeconds::default() };
        let together_started = std::time::Instant::now();
        if parts.together && (interruptions.len() > 1 || answer && !interruptions.is_empty()) {
            let order: Vec<ResponderId> = interruptions.iter().map(|i| i.responder).collect();
            // Without what the responses bring: what they stop earlier would
            // leave the others nothing to stop, and they are not short of
            // anything they need.
            let mut worth: HashMap<u32, f64> = HashMap::new();
            for i in &interruptions {
                let w = worth.entry(i.responder.code).or_default();
                *w = w.max(i.alone);
            }
            let context = Context { worth: Some(&worth), ..Context::default() };
            if let Ok(run) = self.run_with(prelude, &all_steps, Policy::UseAll(&order), context) {
                if interruptions.len() > 1 {
                    for i in &mut interruptions {
                        let used = run.uses.iter().filter(|u| u.responder == i.responder).count() as u32;
                        if used < i.uses {
                            i.value = i.alone * used as f64 / i.uses as f64;
                        }
                    }
                }
                // Both with its answer in the opponent's hand: the cards in its
                // hand change what your random effects take from it.
                if answer {
                    let stopped = |run: &RunTrace| stopped_by(run).iter().map(|p| p.weight()).sum::<f64>();
                    let armed = self.kit.armed(&Arsenal { hand: self.kit.counters.clone(), extra: Vec::new() });
                    let targets: Vec<u32> =
                        interruptions.iter().filter(|i| i.location == location::MZONE).map(|i| i.responder.code).collect();
                    let context = Context { opponent: Some(&armed), ..context };
                    let all_stopped = self.run_with(prelude, &all_steps, Policy::UseAll(&order), context).map_or(0.0, |r| stopped(&r));
                    if all_stopped > 0.0 {
                        let context = Context { counter: Some(&targets), ..context };
                        if let Ok(answered) = self.run_with(prelude, &all_steps, Policy::UseAll(&order), context) {
                            countered = answered.countered.then(|| (stopped(&answered) / all_stopped).min(1.0));
                        }
                    }
                }
                interruptions.sort_by(|a, b| b.value.total_cmp(&a.value));
            }
        }

        // Board breakers, each in a run of its own.
        // Resilience is how the field holds: handtraps are out of the
        // breakers' reach and already counted on their own.
        let total = field_total(&interruptions);
        seconds.together = together_started.elapsed().as_secs_f64();
        let breakers_started = std::time::Instant::now();
        let breakers = if parts.breakers { self.breakers(prelude, &interruptions, &self.kit.breakers) } else { Vec::new() };
        seconds.breakers = breakers_started.elapsed().as_secs_f64();
        let follow_up_started = std::time::Instant::now();
        let weights: f64 = breakers.iter().map(|b| b.weight).sum();
        let resilience = if weights > 0.0 { breakers.iter().map(|b| b.weight * b.remaining).sum::<f64>() / weights } else { total };
        let follow_up = if parts.breakers { self.follow_up(prelude, &interruptions, &breakers) } else { None };
        seconds.follow_up = follow_up_started.elapsed().as_secs_f64();

        let hand_value = interruptions.iter().filter(|i| i.location == location::HAND).map(|i| i.value).sum::<f64>() + 0.0;
        let mut offered: Vec<(u32, u8)> = all.windows.iter().flat_map(|w| w.options.iter().map(|(id, loc)| (id.code, *loc))).collect();
        offered.sort_unstable();
        offered.dedup();
        let measure = Measure {
            field_value: total,
            hand_value,
            floodgate_value: floodgates.iter().map(|p| p.weight()).sum::<f64>() + 0.0,
            interruptions,
            floodgates,
            breakers,
            resilience,
            offered,
            follow_up,
            countered,
            attrition: attrition(&all.taken),
            taken: all.taken.clone(),
            seconds,
        };
        Ok((measure, keep.then_some(Partial { all, responses: played })))
    }

    /// Floodgates: what works on an empty board (`base`) but not with no
    /// answer on this one (`all`, whose probes `all_worked`). A step whose
    /// prerequisite failed is not stopped a second time (no Glow-Up Bulb in
    /// the GY, no effect from there).
    fn floodgates(&self, base: &RunTrace, all: &RunTrace, all_worked: &HashMap<Probe, bool>) -> Vec<Probe> {
        let mut floodgates = Vec::new();
        for (step, (run, base_run)) in self.kit.steps.iter().zip(all.steps.iter().zip(&base.steps)) {
            if step.needs.is_some_and(|p| all_worked.get(&p) != Some(&true)) {
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
        floodgates
    }

    /// A quick value of the board `prelude` reaches, in interruptions (B6):
    /// your responses used together whenever offered, each use that stops
    /// something counted by its effect, and the floodgates' weight. Two
    /// runs: with no answer, then with all of them.
    fn quick_value(&mut self, prelude: Prelude) -> Result<f64, Error> {
        let all_steps: Vec<&KitStep> = self.kit.steps.iter().collect();
        if self.baseline.is_none() {
            self.baseline = Some(self.run(Prelude::EndAtOnce, &all_steps, Policy::Pass)?);
        }
        let base = self.baseline.clone().expect("baseline");
        let all = self.run(prelude, &all_steps, Policy::Pass)?;
        let mut all_worked = HashMap::new();
        for (step, run) in self.kit.steps.iter().zip(&all.steps) {
            for ((probe, _), ok) in step.goals.iter().zip(&run.worked) {
                all_worked.insert(*probe, run.available && *ok);
            }
        }
        let floodgates: f64 = self.floodgates(&base, &all, &all_worked).iter().map(|p| p.weight()).sum();
        let mut order: Vec<ResponderId> = Vec::new();
        for w in &all.windows {
            for &(id, _) in &w.options {
                if !order.contains(&id) {
                    order.push(id);
                }
            }
        }
        if order.is_empty() {
            return Ok(floodgates);
        }
        let known: HashSet<ResponderId> = order.iter().copied().collect();
        let context = Context { known: Some(&known), ..Context::default() };
        let run = self.run_with(prelude, &all_steps, Policy::UseAll(&order), context)?;
        Ok(run.uses.iter().filter_map(|u| u.effect).map(Effect::value).sum::<f64>() + floodgates)
    }

    /// What the handtraps take away from `line` (B6, V1): for each kind of
    /// choke point, the line is cut at each of its `points` and the turn
    /// ends there; the opponent picks the one that leaves you least
    /// ([`Prober::quick_value`]). Maxx "C" and the Mulcharmy give the
    /// opponent a card for each Special Summon after the first. Each loss is
    /// weighed by the chance the opponent holds that kind of handtrap. V1 is
    /// hopeful: the effect stopped does not spend its card.
    pub fn expected_loss(
        &mut self,
        line: &Line,
        points: &[ChokePoint],
        special_summons: u32,
        rates: &HandTrapRates,
    ) -> Result<ExpectedLoss, Error> {
        let full = self.quick_value(Prelude::Line(line))?;
        let mut cuts: HashMap<usize, Option<f64>> = HashMap::new();
        let mut out = ExpectedLoss::default();
        for kind in [ChokeKind::DeckTouch, ChokeKind::FieldEffect, ChokeKind::FifthSummon, ChokeKind::SecondSearch] {
            let mut worst: Option<f64> = None;
            for p in points.iter().filter(|p| p.kind == kind) {
                let value = match cuts.get(&p.answer) {
                    Some(&v) => v,
                    None => {
                        // A cut that cannot be played tells nothing.
                        let v = self.quick_value(Prelude::Cut(line, p.answer)).ok();
                        cuts.insert(p.answer, v);
                        v
                    }
                };
                if let Some(v) = value {
                    worst = Some(worst.map_or(v, |w: f64| w.min(v)));
                }
            }
            if let Some(worst) = worst {
                let loss = (full - worst).max(0.0);
                out.losses.push((kind, loss));
                out.expected += rates.chance(kind) * loss;
            }
        }
        out.maxx = MAXX_PER_SUMMON * special_summons.saturating_sub(1) as f64;
        out.expected += rates.special_summons * out.maxx;
        out.cuts = cuts.len();
        Ok(out)
    }

    /// The board against the moves of a kit made with [`Kit::with_moves`],
    /// each weighing `weights[i]`. A move is made when it is played and not
    /// negated (a summon: when the monster lands). A response is worth the
    /// share of the moves' weight it takes away at its best moment, or used
    /// whenever offered: the moves it makes fail and those lost with them
    /// (a monster summoned, then its effect on the field). It shows the
    /// moves it stops itself: once one failed, the moves after it count only
    /// if they were tried and failed too (stopping the first move is not
    /// stopping the others, D2).
    pub(crate) fn against_moves(&self, prelude: Prelude, weights: &[f64]) -> Result<MovesMeasure, Error> {
        let steps: Vec<&KitStep> = self.kit.steps.iter().collect();
        let made = |t: &RunTrace, i: usize| {
            let run = &t.steps[i];
            run.available && !run.negated && run.worked.iter().all(|w| *w)
        };
        let failed = |reference: &RunTrace, t: &RunTrace| -> Vec<usize> {
            let mut out = Vec::new();
            let mut clean = true;
            for i in 0..steps.len() {
                if made(reference, i) && !made(t, i) {
                    if clean || t.steps[i].available {
                        out.push(i);
                    }
                    clean = false;
                }
            }
            out
        };
        let weight = |moves: &[usize]| moves.iter().map(|&i| weights.get(i).copied().unwrap_or(0.0)).sum::<f64>() + 0.0;
        let made_weight = |t: &RunTrace| weight(&(0..steps.len()).filter(|&i| made(t, i)).collect::<Vec<_>>());
        let base = self.run(Prelude::EndAtOnce, &steps, Policy::Pass)?;
        let all = self.run(prelude, &steps, Policy::Pass)?;
        let possible: Vec<usize> = (0..steps.len()).filter(|&i| made(&base, i)).collect();
        let floodgated = failed(&base, &all);
        let total = weight(&possible);
        // What your board lets the opponent make with no answer.
        let reference = made_weight(&all);

        // Your effects offered with no answer, and where (as in the measure).
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
        let known: HashSet<ResponderId> = offered.iter().map(|(id, _, _)| *id).collect();
        let follow = Context { known: Some(&known), ..Context::default() };
        let none = HashSet::new();
        let mut interruptions = Vec::new();
        for (id, loc, windows) in &offered {
            // Its best moment, or every moment it is offered.
            let policies = windows.iter().take(12).map(|&w| Policy::UseAt(*id, w)).chain([Policy::Use(*id, &none)]);
            let mut best: Option<(f64, RunTrace)> = None;
            for policy in policies {
                let Ok(run) = self.run_with(prelude, &steps, policy, follow) else { continue };
                let taken = reference - made_weight(&run);
                if taken > 1e-9 && best.as_ref().is_none_or(|(b, _)| taken > *b + 1e-9) {
                    best = Some((taken, run));
                }
            }
            let Some((taken, run)) = best else { continue };
            let effect = run.uses.iter().filter_map(|u| u.effect).min().unwrap_or(Effect::Lock);
            let targeted = run.uses.iter().any(|u| u.targeted);
            let coverage = if reference > 0.0 { (taken / reference).min(1.0) } else { 0.0 };
            let value = effect.value() * if targeted { 0.85 } else { 1.0 } * coverage;
            let stops = failed(&all, &run);
            interruptions.push(MoveInterruption { responder: *id, location: *loc, effect, targeted, stops, coverage, value });
        }
        interruptions.sort_by(|a, b| b.value.total_cmp(&a.value));
        // All of them together, the strongest first: what is left of the
        // opponent's moves (two answers that stop the same move count once).
        let order: Vec<ResponderId> = interruptions.iter().map(|i| i.responder).collect();
        let left = match self.run_with(prelude, &steps, Policy::UseAll(&order), follow) {
            Ok(run) if reference > 0.0 && !order.is_empty() => (made_weight(&run) / reference).min(1.0),
            _ => 1.0,
        };
        Ok(MovesMeasure {
            left,
            possible: (0..steps.len()).map(|i| made(&base, i)).collect(),
            here: (0..steps.len()).map(|i| made(&all, i)).collect(),
            floodgate_share: if total > 0.0 { (total - reference).max(0.0) / total } else { 0.0 },
            floodgated,
            value: interruptions.iter().map(|i| i.value).sum::<f64>() + 0.0,
            interruptions,
        })
    }

    /// Each of `breakers` in a run of its own after the board: what is left
    /// of the field's `interruptions`, or which of them negates it.
    pub(crate) fn breakers(&self, prelude: Prelude, interruptions: &[Interruption], breakers: &[Breaker]) -> Vec<BreakerResult> {
        let total = field_total(interruptions);
        let mut out = Vec::new();
        // The breakers go for the monster worth the most first (the attack,
        // the monster a Kaiju takes).
        let targets: Vec<u32> = interruptions.iter().filter(|i| i.location == location::MZONE).map(|i| i.responder.code).collect();
        for breaker in breakers {
            let mut owned = breaker.steps.clone();
            for s in &mut owned {
                s.picks = targets.clone();
            }
            let steps: Vec<&KitStep> = owned.iter().collect();
            let armed = self.kit.armed(&breaker.arsenal);
            let context = Context { opponent: Some(&armed), ..Context::default() };
            let run = match self.run_with(prelude, &steps, Policy::Pass, context) {
                Ok(run) if run.steps.iter().zip(&owned).all(|(r, s)| r.available || s.optional) => run,
                _ => {
                    out.push(BreakerResult {
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
                remaining: self.left(&run.state, interruptions),
            };
            // A negation offered against it: try it (it spends one use).
            let negators: Vec<&Interruption> = interruptions
                .iter()
                .filter(|i| i.effect.negates() && run.windows.iter().any(|w| w.options.iter().any(|(o, _)| *o == i.responder)))
                .collect();
            for negator in negators {
                let none = HashSet::new();
                let Ok(tried) = self.run_with(prelude, &steps, Policy::Use(negator.responder, &none), context) else { continue };
                if tried.steps.iter().any(|s| s.negated) {
                    let spent = if negator.location == location::HAND { 0.0 } else { negator.value / negator.uses as f64 };
                    let left = self.left(&tried.state, interruptions) - spent;
                    if left > result.remaining {
                        result.remaining = left;
                        result.stopped_by = Some(negator.responder);
                    }
                    break;
                }
            }
            out.push(result);
        }
        out
    }

    /// Each card of your board that interrupts, against each of the kit's
    /// removals aimed at it after the board `line` reaches (a run each).
    /// The card answers with its own effects only: what it does for itself,
    /// with the lasting protections of the board (Stardust Sifr's).
    pub fn protection(&self, line: &Line, interruptions: &[Interruption]) -> Vec<Protection> {
        self.protection_from(Prelude::Line(line), interruptions)
    }

    /// [`Prober::protection`] of a board put straight on the field.
    pub fn protection_placed(&self, placed: &[Placement], interruptions: &[Interruption]) -> Vec<Protection> {
        self.protection_from(Prelude::Placed(placed), interruptions)
    }

    fn protection_from(&self, prelude: Prelude, interruptions: &[Interruption]) -> Vec<Protection> {
        let opponent = self.kit.armed(&Arsenal { hand: self.kit.removals.iter().map(|r| r.2).collect(), extra: Vec::new() });
        let context = Context { opponent: Some(&opponent), ..Context::default() };
        let mut out: Vec<Protection> = Vec::new();
        for i in interruptions.iter().filter(|i| matches!(i.location, location::MZONE | location::SZONE) && i.value > 0.0) {
            if out.iter().any(|p| p.responder.code == i.responder.code) {
                continue;
            }
            let own: Vec<ResponderId> =
                interruptions.iter().filter(|o| o.responder.code == i.responder.code).map(|o| o.responder).collect();
            let mut protection = Protection { responder: i.responder, location: i.location, holds: Vec::new(), falls: Vec::new() };
            for &(removal, _, code) in self.kit.removals.iter().filter(|(_, at, _)| *at == i.location) {
                let mut step = KitStep::aiming(vec![i.responder.code]);
                step.act = Act::Activate(code, location::HAND);
                let Ok(run) = self.run_with(prelude, &[&step], Policy::UseAll(&own), context) else { continue };
                // Not played: it had nothing to aim at, this card included.
                let hit = run.steps[0].available && self.left(&run.state, std::slice::from_ref(i)) <= 0.0;
                if hit {
                    protection.falls.push(removal);
                } else {
                    protection.holds.push(removal);
                }
            }
            out.push(protection);
        }
        out
    }

    /// Interruption value of the field still usable in `state`: each
    /// responder's card is still where it was (face-up and not negated on
    /// the field). Handtraps are not counted (see the resilience).
    fn left(&self, state: &DuelState, interruptions: &[Interruption]) -> f64 {
        let db = self.engine.cards();
        let me = &state.players[0];
        let same = |c: u32, code: u32| db.canonical_code(c) == code;
        interruptions
            .iter()
            .filter(|i| {
                let code = i.responder.code;
                match i.location {
                    location::HAND => false,
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

    /// Your next turn after the board breaker that took away the most
    /// (`results` are the breakers' runs, in the kit's order), or after an
    /// opponent who only ends its turn if none could be played.
    fn follow_up(&self, prelude: Prelude, interruptions: &[Interruption], results: &[BreakerResult]) -> Option<FollowUp> {
        let worst = self
            .kit
            .breakers
            .iter()
            .zip(results)
            .filter(|(_, r)| !r.not_applicable)
            .min_by(|(_, a), (_, b)| a.remaining.total_cmp(&b.remaining))
            .map(|(b, _)| b);
        let targets: Vec<u32> = interruptions.iter().filter(|i| i.location == location::MZONE).map(|i| i.responder.code).collect();
        let mut owned = worst.map(|b| b.steps.clone()).unwrap_or_default();
        for s in &mut owned {
            s.picks = targets.clone();
        }
        let steps: Vec<&KitStep> = owned.iter().collect();
        let armed = self.kit.armed(worst.map_or(&Arsenal::default(), |b| &b.arsenal));
        let context = Context { turn_three: true, opponent: Some(&armed), ..Context::default() };
        self.run_with(prelude, &steps, Policy::Pass, context).ok()?.follow_up
    }

    /// From the opponent's End Phase to your next Main Phase: you use your
    /// triggers, not your quick effects (as in your own End Phase), and the
    /// opponent only passes. `idle_effects` are those you could activate at
    /// the end of your first turn.
    fn turn_three(&self, duel: &mut Duel, idle_effects: &HashSet<(u32, u64)>) -> Result<Option<FollowUp>, Error> {
        const DRAW: u32 = 0x200_0000;
        let mut gained = 0;
        for _ in 0..2000 {
            if self.out_of_time() {
                return Err(Error::Engine("the probe turn ran out of time".to_owned()));
            }
            let (decision, events) = match duel.advance()? {
                Step::Decision { decision, events } => (decision, events),
                Step::End { .. } => return Ok(None),
            };
            for e in &events {
                if let Event::Move { from, to, reason, .. } = e {
                    let added = to.location == location::HAND && from.location != location::HAND && reason & DRAW == 0;
                    let summoned = to.location == location::MZONE && from.location != location::MZONE;
                    gained += usize::from(to.controller == 0 && (added || summoned));
                }
            }
            let actions = duel.actions(&decision);
            match decision {
                Decision::IdleCmd { player: 0, activatable, special_summonable, .. } => {
                    // What is still on the field is the resilience's: here,
                    // what comes back from the GY and banishment.
                    let away = |c: &CardRef| matches!(c.location, location::GRAVE | location::REMOVED);
                    let mut effects: HashSet<(u32, u64)> =
                        activatable.iter().filter(|a| away(&a.card)).map(|a| (a.card.code, a.description)).collect();
                    effects.extend(special_summonable.iter().filter(|c| away(c)).map(|c| (c.code, 0)));
                    return Ok(Some(FollowUp { rebuild: effects.len(), gained }));
                }
                _ if decision.player() == Some(1) => duel.respond(&end_turn(&decision, &actions)),
                _ => duel.respond(&end_phase_answer(&decision, &actions, idle_effects)),
            }
        }
        Ok(None)
    }

    /// Plays your turn, then the opponent's steps, answering as `policy` says.
    fn run(&self, prelude: Prelude, steps: &[&KitStep], policy: Policy) -> Result<RunTrace, Error> {
        self.run_with(prelude, steps, policy, Context::default())
    }

    /// [`Prober::run`], knowing more (see [`Context`]).
    fn run_with(&self, prelude: Prelude, steps: &[&KitStep], policy: Policy, context: Context) -> Result<RunTrace, Error> {
        let engine = self.engine;
        let opponent = context.opponent.unwrap_or(&self.kit.opponent);
        let mut duel = match prelude {
            Prelude::Placed(placed) => Duel::goldfish_vs_placed(engine, self.deck, self.hand, self.seed, opponent, placed)?,
            _ => Duel::goldfish_vs(engine, self.deck, self.hand, self.seed, opponent)?,
        };
        // Your line was found against the kit's opponent: against another one
        // (other cards in its Extra Deck), the opponent's cards it picks are
        // picked again by what they are.
        let repick = match (prelude, context.opponent) {
            (Prelude::Line(line), Some(other)) if *other != self.kit.opponent => {
                line_picks(engine, self.deck, self.hand, self.seed, &self.kit.opponent, line).ok()
            }
            _ => None,
        };
        let mut answered = 0usize;
        let mut own = match prelude {
            Prelude::Own(script) => Some(Follower::new(script)),
            _ => None,
        };
        const NO_ANSWERS: &[Vec<u8>] = &[];
        let mut answers = match prelude {
            Prelude::Line(line) => line.answers.iter(),
            Prelude::Cut(line, at) => line.answers[..at.min(line.answers.len())].iter(),
            Prelude::Own(_) => NO_ANSWERS.iter(),
            Prelude::EndAtOnce | Prelude::Placed(_) => NO_ANSWERS.iter(),
        };
        let mut trace = RunTrace {
            steps: vec![StepRun::default(); steps.len()],
            windows: Vec::new(),
            uses: Vec::new(),
            state: DuelState::default(),
            follow_up: None,
            countered: false,
            taken: Vec::new(),
        };
        let mut observer = Observer::default();
        let mut next = 0usize;
        let mut current: Option<(usize, Piles)> = None;
        let mut windows_in_step = 0usize;
        // The step being played went to the Battle Phase already.
        let mut battled = false;
        // Your last use in the step being played: the triggers of your cards
        // that follow it are part of it (a Labrynth Trap resolving, then
        // Lady Labrynth destroying a card).
        let mut last_use: Option<usize> = None;
        // The opponent got back to choose after its last step.
        let mut complete = false;
        // The opponent's line played again, when the kit has one (all its
        // steps are played: they are the line's moves).
        let script = self.kit.script.as_deref().filter(|_| steps.len() == self.kit.steps.len());
        let mut follower = script.map(|l| Follower::new(&l.script));
        // The opponent's line went on to its End Phase (its moves there
        // are its line's too): the run is over when your next turn begins.
        let mut ending = false;
        // The last link of the chain being built is yours: your effects
        // answer the opponent's, never your own (Invoked Mechaba would
        // negate your own Effect Veiler).
        let mut mine_on_top = false;
        // The card of your last link, and the targets of the opponent's
        // answer to it while it picks them.
        let mut your_link: Option<u32> = None;
        let mut aiming: Option<KitStep> = None;
        // The effects you could activate at your last free moment of the
        // Main Phase: in your End Phase, those are quick effects, the
        // others are the End Phase's triggers.
        let mut idle_effects: HashSet<(u32, u64)> = HashSet::new();
        let verbose = std::env::var("DUELFORGE_DEBUG_PROBE").is_ok_and(|v| v == "2");
        let mut shown = false;
        let label =
            |actions: &[Action], answer: &[u8]| actions.iter().find(|a| a.response == answer).map_or("?".to_owned(), |a| a.label.clone());
        for _ in 0..4000 {
            if self.out_of_time() {
                return Err(Error::Engine("the probe turn ran out of time".to_owned()));
            }
            let (decision, events) = match duel.advance()? {
                Step::Decision { decision, events } => (decision, events),
                Step::End { .. } => break,
            };
            let your_turn = duel.turn_info().0 <= 1;
            for e in &events {
                if let (true, Event::Move { code, from, to, .. }) = (your_turn, e) {
                    // On your turn the opponent only passes: what leaves its
                    // Extra Deck, Deck, hand or field for its GY, banishment
                    // or your side is your doing.
                    let kept =
                        matches!(from.location, location::EXTRA | location::DECK | location::HAND | location::MZONE | location::SZONE);
                    let away = matches!(to.location, location::GRAVE | location::REMOVED) || to.controller == 0;
                    if from.controller == 1 && kept && away && (from.location, from.controller) != (to.location, to.controller) {
                        trace.taken.push((*code, from.location));
                    }
                }
                observer.see(engine, e, &mut trace.uses);
                match e {
                    Event::Chaining { card, .. } => {
                        mine_on_top = card.controller == 0;
                        if mine_on_top {
                            your_link = Some(card.code);
                        } else {
                            aiming = None;
                        }
                    }
                    Event::ChainEnd => mine_on_top = false,
                    _ => {}
                }
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
            if ending && turn > 2 {
                if let Some((j, before)) = current.take() {
                    self.finish(&mut trace.steps[j], steps[j], &before, &Piles::of(&duel), &observer);
                }
                complete = true;
                break;
            }
            if let Decision::IdleCmd { player: 0, activatable, .. } = &decision {
                idle_effects = activatable.iter().map(|a| (a.card.code, a.description)).collect();
            }
            // The answers of your turn's line need no list of actions.
            if turn <= 1 && matches!(prelude, Prelude::Line(_) | Prelude::Cut(..)) {
                if let Some(a) = answers.next() {
                    let picks = repick.as_ref().and_then(|r| r.get(answered)?.as_deref());
                    answered += 1;
                    match picks.and_then(|p| pick_again(engine, &decision, p)) {
                        Some(again) => duel.respond(&again),
                        None => duel.respond(a),
                    }
                    continue;
                }
            }
            let actions = duel.actions(&decision);
            // Your turn: the line's answers; the opponent passes.
            if turn <= 1 {
                // The line has every answer of your turn, the opponent's passes
                // included. A provisional board's line stops at "end the turn":
                // its End Phase is played like the search would (doing something first).
                match prelude {
                    Prelude::Line(_) if decision.player() == Some(1) => duel.respond(&passive_answer(&decision, &actions)),
                    Prelude::Line(_) => duel.respond(&end_phase_answer(&decision, &actions, &idle_effects)),
                    Prelude::Cut(..) if decision.player() == Some(1) => duel.respond(&passive_answer(&decision, &actions)),
                    Prelude::Cut(..) if matches!(decision, Decision::IdleCmd { .. } | Decision::BattleCmd { .. }) => {
                        duel.respond(&end_turn(&decision, &actions))
                    }
                    Prelude::Cut(..) => duel.respond(&end_phase_answer(&decision, &actions, &idle_effects)),
                    Prelude::EndAtOnce | Prelude::Placed(_) | Prelude::Own(_) if decision.player() == Some(1) => {
                        duel.respond(&passive_answer(&decision, &actions))
                    }
                    Prelude::Own(_) => match own.as_mut().and_then(|f| f.answer(engine, &decision, &actions, false)) {
                        Some(followed) => duel.respond(&followed.answer),
                        // The line is over at a free moment: the turn ends.
                        None if matches!(decision, Decision::IdleCmd { .. }) => duel.respond(&end_turn(&decision, &actions)),
                        None => duel.respond(&end_phase_answer(&decision, &actions, &idle_effects)),
                    },
                    Prelude::EndAtOnce | Prelude::Placed(_) => duel.respond(&end_turn(&decision, &actions)),
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
                Some(1) if follower.is_some() => {
                    let (Some(follower), Some(script)) = (follower.as_mut(), script) else { break };
                    let idle = matches!(decision, Decision::IdleCmd { .. });
                    match follower.answer(engine, &decision, &actions, mine_on_top).filter(|f| !idle || f.at.is_some()) {
                        Some(followed) => {
                            // One of its moves starts: the one before it is over.
                            if let Some(i) = followed.at.and_then(|k| script.step_of.get(k).copied().flatten()) {
                                if let Some((j, before)) = current.take() {
                                    self.finish(&mut trace.steps[j], steps[j], &before, &Piles::of(&duel), &observer);
                                }
                                trace.steps[i].available = true;
                                current = Some((i, Piles::of(&duel)));
                                observer.new_step();
                                windows_in_step = 0;
                                last_use = None;
                            }
                            if verbose {
                                eprintln!("     opponent's line: {}", label(&actions, &followed.answer));
                            }
                            duel.respond(&followed.answer);
                        }
                        // Its Main Phase is over (the Battle Phase is not
                        // measured): its End Phase is played too, the line's
                        // moves there (Titaniklad from the GY) are its moves.
                        None if idle && !context.turn_three => {
                            ending = true;
                            follower.end_main_phase();
                            if verbose {
                                eprintln!("     opponent's line: to the End Phase");
                            }
                            duel.respond(&end_turn(&decision, &actions));
                        }
                        // Its line is over: nothing left to play, or it ends
                        // its Main Phase (the Battle Phase is not measured).
                        None => {
                            if let Some((j, before)) = current.take() {
                                self.finish(&mut trace.steps[j], steps[j], &before, &Piles::of(&duel), &observer);
                            }
                            if context.turn_three {
                                duel.respond(&end_turn(&decision, &actions));
                                trace.follow_up = self.turn_three(&mut duel, &idle_effects)?;
                            }
                            complete = true;
                            break;
                        }
                    }
                }
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
                            let after = Piles::of(&duel);
                            if verbose {
                                let db = engine.cards();
                                let names = |v: &[u32]| v.iter().map(|&c| db.name(c)).collect::<Vec<_>>().join(", ");
                                let after = duel.state();
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
                                current = Some((i, Piles::of(&duel)));
                                observer.new_step();
                                windows_in_step = 0;
                                battled = false;
                                last_use = None;
                                duel.respond(&answer);
                            }
                            None => {
                                if context.turn_three {
                                    duel.respond(&end_turn(&decision, &actions));
                                    trace.follow_up = self.turn_three(&mut duel, &idle_effects)?;
                                }
                                complete = true;
                                break;
                            }
                        }
                    } else {
                        // Its answer to your first link it can answer, once.
                        if let (Some(others), false, true, Decision::SelectChain { forced: false, .. }) =
                            (context.counter, trace.countered, mine_on_top, &decision)
                        {
                            let db = engine.cards();
                            let counter = actions.iter().find(|a| {
                                a.kind == ActionKind::Activate
                                    && a.card.is_some_and(|c| {
                                        self.kit.counters.iter().any(|&k| db.canonical_code(k) == db.canonical_code(c.code))
                                    })
                            });
                            if let Some(a) = counter {
                                trace.countered = true;
                                observer.answering = true;
                                aiming = Some(KitStep::aiming(your_link.into_iter().chain(others.iter().copied()).collect()));
                                duel.respond(&a.response);
                                continue;
                            }
                        }
                        let step = aiming.as_ref().or(current.as_ref().map(|(i, _)| steps[*i]));
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
                    // A chance to answer your own link is not a window at all.
                    let to_yourself = mine_on_top && matches!(decision, Decision::SelectChain { triggers: false, .. });
                    // Before the opponent's first step (its Draw and Standby
                    // Phases) the windows belong to a step of their own after
                    // the others, `steps.len()`: effects that can only be used
                    // then (setting a Trap, a Quick Summon) are tried there.
                    let step = current.as_ref().map_or(steps.len(), |(i, _)| *i);
                    let before = step == steps.len();
                    match window {
                        Some(_) if to_yourself => duel.respond(&window_answer(&decision, None)),
                        Some(options) => {
                            let at = windows_in_step;
                            windows_in_step += 1;
                            let pick = match policy {
                                Policy::Use(id, skip) if !skip.contains(&(step, at)) => options.iter().position(|(o, _)| *o == id),
                                // All together: nothing to stop before the first step.
                                Policy::UseAll(ids) if !before => ids.iter().find_map(|id| options.iter().position(|(o, _)| o == id)),
                                Policy::UseAt(id, window) if window == (step, at) => options.iter().position(|(o, _)| *o == id),
                                _ => None,
                            };
                            let pick = pick.or_else(|| {
                                context
                                    .known
                                    .filter(|_| !trace.uses.is_empty())
                                    .and_then(|known| options.iter().position(|(o, _)| !known.contains(o)))
                            });
                            trace.windows.push(Window { step, options: options.clone() });
                            let follow_up = matches!(decision, Decision::SelectChain { triggers: true, .. } | Decision::EffectYesNo { .. });
                            match (pick, last_use) {
                                (Some(i), _) => {
                                    trace.uses.push(Use { responder: options[i].0, step, window: at, effect: None, targeted: false });
                                    last_use = Some(trace.uses.len() - 1);
                                    observer.waiting = last_use;
                                    duel.respond(&window_answer(&decision, Some(i)));
                                }
                                (None, Some(u)) if follow_up => {
                                    observer.waiting = Some(u);
                                    duel.respond(&window_answer(&decision, Some(0)));
                                }
                                (None, _) => duel.respond(&window_answer(&decision, None)),
                            }
                        }
                        None => {
                            let db = engine.cards();
                            // Your cards seen answering in this run, or the ones it uses.
                            let used: &[ResponderId] = match &policy {
                                Policy::Use(id, _) | Policy::UseAt(id, _) => std::slice::from_ref(id),
                                Policy::UseAll(ids) => ids,
                                Policy::Pass => &[],
                            };
                            let answers = |a: &Action| {
                                a.card.is_some_and(|c| {
                                    let code = db.canonical_code(c.code);
                                    c.controller == 0
                                        && (used.iter().any(|r| r.code == code)
                                            || trace.windows.iter().any(|w| w.options.iter().any(|(r, _)| r.code == code)))
                                })
                            };
                            let power = |a: &Action| {
                                let worth = a.card.and_then(|c| context.worth?.get(&db.canonical_code(c.code)).copied()).unwrap_or(0.0);
                                let (level, attack) = a.card.and_then(|c| db.get(c.code)).map_or((0, 0), |c| (c.level, c.attack.max(0)));
                                (worth, level, attack)
                            };
                            duel.respond(&your_answer(&decision, &actions, |a| observer.aim(db, a), answers, power))
                        }
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

    /// Whether each goal of a finished step was met. A monster wanted on the
    /// field counts once it landed: removed after that, the step worked
    /// (what is lost is the steps that needed it).
    fn finish(&self, run: &mut StepRun, step: &KitStep, before: &Piles, after: &Piles, observer: &Observer) {
        let db = self.engine.cards();
        let count = |p: &Piles, want: &Want| -> i32 {
            let matches = |c: u32| want.code.is_none_or(|w| db.canonical_code(c) == db.canonical_code(w));
            (match want.location {
                location::HAND => p.hand.iter().filter(|&&c| matches(c)).count(),
                location::GRAVE => p.grave.iter().filter(|&&c| matches(c)).count(),
                _ => 0,
            }) as i32
        };
        let landed = |want: &Want| -> i32 {
            observer.landed.iter().filter(|&&c| want.code.is_none_or(|w| c == db.canonical_code(w))).count() as i32
        };
        run.worked = step
            .goals
            .iter()
            .map(|(_, want)| match want.location {
                location::MZONE => landed(want) >= want.delta,
                RESOLVED => !observer.step_negated,
                _ => count(after, want) - count(before, want) >= want.delta,
            })
            .collect();
        run.negated = observer.step_negated;
    }
}

/// Interruption value of the field (not the hand: handtraps are out of
/// the breakers' reach).
fn field_total(interruptions: &[Interruption]) -> f64 {
    interruptions.iter().filter(|i| i.location != location::HAND).map(|i| i.value).sum::<f64>() + 0.0
}

/// Your End Phase after a provisional line: optional triggers are used,
/// lists are picked forward, quick effects are not (activating Hyperinvoked
/// Aeon there, then negating it with your own Invoked Mechaba, would undo
/// the board). The search plays the End Phase the same way first. The
/// engine can offer an End Phase trigger and quick effects in the same
/// window: the quick effects are those already offered at your last free
/// moment of the Main Phase (`idle_effects`).
fn end_phase_answer(decision: &Decision, actions: &[Action], idle_effects: &HashSet<(u32, u64)>) -> Vec<u8> {
    if let Some(answer) = forward(decision, actions, |_| true) {
        return answer;
    }
    if let Decision::SelectChain { triggers: false, forced: false, .. } = decision {
        let trigger = actions
            .iter()
            .find(|a| a.kind == ActionKind::Activate && a.card.is_some_and(|c| !idle_effects.contains(&(c.code, a.description))));
        return trigger.map_or_else(|| window_answer(decision, None), |a| a.response.clone());
    }
    let act = actions.iter().find(|a| !matches!(a.kind, ActionKind::Pass | ActionKind::No | ActionKind::Cancel | ActionKind::Finish));
    act.or(actions.first()).map(|a| a.response.clone()).unwrap_or_else(|| window_answer(decision, None))
}

/// Ends your turn at once (the empty board).
pub(crate) fn end_turn(decision: &Decision, actions: &[Action]) -> Vec<u8> {
    match actions.iter().find(|a| a.kind == ActionKind::EndTurn) {
        Some(a) => a.response.clone(),
        None => passive_answer(decision, actions),
    }
}

/// Your answer to a chance to respond: pass, or the effect at `index`.
pub(crate) fn window_answer(decision: &Decision, index: Option<usize>) -> Vec<u8> {
    match decision {
        Decision::EffectYesNo { .. } | Decision::YesNo { .. } => (index.is_some() as i32).to_le_bytes().to_vec(),
        _ => index.map_or(-1, |i| i as i32).to_le_bytes().to_vec(),
    }
}

/// In a list where cards are picked one at a time, picking goes forward:
/// finish when allowed, never take a card back (that would go round in circles).
pub(crate) fn forward(decision: &Decision, actions: &[Action], prefer: impl Fn(&Action) -> bool) -> Option<Vec<u8>> {
    forward_by(decision, actions, |a| !prefer(a))
}

/// [`forward`], picking the card with the least `key` (the first of them).
fn forward_by<K: Ord>(decision: &Decision, actions: &[Action], key: impl Fn(&Action) -> K) -> Option<Vec<u8>> {
    let Decision::SelectUnselectCard { finishable, selectable, .. } = decision else { return None };
    if *finishable {
        if let Some(a) = actions.iter().find(|a| a.kind == ActionKind::Finish) {
            return Some(a.response.clone());
        }
    }
    // Picks of the selectable cards come first in the list of answers.
    let picks = &actions[..selectable.len().min(actions.len())];
    picks.iter().min_by_key(|a| key(a)).map(|a| a.response.clone())
}

/// Your other decisions while answering: aim at the opponent's cards, the
/// one it is using first (`aim`, 0 best, 3 for none of its cards). Your own
/// cards to pick (a cost, a tribute, a card to return) are those that
/// answer nothing first (`answers`), then the weakest (`power`: the worth
/// of a responder, then Level and ATK), then by code: the same board pays
/// the same wherever its cards are, and keeps its handtraps when it can.
/// Monsters to Special Summon are the strongest first: a response that
/// summons brings its best card.
fn your_answer(
    decision: &Decision,
    actions: &[Action],
    aim: impl Fn(&Action) -> u8,
    answers: impl Fn(&Action) -> bool,
    power: impl Fn(&Action) -> (f64, u32, i32),
) -> Vec<u8> {
    let summoning =
        matches!(decision, Decision::SelectCard { hint, .. } | Decision::SelectUnselectCard { hint, .. } if *hint == hint::MSG_SPSUMMON);
    let key = |a: &Action| {
        let (worth, level, attack) = power(a);
        let (worth, level, attack) = ((worth * 1000.0) as i64, i64::from(level), i64::from(attack));
        let order = if summoning { (0, -level, -attack) } else { (worth, level, attack) };
        (aim(a), answers(a), order, a.card.map_or(u32::MAX, |c| c.code))
    };
    if let Some(answer) = forward_by(decision, actions, key) {
        return answer;
    }
    let theirs = actions.iter().filter(|a| a.kind != ActionKind::Cancel && aim(a) < 3).min_by_key(|a| aim(a));
    let forced = matches!(decision, Decision::SelectChain { forced: true, .. });
    let pick = theirs
        .or_else(|| if forced { actions.iter().find(|a| a.kind == ActionKind::Activate) } else { None })
        .or_else(|| actions.iter().find(|a| a.kind == ActionKind::Yes))
        .or_else(|| actions.iter().filter(|a| a.kind == ActionKind::Choose).min_by_key(|a| key(a)))
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
    // The picks in their order (a breaker's targets: the card worth the
    // most first), then any other card.
    let rank = |a: &Action| {
        a.card.and_then(|c| picks.iter().position(|&p| db.canonical_code(p) == db.canonical_code(c.code))).unwrap_or(picks.len())
    };
    if let Some(answer) = forward_by(decision, actions, rank) {
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
        // Outer Main Monster Zones first: the ones under the Extra Monster
        // Zones stay free for what a Link arrow (Link Spider's) summons there.
        // A monster that may go to an Extra Monster Zone comes from the Extra
        // Deck: it goes there, where its Link arrows point to your zones
        // (a Main Monster Zone it is offered because your Link Monster
        // points to it would leave its arrows pointing at nothing).
        Decision::SelectPlace { .. } => {
            let extra = actions.iter().any(|a| matches!(*a.response.as_slice(), [_, location::MZONE, seq] if seq >= 5));
            let order: &[u8] = if extra { &[5, 6, 0, 4, 2, 1, 3] } else { &[0, 4, 2, 1, 3, 5, 6] };
            let rank = |a: &Action| match *a.response.as_slice() {
                [_, location::MZONE, seq] => order.iter().position(|&s| s == seq).unwrap_or(7),
                _ => 0,
            };
            actions.iter().min_by_key(|a| rank(a))
        }
        Decision::EffectYesNo { .. } | Decision::YesNo { .. } => first(&[ActionKind::Yes]),
        _ => {
            let picked = actions.iter().filter(|a| rank(a) < picks.len()).min_by_key(|a| rank(a));
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
pub(crate) fn find_act(engine: &Engine, actions: &[Action], act: Act) -> Option<Vec<u8>> {
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
