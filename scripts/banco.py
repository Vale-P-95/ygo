#!/usr/bin/env python3
"""The frozen engine bench (rules in docs/plans/regole-motore.md): lists,
hands and seeds fixed in tests/fixtures/banco-motore/<version>, committed, so
the same test can be run again on any day and any machine; and the rules that
accept a change to the engine, the search or the measure, with thresholds
fixed beforehand in that version's rules.json.

- run: both builds on the same hands, one thread and a node budget each (the
  same boards on any machine), old and new one after the other on the same
  worker in alternating order, so the times compare too;
- same: a change that must not move any output (speed, memory, refactoring):
  every JSON equal, numbers within 1e-9, and the times paired;
- judge: a change to the search: the paired rules (non-inferiority with a
  bootstrap over decks, sign test, big losses, fresh decks, breakages);
  with --measure-cli, the best board of both sides measured again by that
  build, for a change to the measure (both on the new scale).

    python3 scripts/banco.py check
    python3 scripts/banco.py run $S/banco --old <cli of main> --new <cli of the branch> --set full --jobs 4
    python3 scripts/banco.py same $S/banco
    python3 scripts/banco.py judge $S/banco [--measure-cli <cli of the branch>]
    # a new version (rotation): the lists of `generality.py rebuild`, new hands
    python3 scripts/banco.py freeze v2 --sets data/generality

Each command writes <out>/summary.txt (ten lines) and its JSON; the exit code
is 0 only when the change is accepted.
"""
import argparse
import concurrent.futures as cf
import datetime
import json
import math
import os
import platform
import random
import re
import sqlite3
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from generality import MANIFEST, deck_hash, write_ydk, ydk_cards  # noqa: E402
from oracle import HANDS, ROOT, boards_of, value  # noqa: E402

BENCH = os.path.join(ROOT, "tests", "fixtures", "banco-motore")
VERSION = "v1"
# Numbers closer than this (relative) are the same: the sums of the measure
# may change their last digit with the order of the additions.
TOLERANCE = 1e-9


def folder(version):
    return os.path.join(BENCH, version)


def load(version):
    """The version's manifest, rules and hands: (manifest, rules, [case])."""
    base = folder(version)
    manifest = json.load(open(os.path.join(base, "manifest.json")))
    rules = json.load(open(os.path.join(base, "rules.json")))
    cases = []
    for line in open(os.path.join(base, "hands.tsv"), encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        hid, kind, deck, seed, hand, sets = line.rstrip("\n").split("\t")
        cases.append(dict(id=hid, kind=kind, deck=deck, seed=int(seed), hand=hand, sets=sets.split(","),
                          path=os.path.join(base, "decks", kind, deck + ".ydk")))
    return manifest, rules, cases


def cmd_check(a):
    """The committed lists match the manifest's hashes."""
    manifest, _, cases = load(a.version)
    bad = [f"{d['set']}/{d['deck']}" for d in manifest["decks"]
           if deck_hash(*ydk_cards(os.path.join(folder(a.version), "decks", d["set"], d["deck"] + ".ydk"))) != d["hash"]]
    print(f"{len(manifest['decks']) - len(bad)} of {len(manifest['decks'])} lists match; {len(cases)} hands")
    if bad:
        print("changed: " + ", ".join(bad))
    return 1 if bad else 0


def cmd_freeze(a):
    """A new version from the lists of `generality.py rebuild` (tuning/ and
    fresh/): the lists copied, HANDS_PER_DECK hands per deck drawn once with a
    fixed seed and written as passcodes, and the bench's named hands
    (tests/fixtures/hands/bench.tsv) of the decks kept."""
    base = folder(a.version)
    if os.path.exists(base):
        sys.exit(f"{base} exists: a version is never changed, freeze a new one")
    sources = json.load(open(a.manifest))
    db = sqlite3.connect(os.path.join(ROOT, "data", "cdb", "cards.cdb"))
    decks, rows = [], []
    for kind in ("tuning", "fresh"):
        os.makedirs(os.path.join(base, "decks", kind))
        for d in sources[kind]:
            path = os.path.join(a.sets, kind, d["deck"] + ".ydk")
            if not os.path.exists(path):
                print(f"{kind}/{d['deck']}: no list, left out")
                continue
            main, extra = ydk_cards(path)
            write_ydk(os.path.join(base, "decks", kind, d["deck"] + ".ydk"), main, extra)
            h = deck_hash(main, extra)
            decks.append(dict(deck=d["deck"], set=kind, source=d["source"], ref=d["ref"], hash=h,
                              **({"manifest_hash": d["hash"]} if h != d["hash"] else {})))
            for k in range(a.hands):
                hand = random.Random(f"banco-{a.version}-{d['deck']}-{k}").sample(main, 5)
                # The first hand of every deck is the quick set.
                rows.append((f"{kind}.{d['deck']}.{k}", kind, d["deck"], k + 1, ";".join(hand), "quick,full" if k == 0 else "full"))
    kept = {d["deck"]: d["set"] for d in decks}
    for line in open(HANDS, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        name, deck, hand = line.rstrip("\n").split("\t")
        if deck not in kept or hand == "random5":
            continue
        main = ydk_cards(os.path.join(base, "decks", kept[deck], deck + ".ydk"))[0]
        codes = passcodes(db, hand.split(";"), main)
        if codes is None:
            print(f"bench hand {name}: its cards are not in today's {deck}, left out")
            continue
        rows.append((f"bench.{name}", kept[deck], deck, 1, ";".join(codes), "full"))
    manifest = dict(version=a.version, made=datetime.date.today().isoformat(), from_manifest=os.path.relpath(a.manifest, ROOT),
                    hands_per_deck=a.hands, decks=decks)
    json.dump(manifest, open(os.path.join(base, "manifest.json"), "w"), indent=1)
    with open(os.path.join(base, "hands.tsv"), "w", encoding="utf-8") as f:
        f.write("# id\tset\tdeck\tseed\thand (passcodes)\tsets of runs\n")
        for r in rows:
            f.write("\t".join(map(str, r)) + "\n")
    print(f"{len(decks)} lists, {len(rows)} hands in {base}; write rules.json and README.md by hand")


def passcodes(db, hand, main):
    """The hand's cards (names or passcodes) as the passcodes of the copies in `main`, or None when one is missing."""
    left, out = list(main), []
    for card in hand:
        names = db.execute("select name from texts where id = ?", (int(card),)).fetchall() if card.isdigit() else [(card,)]
        codes = {str(r[0]) for (name,) in names for r in db.execute("select id from texts where name = ?", (name,))}
        copy = next((c for c in left if c in codes), None)
        if copy is None:
            return None
        left.remove(copy)
        out.append(copy)
    return out


def combos_command(cli, rules, nodes, path):
    return [cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", path, "--nodes", str(nodes or rules["nodes"]), "--seconds", "86400",
            "--threads", "1", "--top", str(rules["top"])]


def write_info(out, case, rc, wall, stdout, stderr, jobs):
    """<id>.run.json: what broke and the times of one hand."""
    m = re.search(r"after ([\d.]+) s: .*?(\d+) interrupted lines", stdout)
    info = dict(rc=rc, wall=round(wall, 2), host=platform.node(), jobs=jobs, search=float(m[1]) if m else None, interrupted=int(m[2]) if m else None,
                probe_failed=stdout.count("probe turn failed"), stderr=stderr[-300:] if rc else "")
    json.dump(info, open(os.path.join(out, case["id"] + ".run.json"), "w"))
    return info


def run_one(cli, case, rules, out, lines, jobs, nodes=None):
    """One `combos` run into out/<id>.json, with what broke and the times in <id>.run.json."""
    command = combos_command(cli, rules, nodes, case["path"]) + ["--hand", case["hand"], "--seed", str(case["seed"]),
                                                                  "--json", os.path.join(out, case["id"] + ".json")]
    if lines:
        command += ["--save-lines", os.path.join(out, case["id"] + ".lines.json")]
    started = time.time()
    r = subprocess.run(command, capture_output=True, text=True)
    return write_info(out, case, r.returncode, time.time() - started, r.stdout, r.stderr, jobs)


_batches = {}


def batches(cli):
    """Whether `combos` of this build plays several hands in one run (`--cases`)."""
    if cli not in _batches:
        r = subprocess.run([cli, "combos", "--help"], capture_output=True, text=True)
        _batches[cli] = "--cases" in r.stdout
    return _batches[cli]


def run_deck(cli, cases, rules, out, lines, jobs, nodes=None):
    """The hands of one deck in one `combos --cases` run: the engine is
    loaded once. Each hand gets its own files, as with run_one,
    and its own time from the run (the first one also pays for the loading)."""
    listing = os.path.join(out, cases[0]["id"] + ".cases.tsv")
    with open(listing, "w") as f:
        for c in cases:
            f.write("\t".join([c["id"], str(c["seed"]), c["hand"], os.path.join(out, c["id"] + ".json"),
                               os.path.join(out, c["id"] + ".lines.json") if lines else ""]) + "\n")
    r = subprocess.run(combos_command(cli, rules, nodes, cases[0]["path"]) + ["--cases", listing], capture_output=True, text=True)
    parts = re.split(r"^=== case (\S+)\n", r.stdout, flags=re.M)
    sections = dict(zip(parts[1::2], parts[2::2]))
    for c in cases:
        text = sections.get(c["id"], "")
        m = re.search(r"^=== case \S+ (done|failed) in ([\d.]+) s", text, flags=re.M)
        # A hand without its end marker broke the whole run.
        rc = 0 if m and m[1] == "done" else 1
        write_info(out, c, rc, float(m[2]) if m else 0.0, text, r.stderr if rc else "", jobs)
    os.remove(listing)


def cmd_run(a):
    _, rules, cases = load(a.version)
    if cmd_check(a):
        sys.exit("the lists do not match the manifest")
    cases = [c for c in cases if a.set in c["sets"] and a.only in c["id"]]
    # The hands of a deck together: a build that can plays them in one run.
    decks = {}
    for c in cases:
        decks.setdefault(c["path"], []).append(c)
    groups = list(decks.values())
    if a.shard:
        # One of several runners (bench.yml): every n-th deck.
        i, n = map(int, a.shard.split("/"))
        groups = groups[i::n]
        cases = [c for g in groups for c in g]
    sides = [(s, cli) for s, cli in (("old", a.old), ("new", a.new)) if cli]
    nodes = {"old": a.old_nodes, "new": a.new_nodes}
    for side, cli in sides:
        os.makedirs(os.path.join(a.out, side), exist_ok=True)
        json.dump(dict(cli=os.path.abspath(cli), version=a.version, set=a.set, jobs=a.jobs, nodes=nodes[side] or rules["nodes"], host=platform.node(),
                       cpus=os.cpu_count(), machine=platform.machine()), open(os.path.join(a.out, side, "host.json"), "w"))

    def one(job):
        i, group = job
        # Old then new on the same worker, the order alternating: a machine
        # slower for a while weighs on both sides alike.
        for side, cli in sides if i % 2 == 0 else sides[::-1]:
            out = os.path.join(a.out, side)
            todo = [c for c in group if not os.path.exists(os.path.join(out, c["id"] + ".run.json"))]
            if todo and batches(cli):
                run_deck(cli, todo, rules, out, not a.no_lines, a.jobs, nodes[side])
            else:
                for case in todo:
                    run_one(cli, case, rules, out, not a.no_lines, a.jobs, nodes[side])
        if a.measure_cli:
            # A change to the measure: both best boards measured again here, on the same worker.
            for case in group:
                for side, _ in sides:
                    info = os.path.join(a.out, side, case["id"] + ".run.json")
                    path = os.path.join(a.out, side, case["id"] + ".json")
                    best_value(a, case, side, (json.load(open(info)), path if os.path.exists(path) else None))
        return os.path.basename(group[0]["path"])

    started = time.time()
    with cf.ThreadPoolExecutor(a.jobs) as pool:
        for n, deck in enumerate(pool.map(one, enumerate(groups)), 1):
            print(f"{n}/{len(groups)} {deck}", flush=True)
    broken = {side: [c["id"] for c in cases if json.load(open(os.path.join(a.out, side, c["id"] + ".run.json")))["rc"] != 0]
              for side, _ in sides}
    summary(a.out, [f"banco {a.version}, set {a.set}: {len(cases)} hands, {rules['nodes']} nodes, {a.jobs} jobs, "
                    f"{(time.time() - started) / 60:.0f} min"] + [f"{side}: {len(b)} failed runs {b[:5]}" for side, b in broken.items()])


def summary(out, lines, name="summary.txt"):
    text = "\n".join(lines[:10]) + "\n"
    open(os.path.join(out, name), "w").write(text)
    print(text, end="")


def runs(out, side, cases):
    """{id: (run info, path of the JSON or None)} of one side."""
    got = {}
    for c in cases:
        info = os.path.join(out, side, c["id"] + ".run.json")
        if os.path.exists(info):
            path = os.path.join(out, side, c["id"] + ".json")
            got[c["id"]] = (json.load(open(info)), path if os.path.exists(path) else None)
    return got


def canonical(x):
    """A run's JSON without what may differ between two equal runs: times and the deck's path."""
    if isinstance(x, dict):
        return {k: (os.path.basename(v) if k == "deck" and isinstance(v, str) else canonical(v)) for k, v in x.items() if "seconds" not in k}
    if isinstance(x, list):
        return [canonical(v) for v in x]
    return x


def first_difference(a, b, where=""):
    """Where two canonical JSON differ (numbers within TOLERANCE are equal), or None."""
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                return f"{where}.{k} (only on one side)"
            d = first_difference(a[k], b[k], f"{where}.{k}")
            if d:
                return d
        return None
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return f"{where} ({len(a)} against {len(b)} entries)"
        return next((d for d in (first_difference(x, y, f"{where}[{i}]") for i, (x, y) in enumerate(zip(a, b))) if d), None)
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool) and not isinstance(b, bool):
        return None if abs(a - b) <= TOLERANCE * max(1.0, abs(a), abs(b)) else f"{where} ({a} against {b})"
    return None if a == b else f"{where} ({str(a)[:40]} against {str(b)[:40]})"


def bootstrap(values_by_deck, statistic, rules):
    """The statistic over resamples of the decks (the hands of a deck are not
    independent): its value and the (alpha, 1 - alpha) percentiles."""
    decks = sorted(values_by_deck)
    rng = random.Random(rules["bootstrap_seed"])
    whole = statistic([v for d in decks for v in values_by_deck[d]])
    draws = sorted(statistic([v for _ in decks for v in values_by_deck[rng.choice(decks)]]) for _ in range(rules["bootstrap"]))
    lo, hi = draws[int(rules["alpha"] * len(draws))], draws[int((1 - rules["alpha"]) * len(draws)) - 1]
    return whole, lo, hi


def by_deck(rows, key):
    out = {}
    for r in rows:
        out.setdefault(r["deck"], []).append(r[key])
    return out


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


def times(a, cases, old, new, rules):
    """The new/old ratio of the run times, geometric mean over the hands, and its interval."""
    rows = [dict(deck=c["deck"], r=math.log(new[c["id"]][0]["wall"] / old[c["id"]][0]["wall"])) for c in cases
            if c["id"] in old and c["id"] in new and old[c["id"]][0]["wall"] > 0 and new[c["id"]][0]["wall"] > 0]
    if not rows:
        return None
    # Times compare only when both sides of every hand ran on the same machine with the same jobs.
    same_machine = all(old[c["id"]][0].get("host") == new[c["id"]][0].get("host") is not None
                       and old[c["id"]][0].get("jobs") == new[c["id"]][0].get("jobs") for c in cases if c["id"] in old and c["id"] in new)
    ratio, lo, hi = (math.exp(x) for x in bootstrap(by_deck(rows, "r"), mean, rules))
    return dict(ratio=ratio, lo=lo, hi=hi, same_machine=same_machine)


def cmd_same(a):
    _, rules, cases = load(a.version)
    old, new = runs(a.out, "old", cases), runs(a.out, "new", cases)
    cases = [c for c in cases if c["id"] in old and c["id"] in new]
    diffs, broken = [], []
    for c in cases:
        (io, po), (i_n, pn) = old[c["id"]], new[c["id"]]
        if io["rc"] != i_n["rc"] or (po is None) != (pn is None):
            broken.append(c["id"])
            continue
        for suffix in (".json", ".lines.json"):
            fo, fn = os.path.join(a.out, "old", c["id"] + suffix), os.path.join(a.out, "new", c["id"] + suffix)
            if os.path.exists(fo) and os.path.exists(fn):
                d = first_difference(canonical(json.load(open(fo))), canonical(json.load(open(fn))))
                if d:
                    diffs.append((c["id"] + suffix, d))
                    break
    t = times(a, cases, old, new, rules)
    slower = t is not None and t["same_machine"] and t["lo"] > rules["slowdown_block"]
    accepted = not diffs and not broken and not slower and len(cases) > 0
    lines = [f"same, banco {a.version}: {len(cases)} hands, {len(diffs)} with a different output, {len(broken)} failing on one side only"]
    lines += [f"  {hid}: {d}" for hid, d in diffs[:4]]
    if t:
        lines.append(f"time new/old {t['ratio']:.3f} ({1 - 2 * rules['alpha']:.0%} interval {t['lo']:.3f}-{t['hi']:.3f})"
                     + ("" if t["same_machine"] else ", NOT on the same machine and jobs: the times do not count"))
    lines.append("ACCEPTED" if accepted else "REJECTED" + (" (slower)" if slower and not diffs else ""))
    json.dump(dict(hands=len(cases), diffs=diffs, broken=broken, times=t, accepted=accepted), open(os.path.join(a.out, "same.json"), "w"), indent=1)
    summary(a.out, lines)
    return 0 if accepted else 1


def best_value(a, case, side, entry):
    """The value of the side's best board; with --measure-cli, that board measured again by it."""
    info, path = entry
    if path is None:
        return None
    found = boards_of(path)
    if not found:
        return 0.0
    if not a.measure_cli:
        return found[0][0]
    lines = os.path.join(a.out, side, case["id"] + ".lines.json")
    again = os.path.join(a.out, side, case["id"] + ".again.json")
    if not os.path.exists(again):
        index = json.load(open(path))["items"].index(found[0][1])
        subprocess.run([a.measure_cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", case["path"], "--hand", case["hand"],
                        "--seed", str(case["seed"]), "--lines", lines, "--index", str(index), "--json", again], capture_output=True)
    try:
        return value(json.load(open(again))["items"][0]["measure"])
    except (OSError, ValueError, KeyError, IndexError):
        return None


def verdict_of(rows, rules, margin):
    """The paired rules on some hands: non-inferiority, sign test, big losses."""
    wins = sum(r["change"] > rules["even"] for r in rows)
    losses = sum(r["change"] < -rules["even"] for r in rows)
    n = wins + losses
    # One-sided: the chance of this many losses or more if a loss were as likely as a win.
    p_worse = sum(math.comb(n, i) for i in range(losses, n + 1)) / 2**n if n else 1.0
    p_better = sum(math.comb(n, i) for i in range(wins, n + 1)) / 2**n if n else 1.0
    big = [r["id"] for r in rows if r["change"] < -rules["big_loss"]]
    big_wins = sum(r["change"] > rules["big_loss"] for r in rows)
    capped = [dict(deck=r["deck"], c=max(-1.0, min(1.0, r["change"]))) for r in rows]
    m, lo, hi = bootstrap(by_deck(capped, "c"), mean, rules) if rows else (0.0, 0.0, 0.0)
    checks = {
        f"lower bound of the mean change >= -{margin:.0%}": lo >= -margin,
        f"sign test, more losses than wins: p >= {rules['alpha']}": p_worse >= rules["alpha"],
        # A search that only reorders loses big on some hands and wins big on as many.
        f"hands losing over {rules['big_loss']:.0%}: at most {rules['big_loss_share']:.0%} or the big wins":
            len(big) <= max(rules["big_loss_share"] * len(rows), big_wins),
    }
    better = lo > 0 and p_better < rules["alpha"]
    return dict(hands=len(rows), wins=wins, losses=losses, even=len(rows) - n, mean=m, lo=lo, hi=hi, p_worse=p_worse,
                p_better=p_better, big=big, checks=checks, ok=all(checks.values()), better=better)


def cmd_judge(a):
    _, rules, cases = load(a.version)
    old, new = runs(a.out, "old", cases), runs(a.out, "new", cases)
    cases = [c for c in cases if c["id"] in old and c["id"] in new]
    rows, newly_broken = [], []
    for c in cases:
        (io, _), (i_n, _) = old[c["id"]], new[c["id"]]
        # What breaks: a run, a probe turn or a line that the old build did not break.
        for what in ("rc", "probe_failed", "interrupted"):
            if (i_n.get(what) or 0) > (io.get(what) or 0):
                newly_broken.append(f"{c['id']} {what}")
        vo, vn = best_value(a, c, "old", old[c["id"]]), best_value(a, c, "new", new[c["id"]])
        if vo is None or vn is None:
            continue
        change = (vn - vo) / vo if vo > 0 else (0.0 if vn <= 0 else 1.0)
        rows.append(dict(id=c["id"], deck=c["deck"], kind=c["kind"], old=vo, new=vn, change=change))
    every = verdict_of(rows, rules, rules["margin"])
    fresh = verdict_of([r for r in rows if r["kind"] == "fresh"], rules, rules["margin_fresh"])
    t = times(a, cases, old, new, rules)
    slower = t is not None and t["same_machine"] and t["lo"] > rules["slowdown_needs_gain"]
    checks = dict(every["checks"])
    # 60 fresh hands are too few for a margin on the lower bound (calibration of
    # 24/10: 90% half-width of 5 points): their mean, and no proven loss.
    checks[f"fresh decks: mean change >= -{rules['margin_fresh']:.0%}"] = fresh["mean"] >= -rules["margin_fresh"]
    checks["fresh decks: not proven worse (upper bound >= 0)"] = fresh["hi"] >= 0
    checks["nothing new breaks (runs, probe turns, lines)"] = not newly_broken
    checks["every hand measured on both sides"] = len(rows) == len(cases) > 0
    if slower:
        checks[f"slower by over {rules['slowdown_needs_gain'] - 1:.0%}: needs a gain"] = every["better"]
    accepted = all(checks.values())
    s = lambda v: f"{v['hands']} hands {v['wins']}+ {v['losses']}- {v['even']}=, mean {v['mean']:+.2%} ({v['lo']:+.2%} to {v['hi']:+.2%})"
    lines = [f"judge, banco {a.version}" + (" (measured again by --measure-cli)" if a.measure_cli else ""),
             "all: " + s(every) + f", sign p {every['p_worse']:.3f}", "fresh: " + s(fresh),
             f"big losses: {', '.join(every['big'][:5]) or 'none'}; new breakages: {', '.join(newly_broken[:3]) or 'none'}"]
    if t:
        lines.append(f"time new/old {t['ratio']:.3f} ({t['lo']:.3f}-{t['hi']:.3f})" + ("" if t["same_machine"] else ", other machine: not counted"))
    lines += [f"FAILED: {k}" for k, v in checks.items() if not v][:3]
    lines.append(("ACCEPTED" if accepted else "REJECTED") + (", BETTER (gain shown)" if every["better"] else ""))
    json.dump(dict(rows=rows, all=every, fresh=fresh, times=t, checks=checks, accepted=accepted), open(os.path.join(a.out, "judge.json"), "w"), indent=1)
    summary(a.out, lines)
    return 0 if accepted else 1


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--version", default=VERSION, help="the bench's version (a folder of tests/fixtures/banco-motore)")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check", help="the lists match the manifest")
    f = sub.add_parser("freeze", help="a new version from rebuilt lists")
    f.add_argument("new_version")
    f.add_argument("--sets", required=True, help="the folder with tuning/ and fresh/ lists of `generality.py rebuild`")
    f.add_argument("--manifest", default=MANIFEST)
    f.add_argument("--hands", type=int, default=3, help="hands per deck")
    r = sub.add_parser("run", help="the runs of both builds, resumable")
    r.add_argument("out")
    r.add_argument("--old", help="duelforge-cli before the change")
    r.add_argument("--new", help="duelforge-cli after the change")
    r.add_argument("--set", default="full", choices=("quick", "full"))
    r.add_argument("--only", default="", help="only the hands whose id holds this")
    r.add_argument("--jobs", type=int, default=4)
    # Only to calibrate the bench (a known change of the budget): never for a verdict on a PR.
    r.add_argument("--old-nodes", type=int, help="calibration only: another node budget for the old side")
    r.add_argument("--new-nodes", type=int, help="calibration only: another node budget for the new side")
    r.add_argument("--shard", help="I/N: only every N-th hand from the I-th (several runners)")
    r.add_argument("--no-lines", action="store_true", help="do not save the lines (no --measure-cli later)")
    r.add_argument("--measure-cli", help="also measure the best board of both sides again with this build (for judge --measure-cli)")
    for name, help_ in (("same", "a change that must not move any output"), ("judge", "the paired rules for a change to the search")):
        c = sub.add_parser(name, help=help_)
        c.add_argument("out")
        if name == "judge":
            c.add_argument("--measure-cli", help="measure the best board of both sides again with this build (a change to the measure)")
    a = p.parse_args()
    if a.cmd == "freeze":
        a.version = a.new_version
    return {"check": cmd_check, "freeze": cmd_freeze, "run": cmd_run, "same": cmd_same, "judge": cmd_judge}[a.cmd](a) or 0


if __name__ == "__main__":
    sys.exit(main())
