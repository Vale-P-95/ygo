#!/usr/bin/env python3
"""How often the opponent holds each kind of handtrap going second, from a
folder of deck lists (the Master Duel Meta lists of scripts/mdm_decks.py):
the mean copies per Main Deck of each kind, and the chance of at least one
in 6 cards of a 40-card deck. The exposure of a line (docs/plans/esposizione.md)
weighs what each kind takes away by that chance; the table in
crates/ygo-solver/src/probe.rs (`HandTrapRates::default`) comes from here.

    python3 scripts/mdm_decks.py --out /tmp/mdm
    python3 scripts/handtrap_rates.py /tmp/mdm --data data
"""
import argparse
import glob
import json
import os
import sqlite3

# The handtraps of each kind of choke point, by name (alternate artworks
# are found by name too).
KINDS = {
    "deck_touch": ["Ash Blossom & Joyous Spring"],
    "field_effect": ["Effect Veiler", "Infinite Impermanence", "Ghost Ogre & Snow Rabbit"],
    "fifth_summon": ["Nibiru, the Primal Being"],
    "second_search": ["Droll & Lock Bird"],
    "special_summons": ['Maxx "C"', "Mulcharmy Fuwalos", "Mulcharmy Purulia", "Mulcharmy Nyalus"],
}


def chance(copies, deck=40, cards=6):
    """At least one of `copies` cards in `cards` drawn from `deck` (copies may be a mean)."""
    none = 1.0
    for j in range(cards):
        none *= max(0.0, (deck - copies - j) / (deck - j))
    return 1.0 - none


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("decks", help="folder of .ydk files")
    p.add_argument("--data", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data"))
    args = p.parse_args()
    db = sqlite3.connect(os.path.join(args.data, "cards.db"))
    codes = {}
    for kind, cards in KINDS.items():
        for name in cards:
            for code, images in db.execute("SELECT id, image_ids FROM cards WHERE name = ?", (name,)):
                for c in [code] + json.loads(images):
                    codes[int(c)] = kind
    decks = sorted(glob.glob(os.path.join(args.decks, "*.ydk")))
    totals = {k: 0 for k in KINDS}
    for path in decks:
        main = open(path, encoding="utf-8").read().split("#extra")[0].split()
        for w in main:
            if w.isdigit() and int(w) in codes:
                totals[codes[int(w)]] += 1
    print(f"{len(decks)} decks")
    print("kind\tmean copies\tchance in 6 of 40")
    for kind in KINDS:
        mean = totals[kind] / max(1, len(decks))
        print(f"{kind}\t{mean:.2f}\t{chance(mean):.3f}")


if __name__ == "__main__":
    main()
