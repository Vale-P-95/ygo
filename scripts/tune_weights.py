#!/usr/bin/env python3
"""The weights of the board score tuned on the objective bench
(docs/plans/classificazione-board.md, D17; docs/plans/banco-oggettivo.md).

    python3 scripts/tune_weights.py $S/bench/results.json
    python3 scripts/tune_weights.py $S/bench/results.json --owner $S/owner/results.json

The truth of a board is the share of the meta's moves it stops (1 - left of
probe-line --vs X --moves), weighted by how much each opponent is played,
without the doubtful pairs (fewer than 80% of the opponent's moves made on an
empty field). It is the opponent's turn only, so it can tune the weights of
what stops that turn: interruptions, handtraps, floodgates and resilience.
The follow-up, the resources, the cost and the exposure are about your next
turn or about making the board at all: the bench says nothing of them, and
their weights stay as they are.

The weight of the interruptions stays 1 (the scale of the score); the other
three are searched on a grid. A pair of boards of the same deck counts when
their truths differ by at least --gap; a set of weights gets it right when
it orders the two boards the same way. The weights are chosen on the
calibration decks and checked on the others; among the weights within one
pair of the best, the closest to today's are kept (no change without a
reason).
"""
import argparse
import itertools
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import WEIGHTS  # noqa: E402  (today's "Bilanciato" weights of field, hand, floodgates, resilience)

METRICS = ("field", "hand", "floodgates", "resilience")
LABELS = {"field": "interruzioni", "hand": "handtrap", "floodgates": "floodgate", "resilience": "resilienza"}


def boards_of(results, made_min):
    """One entry per board: the meta-weighted truth and the generic measures."""
    by = {}
    for r in results["rows"]:
        if "left" not in r or r.get("made", 0) < made_min or not r.get("generic"):
            continue
        by.setdefault(r["board"], []).append(r)
    out = []
    for board, rs in by.items():
        w = sum(r["weight"] for r in rs)
        g = rs[0]["generic"]
        out.append(dict(board=board, deck=rs[0]["deck"], split=rs[0].get("split", "taratura"), coverage=w,
                        truth=sum(r["weight"] * (1 - r["left"]) for r in rs) / w,
                        **{m: g.get(m) or 0.0 for m in METRICS}, countered=g.get("countered")))
    return out


def pairs_of(boards, gap):
    """Pairs of boards of one deck whose truths differ enough: (better, worse)."""
    out = []
    for deck in sorted({b["deck"] for b in boards}):
        ds = [b for b in boards if b["deck"] == deck]
        for i, x in enumerate(ds):
            for y in ds[i + 1:]:
                if abs(x["truth"] - y["truth"]) >= gap:
                    out.append((x, y) if x["truth"] > y["truth"] else (y, x))
    return out


def diffs(pairs):
    """The measures of the better board minus those of the worse, one row per pair."""
    return np.array([[a[m] - b[m] for m in METRICS] for a, b in pairs]).reshape(-1, len(METRICS))


def right(d, w):
    """Pairs ordered as the truth by the weights w (a tie counts half)."""
    s = d @ np.asarray(w)
    return float((s > 1e-9).sum() + 0.5 * (np.abs(s) <= 1e-9).sum())


def spearman(xs, ys):
    def ranks(v):
        v = np.asarray(v, dtype=float)
        order = v.argsort(kind="stable")
        r = np.empty(len(v))
        i = 0
        while i < len(v):
            j = i
            while j + 1 < len(v) and v[order[j + 1]] == v[order[i]]:
                j += 1
            r[order[i:j + 1]] = (i + j) / 2
            i = j + 1
        return r
    if len(xs) < 3:
        return float("nan")
    rx, ry = ranks(xs), ranks(ys)
    sx, sy = rx.std(), ry.std()
    return float(((rx - rx.mean()) * (ry - ry.mean())).mean() / (sx * sy)) if sx and sy else float("nan")


def score(b, w):
    return sum(k * b[m] for k, m in zip(w, METRICS))


def tune(d, step, top):
    """Grid over handtraps, floodgates and resilience (interruptions = 1): the
    weights within one pair of the best, the closest to today's first."""
    grid = np.round(np.arange(0, top + step / 2, step), 4)
    combos = np.array([(1.0, *c) for c in itertools.product(grid, grid, grid)])
    s = d @ combos.T
    hits = (s > 1e-9).sum(axis=0) + 0.5 * (np.abs(s) <= 1e-9).sum(axis=0)
    best = hits.max()
    near = np.where(hits >= best - 1)[0]
    distance = np.abs(combos[near] - np.asarray(WEIGHTS)).sum(axis=1)
    chosen = combos[near[distance.argmin()]]
    return tuple(float(x) for x in chosen), float(best), len(near)


def owner_check(path, weights_list, gap):
    """How often the weights agree with the owner's verdicts, on the pairs of
    owner-pairs.json measured by owner_bench.py (verdict A, B or pari)."""
    data = json.load(open(path))
    lines = []
    for name, w in weights_list:
        agree = total = 0
        for p in data.get("pairs", []):
            a, b, v = p.get("a"), p.get("b"), p.get("verdict")
            if not a or not b or v not in ("A", "B"):
                continue
            delta = score(b, w) - score(a, w)
            total += 1
            agree += (delta > gap and v == "B") or (delta < -gap and v == "A")
        lines.append(f"| {name} | {agree} su {total} |")
    return ["", "## Verdetti del proprietario", "", "| Pesi | D'accordo |", "|---|---|"] + lines


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("results", help="results.json written by objective_bench.py")
    p.add_argument("--gap", type=float, default=0.02, help="truths closer than this make no pair")
    p.add_argument("--made", type=float, default=0.8, help="pairs with fewer moves made on an empty field are doubtful")
    p.add_argument("--step", type=float, default=0.05)
    p.add_argument("--top", type=float, default=2.0, help="largest weight tried")
    p.add_argument("--owner", help="owner_bench.py results: pairs with a, b (generic measures) and the owner's verdict")
    args = p.parse_args()

    boards = boards_of(json.load(open(args.results)), args.made)
    groups = {g: [b for b in boards if b["split"] == g] for g in ("taratura", "verifica")}
    pairs = {g: pairs_of(bs, args.gap) for g, bs in groups.items()}
    d = {g: diffs(ps) for g, ps in pairs.items()}
    tuned, best, ties = tune(d["taratura"], args.step, args.top)

    candidates = [("oggi", tuple(WEIGHTS)), ("tarati", tuned)]
    candidates += [(f"solo {LABELS[m]}", tuple(1.0 if k == i else 0.0 for k in range(len(METRICS)))) for i, m in enumerate(METRICS)]
    out = ["# Pesi tarati sul banco oggettivo", "",
           f"- board: {len(boards)} ({len(groups['taratura'])} di taratura, {len(groups['verifica'])} di verifica); "
           f"coppie dello stesso deck con verità diverse di almeno {args.gap}: {len(pairs['taratura'])} e {len(pairs['verifica'])}",
           f"- pesi tarati (interruzioni, handtrap, floodgate, resilienza): {', '.join(f'{x:.2f}' for x in tuned)}; "
           f"oggi {', '.join(f'{x:.2f}' for x in WEIGHTS)}; {ties} combinazioni a una coppia dal massimo ({best:.1f})",
           "- follow-up, risorse, costo ed esposizione non si tarano qui: la verità è solo il turno avversario.", "",
           "| Pesi | Taratura: coppie giuste | Verifica: coppie giuste | ρ verità (taratura) | ρ verità (verifica) |",
           "|---|---|---|---|---|"]
    for name, w in candidates:
        cells = []
        for g in ("taratura", "verifica"):
            n = len(pairs[g])
            cells.append(f"{right(d[g], w):.1f} su {n} ({right(d[g], w) / n:.0%})" if n else "—")
        rho = [spearman([score(b, w) for b in groups[g]], [b["truth"] for b in groups[g]]) for g in ("taratura", "verifica")]
        out.append(f"| {name} ({', '.join(f'{x:g}' for x in w)}) | {cells[0]} | {cells[1]} | {rho[0]:.2f} | {rho[1]:.2f} |")
    with_countered = [b for b in boards if b["countered"] is not None]
    if with_countered:
        out += ["", f"- `countered` (solo da mostrare): ρ con la verità {spearman([b['countered'] for b in with_countered], [b['truth'] for b in with_countered]):.2f} "
                f"su {len(with_countered)} board"]
    if args.owner:
        out += owner_check(args.owner, candidates[:2], 0.05)
    print("\n".join(out))


if __name__ == "__main__":
    main()
