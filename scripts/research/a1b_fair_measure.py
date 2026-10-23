#!/usr/bin/env python3
"""A measure remeasured on the boards of the A1b run, judged against the
level 2 truth of that run (docs/plans/ricerca-esperimenti.md, section 6c): the
fair test of a change to the probe turn, on every board and not only on the
disagreements (scripts/research/a1b_remeasure.py does those).

    python3 scripts/research/a1b_fair_measure.py <out of objective_bench.py> NAME=<dir> [NAME=<dir> ...] [--gap 0.02] [--top 3]

Each <dir> holds, per lines file of the run, <hand_file>.json written by
`probe-line --deck ... --lines <out>/lines/<hand_file>.lines.json --json ...`
(the measure only, no --vs). The truth is 1 - `left`, meta-weighted, over
all the opponents measured, and over all of them but the --top most played
(the L2-3 test of section 6b). Pairs: boards of the same deck whose truths
differ by at least --gap, split into the bench's calibration and check decks.
The measure is the bench's score (interruptions, handtraps, floodgates,
resilience with today's weights) and the composite with the next turn.
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import WEIGHTS, follow_up_value  # noqa: E402
from a1b_l23 import spearman  # noqa: E402


def score_of(m):
    return sum(w * (m.get(k) or 0) for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience")))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("measures", nargs="+", metavar="NAME=DIR")
    p.add_argument("--gap", type=float, default=0.02)
    p.add_argument("--made", type=float, default=0.8)
    p.add_argument("--top", type=int, default=3)
    a = p.parse_args()
    res = json.load(open(os.path.join(a.out, "results.json")))
    boards = {b["id"]: b for b in json.load(open(os.path.join(a.out, "boards.json")))}
    top = {o["deck"] for o in sorted(res["meta"]["opponents"], key=lambda o: -o["pop"])[: a.top]}
    rows = {}
    for r in res["rows"]:
        if r.get("left") is None or (r.get("made") or 0) < a.made:
            continue
        rows.setdefault(r["board"], []).append(r)

    def truth(rs):
        w = sum(r["weight"] for r in rs)
        return sum(r["weight"] * (1 - r["left"]) for r in rs) / w if w else None

    base = {}
    for bid, rs in rows.items():
        rest = [r for r in rs if r["opponent"] not in top]
        if not rest or bid not in boards:
            continue
        g = rs[0].get("generic") or {}
        base[bid] = dict(deck=rs[0]["deck"], split=rs[0]["split"], all=truth(rs), rest=truth(rest),
                         m={"run A1b": (score_of(g), score_of(g) + follow_up_value(g))})
    for spec in a.measures:
        name, folder = spec.split("=", 1)
        cache = {}
        for bid, b in base.items():
            hand_file, index = boards[bid]["hand_file"], boards[bid]["index"]
            if hand_file not in cache:
                path = os.path.join(folder, hand_file + ".json")
                cache[hand_file] = json.load(open(path))["items"] if os.path.exists(path) else []
            items = cache[hand_file]
            m = items[index].get("measure") if index < len(items) else None
            if m:
                b["m"][name] = (score_of(m), score_of(m) + follow_up_value(m))
    names = ["run A1b"] + [s.split("=", 1)[0] for s in a.measures]
    common = [b for b in base.values() if all(n in b["m"] for n in names)]
    print(f"{len(common)} board misurate in tutte le configurazioni, su {len(base)} con la verità\n")

    for truth_key, title in (("all", "verità su tutti gli avversari"), ("rest", f"verità senza i {a.top} deck più giocati (come L2-{a.top})")):
        print(f"### {title}\n")
        print("| misura | turno dopo | coppie giuste, taratura | coppie giuste, verifica | tutte | Spearman taratura | Spearman verifica |")
        print("|---|---|---:|---:|---:|---:|---:|")
        for n in names:
            for which, label in ((0, "no"), (1, "sì")):
                cells, rhos, tr, tn = [], [], 0, 0
                for group in ("taratura", "verifica"):
                    bs = [b for b in common if b["split"] == group]
                    right = total = 0
                    for d in {b["deck"] for b in bs}:
                        ds = [b for b in bs if b["deck"] == d]
                        for i, x in enumerate(ds):
                            for y in ds[i + 1:]:
                                if abs(x[truth_key] - y[truth_key]) < a.gap:
                                    continue
                                total += 1
                                right += (x["m"][n][which] - y["m"][n][which]) * (x[truth_key] - y[truth_key]) > 0
                    tr += right
                    tn += total
                    cells.append(f"{right}/{total} ({right / total:.0%})")
                    rhos.append(f"{spearman([b['m'][n][which] for b in bs], [b[truth_key] for b in bs]):.2f}")
                print(f"| {n} | {label} | {cells[0]} | {cells[1]} | {tr}/{tn} ({tr / tn:.0%}) | {rhos[0]} | {rhos[1]} |")
        print()


if __name__ == "__main__":
    main()
