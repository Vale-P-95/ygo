#!/usr/bin/env python3
"""Lua instructions or calls per engine request, from `combo_hunt.py --lua-count`.

What a limit of Lua instructions (or calls, `--calls`, or entries into Lua, `--entries`) per request (from an answer to the next
decision) would cut: per run, the requests and the time over each
threshold, next to the seconds past the time limit when the logs carry
the phases (DUELFORGE_DEBUG_PHASES=1).

    python3 scripts/lua_count.py <counts folder> [--logs <combo_hunt logs>] [--calls | --entries]
"""
import argparse
import array
import collections
import glob
import os
import re

THRESHOLDS = [10**4, 10**5, 10**6, 10**7, 10**8]


def load(path, field=0):
    """(count, microseconds) per request: thousands of instructions (field 0), calls (1) or entries into Lua (2)."""
    a = array.array("I")
    with open(path, "rb") as f:
        a.frombytes(f.read())
    return list(zip(a[field::4], a[3::4]))


def quantile(sorted_values, q):
    return sorted_values[min(len(sorted_values) - 1, int(q * len(sorted_values)))] if sorted_values else 0


def past_limit(log):
    if not log or not os.path.exists(log):
        return None
    text = open(log).read()
    phases = [(float(m.group(1)), float(m.group(2))) for m in re.finditer(r"phase \w+ done at ([\d.]+) s \(limit ([\d.]+) s\)", text)]
    return max(0.0, phases[-1][0] - phases[-1][1]) if phases else None


def short(n):
    return f"{n / 10**6:g}M" if n >= 10**6 else f"{n / 1000:g}k" if n >= 1000 else f"{n:g}"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("counts", help="folder of <run>.bin files")
    parser.add_argument("--logs", help="combo_hunt --logs folder, for the time past the limit")
    parser.add_argument("--calls", action="store_true", help="Lua calls (Lua and C functions) instead of instructions")
    parser.add_argument("--entries", action="store_true", help="times the core entered Lua instead of instructions")
    parser.add_argument("--slow", type=float, default=10.0, help="seconds past the limit that make a run slow")
    args = parser.parse_args()

    unit = 1 if args.calls or args.entries else 1000
    what = "entries" if args.entries else "calls" if args.calls else "instructions"
    runs = []
    for path in sorted(glob.glob(os.path.join(args.counts, "*.bin"))):
        name = os.path.basename(path)[:-4]
        records = load(path, 2 if args.entries else 1 if args.calls else 0)
        over = past_limit(os.path.join(args.logs, name + ".log")) if args.logs else None
        runs.append((name, records, over))

    print(f"{'run':30} {'requests':>9} {'M ' + what[:5]:>8} {'engine s':>8} {'past':>5} {'max':>9} {'p999':>7} {'max ms':>7} {'of slowest':>16}")
    for name, records, over in runs:
        instr = sorted(r[0] for r in records)
        slowest = max(records, key=lambda r: r[1], default=(0, 0))
        print(f"{name:30} {len(records):>9} {sum(instr) * unit / 10**6:>8.0f} {sum(r[1] for r in records) / 1e6:>8.1f} "
              f"{'' if over is None else f'{over:.1f}':>5} {short(unit * instr[-1] if instr else 0):>9} {short(unit * quantile(instr, 0.999)):>7} "
              f"{slowest[1] / 1000:>7.1f} {short(unit * slowest[0]):>16}")

    print()
    print("Per threshold: requests over it and their engine seconds; the seconds a cap there would save at most")
    print("(the part of each request past the threshold, if time grows with the instructions).")
    slow = [r for r in runs if r[2] is not None and r[2] > args.slow]
    groups = [("slow runs (past the limit > %g s)" % args.slow, slow), ("other runs", [r for r in runs if r not in slow])]
    for label, group in groups:
        print(f"  {label}: {len(group)} runs, {sum(len(r[1]) for r in group)} requests")
        for t in THRESHOLDS:
            k = t // unit
            over = [rec for _, records, _ in group for rec in records if rec[0] > k]
            saved = sum(us * (1 - k / ins) for ins, us in over) / 1e6
            print(f"    > {short(t):>5} {what}: {len(over):>7} requests, {sum(us for _, us in over) / 1e6:>7.2f} s, saved at most {saved:.2f} s")

    # Where the slow requests' time goes, whatever their instructions.
    print()
    print(f"Requests by time: count, and the most {what} among them")
    buckets = collections.defaultdict(lambda: [0, 0])
    for _, records, _ in runs:
        for ins, us in records:
            b = 0 if us < 1000 else 1 if us < 10_000 else 2 if us < 100_000 else 3 if us < 1_000_000 else 4
            buckets[b][0] += 1
            buckets[b][1] = max(buckets[b][1], ins)
    for b, label in enumerate(["< 1 ms", "1-10 ms", "10-100 ms", "0.1-1 s", ">= 1 s"]):
        if b in buckets:
            print(f"  {label:>9}: {buckets[b][0]:>9} requests, at most {short(unit * buckets[b][1])} {what}")


if __name__ == "__main__":
    main()
