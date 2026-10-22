#!/usr/bin/env python3
"""The objective bench of board strength (docs/plans/banco-oggettivo.md):
how much of the real turn of the meta's decks a board stops, to calibrate
the weights of the score (#47 D17, #43) on results instead of opinions.

Steps, each one resumable:
- meta: the decks the most played on Master Duel Meta (`popRank` of the
  deck types, the share of the ranked top decks), with their weights;
- select: for every deck of --decks, the first 5 boards with distinct fields
  of `combo_hunt.py --hands 2 --top 8 --seed 1 --json` (in the printed
  order), written as lines files of their own;
- run: `probe-line --vs <opponent> --moves --json` for each lines file and
  each opponent (all its boards at once: the opponent's lines are read
  once), with a time limit;
- report: one row per board and opponent (level 2, level 1, and the
  board's generic measures), the calibration/check split, and tables.

    python3 scripts/mdm_decks.py --out $S/mdm
    python3 scripts/combo_hunt.py $S/mdm --hands 2 --top 8 --seed 1 --json $S/hunt
    python3 scripts/objective_bench.py $S/bench --decks $S/mdm --hunt $S/hunt
    python3 scripts/objective_bench.py $S/bench --decks $S/mdm --hunt $S/hunt --skip-run
"""
import argparse
import concurrent.futures as cf
import glob
import json
import os
import random
import re
import subprocess
import time
import urllib.request

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
TYPES = "https://www.masterduelmeta.com/api/v1/deck-types?sort=-popRank&limit={limit}"

LEVEL1 = re.compile(r"against the deck \(([\d.]+) s\): resilience ([\d.]+)( \(standard breakers\))? · breaker in hand (\d+)% · "
                    r"a handtrap that stops the line (\d+)%")
LEVEL2 = re.compile(r"against its lines \(([\d.]+) s\): (\d+) lines · (\d+)% of their moves made on an empty board · value ([\d.]+) · "
                    r"floodgates stop (\d+)% · (\d+)% of the rest left with all your answers")
ITS = re.compile(r"its lines \(([\d.]+) s\): (\d+) from (\d+) hands")


def slug(name):
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def meta(out, decks, count):
    """The opponents: the most played deck types with a list in --decks."""
    request = urllib.request.Request(TYPES.format(limit=100), headers={"User-Agent": "DuelForge deck tests"})
    with urllib.request.urlopen(request, timeout=60) as response:
        types = json.load(response)
    have = {os.path.splitext(f)[0] for f in os.listdir(decks) if f.endswith(".ydk")}
    rows = [dict(deck=slug(t["name"]), name=t["name"], pop=t.get("popRank") or 0, tier=t.get("tier")) for t in types]
    rows = [r for r in rows if r["pop"] > 0 and r["deck"] in have][:count]
    total = sum(r["pop"] for r in rows)
    for r in rows:
        r["weight"] = round(r["pop"] / total, 4)
    doc = dict(source="masterduelmeta.com /api/v1/deck-types popRank (share of the ranked top decks, %)",
               fetched=time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime()), opponents=rows)
    json.dump(doc, open(os.path.join(out, "meta.json"), "w"), indent=1)
    return doc


def field_key(item):
    """The field without zones: boards that differ only there are one."""
    return re.sub(r" \[EMZ\]", "", item["signature"].split(" | hand:")[0])


def select(out, hunt, per_deck):
    """The first boards with distinct fields of each deck, hand 0 first."""
    os.makedirs(os.path.join(out, "lines"), exist_ok=True)
    boards = []
    decks = sorted({re.sub(r"-\d+$", "", os.path.basename(p)[:-5]) for p in glob.glob(os.path.join(hunt, "*.json")) if not p.endswith(".lines.json")})
    for deck in decks:
        seen, chosen = set(), []
        for path in sorted(glob.glob(os.path.join(hunt, f"{deck}-*.json"))):
            if path.endswith(".lines.json") or len(chosen) >= per_deck:
                continue
            items = json.load(open(path))["items"]
            lines = json.load(open(path[:-5] + ".lines.json"))
            keep = []
            for k, it in enumerate(items):
                key = field_key(it)
                # A board with nothing on the field is the empty board: nothing to measure.
                if len(chosen) + len(keep) < per_deck and key not in seen and it["field"] and it.get("measure"):
                    seen.add(key)
                    keep.append(k)
            if not keep:
                continue
            hand_file = os.path.basename(path)[:-5]
            sub = dict(lines, items=[lines["items"][k] for k in keep])
            json.dump(sub, open(os.path.join(out, "lines", hand_file + ".lines.json"), "w"))
            for i, k in enumerate(keep):
                chosen.append(dict(id=f"{hand_file}#{k}", deck=deck, hand_file=hand_file, index=i, rank=k, signature=items[k]["signature"]))
        boards += chosen
    json.dump(boards, open(os.path.join(out, "boards.json"), "w"), indent=1)
    return boards


def run_one(cli, decks, out, hand_file, deck, opp, limit):
    path = os.path.join(out, "runs", f"{hand_file}.vs.{opp}")
    if os.path.exists(path + ".log"):
        return
    started = time.time()
    try:
        r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", os.path.join(decks, deck + ".ydk"),
                            "--lines", os.path.join(out, "lines", hand_file + ".lines.json"), "--vs", os.path.join(decks, opp + ".ydk"),
                            "--moves", "--json", path + ".json"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=limit)
        text, status = r.stdout, f"rc {r.returncode}"
    except subprocess.TimeoutExpired:
        text, status = "", "timeout"
    open(path + ".log", "w").write(f"# wall {time.time() - started:.1f} {status}\n" + text)


def parse(path):
    text = open(path + ".log").read()
    wall, status = re.match(r"# wall ([\d.]+) (.*)", text).groups()
    its = ITS.search(text)
    out = dict(wall=float(wall), status=status, its_s=float(its[1]) if its else None, boards={})
    items = json.load(open(path + ".json"))["items"] if os.path.exists(path + ".json") else []
    blocks = re.split(r"(?m)^#\d+ field: ", text)[1:]
    for k, block in enumerate(blocks):
        b = {}
        m = LEVEL1.search(block)
        if m:
            b.update(l1_s=float(m[1]), l1_resilience=float(m[2]), standard_breakers=bool(m[3]), breaker_chance=int(m[4]) / 100,
                     hand_trap_chance=int(m[5]) / 100)
        m = LEVEL2.search(block)
        if m:
            b.update(l2_s=float(m[1]), l2_lines=int(m[2]), made=int(m[3]) / 100, l2_value=float(m[4]), floodgates_stop=int(m[5]) / 100,
                     left=int(m[6]) / 100)
        b["failed"] = "failed" in block
        if k < len(items):
            it = items[k]
            ms = it.get("measure") or {}
            b["generic"] = {key: ms.get(key) for key in ("field", "hand", "floodgates", "resilience", "follow_up")}
            b["generic"]["countered"] = len(ms["countered"]) if isinstance(ms.get("countered"), list) else ms.get("countered")
            b["generic"]["exposure"] = it.get("exposure")
        out["boards"][k] = b
    return out


def split(decks, seed, share):
    """Calibration and check, by deck (a deck's boards stay together)."""
    order = sorted(decks)
    random.Random(seed).shuffle(order)
    cut = round(len(order) * share)
    return {d: ("taratura" if i < cut else "verifica") for i, d in enumerate(order)}


def report(out, boards, doc, seed):
    opps = doc["opponents"]
    weight = {o["deck"]: o["weight"] for o in opps}
    parts = split({b["deck"] for b in boards}, seed, 0.7)
    rows, runs, dubious, timeouts, failures = [], [], [], [], []
    for b in boards:
        for o in opps:
            path = os.path.join(out, "runs", f"{b['hand_file']}.vs.{o['deck']}")
            if not os.path.exists(path + ".log"):
                continue
            r = PARSED.setdefault(path, parse(path))
            m = r["boards"].get(b["index"], {})
            row = dict(board=b["id"], deck=b["deck"], split=parts[b["deck"]], rank=b["rank"], opponent=o["deck"], weight=o["weight"],
                       status=r["status"], **{k: v for k, v in m.items() if k != "generic"}, generic=m.get("generic"))
            rows.append(row)
            if r["status"] == "timeout":
                timeouts.append((b["hand_file"], o["deck"]))
            elif "made" in m and m["made"] < 0.8:
                dubious.append((b["deck"], o["deck"], m["made"]))
            if m.get("failed"):
                failures.append((b["id"], o["deck"]))
    for path, r in PARSED.items():
        runs.append(r["wall"])
    json.dump(dict(meta=doc, split_seed=seed, rows=rows), open(os.path.join(out, "results.json"), "w"))
    # Per board: the meta-weighted "left" and stops, next to the generic measures.
    lines = ["# Banco oggettivo: risultati", "",
             f"- board: {len(boards)} di {len({b['deck'] for b in boards})} deck; avversari: {len(opps)}; righe: {len(rows)}",
             f"- invocazioni: {len(runs)}, tempo totale {sum(runs) / 3600:.2f} h, mediana {sorted(runs)[len(runs) // 2] if runs else 0:.1f} s; "
             f"oltre il limite: {len(set(timeouts))}",
             f"- coppie dubbie (mosse fatte su campo vuoto < 80%): {len({(d, o) for d, o, _ in dubious})}", ""]
    lines += ["| Board | Split | Left pesato | Value pesato | Fermata handtrap | Resilienza vs | Campo | Mano | Floodgate | Resilienza |",
              "|---|---|---|---|---|---|---|---|---|---|"]
    for b in boards:
        rs = [r for r in rows if r["board"] == b["id"] and "left" in r]
        if not rs:
            continue
        w = sum(r["weight"] for r in rs)
        avg = lambda k: sum(r["weight"] * r.get(k, 0) for r in rs) / w  # noqa: E731
        g = rs[0]["generic"] or {}
        lines.append(f"| {b['id']} | {parts[b['deck']]} | {avg('left'):.2f} | {avg('l2_value'):.2f} | {avg('hand_trap_chance'):.2f} | "
                     f"{avg('l1_resilience'):.2f} | {g.get('field') or 0:.2f} | {g.get('hand') or 0:.2f} | {g.get('floodgates') or 0:.2f} | "
                     f"{g.get('resilience') or 0:.2f} |")
    lines += ["", "## Coppie dubbie", ""] + [f"- {d} contro {o}: {m:.0%}" for d, o, m in sorted(set(dubious))]
    lines += ["", "## Oltre il limite di tempo", ""] + [f"- {h} contro {o}" for h, o in sorted(set(timeouts))]
    lines += ["", "## Misure fallite", ""] + [f"- {b} contro {o}" for b, o in sorted(set(failures))]
    open(os.path.join(out, "report.md"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines[:6]))


PARSED = {}


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--decks", required=True, help="the Master Duel Meta lists (mdm_decks.py)")
    p.add_argument("--hunt", required=True, help="combo_hunt.py --hands 2 --top 8 --seed 1 --json folder")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--opponents", type=int, default=20)
    p.add_argument("--per-deck", type=int, default=5)
    p.add_argument("--limit", type=int, default=300, help="seconds for one invocation")
    p.add_argument("--jobs", type=int, default=3)
    p.add_argument("--skip", default="", help="comma-separated opponents not to run (too slow), noted in the report")
    p.add_argument("--split-seed", type=int, default=47)
    p.add_argument("--skip-run", action="store_true", help="only the tables, from what is there")
    a = p.parse_args()
    os.makedirs(os.path.join(a.out, "runs"), exist_ok=True)
    mpath = os.path.join(a.out, "meta.json")
    doc = json.load(open(mpath)) if os.path.exists(mpath) else meta(a.out, a.decks, a.opponents)
    bpath = os.path.join(a.out, "boards.json")
    boards = json.load(open(bpath)) if os.path.exists(bpath) else select(a.out, a.hunt, a.per_deck)
    if not a.skip_run:
        skip = set(filter(None, a.skip.split(",")))
        files = sorted({(b["hand_file"], b["deck"]) for b in boards})
        jobs = [(h, d, o["deck"]) for o in doc["opponents"] if o["deck"] not in skip for h, d in files]
        with cf.ThreadPoolExecutor(a.jobs) as ex:
            list(ex.map(lambda j: run_one(a.cli, a.decks, a.out, j[0], j[1], j[2], a.limit), jobs))
    report(a.out, boards, doc, a.split_seed)


if __name__ == "__main__":
    main()
