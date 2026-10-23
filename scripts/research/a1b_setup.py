#!/usr/bin/env python3
"""Boards for A1b step 1 (docs/plans/ricerca-piano-critico.md, A1b; results in
docs/plans/ricerca-esperimenti.md, section 6): writes <out>/boards.json and
<out>/lines/ in the layout of scripts/objective_bench.py, which then plays
each board against the meta at level 2.

    python3 scripts/research/a1b_setup.py <out> --hunt <combo_hunt JSON> --decks <mdm> [--per-hand 5] [--oracle tests/fixtures/oracle]
    (the oracle's hands searched normally: combos --seed <its seed> --nodes 4000 --top 8 --json <hunt>/<deck>-<fixture id>.json --save-lines ...)
    python3 scripts/objective_bench.py <out> --decks <mdm> --hunt - --skip lunalight --jobs 4

Per hand of the hunt: the first --per-hand boards by the full measure, with
distinct fields (zones aside), so the pairs compared are boards the app
would put at the top. From the oracle fixtures: the best board of each one
whose deck list is unchanged (deck_hash), with its line.
"""
import argparse
import glob
import hashlib
import json
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from rank_report import composite  # noqa: E402


def field_key(signature):
    return re.sub(r" \[EMZ\]", "", signature.split(" | hand:")[0])


def deck_hash(path):
    # As scripts/oracle.py.
    cards = [w for w in open(path).read().split() if w.isdigit() or w.startswith(("#", "!"))]
    return hashlib.sha256(" ".join(cards).encode()).hexdigest()[:16]


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--hunt", required=True)
    p.add_argument("--decks", required=True)
    p.add_argument("--per-hand", type=int, default=5)
    p.add_argument("--oracle", default=None)
    a = p.parse_args()
    os.makedirs(os.path.join(a.out, "lines"), exist_ok=True)
    boards = []
    for path in sorted(glob.glob(os.path.join(a.hunt, "*.json"))):
        if path.endswith(".lines.json"):
            continue
        hand_file = os.path.basename(path)[:-5]
        deck = hand_file.split("-")[0]  # the deck slugs have no hyphen
        items = json.load(open(path))["items"]
        lines = json.load(open(path[:-5] + ".lines.json"))
        ranked = sorted((k for k, it in enumerate(items) if it.get("measure") and it["field"]),
                        key=lambda k: (-composite(items[k]), k))
        seen, keep = set(), []
        for k in ranked:
            key = field_key(items[k]["signature"])
            if key not in seen and len(keep) < a.per_hand:
                seen.add(key)
                keep.append(k)
        if len(keep) < 2:
            continue
        # The lines keep the search's order: item k of the JSON is line k.
        json.dump(dict(lines, items=[lines["items"][items[k]["index"]] for k in keep]),
                  open(os.path.join(a.out, "lines", hand_file + ".lines.json"), "w"))
        for i, k in enumerate(keep):
            boards.append(dict(id=f"{hand_file}#{i}", deck=deck, hand_file=hand_file, index=i, rank=i,
                               signature=items[k]["signature"], source="hunt"))
    skipped = []
    if a.oracle:
        for path in sorted(glob.glob(os.path.join(a.oracle, "*.json"))):
            fx = json.load(open(path))
            deck = fx["deck"][:-4]
            ydk = os.path.join(a.decks, fx["deck"])
            if "line" not in fx or not os.path.exists(ydk) or deck_hash(ydk) != fx["deck_hash"]:
                skipped.append(fx["id"])
                continue
            line = dict(answers=fx["line"]["answers"], choices=[], choice_at=[])
            # The same hand searched with the normal budget (the hunt file
            # <deck>-<fixture id>): the oracle's board joins its boards, so
            # they are compared with it.
            hand_file = f"{deck}-{fx['id']}"
            path = os.path.join(a.out, "lines", hand_file + ".lines.json")
            if os.path.exists(path):
                doc = json.load(open(path))
                doc["items"].append(line)
            else:
                hand_file = "oracle-" + fx["id"]
                path = os.path.join(a.out, "lines", hand_file + ".lines.json")
                doc = dict(kind="lines", deck=ydk, hand=fx["hand"], seed=fx["seed"], items=[line])
            json.dump(doc, open(path, "w"))
            boards.append(dict(id=f"{hand_file}#{len(doc['items']) - 1}", deck=deck, hand_file=hand_file, index=len(doc["items"]) - 1,
                               rank=len(doc["items"]) - 1, signature="oracle " + fx["id"], source="oracle"))
    json.dump(boards, open(os.path.join(a.out, "boards.json"), "w"), indent=1)
    print(f"{len(boards)} board in {len({b['hand_file'] for b in boards})} file di linee; oracolo saltato: {', '.join(skipped) or '-'}")


if __name__ == "__main__":
    main()
