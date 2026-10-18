import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef, useState } from "react";
import type { Card } from "../lib/api";
import { CardTile } from "./CardTile";

const MIN_TILE = 128;
const GAP = 14;

interface Props {
  ids: number[];
  getCard: (id: number) => Card | undefined;
  requestCards: (ids: number[]) => void;
  /** Bumps when new card data arrives, to re-render visible tiles. */
  version: number;
  selected: number | null;
  onSelect: (id: number, el: HTMLElement) => void;
  onOpen: (id: number) => void;
  onHover: (id: number, el: HTMLElement) => void;
  onHoverEnd: () => void;
}

/** Virtualized grid: only the visible rows exist in the page. */
export function CardGrid({ ids, getCard, requestCards, version, selected, onSelect, onOpen, onHover, onHoverEnd }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setWidth(el.clientWidth - 16));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const columns = Math.max(1, Math.floor((width + GAP) / (MIN_TILE + GAP)));
  const tile = width > 0 ? (width - GAP * (columns - 1)) / columns : MIN_TILE;
  const rowHeight = tile * (614 / 421) + GAP;
  const rows = Math.ceil(ids.length / columns);

  const virtualizer = useVirtualizer({
    count: rows,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 3,
  });

  useEffect(() => virtualizer.measure(), [rowHeight, virtualizer]);
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [ids]);

  const items = virtualizer.getVirtualItems();
  const first = items[0]?.index ?? 0;
  const last = items[items.length - 1]?.index ?? -1;
  useEffect(() => {
    if (last >= first) requestCards(ids.slice(first * columns, (last + 1) * columns));
  }, [first, last, columns, ids, requestCards]);

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto px-2 pt-2 pb-8" onScroll={onHoverEnd}>
      <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }} data-version={version}>
        {items.map((row) => (
          <div
            key={row.key}
            className="absolute left-0 grid w-full"
            style={{ transform: `translateY(${row.start}px)`, gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, gap: GAP }}
          >
            {ids.slice(row.index * columns, row.index * columns + columns).map((id) => (
              <CardTile
                key={id}
                id={id}
                card={getCard(id)}
                selected={selected === id}
                onSelect={onSelect}
                onOpen={onOpen}
                onHover={onHover}
                onHoverEnd={onHoverEnd}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
