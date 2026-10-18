import type { ReactNode } from "react";
import type { Card, DeckFormat } from "../lib/api";
import { copyLimit, type Zone } from "../lib/deck";
import { useDrag } from "../lib/drag";
import { CardTile, type TileBadge } from "./CardTile";

interface Props {
  zone: Zone;
  title: string;
  ids: number[];
  /** Cards per row. */
  columns: number;
  /** Empty slots shown up to this size (40 for the Main Deck, 15 otherwise). */
  slots: number;
  max: number;
  format: DeckFormat;
  getCard: (id: number) => Card | undefined;
  invalid: Set<number>;
  fresh: { zone: Zone; index: number; nonce: number } | null;
  info?: ReactNode;
  onRemove: (zone: Zone, index: number) => void;
  onOpen: (id: number) => void;
  onHover: (id: number, el: HTMLElement) => void;
  onHoverEnd: () => void;
  emptyHint?: ReactNode;
  /** Dashed frame, for areas that are not part of the deck. */
  dashed?: boolean;
}

function banBadge(card: Card | undefined, format: DeckFormat): TileBadge | undefined {
  if (!card) return undefined;
  const limit = copyLimit(card, format);
  if (limit === 0) return { text: "✕", tone: "ban" };
  if (limit < 3) return { text: String(limit), tone: "limit" };
  return undefined;
}

/** One deck section: a drop target with one tile per copy and empty slots. */
export function DeckZone(props: Props) {
  const { zone, title, ids, columns, slots, max, format, getCard, invalid, fresh, info, onRemove, onOpen, onHover, onHoverEnd, emptyHint, dashed } = props;
  const drag = useDrag();
  const over = drag?.over === zone;
  const target = drag && drag.source.from !== zone;
  const filled = Math.max(slots, Math.ceil(ids.length / columns) * columns);
  const empty = Math.max(0, Math.min(filled, max) - ids.length);
  // Keyed by card and copy number, not position, so inserting a card does
  // not remount (and reload the image of) every tile after it.
  const seen = new Map<number, number>();
  const keys = ids.map((id) => {
    const n = (seen.get(id) ?? 0) + 1;
    seen.set(id, n);
    return `${id}-${n}`;
  });

  return (
    <section className="flex flex-col gap-2">
      <header className="flex items-baseline gap-3">
        <h2 className="display text-sm font-bold tracking-[0.12em] uppercase">{title}</h2>
        <span className={`text-sm font-semibold tabular-nums ${ids.length > max ? "text-danger" : "text-sec"}`}>{ids.length}</span>
        <span className="text-xs text-faint">/ {max}</span>
        <div className="ml-auto flex items-center gap-3 text-xs text-muted">{info}</div>
      </header>
      <div
        data-dropzone={zone}
        className={`relative grid rounded-xl border p-2 transition-all duration-150 ${dashed ? "border-dashed" : ""}`}
        style={{
          gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
          gap: columns > 10 ? 5 : 7,
          borderColor: over ? "var(--sec)" : target ? "color-mix(in oklab, var(--sec) 35%, var(--df-border))" : "var(--df-border)",
          background: over
            ? "radial-gradient(circle at 50% 0%, color-mix(in oklab, var(--sec) 16%, transparent), transparent 70%), var(--df-bg-2)"
            : "color-mix(in oklab, var(--df-bg-2) 80%, transparent)",
          boxShadow: over ? "0 0 30px -8px var(--sec), inset 0 0 20px -10px var(--sec)" : undefined,
        }}
      >
        {ids.map((id, index) => (
          <div key={keys[index]} data-deck-slot={`${zone}-${index}`}>
            <CardTile
              id={id}
              card={getCard(id)}
              compact
              invalid={invalid.has(id)}
              fresh={fresh?.zone === zone && fresh.index === index}
              badge={banBadge(getCard(id), format)}
              dragSource={{ id, from: zone, index }}
              onContext={() => onRemove(zone, index)}
              onOpen={onOpen}
              onHover={onHover}
              onHoverEnd={onHoverEnd}
            />
          </div>
        ))}
        {Array.from({ length: empty }, (_, i) => (
          <div key={`empty-${i}`} className="rounded-[4px] border border-dashed border-border" style={{ aspectRatio: "421 / 614", opacity: 0.55 }} />
        ))}
        {ids.length === 0 && emptyHint && <div className="pointer-events-none absolute inset-0 grid place-items-center p-6 text-center">{emptyHint}</div>}
      </div>
    </section>
  );
}
