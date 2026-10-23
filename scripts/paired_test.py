#!/usr/bin/env python3
"""Paired test to accept a change to the combo search (docs/plans/
ricerca-piano-critico.md, A1): the same hands with the same seed and the
same node budget, before the change and after.

For each hand, the value of the best board measured (the bench's value):
better, worse or even, with a 1% threshold. The change is accepted when the
worse hands are not more than the better ones (sign test) and no hand loses
more than 5%.

--old and --new are each one of:
- a duelforge-cli: this script runs it on the bench's hands and those of
  `combo_hunt.py --seed 7` (see `oracle.py hands`), `--nodes` on one thread,
  into <out>/old and <out>/new (runs already written are kept);
- a folder of `combo_hunt.py --json` (or `combos --json`) files: the hands
  are the files' names;
- the results.json of `bench.py`: the hands are its cases.

    python3 scripts/paired_test.py /tmp/paired --decks /tmp/mdm --old /tmp/cli-main --new target/release/duelforge-cli
    python3 scripts/paired_test.py /tmp/paired --old /tmp/hunt-main --new /tmp/hunt-branch
    python3 scripts/paired_test.py /tmp/paired --old /tmp/bench-main/results.json --new /tmp/bench-branch/results.json

With --measure-cli, the best board of each side is measured again by that
build, the same for both: needed when the change is to the measure itself,
whose values are not on the old scale (the old side measured by the old
measure, the new one by the new, would compare two scales, not two
searches). The runs save their lines for it; a folder of JSON files needs
the lines files of combo_hunt.py next to them.

    python3 scripts/paired_test.py /tmp/paired --decks /tmp/mdm --old /tmp/cli-main --new target/release/duelforge-cli --measure-cli target/release/duelforge-cli
"""
import argparse
import concurrent.futures as cf
import json
import math
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oracle import ROOT, boards_of, hands, run, value  # noqa: E402

# A smaller difference is even; a hand losing more than this blocks the change.
EVEN = 0.01
MAX_LOSS = 0.05


def sign_test(wins, losses):
    """Two-sided p-value of `wins` against `losses` (the even ones left out)."""
    n = wins + losses
    if n == 0:
        return 1.0
    k = min(wins, losses)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2**n)


def side_values(source, side, a):
    """{hand: (best value, search seconds)} of one side: a build to run, a
    folder of combos JSON files, or bench.py's results.json."""
    if os.path.isfile(source) and source.endswith(".json"):
        return {r["name"]: (r["best"], r.get("search_s")) for r in json.load(open(source))}
    if os.path.isdir(source):
        folder = source
    else:
        if not a.decks:
            sys.exit("--decks is needed to run a build")
        folder = os.path.join(a.out, side)
        os.makedirs(folder, exist_ok=True)
        cases = [c for c in hands(a.decks) if not c[4] and a.only in c[0]]

        def one(case):
            path = os.path.join(folder, case[0] + ".json")
            if os.path.exists(path):
                return
            lines = path[:-5] + ".lines.json" if a.measure_cli else None
            secs = run(source, case, a.nodes, path + ".part", out_lines=lines)
            if secs is not None:
                os.replace(path + ".part", path)
                json.dump({"seconds": secs}, open(path[:-5] + ".time.json", "w"))

        with cf.ThreadPoolExecutor(a.jobs) as ex:
            list(ex.map(one, cases))
    out = {}
    for file in sorted(os.listdir(folder)):
        if not file.endswith(".json") or file.endswith((".lines.json", ".time.json")) or a.only not in file:
            continue
        path = os.path.join(folder, file)
        try:
            found = boards_of(path)
        except (KeyError, ValueError):
            continue
        timing = path[:-5] + ".time.json"
        best = found[0][0] if found else 0.0
        if a.measure_cli and found:
            best = measured_again(a, file[:-5], path, found[0][1])
        out[file[:-5]] = (best, json.load(open(timing))["seconds"] if os.path.exists(timing) else None)
    return out


def measured_again(a, name, path, item):
    """The value of the board `item` of the run at `path`, measured by
    --measure-cli (its line from the lines file saved next to the run)."""
    case = next((c for c in hands(a.decks) if c[0] == name), None) if a.decks else None
    lines = path[:-5] + ".lines.json"
    if case is None or not os.path.exists(lines):
        sys.exit(f"{name}: --measure-cli needs --decks and the lines file {lines}")
    index = json.load(open(path))["items"].index(item)
    again = path[:-5] + f".again{index}.json"
    if not os.path.exists(again):
        _, deck, hand, seed, _ = case
        subprocess.run([a.measure_cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", deck, "--hand", hand, "--seed", str(seed),
                        "--lines", lines, "--index", str(index), "--json", again], capture_output=True)
    try:
        return value(json.load(open(again))["items"][0]["measure"])
    except (OSError, ValueError, KeyError, IndexError):
        sys.exit(f"{name}: the best board could not be measured again ({again})")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="folder for the runs and paired.json")
    p.add_argument("--old", required=True, help="before the change: a duelforge-cli, a folder of combos JSON, or bench's results.json")
    p.add_argument("--new", required=True, help="after the change: the same kinds")
    p.add_argument("--decks", help="folder of .ydk files (to run a build)")
    p.add_argument("--nodes", type=int, default=4000)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--only", default="")
    p.add_argument("--measure-cli", help="a duelforge-cli that measures the best board of both sides again (a change to the measure)")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    old, new = side_values(a.old, "old", a), side_values(a.new, "new", a)

    rows = []
    for name in sorted(set(old) & set(new)):
        (v_old, t_old), (v_new, t_new) = old[name], new[name]
        change = (v_new - v_old) / v_old if v_old > 0 else (0.0 if v_new == v_old else math.inf)
        verdict = "better" if change > EVEN else "worse" if change < -EVEN else "even"
        rows.append((name, v_old, v_new, change, verdict, t_old, t_new))
    missing = sorted(set(old) ^ set(new))
    print(f"{'hand':34} {'old':>7} {'new':>7} {'change':>8}  verdict   s old   s new")
    for name, v_old, v_new, change, verdict, t_old, t_new in rows:
        secs = lambda t: f"{t:>7.1f}" if t is not None else f"{'-':>7}"
        print(f"{name:34} {v_old:>7.2f} {v_new:>7.2f} {change:>+8.1%}  {verdict:7} {secs(t_old)} {secs(t_new)}")
    if missing:
        print(f"\nonly on one side, left out: {', '.join(missing)}")
    wins = sum(r[4] == "better" for r in rows)
    losses = sum(r[4] == "worse" for r in rows)
    worst = min((r[3] for r in rows), default=0.0)
    accepted = losses <= wins and worst >= -MAX_LOSS
    total = lambda i: sum(r[i] for r in rows if r[i] is not None)
    print(f"\n{len(rows)} hands: {wins} better, {losses} worse, {len(rows) - wins - losses} even "
          f"(sign test p = {sign_test(wins, losses):.3f}); worst {worst:+.1%}; "
          f"search seconds {total(5):.0f} -> {total(6):.0f}")
    print("ACCEPTED" if accepted else "REJECTED" + (" (a hand loses more than 5%)" if worst < -MAX_LOSS else " (more hands worse than better)"))
    json.dump([dict(zip(("hand", "old", "new", "change", "verdict", "seconds_old", "seconds_new"), r)) for r in rows],
              open(os.path.join(a.out, "paired.json"), "w"), indent=1)
    return 0 if accepted else 1


if __name__ == "__main__":
    sys.exit(main())
