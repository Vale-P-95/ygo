#!/usr/bin/env python3
"""How the board ranking holds up, from the JSON of scripts/combo_hunt.py
(docs/plans/classificazione-board.md, C5). One folder per seed: the first
is the reference, the others are compared with it.

    python3 scripts/rank_report.py /tmp/json-1 /tmp/json-2 /tmp/json-3

Reports, per deck and in total:
- invariance (C5.2): boards with the same cards in other zones whose
  measures differ by more than 0.05;
- correct zeros (C5.3): cards that must be worth 0 in some boards;
- estimate against measure (C5.7): pairs of the top boards ranked one way
  by the search's estimate and the other way by the probe turn, by more
  than 0.1, and whether the #1 listed is the best measured;
- diversity (C5.8): distinct fields among the boards listed;
- time (C5.9): median and p95 of the measure of one board;
- stability between seeds (C5.6): Jaccard index of the first 5 fields.
"""
import glob
import json
import os
import re
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import WEIGHTS, follow_up_value  # noqa: E402  (one place for the weights of the composite)

ZONE = re.compile(r" \[EMZ\]")


def composite(board):
    m = board.get("measure")
    if not m:
        return None
    return sum(w * m[k] for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience"))) + follow_up_value(m)


def field_key(board):
    """The cards, wherever they are (zones left out)."""
    return ZONE.sub("", board["signature"])


def load(folder):
    runs = {}
    for path in sorted(glob.glob(os.path.join(folder, "*.json"))):
        if path.endswith(".lines.json"):
            continue
        data = json.load(open(path, encoding="utf-8"))
        runs[os.path.basename(path)[:-5]] = data["items"]
    return runs


def zeros(board):
    """Cards counted as interruptions where they cannot stop anything."""
    m = board.get("measure") or {}
    bad = []
    hand_empty = not board.get("hand")
    equipped = any(" -> " in f for f in board.get("field", []))
    for i in m.get("interruptions", []):
        if i["value"] <= 0.0:
            continue
        if i["card"] == "Red-Eyes Dark Dragoon" and hand_empty:
            bad.append(f"Dragoon {i['value']:.2f} con la mano vuota")
        if i["card"] == "Fiendsmith's Desirae" and not equipped:
            bad.append(f"Desirae {i['value']:.2f} senza Link equipaggiato")
    for f in m.get("floodgate_list", []):
        if "Secreterion" in f:
            bad.append("floodgate di Secreterion Dragon")
    return bad


def main():
    folders = sys.argv[1:]
    if not folders:
        print(__doc__)
        return 2
    seeds = [load(f) for f in folders]
    ref = seeds[0]
    totals = dict(decks=0, invariance=0, inv_max=0.0, zeros=0, inversions=0, pairs=0, top_not_best=0, low_diversity=0)
    times = []
    rows = []
    for run, boards in sorted(ref.items()):
        totals["decks"] += 1
        measured = [b for b in boards if composite(b) is not None]
        times += [b["measure"]["seconds"] for b in measured]
        # Invariance: the same cards in other zones measure the same.
        groups = {}
        for b in measured:
            groups.setdefault(field_key(b), []).append(composite(b))
        spread = [max(v) - min(v) for v in groups.values() if len(v) > 1]
        inv = [d for d in spread if d > 0.05]
        totals["invariance"] += len(inv)
        totals["inv_max"] = max([totals["inv_max"]] + spread)
        # Correct zeros.
        z = [x for b in measured for x in zeros(b)]
        totals["zeros"] += len(z)
        # Estimate against measure, over the boards listed (search order).
        inversions = 0
        pairs = 0
        for i in range(len(measured)):
            for j in range(i + 1, len(measured)):
                pairs += 1
                if composite(measured[j]) - composite(measured[i]) > 0.1:
                    inversions += 1
        totals["inversions"] += inversions
        totals["pairs"] += pairs
        best = max(measured, key=composite) if measured else None
        top_not_best = bool(measured) and composite(best) - composite(measured[0]) > 0.05
        totals["top_not_best"] += top_not_best
        # Diversity: distinct fields among the boards listed.
        distinct = len({field_key(b) for b in boards})
        totals["low_diversity"] += distinct <= 2
        # Stability between seeds: Jaccard of the first 5 fields.
        jac = []
        first5 = {field_key(b) for b in boards[:5]}
        for other in seeds[1:]:
            if run in other:
                o = {field_key(b) for b in other[run][:5]}
                jac.append(len(first5 & o) / max(1, len(first5 | o)))
        rows.append((run, len(boards), distinct, len(inv), max(spread, default=0.0), len(z), inversions, pairs, top_not_best,
                     composite(measured[0]) if measured else 0.0, composite(best) if best else 0.0, jac, z))

    print("| deck | board | campi distinti | invarianza (>0,05) | Δ max | zeri sbagliati | inversioni | #1 non il migliore | #1 → migliore | Jaccard top 5 |")
    print("|---|---:|---:|---:|---:|---:|---|---|---|---|")
    for r in rows:
        run, n, distinct, inv, dmax, z, inversions, pairs, tnb, first, best, jac, _ = r
        jtxt = ", ".join(f"{j:.2f}" for j in jac) if jac else "—"
        print(f"| {run} | {n} | {distinct} | {inv} | {dmax:.2f} | {z} | {inversions}/{pairs} | {'sì' if tnb else ''} | {first:.2f} → {best:.2f} | {jtxt} |")
    print()
    t = totals
    print(f"- Deck: {t['decks']}.")
    print(f"- Invarianza: {t['invariance']} gruppi di board con le stesse carte che misurano diverso di più di 0,05; Δ massimo {t['inv_max']:.2f}.")
    print(f"- Zeri sbagliati: {t['zeros']}.")
    for r in rows:
        for x in r[12]:
            print(f"  - {r[0]}: {x}")
    print(f"- Stima contro misura: {t['inversions']} coppie invertite di più di 0,1 su {t['pairs']}; "
          f"in {t['top_not_best']} deck il #1 elencato non è il migliore misurato.")
    print(f"- Diversità: {t['low_diversity']} deck con al massimo 2 campi distinti tra le board elencate.")
    if times:
        times.sort()
        p95 = times[min(len(times) - 1, int(0.95 * len(times)))]
        print(f"- Tempo della misura per board: mediana {statistics.median(times):.2f} s, p95 {p95:.2f} s.")
    jall = [j for r in rows for j in r[11]]
    if jall:
        print(f"- Stabilità tra semi: Jaccard medio delle prime 5 {statistics.mean(jall):.2f} su {len(jall)} confronti.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
