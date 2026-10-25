#!/usr/bin/env python3
"""What the dry check reads on the frozen bench's hands
(docs/plans/linee-interazione.md §6.2): `combos --capacity --json` on each
hand, then over all the boards listed:

- the effects checked, and how many the check could not tell (a Lua error:
  assumed usable);
- the interruptions by class and by place, usable or blocked, and why;
- what the usable ones assume (the opponent's cards, the event, a phase);
- the capacity of the first board by the usual order and of the best by
  capacity: how often ranking by capacity changes the board on top.

Writes <out>/<hand id>.json, <out>/report.json and a ten-line
<out>/summary.txt.

    python3 scripts/capacity_report.py $S/capacity --cli target/release/duelforge-cli --set quick --jobs 4
"""
import argparse
import collections
import concurrent.futures as cf
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import banco  # noqa: E402


def run(cli, case, nodes, top, out):
    path = os.path.join(out, case["id"] + ".json")
    if os.path.exists(path):
        return path, 0
    cmd = [cli, "--data", os.path.join(banco.ROOT, "data"), "combos", "--deck", case["path"], "--hand", case["hand"], "--seed",
           str(case["seed"]), "--nodes", str(nodes), "--seconds", "86400", "--threads", "1", "--top", str(top), "--capacity", "--json", path]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        with open(os.path.join(out, case["id"] + ".err"), "w") as f:
            f.write(r.stderr[-2000:])
    return path, r.returncode


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--cli", required=True)
    p.add_argument("--set", choices=["quick", "full"], default="quick")
    p.add_argument("--nodes", type=int, default=1000)
    p.add_argument("--top", type=int, default=10)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--version", default="v1")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    _, _, cases = banco.load(a.version)
    cases = [c for c in cases if a.set in c["sets"]]
    with cf.ThreadPoolExecutor(a.jobs) as pool:
        done = list(pool.map(lambda c: run(a.cli, c, a.nodes, a.top, a.out), cases))
    failed = [c["id"] for c, (_, rc) in zip(cases, done) if rc != 0]

    effects = unchecked = boards = errors = 0
    usable = collections.Counter()
    blocked = collections.Counter()
    places = collections.Counter()
    assumes = collections.Counter()
    tops = []
    for c, (path, rc) in zip(cases, done):
        if rc != 0 or not os.path.exists(path):
            continue
        items = json.load(open(path))["items"]
        caps = []
        for item in items:
            cap = item.get("capacity")
            if cap is None:
                continue
            if "error" in cap:
                errors += 1
                continue
            boards += 1
            effects += cap["effects_checked"]
            unchecked += cap["unchecked"]
            for it in cap["interruptions"]:
                if it["blocked"]:
                    blocked[it["blocked"]] += 1
                    continue
                usable[it["class"]] += 1
                places[it["place"]] += 1
                for x in it["assumes"]:
                    assumes[x] += 1
            caps.append(cap["count"])
        if caps:
            # The items are in the search's order: its first board, and the best by capacity.
            tops.append(dict(id=c["id"], first=caps[0], best=max(caps)))
    total_usable = sum(usable.values())
    changed = sum(1 for t in tops if t["best"] > t["first"])
    report = dict(hands=len(cases), failed=failed, boards=boards, dry_check_errors=errors, effects_checked=effects, unchecked=unchecked,
                  usable_by_class=dict(usable), usable_by_place=dict(places), blocked=dict(blocked), assumes=dict(assumes), tops=tops)
    json.dump(report, open(os.path.join(a.out, "report.json"), "w"), indent=1)
    mean = lambda xs: sum(xs) / len(xs) if xs else 0.0  # noqa: E731
    lines = [
        f"capacity report, bench {a.version} set {a.set}: {len(cases)} hands, {len(failed)} runs failed, {a.nodes} nodes, top {a.top}",
        f"boards checked: {boards}; dry checks failed: {errors}",
        f"effects checked: {effects}; checks that could not tell (Lua error, assumed usable): {unchecked}"
        f" ({100 * unchecked / max(effects, 1):.1f}%)",
        f"interruptions usable: {total_usable}; blocked by your board: {sum(blocked.values())} ("
        + ", ".join(f"{k} {v}" for k, v in blocked.most_common()) + ")",
        "usable by class: " + ", ".join(f"{k} {v}" for k, v in usable.most_common()),
        "usable by place: " + ", ".join(f"{k} {v}" for k, v in places.most_common()),
        "usable that assume: " + ", ".join(f"{k} {v}" for k, v in assumes.most_common()),
        f"capacity of the search's first board: mean {mean([t['first'] for t in tops]):.2f}; of the best listed by capacity:"
        f" {mean([t['best'] for t in tops]):.2f}",
        f"hands where ranking by capacity puts another board on top with more interruptions: {changed} of {len(tops)}",
        f"failed: {', '.join(failed) if failed else 'none'}",
    ]
    open(os.path.join(a.out, "summary.txt"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
