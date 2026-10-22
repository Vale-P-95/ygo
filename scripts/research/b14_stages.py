#!/usr/bin/env python3
"""The measure stages of B14 (docs/plans/ricerca-piano-critico.md), from the
JSON of scripts/combo_hunt.py run with a large --top: how well each stage
ranks the boards the next one will see, and which cuts keep the final #1
and the top 3 (docs/plans/ricerca-esperimenti.md, task 1).

    python3 scripts/research/b14_stages.py /tmp/hunt60 [--level2 /tmp/l2]

Stages, all on the same boards of each hand:
- quick: the search's estimate (`quick_score`);
- unbroken: the probe turn without the board breakers, i.e. the parts of
  the full measure computed before their runs (field, hand, floodgates);
- full: the composite of the full measure, as bench.py and the Combo Lab;
- level2 (optional): files written by `probe-line --vs X --moves --json`,
  named <hand>.<vs>.json, with the `index` of the board.

The final order is the CLI's: full value, ties broken by the search order.
"""
import argparse
import glob
import json
import math
import os
import statistics
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from bench import WEIGHTS  # noqa: E402
from rank_report import composite  # noqa: E402


def unbroken(board):
    m = board.get("measure")
    if not m:
        return None
    return sum(w * m[k] for w, k in zip(WEIGHTS, ("field", "hand", "floodgates")))


def ranks(values):
    """Average ranks (1 = smallest), ties shared."""
    order = sorted(range(len(values)), key=lambda i: values[i])
    out = [0.0] * len(values)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and values[order[j + 1]] == values[order[i]]:
            j += 1
        for k in range(i, j + 1):
            out[order[k]] = (i + j) / 2 + 1
        i = j + 1
    return out


def pearson(x, y):
    if len(x) < 2:
        return None
    mx, my = statistics.fmean(x), statistics.fmean(y)
    sx = math.sqrt(sum((a - mx) ** 2 for a in x))
    sy = math.sqrt(sum((b - my) ** 2 for b in y))
    if sx == 0 or sy == 0:
        return None
    return sum((a - mx) * (b - my) for a, b in zip(x, y)) / (sx * sy)


def spearman(x, y):
    return pearson(ranks(x), ranks(y))


def kendall(x, y):
    """Tau-b: ties in either list count as neither concordant nor discordant."""
    conc = disc = tx = ty = 0
    n = len(x)
    for i in range(n):
        for j in range(i + 1, n):
            a, b = x[i] - x[j], y[i] - y[j]
            if a == 0 and b == 0:
                continue
            if a == 0:
                tx += 1
            elif b == 0:
                ty += 1
            elif (a > 0) == (b > 0):
                conc += 1
            else:
                disc += 1
    den = math.sqrt((conc + disc + tx) * (conc + disc + ty))
    return (conc - disc) / den if den else None


def ndcg(pred, truth, k=3):
    """NDCG@k of the order by `pred` against the gains `truth` (shifted to 0)."""
    low = min(truth)
    gain = [t - low for t in truth]
    order = sorted(range(len(pred)), key=lambda i: (-pred[i], i))
    ideal = sorted(gain, reverse=True)
    dcg = sum(gain[i] / math.log2(r + 2) for r, i in enumerate(order[:k]))
    idcg = sum(g / math.log2(r + 2) for r, g in enumerate(ideal[:k]))
    return dcg / idcg if idcg > 0 else None


def top(values, k):
    """Indices of the k best, ties by position (the search order)."""
    return sorted(range(len(values)), key=lambda i: (-values[i], i))[:k]


STAGES = ["quick", "b6", "unbroken", "full"]


def load(folder, timing=None):
    """Boards measured of each hand, in the search order. With `timing`, the
    measures and times come from a timed probe-line run of the same lines:
    <hand>.json (probe-line --json) and <hand>.err with one `B14TIMING` line
    per board (quick_value=…, quick_s=…, unbroken_s=…, full_s=…)."""
    hands = {}
    for path in sorted(glob.glob(os.path.join(folder, "*.json"))):
        if path.endswith(".lines.json"):
            continue
        name = os.path.basename(path)[:-5]
        data = json.load(open(path, encoding="utf-8"))
        quick = {b["index"]: b.get("quick_score", 0.0) for b in data["items"]}
        items = data["items"]
        times = None
        if timing:
            tpath = os.path.join(timing, name + ".json")
            if not os.path.exists(tpath):
                continue
            items = json.load(open(tpath, encoding="utf-8"))["items"]
            times = [dict(kv.split("=") for kv in line.split()[1:])
                     for line in open(os.path.join(timing, name + ".err"), encoding="utf-8") if line.startswith("B14TIMING")]
            if len(times) != len(items):
                print(f"{name}: {len(times)} times for {len(items)} boards, left out", file=sys.stderr)
                continue
            for b, t in zip(items, times):
                b["_t"] = {k: float(v) for k, v in t.items()}
        boards = [b for b in sorted(items, key=lambda b: b["index"]) if b.get("measure")]
        if len(boards) < 4:
            continue
        h = dict(
            quick=[quick[b["index"]] for b in boards],
            unbroken=[unbroken(b) for b in boards],
            full=[composite(b) for b in boards],
            index=[b["index"] for b in boards],
        )
        if times:
            # A quick value that failed (a run stopped) ranks last.
            h["b6"] = [b["_t"]["quick_value"] if not math.isnan(b["_t"]["quick_value"]) else -1e9 for b in boards]
            # What each stage costs on a board; the full measure after the
            # unbroken one only pays the breakers' runs (same prefix).
            h["cost"] = dict(quick=[0.0] * len(boards), b6=[b["_t"]["quick_s"] for b in boards],
                             # Without the breakers there are no runs with the
                             # opponent answering your first interruption either.
                             unbroken=[b["_t"]["together_s"] for b in boards],
                             full=[b["_t"]["full_s"] for b in boards])
        hands[name] = h
    return hands


def add_cheap(hands, name, folder):
    """A cheaper unbroken stage measured apart (a timed probe-line run with
    fewer windows per response): its unbroken value, and the time up to
    the run with all the responses together."""
    other = load(folder, folder)
    for hand, h in list(hands.items()):
        o = other.get(hand)
        if o is None or o["index"] != h["index"]:
            print(f"{hand}: no {name} stage, left out", file=sys.stderr)
            del hands[hand]
            continue
        h[name] = o["unbroken"]
        h["cost"][name] = o["cost"]["unbroken"]


REF_FIRST = 0


def cascade(h, cuts):
    """cuts: {stage: K}, in the order of STAGES; each stage ranks the boards
    the one before kept and keeps K of them, the full measure ranks the
    last. Returns (#1 identical, #1 of the same value, top-3 overlap, value
    lost by the #1 in %, seconds spent)."""
    kept = list(range(len(h["full"])))
    spent = 0.0
    last = None
    for stage in STAGES:
        if stage not in cuts and stage != "full":
            continue
        if "cost" in h:
            c = h["cost"][stage]
            # The full measure after the unbroken one: only the breakers.
            spent += sum(c[i] - (h["cost"]["unbroken"][i] if stage == "full" and last == "unbroken" else 0.0) for i in kept)
        last = stage
        if stage == "full":
            break
        vals = [h[stage][i] for i in kept]
        kept = [kept[i] for i in top(vals, cuts[stage])]
    full = h["full"]
    # The reference: every board measured, or (--ref-app) the app of today,
    # the full measure on the first boards of the search's estimate.
    pool = top(h["quick"], REF_FIRST) if REF_FIRST else range(len(full))
    final = sorted(pool, key=lambda i: (-full[i], i))
    mine = sorted(kept, key=lambda i: (-full[i], i))
    best = full[final[0]]
    lost = (best - full[mine[0]]) / best * 100 if best > 0 else 0.0
    return mine[0] == final[0], full[mine[0]] >= best - 1e-9, len(set(mine[:3]) & set(final[:3])), lost, spent


def summary(hands, cuts, base):
    res = [cascade(h, cuts) for h in hands.values()]
    n = len(res)
    cells = [f"{sum(r[0] for r in res)}/{n}", f"{sum(r[1] for r in res)}/{n}", f"{sum(r[2] >= 2 for r in res)}/{n}",
             f"{statistics.fmean(r[3] for r in res):.1f}% / {max(r[3] for r in res):.1f}%"]
    if base:
        cells.append(f"{sum(r[4] for r in res) / base:.0%}")
    return cells


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("folder")
    p.add_argument("--timing", help="folder of a timed probe-line run of the same lines (see load)")
    p.add_argument("--cheap", action="append", default=[], metavar="NAME=DIR",
                   help="a cheaper unbroken stage from another timed run (repeatable), ranked between b6 and unbroken")
    p.add_argument("--ref-app", action="store_true",
                   help="compare with the app of today (the full measure on the first --app boards of the estimate), "
                        "not with every board measured; a #1 'of the same value' then means as good or better")
    p.add_argument("--app", type=int, default=30, help="boards the app measures in full today (the time compared with)")
    args = p.parse_args()
    global REF_FIRST
    if args.ref_app:
        REF_FIRST = args.app
    hands = load(args.folder, args.timing)
    timed = args.timing is not None
    for spec in args.cheap:
        name, folder = spec.split("=", 1)
        add_cheap(hands, name, folder)
        STAGES.insert(STAGES.index("unbroken"), name)
    stages = STAGES if timed else ("quick", "unbroken", "full")
    n_boards = [len(h["full"]) for h in hands.values()]
    print(f"{len(hands)} mani, board per mano: mediana {statistics.median(n_boards)}, min {min(n_boards)}, max {max(n_boards)}\n")

    if timed:
        print("Tempo di un gradino su una board (secondi, mediana e 90° percentile su tutte le board):\n")
        for stage in STAGES[1:]:
            xs = sorted(x for h in hands.values() for x in h["cost"][stage])
            print(f"- {stage}: {statistics.median(xs):.3f} s, p90 {xs[int(0.9 * len(xs))]:.3f} s, totale {sum(xs):.0f} s")
        share = sum(x for h in hands.values() for x in h["cost"]["unbroken"]) / sum(x for h in hands.values() for x in h["cost"]["full"])
        print(f"- la misura senza breaker è il {share:.0%} del tempo della misura completa\n")

    print("| coppia di gradini | Spearman (mediana) | Kendall tau-b (mediana) | NDCG@3 (media) | NDCG@3 < 0,9 |")
    print("|---|---:|---:|---:|---:|")
    pairs = list(zip(stages, stages[1:])) + [(s, "full") for s in stages[:-2]]
    for a, b in pairs:
        sp = [s for h in hands.values() if (s := spearman(h[a], h[b])) is not None]
        kt = [s for h in hands.values() if (s := kendall(h[a], h[b])) is not None]
        nd = [s for h in hands.values() if (s := ndcg(h[a], h[b])) is not None]
        print(f"| {a} → {b} | {statistics.median(sp):.2f} | {statistics.median(kt):.2f} | {statistics.fmean(nd):.3f} | {sum(x < 0.9 for x in nd)}/{len(nd)} |")

    print("\nPosizione del #1 finale nell'ordine di ogni gradino (1 = primo):\n")
    for stage in stages[:-1]:
        pos = sorted(top(h[stage], len(h[stage])).index(top(h["full"], 1)[0]) + 1 for h in hands.values())
        print(f"- {stage}: mediana {statistics.median(pos)}, 90° percentile {pos[min(len(pos) - 1, int(0.9 * len(pos)))]}, massimo {pos[-1]}; "
              f"oltre la 10ª {sum(p > 10 for p in pos)}, oltre la 15ª {sum(p > 15 for p in pos)}, oltre la 30ª {sum(p > 30 for p in pos)}")

    base = sum(sum(h["cost"]["full"][: args.app]) for h in hands.values()) if timed else 0.0
    head = "| tagli | #1 identico | #1 di pari valore | top 3 ≥ 2 su 3 | valore perso dal #1 (media / max) |"
    rule = "|---|---:|---:|---:|---:|"
    if timed:
        head += f" tempo rispetto a {args.app} misure complete |"
        rule += "---:|"
    print("\nUn taglio solo (K board per il gradino, poi la misura completa):\n")
    print(head)
    print(rule)
    print("| " + " | ".join([f"full sulle prime {args.app} della ricerca"] + summary(hands, {"quick": args.app}, base)) + " |")
    for stage in stages[:-1]:
        for k in (5, 8, 10, 15, 20, 30):
            print("| " + " | ".join([f"{stage} {k}"] + summary(hands, {stage: k}, base)) + " |")

    print("\nCascate (ogni gradino tiene K board, la misura completa ordina le ultime):\n")
    print(head)
    print(rule)
    combos = []
    for middle in stages[1:-1]:
        for k1 in (20, 30, 40, 60):
            for k2 in (3, 5, 8, 10, 15):
                combos.append({"quick": k1, middle: k2})
    for cuts in combos:
        label = " → ".join(f"{s} {k}" for s, k in cuts.items())
        print("| " + " | ".join([label] + summary(hands, cuts, base)) + " |")


if __name__ == "__main__":
    main()
