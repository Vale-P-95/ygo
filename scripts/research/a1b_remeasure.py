#!/usr/bin/env python3
"""The disagreements of A1b step 1 (docs/plans/ricerca-esperimenti.md,
section 6) with the measure of another build: the boards of the fixture
lines files of scripts/research/a1b_report.py --fixture are measured again,
and each pair is checked with the truth saved in the file. Level 2 plays the
opponent's real lines, not the kit, so a change to the probe turn's kit
moves the measure and leaves the truth.

    python3 scripts/research/a1b_remeasure.py tests/fixtures/research/a1b --decks /tmp/mdm --cli target/release/duelforge-cli --out /tmp/a1b-new

--decks must hold the Master Duel Meta lists the lines were found with (the
"deck" of each file): with another list the lines no longer play. The
measures are written to --out (those already there are kept), and the
pairs that still disagree are counted by their main cause. The composite is
today's (scripts/rank_report.py): the scores saved in the files may have
been computed with another one, and the gap to them says how far.
"""
import argparse
import concurrent.futures as cf
import json
import os
import subprocess
import sys
from collections import Counter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from rank_report import composite  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")


def measure(cli, decks, fixtures, out, name, saved, index):
    """The composite of board `index` of a fixture, measured by `cli`."""
    path = os.path.join(out, f"{name}.{index}.json")
    if not os.path.exists(path):
        hand = ";".join(str(c) for c in saved["hand"])
        subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", os.path.join(decks, saved["deck"]), "--hand", hand,
                        "--seed", str(saved["seed"]), "--lines", os.path.join(fixtures, name + ".lines.json"), "--index", str(index), "--json", path],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        return composite(json.load(open(path))["items"][0])
    except (OSError, ValueError, KeyError, IndexError):
        return None


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("fixtures")
    p.add_argument("--decks", required=True)
    p.add_argument("--cli", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--jobs", type=int, default=4)
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    files = {f[: -len(".lines.json")]: json.load(open(os.path.join(a.fixtures, f))) for f in sorted(os.listdir(a.fixtures)) if f.endswith(".lines.json")}
    jobs = [(name, saved, b["index"]) for name, saved in files.items() for b in saved["a1b"]["boards"]]
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        scores = dict(zip([(n, i) for n, _, i in jobs], ex.map(lambda j: measure(a.cli, a.decks, a.fixtures, a.out, *j), jobs)))

    failed = [k for k, s in scores.items() if s is None]
    gaps = sorted(abs(scores[(n, b["index"])] - b["score"]) for n, saved in files.items() for b in saved["a1b"]["boards"]
                  if scores[(n, b["index"])] is not None)
    pairs, still = Counter(), Counter()
    for name, saved in files.items():
        for pair in saved["a1b"]["pairs"]:
            cause = pair["causes"][0]
            worse, better = scores[(name, pair["worse"])], scores[(name, pair["better"])]
            pairs[cause] += 1
            still[cause] += worse is not None and better is not None and worse > better
    print(f"{len(scores)} boards measured, {len(failed)} failed{': ' + ', '.join(f'{n}#{i}' for n, i in failed) if failed else ''}")
    if gaps:
        print(f"measure against the fixture's score: median gap {gaps[len(gaps) // 2]:.3f}, largest {gaps[-1]:.3f}")
    print(f"pairs still in disagreement: {sum(still.values())} of {sum(pairs.values())}")
    for cause, n in pairs.most_common():
        print(f"  {cause}: {still[cause]} of {n}")


if __name__ == "__main__":
    main()
