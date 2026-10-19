import { useState } from "react";
import type { DeckSummary } from "../lib/api";
import { resolveLook } from "../lib/boxes";
import { FORMAT_LABELS } from "../lib/deck";
import { sound } from "../lib/sound";
import { Popover, useOutsideClose } from "./Popover";

/** The deck being worked on, with a menu of the other decks on the shelf. */
export function DeckMenu({ decks, current, onPick }: { decks: DeckSummary[] | null; current: number | null | undefined; onPick: (id: number) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useOutsideClose(open, () => setOpen(false));
  const deck = decks?.find((d) => d.id === current);
  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        className="btn btn-ghost !py-1.5"
        onClick={() => {
          sound.play(open ? "close" : "open");
          setOpen((v) => !v);
        }}
        aria-expanded={open}
        disabled={!decks?.length}
      >
        {deck && <span className="h-5 w-4 rounded-[3px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.25)]" style={{ background: resolveLook(deck.box, deck.id).color }} />}
        <span className="max-w-64 truncate">{deck?.name ?? "Scegli un deck"}</span>
        {deck && <span className="text-xs text-faint">{FORMAT_LABELS[deck.format]}</span>}
        <span className="text-faint">▾</span>
      </button>
      <Popover open={open} width={320}>
        {decks?.map((d) => (
          <button
            key={d.id}
            type="button"
            onClick={() => {
              setOpen(false);
              sound.play("page");
              onPick(d.id);
            }}
            className={`flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-surface-3 ${d.id === current ? "bg-surface-3" : ""}`}
          >
            <span className="h-7 w-5 shrink-0 rounded-[3px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.25)]" style={{ background: resolveLook(d.box, d.id).color }} />
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm font-medium">{d.name}</span>
              <span className="text-[11px] text-faint">
                {FORMAT_LABELS[d.format]} · {d.mainCount}/{d.extraCount}/{d.sideCount}
              </span>
            </span>
          </button>
        ))}
      </Popover>
    </div>
  );
}
