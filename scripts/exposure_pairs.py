#!/usr/bin/env python3
"""The board pairs (tests/fixtures/boards/pairs.tsv) judged with the
exposure as expected loss to handtraps (docs/plans/esposizione.md): reads
the measures that `scripts/board_pairs.py eval` saved (probe-line JSON,
with `expected_loss`) and compares the outcomes of the composite without
the exposure and with it, at the weight of the Bilanciato profile.

    python3 scripts/board_pairs.py make $S/pairs --decks $S/mdm --cli $CLI
    python3 scripts/board_pairs.py eval $S/pairs --cli $CLI
    python3 scripts/exposure_pairs.py $S/pairs
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from board_pairs import composite, read_pairs  # noqa: E402

# The exposure's weight in the Bilanciato profile (B6).
EXPOSURE_WEIGHT = -0.5


def outcome(d, expected):
    got = "A≈B" if abs(d) < 0.05 else ("A>B" if d > 0 else "B>A")
    if expected not in ("A>B", "B>A", "A≈B"):
        return got, None
    return got, got == expected


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="the folder of board_pairs.py make and eval")
    p.add_argument("--weight", type=float, default=EXPOSURE_WEIGHT)
    a = p.parse_args()
    saved = json.load(open(os.path.join(a.out, "pairs.json")))["pairs"]

    def board(s, side):
        path = os.path.join(a.out, "measures", f"{s['deck']}-{s[side]['index']}.json")
        if not os.path.exists(path):
            return None
        items = json.load(open(path))["items"]
        if not items or "measure" not in items[0]:
            return None
        loss = (items[0].get("expected_loss") or {}).get("expected")
        return composite(items[0]["measure"]), loss

    totals = {c: {"without": [0, 0, 0], "with": [0, 0, 0]} for c in ("sicura", "da confermare")}
    print("| id | deck | A | B | E(A) | E(B) | atteso | senza E | con E |")
    print("|---|---|---:|---:|---:|---:|---|---|---|")
    for pair in read_pairs():
        s = saved.get(pair["id"])
        if not s or not s["a"] or not s["b"]:
            continue
        a_, b_ = board(s, "a"), board(s, "b")
        if not a_ or not b_ or a_[1] is None or b_[1] is None:
            continue
        (ca, ea), (cb, eb) = a_, b_
        g0, ok0 = outcome(ca - cb, pair["expected"])
        g1, ok1 = outcome(ca + a.weight * ea - (cb + a.weight * eb), pair["expected"])
        for key, got, ok in (("without", g0, ok0), ("with", g1, ok1)):
            if ok is None or pair["certainty"] not in totals:
                continue
            t = totals[pair["certainty"]][key]
            t[0 if ok else (1 if got == "A≈B" else 2)] += 1
        mark = lambda got, ok: got if ok is None else f"{got} {'giusta' if ok else 'sbagliata'}"  # noqa: E731
        print(f"| {pair['id']} | {pair['deck']} | {ca:.2f} | {cb:.2f} | {ea:.2f} | {eb:.2f} | {pair['expected']} "
              f"| {mark(g0, ok0)} | {mark(g1, ok1)} |")
    for certainty, t in totals.items():
        for key in ("without", "with"):
            r, e, w = t[key]
            print(f"\n{certainty}, {'senza' if key == 'without' else 'con'} E: {r} giuste, {e} pari, {w} al contrario", end="")
    print()


if __name__ == "__main__":
    main()
