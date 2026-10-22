#!/usr/bin/env python3
"""B9, the first ordering of the staged measure (docs/plans/ricerca-esperimenti.md,
section 5): which cheap ordering of the boards (the search's estimate, the B6
quick value, a cheap measure, alone or combined) puts the board the app picks
today among its first K with the smallest K, and what the ladder then costs.

    python3 scripts/research/b9_order.py <hunt JSON> <timed full> NAME=<timed cheap> ... [--folds 5]

Inputs, for each hand <h>:
- <hunt JSON>/<h>.json: the boards of combo_hunt.py (`quick_score`, `index`);
- <timed full>/<h>.json and <h>.err: a probe-line run of the same lines with
  the timing patch (one `B14TIMING` line per board: quick_value, quick_s,
  together_s, full_s), the full measure and the B6 value;
- each NAME=<dir>: the same for a cheap measure (fewer windows per response,
  no breakers); its value is field + 0.9 hand + 0.8 floodgates, its cost the
  time up to the run with all the responses together.

The reference is the app of today: the full measure on the first 30 boards of
the estimate. A ladder keeps the board it picks when the best full measure
among the K boards it measures is at least as good. Combined orderings are
fitted pairwise (logistic, Bradley-Terry) on the other decks only (folds by
deck), so every hand is ranked by a model that has not seen its deck.
Needs numpy.
"""
import argparse
import glob
import json
import math
import os
import random

import numpy as np

WEIGHTS = (1.0, 0.9, 0.8, 0.6)
FOLLOW_UP = 0.4
REBUILD = 0.3
GAINED = 0.1
APP = 30
# A pick this close to the app's (relative) still counts as the same (--tolerance).
TOLERANCE = 0.0


def full_value(m):
    f = m.get("follow_up") or {}
    parts = (m["field"], m["hand"], m["floodgates"], m["resilience"])
    return sum(w * v for w, v in zip(WEIGHTS, parts)) + FOLLOW_UP * (REBUILD * f.get("rebuild", 0) + GAINED * f.get("gained", 0))


def unbroken(m):
    return m["field"] + 0.9 * m["hand"] + 0.8 * m["floodgates"]


def timed(folder, name):
    """index -> (measure, times) of a timed probe-line run, or None."""
    path = os.path.join(folder, name + ".json")
    if not os.path.exists(path):
        return None
    items = json.load(open(path, encoding="utf-8"))["items"]
    times = [dict((k, float(v)) for k, v in (kv.split("=") for kv in line.split()[1:]))
             for line in open(os.path.join(folder, name + ".err"), encoding="utf-8") if line.startswith("B14TIMING")]
    if len(times) != len(items):
        return None
    return {b["index"]: (b["measure"], t) for b, t in zip(items, times) if b.get("measure")}


def load(hunt, full_dir, cheap):
    hands = {}
    for path in sorted(glob.glob(os.path.join(hunt, "*.json"))):
        if path.endswith(".lines.json"):
            continue
        name = os.path.basename(path)[:-5]
        quick = {b["index"]: b["quick_score"] for b in json.load(open(path, encoding="utf-8"))["items"]}
        full = timed(full_dir, name)
        others = {k: timed(d, name) for k, d in cheap.items()}
        if full is None or any(o is None for o in others.values()):
            continue
        idx = sorted(i for i in full if all(i in o for o in others.values()))
        if len(idx) < 4:
            continue
        q = [quick[i] for i in idx]
        b6 = [full[i][1]["quick_value"] for i in idx]
        h = dict(deck=name.rsplit("-", 1)[0], index=idx, quick=q,
                 b6=[v if not math.isnan(v) else -1e9 for v in b6],
                 full=[full_value(full[i][0]) for i in idx],
                 cost=dict(quick=[0.0] * len(idx), b6=[full[i][1]["quick_s"] for i in idx],
                           full=[full[i][1]["full_s"] for i in idx]))
        for k, o in others.items():
            h[k] = [unbroken(o[i][0]) for i in idx]
            h["cost"][k] = [o[i][1]["together_s"] for i in idx]
        hands[name] = h
    return hands


def order(values):
    return sorted(range(len(values)), key=lambda i: (-values[i], i))


def app_pick(h):
    pool = order(h["quick"])[:APP]
    return max(h["full"][i] for i in pool)


def k_needed(h, score, pool):
    """Smallest K such that the best full measure among the first K of
    `score` (within `pool`) is at least the app's pick."""
    pick = app_pick(h)
    target = pick - max(1e-9, TOLERANCE * abs(pick))
    best = -math.inf
    for k, i in enumerate(sorted(pool, key=lambda i: (-score[i], i)), start=1):
        best = max(best, h["full"][i])
        if best >= target:
            return k
    return math.inf


def fit(hands, names, feats):
    """Pairwise logistic fit, features standardized within each hand (so the
    scale of a deck does not matter), a little L2."""
    rows = []
    for n in names:
        h = hands[n]
        x = np.column_stack([zscore(h[f]) for f in feats])
        full = h["full"]
        for a in range(len(full)):
            for b in range(a + 1, len(full)):
                d = full[a] - full[b]
                if abs(d) > 0.1:
                    rows.append(x[a] - x[b] if d > 0 else x[b] - x[a])
    d = np.array(rows)
    theta = np.zeros(len(feats))
    theta[0] = 1.0
    lam = 1e-3
    for _ in range(100):
        p = 1 / (1 + np.exp(-np.clip(d @ theta, -30, 30)))
        grad = d.T @ (p - 1) / len(d) + 2 * lam * theta
        hess = (d.T * (p * (1 - p))) @ d / len(d) + 2 * lam * np.eye(len(theta))
        step = np.linalg.solve(hess, grad)
        theta -= step
        if np.abs(step).max() < 1e-8:
            break
    return theta


def zscore(v):
    a = np.array(v, dtype=float)
    a = np.where(a < -1e8, np.nan, a)
    m, s = np.nanmean(a), np.nanstd(a)
    z = (a - m) / s if s > 0 else a - m
    return np.nan_to_num(z, nan=-10.0)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("hunt")
    p.add_argument("full")
    p.add_argument("cheap", nargs="*", metavar="NAME=DIR")
    p.add_argument("--folds", type=int, default=5)
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--tolerance", type=float, default=0.0, help="a pick within this share of the app's value counts as kept")
    args = p.parse_args()
    global TOLERANCE
    TOLERANCE = args.tolerance
    cheap = dict(c.split("=", 1) for c in args.cheap)
    hands = load(args.hunt, args.full, cheap)
    decks = sorted({h["deck"] for h in hands.values()})
    random.Random(args.seed).shuffle(decks)
    fold = {d: i % args.folds for i, d in enumerate(decks)}
    names = list(hands)
    print(f"Tolleranza sul #1: {TOLERANCE:.0%}")
    print(f"{len(hands)} mani di {len(decks)} deck, {sum(len(h['full']) for h in hands.values())} board; fold per deck: {args.folds}\n")

    # Orderings: single stages, and linear combinations fitted out of fold.
    rankers = [("stima", ["quick"]), ("B6", ["b6"])] + [(k, [k]) for k in cheap]
    combos = [["quick", "b6"]] + [["quick", k] for k in cheap] + [["quick", "b6", k] for k in cheap]
    scores = {label: {n: hands[n][f[0]] for n in names} for label, f in rankers}
    weights = {}
    for feats in combos:
        label = " + ".join({"quick": "stima", "b6": "B6"}.get(f, f) for f in feats)
        rankers.append((label, feats))
        scores[label] = {}
        ws = []
        for k in range(args.folds):
            train = [n for n in names if fold[hands[n]["deck"]] != k]
            theta = fit(hands, train, feats)
            ws.append(theta)
            for n in names:
                if fold[hands[n]["deck"]] == k:
                    x = np.column_stack([zscore(hands[n][f]) for f in feats])
                    scores[label][n] = list(x @ theta)
        weights[label] = np.mean(ws, axis=0)

    mean_full = np.mean([c for h in hands.values() for c in h["cost"]["full"]])
    base = sum(sum(sorted_first(h["cost"]["full"], order(h["quick"]), APP)) for h in hands.values())

    def cost(feats, pool_size, k, reuse):
        """Seconds: every stage of `feats` on the pool, the full measure on
        the first k; with `reuse` the full measure after the widest cheap
        measure only pays the rest."""
        total = 0.0
        for h in hands.values():
            pool = order(h["quick"])[:pool_size]
            ranked = sorted(pool, key=lambda i: (-score_of[h_name[id(h)]][i], i))[:k]
            for f in feats:
                total += sum(h["cost"][f][i] for i in pool)
            widest = max((f for f in feats if f in cheap), key=lambda f: sum(h["cost"][f]), default=None)
            for i in ranked:
                total += h["cost"]["full"][i] - (h["cost"][widest][i] if reuse and widest else 0.0)
        return total / base

    h_name = {id(h): n for n, h in hands.items()}
    print("| ordinamento | board ordinate | K per 82/82 | K per 80/82 | K per 78/82 | tempo con K per 82/82 | con riuso della misura economica |")
    print("|---|---:|---:|---:|---:|---:|---:|")
    best_rows = []
    for label, feats in rankers:
        score_of = {n: scores[label][n] for n in names}
        for pool_size in (30, 40, 60):
            ks = sorted(k_needed(hands[n], score_of[n], order(hands[n]["quick"])[:pool_size]) for n in names)
            if ks[-1] == math.inf:
                continue
            k100, k80, k78 = ks[-1], ks[len(ks) - 3], ks[len(ks) - 5]
            c = cost(feats, pool_size, k100, False)
            cr = cost(feats, pool_size, k100, True)
            best_rows.append((cr, label, pool_size, k100))
            print(f"| {label} | {pool_size} | {k100} | {k80} | {k78} | {c:.0%} | {cr:.0%} |")
    # The other way round: at a time budget, how many hands keep the pick.
    print(f"\nCon il tempo fissato (con riuso), le mani che tengono il #1 di oggi (miglior K e insieme di board ordinate):\n")
    print("| ordinamento | ≤ 45% | ≤ 55% | ≤ 65% |")
    print("|---|---:|---:|---:|")
    for label, feats in rankers:
        score_of = {n: scores[label][n] for n in names}
        cells = []
        for budget in (0.45, 0.55, 0.65):
            best = (0, None)
            for pool_size in (20, 30, 40):
                for k in range(1, pool_size + 1):
                    if cost(feats, pool_size, k, True) > budget:
                        break
                    kept = sum(k_needed(hands[n], score_of[n], order(hands[n]["quick"])[:pool_size]) <= k for n in names)
                    if kept > best[0]:
                        best = (kept, f"{pool_size}→{k}")
            cells.append(f"{best[0]}/{len(names)} ({best[1]})" if best[1] else "—")
        print(f"| {label} | " + " | ".join(cells) + " |")
    print("\nPesi medi delle combinazioni (caratteristiche standardizzate dentro la mano):\n")
    for label, w in weights.items():
        print(f"- {label}: " + ", ".join(f"{x:+.2f}" for x in w))
    print(f"\nRiferimento: misura completa sulle prime {APP} della stima = 100% ({base:.0f} s; media {mean_full:.3f} s a board).")
    print("Il più economico con 82/82: " + ", ".join(f"{l} su {p} → K={k} ({c:.0%})" for c, l, p, k in sorted(best_rows)[:5]))


def sorted_first(costs, ranked, k):
    return [costs[i] for i in ranked[:k]]


if __name__ == "__main__":
    main()
