//! How much a copy is worth, and which copies to change. Everything here
//! uses the exact calculation: card effects of the simulator are not
//! considered.
//!
//! Changes keep the deck size by swapping with a "neutral" card, a card
//! with no role; when the deck has none, a blank card stands in for it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::cond::{Cond, Target};
use crate::exact::{probability, CardEntry, DeckModel, ExactError};
use crate::sim::Rng;

/// Id of the stand-in neutral card (never a real passcode).
const BLANK: u32 = u32::MAX;

/// Passcodes the condition names: their copies count, so they are never neutral.
fn named_cards(cond: &Cond) -> BTreeSet<u32> {
    let mut targets = BTreeSet::new();
    cond.targets(&mut targets);
    targets.into_iter().filter_map(|t| if let Target::Card(id) = t { Some(id) } else { None }).collect()
}

fn with_copies(deck: &DeckModel, id: u32, delta: i64, roles: &[String]) -> DeckModel {
    let mut d = deck.clone();
    match d.cards.iter_mut().find(|c| c.id == id) {
        Some(c) => c.copies = (c.copies as i64 + delta).max(0) as u32,
        None if delta > 0 => d.cards.push(CardEntry { id, copies: delta as u32, roles: roles.to_vec() }),
        None => {}
    }
    d.cards.retain(|c| c.copies > 0);
    d
}

/// Removes one neutral copy (a card without roles that the condition does
/// not name, the most common one), or returns `None` when the deck has none.
fn take_neutral(deck: &DeckModel, named: &BTreeSet<u32>) -> Option<DeckModel> {
    let neutral = deck.cards.iter().filter(|c| c.roles.is_empty() && !named.contains(&c.id)).max_by_key(|c| c.copies)?.id;
    Some(with_copies(deck, neutral, -1, &[]))
}

fn add_neutral(deck: &DeckModel) -> DeckModel {
    with_copies(deck, BLANK, 1, &[])
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    /// Role key, or the card's passcode as text.
    pub key: String,
    /// Change of probability with one copy more / less (`None` when impossible).
    pub plus: Option<f64>,
    pub minus: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sensitivity {
    pub base: f64,
    pub roles: Vec<Delta>,
    pub cards: Vec<Delta>,
    /// The deck had no neutral card, so "+1" made it one card bigger.
    pub grew: bool,
}

/// Value of one copy more or less, per role and per card.
pub fn sensitivity(
    deck: &DeckModel,
    cond: &Cond,
    hand: u32,
    roles: &[String],
    limits: &BTreeMap<u32, u32>,
) -> Result<Sensitivity, ExactError> {
    let base = probability(deck, cond, hand)?;
    let p = |d: &DeckModel| probability(d, cond, hand).ok().map(|v| v - base);
    let named = named_cards(cond);
    let grew = take_neutral(deck, &named).is_none();
    let plus = |d: DeckModel| take_neutral(&d, &named).unwrap_or(d);
    let roles = roles
        .iter()
        .map(|r| {
            let up = plus(with_copies(deck, BLANK - 1, 1, std::slice::from_ref(r)));
            // One fewer: a card with this role becomes neutral, one the condition
            // does not name if possible, then the one with fewest other roles.
            let down = deck
                .cards
                .iter()
                .filter(|c| c.roles.contains(r))
                .min_by_key(|c| (named.contains(&c.id), c.roles.len()))
                .map(|c| add_neutral(&with_copies(deck, c.id, -1, &[])));
            Delta { key: r.clone(), plus: p(&up), minus: down.as_ref().and_then(p) }
        })
        .collect();
    let cards = deck
        .cards
        .iter()
        .filter(|c| c.id != BLANK)
        .map(|c| {
            let limit = limits.get(&c.id).copied().unwrap_or(3);
            let up = (c.copies < limit).then(|| plus(with_copies(deck, c.id, 1, &c.roles)));
            let down = add_neutral(&with_copies(deck, c.id, -1, &[]));
            Delta { key: c.id.to_string(), plus: up.as_ref().and_then(p), minus: p(&down) }
        })
        .collect();
    Ok(Sensitivity { base, roles, cards, grew })
}

/// Probability with `extra` neutral cards added (40 → 41…).
pub fn size_curve(deck: &DeckModel, cond: &Cond, hand: u32, extra: u32) -> Vec<Option<f64>> {
    (0..=extra).map(|k| probability(&with_copies(deck, BLANK, k as i64, &[]), cond, hand).ok()).collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    /// Copies of role A (rows) and B (columns).
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub p: Vec<Vec<Option<f64>>>,
    /// Current copies.
    pub current: (u32, u32),
}

/// Probability for every mix of copies of two roles around the current
/// one: copies of cards with only that role change, the rest is neutral.
/// Cards the condition names keep their copies.
pub fn heatmap(deck: &DeckModel, cond: &Cond, hand: u32, role_a: &str, role_b: &str, span: u32) -> Heatmap {
    let named = named_cards(cond);
    // Pooled cards lose their passcode, so a named card must stay as it is.
    let pooled = |c: &CardEntry| {
        !named.contains(&c.id) && (c.roles.is_empty() || (c.roles.len() == 1 && (c.roles[0] == role_a || c.roles[0] == role_b)))
    };
    let pure = |r: &str| deck.cards.iter().filter(|c| pooled(c) && c.roles.len() == 1 && c.roles[0] == r).map(|c| c.copies).sum::<u32>();
    let count = |r: &str| deck.copies_with(r);
    let (ca, cb) = (count(role_a), count(role_b));
    let (pa, pb) = (pure(role_a), pure(role_b));
    let neutral: u32 = deck.cards.iter().filter(|c| pooled(c) && c.roles.is_empty()).map(|c| c.copies).sum();
    // The deck without its pure A, pure B and neutral cards; those are rebuilt per cell.
    let mut rest = deck.clone();
    rest.cards.retain(|c| !pooled(c));
    let pool = pa + pb + neutral;
    let a: Vec<u32> = (ca.saturating_sub(span)..=ca + span).collect();
    let b: Vec<u32> = (cb.saturating_sub(span)..=cb + span).collect();
    let p = a
        .iter()
        .map(|&x| {
            b.iter()
                .map(|&y| {
                    // Copies of A and B that must come from pure cards.
                    let (na, nb) = ((x as i64) - (ca - pa) as i64, (y as i64) - (cb - pb) as i64);
                    if na < 0 || nb < 0 || na + nb > pool as i64 {
                        return None;
                    }
                    let mut d = rest.clone();
                    d.cards.push(CardEntry { id: BLANK - 1, copies: na as u32, roles: vec![role_a.to_owned()] });
                    d.cards.push(CardEntry { id: BLANK - 2, copies: nb as u32, roles: vec![role_b.to_owned()] });
                    d.cards.push(CardEntry { id: BLANK, copies: pool - na as u32 - nb as u32, roles: vec![] });
                    d.cards.retain(|c| c.copies > 0);
                    probability(&d, cond, hand).ok()
                })
                .collect()
        })
        .collect();
    Heatmap { a, b, p, current: (ca, cb) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeConfig {
    pub deck: DeckModel,
    /// Cards that may enter the deck (the Maybe board), with their roles.
    #[serde(default)]
    pub candidates: Vec<CardEntry>,
    /// Most copies allowed per card (banlist); 3 when missing.
    #[serde(default)]
    pub limits: BTreeMap<u32, u32>,
    /// Cards whose copies must not change.
    #[serde(default)]
    pub locked: Vec<u32>,
    pub cond: Cond,
    /// Weight of going first (5 cards) against going second (6), 0..=1.
    pub weight_first: f64,
    pub min_size: u32,
    pub max_size: u32,
    pub iterations: u32,
    #[serde(default)]
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub first: f64,
    pub second: f64,
    pub objective: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub id: u32,
    pub from: u32,
    pub to: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeResult {
    pub before: Score,
    pub after: Score,
    pub changes: Vec<Change>,
    pub size: u32,
}

fn score(deck: &DeckModel, cond: &Cond, w: f64) -> Result<Score, ExactError> {
    let first = probability(deck, cond, 5)?;
    let second = probability(deck, cond, 6)?;
    Ok(Score { first, second, objective: w * first + (1.0 - w) * second })
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptimizeError {
    Exact(ExactError),
    /// The deck's cards and the Maybe board, within their limits, cannot make
    /// a deck of an allowed size.
    Size {
        min: u32,
        max: u32,
    },
}

impl From<ExactError> for OptimizeError {
    fn from(e: ExactError) -> Self {
        OptimizeError::Exact(e)
    }
}

impl std::fmt::Display for OptimizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimizeError::Exact(e) => e.fmt(f),
            OptimizeError::Size { min, max } if min == max => {
                write!(f, "con queste carte e questi limiti il deck non può avere {min} carte")
            }
            OptimizeError::Size { min, max } => write!(f, "con queste carte e questi limiti il deck non può avere tra {min} e {max} carte"),
        }
    }
}

impl std::error::Error for OptimizeError {}

/// Simulated annealing over the copies of the free cards.
pub fn optimize(config: &OptimizeConfig) -> Result<OptimizeResult, OptimizeError> {
    let w = config.weight_first.clamp(0.0, 1.0);
    let mut pool = config.deck.clone();
    for c in &config.candidates {
        if !pool.cards.iter().any(|x| x.id == c.id) {
            pool.cards.push(CardEntry { copies: 0, ..c.clone() });
        }
    }
    let live = |d: &DeckModel| DeckModel { cards: d.cards.iter().filter(|c| c.copies > 0).cloned().collect() };
    let before = score(&config.deck, &config.cond, w)?;
    let free: Vec<usize> = (0..pool.cards.len()).filter(|&i| !config.locked.contains(&pool.cards[i].id)).collect();
    let limit = |id: u32| config.limits.get(&id).copied().unwrap_or(3);
    let mut current = pool.clone();
    // A deck outside the allowed sizes first gets there, cards without roles
    // going in (or out) first: only decks of an allowed size are compared.
    let named = named_cards(&config.cond);
    let mut by_role: Vec<usize> = free.clone();
    by_role.sort_by_key(|&i| (named.contains(&current.cards[i].id), !current.cards[i].roles.is_empty()));
    while current.size() < config.min_size {
        let i = by_role.iter().copied().find(|&i| current.cards[i].copies < limit(current.cards[i].id));
        let Some(i) = i else { return Err(OptimizeError::Size { min: config.min_size, max: config.max_size }) };
        current.cards[i].copies += 1;
    }
    while current.size() > config.max_size {
        let Some(i) = by_role.iter().copied().find(|&i| current.cards[i].copies > 0) else {
            return Err(OptimizeError::Size { min: config.min_size, max: config.max_size });
        };
        current.cards[i].copies -= 1;
    }
    let mut rng = Rng::new(config.seed);
    let mut current_score = score(&live(&current), &config.cond, w)?;
    let mut best = (current.clone(), current_score);
    let iterations = config.iterations.max(1);
    for step in 0..iterations {
        if free.is_empty() {
            break;
        }
        let temp = 0.02 * (0.0005f64 / 0.02).powf(step as f64 / iterations as f64);
        let mut next = current.clone();
        let size = next.size();
        let a = free[rng.below(free.len() as u32) as usize];
        let b = free[rng.below(free.len() as u32) as usize];
        let kind = rng.below(3);
        let (remove, add) = match kind {
            0 if size > config.min_size => (Some(a), None),
            1 if size < config.max_size => (None, Some(b)),
            _ => (Some(a), Some(b)),
        };
        if let Some(i) = remove {
            if next.cards[i].copies == 0 {
                continue;
            }
            next.cards[i].copies -= 1;
        }
        if let Some(i) = add {
            if next.cards[i].copies >= limit(next.cards[i].id) || remove == add {
                continue;
            }
            next.cards[i].copies += 1;
        }
        let Ok(s) = score(&live(&next), &config.cond, w) else { continue };
        let delta = s.objective - current_score.objective;
        let accept = delta >= 0.0 || (rng.next_u64() as f64 / u64::MAX as f64) < (delta / temp).exp();
        if accept {
            current = next;
            current_score = s;
            if s.objective > best.1.objective + 1e-12 {
                best = (current.clone(), s);
            }
        }
    }
    let (deck, after) = prune(config, &free, best, |d| score(&live(d), &config.cond, w).ok());
    let changes = deck
        .cards
        .iter()
        .filter_map(|c| {
            let from = config.deck.cards.iter().find(|x| x.id == c.id).map_or(0, |x| x.copies);
            (from != c.copies).then_some(Change { id: c.id, from, to: c.copies })
        })
        .collect();
    Ok(OptimizeResult { before, after, changes, size: deck.size() })
}

/// The annealing also keeps moves that change nothing (ties, and detours
/// taken while it was hot). Undoes them one copy at a time, toward the
/// starting deck, while the objective holds; then a card with a role that
/// went out for nothing goes back in place of a card without roles.
fn prune(
    config: &OptimizeConfig,
    free: &[usize],
    best: (DeckModel, Score),
    score: impl Fn(&DeckModel) -> Option<Score>,
) -> (DeckModel, Score) {
    let (mut deck, mut kept) = best;
    let floor = kept.objective - 1e-12;
    let from: Vec<u32> = deck.cards.iter().map(|c| config.deck.cards.iter().find(|x| x.id == c.id).map_or(0, |x| x.copies)).collect();
    // Tries a move (one copy out of `down`, one into `up`); keeps it when the objective holds.
    let mut attempt = |deck: &mut DeckModel, down: Option<usize>, up: Option<usize>| {
        let mut next = deck.clone();
        if let Some(i) = down {
            next.cards[i].copies -= 1;
        }
        if let Some(i) = up {
            next.cards[i].copies += 1;
        }
        match score(&next) {
            Some(s) if s.objective >= floor => {
                *deck = next;
                kept = s;
                true
            }
            _ => false,
        }
    };
    loop {
        let size = deck.size();
        let more: Vec<usize> = free.iter().copied().filter(|&i| deck.cards[i].copies > from[i]).collect();
        let fewer: Vec<usize> = free.iter().copied().filter(|&i| deck.cards[i].copies < from[i]).collect();
        let mut moves: Vec<(Option<usize>, Option<usize>)> =
            more.iter().flat_map(|&i| fewer.iter().map(move |&j| (Some(i), Some(j)))).collect();
        if size > config.min_size {
            moves.extend(more.iter().map(|&i| (Some(i), None)));
        }
        if size < config.max_size {
            moves.extend(fewer.iter().map(|&j| (None, Some(j))));
        }
        if !moves.into_iter().any(|(down, up)| attempt(&mut deck, down, up)) {
            break;
        }
    }
    let (mut neutral, with_role): (Vec<usize>, Vec<usize>) = free.iter().partition(|&&i| deck.cards[i].roles.is_empty());
    neutral.sort_by_key(|&i| std::cmp::Reverse(deck.cards[i].copies));
    for j in with_role {
        while deck.cards[j].copies < from[j] {
            let swapped = neutral.iter().copied().any(|i| deck.cards[i].copies > 0 && attempt(&mut deck, Some(i), Some(j)));
            if !swapped {
                break;
            }
        }
    }
    (deck, kept)
}

/// What [`climb`] may change, and how hard it tries.
#[derive(Debug, Clone)]
pub struct ClimbConfig {
    pub deck: DeckModel,
    /// Cards that may enter the deck (the Maybe board).
    pub candidates: Vec<CardEntry>,
    /// Most copies allowed per card (banlist); 3 when missing.
    pub limits: BTreeMap<u32, u32>,
    /// Cards whose copies must not change.
    pub locked: Vec<u32>,
    /// Decks measured at most, the starting one excluded.
    pub budget: usize,
    /// Gain a swap must surely bring to be kept (see [`Judged::gain_low`]).
    pub margin: f64,
}

/// One copy of the first card out, one of the second in.
type Swap = (u32, u32);

/// What the objective says of a deck next to the current one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Judged {
    pub objective: f64,
    /// The gain over the current deck that the measure is sure of: the
    /// low end of its interval for a noisy measure, the gain itself for
    /// an exact one.
    pub gain_low: f64,
}

/// A swap [`climb`] measured: one copy of `out` for one of `into`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tried {
    pub out: u32,
    pub into: u32,
    /// `None`: the measure failed.
    pub objective: Option<f64>,
    pub kept: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClimbResult {
    pub before: f64,
    pub after: f64,
    pub changes: Vec<Change>,
    pub tried: Vec<Tried>,
}

/// One copy out and one in at a time, for an objective the exact
/// calculation cannot give: the Monte Carlo played by the engine. Each
/// measure is a search of many hands, so the annealing of [`optimize`] is
/// out of reach. `rank` lists the swaps `(out, into)` of a deck, most
/// promising first; `judge(current, next)` measures them in that order.
/// The first whose sure gain passes the margin is kept and the swaps of
/// the new deck are ranked again, until a deck where none passes or the
/// budget is spent. `before` is the starting deck's objective. The size of
/// the deck never changes, so draws of the same positions stay comparable.
pub fn climb(
    config: &ClimbConfig,
    before: f64,
    mut rank: impl FnMut(&DeckModel) -> Vec<(u32, u32)>,
    mut judge: impl FnMut(&DeckModel, &DeckModel) -> Option<Judged>,
) -> ClimbResult {
    let limit = |id: u32| config.limits.get(&id).copied().unwrap_or(3);
    let mut current = config.deck.clone();
    let mut current_score = before;
    let mut tried = Vec::new();
    // The swaps measured from each deck, by its copies: a deck seen again tries the others.
    let mut seen: BTreeMap<Vec<(u32, u32)>, Vec<Swap>> = BTreeMap::new();
    let copies = |d: &DeckModel| {
        let mut v: Vec<(u32, u32)> = d.cards.iter().filter(|c| c.copies > 0).map(|c| (c.id, c.copies)).collect();
        v.sort_unstable();
        v
    };
    'decks: while tried.len() < config.budget {
        let done = seen.entry(copies(&current)).or_default().clone();
        let have = |id: u32| current.cards.iter().find(|c| c.id == id).map_or(0, |c| c.copies);
        let valid = |&(out, into): &(u32, u32)| {
            out != into
                && have(out) > 0
                && have(into) < limit(into)
                && !config.locked.contains(&out)
                && !config.locked.contains(&into)
                && (current.cards.iter().any(|c| c.id == into) || config.candidates.iter().any(|c| c.id == into))
                && !done.contains(&(out, into))
        };
        let moves: Vec<(u32, u32)> = rank(&current).into_iter().filter(valid).collect();
        for (out, into) in moves {
            if tried.len() >= config.budget {
                break 'decks;
            }
            let roles = config.candidates.iter().find(|c| c.id == into).map(|c| c.roles.clone()).unwrap_or_default();
            let next = with_copies(&with_copies(&current, out, -1, &[]), into, 1, &roles);
            seen.entry(copies(&current)).or_default().push((out, into));
            let judged = judge(&current, &next);
            let kept = judged.is_some_and(|j| j.gain_low > config.margin);
            tried.push(Tried { out, into, objective: judged.map(|j| j.objective), kept });
            if let (true, Some(j)) = (kept, judged) {
                current = next;
                current_score = j.objective;
                continue 'decks;
            }
        }
        break;
    }
    let changes = current
        .cards
        .iter()
        .map(|c| (c.id, c.copies))
        .chain(config.deck.cards.iter().filter(|c| !current.cards.iter().any(|x| x.id == c.id)).map(|c| (c.id, 0)))
        .filter_map(|(id, to)| {
            let from = config.deck.cards.iter().find(|x| x.id == id).map_or(0, |x| x.copies);
            (from != to).then_some(Change { id, from, to })
        })
        .collect();
    ClimbResult { before, after: current_score, changes, tried }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cond::Op;

    fn deck(entries: &[(u32, u32, &[&str])]) -> DeckModel {
        DeckModel {
            cards: entries
                .iter()
                .map(|(id, copies, roles)| CardEntry { id: *id, copies: *copies, roles: roles.iter().map(|r| r.to_string()).collect() })
                .collect(),
        }
    }

    fn invoked() -> DeckModel {
        deck(&[(1, 13, &["starter"]), (2, 11, &["extender"]), (3, 10, &["handtrap"]), (4, 3, &["breaker"]), (5, 3, &[])])
    }

    /// "Mano solida" as a condition: the same thing in a deck where no card has two roles.
    fn solid() -> Cond {
        let starter = |n| Cond::role("starter", Op::AtLeast, n);
        let resist = Cond::any([starter(2), Cond::all([starter(1), Cond::role("extender", Op::AtLeast, 1)])]);
        Cond::all([resist, Cond::role("handtrap", Op::AtLeast, 1)])
    }

    #[test]
    fn role_values_match_the_plan() {
        let roles: Vec<String> = ["starter", "handtrap", "extender", "breaker"].iter().map(|s| s.to_string()).collect();
        let s = sensitivity(&invoked(), &solid(), 5, &roles, &BTreeMap::new()).unwrap();
        let get = |k: &str| s.roles.iter().find(|d| d.key == k).unwrap();
        assert!((get("handtrap").plus.unwrap() - 0.0331).abs() < 5e-4);
        assert!((get("handtrap").minus.unwrap() + 0.0376).abs() < 5e-4);
        assert!((get("starter").plus.unwrap() - 0.0267).abs() < 5e-4);
        assert!(get("breaker").plus.unwrap().abs() < 1e-12);
        assert!(!s.grew);
        // No card goes past three copies (here a group of 13 stands for one card).
        assert!(s.cards.iter().find(|d| d.key == "1").unwrap().plus.is_none());
        assert!(s.cards.iter().find(|d| d.key == "1").unwrap().minus.is_some());
    }

    #[test]
    fn bigger_decks_lower_the_odds() {
        let cond = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]);
        let curve = size_curve(&deck(&[(1, 13, &["starter"]), (3, 10, &["handtrap"]), (5, 17, &[])]), &cond, 5, 5);
        let v: Vec<f64> = curve.into_iter().map(Option::unwrap).collect();
        assert!((v[0] - 0.6701).abs() < 5e-4 && (v[5] - 0.591).abs() < 5e-4, "{v:?}");
        assert!(v.windows(2).all(|w| w[1] < w[0]));
    }

    #[test]
    fn heatmap_centres_on_the_current_deck() {
        let cond = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]);
        let h = heatmap(&deck(&[(1, 13, &["starter"]), (3, 10, &["handtrap"]), (5, 17, &[])]), &cond, 5, "starter", "handtrap", 4);
        assert_eq!((h.a.first(), h.a.last(), h.current), (Some(&9), Some(&17), (13, 10)));
        let cur = h.p[4][4].unwrap();
        assert!((cur - 0.6701).abs() < 5e-4);
        assert!(h.p[8][8].unwrap() > cur && h.p[0][0].unwrap() < cur);
    }

    /// 13 Starters, 10 Handtraps and 17 cards without roles, one card per passcode.
    fn named_deck() -> DeckModel {
        deck(&[
            (10, 3, &["starter"]),
            (11, 3, &["starter"]),
            (12, 3, &["starter"]),
            (13, 3, &["starter"]),
            (14, 1, &["starter"]),
            (20, 3, &["handtrap"]),
            (21, 3, &["handtrap"]),
            (22, 2, &["handtrap"]),
            (23, 2, &["handtrap"]),
            (30, 3, &[]),
            (31, 3, &[]),
            (32, 3, &[]),
            (33, 3, &[]),
            (34, 3, &[]),
            (35, 2, &[]),
        ])
    }

    #[test]
    fn heatmap_keeps_the_cards_the_condition_names() {
        // A named pure Starter, a named pure Handtrap, a named card without roles (also negated).
        let starter = Cond::role("starter", Op::AtLeast, 1);
        let conds = [
            Cond::all([Cond::card(10, Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]),
            Cond::all([starter.clone(), Cond::card(20, Op::AtLeast, 1)]),
            Cond::all([starter.clone(), Cond::card(30, Op::AtLeast, 1)]),
            Cond::all([starter, Cond::card(30, Op::Exactly, 0)]),
        ];
        let d = named_deck();
        for cond in &conds {
            for hand in [5, 6] {
                let h = heatmap(&d, cond, hand, "starter", "handtrap", 4);
                assert_eq!(h.current, (13, 10));
                let exact = probability(&d, cond, hand).unwrap();
                let cur = h.p[4][4].unwrap();
                assert!((cur - exact).abs() < 1e-12, "{cond:?} hand {hand}: {cur} vs {exact}");
            }
        }
        // Reference (exact enumeration): 24.37% and 31.68%; more Handtraps help.
        let h = heatmap(&d, &conds[0], 5, "starter", "handtrap", 4);
        assert!((h.p[4][4].unwrap() - 0.243_667).abs() < 1e-6);
        assert!(h.p[4][5].unwrap() > h.p[4][4].unwrap());
    }

    #[test]
    fn sensitivity_never_swaps_out_the_cards_the_condition_names() {
        let roles: Vec<String> = ["starter", "handtrap"].iter().map(|s| s.to_string()).collect();
        let starter_and_30 = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::card(30, Op::AtLeast, 1)]);
        let get = |v: &[Delta], k: &str| v.iter().find(|d| d.key == k).unwrap().clone();
        // References by exact enumeration: the swapped card is a card without roles the condition does not name.
        let mut cards =
            vec![(10, 3, &["starter"][..]), (11, 3, &["starter"]), (12, 3, &["starter"]), (13, 3, &["starter"]), (14, 1, &["starter"])];
        cards.extend([(20, 3, &["handtrap"][..]), (21, 3, &["handtrap"]), (22, 2, &["handtrap"]), (23, 2, &["handtrap"]), (30, 3, &[])]);
        cards.extend((31..38).map(|id| (id, 2, &[][..])));
        let s = sensitivity(&deck(&cards), &starter_and_30, 5, &roles, &BTreeMap::new()).unwrap();
        assert!((get(&s.roles, "starter").plus.unwrap() - 0.009_263).abs() < 1e-6, "{s:?}");
        assert!(get(&s.roles, "handtrap").plus.unwrap().abs() < 1e-12, "{s:?}");
        assert!(!s.grew);

        let mut cards: Vec<(u32, u32, &[&str])> = (10..15).map(|id| (id, 3, &["starter"][..])).collect();
        cards.extend((20..24).map(|id| (id, 3, &["handtrap"][..])));
        cards.extend((30..36).map(|id| (id, 2, &[][..])));
        cards.push((36, 1, &[]));
        let s = sensitivity(&deck(&cards), &starter_and_30, 5, &roles, &BTreeMap::new()).unwrap();
        assert!((get(&s.cards, "30").plus.unwrap() - 0.089_254).abs() < 1e-6, "{s:?}");

        // Removing a Starter takes one the condition does not name.
        let card_10 = Cond::all([Cond::card(10, Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]);
        let s = sensitivity(&named_deck(), &card_10, 5, &roles, &BTreeMap::new()).unwrap();
        assert!(get(&s.roles, "starter").minus.unwrap().abs() < 1e-12, "{s:?}");

        // Every card without roles is named: "+1" grows the deck instead.
        let only = deck(&[(1, 13, &["starter"]), (30, 3, &[])]);
        let s = sensitivity(&only, &starter_and_30, 5, &roles, &BTreeMap::new()).unwrap();
        assert!(s.grew);
    }

    fn drifting_config(seed: u64) -> OptimizeConfig {
        let mut cards: Vec<(u32, u32, &[&str])> = (1..5).map(|id| (id, 3, &["starter"][..])).collect();
        cards.push((5, 2, &["starter"]));
        cards.extend([(6, 3, &["extender"][..]), (7, 3, &["extender"]), (8, 3, &["extender"])]);
        cards.extend([(10, 3, &["handtrap"][..]), (11, 3, &["handtrap"]), (12, 3, &["handtrap"]), (13, 2, &["handtrap"])]);
        cards.extend([(14, 3, &["breaker"][..]), (15, 2, &[]), (16, 1, &[])]);
        OptimizeConfig {
            deck: deck(&cards),
            candidates: vec![],
            limits: BTreeMap::new(),
            locked: vec![],
            cond: Cond::role("starter", Op::AtLeast, 1),
            weight_first: 0.6,
            min_size: 40,
            max_size: 40,
            iterations: 2500,
            seed,
        }
    }

    #[test]
    fn optimizer_proposes_only_the_changes_that_matter() {
        // The only useful change: the fifteenth Starter in place of a card without roles
        // (objective 0.91603 → 0.93310 by exact enumeration).
        for seed in 0..8 {
            let r = optimize(&drifting_config(seed)).unwrap();
            assert!((r.before.objective - 0.916_027).abs() < 1e-6 && (r.after.objective - 0.933_098).abs() < 1e-6, "seed {seed}: {r:?}");
            let mut changes: Vec<(u32, u32, u32)> = r.changes.iter().map(|c| (c.id, c.from, c.to)).collect();
            changes.sort();
            assert!(changes == [(5, 2, 3), (15, 2, 1)] || changes == [(5, 2, 3), (16, 1, 0)], "seed {seed}: {changes:?}");
            assert_eq!(r.size, 40);
        }
    }

    #[test]
    fn optimizer_reaches_the_minimum_size() {
        // 35 cards that can make 40: the result has 40, and its score is the one of those 40.
        let mut cards: Vec<(u32, u32, &[&str])> = vec![(1, 2, &["starter"]), (2, 2, &["starter"]), (3, 1, &["starter"])];
        cards.extend([(10, 2, &["handtrap"][..]), (11, 2, &["handtrap"])]);
        cards.extend((20..33).map(|id| (id, 2, &[][..])));
        let d = deck(&cards);
        assert_eq!(d.size(), 35);
        let cond = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::role("handtrap", Op::AtLeast, 1)]);
        for seed in 0..4 {
            let config = OptimizeConfig {
                deck: d.clone(),
                candidates: vec![],
                limits: BTreeMap::new(),
                locked: vec![],
                cond: cond.clone(),
                weight_first: 0.6,
                min_size: 40,
                max_size: 40,
                iterations: 1500,
                seed,
            };
            let r = optimize(&config).unwrap();
            assert_eq!(r.size, 40, "seed {seed}: {r:?}");
            let mut after = d.clone();
            for c in &r.changes {
                match after.cards.iter_mut().find(|x| x.id == c.id) {
                    Some(x) => x.copies = c.to,
                    None => after.cards.push(CardEntry { id: c.id, copies: c.to, roles: vec![] }),
                }
            }
            after.cards.retain(|c| c.copies > 0);
            assert_eq!(after.size(), 40);
            let objective = 0.6 * probability(&after, &cond, 5).unwrap() + 0.4 * probability(&after, &cond, 6).unwrap();
            assert!((objective - r.after.objective).abs() < 1e-12);
        }
        // Every card already at three copies: 40 cannot be reached.
        let mut full: Vec<(u32, u32, &[&str])> = vec![(1, 3, &["starter"]), (2, 3, &["starter"]), (3, 2, &["starter"])];
        full.extend((10..13).map(|id| (id, 3, &["handtrap"][..])));
        full.extend((20..26).map(|id| (id, 3, &[][..])));
        let config = OptimizeConfig {
            deck: deck(&full),
            candidates: vec![],
            limits: BTreeMap::new(),
            locked: vec![],
            cond,
            weight_first: 0.6,
            min_size: 40,
            max_size: 40,
            iterations: 100,
            seed: 0,
        };
        assert_eq!(optimize(&config).unwrap_err(), OptimizeError::Size { min: 40, max: 40 });
    }

    #[test]
    fn optimizer_brings_in_starters_from_the_maybe_board() {
        let d = deck(&[(1, 2, &["starter"]), (9, 38, &[])]);
        let config = OptimizeConfig {
            deck: d,
            candidates: vec![CardEntry { id: 2, copies: 0, roles: vec!["starter".into()] }],
            limits: BTreeMap::from([(1, 3)]),
            locked: vec![],
            cond: Cond::role("starter", Op::AtLeast, 1),
            weight_first: 1.0,
            min_size: 40,
            max_size: 40,
            iterations: 400,
            seed: 5,
        };
        let r = optimize(&config).unwrap();
        assert_eq!(r.size, 40);
        assert!(r.after.objective > r.before.objective + 0.2, "{r:?}");
        assert!(r.changes.iter().any(|c| c.id == 2 && c.to == 3) && r.changes.iter().any(|c| c.id == 1 && c.to == 3), "{r:?}");
    }

    #[test]
    fn climb_keeps_the_swaps_that_gain_within_the_budget() {
        // The objective of the annealing test, measured: one more Starter in place of a card without roles.
        let config = drifting_config(0);
        let cond = config.cond.clone();
        let climb_config = ClimbConfig {
            deck: config.deck.clone(),
            candidates: vec![],
            limits: BTreeMap::new(),
            locked: vec![],
            budget: 40,
            margin: 1e-9,
        };
        let ids: Vec<u32> = config.deck.cards.iter().map(|c| c.id).collect();
        let all = |_: &DeckModel| ids.iter().flat_map(|&a| ids.iter().map(move |&b| (a, b))).collect::<Vec<_>>();
        let mut measured = 0;
        let p = |d: &DeckModel| probability(d, &cond, 5).unwrap();
        let mut judge = |current: &DeckModel, next: &DeckModel| {
            measured += 1;
            Some(Judged { objective: p(next), gain_low: p(next) - p(current) })
        };
        let r = climb(&climb_config, p(&config.deck), all, &mut judge);
        assert!(r.after > r.before + 1e-3, "{r:?}");
        let mut changes: Vec<(u32, u32, u32)> = r.changes.iter().map(|c| (c.id, c.from, c.to)).collect();
        changes.sort();
        // The only gain: card 5, the Starter below three copies, in place of a card that is not a Starter
        // (the first in `rank`'s order, an Extender: to this objective it is worth as much as a neutral card).
        assert_eq!(changes, [(5, 2, 3), (6, 3, 2)]);
        assert_eq!(measured, r.tried.len());
        assert!(r.tried.len() <= 40 && r.tried.iter().filter(|t| t.kept).count() == 1);
        // A budget of one swap measures one.
        let r = climb(&ClimbConfig { budget: 1, ..climb_config }, 0.0, all, |_, n| Some(Judged { objective: p(n), gain_low: 0.0 }));
        assert_eq!(r.tried.len(), 1);
    }
}
