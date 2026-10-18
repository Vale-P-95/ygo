//! How much a copy is worth, and which copies to change. Everything here
//! uses the exact calculation: card effects of the simulator are not
//! considered.
//!
//! Changes keep the deck size by swapping with a "neutral" card, a card
//! with no role; when the deck has none, a blank card stands in for it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::cond::Cond;
use crate::exact::{probability, CardEntry, DeckModel, ExactError};
use crate::sim::Rng;

/// Id of the stand-in neutral card (never a real passcode).
const BLANK: u32 = u32::MAX;

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

/// Removes one neutral copy (a card without roles, the most common one),
/// or returns `None` when the deck has none.
fn take_neutral(deck: &DeckModel) -> Option<DeckModel> {
    let neutral = deck.cards.iter().filter(|c| c.roles.is_empty()).max_by_key(|c| c.copies)?.id;
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
    let grew = take_neutral(deck).is_none();
    let plus = |d: DeckModel| take_neutral(&d).unwrap_or(d);
    let roles = roles
        .iter()
        .map(|r| {
            let up = plus(with_copies(deck, BLANK - 1, 1, std::slice::from_ref(r)));
            // One fewer: a card with this role (the one with fewest other roles) becomes neutral.
            let down = deck
                .cards
                .iter()
                .filter(|c| c.roles.contains(r))
                .min_by_key(|c| c.roles.len())
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
pub fn heatmap(deck: &DeckModel, cond: &Cond, hand: u32, role_a: &str, role_b: &str, span: u32) -> Heatmap {
    let pure = |r: &str| deck.cards.iter().filter(|c| c.roles.len() == 1 && c.roles[0] == r).map(|c| c.copies).sum::<u32>();
    let count = |r: &str| deck.copies_with(r);
    let (ca, cb) = (count(role_a), count(role_b));
    let (pa, pb) = (pure(role_a), pure(role_b));
    let neutral: u32 = deck.cards.iter().filter(|c| c.roles.is_empty()).map(|c| c.copies).sum();
    // The deck without its pure A, pure B and neutral cards; those are rebuilt per cell.
    let mut rest = deck.clone();
    rest.cards.retain(|c| !(c.roles.is_empty() || (c.roles.len() == 1 && (c.roles[0] == role_a || c.roles[0] == role_b))));
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

/// Simulated annealing over the copies of the free cards.
pub fn optimize(config: &OptimizeConfig) -> Result<OptimizeResult, ExactError> {
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
    let mut rng = Rng::new(config.seed);
    let mut current = pool.clone();
    let mut current_score = before;
    let mut best = (current.clone(), before);
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
    let changes = best
        .0
        .cards
        .iter()
        .filter_map(|c| {
            let from = config.deck.cards.iter().find(|x| x.id == c.id).map_or(0, |x| x.copies);
            (from != c.copies).then_some(Change { id: c.id, from, to: c.copies })
        })
        .collect();
    Ok(OptimizeResult { before, after: best.1, changes, size: best.0.size() })
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

    fn solid() -> Cond {
        crate::overview::solid()
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
}
