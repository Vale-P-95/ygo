#!/usr/bin/env python3
"""Check the probe turn against a deck, level 2 (`probe-line --vs X --moves`:
the opponent plays its best lines again in its turn after yours), on boards
against many decks and with two seeds (docs/plans/turno-di-prova-matchup.md,
L4; docs/plans/collaudo-strumenti.md).

    python3 scripts/combo_hunt.py $S/mdm --hands 2 --top 8 --seed 7 --json $S/hunt
    python3 scripts/moves_report.py $S/moves --decks $S/mdm --hunt $S/hunt --own invoked,branded --seeds 1,7
    python3 scripts/moves_report.py $S/moves --decks $S/mdm --hunt $S/hunt --own invoked,branded --skip-run

Each own deck's board is the first of `<hunt>/<deck>-0.lines.json`. Writes
the logs and <out>/report.md: moves made on an empty board, times, value,
floodgates and "left" per seed, and the moves not made with their reason.
"""
import argparse
import concurrent.futures as cf
import glob
import os
import re
import statistics
import subprocess
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

ITS = re.compile(r"its lines \(([\d.]+) s\): (\d+) from (\d+) hands")
AGAINST = re.compile(r"against its lines \(([\d.]+) s\): (\d+) lines · (\d+)% of their moves made on an empty board · value ([\d.]+) · "
                     r"floodgates stop (\d+)% · (\d+)% of the rest left with all your answers")
MOVE = re.compile(r"^\s+(\d+)\. line (\d+): (.*) · ([a-z][a-z ,]*)$", re.M)


def run(cli, own, deck, vs, seed, out):
    base = os.path.join(out, f"{own}.vs.{os.path.splitext(os.path.basename(vs))[0]}.s{seed}")
    if os.path.exists(base + ".log"):
        return
    started = time.time()
    r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", deck, "--lines", own_lines(own), "--index", "0",
                        "--vs", vs, "--moves", "--seed", str(seed)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    open(base + ".log", "w").write(f"# wall {time.time() - started:.1f} rc {r.returncode}\n" + r.stdout)


def own_lines(own):
    return LINES[own]


LINES = {}


def parse(path):
    text = open(path).read()
    wall, rc = re.match(r"# wall ([\d.]+) rc (-?\d+)", text).groups()
    out = dict(wall=float(wall), rc=int(rc), error=("failed" in text or int(rc) != 0))
    m = ITS.search(text)
    out["its_s"], out["lines_found"], out["hands"] = (float(m[1]), int(m[2]), int(m[3])) if m else (None, None, None)
    m = AGAINST.search(text)
    if m:
        out.update(against_s=float(m[1]), lines=int(m[2]), made_pct=int(m[3]), value=float(m[4]), floodgates=int(m[5]), left=int(m[6]))
    # The moves are listed after the measure line.
    tail = text[m.end():] if m else ""
    out["moves"] = [dict(n=int(x[0]), line=int(x[1]), what=x[2], status=x[3]) for x in MOVE.findall(tail)]
    return out


def report(out, owns, vss, seeds):
    rows, notmade, statuses = [], {}, {}
    data = {}
    for own in owns:
        for vs in vss:
            for s in seeds:
                p = os.path.join(out, f"{own}.vs.{vs}.s{s}.log")
                if os.path.exists(p):
                    data[(own, vs, s)] = parse(p)
    lines = ["# Moves report (level 2)", ""]
    # Totals per seed.
    for s in seeds:
        runs = [d for (o, v, ss), d in data.items() if ss == s]
        moves = [m for d in runs for m in d["moves"]]
        made = sum(m["status"] != "not made even on an empty board" for m in moves)
        against = sorted(d["against_s"] for d in runs if "against_s" in d)
        its = sorted(d["its_s"] for d in runs if d.get("its_s") is not None)
        errors = [k for k, d in data.items() if k[2] == s and d["error"]]
        nomeasure = [k for k, d in data.items() if k[2] == s and "against_s" not in d]
        q = lambda xs, f: xs[min(len(xs) - 1, int(len(xs) * f))] if xs else None  # noqa: E731
        lines += [f"## Seed {s}", "",
                  f"- runs: {len(runs)}, errors: {len(errors)} {errors[:5]}, without a measure: {len(nomeasure)}",
                  f"- moves made on an empty board: {made} of {len(moves)} ({100 * made / max(len(moves), 1):.1f}%)",
                  f"- against its lines (s): median {q(against, .5)}, p90 {q(against, .9)}, max {against[-1] if against else None}",
                  f"- its lines (s): median {q(its, .5)}, p90 {q(its, .9)}, max {its[-1] if its else None}", ""]
        for m in moves:
            statuses[(s, m["status"])] = statuses.get((s, m["status"]), 0) + 1
    lines += ["## Statuses", "", "| Seed | Status | Moves |", "|---|---|---|"]
    lines += [f"| {s} | {st} | {n} |" for (s, st), n in sorted(statuses.items())]
    # Per own board: made share, and stability between the seeds.
    lines += ["", "## Per board", "", "| Own | " + " | ".join(f"made s{s} · value · floodgates · left" for s in seeds) + " | Δvalue | Δfloodgates | Δleft |",
              "|---|" + "---|" * (len(seeds) + 3)]
    for own in owns:
        cells, per = [], {}
        for s in seeds:
            ds = [data[(own, v, s)] for v in vss if (own, v, s) in data and "against_s" in data[(own, v, s)]]
            mv = [m for d in ds for m in d["moves"]]
            made = sum(m["status"] != "not made even on an empty board" for m in mv)
            per[s] = {v: data[(own, v, s)] for v in vss if (own, v, s) in data and "against_s" in data[(own, v, s)]}
            mean = lambda k: statistics.mean(d[k] for d in ds) if ds else float("nan")  # noqa: E731
            cells.append(f"{100 * made / max(len(mv), 1):.0f}% · {mean('value'):.2f} · {mean('floodgates'):.0f}% · {mean('left'):.0f}%")
        deltas = []
        if len(seeds) >= 2:
            a, b = per[seeds[0]], per[seeds[1]]
            common = [v for v in a if v in b]
            for k in ("value", "floodgates", "left"):
                deltas.append(statistics.mean(abs(a[v][k] - b[v][k]) for v in common) if common else float("nan"))
        lines.append(f"| {own} | " + " | ".join(cells) + " | " + " | ".join(f"{d:.2f}" for d in deltas) + " |")
    # Moves not made, by what they are.
    for (own, vs, s), d in data.items():
        for m in d["moves"]:
            if m["status"] == "not made even on an empty board":
                notmade.setdefault(re.sub(r" \(.*\)$", "", m["what"]) + " " + (re.search(r"\((.*)\)$", m["what"]) or [None, ""])[1], []).append(f"{own}/{vs}/s{s}#{m['n']}")
    lines += ["", f"## Not made even on an empty board ({sum(len(v) for v in notmade.values())})", ""]
    for what, where in sorted(notmade.items(), key=lambda x: -len(x[1])):
        lines.append(f"- {what}: {len(where)} ({', '.join(where[:4])})")
    open(os.path.join(out, "report.md"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines[:40]))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--decks", required=True)
    p.add_argument("--hunt", required=True, help="folder of combo_hunt.py --json: <deck>-0.lines.json")
    p.add_argument("--own", required=True)
    p.add_argument("--vs", default="", help="comma-separated decks (all of --decks when empty)")
    p.add_argument("--limit", type=int, default=0, help="at most this many opponent decks, in name order")
    p.add_argument("--seeds", default="1,7")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--skip-run", action="store_true")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    owns = a.own.split(",")
    vss = a.vs.split(",") if a.vs else sorted(os.path.splitext(os.path.basename(x))[0] for x in glob.glob(os.path.join(a.decks, "*.ydk")))
    if a.limit:
        vss = vss[: a.limit]
    seeds = [int(s) for s in a.seeds.split(",")]
    for own in owns:
        LINES[own] = os.path.join(a.hunt, f"{own}-0.lines.json")
    if not a.skip_run:
        jobs = [(own, v, s) for s in seeds for own in owns for v in vss]
        with cf.ThreadPoolExecutor(a.jobs) as ex:
            list(ex.map(lambda j: run(a.cli, j[0], os.path.join(a.decks, j[0] + ".ydk"), os.path.join(a.decks, j[1] + ".ydk"), j[2], a.out), jobs))
    report(a.out, owns, vss, seeds)


if __name__ == "__main__":
    main()
