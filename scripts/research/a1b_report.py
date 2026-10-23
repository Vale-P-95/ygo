#!/usr/bin/env python3
"""A1b step 1 (docs/plans/ricerca-piano-critico.md, A1b; results in
docs/plans/ricerca-esperimenti.md, section 6): where the probe turn's measure
and level 2 against the meta disagree, and why.

    python3 scripts/research/a1b_report.py <out of objective_bench.py> [--margin 0.05] [--json <file>] [--fixture <folder>]

With --fixture, each hand with a disagreement gets a lines file of its boards
in the pairs, to play again (the deck is the Master Duel Meta list named in
"deck", not committed):

    duelforge-cli probe-line --deck <mdm>/<deck> --lines <folder>/<hand>.lines.json --index <i> --vs <mdm>/<opponent>.ydk --moves

For each board: the truth of the objective bench (1 - `left`, weighted by the
meta, pairs with less than 80% of the opponent's moves made on an empty board
or with no line left out) and the composite of the measure, from the same
runs. A disagreement is a pair of boards of the same hand whose truths
differ by at least --margin while the measure puts the other one first.
Each is classified, by what tells the two boards apart, as:
- "turno dopo": without the next turn (follow_up) the measure would order
  them right;
- "trappole in mano": the measure counts a Trap in the worse board's hand as
  an interruption, which cannot be activated from there;
- "Extra Deck": the better board stops a larger share of the opponents'
  summons from the Extra Deck than of their other moves, by at least 0.1 more
  than the worse board (the kit has one Extra Deck monster);
- "risposte insieme": the worse board's answers are worth more one by one at
  level 2, but stop less together;
- a place ("Cimitero", "mano", "mostri", "Magie e Trappole"): what the
  answers from there are worth at level 2 (B minus A, meta-weighted)
  differs from what the measure gives them by at least 0.1, in favour of the
  better board;
- "altro".
Several causes can hold; the first in this order is the main one.
"""
import argparse
import glob
import json
import os
import re
import sqlite3
import sys
from collections import Counter, defaultdict

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
WEIGHTS = (1.0, 0.9, 0.8, 0.6)
FOLLOW_UP, REBUILD, GAINED = 0.4, 0.3, 0.1

LEVEL2 = re.compile(r"against its lines \(([\d.]+) s\): (\d+) lines · (-?\d+)% of their moves made on an empty board · value ([\d.]+) · "
                    r"floodgates stop (\d+)% · (\d+)% of the rest left")
MOVE = re.compile(r"^\s+(\d+)\. line (\d+): (.*) · (\w[\w ]*)$")
ANSWER = re.compile(r"^\s{5}(.+?) \(([^)]+)\): (.+?) moves ([\d, ]+) · coverage (\d+)% · value ([\d.]+)$")
PLACE = {"hand": "mano", "field": "mostri", "GY": "Cimitero", "banished": "Cimitero", "S/T zone": "Magie e Trappole"}
# The measure's locations, in the same places.
MEASURE_PLACE = {"Hand": "mano", "Monster Zone": "mostri", "GY": "Cimitero", "Banished": "Cimitero", "Spell/Trap Zone": "Magie e Trappole"}
PLACES = ("Cimitero", "mano", "mostri", "Magie e Trappole")


def follow_value(m):
    f = m.get("follow_up") or {}
    return FOLLOW_UP * (REBUILD * f.get("rebuild", 0) + GAINED * f.get("gained", 0))


def composite(m):
    parts = (m["field"], m["hand"], m["floodgates"], m["resilience"])
    return sum(w * v for w, v in zip(WEIGHTS, parts)) + follow_value(m)


def parse_log(path):
    """Per board block (in the order of the lines file): level 2 numbers, the
    opponent's moves, and your answers with where they are and what they stop."""
    blocks, cur = [], None
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if re.match(r"^#\d+ field:", line):
            cur = dict(moves={}, answers=[], l2=None)
            blocks.append(cur)
            continue
        if cur is None:
            continue
        m = LEVEL2.search(line)
        if m:
            cur["l2"] = dict(lines=int(m.group(2)), made=int(m.group(3)) / 100, value=float(m.group(4)), left=int(m.group(6)) / 100)
            continue
        m = MOVE.match(line)
        if m and cur["l2"] is not None:
            cur["moves"][int(m.group(1))] = (m.group(3), m.group(4))
            continue
        m = ANSWER.match(line)
        if m and cur["l2"] is not None:
            cur["answers"].append(dict(card=m.group(1), place=PLACE.get(m.group(2), m.group(2)),
                                       moves=[int(x) for x in m.group(4).replace(" ", "").split(",") if x], value=float(m.group(6))))
    return blocks


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out")
    p.add_argument("--margin", type=float, default=0.05)
    p.add_argument("--json")
    p.add_argument("--fixture", help="folder: one lines file per hand with a disagreement (only its boards), with the pairs")
    p.add_argument("--db", default=os.path.join(ROOT, "data", "cards.db"))
    a = p.parse_args()
    db = sqlite3.connect(a.db)
    kind = {name: k for name, k in db.execute("SELECT name, kind FROM cards")}
    meta = json.load(open(os.path.join(a.out, "meta.json")))["opponents"]
    weight = {o["deck"]: o["weight"] for o in meta}
    boards = json.load(open(os.path.join(a.out, "boards.json")))

    # Per board: measure (from the first run that has it) and level 2 per opponent.
    info = {b["id"]: dict(b, opp={}, measure=None) for b in boards}
    by_file = defaultdict(list)
    for b in boards:
        by_file[b["hand_file"]].append(b["id"])
    runs = 0
    for log in glob.glob(os.path.join(a.out, "runs", "*.log")):
        base = os.path.basename(log)[:-4]
        hand_file, opp = base.split(".vs.")
        if hand_file not in by_file or opp not in weight:
            continue
        runs += 1
        blocks = parse_log(log)
        items = []
        jpath = log[:-4] + ".json"
        if os.path.exists(jpath):
            try:
                items = json.load(open(jpath))["items"]
            except (ValueError, KeyError):
                items = []
        for i, bid in enumerate(by_file[hand_file]):
            if i < len(items) and items[i].get("measure") and info[bid]["measure"] is None:
                info[bid]["measure"] = items[i]["measure"]
                info[bid]["hand_names"] = items[i].get("hand_names", [])
            if i < len(blocks) and blocks[i]["l2"] and blocks[i]["l2"]["lines"] > 0 and blocks[i]["l2"]["made"] >= 0.8:
                info[bid]["opp"][opp] = blocks[i]

    def truth(b):
        ws = {o: weight[o] for o in b["opp"]}
        total = sum(ws.values())
        return sum(w * (1 - b["opp"][o]["l2"]["left"]) for o, w in ws.items()) / total if total else None

    def l2_place(b, place):
        """Level 2 value of your answers from a place, meta-weighted."""
        ws = {o: weight[o] for o in b["opp"]}
        total = sum(ws.values())
        return sum(w * sum(x["value"] for x in b["opp"][o]["answers"] if x["place"] == place) for o, w in ws.items()) / total

    def extra_share(b):
        """Share of the opponents' summons from the Extra Deck your answers
        stop, minus the share of their other moves, meta-weighted: what
        sets the Extra Deck apart from the rest."""
        num = den = 0.0
        for o, blk in b["opp"].items():
            ed = {k for k, (desc, _) in blk["moves"].items() if "(Extra Deck)" in desc and desc.startswith("Special Summon")}
            rest = set(blk["moves"]) - ed
            if not ed or not rest:
                continue
            stopped = set().union(*[set(x["moves"]) for x in blk["answers"]]) if blk["answers"] else set()
            num += weight[o] * (len(ed & stopped) / len(ed) - len(rest & stopped) / len(rest))
            den += weight[o]
        return num / den if den else 0.0

    def measure_place(m, place):
        return sum(i["value"] for i in m["interruptions"] if MEASURE_PLACE.get(i["location"]) == place)

    def trap_in_hand(b):
        return sum(i["value"] for i in b["measure"]["interruptions"] if i["location"] == "Hand" and kind.get(i["card"]) == "trap")

    rows = []
    for bid, b in info.items():
        if b["measure"] is None or not b["opp"]:
            continue
        b["truth"] = truth(b)
        b["score"] = composite(b["measure"])
        rows.append(b)
    have = {r["id"] for r in rows}
    pairs = []
    for hand_file, ids in by_file.items():
        ids = [i for i in ids if i in have]
        for x in ids:
            for y in ids:
                A, B = info[x], info[y]
                if B["truth"] - A["truth"] >= a.margin and A["score"] > B["score"]:
                    causes = []
                    fa, fb = follow_value(A["measure"]), follow_value(B["measure"])
                    if fa > fb and A["score"] - fa <= B["score"] - fb:
                        causes.append("turno dopo")
                    if trap_in_hand(A) >= 0.1:
                        causes.append("trappole in mano")
                    if extra_share(B) - extra_share(A) >= 0.1:
                        causes.append("Extra Deck")
                    # Each answer of A is worth more at level 2, yet B stops
                    # more: A's answers spoil each other when used together
                    # (one uses a piece the other needs, or its own board).
                    if sum(l2_place(A, q) for q in PLACES) >= sum(l2_place(B, q) for q in PLACES):
                        causes.append("risposte insieme")
                    gaps = {}
                    for place in PLACES:
                        d_l2 = l2_place(B, place) - l2_place(A, place)
                        d_m = measure_place(B["measure"], place) - measure_place(A["measure"], place)
                        gaps[place] = d_l2 - d_m
                    for place, g in sorted(gaps.items(), key=lambda kv: -kv[1]):
                        if g >= 0.1:
                            causes.append(place)
                    pairs.append(dict(hand=hand_file, deck=A["deck"], worse=A["id"], better=B["id"], truth_gap=B["truth"] - A["truth"],
                                      score_gap=A["score"] - B["score"], causes=causes or ["altro"],
                                      worse_board=A["signature"], better_board=B["signature"],
                                      extra=(extra_share(A), extra_share(B)), place_gaps=gaps, follow=(fa, fb)))

    pairs.sort(key=lambda r: -r["truth_gap"])
    n_pairs = sum(len([i for i in ids if i in have]) * (len([i for i in ids if i in have]) - 1) // 2 for ids in by_file.values())
    print(f"{runs} invocazioni lette; {len(rows)} board con verità e misura in {len({r['hand_file'] for r in rows})} mani; "
          f"avversari per board: mediana {sorted(len(r['opp']) for r in rows)[len(rows) // 2] if rows else 0}")
    print(f"coppie della stessa mano: {n_pairs}; disaccordi (verità ≥ {a.margin} al contrario della misura): {len(pairs)} "
          f"in {len({p['hand'] for p in pairs})} mani\n")
    main_cause = Counter(p["causes"][0] for p in pairs)
    any_cause = Counter(c for p in pairs for c in p["causes"])
    print("| causa | principale | presente |")
    print("|---|---:|---:|")
    for c in ("turno dopo", "trappole in mano", "Extra Deck", "risposte insieme") + PLACES + ("altro",):
        print(f"| {c} | {main_cause[c]} | {any_cause[c]} |")
    print("\nPer deck (disaccordi / coppie):\n")
    per_deck = Counter(p["deck"] for p in pairs)
    for d, n in per_deck.most_common():
        print(f"- {d}: {n}")
    print("\nI disaccordi più grandi:\n")
    print("| mano | board peggiore per la misura (A) | board migliore (B) | verità B − A | misura A − B | cause |")
    print("|---|---|---|---:|---:|---|")
    for p in pairs[:25]:
        print(f"| {p['hand']} | {p['worse'].split('#')[1]}: {p['worse_board'][:60]} | {p['better'].split('#')[1]}: {p['better_board'][:60]} | "
              f"{p['truth_gap']:.2f} | {p['score_gap']:.2f} | {', '.join(p['causes'])} |")
    if a.fixture:
        os.makedirs(a.fixture, exist_ok=True)
        hands = defaultdict(list)
        for q in pairs:
            hands[q["hand"]].append(q)
        for hand_file, ps in hands.items():
            src = json.load(open(os.path.join(a.out, "lines", hand_file + ".lines.json")))
            used = sorted({info[q[k]]["index"] for q in ps for k in ("worse", "better")})
            where = {i: n for n, i in enumerate(used)}
            doc = dict(kind="lines", deck=os.path.basename(src.get("deck", "")), hand=src["hand"], seed=src["seed"],
                       items=[src["items"][i] for i in used],
                       a1b=dict(margin=a.margin, boards=[dict(index=where[info[b]["index"]], truth=round(info[b]["truth"], 3),
                                                              score=round(info[b]["score"], 3), board=info[b]["signature"])
                                                         for b in sorted({q[k] for q in ps for k in ("worse", "better")},
                                                                         key=lambda b: info[b]["index"])],
                                pairs=[dict(worse=where[info[q["worse"]]["index"]], better=where[info[q["better"]]["index"]],
                                            truth_gap=round(q["truth_gap"], 3), score_gap=round(q["score_gap"], 3), causes=q["causes"])
                                       for q in ps]))
            json.dump(doc, open(os.path.join(a.fixture, hand_file + ".lines.json"), "w"), separators=(",", ":"))
    if a.json:
        json.dump(dict(margin=a.margin, boards=[dict(id=r["id"], truth=r["truth"], score=r["score"], opponents=len(r["opp"])) for r in rows],
                       pairs=pairs), open(a.json, "w"), indent=1)


if __name__ == "__main__":
    sys.exit(main())
