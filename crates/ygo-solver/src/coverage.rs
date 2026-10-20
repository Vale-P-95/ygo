//! What the search did with the actions of your cards: how often the engine
//! offered each one, why it was left out when it was (the answer policy,
//! or leading back where the search has been), how often a line chose it
//! and how often the effect was really activated. A mechanic offered but
//! never chosen, or chosen but never activated, is where to look for a
//! card the search does not play.

use std::collections::HashMap;

use ygo_engine::{Action, ActionKind, CardDb, Decision, Event};

/// One of your actions: a card's summon, Set or position change, or one of
/// its effects (by its description), or an option of an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mechanic {
    /// The card's code (the original one of an alternate artwork).
    pub code: u32,
    pub kind: Kind,
    /// The effect's description (`aux.Stringid`), 0 when it has none.
    pub description: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    NormalSummon,
    SpecialSummon,
    SetMonster,
    SetSpellTrap,
    ChangePosition,
    Activate,
    /// "Yes" to an optional effect.
    Optional,
    /// An option of an effect ("choose 1 of these effects").
    Option,
    Attack,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::NormalSummon => "Normal Summon",
            Kind::SpecialSummon => "Special Summon",
            Kind::SetMonster => "Set (monster)",
            Kind::SetSpellTrap => "Set (Spell/Trap)",
            Kind::ChangePosition => "change position",
            Kind::Activate => "activate",
            Kind::Optional => "optional effect",
            Kind::Option => "option",
            Kind::Attack => "attack",
        }
    }
}

/// How often a mechanic met each fate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Decisions where the engine offered it.
    pub offered: u64,
    /// Left out by the answer policy (a handtrap kept in hand, a pointless
    /// Set, a forbidden card...).
    pub policy: u64,
    /// Left out as leading back where the search has been (or goes anyway).
    pub returns: u64,
    /// Left out to close a line that grew too long.
    pub cut: u64,
    /// Kept: chosen by the line, or queued to be tried later.
    pub kept: u64,
    /// Chosen by a line that was played.
    pub chosen: u64,
    /// Activated (a chain link of yours with this effect).
    pub activated: u64,
    /// Negated by another of your cards (the line is dropped).
    pub self_negated: u64,
    /// Negated one of your own chain links (while resolving).
    pub negated_own: u64,
    /// Summoned (for the summon mechanics: any Normal or Special Summon of
    /// the card, by its own procedure or by another card's effect).
    pub summoned: u64,
    /// Locations it was offered from (bits of the engine's locations).
    pub locations: u8,
}

/// What the search did with each mechanic of your cards.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    pub counts: HashMap<Mechanic, Counts>,
}

impl Coverage {
    pub fn merge(&mut self, other: &Coverage) {
        for (m, c) in &other.counts {
            let e = self.counts.entry(*m).or_default();
            e.offered += c.offered;
            e.policy += c.policy;
            e.returns += c.returns;
            e.cut += c.cut;
            e.kept += c.kept;
            e.chosen += c.chosen;
            e.activated += c.activated;
            e.self_negated += c.self_negated;
            e.negated_own += c.negated_own;
            e.summoned += c.summoned;
            e.locations |= c.locations;
        }
    }

    pub(crate) fn entry(&mut self, m: Mechanic) -> &mut Counts {
        self.counts.entry(m).or_default()
    }

    /// The mechanics by card, then kind and description.
    pub fn sorted(&self) -> Vec<(Mechanic, Counts)> {
        let mut v: Vec<(Mechanic, Counts)> = self.counts.iter().map(|(m, c)| (*m, *c)).collect();
        v.sort_by_key(|(m, _)| *m);
        v
    }
}

/// The mechanic of an action of yours, if it is one (not a target, a
/// material, a zone or ending the turn).
pub(crate) fn mechanic(db: &CardDb, decision: &Decision, action: &Action) -> Option<(Mechanic, u8)> {
    let kind = match action.kind {
        ActionKind::NormalSummon => Kind::NormalSummon,
        ActionKind::SpecialSummon => Kind::SpecialSummon,
        ActionKind::SetMonster => Kind::SetMonster,
        ActionKind::SetSpellTrap => Kind::SetSpellTrap,
        ActionKind::ChangePosition => Kind::ChangePosition,
        ActionKind::Activate => Kind::Activate,
        ActionKind::Attack => Kind::Attack,
        ActionKind::Yes if action.description != 0 => Kind::Optional,
        ActionKind::Choose if matches!(decision, Decision::Option { .. }) => Kind::Option,
        _ => return None,
    };
    let (code, location) = match action.card {
        Some(c) => (c.code, c.location),
        // An option or a "Yes" with no card: the card whose text it is.
        None => ((action.description >> 20) as u32, 0),
    };
    if code == 0 {
        return None;
    }
    Some((Mechanic { code: db.canonical_code(code), kind, description: action.description }, location))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fate {
    Policy,
    Cut,
    Returns,
    Kept,
}

/// The actions the engine offered at one decision, and what became of
/// each, recorded once the search has chosen.
pub(crate) struct Audit {
    offered: Vec<(Action, Fate)>,
}

impl Audit {
    /// `offered`: all the legal actions; `kept`: those the answer policy left.
    pub(crate) fn new(offered: Vec<Action>, kept: &[Action]) -> Self {
        let offered = offered
            .into_iter()
            .map(|a| {
                let fate = if kept.iter().any(|k| k.response == a.response) { Fate::Kept } else { Fate::Policy };
                (a, fate)
            })
            .collect();
        Audit { offered }
    }

    fn left_out(&mut self, kept: &[Action], fate: Fate) {
        for (a, f) in &mut self.offered {
            if *f == Fate::Kept && !kept.iter().any(|k| k.response == a.response) {
                *f = fate;
            }
        }
    }

    /// The line grew too long: only `kept` is left.
    pub(crate) fn cut(&mut self, kept: &[Action]) {
        self.left_out(kept, Fate::Cut);
    }

    /// The answers leading back where the search has been were dropped.
    pub(crate) fn returns(&mut self, kept: &[Action]) {
        self.left_out(kept, Fate::Returns);
    }

    pub(crate) fn record(&self, db: &CardDb, decision: &Decision, chosen: Option<&[u8]>, coverage: &mut Coverage) {
        for (a, fate) in &self.offered {
            let Some((m, location)) = mechanic(db, decision, a) else { continue };
            let c = coverage.entry(m);
            c.offered += 1;
            c.locations |= location;
            match fate {
                Fate::Policy => c.policy += 1,
                Fate::Cut => c.cut += 1,
                Fate::Returns => c.returns += 1,
                Fate::Kept => c.kept += 1,
            }
            if chosen == Some(a.response.as_slice()) {
                c.chosen += 1;
            }
        }
    }
}

/// Counts an answer chosen by replaying a queued edge.
pub(crate) fn chosen(db: &CardDb, decision: &Decision, offered: &[Action], answer: &[u8], coverage: &mut Coverage) {
    if let Some((m, _)) = offered.iter().find(|a| a.response == answer).and_then(|a| mechanic(db, decision, a)) {
        coverage.entry(m).chosen += 1;
    }
}

/// The chain links of your line so far, to tell who negates whom.
#[derive(Default)]
pub(crate) struct Links {
    /// Your links by number (the opponent never answers on your turn).
    mine: HashMap<u32, Mechanic>,
    /// The link resolving.
    solving: Option<u32>,
}

/// One of your chain links negated on your turn: by another of your cards.
pub(crate) struct Negation {
    /// The link negated, when it is known (it may have started before
    /// the part of the line followed).
    pub(crate) negated: Option<Mechanic>,
    /// The link that negated it while resolving; `None` for a lasting
    /// effect (Crossout Designator on a name) or an unknown link.
    pub(crate) by: Option<Mechanic>,
}

impl Links {
    /// Follows the chain through one event. A negation returns who was
    /// negated and by whom.
    #[must_use = "a negation is how a line loses"]
    pub(crate) fn follow(&mut self, db: &CardDb, event: &Event) -> Option<Negation> {
        match event {
            Event::Chaining { card, description, chain_link } if card.controller == 0 => {
                let m = Mechanic { code: db.canonical_code(card.code), kind: Kind::Activate, description: *description };
                self.mine.insert(*chain_link, m);
            }
            Event::ChainSolving { chain_link } => self.solving = Some(u32::from(*chain_link)),
            Event::ChainNegated { chain_link } | Event::ChainDisabled { chain_link } => {
                let negated = u32::from(*chain_link);
                return Some(Negation {
                    negated: self.mine.get(&negated).copied(),
                    by: self.solving.filter(|&s| s != negated).and_then(|s| self.mine.get(&s)).copied(),
                });
            }
            Event::ChainEnd => {
                self.mine.clear();
                self.solving = None;
            }
            _ => {}
        }
        None
    }
}

/// Counts an event of your line: your summons and activations, and the
/// negation `follow` found in it.
pub(crate) fn count(db: &CardDb, event: &Event, negation: Option<&Negation>, coverage: &mut Coverage) {
    match event {
        Event::Chaining { card, description, .. } if card.controller == 0 => {
            coverage.entry(Mechanic { code: db.canonical_code(card.code), kind: Kind::Activate, description: *description }).activated += 1;
        }
        Event::NormalSummoning { card } | Event::FlipSummoning { card } if card.controller == 0 => {
            coverage.entry(Mechanic { code: db.canonical_code(card.code), kind: Kind::NormalSummon, description: 0 }).summoned += 1;
        }
        Event::SpecialSummoning { card } if card.controller == 0 => {
            coverage.entry(Mechanic { code: db.canonical_code(card.code), kind: Kind::SpecialSummon, description: 0 }).summoned += 1;
        }
        _ => {}
    }
    if let Some(n) = negation {
        if let Some(m) = n.negated {
            coverage.entry(m).self_negated += 1;
        }
        if let Some(m) = n.by {
            coverage.entry(m).negated_own += 1;
        }
    }
}
