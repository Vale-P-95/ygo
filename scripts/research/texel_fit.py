#!/usr/bin/env python3
"""How much the search's estimate could rank boards closer to the probe
turn's measure (preparation of B9, docs/plans/ricerca-esperimenti.md,
task 2), from the JSON of scripts/combo_hunt.py.

A pairwise (Bradley-Terry) logistic fit, Texel style: for two boards of
the same hand whose full measures differ by more than 0.1, the chance the
better one ranks first is sigmoid(s_a - s_b), with

    s = a * quick_score + sum_k w_k * feature_k

and features read from the board itself (cards and zones, the card list in
data/cards.db, the exposure counters). The weights are pulled towards
today's estimate (a fitted alone, w = 0) by an L2 term. Decks are split:
70% to fit, 30% to check, over several random splits.

    python3 scripts/research/texel_fit.py /tmp/hunt60 [/tmp/other ...] --splits 10

Needs numpy.
"""
import argparse
import glob
import json
import math
import os
import random
import re
import sqlite3
import statistics
import sys

import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from rank_report import composite  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
ED = ("Fusion", "Synchro", "Xyz", "Link")
# Words of a card text that hint at an interruption in the opponent's turn.
QUICK = re.compile(r"Quick Effect|\(Quick", re.I)
NEGATE = re.compile(r"\bnegate\b", re.I)
REMOVE = re.compile(r"\b(destroy|banish|return .{0,40}to the hand|shuffle .{0,40}into the Deck)\b", re.I)


class Cards:
    def __init__(self, path):
        db = sqlite3.connect(path)
        self.by_name = {}
        self.by_id = {}
        for cid, name, kind, type_line, race, desc in db.execute("SELECT id, name, kind, type_line, race, desc FROM cards"):
            row = dict(kind=kind, type_line=type_line, race=race, desc=desc)
            self.by_name.setdefault(name.lower(), row)
            self.by_id[cid] = row

    def name(self, text):
        return self.by_name.get(text.lower())


def parse_field(entry):
    """'Name (set) [EMZ]' or 'Name (set) -> Target' → (name, set, emz, equip)."""
    equip = " -> " in entry
    entry = entry.split(" -> ")[0]
    emz = entry.endswith(" [EMZ]")
    entry = entry.removesuffix(" [EMZ]")
    face_down = entry.endswith(" (set)")
    return entry.removesuffix(" (set)"), face_down, emz, equip


FEATURES = [
    "main_monsters", "emz_monsters", "ed_monsters", "link_monsters", "xyz_monsters", "set_monsters",
    "spells_up", "continuous_up", "set_spells", "set_traps", "equips",
    "field_quick", "field_negate", "field_remove", "set_negate",
    "hand", "hand_traps", "hand_other_quick",
    "grave", "banished",
    "summons", "special_summons", "searches", "deck_cost", "lp_paid",
    "cards_used", "choices",
]


def features(board, cards):
    f = dict.fromkeys(FEATURES, 0.0)
    for entry in board["field"]:
        name, face_down, emz, equip = parse_field(entry)
        card = cards.name(name) or dict(kind="monster" if emz else "", type_line="", race="", desc="")
        monster = card["kind"] in ("monster", "token")
        text = card["desc"]
        if monster:
            f["emz_monsters" if emz else "main_monsters"] += 1
            if any(f"|{t}|" in card["type_line"] for t in ED):
                f["ed_monsters"] += 1
            f["link_monsters"] += "|Link|" in card["type_line"]
            f["xyz_monsters"] += "|Xyz|" in card["type_line"]
            f["set_monsters"] += face_down
            if not face_down:
                f["field_quick"] += bool(QUICK.search(text))
                f["field_negate"] += bool(NEGATE.search(text))
                f["field_remove"] += bool(REMOVE.search(text))
        else:
            trap = card["kind"] == "trap"
            if equip:
                f["equips"] += 1
            if face_down:
                f["set_traps" if trap else "set_spells"] += 1
                f["set_negate"] += bool(NEGATE.search(text))
            else:
                f["spells_up"] += 1
                f["continuous_up"] += card["race"] in ("Continuous", "Field")
                f["field_negate"] += bool(NEGATE.search(text))
                f["field_remove"] += bool(REMOVE.search(text))
    f["hand"] = len(board["hand"])
    f["hand_traps"] = board.get("hand_traps_in_hand", 0)
    for code in board["hand"]:
        card = cards.by_id.get(code)
        if card and QUICK.search(card["desc"]):
            f["hand_other_quick"] += 1
    f["hand_other_quick"] = max(0.0, f["hand_other_quick"] - f["hand_traps"])
    f["grave"] = board.get("grave", 0)
    f["banished"] = board.get("banished", 0)
    x = board.get("exposure") or {}
    for k in ("summons", "special_summons", "searches", "deck_cost"):
        f[k] = x.get(k, 0)
    f["lp_paid"] = x.get("lp_paid", 0) / 1000
    f["cards_used"] = board.get("cards_used", 0)
    f["choices"] = board.get("choices", 0) / 10
    return [f[k] for k in FEATURES]


def load(folders, cards):
    hands = {}
    for folder in folders:
        for path in sorted(glob.glob(os.path.join(folder, "*.json"))):
            if path.endswith(".lines.json"):
                continue
            data = json.load(open(path, encoding="utf-8"))
            boards = [b for b in sorted(data["items"], key=lambda b: b["index"]) if b.get("measure")]
            if len(boards) < 2:
                continue
            name = os.path.basename(path)[:-5]
            hands[name] = dict(
                deck=name.rsplit("-", 1)[0],
                quick=[b["quick_score"] for b in boards],
                full=[composite(b) for b in boards],
                x=[features(b, cards) for b in boards],
            )
    return hands


def pairs(h, margin=0.1):
    out = []
    n = len(h["full"])
    for i in range(n):
        for j in range(i + 1, n):
            d = h["full"][i] - h["full"][j]
            if abs(d) > margin:
                out.append((i, j) if d > 0 else (j, i))
    return out


def sigmoid(z):
    return 1 / (1 + math.exp(-z)) if z > -30 else 0.0


def fit(hands, names, use, scale, prior, lam, steps=300):
    """Newton steps on the mean pairwise log loss plus lam * |θ - prior|².
    θ[0] multiplies quick_score, θ[1:] the features `use` (standardized)."""
    diffs = []
    for name in names:
        h = hands[name]
        rows = np.column_stack([np.array(h["quick"])] + [(np.array(h["x"])[:, k] - scale[k][0]) / scale[k][1] for k in use])
        pr = pairs(h)
        if pr:
            a, b = np.array(pr).T
            diffs.append(rows[a] - rows[b])
    prior = np.array(prior, dtype=float)
    theta = prior.copy()
    if not diffs:
        return list(theta)
    d = np.vstack(diffs)
    eye = np.eye(len(theta))
    for _ in range(steps):
        p = 1 / (1 + np.exp(-np.clip(d @ theta, -30, 30)))
        grad = d.T @ (p - 1) / len(d) + 2 * lam * (theta - prior)
        hess = (d.T * (p * (1 - p))) @ d / len(d) + 2 * lam * eye + 1e-9 * eye
        step = np.linalg.solve(hess, grad)
        theta -= step
        if np.abs(step).max() < 1e-7:
            break
    return list(theta)


def score(h, theta, use, scale):
    return [theta[0] * h["quick"][i] + sum(t * (h["x"][i][k] - scale[k][0]) / scale[k][1] for t, k in zip(theta[1:], use))
            for i in range(len(h["quick"]))]


def top8_inversions(h, est, top=8, margin=0.1):
    """C5.7 with the estimate `est`: among the 8 boards it ranks first, pairs
    it orders one way and the measure the other by more than 0.1."""
    order = sorted(range(len(est)), key=lambda i: (-est[i], i))[:top]
    inv = tot = 0
    for a in range(len(order)):
        for b in range(a + 1, len(order)):
            tot += 1
            inv += h["full"][order[b]] - h["full"][order[a]] > margin
    return inv, tot


def kendall(x, y):
    conc = disc = 0
    for i in range(len(x)):
        for j in range(i + 1, len(x)):
            s = (x[i] - x[j]) * (y[i] - y[j])
            conc += s > 0
            disc += s < 0
    return (conc - disc) / (conc + disc) if conc + disc else None


def best_kept(h, est, k):
    """The final #1 is among the k first of the estimate."""
    best = max(range(len(h["full"])), key=lambda i: (h["full"][i], -i))
    return best in sorted(range(len(est)), key=lambda i: (-est[i], i))[:k]


def evaluate(hands, names, est_of):
    inv = tot = 0
    taus, kept10, kept15, pairwise_ok, pairwise_n = [], 0, 0, 0, 0
    for name in names:
        h = hands[name]
        est = est_of(h)
        a, b = top8_inversions(h, est)
        inv += a
        tot += b
        if (t := kendall(est, h["full"])) is not None:
            taus.append(t)
        kept10 += best_kept(h, est, 10)
        kept15 += best_kept(h, est, 15)
        for p, q in pairs(h):
            pairwise_n += 1
            pairwise_ok += est[p] > est[q]
    return dict(inv=inv, tot=tot, tau=statistics.median(taus) if taus else float("nan"), kept10=kept10, kept15=kept15,
                acc=pairwise_ok / pairwise_n if pairwise_n else float("nan"), n=len(names))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("folders", nargs="+")
    p.add_argument("--db", default=os.path.join(ROOT, "data", "cards.db"))
    p.add_argument("--splits", type=int, default=10)
    p.add_argument("--lam", type=float, nargs="+", default=[0.001, 0.01, 0.1])
    p.add_argument("--seed", type=int, default=1)
    args = p.parse_args()
    cards = Cards(args.db)
    hands = load(args.folders, cards)
    decks = sorted({h["deck"] for h in hands.values()})
    nb = sum(len(h["quick"]) for h in hands.values())
    print(f"{len(hands)} mani di {len(decks)} deck, {nb} board, {sum(len(pairs(h)) for h in hands.values())} coppie con misure distanti più di 0,1\n")

    use = list(range(len(FEATURES)))
    rng = random.Random(args.seed)
    results = {"oggi": [], **{f"lam {lam}": [] for lam in args.lam}}
    weights = {lam: [] for lam in args.lam}
    for _ in range(args.splits):
        shuffled = decks[:]
        rng.shuffle(shuffled)
        test_decks = set(shuffled[: max(1, round(0.3 * len(decks)))])
        train = [n for n, h in hands.items() if h["deck"] not in test_decks]
        test = [n for n, h in hands.items() if h["deck"] in test_decks]
        allx = [x for n in train for x in hands[n]["x"]]
        scale = []
        for k in range(len(FEATURES)):
            col = [x[k] for x in allx]
            sd = statistics.pstdev(col) if len(col) > 1 else 0.0
            scale.append((statistics.fmean(col), sd if sd > 0 else 1.0))
        # Today's estimate, only rescaled: the prior.
        a = fit(hands, train, [], scale, [1.0], 0.0)[0]
        prior = [a] + [0.0] * len(use)
        results["oggi"].append(evaluate(hands, test, lambda h: h["quick"]))
        for lam in args.lam:
            theta = fit(hands, train, use, scale, prior, lam)
            weights[lam].append([t / s[1] for t, s in zip(theta[1:], scale)] + [theta[0]])
            results[f"lam {lam}"].append(evaluate(hands, test, lambda h, th=theta: score(h, th, use, scale)))

    print(f"Verifica sul 30% dei deck lasciati fuori, {args.splits} divisioni a caso (somme sulle divisioni):\n")
    print("| stima | inversioni nella top 8 (C5.7) | coppie giuste | Kendall (mediana) | #1 nelle prime 10 | #1 nelle prime 15 |")
    print("|---|---:|---:|---:|---:|---:|")
    for label, rs in results.items():
        inv = sum(r["inv"] for r in rs)
        tot = sum(r["tot"] for r in rs)
        n = sum(r["n"] for r in rs)
        print(f"| {label} | {inv}/{tot} ({inv / tot:.1%}) | {statistics.fmean(r['acc'] for r in rs):.1%} | "
              f"{statistics.fmean(r['tau'] for r in rs):.2f} | {sum(r['kept10'] for r in rs)}/{n} | {sum(r['kept15'] for r in rs)}/{n} |")

    lam = args.lam[len(args.lam) // 2]
    print(f"\nPesi medi con lam {lam} (per unità della caratteristica; stima = a · quick_score + Σ w · x), con la deviazione tra le divisioni:\n")
    ws = weights[lam]
    rows = []
    for k, name in enumerate(FEATURES + ["a (quick_score)"]):
        col = [w[k] for w in ws]
        rows.append((name, statistics.fmean(col), statistics.pstdev(col)))
    for name, m, s in sorted(rows, key=lambda r: -abs(r[1])):
        print(f"- {name}: {m:+.3f} ± {s:.3f}")


if __name__ == "__main__":
    main()
