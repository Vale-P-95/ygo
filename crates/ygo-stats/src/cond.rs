//! Conditions on a hand: "at least 1 Starter and at most 1 Brick", "Card A
//! and Card B", nested with all/any/not.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Cond {
    All {
        items: Vec<Cond>,
    },
    Any {
        items: Vec<Cond>,
    },
    Not {
        item: Box<Cond>,
    },
    /// How many cards of the hand match `target`, compared with `n`.
    Count {
        target: Target,
        op: Op,
        n: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Target {
    /// Cards with this role (`starter`, `handtrap`, or a custom one).
    Role(String),
    /// Copies of one card, by passcode.
    Card(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Op {
    AtLeast,
    AtMost,
    Exactly,
}

impl Op {
    pub fn test(self, count: u32, n: u32) -> bool {
        match self {
            Op::AtLeast => count >= n,
            Op::AtMost => count <= n,
            Op::Exactly => count == n,
        }
    }
}

impl Cond {
    pub fn role(role: &str, op: Op, n: u32) -> Self {
        Cond::Count { target: Target::Role(role.to_owned()), op, n }
    }

    pub fn card(id: u32, op: Op, n: u32) -> Self {
        Cond::Count { target: Target::Card(id), op, n }
    }

    pub fn all(items: impl IntoIterator<Item = Cond>) -> Self {
        Cond::All { items: items.into_iter().collect() }
    }

    pub fn any(items: impl IntoIterator<Item = Cond>) -> Self {
        Cond::Any { items: items.into_iter().collect() }
    }

    pub fn negate(item: Cond) -> Self {
        Cond::Not { item: Box::new(item) }
    }

    /// Roles and cards the condition looks at.
    pub fn targets(&self, out: &mut BTreeSet<Target>) {
        match self {
            Cond::All { items } | Cond::Any { items } => items.iter().for_each(|c| c.targets(out)),
            Cond::Not { item } => item.targets(out),
            Cond::Count { target, .. } => {
                out.insert(target.clone());
            }
        }
    }

    /// Evaluates the condition, `count` giving how many cards match a target.
    pub fn eval(&self, count: &impl Fn(&Target) -> u32) -> bool {
        match self {
            Cond::All { items } => items.iter().all(|c| c.eval(count)),
            Cond::Any { items } => items.iter().any(|c| c.eval(count)),
            Cond::Not { item } => !item.eval(count),
            Cond::Count { target, op, n } => op.test(count(target), *n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_shape_is_stable() {
        let c = Cond::all([Cond::role("starter", Op::AtLeast, 1), Cond::negate(Cond::card(14558127, Op::Exactly, 0))]);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(
            json,
            r#"{"type":"all","items":[{"type":"count","target":{"kind":"role","value":"starter"},"op":"atLeast","n":1},{"type":"not","item":{"type":"count","target":{"kind":"card","value":14558127},"op":"exactly","n":0}}]}"#
        );
        assert_eq!(serde_json::from_str::<Cond>(&json).unwrap(), c);
    }

    #[test]
    fn evaluates_nested_logic() {
        let c = Cond::all([
            Cond::role("starter", Op::AtLeast, 1),
            Cond::any([Cond::role("starter", Op::AtLeast, 2), Cond::role("extender", Op::AtLeast, 1)]),
        ]);
        let hand = |s: u32, e: u32| {
            move |t: &Target| {
                if *t == Target::Role("starter".into()) {
                    s
                } else if *t == Target::Role("extender".into()) {
                    e
                } else {
                    0
                }
            }
        };
        assert!(c.eval(&hand(2, 0)));
        assert!(c.eval(&hand(1, 1)));
        assert!(!c.eval(&hand(1, 0)));
        assert!(!c.eval(&hand(0, 3)));
    }
}
