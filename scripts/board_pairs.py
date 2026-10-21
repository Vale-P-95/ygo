#!/usr/bin/env python3
"""Pairs of boards whose order is known (tests/fixtures/boards/pairs.tsv),
measured the same way before and after a change to the probe turn
(docs/plans/classificazione-board.md, C2 and C5.1).

A board is told by its cards, not by its rank (ranks move whenever the
search changes): each side of a pair is a list of conditions separated by
";", all of which the board must meet:
- "Name": a card on your field whose name contains Name;
- "2xName": at least two of them;
- "!Name": none;
- "re:REGEX": a field entry matching the regular expression (an equipped
  card reads "Equip -> Monster", an Extra Monster Zone "Monster [EMZ]");
- "hand:Name", "hand:-" (empty hand), "!hand:-" (not empty);
- "sig=SIGNATURE": exactly these cards on the field and in the hand
  (the board's signature with the zones left out), as `panel` writes.

    python3 scripts/board_pairs.py make $S/pairs --decks $S/mdm --extra-decks $S/decks --cli $CLI_BEFORE
    python3 scripts/board_pairs.py eval $S/pairs --cli $CLI_AFTER
    python3 scripts/board_pairs.py sheet $S/json-1 $S/sheet.md
    python3 scripts/board_pairs.py panel $S/json-1 $S/sheet.md $S/judge-1.json $S/judge-2.json $S/judge-3.json

`make` searches each deck of the pairs once (hand of the "-lines" rule,
one thread, a fixed seed) and saves the lines of the first board meeting
each side; `eval` measures those lines again with `probe-line` (no
search), so the same boards are compared whatever the search finds now.
`sheet` writes, for the judges, the first 5 distinct boards of each deck
in a shuffled order and without scores; `panel` reads the judges'
rankings of that sheet and prints, as lines for pairs.tsv, the pairs all
judges put in the same order (no tie, no low confidence).
"""
import argparse
import json
import os
import random
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bench import WEIGHTS  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
PAIRS = os.path.join(ROOT, "tests", "fixtures", "boards", "pairs.tsv")


def read_pairs():
    out = []
    for line in open(PAIRS, encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            continue
        pid, deck, a, b, expected, certainty, why = line.rstrip("\n").split("\t")
        out.append(dict(id=pid, deck=deck, a=a, b=b, expected=expected, certainty=certainty, why=why))
    return out


def meets(board, spec):
    field = board["field"]
    hand = board["hand_names"]
    if spec.startswith("sig="):
        return unzoned(board["signature"]) == spec[4:]
    for cond in (c.strip() for c in spec.split(";") if c.strip()):
        negate = cond.startswith("!")
        cond = cond[1:] if negate else cond
        if cond == "hand:-":
            ok = not hand
        elif cond.startswith("hand:"):
            ok = any(cond[5:] in h for h in hand)
        elif cond.startswith("re:"):
            ok = any(re.search(cond[3:], f) for f in field)
        elif re.match(r"\d+x", cond):
            n, name = cond.split("x", 1)
            ok = sum(name in f for f in field) >= int(n)
        else:
            ok = any(cond in f for f in field)
        if ok == negate:
            return False
    return True


def unzoned(signature):
    """The cards, wherever they are (the same board in other zones)."""
    return re.sub(r" \[EMZ\]", "", signature)


def composite(measure):
    return sum(w * measure[k] for w, k in zip(WEIGHTS, ("field", "hand", "floodgates", "resilience")))


def deck_path(deck, decks, extra):
    if deck.startswith("x_") and extra:
        return os.path.join(extra, deck[2:] + ".ydk")
    return os.path.join(decks, deck + ".ydk")


def hand_of(path, deck):
    main = [w for w in open(path).read().split("#extra")[0].split() if w.isdigit()]
    return random.Random(f"{deck}-lines").sample(main, 5)


def make(a):
    os.makedirs(a.out, exist_ok=True)
    pairs = read_pairs()
    found = {}
    for deck in sorted({p["deck"] for p in pairs}):
        path = deck_path(deck, a.decks, a.extra_decks)
        if not os.path.exists(path):
            print(f"{deck}: no deck file", file=sys.stderr)
            continue
        hand = hand_of(path, deck)
        boards_json = os.path.join(a.out, f"{deck}.json")
        lines_json = os.path.join(a.out, f"{deck}.lines.json")
        subprocess.run([a.cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", path, "--hand", ";".join(hand),
                        "--nodes", str(a.nodes), "--seconds", "900", "--threads", "1", "--seed", str(a.seed), "--top", str(a.top),
                        "--json", boards_json, "--save-lines", lines_json], capture_output=True, text=True)
        boards = json.load(open(boards_json))["items"] if os.path.exists(boards_json) else []
        for p in (p for p in pairs if p["deck"] == deck):
            sides = {}
            for side in ("a", "b"):
                k = next((i for i, b in enumerate(boards) if meets(b, p[side])), None)
                sides[side] = None if k is None else {"index": k, "signature": boards[k]["signature"]}
            found[p["id"]] = {"deck": deck, "path": path, "hand": hand, **sides}
            print(f"{p['id']} {deck}: A {'#' + str(sides['a']['index']) if sides['a'] else 'non trovata'}, "
                  f"B {'#' + str(sides['b']['index']) if sides['b'] else 'non trovata'}")
    json.dump({"seed": a.seed, "pairs": found}, open(os.path.join(a.out, "pairs.json"), "w"), indent=1)


def measure(a, saved, side):
    s = saved[side]
    out = os.path.join(a.out, f"measure-{saved['deck']}-{s['index']}.json")
    subprocess.run([a.cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", saved["path"], "--hand", ";".join(saved["hand"]),
                    "--seed", str(a.seed), "--lines", os.path.join(a.out, f"{saved['deck']}.lines.json"), "--index", str(s["index"]),
                    "--json", out], capture_output=True, text=True)
    items = json.load(open(out))["items"] if os.path.exists(out) else []
    return composite(items[0]["measure"]) if items and "measure" in items[0] else None


def evaluate(a):
    saved = json.load(open(os.path.join(a.out, "pairs.json")))
    a.seed = saved["seed"]
    rows = []
    for p in read_pairs():
        s = saved["pairs"].get(p["id"])
        if not s or not s["a"] or not s["b"]:
            rows.append((p, None, None, "non trovata"))
            continue
        ca, cb = measure(a, s, "a"), measure(a, s, "b")
        if ca is None or cb is None:
            rows.append((p, ca, cb, "misura fallita"))
            continue
        d = ca - cb
        got = "A≈B" if abs(d) < 0.05 else ("A>B" if d > 0 else "B>A")
        verdict = "—" if p["expected"] not in ("A>B", "B>A", "A≈B") else ("giusta" if got == p["expected"] else f"sbagliata ({got})")
        rows.append((p, ca, cb, verdict))
    print("| id | deck | A | B | atteso | sicurezza | esito |")
    print("|---|---|---:|---:|---|---|---|")
    for p, ca, cb, verdict in rows:
        f = lambda x: "—" if x is None else f"{x:.2f}"  # noqa: E731
        print(f"| {p['id']} | {p['deck']} | {f(ca)} | {f(cb)} | {p['expected']} | {p['certainty']} | {verdict} |")
    for certainty in ("sicura", "da confermare"):
        judged = [r for r in rows if r[0]["certainty"] == certainty and (r[3] == "giusta" or r[3].startswith("sbagliata"))]
        right = sum(r[3] == "giusta" for r in judged)
        print(f"\n{certainty}: {right} giuste su {len(judged)} misurate", end="")
    print()


def sheet(a):
    rnd = random.Random("sheet")
    out = ["# Board da ordinare", "", "Per ogni deck: le prime 5 board distinte trovate dalla ricerca, in ordine casuale, senza punteggi. "
           "Ordinale dalla più forte alla più debole come board del primo turno contro un avversario qualsiasi.", ""]
    for path in sorted(p for p in os.listdir(a.folder) if p.endswith(".json") and not p.endswith(".lines.json")):
        boards = json.load(open(os.path.join(a.folder, path)))["items"]
        seen, distinct = set(), []
        for b in boards:
            key = re.sub(r" \[EMZ\]", "", b["signature"])
            if key not in seen:
                seen.add(key)
                distinct.append(b)
        distinct = distinct[:5]
        rnd.shuffle(distinct)
        out += [f"## {path[:-5]}", ""]
        for i, b in enumerate(distinct):
            out.append(f"- **{chr(65 + i)}** (indice {b['index']}): campo: {', '.join(b['field']) or '—'}; mano: {', '.join(b['hand_names']) or '—'}")
        out.append("")
    open(a.out_md, "w", encoding="utf-8").write("\n".join(out))


def panel(a):
    """Pairs all the judges agree on, as pairs.tsv lines."""
    letters = {}
    deck = None
    for line in open(a.sheet, encoding="utf-8"):
        m = re.match(r"## (\S+)", line)
        if m:
            deck = m.group(1)
            continue
        m = re.match(r"- \*\*(\w)\*\* \(indice (\d+)\)", line)
        if m and deck:
            letters.setdefault(deck, {})[m.group(1)] = int(m.group(2))
    judges = [json.load(open(j, encoding="utf-8")) for j in a.judges]
    n = 0
    for deck in sorted(letters):
        verdicts = [j.get(deck) for j in judges]
        if any(v is None or v.get("confidence") == "bassa" for v in verdicts):
            continue
        boards = {b["index"]: b for b in json.load(open(os.path.join(a.folder, deck + ".json"), encoding="utf-8"))["items"]}
        tied = [{frozenset(t) for t in v.get("ties", []) for _ in [0]} for v in verdicts]
        ranks = [{x: i for i, x in enumerate(v["order"])} for v in verdicts]
        agreed = []
        for x in letters[deck]:
            for y in letters[deck]:
                if x == y or any(any({x, y} <= t for t in ts) for ts in tied):
                    continue
                if all(r.get(x, 99) < r.get(y, 99) for r in ranks):
                    gap = min(r[y] - r[x] for r in ranks)
                    agreed.append((gap, x, y))
        # The widest agreements first, two per deck at most.
        for gap, x, y in sorted(agreed, reverse=True)[: a.per_deck]:
            n += 1
            bx, by = boards[letters[deck][x]], boards[letters[deck][y]]
            why = "; ".join(v.get("note", "") for v in verdicts[:1])
            name = deck[:-2] if deck.endswith("-0") else deck
            print(f"J{n}\t{name}\tsig={unzoned(bx['signature'])}\tsig={unzoned(by['signature'])}\tA>B\tsicura\tgiudici d'accordo 3 su 3: {why}")
    print(f"# {n} coppie", file=sys.stderr)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("make")
    m.add_argument("out")
    m.add_argument("--decks", required=True)
    m.add_argument("--extra-decks")
    m.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    m.add_argument("--nodes", type=int, default=3000)
    m.add_argument("--top", type=int, default=30, help="boards listed, among which the sides are looked for")
    m.add_argument("--seed", type=int, default=1)
    e = sub.add_parser("eval")
    e.add_argument("out")
    e.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    s = sub.add_parser("sheet")
    s.add_argument("folder", help="a folder written by combo_hunt.py --json")
    s.add_argument("out_md")
    j = sub.add_parser("panel")
    j.add_argument("folder", help="the folder the sheet was made from")
    j.add_argument("sheet")
    j.add_argument("judges", nargs="+", help="the judges' JSON files")
    j.add_argument("--per-deck", type=int, default=2)
    a = p.parse_args()
    {"make": make, "eval": evaluate, "sheet": sheet, "panel": panel}[a.cmd](a)


if __name__ == "__main__":
    main()
