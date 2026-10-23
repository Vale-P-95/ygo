#!/usr/bin/env python3
"""The robustness sweep (docs/plans/revisione-motore.md, section 4.2): the
whole pipeline on decks nobody chose, to find what breaks before a user does.

- Decks: `duelforge-cli random-decks`, one per archetype of the card pool
  (its Main Deck cards x3, random cards to fill), the same for the same seed.
- For each deck, hands drawn as `combo_hunt.py` draws them: the search and
  the probe turn (`combos --probe --save-lines`), then every saved line
  played again (`probe-line`), as the Combo Lab plays them for the
  explanation and the level 2.
- Then `duelforge-cli sweep`: random duels with random answers, for the
  decisions no answer can be given to and the answers the engine rejects.

Every problem is written with the command that reproduces it:
- crash: a run that panics or ends with an error;
- interrupted: lines the search had to stop (an answer refused, a time limit);
- probe failed: a probe turn that could not be played;
- replay: a saved line that `probe-line` cannot play again or measure;
- no answer / rejected: from the duel sweep.

    python3 scripts/robustness.py $S/rob --count 100 --jobs 4
    python3 scripts/robustness.py $S/rob --report
"""
import argparse
import concurrent.futures as cf
import json
import os
import random
import re
import shlex
import subprocess
import sys
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
SEED = 7


def cli_cmd(cli, *args):
    return [cli, "--data", os.path.join(ROOT, "data"), *map(str, args)]


def run(command, timeout):
    started = time.time()
    try:
        r = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
        return dict(rc=r.returncode, out=r.stdout, err=r.stderr, seconds=round(time.time() - started, 1))
    except subprocess.TimeoutExpired:
        return dict(rc="timeout", out="", err="", seconds=timeout)


def problems_of(kind_run, r, command):
    """The problems a run shows, each with its command."""
    out = []
    repro = " ".join(shlex.quote(c) for c in command)
    panicked = "panicked at" in r["err"]
    if r["rc"] != 0 or panicked:
        detail = (re.search(r"panicked at[^\n]*\n[^\n]*", r["err"]) or re.search(r"Error:[^\n]*", r["err"]) or [r["err"][-300:]])[0]
        out.append(dict(kind="crash", where=kind_run, detail=f"rc {r['rc']}: {detail.strip()}", repro=repro))
    m = re.search(r"(\d+) interrupted lines", r["out"])
    if m and int(m.group(1)) > 0:
        out.append(dict(kind="interrupted", where=kind_run, detail=f"{m.group(1)} interrupted lines", repro=repro))
    failed = re.findall(r"probe turn failed: ([^\n]*)", r["out"])
    if failed:
        out.append(dict(kind="probe failed" if kind_run == "combos" else "replay", where=kind_run,
                        detail=f"{len(failed)}: {sorted(set(failed))[:3]}", repro=repro))
    return out


def one_hand(a, deck, k):
    name = os.path.basename(deck)[:-4]
    main = [w for w in open(deck).read().split("#extra")[0].split() if w.isdigit()]
    hand = ";".join(random.Random(f"{name}-{k}").sample(main, 5))
    base = os.path.join(a.out, "runs", f"{name}-{k}")
    combos = cli_cmd(a.cli, "combos", "--deck", deck, "--hand", hand, "--nodes", a.nodes, "--threads", 1, "--top", a.top, "--seed", SEED,
                     "--probe", "--json", base + ".json", "--save-lines", base + ".lines.json")
    r = run(combos, a.timeout)
    found = problems_of("combos", r, combos)
    if os.path.exists(base + ".lines.json"):
        replay = cli_cmd(a.cli, "probe-line", "--deck", deck, "--lines", base + ".lines.json", "--json", base + ".replay.json")
        rr = run(replay, a.timeout)
        found += problems_of("probe-line", rr, replay)
        seconds = r["seconds"] + rr["seconds"]
    else:
        seconds = r["seconds"]
    head = next((l for l in r["out"].splitlines() if l.startswith(("NodeLimit", "TimeLimit", "Exhausted"))), "")
    result = dict(hand=f"{name}-{k}", deck=deck, seconds=seconds, head=head, problems=found)
    json.dump(result, open(base + ".result.json", "w"), indent=1)
    return result


def cmd_run(a):
    decks_dir = os.path.join(a.out, "decks")
    if not os.path.isdir(decks_dir):
        r = subprocess.run(cli_cmd(a.cli, "random-decks", "--count", a.count, "--seed", a.seed, "--out", decks_dir), capture_output=True, text=True)
        print(r.stdout.strip() or r.stderr.strip())
    os.makedirs(os.path.join(a.out, "runs"), exist_ok=True)
    decks = sorted(os.path.join(decks_dir, f) for f in os.listdir(decks_dir) if f.endswith(".ydk"))
    todo = [(d, k) for d in decks for k in range(a.hands)
            if not os.path.exists(os.path.join(a.out, "runs", f"{os.path.basename(d)[:-4]}-{k}.result.json"))]
    print(f"{len(todo)} hands", flush=True)
    with cf.ThreadPoolExecutor(a.jobs) as pool:
        for res in pool.map(lambda t: one_hand(a, *t), todo):
            tag = ", ".join(p["kind"] for p in res["problems"]) or "ok"
            print(f"{res['hand']}: {res['seconds']} s {tag}", flush=True)
    sweep_path = os.path.join(a.out, "sweep.txt")
    if a.duels and not os.path.exists(sweep_path):
        command = cli_cmd(a.cli, "sweep", "--duels", a.duels, "--seed", a.seed, "--threads", a.jobs)
        r = run(command, 7200)
        open(sweep_path, "w").write("# " + " ".join(shlex.quote(c) for c in command) + f"\n# rc {r['rc']}\n" + r["out"] + r["err"][-2000:])
    cmd_report(a)


def cmd_report(a):
    results = [json.load(open(os.path.join(a.out, "runs", f))) for f in sorted(os.listdir(os.path.join(a.out, "runs"))) if f.endswith(".result.json")]
    problems = [dict(p, hand=r["hand"]) for r in results for p in r["problems"]]
    sweep_path = os.path.join(a.out, "sweep.txt")
    if os.path.exists(sweep_path):
        text = open(sweep_path).read()
        repro = text.splitlines()[0][2:]
        for m in re.finditer(r"^(NoAnswer|Rejected|Engine) in duel (\d+) at decision (\d+): (.*)\n\s+(.*)$", text, re.M):
            kind = {"NoAnswer": "no answer", "Rejected": "rejected", "Engine": "crash"}[m[1]]
            problems.append(dict(kind=kind, where="sweep", hand=f"duel {m[2]}", detail=f"decision {m[3]}: {m[4][:200]} {m[5][:200]}", repro=repro))
    counts = {}
    for p in problems:
        counts[p["kind"]] = counts.get(p["kind"], 0) + 1
    hands_with = len({p["hand"] for p in problems if p["where"] != "sweep"})
    print(f"{len(results)} hands of {len({r['deck'] for r in results})} decks, {hands_with} with a problem; "
          + (", ".join(f"{k} {v}" for k, v in sorted(counts.items())) or "no problems"))
    json.dump(problems, open(os.path.join(a.out, "problems.json"), "w"), indent=1)
    if a.list:
        for p in problems:
            print(f"- {p['kind']} · {p['hand']} · {p['detail']}\n    {p['repro']}")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--count", type=int, default=100, help="decks, one archetype each")
    p.add_argument("--seed", type=int, default=1, help="seed of the decks and of the duel sweep")
    p.add_argument("--hands", type=int, default=2)
    p.add_argument("--nodes", type=int, default=1000)
    p.add_argument("--top", type=int, default=5, help="boards measured and played again per hand")
    p.add_argument("--duels", type=int, default=500, help="duels of the random sweep (0: none)")
    p.add_argument("--timeout", type=int, default=900, help="seconds a run may take")
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--report", action="store_true", help="only the report of what is already run")
    p.add_argument("--list", action="store_true", help="every problem with its command")
    a = p.parse_args()
    (cmd_report if a.report else cmd_run)(a)


if __name__ == "__main__":
    sys.exit(main())
