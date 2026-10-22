#!/usr/bin/env python3
"""Paired test to accept a change to the combo search (docs/plans/
ricerca-piano-critico.md, A1): the same hands with the same seed and the
same node budget, the old build against the new one.

The hands: the bench's and those of `combo_hunt.py --seed 7` (see
`oracle.py hands`). For each hand, the value of the best board measured
(the bench's value): better, worse or even, with a 1% threshold. The change
is accepted when the worse hands are not more than the better ones (sign
test) and no hand loses more than 5%.

    python3 scripts/paired.py /tmp/paired --decks /tmp/mdm --old /tmp/cli-main --new target/release/duelforge-cli
    python3 scripts/paired.py /tmp/paired --decks /tmp/mdm --old ... --new ... --skip-run   # only the table again

Runs already written in <out>/old and <out>/new are kept.
"""
import argparse
import concurrent.futures as cf
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oracle import boards_of, hands, run  # noqa: E402

# A smaller difference is even; a hand losing more than this blocks the change.
EVEN = 0.01
MAX_LOSS = 0.05


def sign_test(wins, losses):
    """Two-sided p-value of `wins` against `losses` (the even ones left out)."""
    n = wins + losses
    if n == 0:
        return 1.0
    k = min(wins, losses)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2**n)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--decks", required=True)
    p.add_argument("--old", required=True, help="duelforge-cli before the change")
    p.add_argument("--new", required=True, help="duelforge-cli after the change")
    p.add_argument("--nodes", type=int, default=4000)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--only", default="")
    p.add_argument("--skip-run", action="store_true")
    a = p.parse_args()
    cases = [c for c in hands(a.decks) if not c[4] and a.only in c[0]]
    for side in ("old", "new"):
        os.makedirs(os.path.join(a.out, side), exist_ok=True)

    def one(job):
        side, case = job
        path = os.path.join(a.out, side, case[0] + ".json")
        if a.skip_run or os.path.exists(path):
            return
        secs = run(getattr(a, side), case, a.nodes, path + ".part")
        if secs is not None:
            os.replace(path + ".part", path)
            json.dump({"seconds": secs}, open(path[:-5] + ".time.json", "w"))

    with cf.ThreadPoolExecutor(a.jobs) as ex:
        list(ex.map(one, [(side, c) for c in cases for side in ("old", "new")]))

    def best(side, case):
        path = os.path.join(a.out, side, case[0] + ".json")
        if not os.path.exists(path):
            return None, None
        found = boards_of(path)
        secs = json.load(open(path[:-5] + ".time.json"))["seconds"] if os.path.exists(path[:-5] + ".time.json") else None
        return (found[0][0] if found else 0.0), secs

    rows = []
    for c in cases:
        (old, t_old), (new, t_new) = best("old", c), best("new", c)
        if old is None or new is None:
            continue
        change = (new - old) / old if old > 0 else (0.0 if new == old else math.inf)
        verdict = "better" if change > EVEN else "worse" if change < -EVEN else "even"
        rows.append((c[0], old, new, change, verdict, t_old, t_new))
    print(f"{'hand':34} {'old':>7} {'new':>7} {'change':>8}  verdict   s old   s new")
    for name, old, new, change, verdict, t_old, t_new in rows:
        secs = lambda t: f"{t:>7.1f}" if t is not None else f"{'-':>7}"
        print(f"{name:34} {old:>7.2f} {new:>7.2f} {change:>+8.1%}  {verdict:7} {secs(t_old)} {secs(t_new)}")
    wins = sum(r[4] == "better" for r in rows)
    losses = sum(r[4] == "worse" for r in rows)
    worst = min((r[3] for r in rows), default=0.0)
    accepted = losses <= wins and worst >= -MAX_LOSS
    total = lambda i: sum(r[i] for r in rows if r[i] is not None)
    print(f"\n{len(rows)} hands: {wins} better, {losses} worse, {len(rows) - wins - losses} even "
          f"(sign test p = {sign_test(wins, losses):.3f}); worst {worst:+.1%}; "
          f"search seconds {total(5):.0f} -> {total(6):.0f}")
    print("ACCEPTED" if accepted else "REJECTED" + (" (a hand loses more than 5%)" if worst < -MAX_LOSS else " (more hands worse than better)"))
    json.dump([dict(zip(("hand", "old", "new", "change", "verdict", "seconds_old", "seconds_new"), r)) for r in rows],
              open(os.path.join(a.out, "paired.json"), "w"), indent=1)
    return 0 if accepted else 1


if __name__ == "__main__":
    sys.exit(main())
