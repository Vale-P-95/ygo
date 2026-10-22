#!/usr/bin/env python3
"""The bench of a change to the probe turn: the boards the search finds
before and after it, all measured with the probe turn after it.

A change to the measure changes the values the search steers by (value.rs
measures each card with the probe turn), so the boards found change too;
but scripts/bench.py measures each version with its own probe turn, and
the numbers before and after are on two different scales. Here both
searches save the lines of their best boards (combos --save-lines) and the
new CLI measures them all (probe-line): a hand whose best board is worth
less than before lost a board the new measure prefers.

    python3 scripts/bench_cross.py $S/cross --decks $S/mdm --old $CLI_BEFORE --new $CLI_AFTER
"""
import argparse
import concurrent.futures as cf
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import ROOT, WEIGHTS, cases, follow_up_value  # noqa: E402


def composite(m):
    return sum(w * m[k] for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience"))) + follow_up_value(m)


def search(cli, case, nodes, top, lines):
    name, deck, hand = case
    subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", deck, "--hand", hand, "--nodes", str(nodes),
                    "--seconds", "900", "--threads", "1", "--top", str(top), "--save-lines", lines], capture_output=True, text=True)
    return len(json.load(open(lines))["items"]) if os.path.exists(lines) else 0


def measure(cli, case, lines, index, out):
    name, deck, hand = case
    if os.path.exists(out):
        os.remove(out)
    subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", deck, "--hand", hand, "--lines", lines,
                    "--index", str(index), "--json", out], capture_output=True, text=True)
    items = json.load(open(out))["items"] if os.path.exists(out) else []
    return composite(items[0]["measure"]) if items and "measure" in items[0] else None


def one(a, case, side):
    cli = a.old if side == "old" else a.new
    lines = os.path.join(a.out, f"{case[0]}-{side}.lines.json")
    n = search(cli, case, a.nodes, a.top, lines)
    values = [measure(a.new, case, lines, i, os.path.join(a.out, f"{case[0]}-{side}-{i}.json")) for i in range(n)]
    return [v for v in values if v is not None]


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="folder for the lines and measures")
    p.add_argument("--decks", required=True, help="folder with the .ydk files named in tests/fixtures/hands/bench.tsv")
    p.add_argument("--old", required=True, help="CLI before the change")
    p.add_argument("--new", required=True, help="CLI after the change (it measures both)")
    p.add_argument("--nodes", type=int, default=4000)
    p.add_argument("--top", type=int, default=10, help="boards measured for each search")
    p.add_argument("--jobs", type=int, default=4, help="searches at once (each on one thread)")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    todo = [(c, side) for c in cases(a.decks) for side in ("old", "new")]
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        found = dict(zip([(c[0], side) for c, side in todo], ex.map(lambda t: one(a, *t), todo)))
    print(f"{'case':16} {'best':>14} {'first':>14}")
    sums = {"old": [0.0, 0.0], "new": [0.0, 0.0]}
    worse = []
    for name, _, _ in cases(a.decks):
        row = {}
        for side in ("old", "new"):
            v = found[(name, side)]
            row[side] = (max(v, default=0.0), v[0] if v else 0.0)
            sums[side][0] += row[side][0]
            sums[side][1] += row[side][1]
        (ob, of), (nb, nf) = row["old"], row["new"]
        mark = "" if abs(nb - ob) < 0.05 else ("  meglio" if nb > ob else "  PEGGIO")
        if nb < ob - 0.05:
            worse.append(name)
        print(f"{name:16} {ob:6.2f} -> {nb:5.2f} {of:6.2f} -> {nf:5.2f}{mark}")
    print(f"sum best {sums['old'][0]:.2f} -> {sums['new'][0]:.2f} sum first {sums['old'][1]:.2f} -> {sums['new'][1]:.2f}")
    print(f"worse: {', '.join(worse) or 'none'}")


if __name__ == "__main__":
    main()
