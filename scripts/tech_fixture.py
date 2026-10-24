#!/usr/bin/env python3
"""Turn a `duelforge-cli tech-table --json` file into the mock's recorded tech table.

The table takes the shape of the backend's `TechTableView` (what `api.techTable`
returns), and the cards it names get their name and kind from data/cards.db, so
the preview shows them without the texts.

    python3 scripts/tech_fixture.py out/tech/invoked-siracusa-md-s1.json \
        apps/desktop/src/lib/fixtures/tech-table.json
"""
import argparse
import json
import os
import sqlite3


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("table", help="file written by tech-table --json")
    parser.add_argument("out", help="fixture to write")
    parser.add_argument("--db", default=os.path.join(os.path.dirname(__file__), "..", "data", "cards.db"))
    args = parser.parse_args()

    with open(args.table, encoding="utf-8") as f:
        cli = json.load(f)
    rows = [
        {
            "code": r["code"],
            "main": r["main"],
            "side": r["side"],
            "chance": r["chance"],
            "value": r["value"],
            "lost": r["lost"],
            "perDeck": [{"usable": d["usable"], "stopped": d["stopped"], "lost": d["lost"]} for d in r["decks"]],
            "step": r["step"],
            "at": [{"code": a["code"], "lines": a["lines"]} for a in r["at"]],
        }
        for r in cli["rows"]
    ]
    decks = [{"name": d["name"], "weight": d["weight"], "lines": d["lines"], "measured": d["measured"]} for d in cli["decks"]]
    codes = sorted({r["code"] for r in rows} | {a["code"] for r in rows for a in r["at"]})
    db = sqlite3.connect(args.db)
    cards = []
    for code in codes:
        found = db.execute("SELECT name, kind FROM cards WHERE id = ?", (code,)).fetchone()
        if found:
            cards.append({"id": code, "name": found[0], "kind": found[1]})
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump({"table": {"deckId": 0, "decks": decks, "rows": rows}, "cards": cards}, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"{len(rows)} rows, {len(decks)} decks, {len(cards)} of {len(codes)} cards named")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
