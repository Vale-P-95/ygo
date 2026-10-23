#!/usr/bin/env python3
"""The weights of the board score retuned with the next turn too
(docs/plans/ricerca-esperimenti.md, section 7): scripts/tune_weights.py with a
fifth weight, the follow-up's, and a held-out check over many deck splits.

    python3 scripts/research/tune_follow_up.py <results.json of objective_bench.py, or tests/fixtures/boards/objective-bench.json> [--splits 20] [--step 0.1]

The truth and the pairs are those of tune_weights.py (1 - left, meta-weighted;
pairs of one deck whose truths differ by at least --gap). The fifth measure is
bench.follow_up_value (today 0.4 of 0.3 rebuild + 0.1 gained), and its weight
multiplies it: 1 is today's composite, 0 the bench's score. The truth is the
opponent's turn only, so the follow-up can only be checked against it: a
weight above 0 that wins pairs says that boards which rebuild also stop more.

Two checks:
- the bench's own split (taratura/verifica, seed 47 of objective_bench.py):
  the weights tuned on the calibration decks, judged on the check decks;
- --splits random 70/30 splits by deck: each tunes on its 70% and is judged
  on its 30%, against today's weights on the same 30%. The weights that win
  only on the decks they were tuned on show up here.
"""
import argparse
import itertools
import json
import math
import os
import random
import statistics
import sys

import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from bench import WEIGHTS, follow_up_value  # noqa: E402
from tune_weights import METRICS, boards_of, pairs_of, rows_of, spearman  # noqa: E402

TODAY = (*WEIGHTS, 1.0)


def with_follow_up(results, made):
    """The boards of tune_weights.py, plus the follow-up of the generic measure."""
    rows = {}
    for r in rows_of(results):
        if r.get("generic"):
            rows.setdefault(r["board"], r["generic"])
    boards = boards_of(results, made)
    for b in boards:
        b["follow_up"] = follow_up_value(rows[b["board"]])
    return boards


def diffs(pairs):
    keys = (*METRICS, "follow_up")
    return np.array([[a[k] - b[k] for k in keys] for a, b in pairs]).reshape(-1, len(keys))


def right(d, w):
    s = d @ np.asarray(w)
    return float((s > 1e-9).sum() + 0.5 * (np.abs(s) <= 1e-9).sum())


def score(b, w):
    return sum(k * b[m] for k, m in zip(w, (*METRICS, "follow_up")))


def tune(d, grid, follow_grid):
    """Interruptions = 1; the others on the grid. Among the weights within one
    pair of the best, the closest to today's (as tune_weights.py)."""
    base = np.array(list(itertools.product(grid, grid, grid)))
    combos, hits = [], []
    for f in follow_grid:
        c = np.column_stack([np.ones(len(base)), base, np.full(len(base), f)])
        s = d @ c.T
        combos.append(c)
        hits.append((s > 1e-9).sum(axis=0) + 0.5 * (np.abs(s) <= 1e-9).sum(axis=0))
    combos, hits = np.vstack(combos), np.concatenate(hits)
    best = hits.max()
    near = np.where(hits >= best - 1)[0]
    chosen = combos[near[np.abs(combos[near] - np.asarray(TODAY)).sum(axis=1).argmin()]]
    return tuple(float(x) for x in chosen), float(best), len(near)


def judge(boards, gap, w):
    ps = pairs_of(boards, gap)
    if not ps:
        return float("nan"), 0, float("nan")
    return right(diffs(ps), w) / len(ps), len(ps), spearman([score(b, w) for b in boards], [b["truth"] for b in boards])


def compare(boards, gap, tries):
    """Fixed weights against today's on all the pairs: pairs won and lost, by deck."""
    pairs = pairs_of(boards, gap)
    d = diffs(pairs)
    today = d @ np.asarray((*WEIGHTS, 0.0))
    print(f"{len(boards)} board, {len(pairs)} coppie; oggi (1, 0.9, 0.8, 0.6, 0): {right(d, (*WEIGHTS, 0.0)) / len(pairs):.1%}")
    for w in tries:
        s = d @ np.asarray(w)
        won = [p[0]["deck"] for p, x, y in zip(pairs, s, today) if x > 1e-9 and y <= 1e-9]
        lost = [p[0]["deck"] for p, x, y in zip(pairs, s, today) if x <= 1e-9 and y > 1e-9]
        n, k = len(won) + len(lost), min(len(won), len(lost))
        pv = min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n) if n else 1.0
        by = {g: [b for b in boards if b["split"] == g] for g in ("taratura", "verifica")}
        rho = [spearman([score(b, w) for b in by[g]], [b["truth"] for b in by[g]]) for g in by]
        top = sorted(set(won) | set(lost), key=lambda x: -abs(won.count(x) - lost.count(x)))[:4]
        print(f"- {', '.join(f'{x:g}' for x in w)}: {right(d, w) / len(pairs):.1%}; {len(won)} guadagnate, {len(lost)} perse, p = {pv:.3f}; "
              f"ρ {rho[0]:.2f}/{rho[1]:.2f}; " + ", ".join(f"{x} +{won.count(x)}/-{lost.count(x)}" for x in top))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("results")
    p.add_argument("--gap", type=float, default=0.02)
    p.add_argument("--made", type=float, default=0.8)
    p.add_argument("--step", type=float, default=0.1)
    p.add_argument("--top", type=float, default=2.0)
    p.add_argument("--follow-top", type=float, default=3.0, help="largest multiplier of today's follow-up tried")
    p.add_argument("--splits", type=int, default=20)
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--try", dest="tries", action="append", default=[], metavar="W1,W2,W3,W4,W5",
                   help="fixed weights judged on every pair against today's, with the sign test (no tuning)")
    a = p.parse_args()

    boards = with_follow_up(json.load(open(a.results)), a.made)
    if a.tries:
        compare(boards, a.gap, [tuple(float(x) for x in t.split(",")) for t in a.tries])
        return
    grid = np.round(np.arange(0, a.top + a.step / 2, a.step), 4)
    follow_grid = np.round(np.arange(0, a.follow_top + a.step / 2, a.step), 4)
    groups = {g: [b for b in boards if b["split"] == g] for g in ("taratura", "verifica")}
    d_cal = diffs(pairs_of(groups["taratura"], a.gap))
    tuned5, best, ties = tune(d_cal, grid, follow_grid)
    tuned4, _, _ = tune(d_cal, grid, [0.0])

    print(f"{len(boards)} board ({len(groups['taratura'])} di taratura, {len(groups['verifica'])} di verifica), "
          f"{len({b['deck'] for b in boards})} deck; griglia a passo {a.step}")
    print(f"tarati con il turno dopo: {', '.join(f'{x:g}' for x in tuned5)} ({ties} combinazioni a una coppia dal massimo, {best:.1f})")
    print(f"tarati senza: {', '.join(f'{x:g}' for x in tuned4)}\n")
    candidates = [("oggi, senza turno dopo", (*WEIGHTS, 0.0)), ("oggi, con turno dopo", TODAY),
                  ("tarati senza turno dopo", tuned4), ("tarati con turno dopo", tuned5)]
    print("| pesi (interruzioni, handtrap, floodgate, resilienza, turno dopo) | taratura | verifica | ρ taratura | ρ verifica |")
    print("|---|---:|---:|---:|---:|")
    for name, w in candidates:
        (c, nc, rc), (v, nv, rv) = (judge(groups[g], a.gap, w) for g in ("taratura", "verifica"))
        print(f"| {name} ({', '.join(f'{x:g}' for x in w)}) | {c:.1%} di {nc} | {v:.1%} di {nv} | {rc:.2f} | {rv:.2f} |")

    # Held out: random 70/30 splits by deck, coarser grid to keep it quick.
    decks = sorted({b["deck"] for b in boards})
    rng = random.Random(a.seed)
    gains = {"con turno dopo": [], "senza turno dopo": []}
    follow = []
    for _ in range(a.splits):
        held = set(rng.sample(decks, round(0.3 * len(decks))))
        fit = [b for b in boards if b["deck"] not in held]
        test = [b for b in boards if b["deck"] in held]
        d_fit = diffs(pairs_of(fit, a.gap))
        today, _, _ = judge(test, a.gap, (*WEIGHTS, 0.0))
        for name, fg in (("con turno dopo", follow_grid), ("senza turno dopo", [0.0])):
            w, _, _ = tune(d_fit, grid, fg)
            got, _, _ = judge(test, a.gap, w)
            gains[name].append(got - today)
            if fg is follow_grid:
                follow.append(w[-1])
    print(f"\n{a.splits} divisioni 70/30 a caso per deck: coppie giuste sul 30% contro i pesi di oggi senza turno dopo")
    for name, g in gains.items():
        g = [x for x in g if x == x]
        print(f"- tarati {name}: media {statistics.fmean(g):+.1%}, mediana {statistics.median(g):+.1%}, "
              f"meglio in {sum(x > 0 for x in g)}, peggio in {sum(x < 0 for x in g)} di {len(g)}")
    print(f"- peso del turno dopo scelto: mediana {statistics.median(follow):g}, da {min(follow):g} a {max(follow):g}")


if __name__ == "__main__":
    main()
