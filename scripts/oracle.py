#!/usr/bin/env python3
"""The oracle of the combo search (docs/plans/ricerca-piano-critico.md, A1):
the same search with a huge budget on a fixed set of hands, its best boards
kept as references, and how the normal search gets close to them.

The hands: the 16 of the bench (tests/fixtures/hands/bench.tsv, seed 1),
the 82 of `combo_hunt.py --seed 7` (two per deck), and the goal searches
(GOALS below, and with --auto-goals N the Extra Deck monsters of the best
board the oracle found for N of the combo_hunt hands). A hand whose deck is
not in --decks is skipped.

    python3 scripts/mdm_decks.py --out /tmp/mdm
    python3 scripts/oracle.py hands --decks /tmp/mdm
    # the long runs (hours: the owner's, at night)
    python3 scripts/oracle.py build /tmp/oracle --decks /tmp/mdm --mult 50 --seeds 1,2,3 --jobs 4
    # the fixtures: codes and zones, the measure, the line's answers (no card text)
    python3 scripts/oracle.py fixtures /tmp/oracle tests/fixtures/oracle
    # the normal search against them
    python3 scripts/oracle.py measure tests/fixtures/oracle /tmp/oracle-measure --decks /tmp/mdm

`build` runs `duelforge-cli combos --probe --json --save-lines` for each
hand and seed, with NODES x MULT nodes on one thread (the same boards on any
machine), into <out>/runs. Runs already written are kept: a night cut short
goes on where it stopped. A board's value is the bench's (`bench.py`): the
probe turn's parts weighed as the "Bilanciato" profile, plus the follow-up.
Only the hand's own seed counts for its fixture: a line found with another
deck order cannot be played again with this one.

`measure` runs the normal search at growing node budgets on the oracle's
hand and seed, and writes per hand: the budget and the search seconds at
which the best board measured reaches 90%, 95% and 100% of the oracle's
value; whether the oracle's best board is listed, and where; for a goal
search, the best board reaching the goal against the oracle's.
"""
import argparse
import concurrent.futures as cf
import json
import os
import random
import re
import sqlite3
import subprocess
import sys
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import HANDS, WEIGHTS, follow_up_value  # noqa: E402

# The normal search's budget, that the oracle multiplies.
NODES = 4000
# Boards measured by the probe turn in each run.
TOP = 10
# Goal searches named by hand: the owner's Invoked case (Invoked Oceanus in
# the Extra Monster Zone and S:P Little Knight), on the committed Invoked list.
GOALS = [
    ("goal-invoked-oceanus-sp", os.path.join(ROOT, "tests", "fixtures", "decks", "invoked-siracusa-md.ydk"),
     "Aleister the Invoker;76334960;Magical Meltdown;Ash Blossom & Joyous Spring;Mulcharmy Fuwalos", 1,
     "Invoked Oceanus@extra;S:P Little Knight"),
]


def bench_hands(decks):
    """The bench's hands whose deck is in `decks`: (name, path, hand)."""
    out = []
    for line in open(HANDS, encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            continue
        name, deck, hand = line.rstrip("\n").split("\t")
        path = os.path.join(decks, deck + ".ydk")
        if not os.path.exists(path):
            continue
        if hand == "random5":
            main = [w for w in open(path).read().split("#extra")[0].split() if w.isdigit()]
            hand = ";".join(random.Random(f"{deck}-0").sample(main, 5))
        out.append((name, path, hand))
    return out


def hunt_hands(decks):
    """The hands of `combo_hunt.py --seed 7`: two per deck, (name, path, hand)."""
    out = []
    for file in sorted(os.listdir(decks)):
        if file.endswith(".ydk"):
            name, path = file[:-4], os.path.join(decks, file)
            main = [w for w in open(path).read().split("#extra")[0].split() if w.isdigit()]
            for k in range(2):
                out.append((f"{name}-{k}", path, ";".join(random.Random(f"{name}-{k}").sample(main, 5))))
    return out


def hands(decks, goals=()):
    """The fixed set: (id, deck path, hand, seed, goal or "")."""
    out = [(f"bench-{n}", p, h, 1, "") for n, p, h in bench_hands(decks)]
    out += [(f"hunt-{n}", p, h, 7, "") for n, p, h in hunt_hands(decks)]
    out += [g for g in GOALS if os.path.exists(g[1])]
    out += list(goals)
    return out


def value(measure):
    """A board's measured value, as in the bench."""
    parts = (measure["field"], measure["hand"], measure["floodgates"], measure["resilience"])
    return sum(w * v for w, v in zip(WEIGHTS, parts)) + follow_up_value(measure)


def run(cli, case, nodes, out_json, out_lines=None, threads=1, top=TOP):
    """One `combos` run; the search seconds, or None when it failed."""
    _, deck, hand, seed, goal = case
    command = [cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", deck, "--hand", hand, "--nodes", str(nodes),
               "--seconds", "86400", "--threads", str(threads), "--top", str(top), "--seed", str(seed), "--json", out_json]
    if out_lines:
        command += ["--save-lines", out_lines]
    if goal:
        command += ["--goal", goal]
    r = subprocess.run(command, capture_output=True, text=True)
    head = next((l for l in r.stdout.splitlines() if l.startswith(("NodeLimit", "TimeLimit", "Exhausted"))), "")
    m = re.search(r"after ([\d.]+) s", head)
    if r.returncode != 0 or not os.path.exists(out_json):
        sys.stderr.write(f"{case[0]}: combos failed ({r.returncode}): {r.stderr[-500:]}\n")
        return None
    return float(m.group(1)) if m else -1.0


def goal_reached(field, goal):
    """The board's field (names as `combos --json` writes them) holds every card of `goal`."""
    for want in filter(None, goal.split(";")):
        name, _, zone = want.partition("@")
        names = [f for f in field if f.split(" (set)")[0].split(" [EMZ]")[0].split(" -> ")[0] == name]
        if zone == "extra":
            names = [f for f in names if "[EMZ]" in f]
        if not names:
            return False
    return True


def boards_of(path, goal=""):
    """The boards measured in a run: (value, item), the best first."""
    doc = json.load(open(path))
    out = [(value(i["measure"]), i) for i in doc["items"] if i.get("measure")]
    if goal:
        out = [b for b in out if goal_reached(b[1]["field"], goal)]
    return sorted(out, key=lambda b: -b[0])


def cmd_hands(a):
    for case in hands(a.decks):
        print("\t".join(map(str, case)))


def cmd_build(a):
    runs = os.path.join(a.out, "runs")
    os.makedirs(runs, exist_ok=True)
    seeds = [int(s) for s in a.seeds.split(",")]
    todo = []
    for case in hands(a.decks, auto_goals(a)):
        if a.only not in case[0]:
            continue
        # The hand's own seed first, then the other deck orders.
        for seed in dict.fromkeys([case[3]] + ([] if a.own_seed_only else seeds)):
            json_path = os.path.join(runs, f"{case[0]}.s{seed}.json")
            if not os.path.exists(json_path):
                todo.append((case[:3] + (seed,) + case[4:], json_path))
    print(f"{len(todo)} runs of {NODES * a.mult} nodes", flush=True)
    json.dump({"nodes": NODES * a.mult, "seeds": seeds, "hands": [c[0] for c in hands(a.decks, auto_goals(a))]},
              open(os.path.join(a.out, "oracle.json"), "w"), indent=1)

    def one(item):
        case, json_path = item
        started = time.time()
        secs = run(a.cli, case, NODES * a.mult, json_path + ".part", json_path[:-5] + ".lines.json")
        if secs is not None:
            os.replace(json_path + ".part", json_path)
        print(f"{case[0]} seed {case[3]}: {time.time() - started:.0f} s", flush=True)

    with cf.ThreadPoolExecutor(a.jobs) as ex:
        list(ex.map(one, todo))


def auto_goals(a):
    """Goal searches from the oracle's best boards so far: their Extra Deck monsters."""
    n = getattr(a, "auto_goals", 0)
    if not n:
        return []
    out = []
    runs = os.path.join(a.out, "runs")
    for name, path, hand in hunt_hands(a.decks):
        best = os.path.join(runs, f"hunt-{name}.s7.json")
        if not os.path.exists(best) or len(out) >= n:
            continue
        boards = boards_of(best)
        if not boards:
            continue
        extra = [f.split(" [EMZ]")[0] + "@extra" for f in boards[0][1]["field"] if f.endswith("[EMZ]")]
        if extra:
            out.append((f"goal-{name}", path, hand, 7, ";".join(extra)))
    return out


def code_of(names):
    """Card names to codes, from the card database."""
    db = sqlite3.connect(os.path.join(ROOT, "data", "cdb", "cards.cdb"))
    out = {}
    for n in names:
        row = db.execute("select id from texts where name = ? order by id limit 1", (n,)).fetchone()
        out[n] = row[0] if row else None
    return out


def place(entry):
    """A field entry of `combos --json` as a name, a zone and its state."""
    name, target = entry, None
    if " -> " in name:
        name, target = name.split(" -> ", 1)
    face_down = name.endswith(" (set)")
    name = name.removesuffix(" (set)")
    zone = "emz" if name.endswith(" [EMZ]") else None
    name = name.removesuffix(" [EMZ]")
    return name, zone, face_down, target


def cmd_fixtures(a):
    meta = json.load(open(os.path.join(a.oracle, "oracle.json")))
    os.makedirs(a.dest, exist_ok=True)
    cases = {c[0]: c for c in hands(a.decks, auto_goals(argparse.Namespace(out=a.oracle, decks=a.decks, auto_goals=a.auto_goals)))}
    written = 0
    for hid in meta["hands"]:
        case = cases.get(hid)
        path = os.path.join(a.oracle, "runs", f"{hid}.s{case[3]}.json") if case else ""
        if not case or not os.path.exists(path):
            continue
        boards = boards_of(path, case[4])[: a.keep]
        lines = json.load(open(path[:-5] + ".lines.json"))["items"]
        doc = json.load(open(path))
        names = {place(f)[0] for _, i in boards for f in i["field"]} | {place(f)[3] for _, i in boards for f in i["field"]}
        codes = code_of(n for n in names if n)
        others = []
        for seed in meta["seeds"]:
            other = os.path.join(a.oracle, "runs", f"{hid}.s{seed}.json")
            if seed != case[3] and os.path.exists(other):
                found = boards_of(other, case[4])
                others.append({"seed": seed, "value": found[0][0] if found else None})
        fixture = {
            "id": hid,
            "deck": os.path.relpath(case[1], ROOT) if case[1].startswith(ROOT) else os.path.basename(case[1]),
            "hand": doc["hand"],
            "seed": case[3],
            "goal": case[4],
            "nodes": meta["nodes"],
            "boards_found": doc["boards_found"],
            "boards": [{
                "value": v,
                "look_key": i["look_key"],
                "field": [{"code": codes.get(n), "name": n, **({"zone": z} if z else {}), **({"set": True} if s else {}),
                           **({"equipped_to": codes.get(t)} if t else {})} for n, z, s, t in map(place, i["field"])],
                "hand": i["hand"],
                "measure": {k: i["measure"][k] for k in ("field", "hand", "floodgates", "resilience", "follow_up")},
                "line": {"answers": lines[i["index"]]["answers"]},
            } for v, i in boards],
            # The best value found with the other deck orders: how far this seed is from them.
            "other_seeds": others,
        }
        json.dump(fixture, open(os.path.join(a.dest, f"{hid}.json"), "w"), indent=1)
        written += 1
    print(f"{written} fixtures in {a.dest}")


def cmd_measure(a):
    os.makedirs(a.out, exist_ok=True)
    budgets = [int(b) for b in a.budgets.split(",")]
    fixtures = [json.load(open(os.path.join(a.oracle, f))) for f in sorted(os.listdir(a.oracle)) if f.endswith(".json")]
    fixtures = [f for f in fixtures if a.only in f["id"] and f["boards"]]

    def one(fx):
        deck = os.path.join(ROOT, fx["deck"]) if os.path.exists(os.path.join(ROOT, fx["deck"])) else os.path.join(a.decks, fx["deck"])
        if not os.path.exists(deck):
            deck = os.path.join(a.decks, os.path.basename(fx["deck"]))
        case = (fx["id"], deck, ";".join(map(str, fx["hand"])), fx["seed"], fx["goal"])
        oracle_value, oracle_key = fx["boards"][0]["value"], fx["boards"][0]["look_key"]
        steps = []
        for nodes in budgets:
            out = os.path.join(a.out, f"{fx['id']}.n{nodes}.json")
            secs = run(a.cli, case, nodes, out)
            if secs is None:
                steps.append({"nodes": nodes, "failed": True})
                continue
            found = boards_of(out, fx["goal"])
            keys = [i["look_key"] for _, i in sorted(boards_of(out), key=lambda b: b[1].get("printed", 99))]
            steps.append({"nodes": nodes, "seconds": secs, "best": found[0][0] if found else 0.0,
                          "oracle_rank": keys.index(oracle_key) + 1 if oracle_key in keys else None})
        reach = {}
        for frac in (0.9, 0.95, 1.0):
            hit = next((s for s in steps if not s.get("failed") and s["best"] >= frac * oracle_value * (1 - 1e-9)), None)
            reach[str(frac)] = {"nodes": hit["nodes"], "seconds": hit["seconds"]} if hit else None
        return {"id": fx["id"], "oracle": oracle_value, "goal": fx["goal"], "steps": steps, "reach": reach}

    with cf.ThreadPoolExecutor(a.jobs) as ex:
        results = list(ex.map(one, fixtures))
    json.dump(results, open(os.path.join(a.out, "measure.json"), "w"), indent=1)
    print(f"{'hand':34} {'oracle':>7} " + " ".join(f"{b:>8}" for b in budgets) + "   90%    95%   100%  rank")
    for r in results:
        cells = " ".join(f"{s['best'] / r['oracle']:>8.0%}" if not s.get("failed") and r["oracle"] > 0 else f"{'-':>8}" for s in r["steps"])
        reach = " ".join(f"{r['reach'][k]['nodes']:>6}" if r["reach"][k] else f"{'-':>6}" for k in ("0.9", "0.95", "1.0"))
        rank = r["steps"][-1].get("oracle_rank") if r["steps"] else None
        print(f"{r['id']:34} {r['oracle']:>7.2f} {cells} {reach}  {rank if rank else '-'}")
    n = len(results)
    if n:
        last = budgets[-1]
        at95 = sum(1 for r in results if r["reach"]["0.95"] and r["reach"]["0.95"]["nodes"] <= last)
        top3 = sum(1 for r in results if r["steps"] and (r["steps"][-1].get("oracle_rank") or 99) <= 3)
        print(f"\n{at95}/{n} hands reach 95% of the oracle within {last} nodes; the oracle's board in the top 3 in {top3}/{n}")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    h = sub.add_parser("hands", help="list the fixed set of hands")
    h.add_argument("--decks", required=True)
    b = sub.add_parser("build", help="the long runs")
    b.add_argument("out")
    b.add_argument("--decks", required=True)
    b.add_argument("--mult", type=int, default=50, help=f"budget: {NODES} nodes times this")
    b.add_argument("--seeds", default="1,2,3", help="deck orders tried (the hand's own seed makes its fixture)")
    b.add_argument("--own-seed-only", action="store_true", help="only the hand's own seed")
    b.add_argument("--auto-goals", type=int, default=0, help="goal searches from this many combo_hunt hands (after a first build)")
    b.add_argument("--jobs", type=int, default=4)
    b.add_argument("--only", default="")
    b.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    f = sub.add_parser("fixtures", help="write tests/fixtures/oracle from a build")
    f.add_argument("oracle")
    f.add_argument("dest")
    f.add_argument("--decks", required=True)
    f.add_argument("--keep", type=int, default=5, help="boards kept per hand")
    f.add_argument("--auto-goals", type=int, default=0)
    m = sub.add_parser("measure", help="the normal search against the oracle")
    m.add_argument("oracle", help="folder of fixtures")
    m.add_argument("out")
    m.add_argument("--decks", required=True)
    m.add_argument("--budgets", default="1000,2000,4000,8000")
    m.add_argument("--jobs", type=int, default=4)
    m.add_argument("--only", default="")
    m.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    a = p.parse_args()
    {"hands": cmd_hands, "build": cmd_build, "fixtures": cmd_fixtures, "measure": cmd_measure}[a.cmd](a)


if __name__ == "__main__":
    main()
