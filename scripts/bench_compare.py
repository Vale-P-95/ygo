#!/usr/bin/env python3
"""Compares two runs of scripts/bench.py hand by hand, as a Markdown table.
Each side can be several results.json files (the shards of a run on many
machines, as the Bench workflow makes).

    python3 scripts/bench_compare.py --base before/results.json --head after/results.json
"""
import argparse
import json


def load(paths):
    rows = {}
    for p in paths:
        for r in json.load(open(p, encoding="utf-8")):
            rows[r["name"]] = r
    return rows


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--base", nargs="+", required=True, help="results.json files of the version before")
    p.add_argument("--head", nargs="+", required=True, help="results.json files of the version after")
    p.add_argument("--title", default="Bench")
    a = p.parse_args()
    base, head = load(a.base), load(a.head)
    names = [n for n in base if n in head] + [n for n in head if n not in base]
    out = [f"### {a.title}", "", "| mano | best prima | best dopo | Δ | primo prima | primo dopo | nodi | replay | board | errori |",
           "|---|---:|---:|---:|---:|---:|---|---|---|---|"]
    for n in names:
        b, h = base.get(n), head.get(n)
        if not b or not h:
            out.append(f"| {n} | {'—' if not b else b['best']} | {'—' if not h else h['best']} | | | | | | | solo da un lato |")
            continue
        delta = h["best"] - b["best"]
        mark = " ▲" if delta > 0.05 else " ▼" if delta < -0.05 else ""
        errors = "" if b["rc"] == 0 and h["rc"] == 0 else f"rc {b['rc']}/{h['rc']}"
        out.append(f"| {n} | {b['best']:.2f} | {h['best']:.2f} | {delta:+.2f}{mark} | {b['first']:.2f} | {h['first']:.2f} | "
                   f"{b['nodes']}→{h['nodes']} | {b['replays']}→{h['replays']} | {b['boards']}→{h['boards']} | {errors} |")
    both = [n for n in names if n in base and n in head]
    sb = sum(base[n]["best"] for n in both)
    sh = sum(head[n]["best"] for n in both)
    fb = sum(base[n]["first"] for n in both)
    fh = sum(head[n]["first"] for n in both)
    rb = sum(base[n]["replays"] for n in both)
    rh = sum(head[n]["replays"] for n in both)
    out += ["", f"Somma dei board migliori: **{sb:.2f} → {sh:.2f}** ({sh - sb:+.2f}); dei primi board trovati: {fb:.2f} → {fh:.2f} "
            f"({fh - fb:+.2f}); replay: {rb} → {rh}. Mani confrontate: {len(both)} di {len(names)}."]
    print("\n".join(out))


if __name__ == "__main__":
    main()
