#!/usr/bin/env python3
"""Check the probe turn against a chosen deck (`duelforge-cli probe-line
--vs`) on many pairs of decks: the breakers and handtraps it reads from the
opponent's deck, the resilience against it, and where each handtrap stops
the line. Flags the cases to look at by hand.

    python3 scripts/mdm_decks.py --out /tmp/mdm
    python3 scripts/matchup_report.py /tmp/mu --decks /tmp/mdm --own branded,yubel --vs kaiju,labrynth
    python3 scripts/matchup_report.py /tmp/mu --decks /tmp/mdm --own branded,yubel --vs kaiju,labrynth --skip-run

Each deck of --own gets a hand drawn with a fixed seed (as scripts/bench.py
does; another seed when the search finds no board of 2 cards), its best
lines are searched and saved once, then measured against each deck of --vs.
Writes the logs and <out>/report.md.
"""
import argparse
import json
import os
import random
import re
import sqlite3
import subprocess
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
CDB = os.path.join(ROOT, "data", "cdb", "cards.cdb")

# Reference for the check only (the engine reads them; these are the
# handtraps players know): name -> what it should stop.
KNOWN_HAND_TRAPS = {
    'Maxx "C"': "nothing", "Mulcharmy Fuwalos": "nothing", "Mulcharmy Purulia": "nothing", "Mulcharmy Nyalus": "nothing",
    "Ash Blossom & Joyous Spring": "search, summon from Deck, send from Deck",
    "Effect Veiler": "monster effect on the field", "Infinite Impermanence": "monster effect on the field",
    "Ghost Ogre & Snow Rabbit": "card on the field", "Nibiru, the Primal Being": "5th summon",
    "Droll & Lock Bird": "second card added", "Ghost Belle & Haunted Mansion": "GY effect, from GY",
    "Ghost Mourner & Moonlit Chill": "monster from the GY", "D.D. Crow": "card in the GY",
    "Bystial Saronir": "LIGHT/DARK in the GY", "Bystial Magnamhut": "LIGHT/DARK in the GY", "Bystial Druiswurm": "LIGHT/DARK in the GY",
    "Dominus Impulse": "monster effect", "Dominus Spark": "spell/trap", "Dominus Purge": "monster effect",
    "Psy-Framegear Gamma": "monster effect", "Crossout Designator": "card with the same name",
    "Called by the Grave": "GY effect", "Dimension Shifter": "GY",
}
NOTHING = {n for n, v in KNOWN_HAND_TRAPS.items() if v == "nothing"}


def names(codes):
    db = sqlite3.connect(CDB)
    out = {}
    for c in codes:
        row = db.execute("select name from texts where id = ?", (c,)).fetchone()
        out[c] = row[0] if row else str(c)
    return out


def main_deck(path):
    return [int(w) for w in open(path).read().split("#extra")[0].split() if w.isdigit()]


def search(cli, deck, out, nodes, top, tries):
    """Saves the best lines of a hand of `deck`; the hand and the search output."""
    name = os.path.splitext(os.path.basename(deck))[0]
    lines = os.path.join(out, name + ".lines.json")
    for k in range(tries):
        hand = ";".join(map(str, random.Random(f"{name}-{k}").sample(main_deck(deck), 5)))
        started = time.time()
        r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "combos", "--deck", deck, "--hand", hand, "--nodes", str(nodes),
                            "--seconds", "900", "--threads", "1", "--top", str(top), "--save-lines", lines], capture_output=True, text=True)
        open(os.path.join(out, name + ".combos.log"), "w").write(r.stdout + "\n--- stderr ---\n" + r.stderr[-3000:])
        fields = re.findall(r"^   field: (.*)$", r.stdout, re.M)
        if fields and len(fields[0].split(", ")) >= 2:
            return dict(hand=hand, seed=k, search_s=round(time.time() - started, 1), boards=len(fields), field=fields[0])
    return None


def measure(cli, deck, rival, out):
    own = os.path.splitext(os.path.basename(deck))[0]
    vs = os.path.splitext(os.path.basename(rival))[0]
    base = os.path.join(out, f"{own}.vs.{vs}")
    started = time.time()
    env = dict(os.environ, DUELFORGE_DEBUG_MATCHUP="1")
    r = subprocess.run([cli, "--data", os.path.join(ROOT, "data"), "probe-line", "--deck", deck, "--lines", os.path.join(out, own + ".lines.json"),
                        "--vs", rival, "--json", base + ".json"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=env)
    # One stream: the debug lines of a board stay under it (Rust flushes stdout by line).
    open(base + ".log", "w").write(r.stdout)
    json.dump({"wall": round(time.time() - started, 1), "rc": r.returncode}, open(base + ".time.json", "w"))


def parse(out, own, vs):
    """Per board: generic resilience, the matchup, its time, the debug lines."""
    base = os.path.join(out, f"{own}.vs.{vs}")
    if not os.path.exists(base + ".log"):
        return None
    log = open(base + ".log").read()
    head = dict(breakers=(re.search(r"^   breakers: (.*)$", log, re.M) or [None, ""])[1],
                hand_traps=(re.search(r"^   handtraps: (.*)$", log, re.M) or [None, ""])[1],
                wall=json.load(open(base + ".time.json"))["wall"])
    boards = []
    for chunk in re.split(r"^#\d+ field: ", log, flags=re.M)[1:]:
        b = dict(field=chunk.splitlines()[0])
        m = re.search(r"probe turn \(([\d.]+) s\): field ([\d.]+) · hand [\d.]+ · floodgates [\d.]+ · resilience ([\d.]+)", chunk)
        b["probe_s"], b["field_value"], b["resilience"] = (float(m[1]), float(m[2]), float(m[3])) if m else (None, None, None)
        m = re.search(r"against the deck \(([\d.]+) s\): resilience ([\d.]+)", chunk)
        b["vs_s"], b["vs_resilience"] = (float(m[1]), float(m[2])) if m else (None, None)
        b["failed"] = "matchup failed" in chunk or "probe turn failed" in chunk
        b["debug"] = [l.strip() for l in chunk.splitlines() if re.match(r"^   (?:\S.* at \d+: |diverged at answer)", l)]
        boards.append(b)
    if os.path.exists(base + ".json"):
        for b, item in zip(boards, json.load(open(base + ".json"))["items"]):
            b["matchup"] = item.get("matchup")
    return head, boards


def flags(hts, nm):
    """The cases to look at: a handtrap that should stop nothing and does."""
    out = []
    for h in hts:
        n = nm.get(h["code"], str(h["code"]))
        if n in NOTHING and h["stops"] > 0:
            out.append(f"{n} stops {h['stops']}/{h['windows']}")
    return out


def report(out, owns, vss, searched):
    codes = set()
    data = {}
    for own in owns:
        for vs in vss:
            p = parse(out, own, vs)
            data[(own, vs)] = p
            if p:
                for b in p[1]:
                    for h in (b.get("matchup") or {}).get("hand_traps", []):
                        codes.add(h["code"])
                        codes.update(h["at"])
                    for x in (b.get("matchup") or {}).get("breakers", []):
                        codes.add(x["code"])
    nm = names(codes)
    lines = ["# Matchup report", "", "## Own decks", "", "| Deck | Hand seed | Search (s) | Boards | Best field |", "|---|---|---|---|---|"]
    for own in owns:
        s = searched.get(own)
        lines.append(f"| {own} | {s['seed']} | {s['search_s']} | {s['boards']} | {s['field']} |" if s else f"| {own} | no board | | | |")
    lines += ["", "## Opponents", "", "| Deck | Breakers read | Handtraps read |", "|---|---|---|"]
    for vs in vss:
        head = next((data[(o, vs)][0] for o in owns if data.get((o, vs))), None)
        if head:
            lines.append(f"| {vs} | {head['breakers']} | {head['hand_traps']} |")
    lines += ["", "## Boards", "",
              "| Own | Vs | # | Field | Resilience → vs | Breakers (left) | Stopped | Handtraps: stops/windows | Time vs (s) | Flags |",
              "|---|---|---|---|---|---|---|---|---|---|"]
    all_flags = []
    for own in owns:
        for vs in vss:
            p = data.get((own, vs))
            if not p:
                continue
            for i, b in enumerate(p[1]):
                m = b.get("matchup") or {}
                brk = ", ".join(f"{nm.get(x['code'])} {x['remaining']:.2f}" + (" n/a" if x["not_applicable"] else "") for x in m.get("breakers", []))
                hts = m.get("hand_traps", [])
                ht = ", ".join(f"{nm.get(h['code'])} {h['stops']}/{h['windows']}" for h in hts)
                f = flags(hts, nm) + (["FAILED"] if b["failed"] else [])
                if f:
                    all_flags.append((own, vs, i + 1, f, [d for d in b["debug"] if "lost" in d or "diverged at" in d or "error" in d.lower()]))
                res = f"{b['resilience']} → {b['vs_resilience']}" if b["resilience"] is not None else "?"
                chance = f"{m.get('hand_trap_chance', 0) * 100:.0f}%" if m else "?"
                lines.append(f"| {own} | {vs} | {i + 1} | {b['field_value']} | {res} | {brk or 'standard'} | {chance} | {ht} | {b['vs_s']} | {'; '.join(f)} |")
    lines += ["", f"## Flags ({len(all_flags)})", ""]
    for own, vs, i, f, dbg in all_flags:
        lines.append(f"- {own} vs {vs} #{i}: {'; '.join(f)}")
        lines += [f"  - `{d[:220]}`" for d in dbg[:6]]
    open(os.path.join(out, "report.md"), "w").write("\n".join(lines) + "\n")
    print(f"{len(all_flags)} flags -> {os.path.join(out, 'report.md')}")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("out", help="folder for the lines, logs and report.md")
    p.add_argument("--decks", required=True, help="folder with .ydk files")
    p.add_argument("--own", required=True, help="comma-separated deck names whose lines are measured")
    p.add_argument("--vs", required=True, help="comma-separated deck names to measure them against")
    p.add_argument("--cli", default=os.path.join(ROOT, "target", "release", "duelforge-cli"))
    p.add_argument("--nodes", type=int, default=3000)
    p.add_argument("--top", type=int, default=3)
    p.add_argument("--tries", type=int, default=3, help="hands tried for a deck before giving up")
    p.add_argument("--skip-run", action="store_true", help="only read the logs already written")
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    owns, vss = a.own.split(","), a.vs.split(",")
    deck = lambda n: os.path.join(a.decks, n + ".ydk")
    state = os.path.join(a.out, "searched.json")
    searched = json.load(open(state)) if os.path.exists(state) else {}
    if not a.skip_run:
        for own in owns:
            if own not in searched:
                searched[own] = search(a.cli, deck(own), a.out, a.nodes, a.top, a.tries)
                json.dump(searched, open(state, "w"), indent=1)
            print(own, "searched", flush=True)
            if not searched[own]:
                continue
            for vs in vss:
                if vs != own:
                    measure(a.cli, deck(own), deck(vs), a.out)
                    print(" ", own, "vs", vs, flush=True)
    report(a.out, owns, vss, searched)


if __name__ == "__main__":
    main()
