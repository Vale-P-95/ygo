#!/usr/bin/env python3
"""The generality bench (docs/plans/revisione-motore.md, section 4.2; the
rules in docs/plans/banco-generalita.md): is the engine worse on decks
nobody tuned it on?

Two sets of decks, frozen by their lists' hashes in
tests/fixtures/generality/sets.json (the lists are downloaded, never
committed):
- tuning: the Master Duel Meta lists the engine has been tuned on;
- fresh: decks no change was ever measured on. Tournament lists from
  YGOPRODeck (TCG and OCG, archetypes not yet in Master Duel too) and the
  newest Master Duel Meta lists, kept only when they share little with every
  tuning deck: of their cards that are not staples (in 6 tuning decks or
  more), at most a quarter in any one tuning deck. No deck is chosen by name;
  the decks of the bench (tests/fixtures/hands/bench.tsv) count as tuned on
  even when their list is gone, so a fresh deck of theirs is left out.

For each deck, two hands (drawn as `combo_hunt.py` draws them, seed 7) are
searched at the normal budget and at --mult times it, one thread each. The
number per hand is the best board measured at the normal budget over the
best at the large one (the bench's value): how close the search gets with
the time it is given. The number per set is its mean; the gap is tuning
minus fresh, in points. The runs also count what breaks: runs that fail,
probe turns that fail, interrupted lines.

    python3 scripts/generality.py freeze $S/gen --tuning $S/mdm            # once: writes the lists and sets.json
    python3 scripts/generality.py check $S/gen                             # the lists match sets.json
    python3 scripts/generality.py run $S/gen/runs-main --sets $S/gen --cli <cli> --jobs 4
    python3 scripts/generality.py report $S/gen/runs-main [--against $S/gen/runs-branch]
"""
import argparse
import collections
import concurrent.futures as cf
import hashlib
import json
import os
import random
import re
import shutil
import sqlite3
import statistics
import subprocess
import sys
import time
import urllib.parse
import urllib.request

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oracle import value  # noqa: E402

MANIFEST = os.path.join(ROOT, "tests", "fixtures", "generality", "sets.json")
YGOPRODECK = "https://ygoprodeck.com/api/decks/getDecks.php?format={fmt}&offset={offset}"
FORMATS = ("Tournament Meta Decks", "Tournament Meta Decks OCG")
MDM = "https://www.masterduelmeta.com/api/v1/top-decks?sort=-created&limit=150"
AGENT = {"User-Agent": "DuelForge deck tests"}
# A card in this many tuning decks is a staple: it says nothing of the archetype.
STAPLE = 6
# A fresh deck shares at most this share of its other cards with any tuning deck,
# and two fresh decks sharing more than FRESH_SAME are one archetype (the newest kept).
OVERLAP = 1 / 4
FRESH_SAME = 0.5
# The normal search's budget, and the hands per deck.
NODES = 4000
HANDS = 2
SEED = 7
TOP = 10


def slug(name):
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def ydk_cards(path):
    text = open(path).read()
    main, _, rest = text.partition("#extra")
    extra = rest.split("!side")[0]
    return [w for w in main.split() if w.isdigit()], [w for w in extra.split() if w.isdigit()]


def deck_hash(main, extra):
    """The list's id: Main and Extra Deck passcodes in order."""
    return hashlib.sha256((" ".join(main) + " | " + " ".join(extra)).encode()).hexdigest()[:16]


def write_ydk(path, main, extra):
    with open(path, "w") as f:
        f.write("#main\n" + "\n".join(main) + "\n#extra\n" + "\n".join(extra) + "\n!side\n")


def get_json(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers=AGENT), timeout=60) as r:
        return json.load(r)


def names_of(db, codes):
    """Card names (one per artwork) for passcodes; unknown ones as None."""
    out = []
    for c in codes:
        row = db.execute("select name from texts where id = ?", (int(c),)).fetchone()
        out.append(row[0] if row else None)
    return out


def candidates(db):
    """Fresh candidates, the newest first: (name, source, main, extra)."""
    out = []
    for fmt in FORMATS:
        for offset in range(0, 200, 20):
            page = get_json(YGOPRODECK.format(fmt=urllib.parse.quote(fmt), offset=offset))
            if not isinstance(page, list) or not page:
                break
            for d in page:
                main, extra = json.loads(d["main_deck"] or "[]"), json.loads(d["extra_deck"] or "[]")
                source = f"ygoprodeck {fmt} {d.get('pretty_url')} ({d.get('tournamentName') or ''})"
                out.append((d["deck_name"], source, [str(c) for c in main], [str(c) for c in extra]))
    for d in get_json(MDM):
        main, extra = [], []
        for part, cards in (("main", main), ("extra", extra)):
            for e in d.get(part, []):
                row = db.execute("select id from texts where lower(name) = lower(?)", (e["card"]["name"],)).fetchone()
                if row:
                    cards.extend([str(row[0])] * e["amount"])
        kind = (d.get("deckType") or {}).get("name") or "?"
        out.append((kind, f"masterduelmeta {d.get('_id')} {d.get('created', '')[:10]}", main, extra))
    return out


def cmd_freeze(a):
    db = sqlite3.connect(os.path.join(ROOT, "data", "cdb", "cards.cdb"))
    os.makedirs(os.path.join(a.out, "tuning"), exist_ok=True)
    os.makedirs(os.path.join(a.out, "fresh"), exist_ok=True)
    tuning = {}
    for f in sorted(os.listdir(a.tuning)):
        if f.endswith(".ydk"):
            main, extra = ydk_cards(os.path.join(a.tuning, f))
            tuning[f[:-4]] = (main, extra)
            shutil.copy(os.path.join(a.tuning, f), os.path.join(a.out, "tuning", f))
    tuning_names = {k: {n for n in names_of(db, m) if n} for k, (m, _) in tuning.items()}
    # The bench's decks were tuned on too (their Master Duel Meta lists may be gone).
    bench = {line.split("\t")[1] for line in open(os.path.join(ROOT, "tests", "fixtures", "hands", "bench.tsv"))
             if line.strip() and not line.startswith("#")}
    count = collections.Counter(n for names in tuning_names.values() for n in names)
    staples = {n for n, c in count.items() if c >= STAPLE}
    fresh, fresh_names = [], []
    for name, source, main, extra in candidates(db):
        names = names_of(db, main + extra)
        if any(slug(name) == b or slug(name).startswith(b + "_") for b in bench):
            continue
        if len(main) < 40 or None in names:
            continue  # a short list, or a card the engine does not know
        own = {n for n in names_of(db, main) if n} - staples
        if not own:
            continue
        share = max(len(own & t) / len(own) for t in tuning_names.values())
        if share > OVERLAP or any(len(own & f) / len(own) > FRESH_SAME for f in fresh_names):
            continue
        fresh_names.append(own)
        fresh.append(dict(deck=slug(name), name=name, source=source, main=main, extra=extra, tuning_share=round(share, 2)))
        if len(fresh) >= a.count:
            break
    seen = collections.Counter()
    sets = dict(made=time.strftime("%Y-%m-%d"), nodes=NODES, hands=HANDS, seed=SEED, staple=STAPLE, overlap=round(OVERLAP, 3),
                tuning=[], fresh=[])
    for k, (main, extra) in tuning.items():
        sets["tuning"].append(dict(deck=k, source="masterduelmeta (scripts/mdm_decks.py)", hash=deck_hash(main, extra)))
    for d in fresh:
        seen[d["deck"]] += 1
        deck = d["deck"] if seen[d["deck"]] == 1 else f"{d['deck']}_{seen[d['deck']]}"
        write_ydk(os.path.join(a.out, "fresh", deck + ".ydk"), d["main"], d["extra"])
        sets["fresh"].append(dict(deck=deck, name=d["name"], source=d["source"], tuning_share=d["tuning_share"], hash=deck_hash(d["main"], d["extra"])))
    json.dump(sets, open(os.path.join(a.out, "sets.json"), "w"), indent=1)
    print(f"{len(sets['tuning'])} tuning decks, {len(sets['fresh'])} fresh ({len(staples)} staples); copy {a.out}/sets.json to {MANIFEST}")


def check(sets_dir, manifest):
    """Decks of the manifest whose list is missing or changed."""
    sets = json.load(open(manifest))
    bad = []
    for kind in ("tuning", "fresh"):
        for d in sets[kind]:
            path = os.path.join(sets_dir, kind, d["deck"] + ".ydk")
            if not os.path.exists(path) or deck_hash(*ydk_cards(path)) != d["hash"]:
                bad.append(f"{kind}/{d['deck']}")
    return sets, bad


def cmd_check(a):
    _, bad = check(a.sets, a.manifest)
    print("all lists match" if not bad else "not matching: " + ", ".join(bad))
    return 1 if bad else 0


def run_one(cli, path, hand, nodes, out_json, extra):
    command = [cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", path, "--hand", hand, "--nodes", str(nodes), "--seconds",
               "86400", "--threads", "1", "--top", str(TOP), "--seed", str(SEED), "--probe", "--json", out_json] + extra
    started = time.time()
    r = subprocess.run(command, capture_output=True, text=True)
    out = r.stdout
    m = re.search(r"(\d+) interrupted lines", out)
    return dict(rc=r.returncode, seconds=round(time.time() - started, 1), probe_failed=out.count("probe turn failed"),
                interrupted=int(m.group(1)) if m else None, stderr=r.stderr[-300:] if r.returncode else "")


def cmd_run(a):
    sets, bad = check(a.sets, a.manifest)
    if bad:
        sys.exit("lists not matching the manifest: " + ", ".join(bad))
    os.makedirs(a.out, exist_ok=True)
    extra = a.combos_args.split() if a.combos_args else []
    todo = []
    for kind in ("tuning", "fresh"):
        for d in sets[kind]:
            path = os.path.join(a.sets, kind, d["deck"] + ".ydk")
            main, _ = ydk_cards(path)
            for k in range(HANDS):
                hand = ";".join(random.Random(f"{d['deck']}-{k}").sample(main, 5))
                for budget, nodes in (("normal", NODES), ("large", NODES * a.mult)):
                    name = f"{kind}.{d['deck']}-{k}.{budget}"
                    if not os.path.exists(os.path.join(a.out, name + ".run.json")):
                        todo.append((name, path, hand, nodes))

    def one(job):
        name, path, hand, nodes = job
        r = run_one(a.cli, path, hand, nodes, os.path.join(a.out, name + ".json"), extra)
        json.dump(r, open(os.path.join(a.out, name + ".run.json"), "w"))
        return name, r

    print(f"{len(todo)} runs", flush=True)
    with cf.ThreadPoolExecutor(a.jobs) as pool:
        for name, r in pool.map(one, todo):
            print(f"{name}: rc {r['rc']} {r['seconds']} s, probe failed {r['probe_failed']}, interrupted {r['interrupted']}", flush=True)


def best(path):
    if not os.path.exists(path):
        return None
    vals = [value(i["measure"]) for i in json.load(open(path))["items"] if i.get("measure") and i.get("stage") != "cheap"]
    return max(vals, default=0.0)


def rows(out):
    """Per hand: set, deck, the ratio normal/large, and what broke."""
    hands = {}
    for f in sorted(os.listdir(out)):
        m = re.match(r"(tuning|fresh)\.(.+)\.(normal|large)\.run\.json$", f)
        if m:
            hands.setdefault((m[1], m[2]), {})[m[3]] = json.load(open(os.path.join(out, f)))
    result = []
    for (kind, hand), runs in sorted(hands.items()):
        n, l = best(os.path.join(out, f"{kind}.{hand}.normal.json")), best(os.path.join(out, f"{kind}.{hand}.large.json"))
        ratio = None if n is None or l is None else (1.0 if l <= 0 else min(n / l, 1.0))
        broken = sum(r["rc"] != 0 for r in runs.values())
        result.append(dict(set=kind, hand=hand, ratio=ratio, failed_runs=broken,
                           probe_failed=sum(r["probe_failed"] for r in runs.values()),
                           interrupted=sum(r["interrupted"] or 0 for r in runs.values()),
                           seconds=sum(r["seconds"] for r in runs.values())))
    return result


def summary(rs):
    out = {}
    for kind in ("tuning", "fresh"):
        s = [r for r in rs if r["set"] == kind]
        ratios = [r["ratio"] for r in s if r["ratio"] is not None]
        out[kind] = dict(hands=len(s), value=100 * statistics.fmean(ratios) if ratios else float("nan"),
                         at95=sum(x >= 0.95 for x in ratios), failed_runs=sum(r["failed_runs"] for r in s),
                         probe_failed=sum(r["probe_failed"] for r in s), interrupted=sum(r["interrupted"] for r in s))
    out["gap"] = out["tuning"]["value"] - out["fresh"]["value"]
    return out


def cmd_report(a):
    rs = rows(a.out)
    s = summary(rs)
    if a.list:
        for r in rs:
            ratio = "-" if r["ratio"] is None else f"{100 * r['ratio']:5.1f}%"
            print(f"  {r['set']:6} {r['hand']:34} {ratio} failed {r['failed_runs']} probe {r['probe_failed']} interrupted {r['interrupted']}")
    for kind in ("tuning", "fresh"):
        k = s[kind]
        print(f"{kind:6}: {k['hands']} hands, value {k['value']:.1f}% of the large budget, {k['at95']} hands at 95%+, "
              f"{k['failed_runs']} failed runs, {k['probe_failed']} failed probe turns, {k['interrupted']} interrupted lines")
    print(f"gap (tuning - fresh): {s['gap']:+.1f} points")
    if a.against:
        b = summary(rows(a.against))
        for kind in ("tuning", "fresh"):
            print(f"{kind:6}: {s[kind]['value']:.1f}% -> {b[kind]['value']:.1f}% ({b[kind]['value'] - s[kind]['value']:+.1f})")
        print(f"gap: {s['gap']:+.1f} -> {b['gap']:+.1f} points")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    f = sub.add_parser("freeze", help="choose the fresh decks and write the lists and sets.json")
    f.add_argument("out")
    f.add_argument("--tuning", required=True, help="folder of the tuning .ydk (the Master Duel Meta lists used so far)")
    f.add_argument("--count", type=int, default=20, help="fresh decks to keep")
    c = sub.add_parser("check", help="the lists match the manifest")
    c.add_argument("sets")
    c.add_argument("--manifest", default=MANIFEST)
    r = sub.add_parser("run", help="the runs, resumable")
    r.add_argument("out")
    r.add_argument("--sets", required=True, help="the folder with tuning/ and fresh/ lists")
    r.add_argument("--manifest", default=MANIFEST)
    r.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    r.add_argument("--mult", type=int, default=10, help="the large budget, in normal budgets")
    r.add_argument("--jobs", type=int, default=4)
    r.add_argument("--combos-args", default="", help="more options for combos (e.g. --staged)")
    s = sub.add_parser("report", help="the numbers per set and the gap")
    s.add_argument("out")
    s.add_argument("--against", help="another run's folder (after a change)")
    s.add_argument("--list", action="store_true")
    a = p.parse_args()
    return {"freeze": cmd_freeze, "check": cmd_check, "run": cmd_run, "report": cmd_report}[a.cmd](a) or 0


if __name__ == "__main__":
    sys.exit(main())
