#!/usr/bin/env python3
"""Download the latest Master Duel Meta top decks and write one .ydk per deck type.

The decklists are downloaded when needed and never committed. Card names are
resolved to passcodes with the card database of the app (data/cards.db, made
by `duelforge-cli update-cards`).

    python3 scripts/mdm_decks.py --out /tmp/mdm --limit 80
"""
import argparse
import json
import os
import re
import sqlite3
import sys
import urllib.request

API = "https://www.masterduelmeta.com/api/v1/top-decks?sort=-created&limit={limit}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", required=True, help="folder for the .ydk files")
    parser.add_argument("--db", default=os.path.join(os.path.dirname(__file__), "..", "data", "cards.db"))
    parser.add_argument("--limit", type=int, default=80, help="most recent top decks to read")
    args = parser.parse_args()

    request = urllib.request.Request(API.format(limit=args.limit), headers={"User-Agent": "DuelForge deck tests"})
    with urllib.request.urlopen(request, timeout=60) as response:
        decks = json.load(response)
    db = sqlite3.connect(args.db)

    def passcode(name: str):
        row = db.execute("select id from cards where lower(name) = lower(?)", (name,)).fetchone()
        return row[0] if row else None

    os.makedirs(args.out, exist_ok=True)
    seen = set()
    # Tournament lists first; the API gives the most recent first. One list per deck type.
    for deck in sorted(decks, key=lambda d: 0 if d.get("tournamentType") else 1):
        kind = (deck.get("deckType") or {}).get("name")
        if not kind or kind in seen:
            continue
        seen.add(kind)
        parts, missing = {"main": [], "extra": []}, []
        for part, cards in parts.items():
            for entry in deck.get(part, []):
                code = passcode(entry["card"]["name"])
                if code is None:
                    missing.append(entry["card"]["name"])
                else:
                    cards.extend([code] * entry["amount"])
        slug = re.sub(r"[^a-z0-9]+", "_", kind.lower()).strip("_")
        with open(os.path.join(args.out, slug + ".ydk"), "w") as f:
            f.write("#main\n" + "\n".join(map(str, parts["main"])) + "\n#extra\n" + "\n".join(map(str, parts["extra"])) + "\n!side\n")
        note = f" (missing: {', '.join(sorted(set(missing)))})" if missing else ""
        print(f"{slug}: main {len(parts['main'])}, extra {len(parts['extra'])}{note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
