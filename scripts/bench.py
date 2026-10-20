#!/usr/bin/env python3
"""Benchmark of the combo search: fixed hands, a fixed node budget on one
thread (so the result does not depend on the machine's load), and the best
board measured by the probe turn. Run it before and after a change to the
search and compare the tables (README, PIANO §29.5).

    python3 scripts/mdm_decks.py --out /tmp/mdm
    python3 scripts/bench.py /tmp/bench-before --decks /tmp/mdm --nodes 4000
    python3 scripts/bench.py /tmp/bench-after --decks /tmp/mdm --nodes 4000 --cli path/to/other/duelforge-cli
"""
import argparse
import concurrent.futures as cf
import json
import os
import random
import re
import subprocess
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
HANDS = os.path.join(ROOT, "tests", "fixtures", "hands", "bench.tsv")
# Weights of field, hand, floodgates and resilience in the measured value
# (the "Bilanciato" profile).
WEIGHTS = (1.0, 0.9, 0.8, 0.6)


def cases(decks: str):
    out = []
    for line in open(HANDS, encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            continue
        name, deck, hand = line.rstrip("\n").split("\t")
        path = os.path.join(decks, deck + ".ydk")
        if hand == "random5":
            main = [w for w in open(path).read().split("#extra")[0].split() if w.isdigit()]
            hand = ";".join(random.Random(f"{deck}-0").sample(main, 5))
        out.append((name, path, hand))
    return out


def run(cli, case, nodes, top, logs):
    name, deck, hand = case
    started = time.time()
    r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", deck, "--hand", hand, "--nodes", str(nodes),
                        "--seconds", "900", "--threads", "1", "--top", str(top), "--probe", "--lines"], capture_output=True, text=True)
    open(os.path.join(logs, name + ".log"), "w").write(r.stdout + "\n--- stderr ---\n" + r.stderr[-3000:])
    head = next((l for l in r.stdout.splitlines() if l.startswith(("NodeLimit", "TimeLimit", "Exhausted"))), "")
    m = re.search(r"after ([\d.]+) s: (\d+) nodes .*?(\d+) replays, (\d+) duplicates, (\d+) boards", head)
    measured = [tuple(map(float, x)) for x in
                re.findall(r"probe turn \([^)]*\): field ([\d.]+) · hand ([\d.]+) · floodgates ([\d.]+) · resilience ([\d.]+)", r.stdout)]
    values = [sum(w * v for w, v in zip(WEIGHTS, x)) for x in measured]
    num = lambda i, f: f(m.group(i)) if m else -1
    return dict(name=name, rc=r.returncode, search_s=num(1, float), nodes=num(2, int), replays=num(3, int), dups=num(4, int),
                boards=num(5, int), best=round(max(values, default=0.0), 3), first=round(values[0] if values else 0.0, 3),
                wall=round(time.time() - started, 1))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="folder for the logs and results.json")
    p.add_argument("--decks", required=True, help="folder with the .ydk files named in tests/fixtures/hands/bench.tsv")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--nodes", type=int, default=4000)
    p.add_argument("--jobs", type=int, default=4, help="hands searched at once")
    p.add_argument("--top", type=int, default=10, help="boards put through the probe turn")
    p.add_argument("--only", default="", help="only the hands whose name contains this")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    todo = [c for c in cases(a.decks) if a.only in c[0]]
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        res = list(ex.map(lambda c: run(a.cli, c, a.nodes, a.top, a.out), todo))
    json.dump(res, open(os.path.join(a.out, "results.json"), "w"), indent=1)
    print(f"{'case':16} {'rc':>2} {'search':>7} {'nodes':>6} {'repl':>6} {'dups':>6} {'boards':>6} {'first':>6} {'best':>6}")
    for x in res:
        print(f"{x['name']:16} {x['rc']:>2} {x['search_s']:>7.1f} {x['nodes']:>6} {x['replays']:>6} {x['dups']:>6} {x['boards']:>6} "
              f"{x['first']:>6.2f} {x['best']:>6.2f}")
    print("sum best", round(sum(x["best"] for x in res), 2), "sum first", round(sum(x["first"] for x in res), 2))


if __name__ == "__main__":
    main()
