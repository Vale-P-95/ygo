#!/usr/bin/env python3
"""Where the work of the search goes (plan A2 of
docs/plans/ricerca-piano-critico.md), from the `search-stats` lines that
`DUELFORGE_SEARCH_STATS=1` prints on stderr, one file per search
(`*.err`, `*.log`). With `DUELFORGE_DEBUG_PHASES=1` the phases after the
search are added up too.

    python3 scripts/search_stats.py <folder> [<folder> ...]

Prints, over all the searches: the share of nodes and time of each way a
dive ends, of the dives by the best board they read, the replays, the
answers that change nothing shown, and where the right answer was in the
order tried along the best lines.
"""
import collections
import glob
import json
import os
import re
import sys

STATS = re.compile(r"^search-stats (\{.*\})$", re.M)
PHASE = re.compile(r"phase (\w+) done at ([\d.]+) s")


def main():
    runs = []
    for folder in sys.argv[1:]:
        for path in sorted(glob.glob(os.path.join(folder, "*.err")) + glob.glob(os.path.join(folder, "*.log"))):
            text = open(path, errors="replace").read()
            found = STATS.findall(text)
            if found:
                phases = dict((m[0], float(m[1])) for m in PHASE.findall(text))
                runs.append((os.path.basename(path), json.loads(found[-1]), phases))
    if not runs:
        sys.exit("no search-stats lines")

    ends = collections.defaultdict(lambda: collections.Counter())
    boards = collections.defaultdict(lambda: collections.Counter())
    unchanged = collections.Counter()
    ranks = collections.Counter()
    elapsed = learning = 0.0
    phases = collections.Counter()
    for _, s, ph in runs:
        elapsed += s["elapsed_s"]
        learning += s["learning_s"]
        for e, d in s["ends"].items():
            ends[e].update(d)
        for b, d in s["boards"].items():
            boards[b].update(d)
        unchanged.update(s["no_change"])
        ranks.update("auto" if r is None else min(r, 10) for r in s["best_line_ranks"])
        if "search" in ph:
            last = max(ph.values())
            phases["search"] += ph["search"]
            phases["after the search"] += last - ph["search"]

    nodes = sum(d["nodes"] for d in ends.values())
    time = sum(d["time_s"] for d in ends.values())
    replay = sum(d["replay_s"] for d in ends.values())
    print(f"{len(runs)} searches, {nodes} nodes, {elapsed:.1f} s of search: {time:.1f} s in dives "
          f"({replay:.1f} s replaying lines from the start, {100 * replay / time:.0f}%), {learning:.1f} s of learning rounds")
    if phases:
        print(f"phases: search {phases['search']:.1f} s, after the search {phases['after the search']:.1f} s")

    def table(title, groups):
        print(f"\n{title}")
        print(f"{'':16} {'dives':>8} {'nodes':>8} {'% nodes':>8} {'time s':>8} {'% time':>7} {'replay s':>9}")
        for k, d in sorted(groups.items(), key=lambda kv: -kv[1]["time_s"]):
            print(f"{k:16} {d['dives']:>8} {d['nodes']:>8} {100 * d['nodes'] / max(nodes, 1):>7.1f}% "
                  f"{d['time_s']:>8.1f} {100 * d['time_s'] / max(time, 1e-9):>6.1f}% {d['replay_s']:>9.1f}")

    table("How the dives end", ends)
    table("Best board read by the dive (weak: under 90% of the search's best)", boards)
    print(f"\nAnswers after which nothing shown changed ({sum(unchanged.values())} nodes, "
          f"{100 * sum(unchanged.values()) / max(nodes, 1):.1f}% of the nodes):")
    print("  " + ", ".join(f"{k} {v}" for k, v in unchanged.most_common()))
    total = sum(ranks.values())
    print(f"\nWhere the answer of the best line was in the order tried ({total} answers):")
    print("  " + ", ".join(f"{'10+' if k == 10 else k}: {100 * v / total:.1f}%" for k, v in sorted(ranks.items(), key=lambda kv: (isinstance(kv[0], str), kv[0]))))


if __name__ == "__main__":
    main()
