#!/usr/bin/env python3
"""L2-3 on the A1b run, the fair test (docs/plans/ricerca-esperimenti.md,
section 6b): level 2 against the three most played decks only, scored against
the truth of the other opponents, so that it is not judged on itself.

    python3 scripts/research/a1b_l23.py <out of objective_bench.py> [--top 3] [--gap 0.02]

From results.json of objective_bench.py: per board,
- L2-k: 1 - `left` against the k decks with the highest popRank, weighted
  among them;
- the truth: 1 - `left` against all the other opponents, meta-weighted;
- the measure: the bench's score (interruptions, handtraps, floodgates,
  resilience with today's weights) and the composite with the next turn
  (bench.py).
Rows with less than 80% of the opponent's moves made on an empty board are
left out, as in the bench. A pair is two boards of the same deck whose
truths differ by at least --gap; an ordering gets it right when it puts them
in the truth's order. Split by the bench's calibration and check decks.
"""
import argparse
import json
import os
import statistics
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from bench import WEIGHTS, follow_up_value  # noqa: E402


def ranks(v):
    order = sorted(range(len(v)), key=lambda i: v[i])
    out = [0.0] * len(v)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and v[order[j + 1]] == v[order[i]]:
            j += 1
        for k in range(i, j + 1):
            out[order[k]] = (i + j) / 2
        i = j + 1
    return out


def spearman(x, y):
    rx, ry = ranks(x), ranks(y)
    mx, my = statistics.fmean(rx), statistics.fmean(ry)
    num = sum((a - mx) * (b - my) for a, b in zip(rx, ry))
    den = (sum((a - mx) ** 2 for a in rx) * sum((b - my) ** 2 for b in ry)) ** 0.5
    return num / den if den else float("nan")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--top", type=int, default=3)
    p.add_argument("--gap", type=float, default=0.02)
    p.add_argument("--made", type=float, default=0.8)
    p.add_argument("--exclude", default="", help="comma-separated opponents left out of the truth too (close relatives of the top decks)")
    a = p.parse_args()
    res = json.load(open(os.path.join(a.out, "results.json")))
    opps = [o for o in res["meta"]["opponents"]]
    top = [o["deck"] for o in sorted(opps, key=lambda o: -o["pop"])[: a.top]]
    excluded = set(filter(None, a.exclude.split(",")))
    by = {}
    for r in res["rows"]:
        if r.get("left") is None or (r.get("made") or 0) < a.made or not r.get("generic"):
            continue
        by.setdefault(r["board"], []).append(r)
    boards = []
    for bid, rs in by.items():
        t3 = [r for r in rs if r["opponent"] in top]
        rest = [r for r in rs if r["opponent"] not in top and r["opponent"] not in excluded]
        if not t3 or not rest:
            continue
        w3, wr = sum(r["weight"] for r in t3), sum(r["weight"] for r in rest)
        g = rs[0]["generic"]
        score = sum(w * (g.get(m) or 0) for w, m in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience")))
        boards.append(dict(board=bid, deck=rs[0]["deck"], split=rs[0]["split"],
                           l23=sum(r["weight"] * (1 - r["left"]) for r in t3) / w3,
                           truth=sum(r["weight"] * (1 - r["left"]) for r in rest) / wr,
                           score=score, composite=score + follow_up_value(g), opponents=len(rs), top_seen=len(t3),
                           seconds=sum(r.get("l2_s") or 0 for r in t3), truth_opponents=len(rest)))
    print(f"L2-{a.top} su {', '.join(top)}; verità sugli altri avversari misurati" + (f", senza {', '.join(sorted(excluded))}" if excluded else ""))
    print(f"{len(boards)} board con L2-{a.top} e verità (mediana avversari per board {statistics.median(b['opponents'] for b in boards)}; "
          f"board con tutti e {a.top} i deck: {sum(b['top_seen'] == a.top for b in boards)}; avversari della verità: mediana "
          f"{statistics.median(b['truth_opponents'] for b in boards)})")
    print(f"tempo del livello 2 contro i {a.top} deck, per board: mediana {statistics.median(b['seconds'] for b in boards):.1f} s, "
          f"90° percentile {sorted(b['seconds'] for b in boards)[int(0.9 * len(boards))]:.1f} s (più la lettura delle linee di ogni deck, una volta)\n")

    orders = [("misura (punteggio del banco)", lambda b: b["score"]), ("misura con il turno dopo", lambda b: b["composite"]),
              (f"L2-{a.top}", lambda b: b["l23"])]
    for k in (1, 2, 4):
        orders.append((f"misura + {k}·L2-{a.top}", lambda b, k=k: b["score"] + k * b["l23"]))
    print("| ordinamento | coppie giuste, taratura | coppie giuste, verifica | tutte | Spearman taratura | Spearman verifica |")
    print("|---|---:|---:|---:|---:|---:|")
    for label, f in orders:
        cells, rhos = [], []
        tot_r = tot_n = 0
        for group in ("taratura", "verifica"):
            bs = [b for b in boards if b["split"] == group]
            right = total = 0
            for d in {b["deck"] for b in bs}:
                ds = [b for b in bs if b["deck"] == d]
                for i, x in enumerate(ds):
                    for y in ds[i + 1:]:
                        if abs(x["truth"] - y["truth"]) < a.gap:
                            continue
                        total += 1
                        right += (f(x) - f(y)) * (x["truth"] - y["truth"]) > 0
            tot_r += right
            tot_n += total
            cells.append(f"{right}/{total} ({right / total:.0%})")
            rhos.append(f"{spearman([f(b) for b in bs], [b['truth'] for b in bs]):.2f}")
        print(f"| {label} | {cells[0]} | {cells[1]} | {tot_r}/{tot_n} ({tot_r / tot_n:.0%}) | {rhos[0]} | {rhos[1]} |")


if __name__ == "__main__":
    main()
