#!/usr/bin/env python3
"""Check the chances of the map of the links (`duelforge-cli links`)
against a measure: opening hands drawn at random, searched as the map
does, and the share of them whose best line brings each card into play
(held, added to the hand, on the field).

    duelforge-cli links --deck <d>.ydk --truth 150 --json <out>/<d>.truth.json
    duelforge-cli links --deck <d>.ydk --json <out>/<d>.json --hands-json <out>/<d>.hands.json
    python3 scripts/links_chance.py <out> [--decks a,b,c]

Prints, for each deck and each way to compute the chance, the mean error
against the measure, the cards at 0.99 or more, and the worst cards.
"""
import argparse
import glob
import json
import os
from collections import Counter
from math import comb

SAT = 0.99


def none_drawn(size, copies, hand=5):
    if copies <= 0:
        return 1.0
    return comb(size - copies, hand) / comb(size, hand) if size - copies >= hand else 0.0


def touched(entry, n):
    """The cards the first `n` lines of a hand bring into play."""
    out = set(entry["hand"])
    for line in entry["lines"][:n]:
        out |= set(line)
    return out


def truth(doc, n):
    hands = doc["hands"]
    count = Counter()
    for h in hands:
        count.update(touched(h, n))
    return {c: count[c] / len(hands) for c in set(doc["main"]) | set(doc["extra"])}


def closure_chance(cards, links, size, steps, only_used=False):
    """The chance of a hand of 5 holding the card or one bringing it in at most `steps` steps."""
    brings = {}
    for l in links:
        if l["kind"] in ("search", "summon", "material", "set") and l["zone"] in ("deck", "extra"):
            if only_used and l["used"] <= 0:
                continue
            brings.setdefault(l["from"], set()).add(l["to"])
    copies = {c["code"]: c["copies"] for c in cards if not c["extra"]}
    reach = {}
    for c in copies:
        seen, front = set(), {c}
        for _ in range(steps):
            front = {n for f in front for n in brings.get(f, ()) if n != c and n not in seen}
            seen |= front
            if not front:
                break
        reach[c] = seen
    out = {}
    for card in cards:
        t = card["code"]
        leading = sum(k for c, k in copies.items() if c == t or t in reach[c])
        out[t] = 1 - none_drawn(size, leading)
    return out


def measured(cards, hands, size, n=1, exact_hand=False, prior=None, strength=0.0):
    """The share of the hands searched for the map whose first `n` lines bring
    the card in, each hand weighted back to its chance in a real duel: the map
    draws hands holding a given card, so a hand counts 1 / (sum over its
    cards of 1 / copies). With `exact_hand`, the chance of holding the card
    is exact and only the rest is measured, on the hands without it. With a
    `prior`, its share counts as `strength` hands more (of average weight)."""
    copies = {c["code"]: c["copies"] for c in cards if not c["extra"]}
    weights = [1 / sum(1 / copies.get(c, 1) for c in h["hand"]) for h in hands]
    sets = [touched(h, n) for h in hands]
    one = sum(weights) / len(weights) if weights else 0.0
    out = {}
    for card in cards:
        t = card["code"]
        held = 1 - none_drawn(size, copies[t]) if exact_hand and not card["extra"] else 0.0
        rest = [(w, s) for w, s, h in zip(weights, sets, hands) if held == 0 or t not in h["hand"]]
        total = sum(w for w, _ in rest)
        got = sum(w for w, s in rest if t in s)
        if prior is not None:
            a = strength * one
            got, total = got + a * max(prior[t] - held, 0.0) / (1 - held), total + a
        out[t] = held + (1 - held) * (got / total if total else 0.0)
    return out


def variants(m, hands):
    cards, links = m["cards"], m["links"]
    size = sum(c["copies"] for c in cards if not c["extra"])
    out = {"map": {c["code"]: c["chance"] for c in cards}}
    for k in (1, 2, 3):
        out[f"closure{k}"] = closure_chance(cards, links, size, k)
    out["closure_used"] = closure_chance(cards, links, size, 99, only_used=True)
    if hands is not None:
        out["measured"] = measured(cards, hands, size)
        out["measured_top3"] = measured(cards, hands, size, n=3)
        out["held+measured"] = measured(cards, hands, size, exact_hand=True)
        for a in (1, 3):
            out[f"held+measured+closure1 x{a}"] = measured(cards, hands, size, exact_hand=True, prior=out["closure1"], strength=a)
    return out


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="folder with <deck>.truth.json, <deck>.json and <deck>.hands.json")
    p.add_argument("--decks", default="", help="comma-separated deck names (default: every truth file)")
    p.add_argument("--worst", type=int, default=5)
    p.add_argument("--extra", action="store_true", help="also the Extra Deck in the error")
    args = p.parse_args()
    names = args.decks.split(",") if args.decks else sorted(os.path.basename(f)[: -len(".truth.json")]
                                                             for f in glob.glob(os.path.join(args.out, "*.truth.json")))
    table = {}
    for name in names:
        t_path, m_path, h_path = (os.path.join(args.out, name + s) for s in (".truth.json", ".json", ".hands.json"))
        if not (os.path.exists(t_path) and os.path.exists(m_path)):
            continue
        doc, m = json.load(open(t_path)), json.load(open(m_path))
        hands = json.load(open(h_path)) if os.path.exists(h_path) else None
        names_of = {c["code"]: c["name"] for c in m["cards"]}
        extra = {c["code"] for c in m["cards"] if c["extra"]}
        best = truth(doc, 1)
        top = {n: truth(doc, n) for n in (3, 20)}
        codes = [c for c in best if args.extra or c not in extra]
        print(f"\n## {name}: {len(doc['hands'])} hands measured, {len(codes)} cards")
        sat_true = sum(1 for c in codes if best[c] >= SAT)
        shift = {n: sum(abs(top[n][c] - best[c]) for c in codes) / len(codes) for n in top}
        print(f"measure: {sat_true} cards at {SAT:.0%}+; the first 3 lines instead of the best add {shift[3]:.3f} on average, "
              f"the first 20 {shift[20]:.3f}")
        for v, est in variants(m, hands).items():
            errs = [(est.get(c, 0.0) - best[c], c) for c in codes]
            mae = sum(abs(e) for e, _ in errs) / len(errs)
            bias = sum(e for e, _ in errs) / len(errs)
            sat = sum(1 for c in codes if est.get(c, 0.0) >= SAT)
            table.setdefault(v, {})[name] = (mae, bias, sat, sat_true)
            worst = sorted(errs, key=lambda e: -abs(e[0]))[: args.worst]
            print(f"  {v:<14} MAE {mae:.3f} bias {bias:+.3f} saturated {sat:>2}  worst: " +
                  "; ".join(f"{names_of.get(c, c)[:28]} {est.get(c, 0):.2f}/{best[c]:.2f}" for _, c in worst))
    print("\n## MAE per variant (saturated in brackets; measure's saturated in the header)")
    decks = [n for n in names if any(n in t for t in table.values())]
    print("| variant | " + " | ".join(f"{d} ({next(iter(table.values()))[d][3]})" for d in decks) + " | mean |")
    print("|---" * (len(decks) + 2) + "|")
    for v, row in table.items():
        cells = [f"{row[d][0]:.3f} ({row[d][2]})" if d in row else "-" for d in decks]
        mean = sum(row[d][0] for d in row) / len(row)
        print(f"| {v} | " + " | ".join(cells) + f" | {mean:.3f} |")


if __name__ == "__main__":
    main()
