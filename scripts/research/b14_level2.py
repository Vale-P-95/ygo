#!/usr/bin/env python3
"""The last stage of B14: the probe turn against a deck's own lines (level
2, `probe-line --vs X --moves`) on the boards the full measure ranks first
(docs/plans/ricerca-esperimenti.md, task 1).

    python3 scripts/research/b14_level2.py <full JSON folder> <level 2 folder>

The level 2 folder holds, for each hand, <hand>.lines.json (the lines put
through level 2, with `b14_index`: their index in the search) and
<hand>.out (the output of probe-line). The full measures come from the
JSON of the same boards (combo_hunt.py or a probe-line run). Level 2 ranks
by `left` (share of the opponent's moves still made with all your
answers, lower is better), ties by its `value`.
"""
import glob
import json
import os
import re
import statistics
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from rank_report import composite  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from b14_stages import kendall, spearman  # noqa: E402

LINE = re.compile(r"against its lines \(([\d.]+) s\): (\d+) lines · (\d+)% of their moves made on an empty board · value ([\d.]+) · "
                  r"floodgates stop (\d+)% · (\d+)% of the rest left")


def main():
    full_dir, l2_dir = sys.argv[1], sys.argv[2]
    rows = []
    for path in sorted(glob.glob(os.path.join(l2_dir, "*.lines.json"))):
        name = os.path.basename(path)[: -len(".lines.json")]
        order = json.load(open(path, encoding="utf-8"))["b14_index"]
        out = open(os.path.join(l2_dir, name + ".out"), encoding="utf-8").read()
        found = LINE.findall(out)
        if len(found) != len(order):
            print(f"{name}: {len(found)} level 2 results for {len(order)} boards, left out", file=sys.stderr)
            continue
        boards = {b["index"]: b for b in json.load(open(os.path.join(full_dir, name + ".json"), encoding="utf-8"))["items"]}
        full = [composite(boards[i]) for i in order]
        l2 = [(-int(f[5]), float(f[3])) for f in found]
        seconds = [float(f[0]) for f in found]
        rows.append((name, full, l2, seconds))
    print(f"{len(rows)} mani, {sum(len(r[1]) for r in rows)} board al livello 2\n")
    sp = [s for _, f, l, _ in rows if (s := spearman(f, [x[0] + x[1] / 100 for x in l])) is not None]
    kt = [s for _, f, l, _ in rows if (s := kendall(f, [x[0] + x[1] / 100 for x in l])) is not None]
    print(f"- misura completa → livello 2, sulle prime 8 della misura: Spearman mediana {statistics.median(sp):.2f}, "
          f"Kendall mediana {statistics.median(kt):.2f} ({len(sp)} mani con ordine non costante)")
    secs = sorted(s for r in rows for s in r[3])
    print(f"- tempo del livello 2 per board: mediana {statistics.median(secs):.1f} s, p90 {secs[int(0.9 * len(secs))]:.1f} s")
    # Where the level 2 #1 sits in the full measure's order (1 = first).
    pos = []
    for _, full, l2, _ in rows:
        best = max(range(len(l2)), key=lambda i: (l2[i], -i))
        if l2[best] == min(l2) and len(set(l2)) == 1:
            continue
        rank = sorted(range(len(full)), key=lambda i: (-full[i], i)).index(best) + 1
        pos.append(rank)
    pos.sort()
    print(f"- posizione nella misura completa del #1 del livello 2 ({len(pos)} mani dove il livello 2 distingue): "
          + ", ".join(f"≤{k}: {sum(p <= k for p in pos)}" for k in (1, 2, 3, 4, 6, 8)))
    same = sum(1 for _, _, l2, _ in rows if len(set(l2)) == 1)
    print(f"- mani dove il livello 2 dà lo stesso valore a tutte le 8 board: {same}")


if __name__ == "__main__":
    main()
