#!/usr/bin/env python3
"""Lua instructions per engine request, from `combo_hunt.py --lua-count`.

What a limit of Lua instructions per request (from an answer to the next
decision) would cut: per run, the requests and the time over each
threshold, next to the seconds past the time limit when the logs carry
the phases (DUELFORGE_DEBUG_PHASES=1).

    python3 scripts/lua_count.py <counts folder> [--logs <combo_hunt logs>]
"""
import argparse
import array
import collections
import glob
import os
import re

THRESHOLDS = [10**4, 10**5, 10**6, 10**7, 10**8]


def load(path):
    a = array.array("I")
    with open(path, "rb") as f:
        a.frombytes(f.read())
    # Thousands of instructions, microseconds.
    return list(zip(a[0::2], a[1::2]))


def quantile(sorted_values, q):
    return sorted_values[min(len(sorted_values) - 1, int(q * len(sorted_values)))] if sorted_values else 0


def past_limit(log):
    if not log or not os.path.exists(log):
        return None
    text = open(log).read()
    phases = [(float(m.group(1)), float(m.group(2))) for m in re.finditer(r"phase \w+ done at ([\d.]+) s \(limit ([\d.]+) s\)", text)]
    return max(0.0, phases[-1][0] - phases[-1][1]) if phases else None


def short(n):
    return f"{n / 10**6:g}M" if n >= 10**6 else f"{n / 1000:g}k"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("counts", help="folder of <run>.bin files")
    parser.add_argument("--logs", help="combo_hunt --logs folder, for the time past the limit")
    parser.add_argument("--slow", type=float, default=10.0, help="seconds past the limit that make a run slow")
    args = parser.parse_args()

    runs = []
    for path in sorted(glob.glob(os.path.join(args.counts, "*.bin"))):
        name = os.path.basename(path)[:-4]
        records = load(path)
        over = past_limit(os.path.join(args.logs, name + ".log")) if args.logs else None
        runs.append((name, records, over))

    print(f"{'run':30} {'requests':>9} {'Minstr':>8} {'engine s':>8} {'past':>5} {'max instr':>9} {'p999':>7} {'max ms':>7} {'instr of slowest':>16}")
    for name, records, over in runs:
        instr = sorted(r[0] for r in records)
        slowest = max(records, key=lambda r: r[1], default=(0, 0))
        print(f"{name:30} {len(records):>9} {sum(instr) / 1000:>8.0f} {sum(r[1] for r in records) / 1e6:>8.1f} "
              f"{'' if over is None else f'{over:.1f}':>5} {short(1000 * instr[-1] if instr else 0):>9} {short(1000 * quantile(instr, 0.999)):>7} "
              f"{slowest[1] / 1000:>7.1f} {short(1000 * slowest[0]):>16}")

    print()
    print("Per threshold: requests over it and their engine seconds; the seconds a cap there would save at most")
    print("(the part of each request past the threshold, if time grows with the instructions).")
    slow = [r for r in runs if r[2] is not None and r[2] > args.slow]
    groups = [("slow runs (past the limit > %g s)" % args.slow, slow), ("other runs", [r for r in runs if r not in slow])]
    for label, group in groups:
        print(f"  {label}: {len(group)} runs, {sum(len(r[1]) for r in group)} requests")
        for t in THRESHOLDS:
            k = t // 1000
            over = [rec for _, records, _ in group for rec in records if rec[0] > k]
            saved = sum(us * (1 - k / ins) for ins, us in over) / 1e6
            print(f"    > {short(t):>5} instructions: {len(over):>7} requests, {sum(us for _, us in over) / 1e6:>7.2f} s, saved at most {saved:.2f} s")

    # Where the slow requests' time goes, whatever their instructions.
    print()
    print("Requests by time: count, and the most instructions among them")
    buckets = collections.defaultdict(lambda: [0, 0])
    for _, records, _ in runs:
        for ins, us in records:
            b = 0 if us < 1000 else 1 if us < 10_000 else 2 if us < 100_000 else 3 if us < 1_000_000 else 4
            buckets[b][0] += 1
            buckets[b][1] = max(buckets[b][1], ins)
    for b, label in enumerate(["< 1 ms", "1-10 ms", "10-100 ms", "0.1-1 s", ">= 1 s"]):
        if b in buckets:
            print(f"  {label:>9}: {buckets[b][0]:>9} requests, at most {short(1000 * buckets[b][1])} instructions")


if __name__ == "__main__":
    main()
