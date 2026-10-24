//! What a search can aim at ([`Goal`]): cards wanted at the end of the turn,
//! each in a place (a kind of zone on your field, the GY, the banishment,
//! the hand), hard (the goal is reached only with it) or soft (a board that
//! has it is preferred).

use ygo_engine::consts::location;
use ygo_engine::{Action, ActionKind, Decision, DuelState, Engine};

/// Where a card of the goal must be at the end of the turn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GoalZone {
    /// On your field: a face-up monster, or any card in the Spell & Trap row.
    #[default]
    Any,
    /// An Extra Monster Zone (some monsters work differently there, like
    /// Invoked Oceanus).
    Extra,
    /// A Main Monster Zone.
    Main,
    /// A face-up monster in any Monster Zone.
    Monster,
    /// A face-up card in a Spell & Trap Zone (Pendulum Zones included, the
    /// Field Zone not).
    FaceUp,
    /// A face-down card in a Spell & Trap Zone or the Field Zone.
    Set,
    /// A face-up card in the Field Zone.
    FieldZone,
    /// Your GY.
    Grave,
    /// Your banished cards, face-up or face-down.
    Banished,
    /// Your hand.
    Hand,
}

impl GoalZone {
    /// Off the field: the End Phase often moves such cards (a card banished
    /// "until the End Phase", one added to the hand at the end), so only a
    /// board played to the end of the turn says if they are there.
    pub fn off_field(self) -> bool {
        matches!(self, GoalZone::Grave | GoalZone::Banished | GoalZone::Hand)
    }

    /// Smaller first: a card in an Extra Monster Zone is also in "a Monster
    /// Zone" and on "the field", so the narrower places pick their cards
    /// first and the counting is the best one.
    fn breadth(self) -> u8 {
        match self {
            GoalZone::Any => 2,
            GoalZone::Monster => 1,
            _ => 0,
        }
    }

    fn holds(self, slot: Slot) -> bool {
        matches!(
            (self, slot),
            (GoalZone::Any, Slot::Main | Slot::Extra | Slot::FaceUp | Slot::Set | Slot::FieldZone)
                | (GoalZone::Monster, Slot::Main | Slot::Extra)
                | (GoalZone::Extra, Slot::Extra)
                | (GoalZone::Main, Slot::Main)
                | (GoalZone::FaceUp, Slot::FaceUp)
                | (GoalZone::Set, Slot::Set)
                | (GoalZone::FieldZone, Slot::FieldZone)
                | (GoalZone::Grave, Slot::Grave)
                | (GoalZone::Banished, Slot::Banished)
                | (GoalZone::Hand, Slot::Hand)
        )
    }

    /// What picking this card in this choice does for a card wanted here.
    fn pick(self, decision: &Decision, action: &Action) -> Move {
        let Some(card) = action.card else { return Move::Other };
        let hint = hint(decision);
        let mine = card.controller == 0;
        match self {
            // Summoning it, activating it, choosing it in a list: toward;
            // a choice that sends it away (a cost, a material): away.
            GoalZone::Any | GoalZone::Extra | GoalZone::Main | GoalZone::Monster | GoalZone::FaceUp | GoalZone::FieldZone => {
                if mine && sends_away(hint) {
                    Move::Away
                } else {
                    Move::Toward
                }
            }
            GoalZone::Set => match action.kind {
                ActionKind::SetSpellTrap => Move::Toward,
                ActionKind::Activate if mine => Move::Away,
                _ if hint == Some(HINT_SET) => Move::Toward,
                _ if mine && sends_away(hint) => Move::Away,
                _ => Move::Other,
            },
            // Activating or summoning it may also end in the GY, or not: only
            // the choices that say where the card goes count.
            GoalZone::Grave => match hint {
                Some(h) if mine && TO_GRAVE.contains(&h) => Move::Toward,
                Some(HINT_BANISH | HINT_TO_DECK) if mine => Move::Away,
                _ => Move::Other,
            },
            // The GY is often the way to the banishment (a cost banished from
            // there): only going back to the Deck is away.
            GoalZone::Banished => match hint {
                Some(HINT_BANISH) if mine => Move::Toward,
                Some(HINT_TO_DECK) if mine => Move::Away,
                _ => Move::Other,
            },
            GoalZone::Hand => match hint {
                Some(HINT_RETURN_TO_HAND | HINT_ADD_TO_HAND) if mine => Move::Toward,
                _ if !mine || card.location != location::HAND => Move::Other,
                _ if sends_away(hint) => Move::Away,
                _ => match action.kind {
                    ActionKind::Activate
                    | ActionKind::NormalSummon
                    | ActionKind::SpecialSummon
                    | ActionKind::SetMonster
                    | ActionKind::SetSpellTrap => Move::Away,
                    _ => Move::Other,
                },
            },
        }
    }
}

/// A card wanted at the end of the turn, where, and how much.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalCard {
    pub code: u32,
    pub zone: GoalZone,
    /// The goal is reached only with it; otherwise (soft) a board that has
    /// it is preferred.
    pub hard: bool,
}

impl GoalCard {
    pub fn new(code: u32, zone: GoalZone) -> GoalCard {
        GoalCard { code, zone, hard: true }
    }
}

/// Cards wanted at the end of the turn. The search favours the lines that
/// bring them, and ranks the boards that have the hard ones first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Goal {
    /// A card listed twice wants two copies.
    pub cards: Vec<GoalCard>,
    /// All the hard ones; otherwise at least one.
    pub all: bool,
    /// Stop at the first board that reaches the goal.
    pub first_only: bool,
}

/// How many cards of the goal a board has ([`Goal::hits`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoalHits {
    pub hard: usize,
    pub soft: usize,
}

impl Goal {
    pub fn is_set(&self) -> bool {
        !self.cards.is_empty()
    }

    /// Any zone of your field for each of `codes`, all hard.
    pub fn cards(codes: &[u32], all: bool, first_only: bool) -> Goal {
        Goal { cards: codes.iter().map(|&code| GoalCard::new(code, GoalZone::Any)).collect(), all, first_only }
    }

    /// Some card is hard: only then can the goal be reached (soft cards
    /// alone only steer).
    pub fn has_target(&self) -> bool {
        self.cards.iter().any(|g| g.hard)
    }

    /// Some hard card is wanted off the field ([`GoalZone::off_field`]).
    pub fn off_field(&self) -> bool {
        self.cards.iter().any(|g| g.hard && g.zone.off_field())
    }

    /// How many of the wanted cards are where they are wanted, hard and soft
    /// apart (alternate artworks count). Each card of the board counts for
    /// one wanted card at most: the hard ones pick first, then the narrower
    /// places.
    pub fn hits(&self, engine: &Engine, state: &DuelState) -> GoalHits {
        if !self.is_set() {
            return GoalHits::default();
        }
        let db = engine.cards();
        let me = &state.players[0];
        let monsters =
            me.monsters.iter().enumerate().filter_map(|(i, c)| {
                c.as_ref().filter(|c| c.is_face_up()).map(|c| (c.code, if i >= 5 { Slot::Extra } else { Slot::Main }))
            });
        let spells = me.spells.iter().enumerate().filter_map(|(i, c)| {
            c.as_ref().map(|c| {
                let slot = match (c.is_face_up(), i == FIELD_ZONE) {
                    (false, _) => Slot::Set,
                    (true, true) => Slot::FieldZone,
                    (true, false) => Slot::FaceUp,
                };
                (c.code, slot)
            })
        });
        let off_field = me
            .grave
            .iter()
            .map(|&c| (c, Slot::Grave))
            .chain(me.banished.iter().map(|&(c, _)| (c, Slot::Banished)))
            .chain(me.hand.iter().map(|&c| (c, Slot::Hand)));
        let mut board: Vec<(u32, Slot)> = monsters.chain(spells).chain(off_field).map(|(c, s)| (db.canonical_code(c), s)).collect();
        let mut order: Vec<&GoalCard> = self.cards.iter().collect();
        order.sort_by_key(|g| (!g.hard, g.zone.breadth()));
        let mut hits = GoalHits::default();
        for wanted in order {
            let code = db.canonical_code(wanted.code);
            if let Some(i) = board.iter().position(|&(c, s)| c == code && wanted.zone.holds(s)) {
                board.swap_remove(i);
                if wanted.hard {
                    hits.hard += 1;
                } else {
                    hits.soft += 1;
                }
            }
        }
        hits
    }

    /// The Monster Zone the goal wants this card in, if it names one.
    pub(crate) fn zone_of(&self, engine: &Engine, code: u32) -> Option<GoalZone> {
        let db = engine.cards();
        self.cards
            .iter()
            .find(|g| matches!(g.zone, GoalZone::Extra | GoalZone::Main) && db.canonical_code(g.code) == db.canonical_code(code))
            .map(|g| g.zone)
    }

    /// Some card is wanted in an Extra Monster Zone.
    pub(crate) fn wants_extra(&self) -> bool {
        self.cards.iter().any(|g| g.zone == GoalZone::Extra)
    }

    /// The hard cards are there (all of them, or one).
    pub fn reached(&self, hits: usize) -> bool {
        let hard = self.cards.iter().filter(|g| g.hard).count();
        hard > 0 && if self.all { hits == hard } else { hits > 0 }
    }

    /// What the answer does for the goal: picking a goal card brings it
    /// closer, unless the choice takes it from where it is wanted (for a
    /// monster, a cost that banishes it or a material; for a card wanted in
    /// the hand, playing it).
    pub(crate) fn pick(&self, engine: &Engine, decision: &Decision, state: &DuelState, action: &Action) -> GoalPick {
        let Some(card) = action.card else { return GoalPick::Other };
        let db = engine.cards();
        let code = db.canonical_code(card.code);
        let (mut toward, mut away) = (false, false);
        for g in self.cards.iter().filter(|g| db.canonical_code(g.code) == code) {
            match g.zone.pick(decision, action) {
                Move::Toward => toward = true,
                Move::Away => away = true,
                Move::Other => {}
            }
        }
        if toward || !away {
            return if toward { GoalPick::Toward } else { GoalPick::Other };
        }
        // Copies of it still in your hand, Deck, Extra Deck or on the field.
        let same = |c: u32| db.canonical_code(c) == code;
        let me = &state.players[0];
        let copies = me.hand.iter().filter(|&&c| same(c)).count()
            + me.deck.iter().filter(|&&c| same(c)).count()
            + me.extra.iter().filter(|&&(c, _)| same(c)).count()
            + me.monsters.iter().chain(&me.spells).flatten().filter(|c| same(c.code)).count();
        let wanted = self.cards.iter().filter(|g| db.canonical_code(g.code) == code).count();
        let in_play = card.location & (location::HAND | location::DECK | location::EXTRA | location::MZONE | location::SZONE) != 0;
        GoalPick::Away { last: in_play && copies <= wanted }
    }
}

/// What an answer does for the goal ([`Goal::pick`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoalPick {
    Toward,
    /// `last`: no other copy is left to reach the goal with.
    Away {
        last: bool,
    },
    Other,
}

/// What a pick does for one card of the goal.
enum Move {
    Toward,
    Away,
    Other,
}

/// Where a card of the board is, for the places of [`GoalZone`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Main,
    Extra,
    FaceUp,
    Set,
    FieldZone,
    Grave,
    Banished,
    Hand,
}

/// The Field Zone among `PlayerState::spells`.
const FIELD_ZONE: usize = 5;

// The engine's hints for a selection (`HINTMSG_*` of the scripts).
const HINT_BANISH: u64 = 503;
const HINT_RETURN_TO_HAND: u64 = 505;
const HINT_ADD_TO_HAND: u64 = 506;
const HINT_TO_DECK: u64 = 507;
const HINT_SET: u64 = 510;
/// Tributed, discarded, destroyed, sent to the GY, used as Fusion, Synchro
/// or Link Material (an Xyz Material stays on the field).
const TO_GRAVE: [u64; 7] = [500, 501, 502, 504, 511, 512, 533];

fn hint(decision: &Decision) -> Option<u64> {
    match decision {
        Decision::SelectCard { hint, .. }
        | Decision::SelectUnselectCard { hint, .. }
        | Decision::SelectTribute { hint, .. }
        | Decision::SelectSum { hint, .. } => Some(*hint),
        _ => None,
    }
}

/// The engine's hint says the cards chosen leave: Tributed, discarded,
/// destroyed, banished, sent to the GY or the Deck, used as Fusion, Synchro,
/// Xyz or Link Material.
fn sends_away(hint: Option<u64>) -> bool {
    matches!(hint, Some(500..=504 | 507 | 511..=513 | 533))
}
