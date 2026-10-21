#!/usr/bin/env python3
"""Check the map of the links between the cards (`duelforge-cli links`) on
many decks: time, cards, links, and the links the card texts suggest are
missing or wrong. The text is only a hint for a check by hand: the map is
read from the engine on purpose.

    python3 scripts/mdm_decks.py --out /tmp/mdm
    python3 scripts/links_report.py /tmp/links --decks /tmp/mdm
    python3 scripts/links_report.py /tmp/links --decks /tmp/mdm --skip-run   # only the tables again

Writes <out>/<deck>.json and .log for each deck, and <out>/report.md.
"""
import argparse
import glob
import json
import os
import re
import sqlite3
import subprocess
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
CDB = os.path.join(ROOT, "data", "cdb", "cards.cdb")

# What a sentence of the text says the card does, and the link kind and
# zone the map should have for it.
PATTERNS = [
    ("search", "deck", re.compile(r"add [^.]*?from your deck(?: or gy)? to your hand")),
    ("summon", "deck", re.compile(r"special summon [^.]*?from your (?:hand or )?deck")),
    ("summon", "extra", re.compile(r"(?:special|fusion|synchro|xyz|link|ritual) summon [^.]*?from your extra deck")),
    ("toGrave", "deck", re.compile(r"send [^.]*?from your deck to the gy")),
    ("recover", "grave", re.compile(r"add [^.]*?from your (?:deck or )?gy to your hand")),
    # Not a kind of the map: Set from the Deck (seen as missing on purpose).
    ("set", "deck", re.compile(r"set [^.]*?from your deck")),
]

# Words the text of the cause must hold for a link of each kind.
NEEDS = {
    "search": ("hand",),
    "recover": ("hand", "deck"),
    "summon": ("summon",),
    "toGrave": ("gy", "graveyard"),
    "material": (),
}


def texts(codes):
    db = sqlite3.connect(CDB)
    out = {}
    for c in codes:
        row = db.execute("select name, desc from texts where id = ?", (c,)).fetchone()
        if row:
            out[c] = (row[0], row[1])
    return out


def run(cli, deck, out, nodes, hands, threads):
    name = os.path.splitext(os.path.basename(deck))[0]
    path = os.path.join(out, name + ".json")
    started = time.time()
    r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "links", "--deck", deck, "--nodes", str(nodes), "--hands", str(hands),
                        "--threads", str(threads), "--json", path], capture_output=True, text=True)
    open(os.path.join(out, name + ".log"), "w").write(r.stdout + "\n--- stderr ---\n" + r.stderr[-3000:])
    json.dump({"wall": round(time.time() - started, 1), "rc": r.returncode}, open(os.path.join(out, name + ".time.json"), "w"))


def quoted_in(sentence):
    return [q for q in re.findall(r'"([^"]+)"', sentence)]


def expected(code, text, cards):
    """The links the text of `code` suggests: (kind, zone, sentence, targets in the deck)."""
    out = []
    low = text.lower()
    for kind, zone, pat in PATTERNS:
        for m in pat.finditer(low):
            sentence = m.group(0)
            names = quoted_in(sentence)
            if names:
                targets = [c for c in cards if c["code"] != code and any(n in c["name"].lower() for n in names)]
                # A "Name" quoted only names that card or archetype; nothing in the deck: no link to expect.
                if not targets:
                    continue
            else:
                targets = None
            out.append((kind, zone, sentence, targets))
    return out


def analyse(out_dir, deck_names):
    rows, missing, odd = [], [], []
    for name in deck_names:
        path = os.path.join(out_dir, name + ".json")
        if not os.path.exists(path):
            rows.append((name, None))
            continue
        doc = json.load(open(path))
        timing = json.load(open(os.path.join(out_dir, name + ".time.json")))
        cards = doc["cards"]
        by_code = {c["code"]: c for c in cards}
        txt = texts(by_code)
        links = doc["links"]
        used = [l for l in links if l["used"] > 0]
        rows.append((name, dict(wall=timing["wall"], cards=len(cards), links=len(links), used=len(used), hands=doc["hands"], lines=doc["lines"],
                                isolated=sum(1 for c in cards if c["reach"] == 0 and not any(l["to"] == c["code"] for l in links)),
                                kinds={k: sum(1 for l in links if l["kind"] == k) for k in ("search", "summon", "toGrave", "recover", "material")})))
        # Missing: a sentence of the text and no link of that kind (and zone) from the card.
        for c in cards:
            if c["code"] not in txt:
                continue
            for kind, zone, sentence, targets in expected(c["code"], txt[c["code"]][1], cards):
                have = [l for l in links if l["from"] == c["code"] and l["kind"] == kind and l["zone"] == zone]
                if kind == "set" or not have:
                    tn = ", ".join(sorted({t["name"] for t in targets})) if targets else "(generic)"
                    missing.append((name, c["name"], kind, zone, sentence[:110], tn))
        # Odd: a link whose cause's text has none of the words its kind needs,
        # or a zone the text never names.
        for l in links:
            if l["from"] not in txt:
                continue
            low = txt[l["from"]][1].lower()
            reasons = []
            if NEEDS[l["kind"]] and not any(w in low for w in NEEDS[l["kind"]]):
                reasons.append("text has no " + "/".join(NEEDS[l["kind"]]))
            if l["kind"] in ("search", "toGrave") and l["zone"] == "deck" and "deck" not in low:
                reasons.append("text has no Deck")
            if l["kind"] == "summon" and l["zone"] == "deck" and "deck" not in low:
                reasons.append("summons from the Deck, text has no Deck")
            if l["kind"] == "summon" and l["zone"] == "extra" and not re.search(r"extra deck|fusion summon|synchro summon|xyz summon|link summon", low):
                reasons.append("summons from the Extra Deck, text names no such summon")
            if reasons:
                odd.append((name, by_code[l["from"]]["name"], l["kind"], l["zone"], by_code[l["to"]]["name"], round(l["used"], 3), "; ".join(reasons)))
    return rows, missing, odd


def write(out_dir, rows, missing, odd):
    lines = ["# Links report", "", "| Deck | Time (s) | Cards | Links | Used | Search | Summon | ToGrave | Recover | Material | Isolated | Hands | Lines |",
             "|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for name, r in rows:
        if r is None:
            lines.append(f"| {name} | failed | | | | | | | | | | | |")
            continue
        k = r["kinds"]
        lines.append(f"| {name} | {r['wall']} | {r['cards']} | {r['links']} | {r['used']} | {k['search']} | {k['summon']} | {k['toGrave']} | "
                     f"{k['recover']} | {k['material']} | {r['isolated']} | {r['hands']} | {r['lines']} |")
    lines += ["", f"## Missing by text ({len(missing)})", "", "| Deck | Card | Kind | Zone | Text | Targets in the deck |", "|---|---|---|---|---|---|"]
    lines += [f"| {' | '.join(str(x).replace('|', '/') for x in m)} |" for m in missing]
    lines += ["", f"## Odd links ({len(odd)})", "", "| Deck | From | Kind | Zone | To | Used | Why |", "|---|---|---|---|---|---|---|"]
    lines += [f"| {' | '.join(str(x).replace('|', '/') for x in o)} |" for o in odd]
    open(os.path.join(out_dir, "report.md"), "w").write("\n".join(lines) + "\n")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="folder for the maps, logs and report.md")
    p.add_argument("--decks", required=True, help="folder with .ydk files")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--nodes", type=int, default=1000)
    p.add_argument("--hands", type=int, default=2)
    p.add_argument("--threads", type=int, default=4)
    p.add_argument("--only", default="", help="only the decks whose name contains this")
    p.add_argument("--skip-run", action="store_true", help="only read the maps already written")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    decks = sorted(d for d in glob.glob(os.path.join(a.decks, "*.ydk")) if a.only in os.path.basename(d))
    names = [os.path.splitext(os.path.basename(d))[0] for d in decks]
    if not a.skip_run:
        for d in decks:
            run(a.cli, d, a.out, a.nodes, a.hands, a.threads)
            print(os.path.basename(d), "done", flush=True)
    rows, missing, odd = analyse(a.out, names)
    write(a.out, rows, missing, odd)
    print(f"{len(rows)} decks, {len(missing)} missing by text, {len(odd)} odd links -> {os.path.join(a.out, 'report.md')}")


if __name__ == "__main__":
    main()
