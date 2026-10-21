//! Board score weights learned from the player's choices (Bradley-Terry):
//! between two boards the player prefers one, and the chance of preferring
//! A over B is modelled as `1 / (1 + e^(-w·(a - b)))`, `a` and `b` the
//! boards' metrics. The weights `w` are fitted by penalised logistic
//! regression, pulled towards the profile in use so that a few choices only
//! nudge it.

use std::collections::BTreeMap;

/// One choice: the metrics of the board preferred and of the other one.
#[derive(Debug, Clone)]
pub struct Choice {
    pub chosen: BTreeMap<String, f64>,
    pub other: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Learned {
    /// The weights, on the metrics of `prior`.
    pub weights: BTreeMap<String, f64>,
    pub choices: usize,
    /// Share of the choices the weights explain (the board preferred scores more).
    pub agreement: f64,
    /// The same with the prior weights, to compare.
    pub prior_agreement: f64,
}

/// How strongly the weights are pulled towards the prior: as many choices
/// as this weigh as much as the prior.
const PULL: f64 = 4.0;

/// Fits weights on the metrics of `prior` to explain `choices`.
pub fn learn_weights(choices: &[Choice], prior: &BTreeMap<String, f64>) -> Learned {
    let keys: Vec<&String> = prior.keys().collect();
    let n = keys.len();
    let w0: Vec<f64> = keys.iter().map(|k| prior[*k]).collect();
    // The difference of the metrics in each choice, scaled so that every
    // metric weighs alike in the penalty (metrics have different ranges).
    let diffs: Vec<Vec<f64>> = choices
        .iter()
        .map(|c| keys.iter().map(|k| c.chosen.get(*k).copied().unwrap_or(0.0) - c.other.get(*k).copied().unwrap_or(0.0)).collect())
        .collect();
    let scale: Vec<f64> = (0..n)
        .map(|j| {
            let spread = diffs.iter().map(|d| d[j].abs()).fold(0.0, f64::max);
            if spread > 1e-9 {
                spread
            } else {
                1.0
            }
        })
        .collect();
    let x: Vec<Vec<f64>> = diffs.iter().map(|d| d.iter().zip(&scale).map(|(v, s)| v / s).collect()).collect();
    let mut v: Vec<f64> = w0.iter().zip(&scale).map(|(w, s)| w * s).collect();
    let v0 = v.clone();
    let lambda = PULL / (v0.iter().map(|a| a * a).sum::<f64>().max(1e-9));

    // Newton's method on: sum of log(1 + e^(-v·x)) + lambda/2 |v - v0|^2.
    for _ in 0..50 {
        let mut grad: Vec<f64> = (0..n).map(|j| lambda * (v[j] - v0[j])).collect();
        let mut hess: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| if i == j { lambda } else { 0.0 }).collect()).collect();
        for xi in &x {
            let margin: f64 = xi.iter().zip(&v).map(|(a, b)| a * b).sum();
            let p = 1.0 / (1.0 + margin.exp());
            for j in 0..n {
                grad[j] -= p * xi[j];
                for k in 0..n {
                    hess[j][k] += p * (1.0 - p) * xi[j] * xi[k];
                }
            }
        }
        let Some(step) = solve(hess, grad) else { break };
        let size: f64 = step.iter().map(|s| s * s).sum::<f64>().sqrt();
        for j in 0..n {
            v[j] -= step[j];
        }
        if size < 1e-9 {
            break;
        }
    }

    let agreement = |w: &[f64]| -> f64 {
        if x.is_empty() {
            return 0.0;
        }
        x.iter().filter(|xi| xi.iter().zip(w).map(|(a, b)| a * b).sum::<f64>() > 0.0).count() as f64 / x.len() as f64
    };
    Learned {
        weights: keys.iter().zip(v.iter().zip(&scale)).map(|(k, (v, s))| ((*k).clone(), v / s)).collect(),
        choices: choices.len(),
        agreement: agreement(&v),
        prior_agreement: agreement(&v0),
    }
}

/// Solves `a · x = b` (Gaussian elimination with partial pivoting).
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..n {
            let f = a[row][col] / a[col][col];
            let (top, bottom) = a.split_at_mut(row);
            for (cell, above) in bottom[0][col..].iter_mut().zip(&top[col][col..]) {
                *cell -= f * above;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let rest: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - rest) / a[row][row];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn no_choices_keep_the_prior() {
        let prior = metrics(&[("interruptions", 1.0), ("cost", -0.3)]);
        let learned = learn_weights(&[], &prior);
        assert!((learned.weights["interruptions"] - 1.0).abs() < 1e-9 && (learned.weights["cost"] + 0.3).abs() < 1e-9);
    }

    #[test]
    fn a_player_who_keeps_cards_in_hand_weighs_them_more() {
        // The prior likes interruptions only; the player always picks the
        // board with more cards left in hand, even with one interruption less.
        let prior = metrics(&[("interruptions", 1.0), ("resources", 0.1)]);
        let choices: Vec<Choice> = (0..12)
            .map(|i| Choice {
                chosen: metrics(&[("interruptions", 2.0 + (i % 3) as f64), ("resources", 3.0)]),
                other: metrics(&[("interruptions", 3.0 + (i % 3) as f64), ("resources", 0.0)]),
            })
            .collect();
        let learned = learn_weights(&choices, &prior);
        assert_eq!(learned.prior_agreement, 0.0);
        assert!(learned.agreement > 0.99, "{learned:?}");
        // Three cards in hand are now worth more than one interruption.
        assert!(3.0 * learned.weights["resources"] > learned.weights["interruptions"], "{learned:?}");
        assert!(learned.weights["interruptions"] > 0.0, "{learned:?}");
    }

    #[test]
    fn solves_a_small_system() {
        let x = solve(vec![vec![2.0, 1.0], vec![1.0, 3.0]], vec![3.0, 5.0]).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-9 && (x[1] - 1.4).abs() < 1e-9, "{x:?}");
    }
}
