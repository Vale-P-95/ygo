#!/usr/bin/env python3
"""The owner's pairs of boards (tests/fixtures/boards/owner-pairs.json) in the
objective bench (docs/plans/banco-oggettivo.md): the owner's verdict next to
the bench's truth, today's generic measure and the score of before.

Steps:
- prep: find each board again, by its signature without zones (and the same
  GY and banished cards), in the --hunt folders of combo_hunt.py --json, and
  write <out>/boards.json and lines files as objective_bench.py reads them.
  The own decks are copied as own_<deck>.ydk next to the meta's decks, in
  <out>/decks, since a deck can be both;
- measure: `probe-line` of each lines file with the current CLI (the generic
  measure of today), and a check that the line still makes the same board;
- then `objective_bench.py <out> --decks <out>/decks` for level 2 against the
  meta, and `--table` for the table per pair.

    python3 scripts/owner_bench.py $S/owner --own $S/mdm2 --meta-decks $S/mdm6 --meta $S/bench/meta.json --hunt $S/h7m $S/h7bm $S/op_make
    python3 scripts/objective_bench.py $S/owner --decks $S/owner/decks --hunt - --skip lunalight
    python3 scripts/owner_bench.py $S/owner --table tests/fixtures/boards/owner-verdicts.tsv
"""
import argparse
import glob
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import WEIGHTS  # noqa: E402
from board_pairs import unzoned  # noqa: E402
from objective_bench import spearman  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
PAIRS = os.path.join(ROOT, "tests", "fixtures", "boards", "owner-pairs.json")
# Below these differences two boards count as equal.
TIE_TRUTH, TIE_SCORE = 0.02, 0.05


def names(xs):
    return sorted(x["name"] if isinstance(x, dict) else x for x in (xs or []))


def composite(m):
    return sum(w * (m.get(k) or 0) for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience")))


def locate(hunts, deck, side, cache):
    """The first board with the side's signature: with the same GY and banished cards when the pair has their names."""
    sig = unzoned(side["signature"])
    found = []
    for n, hunt in enumerate(hunts):
        for f in sorted(glob.glob(os.path.join(hunt, f"{deck}-*.json")) + glob.glob(os.path.join(hunt, f"{deck}.json"))):
            if f.endswith(".lines.json"):
                continue
            if f not in cache:
                cache[f] = json.load(open(f)).get("items", [])
            for k, it in enumerate(cache[f]):
                if unzoned(it["signature"]) != sig:
                    continue
                same = not isinstance(side.get("grave"), list) or (
                    names(side["grave"]) == names(it.get("grave_names")) and names(side.get("banished")) == names(it.get("banished_names")))
                found.append((not same, n, f, k))
    return min(found) if found else None


def prep(out, own, meta_decks, meta, hunts):
    os.makedirs(os.path.join(out, "lines"), exist_ok=True)
    os.makedirs(os.path.join(out, "decks"), exist_ok=True)
    for f in glob.glob(os.path.join(meta_decks, "*.ydk")):
        shutil.copy(f, os.path.join(out, "decks"))
    for f in glob.glob(os.path.join(own, "*.ydk")):
        shutil.copy(f, os.path.join(out, "decks", "own_" + os.path.basename(f)))
    shutil.copy(meta, os.path.join(out, "meta.json"))
    groups, boards, missing, cache = {}, [], [], {}
    for p in json.load(open(PAIRS))["pairs"]:
        for s in "ab":
            side = p[s]
            # A board rebuilt from its signature has no line to play.
            loc = None if side.get("rebuilt") else locate(hunts, p["deck"], side, cache)
            if not loc:
                missing.append(f"{p['id']}{s}")
                continue
            mismatch, n, f, k = loc
            hand_file = f"{os.path.basename(os.path.normpath(hunts[n]))}-{os.path.basename(f)[:-5]}"
            g = groups.setdefault(hand_file, dict(src=f, ks=[]))
            if k not in g["ks"]:
                g["ks"].append(k)
            boards.append(dict(id=f"{p['id']}{s}", pair=p["id"], side=s, deck="own_" + p["deck"], hand_file=hand_file, src=f, rank=k,
                               signature=side["signature"], grave_mismatch=mismatch))
    write_lines(out, boards)
    json.dump([dict(id=m, why="not found") for m in missing], open(os.path.join(out, "missing.json"), "w"), indent=1)
    print(f"{len(boards)} boards; not found: {', '.join(missing) or '-'}")
    print("GY or banished different:", ", ".join(b["id"] for b in boards if b["grave_mismatch"]) or "-")


def write_lines(out, boards):
    """One lines file per source file, with only the boards kept, and their index in it."""
    for hand_file in {b["hand_file"] for b in boards}:
        bs = [b for b in boards if b["hand_file"] == hand_file]
        ks = sorted({b["rank"] for b in bs})
        lines = json.load(open(bs[0]["src"][:-5] + ".lines.json"))
        json.dump(dict(lines, items=[lines["items"][k] for k in ks]), open(os.path.join(out, "lines", hand_file + ".lines.json"), "w"))
        for b in bs:
            b["index"] = ks.index(b["rank"])
    json.dump(boards, open(os.path.join(out, "boards.json"), "w"), indent=1)


def measure(out, cli):
    """The generic measure of today, one board at a time: a line that no longer plays fails alone."""
    os.makedirs(os.path.join(out, "plain"), exist_ok=True)
    boards = json.load(open(os.path.join(out, "boards.json")))
    for b in boards:
        path = os.path.join(out, "plain", b["id"])
        # Alone, from its source: one line that no longer plays fails only itself.
        if not os.path.exists(path + ".lines.json"):
            lines = json.load(open(b["src"][:-5] + ".lines.json"))
            json.dump(dict(lines, items=[lines["items"][b["rank"]]]), open(path + ".lines.json", "w"))
        if not os.path.exists(path + ".log"):
            r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", os.path.join(out, "decks", b["deck"] + ".ydk"),
                                "--lines", path + ".lines.json",
                                "--json", path + ".json"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            open(path + ".log", "w").write(f"# rc {r.returncode}\n" + r.stdout)
        it = json.load(open(path + ".json"))["items"][0] if os.path.exists(path + ".json") else None
        b["today"] = round(composite(it["measure"]), 3) if it and it.get("measure") else None
        b["same_board"] = bool(it) and unzoned(it["signature"]) == unzoned(b["signature"])
    # The boards a line no longer makes stay out of the run against the meta.
    bad = [b for b in boards if not b["same_board"]]
    missing = json.load(open(os.path.join(out, "missing.json")))
    missing += [dict(id=b["id"], why="the line no longer makes the board") for b in bad if b["id"] not in {m["id"] for m in missing}]
    json.dump(missing, open(os.path.join(out, "missing.json"), "w"), indent=1)
    write_lines(out, [b for b in boards if b["same_board"]])
    print(f"measured {len(boards)}; the line no longer makes the board: {', '.join(b['id'] for b in bad) or '-'}")


def verdict_of(delta, tie):
    return "pari" if delta is None or abs(delta) < tie else ("B" if delta > 0 else "A")


def table(out, verdicts_path):
    pairs = {p["id"]: p for p in json.load(open(PAIRS))["pairs"]}
    rows = [line.rstrip("\n").split("\t") for line in open(verdicts_path) if line.strip() and not line.startswith("#")]
    head, rows = rows[0], rows[1:]
    owner = {r[head.index("id")]: r[head.index("verdict")] for r in rows}
    boards = {b["id"]: b for b in json.load(open(os.path.join(out, "boards.json")))}
    missing = {m["id"]: m["why"] for m in json.load(open(os.path.join(out, "missing.json")))}
    tpath = os.path.join(out, "truth.json")
    truth = {t["board"]: t for t in json.load(open(tpath))} if os.path.exists(tpath) else {}
    lines = ["| Coppia | Deck | Tipo | Proprietario | Verità A | Verità B | B−A | Generica oggi B−A | Punteggio di prima B−A | Note |",
             "|---|---|---|---|---|---|---|---|---|---|"]
    agree = {k: [0, 0] for k in ("truth", "today", "before")}
    xs = {k: [] for k in ("truth", "today", "before")}
    for pid, p in pairs.items():
        v = owner.get(pid, "")
        a, b = boards.get(pid + "a"), boards.get(pid + "b")
        notes = []
        if p["a"].get("rebuilt"):
            notes.append("ricostruite dalla firma: nessuna linea")
        elif not a or not b:
            notes.append("la linea non rifà più la board: " + ", ".join(i for i in (pid + "a", pid + "b") if i in missing))
        ta, tb = (truth.get(pid + s, {}).get("truth") for s in "ab")
        dt = tb - ta if ta is not None and tb is not None else None
        dg = b["today"] - a["today"] if a and b and a.get("today") is not None and b.get("today") is not None and not notes else None
        db = p["b"]["score"] - p["a"]["score"] if p["a"].get("score") is not None and p["b"].get("score") is not None else None
        for k, d, tie in (("truth", dt, TIE_TRUTH), ("today", dg, TIE_SCORE), ("before", db, TIE_SCORE)):
            if d is None or not v:
                continue
            agree[k][1] += 1
            agree[k][0] += verdict_of(d, tie) == v
            if v != "pari":
                xs[k].append((d, 1 if v == "B" else -1))
        fmt = lambda x: "—" if x is None else f"{x:+.2f}"  # noqa: E731
        num = lambda x: "—" if x is None else f"{x:.2f}"  # noqa: E731
        lines.append(f"| {pid} | {p['deck']} | {p['kind']} | {v} | {num(ta)} | {num(tb)} | {fmt(dt)} | {fmt(dg)} | {fmt(db)} | {'; '.join(notes)} |")
    lines += ["", "| Misura | D'accordo col proprietario (pari compresi) | Stesso verso sulle coppie non pari |", "|---|---|---|"]
    for k, label in (("truth", "Verità del banco"), ("today", "Misura generica di oggi"), ("before", "Punteggio di prima")):
        right, total = agree[k]
        same = sum(1 for d, s in xs[k] if d * s > 0)
        lines.append(f"| {label} | {right} su {total} | {same} su {len(xs[k])} |")
    # How the truth follows the generic measure on these boards too.
    tb = [(boards[i]["today"], t["truth"]) for i, t in truth.items() if i in boards and boards[i].get("today") is not None]
    if len(tb) >= 3:
        lines += ["", f"ρ (misura generica di oggi, verità) sulle {len(tb)} board: {spearman([x for x, _ in tb], [y for _, y in tb]):.2f}"]
    open(os.path.join(out, "owner-table.md"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--own", help="the deck lists the pairs were made with")
    p.add_argument("--meta-decks", help="the meta's lists (objective_bench.py --decks)")
    p.add_argument("--meta", help="meta.json of the objective bench (same opponents and weights)")
    p.add_argument("--hunt", nargs="*", default=[], help="combo_hunt.py --json folders, in order of preference")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--table", help="the owner's verdicts (owner-verdicts.tsv): only the table")
    a = p.parse_args()
    if a.table:
        table(a.out, a.table)
        return
    if not os.path.exists(os.path.join(a.out, "boards.json")):
        prep(a.out, a.own, a.meta_decks, a.meta, a.hunt)
    measure(a.out, a.cli)


if __name__ == "__main__":
    main()
