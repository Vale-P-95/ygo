//! Statistics of the match log: win rates with Wilson and Bayesian
//! intervals, splits by opponent, turn order, game 1 against post-side
//! games, versions of the deck and tags; Fisher's exact test, the chance
//! that one version is better than another, and how many matches it takes
//! to tell a difference.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sim::{wilson, Rng};

/// One logged match, as the statistics need it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchInput {
    pub opponent: String,
    /// Went first in game 1 (`None` when not recorded).
    pub first: Option<bool>,
    /// Each game: 'W', 'L' or 'D'.
    pub games: Vec<char>,
    /// The match: 'W', 'L' or 'D'.
    pub result: char,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Deck version (hash of the list).
    pub version: String,
    pub version_name: String,
    /// ISO date-time; the input is in chronological order.
    pub played_at: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    /// Win rate over decided matches (wins / (wins + losses)).
    pub p: f64,
    /// 95% Wilson interval.
    pub lo: f64,
    pub hi: f64,
    /// 95% credible interval of a Beta(1 + wins, 1 + losses) posterior.
    pub beta_lo: f64,
    pub beta_hi: f64,
}

impl Record {
    pub fn add(&mut self, outcome: char) {
        match outcome {
            'W' => self.wins += 1,
            'L' => self.losses += 1,
            _ => self.draws += 1,
        }
    }

    pub fn decided(&self) -> u32 {
        self.wins + self.losses
    }

    fn finish(mut self) -> Self {
        let n = self.decided();
        self.p = if n == 0 { 0.0 } else { self.wins as f64 / n as f64 };
        (self.lo, self.hi) = wilson(self.wins as u64, n as u64);
        let (a, b) = (1.0 + self.wins as f64, 1.0 + self.losses as f64);
        self.beta_lo = beta_quantile(0.025, a, b);
        self.beta_hi = beta_quantile(0.975, a, b);
        self
    }
}

fn record(outcomes: impl IntoIterator<Item = char>) -> Record {
    let mut r = Record::default();
    outcomes.into_iter().for_each(|o| r.add(o));
    r.finish()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Named {
    pub name: String,
    pub record: Record,
    /// Share of all matches.
    pub share: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub index: u32,
    pub played_at: String,
    /// Win rate of the last 20 decided matches (fewer at the start).
    pub rolling: f64,
    pub cumulative: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionStats {
    pub version: String,
    pub name: String,
    pub first_played: String,
    pub last_played: String,
    pub record: Record,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    /// Two-sided p-value of Fisher's exact test.
    pub fisher_p: f64,
    /// Posterior probability that the newer version wins more often.
    pub prob_newer_better: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub matches: u32,
    pub total: Record,
    pub first: Record,
    pub second: Record,
    /// Going first against going second.
    pub order_fisher_p: Option<f64>,
    pub game1: Record,
    /// Games 2 and 3, after siding.
    pub post_side: Record,
    pub opponents: Vec<Named>,
    /// Opponent × first / second.
    pub matrix: Vec<(String, Record, Record)>,
    /// Win rate to expect if the next matches meet the opponents of the
    /// last [`META_WINDOW`] (see [`meta_expected`]).
    pub meta_expected: f64,
    pub trend: Vec<TrendPoint>,
    pub versions: Vec<VersionStats>,
    /// The two most recent versions compared, when there are two.
    pub latest_comparison: Option<Comparison>,
    pub tags: Vec<Named>,
}

pub fn report(matches: &[MatchInput]) -> Report {
    let n = matches.len() as u32;
    let total = record(matches.iter().map(|m| m.result));
    let first = record(matches.iter().filter(|m| m.first == Some(true)).map(|m| m.result));
    let second = record(matches.iter().filter(|m| m.first == Some(false)).map(|m| m.result));
    let order_fisher_p =
        (first.decided() > 0 && second.decided() > 0).then(|| fisher_exact(first.wins, first.losses, second.wins, second.losses));
    let game1 = record(matches.iter().filter_map(|m| m.games.first().copied()));
    let post_side = record(matches.iter().flat_map(|m| m.games.iter().skip(1).copied()));

    let mut by_opp: BTreeMap<String, Vec<&MatchInput>> = BTreeMap::new();
    for m in matches {
        let name = if m.opponent.trim().is_empty() { "Sconosciuto".to_owned() } else { m.opponent.trim().to_owned() };
        by_opp.entry(name).or_default().push(m);
    }
    let mut opponents: Vec<Named> = by_opp
        .iter()
        .map(|(name, ms)| Named {
            name: name.clone(),
            record: record(ms.iter().map(|m| m.result)),
            share: ms.len() as f64 / n.max(1) as f64,
        })
        .collect();
    opponents.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.name.cmp(&b.name)));
    let matrix = opponents
        .iter()
        .map(|o| {
            let ms = &by_opp[&o.name];
            (
                o.name.clone(),
                record(ms.iter().filter(|m| m.first == Some(true)).map(|m| m.result)),
                record(ms.iter().filter(|m| m.first == Some(false)).map(|m| m.result)),
            )
        })
        .collect();
    let meta_expected = meta_expected(matches, &total);

    let mut trend = Vec::new();
    let decided: Vec<(&MatchInput, bool)> = matches.iter().filter(|m| m.result != 'D').map(|m| (m, m.result == 'W')).collect();
    let mut wins = 0u32;
    for (i, (m, won)) in decided.iter().enumerate() {
        wins += *won as u32;
        let window = &decided[i.saturating_sub(19)..=i];
        trend.push(TrendPoint {
            index: i as u32 + 1,
            played_at: m.played_at.clone(),
            rolling: window.iter().filter(|(_, w)| *w).count() as f64 / window.len() as f64,
            cumulative: wins as f64 / (i + 1) as f64,
        });
    }

    let mut versions: Vec<VersionStats> = Vec::new();
    for m in matches {
        match versions.iter_mut().find(|v| v.version == m.version) {
            Some(v) => {
                v.record.add(m.result);
                v.last_played = m.played_at.clone();
            }
            None => {
                let mut record = Record::default();
                record.add(m.result);
                versions.push(VersionStats {
                    version: m.version.clone(),
                    name: m.version_name.clone(),
                    first_played: m.played_at.clone(),
                    last_played: m.played_at.clone(),
                    record,
                })
            }
        }
    }
    versions.iter_mut().for_each(|v| v.record = v.record.finish());
    let latest_comparison = match versions.as_slice() {
        [.., older, newer] => Some(compare(&older.record, &newer.record)),
        _ => None,
    };

    let mut by_tag: BTreeMap<String, Vec<char>> = BTreeMap::new();
    for m in matches {
        for t in &m.tags {
            by_tag.entry(t.clone()).or_default().push(m.result);
        }
    }
    let mut tags: Vec<Named> =
        by_tag.into_iter().map(|(name, rs)| Named { share: rs.len() as f64 / n.max(1) as f64, name, record: record(rs) }).collect();
    tags.sort_by(|a, b| b.share.total_cmp(&a.share));

    Report {
        matches: n,
        total,
        first,
        second,
        order_fisher_p,
        game1,
        post_side,
        opponents,
        matrix,
        meta_expected,
        trend,
        versions,
        latest_comparison,
        tags,
    }
}

/// Matches that make up the "recent meta".
pub const META_WINDOW: usize = 30;

/// Win rate expected against the recent meta: each matchup's win rate over
/// every match, weighted by how often that opponent appeared in the last
/// [`META_WINDOW`] matches. A matchup with few matches is pulled toward the
/// overall win rate (as if it had 4 more matches at that rate), so one lucky
/// win does not count as a 100% matchup. Weighting by the whole log would
/// give back the overall win rate exactly.
fn meta_expected(matches: &[MatchInput], total: &Record) -> f64 {
    const PRIOR: f64 = 4.0;
    let overall = if total.decided() > 0 { total.p } else { 0.5 };
    let key = |m: &MatchInput| m.opponent.trim().to_lowercase();
    let recent = &matches[matches.len().saturating_sub(META_WINDOW)..];
    if recent.is_empty() {
        return overall;
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    recent.iter().for_each(|m| *seen.entry(key(m)).or_default() += 1);
    seen.iter()
        .map(|(opponent, count)| {
            let r = record(matches.iter().filter(|m| key(m) == *opponent).map(|m| m.result));
            let p = (r.wins as f64 + PRIOR * overall) / (r.decided() as f64 + PRIOR);
            *count as f64 / recent.len() as f64 * p
        })
        .sum()
}

/// Compares an older and a newer record.
pub fn compare(older: &Record, newer: &Record) -> Comparison {
    Comparison {
        fisher_p: fisher_exact(older.wins, older.losses, newer.wins, newer.losses),
        prob_newer_better: prob_better(newer.wins, newer.losses, older.wins, older.losses, 200_000, 11),
    }
}

fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation (g = 7, n = 9).
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return std::f64::consts::PI.ln() - (std::f64::consts::PI * x).sin().ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + 7.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Continued fraction for the incomplete beta function (Numerical Recipes).
fn beta_cf(x: f64, a: f64, b: f64) -> f64 {
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let tiny = 1e-300;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..300 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        d = if d.abs() < tiny { tiny } else { d };
        c = 1.0 + aa / c;
        c = if c.abs() < tiny { tiny } else { c };
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        d = if d.abs() < tiny { tiny } else { d };
        c = 1.0 + aa / c;
        c = if c.abs() < tiny { tiny } else { c };
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-12 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta I_x(a, b): the Beta(a, b) distribution function.
pub fn beta_cdf(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_cf(x, a, b) / a
    } else {
        1.0 - front * beta_cf(1.0 - x, b, a) / b
    }
}

/// Quantile of Beta(a, b), by bisection.
pub fn beta_quantile(q: f64, a: f64, b: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if beta_cdf(mid, a, b) < q {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn ln_choose(n: u32, k: u32) -> f64 {
    ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0)
}

/// Two-sided Fisher's exact test on the 2×2 table [[a, b], [c, d]].
pub fn fisher_exact(a: u32, b: u32, c: u32, d: u32) -> f64 {
    let (row1, col1, n) = (a + b, a + c, a + b + c + d);
    if n == 0 {
        return 1.0;
    }
    let p = |x: u32| (ln_choose(row1, x) + ln_choose(n - row1, col1 - x) - ln_choose(n, col1)).exp();
    let observed = p(a);
    let (lo, hi) = (col1.saturating_sub(n - row1), row1.min(col1));
    (lo..=hi).map(p).filter(|&q| q <= observed * (1.0 + 1e-7)).sum::<f64>().min(1.0)
}

/// Gamma(shape, 1) sample (Marsaglia–Tsang).
fn gamma(rng: &mut Rng, shape: f64) -> f64 {
    let uniform = |rng: &mut Rng| ((rng.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64;
    if shape < 1.0 {
        return gamma(rng, shape + 1.0) * uniform(rng).powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        // Standard normal by Box–Muller.
        let z = (-2.0 * uniform(rng).ln()).sqrt() * (2.0 * std::f64::consts::PI * uniform(rng)).cos();
        let v = (1.0 + c * z).powi(3);
        if v <= 0.0 {
            continue;
        }
        if uniform(rng).ln() < 0.5 * z * z + d - d * v + d * v.ln() {
            return d * v;
        }
    }
}

/// P(p_x > p_y) for Beta(1 + wins, 1 + losses) posteriors, by sampling.
pub fn prob_better(x_wins: u32, x_losses: u32, y_wins: u32, y_losses: u32, samples: u32, seed: u64) -> f64 {
    let mut rng = Rng::new(seed);
    let mut beta = |w: u32, l: u32| {
        let a = gamma(&mut rng, 1.0 + w as f64);
        let b = gamma(&mut rng, 1.0 + l as f64);
        a / (a + b)
    };
    let better = (0..samples).filter(|_| beta(x_wins, x_losses) > beta(y_wins, y_losses)).count();
    better as f64 / samples as f64
}

/// Matches needed to tell a win rate `p1` from `p0` (two-sided α = 5%,
/// power 80%): against a fixed rate, and per version when comparing two.
pub fn sample_size(p0: f64, p1: f64) -> (u32, u32) {
    let (z, zb) = (1.959_964, 0.841_621);
    let d = (p1 - p0).abs().max(1e-6);
    let one = ((z * (p0 * (1.0 - p0)).sqrt() + zb * (p1 * (1.0 - p1)).sqrt()) / d).powi(2);
    let pbar = (p0 + p1) / 2.0;
    let two = ((z * (2.0 * pbar * (1.0 - pbar)).sqrt() + zb * (p0 * (1.0 - p0) + p1 * (1.0 - p1)).sqrt()) / d).powi(2);
    (one.ceil() as u32, two.ceil() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(opponent: &str, first: bool, games: &str, version: &str) -> MatchInput {
        let games: Vec<char> = games.chars().collect();
        let wins = games.iter().filter(|&&g| g == 'W').count();
        let losses = games.iter().filter(|&&g| g == 'L').count();
        MatchInput {
            opponent: opponent.into(),
            first: Some(first),
            result: if wins > losses {
                'W'
            } else if losses > wins {
                'L'
            } else {
                'D'
            },
            games,
            tags: vec![],
            version: version.into(),
            version_name: version.into(),
            played_at: String::new(),
        }
    }

    #[test]
    fn reference_values() {
        // Beta(1, 1) is uniform; Beta(60, 44) median near 0.578.
        assert!((beta_cdf(0.3, 1.0, 1.0) - 0.3).abs() < 1e-9);
        assert!((beta_quantile(0.5, 60.0, 44.0) - 0.5779).abs() < 2e-3);
        // Fisher's tea tasting: [[3, 1], [1, 3]] → p = 0.4857.
        assert!((fisher_exact(3, 1, 1, 3) - 0.4857).abs() < 1e-3);
        assert!((fisher_exact(10, 0, 0, 10) - 1.0825e-5).abs() < 1e-7);
        assert_eq!(sample_size(0.5, 0.55), (783, 1565));
        assert_eq!(sample_size(0.5, 0.6).0, 194);
        let p = prob_better(70, 30, 50, 50, 100_000, 1);
        assert!(p > 0.99, "{p}");
        assert!((prob_better(20, 20, 20, 20, 100_000, 2) - 0.5).abs() < 0.01);
    }

    #[test]
    fn report_splits_and_versions() {
        let matches = vec![
            m("Snake-Eye", true, "W", "a"),
            m("Snake-Eye", false, "L", "a"),
            m("Yubel", true, "WLW", "a"),
            m("Yubel", false, "LL", "b"),
            m("Snake-Eye", true, "W", "b"),
            m("Tenpai", false, "WW", "b"),
        ];
        let r = report(&matches);
        assert_eq!((r.matches, r.total.wins, r.total.losses), (6, 4, 2));
        assert_eq!((r.first.wins, r.first.losses, r.second.wins, r.second.losses), (3, 0, 1, 2));
        assert_eq!((r.game1.wins, r.game1.losses), (4, 2));
        assert_eq!((r.post_side.wins, r.post_side.losses), (2, 2));
        assert_eq!(r.opponents[0].name, "Snake-Eye");
        assert_eq!(r.versions.len(), 2);
        assert!(r.latest_comparison.is_some());
        assert_eq!(r.trend.len(), 6);
        assert!((r.trend[5].cumulative - 4.0 / 6.0).abs() < 1e-12);
        assert!(r.total.lo < r.total.p && r.total.p < r.total.hi && r.total.beta_lo < r.total.beta_hi);
    }

    #[test]
    fn meta_follows_the_recent_opponents() {
        // 30 old matches against a good matchup, then the meta moves to a bad one.
        let mut matches: Vec<MatchInput> = (0..30).map(|i| m("Yubel", true, if i % 5 == 0 { "L" } else { "W" }, "a")).collect();
        matches.extend((0..30).map(|i| m("Tenpai", true, if i % 3 == 0 { "W" } else { "L" }, "a")));
        let r = report(&matches);
        assert!((r.total.p - 34.0 / 60.0).abs() < 1e-12);
        // Only Tenpai in the last 30: 10 wins in 30, pulled a little toward 34/60.
        let expected = (10.0 + 4.0 * 34.0 / 60.0) / 34.0;
        assert!((r.meta_expected - expected).abs() < 1e-12, "{}", r.meta_expected);
        // A single win does not make a 100% matchup.
        let one = report(&[m("Snake-Eye", true, "L", "a"), m("Exodia", true, "W", "a")]);
        assert!(one.meta_expected < 0.6, "{}", one.meta_expected);
    }
}
