#!/usr/bin/env python3
"""Pairs of boards for the owner to judge (docs/plans/classificazione-board.md,
D17; corsia 3 of docs/plans/piano-prossimi-passi.md): the pairs where the
ranking is least sure, plus a few sure ones as a check.

Three sources, in this order of priority:
1. the pairs of tests/fixtures/boards/pairs.tsv still to confirm, or with
   "?" or "A≈B", found again with today's code (`board_pairs.py make`);
2. new pairs: the boards of one hand (`combo_hunt.py --json`) with close
   scores and different cards;
3. a few sure pairs as a check, whose verdict the owner should give: from
   pairs.tsv when found again, else a board and the same board with one
   more card on the field (the same hand).

    python3 scripts/board_pairs.py make $S/made --decks $S/mdm
    python3 scripts/combo_hunt.py $S/mdm --lines-rule --top 8 --seed 7 --json $S/hunt
    python3 scripts/owner_pairs.py --made $S/made --hunt $S/hunt --decks $S/mdm

Writes tests/fixtures/boards/owner-pairs.tsv (the conditions, as in
pairs.tsv) and owner-pairs.json (the boards, for the judging page). Only
card names and codes: no card text, no full decklist.
"""
import argparse
import glob
import json
import os
import re
import sqlite3
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import WEIGHTS  # noqa: E402
from board_pairs import read_pairs, unzoned  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUT = os.path.join(ROOT, "tests", "fixtures", "boards")
CDB = os.path.join(ROOT, "data", "cdb", "cards.cdb")
MONSTER = 0x1

# A new pair: scores at most this far apart, and at least this many cards
# (field and hand together) that differ.
CLOSE = 0.15
MIN_DIFF = 2
# Boards worth judging: the better of the two scores at least this much.
MIN_SCORE = 0.5


def composite(m):
    return round(sum(w * m[k] for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience"))), 3)


class Cards:
    """Name -> code and type, from the engine's card database."""

    def __init__(self):
        self.db = sqlite3.connect(CDB)

    def code(self, name):
        row = self.db.execute("select t.id from texts t join datas d on d.id = t.id where t.name = ? and d.alias = 0 order by t.id limit 1",
                              (name,)).fetchone()
        row = row or self.db.execute("select id from texts where name = ? order by id limit 1", (name,)).fetchone()
        return row[0] if row else None

    def name(self, code):
        row = self.db.execute("select name from texts where id = ?", (code,)).fetchone()
        return row[0] if row else str(code)

    def is_monster(self, code):
        row = self.db.execute("select type from datas where id = ?", (code,)).fetchone()
        return bool(row and row[0] & MONSTER)


def field_entry(cards, text):
    """A field entry of combos --json ("Name (set) -> Target [EMZ]") as zone and position."""
    emz = text.endswith(" [EMZ]")
    text = text[:-6] if emz else text
    target = None
    if " -> " in text:
        text, target = text.split(" -> ", 1)
    face_down = text.endswith(" (set)")
    name = text[:-6] if face_down else text
    code = cards.code(name)
    # Monsters are in a Monster Zone unless they are equipped to another
    # (then they sit in the Spell & Trap Zone, as Equip Spells).
    monster = cards.is_monster(code) and target is None
    zone = "extra_monster" if emz else ("monster" if monster else "spell_trap")
    if monster:
        position = "face-down defense" if face_down else "face-up"
    else:
        position = "set" if face_down else "face-up"
    out = {"name": name, "code": code, "zone": zone, "position": position}
    if target:
        out["equipped_to"] = target
    return out


def cards_of(cards, names):
    return [{"name": n, "code": cards.code(n)} for n in names]


def board_of(cards, deck, hand, item):
    m = item.get("measure") or {}
    codes = item.get("hand") or [cards.code(n) for n in item["hand_names"]]
    return {
        "field": [field_entry(cards, f) for f in item["field"]],
        "hand": [{"name": cards.name(c), "code": c} for c in codes],
        # The cards when the CLI writes their names; with an older CLI, how many.
        "grave": cards_of(cards, item["grave_names"]) if "grave_names" in item else item.get("grave"),
        "banished": cards_of(cards, item["banished_names"]) if "banished_names" in item else item.get("banished"),
        "line_length": item.get("choices"),
        # Rebuilt from a signature of pairs.tsv: no line, no measure today.
        "rebuilt": "signature" in item and "measure" not in item,
        "score": composite(m) if m else None,
        "metrics": {k: round(m[k], 3) for k in ("field", "hand", "floodgates", "resilience")} if m else None,
        "signature": unzoned(item["signature"]),
    }


def card_bag(item):
    names = [re.sub(r" \(set\)| -> .*| \[EMZ\]", "", f) for f in item["field"]] + item["hand_names"]
    bag = {}
    for n in names:
        bag[n] = bag.get(n, 0) + 1
    return bag


def differing(a, b, hand=True):
    ba, bb = card_bag(a if hand else {**a, "hand_names": []}), card_bag(b if hand else {**b, "hand_names": []})
    return sum(abs(ba.get(k, 0) - bb.get(k, 0)) for k in set(ba) | set(bb))


def distinct(items):
    seen, out = set(), []
    for it in items:
        key = unzoned(it["signature"])
        if key not in seen and it.get("measure"):
            seen.add(key)
            out.append(it)
    return out


def hand_names(cards, codes):
    return [{"name": cards.name(int(c)), "code": int(c)} for c in codes]


def from_made(cards, made, wanted):
    """The pairs of pairs.tsv found again by board_pairs.py make."""
    saved = json.load(open(os.path.join(made, "pairs.json")))["pairs"]
    out = []
    for p in read_pairs():
        if not wanted(p):
            continue
        s = saved.get(p["id"])
        if not s or not s["a"] or not s["b"]:
            continue
        items = json.load(open(os.path.join(made, s["deck"] + ".json")))["items"]
        a, b = items[s["a"]["index"]], items[s["b"]["index"]]
        if not a.get("measure") or not b.get("measure") or unzoned(a["signature"]) == unzoned(b["signature"]):
            continue
        out.append(dict(source=p["id"], deck=s["deck"], hand=s["hand"], a=a, b=b, cond_a=p["a"], cond_b=p["b"],
                        panel=p["expected"], certainty=p["certainty"], why=p["why"]))
    return out


def superset(a, b):
    """A has every card of B, on the field and in the hand, and more on the field,
    with the same GY and banishment (a card in B's GY can still work there)."""
    zones = lambda x: tuple(sorted(x[k]) if isinstance(x.get(k), list) else x.get(k)  # noqa: E731
                            for k in (("grave_names" if "grave_names" in x else "grave"), ("banished_names" if "banished_names" in x else "banished")))
    if zones(a) != zones(b):
        return False
    fa, fb = card_bag({"field": a["field"], "hand_names": []}), card_bag({"field": b["field"], "hand_names": []})
    ha, hb = card_bag({"field": [], "hand_names": a["hand_names"]}), card_bag({"field": [], "hand_names": b["hand_names"]})
    return all(fa.get(k, 0) >= v for k, v in fb.items()) and ha == hb and sum(fa.values()) > sum(fb.values())


def hunted(hunts):
    for hunt in hunts:
        for path in sorted(glob.glob(os.path.join(hunt, "*.json"))):
            if not path.endswith(".lines.json"):
                yield path


def controls_from_hunt(cards, hunts):
    """Sure pairs: a board, and the same board with one more card on the field."""
    out = []
    for path in hunted(hunts):
        doc = json.load(open(path))
        boards = distinct(doc["items"])
        deck = re.sub(r"-\d+$", "", os.path.basename(path)[:-5])
        for a in (x for x in boards if composite(x["measure"]) >= MIN_SCORE):
            b = next((b for b in boards if b is not a and superset(a, b)), None)
            if b:
                out.append(dict(source="controllo", deck=deck, hand=[str(c) for c in doc["hand"]], a=a, b=b,
                                cond_a="sig=" + unzoned(a["signature"]), cond_b="sig=" + unzoned(b["signature"]), panel="A>B",
                                certainty="", why="A è B con una carta in più sul campo, la stessa mano"))
                break
    return out


def split_cards(cards, text):
    """A ", "-separated list of entries whose card names may hold ", " too."""
    parts, out, i = [p for p in text.split(", ") if p], [], 0
    base = lambda e: re.sub(r" \(set\)| -> .*| \[EMZ\]", "", e)  # noqa: E731
    while i < len(parts):
        # The longest run of parts that makes a card name.
        for j in range(len(parts), i, -1):
            entry = ", ".join(parts[i:j])
            if cards.code(base(entry)) is not None:
                out.append(entry)
                i = j
                break
        else:
            out.append(parts[i])
            i += 1
    return out


def from_signature(cards, made, found_ids):
    """Unsure pairs of pairs.tsv not found again (today's decklists give
    another hand): the boards rebuilt from their signatures, without today's
    scores. Only those whose two sides are exact signatures."""
    out = []
    for p in read_pairs():
        if p["id"] in found_ids or not (p["certainty"] == "da confermare" or p["expected"] in ("?", "A≈B")):
            continue
        if not (p["a"].startswith("sig=") and p["b"].startswith("sig=")):
            continue
        sides = []
        for spec in (p["a"], p["b"]):
            field, hand = spec[4:].split(" | hand: ")
            sides.append({"field": split_cards(cards, field), "hand_names": split_cards(cards, hand), "signature": spec[4:]})
        out.append(dict(source=p["id"], deck=p["deck"], hand=None, a=sides[0], b=sides[1], cond_a=p["a"], cond_b=p["b"],
                        panel=p["expected"], certainty=p["certainty"], why=p["why"], rebuilt=True))
    return out


def from_hunt(cards, hunts, per_deck):
    """New pairs: close scores, different cards, among the boards of one hand."""
    out = []
    for path in hunted(hunts):
        if path.endswith(".lines.json"):
            continue
        doc = json.load(open(path))
        boards = distinct(doc["items"])[:8]
        deck = re.sub(r"-\d+$", "", os.path.basename(path)[:-5])
        if not boards or composite(boards[0]["measure"]) <= 0:
            continue
        cands = []
        for i, a in enumerate(boards):
            for b in boards[i + 1:]:
                sa, sb = composite(a["measure"]), composite(b["measure"])
                diff = differing(a, b)
                # The field must differ too: a card more in hand alone says little.
                if abs(sa - sb) <= CLOSE and diff >= MIN_DIFF and differing(a, b, hand=False) >= 1 and max(sa, sb) >= MIN_SCORE:
                    # The most different boards first, then the closest scores.
                    cands.append((-diff, abs(sa - sb), a, b))
        used = set()
        for _, _, a, b in sorted(cands, key=lambda c: (c[0], c[1])):
            if len([x for x in out if x["deck"] == deck]) >= per_deck:
                break
            # Each board in one pair at most: more boards to look at.
            if id(a) in used or id(b) in used:
                continue
            used |= {id(a), id(b)}
            out.append(dict(source="nuova", deck=deck, hand=[str(c) for c in doc["hand"]], a=a, b=b,
                            cond_a="sig=" + unzoned(a["signature"]), cond_b="sig=" + unzoned(b["signature"]),
                            panel="", certainty="", why=f"punteggi vicini ({composite(a['measure']):.2f} e {composite(b['measure']):.2f}), "
                                                      f"{differing(a, b)} carte diverse"))
    return out


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--made", required=True, help="folder written by board_pairs.py make")
    p.add_argument("--hunt", required=True, nargs="+", help="folders written by combo_hunt.py --json")
    p.add_argument("--per-deck", type=int, default=4, help="pairs per deck at most")
    p.add_argument("--new", type=int, default=3, help="new pairs per deck at most")
    p.add_argument("--controls", type=int, default=6, help="sure pairs of pairs.tsv, as a check")
    p.add_argument("--only", default="", help="comma-separated decks to keep (all when empty)")
    p.add_argument("--out", default=OUT)
    a = p.parse_args()
    cards = Cards()

    unsure = from_made(cards, a.made, lambda q: q["certainty"] == "da confermare" or q["expected"] in ("?", "A≈B"))
    # Those not found again, from their signatures: one per deck.
    rebuilt, seen_deck = [], set()
    for x in from_signature(cards, a.made, {x["source"] for x in unsure}):
        if x["deck"] not in seen_deck:
            rebuilt.append(x)
            seen_deck.add(x["deck"])
    unsure += rebuilt
    new = from_hunt(cards, a.hunt, a.new)
    sure = from_made(cards, a.made, lambda q: q["certainty"] == "sicura" and q["expected"] in ("A>B", "B>A"))
    # The sure pairs from decks the others do not cover first, one per deck.
    sure.sort(key=lambda x: (x["deck"] in {y["deck"] for y in unsure + new}, x["source"]))
    keep = set(filter(None, a.only.split(",")))
    controls, seen = [], set()
    for x in sure + controls_from_hunt(cards, a.hunt):
        if (not keep or x["deck"] in keep) and x["deck"] not in seen and len(controls) < a.controls:
            controls.append(x)
            seen.add(x["deck"])
    unsure, new = [x for x in unsure if not keep or x["deck"] in keep], [x for x in new if not keep or x["deck"] in keep]
    chosen = []
    # The controls before the new pairs: each deck keeps room for its control.
    for kind, group in (("da confermare", unsure), ("controllo", controls), ("nuova", new)):
        for x in group:
            if sum(y["deck"] == x["deck"] for y in chosen) < a.per_deck:
                chosen.append({**x, "kind": kind})

    rows = ["# Coppie di board da far giudicare al proprietario (docs/plans/collaudo-strumenti.md, corsia 3), scritte da scripts/owner_pairs.py.",
            "# Colonne (separate da tab): id, deck, condizioni della board A, condizioni della board B, tipo (da confermare, nuova, controllo),",
            "# origine (id in pairs.tsv o \"nuova\"), punteggio A, punteggio B (di oggi), verdetto del pannello (se c'è), perché è stata scelta.",
            "# Le condizioni sono quelle di board_pairs.py: le coppie restano valide quando la ricerca cambia."]
    pages = []
    for i, x in enumerate(chosen, 1):
        pid = f"O{i}"
        ba, bb = board_of(cards, x["deck"], x["hand"], x["a"]), board_of(cards, x["deck"], x["hand"], x["b"])
        panel = x["panel"] + (f" ({x['certainty']})" if x["certainty"] else "")
        score = lambda b: "—" if b["score"] is None else f"{b['score']:.2f}"  # noqa: E731
        rows.append("\t".join([pid, x["deck"], x["cond_a"], x["cond_b"], x["kind"], x["source"], score(ba), score(bb),
                               panel or "—", x["why"]]))
        pages.append({"id": pid, "deck": x["deck"], "kind": x["kind"], "source": x["source"], "panel": x["panel"] or None,
                      "why": x["why"], "start_hand": hand_names(cards, x["hand"]) if x["hand"] else None, "a": ba, "b": bb})
    os.makedirs(a.out, exist_ok=True)
    open(os.path.join(a.out, "owner-pairs.tsv"), "w", encoding="utf-8").write("\n".join(rows) + "\n")
    json.dump({"pairs": pages}, open(os.path.join(a.out, "owner-pairs.json"), "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    kinds = {k: sum(x["kind"] == k for x in chosen) for k in ("da confermare", "nuova", "controllo")}
    print(f"{len(chosen)} coppie su {len({x['deck'] for x in chosen})} deck: {kinds}")


if __name__ == "__main__":
    main()
