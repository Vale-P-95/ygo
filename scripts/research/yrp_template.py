#!/usr/bin/env python3
"""Writes an EDOPro replay (.yrp, version 1, uncompressed) with no answers:
only the duel parameters and the two decks. ygo-combo-solver needs a
replay as a duel template even in goal-only mode (--no-ref --deck --hand
--target); this one gives it ours (docs/plans/ricerca-esperimenti.md,
task 3). The opponent's deck is 40 copies of a vanilla monster unless
another .ydk is given.

    python3 scripts/research/yrp_template.py tests/fixtures/decks/invoked-siracusa-md.ydk /tmp/template.yrp
"""
import argparse
import struct

YRP1 = 0x31707279
FLAG_NEWREPLAY = 0x020
FLAG_64BIT_DUELFLAG = 0x100
FLAG_EXTENDED_HEADER = 0x200
# Master Rule 5, as crates/ygo-engine/src/consts.rs (duel_flags::MODE_MR5).
MODE_MR5 = 0x800 | 0x2000 | 0x4000 | 0x8000 | 0x20000
VANILLA = 89631139  # Blue-Eyes White Dragon


def ydk(path):
    main, extra, part = [], [], None
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if line in ("#main", "#extra", "!side"):
            part = line
        elif line.isdigit() and part == "#main":
            main.append(int(line))
        elif line.isdigit() and part == "#extra":
            extra.append(int(line))
    return main, extra


def name(text):
    units = [ord(c) for c in text[:19]]
    return struct.pack("<20H", *(units + [0] * (20 - len(units))))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("deck")
    p.add_argument("out")
    p.add_argument("--opponent", help="the opponent's .ydk")
    p.add_argument("--seed", type=int, default=7)
    args = p.parse_args()
    decks = [ydk(args.deck), ydk(args.opponent) if args.opponent else ([VANILLA] * 40, [])]
    body = b""
    for player in ("DuelForge", "Opponent"):
        body += struct.pack("<I", 1) + name(player)
    body += struct.pack("<III", 8000, 5, 1)  # LP, starting hand, draw per turn
    body += struct.pack("<Q", MODE_MR5)
    for main_deck, extra in decks:
        body += struct.pack("<I", len(main_deck)) + b"".join(struct.pack("<I", c) for c in main_deck)
        body += struct.pack("<I", len(extra)) + b"".join(struct.pack("<I", c) for c in extra)
    body += struct.pack("<I", 0)  # no rule cards
    body += b"\x00"  # no answers
    flag = FLAG_NEWREPLAY | FLAG_64BIT_DUELFLAG | FLAG_EXTENDED_HEADER
    seed = [args.seed, args.seed * 31 + 1, 0xDEADBEEF, 0x12345678]
    header = struct.pack("<IIIII", YRP1, 0x1000, flag, 0, len(body)) + struct.pack("<I", 0) + bytes(8)
    header += struct.pack("<Q", 1) + struct.pack("<4Q", *seed)
    with open(args.out, "wb") as f:
        f.write(header + body)


if __name__ == "__main__":
    main()
