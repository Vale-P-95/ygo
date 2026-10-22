#!/usr/bin/env python3
"""Run the combo search and the probe turn on every deck of a folder.

For each .ydk, a few starting hands drawn at random (the same ones every
time) are searched with a node budget on one thread, so two runs of the same
code give the same boards. Prints one line per hand: how the search ended,
the probe turn failures, crashes, and the best board measured. Used to look
for engine bugs and as a regression table.

    python3 scripts/combo_hunt.py /tmp/mdm --hands 2 --nodes 3000

For the board classification (docs/plans/classificazione-board.md, C1):
one hand per deck drawn with the "-lines" rule, the archetype decks too
(named x_*), a fixed seed, and every board listed written as JSON with
its line, for scripts/rank_report.py and scripts/board_pairs.py:

    python3 scripts/combo_hunt.py /tmp/mdm --extra-decks /tmp/decks --lines-rule --top 8 --seed 1 --json /tmp/json-1
"""
import argparse
import os
import random
import re
import shlex
import subprocess
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("decks", help="folder of .ydk files")
    parser.add_argument("--hands", type=int, default=2)
    parser.add_argument("--nodes", type=int, default=3000)
    parser.add_argument("--seconds", type=int, default=600, help="time limit of each search")
    parser.add_argument("--top", type=int, default=5, help="boards put through the probe turn")
    parser.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    parser.add_argument("--logs", help="folder for the full output of each run")
    parser.add_argument("--extra-decks", help="a second folder of .ydk files, named x_<name> (the archetype decks)")
    parser.add_argument("--lines-rule", action="store_true", help='one hand per deck, drawn with the "<name>-lines" seed of the studies')
    parser.add_argument("--seed", type=int, help="seed of the duels (the search and the probe turn)")
    parser.add_argument("--json", help="folder for each run's boards (combos --json) and lines (--save-lines)")
    parser.add_argument("--only", default="", help="only the decks whose name contains this")
    parser.add_argument("--shard", default="0/1", help="K/N: only every Nth deck from the Kth")
    parser.add_argument("--staged", action="store_true", help="measure in steps, as the Combo Lab does (combos --staged)")
    parser.add_argument("--combos-args", default="", help="more options for combos, as one string (e.g. \"--staged-windows 6\")")
    parser.add_argument("--lua-count", help="folder for each run's Lua instructions per engine request (DUELFORGE_LUA_COUNT)")
    args = parser.parse_args()

    decks = [(name[:-4], os.path.join(args.decks, name)) for name in os.listdir(args.decks) if name.endswith(".ydk")]
    if args.extra_decks:
        decks += [("x_" + name[:-4], os.path.join(args.extra_decks, name)) for name in os.listdir(args.extra_decks) if name.endswith(".ydk")]
    k_shard, n_shard = map(int, args.shard.split("/"))
    decks = [d for i, d in enumerate(sorted(decks)) if args.only in d[0] and i % n_shard == k_shard]
    if args.json:
        os.makedirs(args.json, exist_ok=True)

    problems = 0
    for name, path in decks:
        main_deck = [l for l in open(path).read().split("#extra")[0].split() if l.isdigit()]
        for k in range(1 if args.lines_rule else args.hands):
            hand = random.Random(f"{name}-lines" if args.lines_rule else f"{name}-{k}").sample(main_deck, 5)
            command = [args.cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", path, "--hand", ";".join(hand),
                       "--nodes", str(args.nodes), "--seconds", str(args.seconds), "--threads", "1", "--top", str(args.top), "--probe"]
            if args.seed is not None:
                command += ["--seed", str(args.seed)]
            if args.staged:
                command += ["--staged"]
            command += shlex.split(args.combos_args)
            if args.json:
                command += ["--json", os.path.join(args.json, f"{name}-{k}.json"), "--save-lines", os.path.join(args.json, f"{name}-{k}.lines.json")]
            env = None
            if args.lua_count:
                os.makedirs(args.lua_count, exist_ok=True)
                counts = os.path.join(args.lua_count, f"{name}-{k}.bin")
                if os.path.exists(counts):
                    os.remove(counts)
                env = dict(os.environ, DUELFORGE_LUA_COUNT=counts)
            run = subprocess.run(command, capture_output=True, text=True, env=env)
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
