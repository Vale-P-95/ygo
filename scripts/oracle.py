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
    # the fixtures: codes and zones, the measure, the best line's answers (no card text)
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
# The other kinds of boards a fixture keeps: the best by interruptions on
# the field, by protection against the breakers, by what is left to rebuild
# with in the next turn.
KINDS = [
    ("field", lambda m: m["field"]),
    ("resilience", lambda m: m["resilience"]),
    ("follow_up", lambda m: (m.get("follow_up") or {}).get("rebuild", 0)),
]
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
        main = [w for w in open(path).read().split("#extra")[0].split() if w.isdigit()]
        if hand == "random5":
            hand = ";".join(random.Random(f"{deck}-0").sample(main, 5))
        missing = missing_cards(hand.split(";"), main)
        if missing:
            sys.stderr.write(f"bench hand {name} skipped: {', '.join(missing)} not in {deck}.ydk (or fewer copies)\n")
            continue
        out.append((name, path, hand))
    return out


def missing_cards(hand, main):
    """The cards of `hand` (names or passcodes) the Main Deck `main` (passcodes) has fewer copies of, any artwork."""
    db = sqlite3.connect(os.path.join(ROOT, "data", "cdb", "cards.cdb"))
    left = list(main)
    missing = []
    for card in hand:
        names = db.execute("select name from texts where id = ?", (int(card),)).fetchall() if card.isdigit() else [(card,)]
        codes = {str(r[0]) for (name,) in names for r in db.execute("select id from texts where name = ?", (name,))}
        copy = next((c for c in left if c in codes), None)
        if copy is None:
            missing.append(card)
        else:
            left.remove(copy)
    return missing


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


def deck_hash(path):
    """An id of the deck list (Main and Extra Deck, in order): the Master Duel Meta lists change and are not committed."""
    import hashlib
    cards = [w for w in open(path).read().split() if w.isdigit() or w.startswith(("#", "!"))]
    return hashlib.sha256(" ".join(cards).encode()).hexdigest()[:16]


def value(measure):
    """A board's measured value, as in the bench."""
    parts = (measure["field"], measure["hand"], measure["floodgates"], measure["resilience"])
    return sum(w * v for w, v in zip(WEIGHTS, parts)) + follow_up_value(measure)


def run(cli, case, nodes, out_json, out_lines=None, threads=1, top=TOP, staged=None, extra=(), timing=None):
    """One `combos` run; the search seconds, or None when it failed. With
    `staged` (cheap, full): the cheap measure of the first `cheap` fields by
    the estimate, then the full one of the best `full` by it (B14). `extra`:
    more options for combos (e.g. --checkpoints). With `timing`, the search
    seconds and the whole run's go to that file."""
    _, deck, hand, seed, goal = case
    if staged and goal:
        # The boards reaching the goal are listed first: the full measure of
        # the first FULL, not of the best by the cheap measure among all.
        top, staged = staged[1], None
    if staged:
        top = staged[0]
    command = [cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", deck, "--hand", hand, "--nodes", str(nodes),
               "--seconds", "86400", "--threads", str(threads), "--top", str(top), "--seed", str(seed), "--json", out_json]
    if staged:
        command += ["--staged", "--staged-cheap", str(staged[0]), "--staged-full", str(staged[1])]
    if out_lines:
        command += ["--save-lines", out_lines]
    if goal:
        command += ["--goal", goal]
    command += list(extra)
    started = time.time()
    r = subprocess.run(command, capture_output=True, text=True)
    wall = time.time() - started
    head = next((l for l in r.stdout.splitlines() if l.startswith(("NodeLimit", "TimeLimit", "Exhausted"))), "")
    m = re.search(r"after ([\d.]+) s", head)
    if r.returncode != 0 or not os.path.exists(out_json):
        sys.stderr.write(f"{case[0]}: combos failed ({r.returncode}): {r.stderr[-500:]}\n")
        return None
    secs = float(m.group(1)) if m else -1.0
    if timing:
        json.dump({"seconds": secs, "wall": wall}, open(timing, "w"))
    return secs


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
    # A board with the cheap measure only (combos --staged) has no value to rank.
    out = [(value(i["measure"]), i) for i in doc["items"] if i.get("measure") and i.get("stage") != "cheap"]
    if goal:
        out = [b for b in out if goal_reached(b[1]["field"], goal)]
    return sorted(out, key=lambda b: -b[0])


def staged(a):
    """`--staged` as (cheap, full), or None."""
    return tuple(int(x) for x in a.staged.split(",")) if getattr(a, "staged", None) else None


def matches(only, hid):
    """`hid` is one of the hands `--only` names (any of its comma-separated parts)."""
    return any(part in hid for part in only.split(","))


def cmd_hands(a):
    for case in hands(a.decks):
        print("\t".join(map(str, case)))


def cmd_build(a):
    runs = os.path.join(a.out, "runs")
    os.makedirs(runs, exist_ok=True)
    seeds = [int(s) for s in a.seeds.split(",")]
    todo = []
    for case in hands(a.decks, auto_goals(a)):
        if not matches(a.only, case[0]):
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
        secs = run(a.cli, case, NODES * a.mult, json_path + ".part", json_path[:-5] + ".lines.json", staged=staged(a))
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
        found = boards_of(path, case[4])
        boards = [(v, i, []) for v, i in found[: a.keep]]
        # Boards of other kinds stay among the references too, the best by
        # each part of the measure: a kind the measure undervalues is kept.
        for why, part in KINDS:
            best = max(found, key=lambda b: part(b[1]["measure"]), default=None)
            if best is None or part(best[1]["measure"]) <= 0:
                continue
            same = next((b for b in boards if b[1]["look_key"] == best[1]["look_key"]), None)
            if same:
                same[2].append(why)
            else:
                boards.append((best[0], best[1], [why]))
        lines = json.load(open(path[:-5] + ".lines.json"))["items"]
        doc = json.load(open(path))
        names = {place(f)[0] for _, i, _ in boards for f in i["field"]} | {place(f)[3] for _, i, _ in boards for f in i["field"]}
        codes = code_of(n for n in names if n)
        others = []
        for seed in meta["seeds"]:
            other = os.path.join(a.oracle, "runs", f"{hid}.s{seed}.json")
            if seed != case[3] and os.path.exists(other):
                found_other = boards_of(other, case[4])
                others.append({"seed": seed, "value": found_other[0][0] if found_other else None})
        previous = os.path.join(a.dest, f"{hid}.json")
        fixture = {
            "id": hid,
            "deck": os.path.relpath(case[1], ROOT) if case[1].startswith(ROOT) else os.path.basename(case[1]),
            "hand": doc["hand"],
            "deck_hash": deck_hash(case[1]),
            "seed": case[3],
            "goal": case[4],
            "nodes": meta["nodes"],
            "boards_found": doc["boards_found"],
            "boards": [{
                "value": v,
                "look_key": i["look_key"],
                **({"kept_for": why} if why else {}),
                "field": [{"code": codes.get(n), "name": n, **({"zone": z} if z else {}), **({"set": True} if s else {}),
                           **({"equipped_to": codes.get(t)} if t else {})} for n, z, s, t in map(place, i["field"])],
                "hand": i["hand"],
                "measure": {k: i["measure"][k] for k in ("field", "hand", "floodgates", "resilience", "follow_up")},
                # The answers of its line, one hex string each (no card text): `oracle.py lines` writes them for probe-line.
                "line": [bytes(x).hex() for x in lines[i["index"]]["answers"]],
            } for v, i, why in boards],
            # The best value found with the other deck orders: how far this seed is from them.
            "other_seeds": others,
        }
        # The convergence check of an earlier fixture of the same hand and budget stays (`converge`).
        if os.path.exists(previous):
            old = json.load(open(previous))
            if old.get("convergence") and old.get("nodes") == fixture["nodes"]:
                fixture["convergence"] = old["convergence"]
        json.dump(fixture, open(os.path.join(a.dest, f"{hid}.json"), "w"), indent=1)
        written += 1
    print(f"{written} fixtures in {a.dest}")


def cmd_measure(a):
    os.makedirs(a.out, exist_ok=True)
    budgets = [int(b) for b in a.budgets.split(",")]
    fixtures = [json.load(open(os.path.join(a.oracle, f))) for f in sorted(os.listdir(a.oracle)) if f.endswith(".json")]
    fixtures = [f for f in fixtures if matches(a.only, f["id"]) and f["boards"]]

    def one(fx):
        deck = os.path.join(ROOT, fx["deck"]) if os.path.exists(os.path.join(ROOT, fx["deck"])) else os.path.join(a.decks, fx["deck"])
        if not os.path.exists(deck):
            deck = os.path.join(a.decks, os.path.basename(fx["deck"]))
        if not os.path.exists(deck):
            sys.stderr.write(f"{fx['id']}: {fx['deck']} is not in --decks; left out\n")
            return None
        if fx.get("deck_hash") and deck_hash(deck) != fx["deck_hash"]:
            sys.stderr.write(f"{fx['id']}: {deck} is not the list the oracle searched; its numbers do not compare\n")
        case = (fx["id"], deck, ";".join(map(str, fx["hand"])), fx["seed"], fx["goal"])
        oracle_value, oracle_key = fx["boards"][0]["value"], fx["boards"][0]["look_key"]
        steps = []
        for nodes in budgets:
            out = os.path.join(a.out, f"{fx['id']}.n{nodes}.json")
            timing = out[:-5] + ".time.json"
            # Runs already written are kept: a measure cut short goes on.
            if os.path.exists(out) and os.path.exists(timing):
                secs = json.load(open(timing))["seconds"]
            else:
                secs = run(a.cli, case, nodes, out, staged=staged(a), extra=a.combos_args.split(), timing=timing)
            if secs is None:
                steps.append({"nodes": nodes, "failed": True})
                continue
            wall = json.load(open(timing)).get("wall")
            found = boards_of(out, fx["goal"])
            listed = sorted(boards_of(out, fx["goal"]), key=lambda b: b[1].get("printed", 99))
            keys = [i["look_key"] for _, i in listed]
            # The oracle's board itself, and the first board as good (boards
            # can differ in what the measure does not value).
            as_good = next((k + 1 for k, (v, _) in enumerate(listed) if v >= 0.99 * oracle_value), None)
            steps.append({"nodes": nodes, "seconds": secs, "wall": wall, "best": found[0][0] if found else 0.0,
                          "oracle_rank": keys.index(oracle_key) + 1 if oracle_key in keys else None, "as_good_rank": as_good})
        reach = {}
        for frac in (0.9, 0.95, 1.0):
            hit = next((s for s in steps if not s.get("failed") and s["best"] >= frac * oracle_value * (1 - 1e-9)), None)
            reach[str(frac)] = {"nodes": hit["nodes"], "seconds": hit["seconds"]} if hit else None
        stable = (fx.get("convergence") or {}).get("stable")
        return {"id": fx["id"], "deck": deck_name(fx), "oracle": oracle_value, "goal": fx["goal"], "steps": steps, "reach": reach,
                "stable": stable}

    with cf.ThreadPoolExecutor(a.jobs) as ex:
        results = [r for r in ex.map(one, fixtures) if r]
    json.dump(results, open(os.path.join(a.out, "measure.json"), "w"), indent=1)
    print(f"{'hand':34} {'oracle':>7} " + " ".join(f"{b:>8}" for b in budgets) + "   90%    95%   100%  same  as good")
    for r in results:
        cells = " ".join(f"{s['best'] / r['oracle']:>8.0%}" if not s.get("failed") and r["oracle"] > 0 else f"{'-':>8}" for s in r["steps"])
        reach = " ".join(f"{r['reach'][k]['nodes']:>6}" if r["reach"][k] else f"{'-':>6}" for k in ("0.9", "0.95", "1.0"))
        last = r["steps"][-1] if r["steps"] else {}
        rank = lambda k: f"{last[k]:>4}" if last.get(k) else f"{'-':>4}"
        print(f"{r['id']:34} {r['oracle']:>7.2f} {cells} {reach}  {rank('oracle_rank')}  {rank('as_good_rank')}")
    n = len(results)
    if n:
        last = budgets[-1]
        at95 = sum(1 for r in results if r["reach"]["0.95"] and r["reach"]["0.95"]["nodes"] <= last)
        top3 = sum(1 for r in results if r["steps"] and (r["steps"][-1].get("oracle_rank") or 99) <= 3)
        good3 = sum(1 for r in results if r["steps"] and (r["steps"][-1].get("as_good_rank") or 99) <= 3)
        print(f"\n{at95}/{n} hands reach 95% of the oracle within {last} nodes; in the top 3 at {last} nodes: "
              f"the oracle's board in {top3}/{n}, a board as good (99%) in {good3}/{n}")
        report_groups(results, budgets)
        report_drops(results, budgets)


def report_drops(results, budgets):
    """The hands whose best board measured gets worse by over 1% from a
    budget to the next one (docs/plans/monotonia-budget.md, E-c2), and the
    seconds of the whole runs at each budget."""
    drops = []
    for r in results:
        steps = [s for s in r["steps"] if not s.get("failed")]
        worst = min(((b["best"] - a["best"]) / a["best"] for a, b in zip(steps, steps[1:]) if a["best"] > 0), default=0.0)
        if worst < -0.01:
            drops.append((r["id"], worst))
    print(f"\n{len(drops)}/{len(results)} hands get worse by over 1% from a budget to the next"
          + (": " + ", ".join(f"{h} {w:+.0%}" for h, w in sorted(drops, key=lambda d: d[1])) if drops else ""))
    walls = []
    for i in range(len(budgets)):
        vals = [r["steps"][i].get("wall") for r in results if i < len(r["steps"]) and not r["steps"][i].get("failed")]
        walls.append(f"{sum(v for v in vals if v):.0f}" if all(vals) and vals else "-")
    print(f"{'run seconds':12} " + " ".join(f"{w:>7}" for w in walls))


def share(r, i):
    """The normal search's share of the oracle's value at budget `i` (up to 100%), or None."""
    s = r["steps"][i] if i < len(r["steps"]) else {}
    return min(s["best"] / r["oracle"], 1.0) if r["oracle"] > 0 and not s.get("failed") and "best" in s else None


def report_groups(results, budgets):
    """The mean share by group: the decks to tune on and those kept apart (70/30 by deck, seed 47, as the
    objective bench), and the hands whose oracle is stable or provisional (`converge`)."""
    plain = [r for r in results if not r["goal"] and r["oracle"] > 0]
    parts = split({r["deck"] for r in plain})
    groups = [("tuning", lambda r: parts[r["deck"]] == "taratura"), ("check", lambda r: parts[r["deck"]] == "verifica"),
              ("stable", lambda r: r.get("stable") is True), ("provisional", lambda r: r.get("stable") is False)]
    print(f"\n{'mean share':12} {'hands':>5} " + " ".join(f"{b:>7}" for b in budgets))
    means = {}
    for name, keep in groups:
        rows = [r for r in plain if keep(r)]
        if not rows:
            continue
        cells = []
        for i in range(len(budgets)):
            vals = [v for v in (share(r, i) for r in rows) if v is not None]
            means[(name, i)] = sum(vals) / len(vals) if vals else None
            cells.append(f"{means[(name, i)]:>7.1%}" if vals else f"{'-':>7}")
        print(f"{name:12} {len(rows):>5} " + " ".join(cells))
    i = len(budgets) - 1
    tune, check = means.get(("tuning", i)), means.get(("check", i))
    if tune is not None and check is not None:
        gap = tune - check
        print(f"decks kept apart: {gap * 100:+.1f} points below the tuning decks at {budgets[i]} nodes "
              f"({'over 5: fitted to the bench, not general' if gap > 0.05 else 'within 5'})")


def deck_name(fx):
    """The deck of a fixture, by its file's name."""
    return os.path.splitext(os.path.basename(fx["deck"]))[0]


def split(decks, seed=47, share=0.7):
    """Tuning and check, by deck, as `objective_bench.py` splits them."""
    order = sorted(decks)
    random.Random(seed).shuffle(order)
    cut = round(len(order) * share)
    return {d: ("taratura" if i < cut else "verifica") for i, d in enumerate(order)}


def cmd_converge(a):
    """Marks each fixture stable when its value does not rise from the
    fixture's budget to the larger builds (over 1%), else provisional."""
    written = 0
    for file in sorted(os.listdir(a.fixtures)):
        if not file.endswith(".json"):
            continue
        path = os.path.join(a.fixtures, file)
        fx = json.load(open(path))
        if not fx["boards"]:
            continue
        values = {fx["nodes"]: fx["boards"][0]["value"]}
        for build in a.builds:
            meta = json.load(open(os.path.join(build, "oracle.json")))
            run_path = os.path.join(build, "runs", f"{fx['id']}.s{fx['seed']}.json")
            if os.path.exists(run_path):
                found = boards_of(run_path, fx["goal"])
                values[meta["nodes"]] = found[0][0] if found else 0.0
        if len(values) < 2:
            continue
        top = max(values.values())
        fx["convergence"] = {"values": {str(k): v for k, v in sorted(values.items())},
                             "stable": top <= fx["boards"][0]["value"] * 1.01}
        json.dump(fx, open(path, "w"), indent=1)
        written += 1
        print(f"{fx['id']:34} " + " ".join(f"{k}: {v:.3f}" for k, v in sorted(values.items())) +
              f"  {'stable' if fx['convergence']['stable'] else 'provisional'}")
    print(f"{written} fixtures checked")


def cmd_as_hunt(a):
    """The combo_hunt hands' fixtures as a `combo_hunt.py --json` folder
    (<deck>-<k>.json and .lines.json), for `objective_bench.py --hunt`:
    the level 2 against the meta on the oracle's boards (A1b, step 1)."""
    os.makedirs(a.out, exist_ok=True)
    written = 0
    for file in sorted(os.listdir(a.fixtures)):
        if not file.startswith("hunt-") or not file.endswith(".json"):
            continue
        fx = json.load(open(os.path.join(a.fixtures, file)))
        if not fx["boards"]:
            continue
        name = fx["id"][len("hunt-"):]

        def entry(f):
            text = f["name"] + (" (set)" if f.get("set") else "") + (" [EMZ]" if f.get("zone") == "emz" else "")
            return text + (f" -> {f['equipped_to']}" if f.get("equipped_to") else "")

        items = []
        for k, b in enumerate(fx["boards"]):
            field = sorted(entry(f) for f in b["field"])
            items.append({"index": k, "printed": k, "look_key": b["look_key"], "field": field, "hand": b["hand"],
                          "signature": ", ".join(field) + " | hand: " + ", ".join(map(str, sorted(b["hand"]))),
                          "measure": b["measure"], "oracle_value": b["value"], "kept_for": b.get("kept_for", [])})
        head = {"deck": fx["deck"], "hand": fx["hand"], "seed": fx["seed"]}
        json.dump(dict(head, kind="boards", items=items), open(os.path.join(a.out, name + ".json"), "w"))
        lines = [{"answers": [list(bytes.fromhex(x)) for x in b["line"]]} for b in fx["boards"]]
        json.dump(dict(head, kind="lines", items=lines), open(os.path.join(a.out, name + ".lines.json"), "w"))
        written += 1
    print(f"{written} hands in {a.out}")


def cmd_disagree(a):
    """Where the measure and the level 2 against the meta disagree on the
    oracle's boards (A1b, step 1): from `objective_bench.py` run on an
    `as-hunt` folder, the truth of a board is 1 - left, weighted by the
    opponents' popularity, without the doubtful pairs (under 80% of the
    opponent's moves made on an empty board). Two boards of a hand
    disagree when the measure puts one first and the truth puts the other
    first by at least `--gap`. Each one is a blindness of the measure to
    correct (B5), with the fixture's board to play again (`oracle.py lines`)."""
    rows = json.load(open(os.path.join(a.bench, "results.json")))["rows"]
    truth = {}
    for r in rows:
        if "left" in r and r.get("made", 1) >= 0.8 and not r.get("failed"):
            t = truth.setdefault(r["board"], [0.0, 0.0])
            t[0] += r["weight"] * (1 - r["left"])
            t[1] += r["weight"]
    truth = {b: t / w for b, (t, w) in truth.items() if w > 0}
    boards = json.load(open(os.path.join(a.bench, "boards.json")))
    by_hand = {}
    for b in boards:
        if b["id"] not in truth:
            continue
        item = json.load(open(os.path.join(a.hunt, b["hand_file"] + ".json")))["items"][b["rank"]]
        by_hand.setdefault(b["hand_file"], []).append(dict(id=b["id"], deck=b["deck"], index=b["rank"], value=item["oracle_value"],
                                                           truth=truth[b["id"]], field=item["field"], kept_for=item.get("kept_for", [])))
    out, pairs, firsts, kinds = [], 0, 0, []
    for hand, bs in sorted(by_hand.items()):
        best_value = max(bs, key=lambda x: x["value"])
        best_truth = max(bs, key=lambda x: x["truth"])
        firsts += best_truth["truth"] - best_value["truth"] < a.gap
        for x in bs:
            if x["kept_for"] and x["truth"] >= best_value["truth"] + a.gap:
                kinds.append((hand, x["kept_for"], x["truth"] - best_value["truth"]))
        for x in bs:
            for y in bs:
                if x["value"] > y["value"] and y["truth"] >= x["truth"] + a.gap:
                    pairs += 1
                    out.append(dict(hand=hand, deck=x["deck"], fixture=f"hunt-{hand}", gap=round(y["truth"] - x["truth"], 3),
                                    first_by_measure=dict(board=x["index"], value=round(x["value"], 3), truth=round(x["truth"], 3), field=x["field"]),
                                    first_by_truth=dict(board=y["index"], value=round(y["value"], 3), truth=round(y["truth"], 3), field=y["field"],
                                                        kept_for=y["kept_for"])))
    out.sort(key=lambda d: (-d["gap"], d["hand"]))
    hands = len(by_hand)
    doc = dict(gap=a.gap, hands=hands, boards=sum(len(v) for v in by_hand.values()), pairs=pairs,
               best_by_measure_is_best_by_truth=firsts, kinds_better_than_best=len(kinds), disagreements=out)
    json.dump(doc, open(a.out, "w"), indent=1)
    decks = {}
    for d in out:
        decks[d["deck"]] = decks.get(d["deck"], 0) + 1
    print(f"{hands} hands, {doc['boards']} boards with a truth; {pairs} pairs where the measure and the truth disagree by {a.gap} or more")
    print(f"the measure's best board is the truth's best (within {a.gap}) in {firsts}/{hands} hands")
    print(f"a board kept for its kind beats the measure's best by {a.gap} or more in {len(kinds)} hands: "
          + ", ".join(f"{h} ({'/'.join(k)}, +{g:.2f})" for h, k, g in kinds[:10]))
    print("by deck: " + ", ".join(f"{d} {n}" for d, n in sorted(decks.items(), key=lambda x: -x[1])))
    for d in out[:a.show]:
        m, t = d["first_by_measure"], d["first_by_truth"]
        print(f"  {d['hand']}: #{m['board']} value {m['value']:.2f} truth {m['truth']:.2f} < #{t['board']} value {t['value']:.2f} "
              f"truth {t['truth']:.2f} (+{d['gap']:.2f})  {', '.join(t['field'])}")


def cmd_lines(a):
    """The lines of a fixture's boards as `combos --save-lines` writes them, for `probe-line`."""
    fx = json.load(open(a.fixture))
    items = [{"answers": [list(bytes.fromhex(x)) for x in b["line"]]} for b in fx["boards"]]
    json.dump({"kind": "lines", "deck": fx["deck"], "hand": fx["hand"], "seed": fx["seed"], "items": items}, open(a.out, "w"))
    print(f"{len(items)} lines in {a.out}")


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
    b.add_argument("--only", default="", help="only the hands whose id holds this; several, comma-separated")
    b.add_argument("--staged", help="CHEAP,FULL: the cheap measure of the first CHEAP fields, the full one of the best FULL by it "
                                    f"(default: the full measure of the first {TOP})")
    b.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    f = sub.add_parser("fixtures", help="write tests/fixtures/oracle from a build")
    f.add_argument("oracle")
    f.add_argument("dest")
    f.add_argument("--decks", required=True)
    f.add_argument("--keep", type=int, default=5, help="boards kept per hand")
    f.add_argument("--auto-goals", type=int, default=0)
    c = sub.add_parser("converge", help="mark the fixtures stable or provisional from larger builds")
    c.add_argument("fixtures")
    c.add_argument("builds", nargs="+", help="build folders with a larger --mult")
    ah = sub.add_parser("as-hunt", help="the fixtures as a combo_hunt --json folder, for objective_bench.py")
    ah.add_argument("fixtures")
    ah.add_argument("out")
    dg = sub.add_parser("disagree", help="where the measure and the level 2 disagree on the oracle's boards")
    dg.add_argument("bench", help="objective_bench.py output folder (run on an as-hunt folder)")
    dg.add_argument("hunt", help="the as-hunt folder")
    dg.add_argument("out", help="JSON report")
    dg.add_argument("--gap", type=float, default=0.05)
    dg.add_argument("--show", type=int, default=20)
    ln = sub.add_parser("lines", help="a fixture's lines for probe-line")
    ln.add_argument("fixture")
    ln.add_argument("out")
    m = sub.add_parser("measure", help="the normal search against the oracle")
    m.add_argument("oracle", help="folder of fixtures")
    m.add_argument("out")
    m.add_argument("--decks", required=True)
    m.add_argument("--budgets", default="1000,2000,4000,8000")
    m.add_argument("--jobs", type=int, default=4)
    m.add_argument("--only", default="")
    m.add_argument("--staged", help="CHEAP,FULL: measure as combos --staged (default: the full measure of the first 10)")
    m.add_argument("--combos-args", default="", help="more options for combos, as one string (e.g. \"--checkpoints\")")
    m.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    a = p.parse_args()
    {"hands": cmd_hands, "build": cmd_build, "fixtures": cmd_fixtures, "measure": cmd_measure, "converge": cmd_converge,
     "lines": cmd_lines, "as-hunt": cmd_as_hunt, "disagree": cmd_disagree}[a.cmd](a)


if __name__ == "__main__":
    main()
