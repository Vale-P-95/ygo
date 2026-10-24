#!/usr/bin/env python3
"""L2-3 as the last step of the ranking, on the fresh decks of the generality
bench (docs/plans/piano-motore-impatto.md, item 1): no new run, only the rows
of tests/fixtures/boards/generality-fresh.json. On each hand's best N boards by
the measure ("Bilanciato" weights), the #1 by L2-3 (level 2 against the 3 decks
with the highest popRank) against the #1 by the measure, both judged on the
truth of the other opponents (meta-weighted 1 - left). Rows with less than 80%
of the opponent's moves made are left out, as in l23_final.py.

    python3 scripts/research/l23_fresh.py [--exclude elfnote_kewl_tune,magistus_fairy_tail]
"""
import argparse, json, statistics

WEIGHTS = (1.0, 0.9, 0.4, 0.0)


def composite(g):
    fu = g.get("follow_up") or {}
    return sum(w * (g.get(k) or 0) for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience"))) + 0.4 * 0.3 * fu.get("rebuild", 0)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--fixture", default="tests/fixtures/boards/generality-fresh.json")
    p.add_argument("--exclude", default=""); p.add_argument("--gap", type=float, default=0.02)
    a = p.parse_args()
    d = json.load(open(a.fixture))
    weight = {o["deck"]: o["weight"] for o in d["meta"]["opponents"]}
    top = [o["deck"] for o in sorted(d["meta"]["opponents"], key=lambda o: -o["pop"])[:3]]
    excl = set(filter(None, a.exclude.split(",")))
    rows = {}
    for r in d["rows"]:
        r = dict(zip(d["columns"], r))
        if r["left"] is not None and (r["made"] or 0) >= 0.8:
            rows.setdefault(r["board"], []).append(r)
    mean = lambda rs: sum(weight[r["opponent"]] * (1 - r["left"]) for r in rs) / sum(weight[r["opponent"]] for r in rs)
    hands = {}
    for bid, rs in rows.items():
        t3 = [r for r in rs if r["opponent"] in top]
        rest = [r for r in rs if r["opponent"] not in top and r["opponent"] not in excl]
        if t3 and rest:
            hands.setdefault(bid.split("#")[0], []).append(dict(measure=composite(d["boards"][bid]["generic"]), l23=mean(t3), truth=mean(rest)))
    for n in (3, 4, 5):
        changed = better = worse = 0
        old, new = [], []
        for bs in hands.values():
            bs = sorted(bs, key=lambda x: -x["measure"])[:n]
            first, pick = bs[0], max(bs, key=lambda x: x["l23"])
            old.append(first["truth"]); new.append(pick["truth"])
            if pick is not first:
                changed += 1
                better += pick["truth"] - first["truth"] > a.gap
                worse += first["truth"] - pick["truth"] > a.gap
        print(f"N={n}: {len(old)} hands, #1 changed {changed}, better {better}, worse {worse}, "
              f"truth of the #1 {statistics.mean(old):.3f} -> {statistics.mean(new):.3f}")
    for key in ("measure", "l23"):
        ok = total = 0
        for bs in hands.values():
            for i, x in enumerate(bs):
                for y in bs[i + 1:]:
                    if abs(x["truth"] - y["truth"]) >= a.gap:
                        total += 1
                        ok += (x[key] - y[key]) * (x["truth"] - y["truth"]) > 0
        print(f"pairs right by {key}: {ok}/{total} = {ok / total:.1%}")


if __name__ == "__main__":
    main()
