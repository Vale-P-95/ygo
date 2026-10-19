#!/usr/bin/env python3
"""Run the combo search and the probe turn on every deck of a folder.

For each .ydk, a few starting hands drawn at random (the same ones every
time) are searched with a node budget on one thread, so two runs of the same
code give the same boards. Prints one line per hand: how the search ended,
the probe turn failures, crashes, and the best board measured. Used to look
for engine bugs and as a regression table.

    python3 scripts/combo_hunt.py /tmp/mdm --hands 2 --nodes 3000
"""
import argparse
import os
import random
import re
import subprocess
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("decks", help="folder of .ydk files")
    parser.add_argument("--hands", type=int, default=2)
    parser.add_argument("--nodes", type=int, default=3000)
    parser.add_argument("--top", type=int, default=5, help="boards put through the probe turn")
    parser.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    parser.add_argument("--logs", help="folder for the full output of each run")
    args = parser.parse_args()

    problems = 0
    for name in sorted(f[:-4] for f in os.listdir(args.decks) if f.endswith(".ydk")):
        path = os.path.join(args.decks, name + ".ydk")
        main_deck = [l for l in open(path).read().split("#extra")[0].split() if l.isdigit()]
        for k in range(args.hands):
            hand = random.Random(f"{name}-{k}").sample(main_deck, 5)
            run = subprocess.run(
                [args.cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", path, "--hand", ";".join(hand),
                 "--nodes", str(args.nodes), "--seconds", "600", "--threads", "1", "--top", str(args.top), "--probe"],
                capture_output=True, text=True,
            )
            out = run.stdout
            if args.logs:
                os.makedirs(args.logs, exist_ok=True)
                with open(os.path.join(args.logs, f"{name}-{k}.log"), "w") as f:
                    f.write(out + "\n--- stderr ---\n" + run.stderr[-5000:])
            head = out.splitlines()[0] if out else "no output"
            failed = out.count("probe turn failed")
            measured = [tuple(map(float, m)) for m in re.findall(r"probe turn \([^)]*\): field ([\d.]+) · hand ([\d.]+) · floodgates ([\d.]+) · resilience ([\d.]+)", out)]
            best = max(measured, key=sum, default=None)
            status = "ok" if run.returncode == 0 and not failed else "PROBLEM"
            problems += status != "ok"
            best_text = "field {:.2} hand {:.2} floodgates {:.2} resilience {:.2}".format(*best) if best else "-"
            print(f"{status:7} {name}-{k} exit={run.returncode} probe_failures={failed} | {head} | best: {best_text}", flush=True)
    print(f"{problems} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
