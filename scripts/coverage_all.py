#!/usr/bin/env python3
"""Mechanics coverage on every deck of a folder: `duelforge-cli coverage`
(one hand per Main Deck card; what is offered, left out, tried, activated,
negated) and the top lines of one random hand, one file each.

    python3 scripts/coverage_all.py /tmp/mdm /tmp/coverage --nodes 1500
"""
import argparse
import os
import random
import subprocess
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("decks", help="folder of .ydk files")
    p.add_argument("out", help="folder for the reports")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--nodes", type=int, default=1500, help="choices explored per hand")
    p.add_argument("--threads", type=int, default=4)
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    data = os.path.join(ROOT, "data")
    for file in sorted(f for f in os.listdir(a.decks) if f.endswith(".ydk")):
        name, deck = file[:-4], os.path.join(a.decks, file)
        if os.path.exists(os.path.join(a.out, name + ".coverage.txt")):
            continue
        started = time.time()
        r = subprocess.run([a.cli, "--data", data, "coverage", "--deck", deck, "--nodes", str(a.nodes), "--threads", str(a.threads), "--all"],
                           capture_output=True, text=True)
        open(os.path.join(a.out, name + ".coverage.txt"), "w").write(r.stdout + "\n--- stderr ---\n" + r.stderr[-4000:])
        main_deck = [w for w in open(deck).read().split("#extra")[0].split() if w.isdigit()]
        hand = random.Random(f"{name}-lines").sample(main_deck, 5)
        r = subprocess.run([a.cli, "--data", data, "combos", "--deck", deck, "--hand", ";".join(hand), "--nodes", "3000",
                            "--threads", str(a.threads), "--seconds", "600", "--top", "8", "--lines", "--probe"], capture_output=True, text=True)
        open(os.path.join(a.out, name + ".lines.txt"), "w").write(r.stdout + "\n--- stderr ---\n" + r.stderr[-4000:])
        print(f"{name}: {time.time() - started:.0f} s", flush=True)


if __name__ == "__main__":
    main()
