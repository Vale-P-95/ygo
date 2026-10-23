#!/usr/bin/env python3
"""L2-3 as the last step of the ranking (docs/plans/velocita-ricerca.md,
"L2-3"): on each hand's best N boards by the measure, the #1 by L2-3 (level 2
against the K decks with the highest popRank) against the #1 by the measure,
both judged on the truth of the other opponents (meta-weighted 1 - left), as
in scripts/research/a1b_l23.py. Rows with less than 80% of the opponent's
moves made on an empty board are left out.

    python3 scripts/research/l23_final.py <out of objective_bench.py> [--top N] [--k 3] [--exclude a,b] [--list]
"""
import argparse, json, os, statistics, sys
WEIGHTS = (1.0, 0.9, 0.8, 0.6)

def composite(g):
    fu = g.get("follow_up") or {}
    return sum(w * (g.get(k) or 0) for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience"))) + 0.4 * 0.3 * fu.get("rebuild", 0)

def main():
    p = argparse.ArgumentParser()
    p.add_argument("out"); p.add_argument("--top", type=int, default=5); p.add_argument("--k", type=int, default=3)
    p.add_argument("--made", type=float, default=0.8); p.add_argument("--exclude", default=""); p.add_argument("--gap", type=float, default=0.02)
    p.add_argument("--list", action="store_true")
    a = p.parse_args()
    res = json.load(open(os.path.join(a.out, "results.json")))
    opps = res["meta"]["opponents"]
    top = [o["deck"] for o in sorted(opps, key=lambda o: -o["pop"])[: a.k]]
    excl = set(filter(None, a.exclude.split(",")))
    by = {}
    for r in res["rows"]:
        if r.get("left") is None or (r.get("made") or 0) < a.made or not r.get("generic"):
            continue
        by.setdefault(r["board"], []).append(r)
    hands = {}
    for bid, rs in by.items():
        t3 = [r for r in rs if r["opponent"] in top]
        rest = [r for r in rs if r["opponent"] not in top and r["opponent"] not in excl]
        if not rest:
            continue
        b = dict(board=bid, measure=composite(rs[0]["generic"]),
                 l23=(sum(r["weight"] * (1 - r["left"]) for r in t3) / sum(r["weight"] for r in t3)) if t3 else None,
                 truth=sum(r["weight"] * (1 - r["left"]) for r in rest) / sum(r["weight"] for r in rest))
        hands.setdefault(bid.split("#")[0], []).append(b)
    changed = better = worse = even = 0; deltas = []; tb = []; ta = []; best_b = best_a = 0; n = 0
    for h, bs in sorted(hands.items()):
        bs = sorted(bs, key=lambda b: -b["measure"])[: a.top]
        if len(bs) < 2:
            continue
        n += 1
        m1 = bs[0]
        l1 = max(bs, key=lambda b: (b["l23"] if b["l23"] is not None else -1, -bs.index(b)))
        tb.append(m1["truth"]); ta.append(l1["truth"])
        tbest = max(b["truth"] for b in bs)
        best_b += m1["truth"] >= tbest - 1e-9; best_a += l1["truth"] >= tbest - 1e-9
        if l1 is not m1:
            changed += 1; d = l1["truth"] - m1["truth"]; deltas.append(d)
            better += d >= a.gap; worse += d <= -a.gap; even += abs(d) < a.gap
            if a.list:
                print(f"  {h}: #{bs.index(l1)+1} by measure -> #1, truth {m1['truth']:.3f} -> {l1['truth']:.3f} ({d:+.3f})")
    print(f"top {a.top}, {n} hands: #1 changed in {changed}; of those better {better}, even {even}, worse {worse} (gap {a.gap})")
    if deltas:
        print(f"  truth change where it changed: mean {statistics.fmean(deltas):+.3f}, median {statistics.median(deltas):+.3f}, min {min(deltas):+.3f}, max {max(deltas):+.3f}")
    print(f"  mean truth of the #1: measure {statistics.fmean(tb):.3f} -> L2-3 {statistics.fmean(ta):.3f}")
    print(f"  #1 is the truth-best of the {a.top}: measure {best_b}/{n}, L2-3 {best_a}/{n}")

main()
