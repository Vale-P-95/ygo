import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef, useState } from "react";
import type { Card } from "../lib/api";
import type { DropZone } from "../lib/drag";
import { CardTile, type TileBadge } from "./CardTile";

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
  onContext?: (id: number, el: HTMLElement, shift: boolean) => void;
  /** Tiles can be dragged out of this grid (as coming from `dragFrom`). */
  dragFrom?: DropZone;
  badgeFor?: (id: number) => TileBadge | undefined;
  minTile?: number;
}

/** Virtualized grid: only the visible rows exist in the page. */
export function CardGrid(props: Props) {
  const { ids, getCard, requestCards, version, selected, onSelect, onOpen, onHover, onHoverEnd, onContext, dragFrom, badgeFor, minTile = 128 } = props;
  const gap = minTile < 70 ? 6 : minTile < 110 ? 8 : GAP;
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setWidth(el.clientWidth - 16));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const columns = Math.max(1, Math.floor((width + gap) / (minTile + gap)));
  const tile = width > 0 ? (width - gap * (columns - 1)) / columns : minTile;
  const rowHeight = tile * (614 / 421) + gap;
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
            style={{ transform: `translateY(${row.start}px)`, gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, gap }}
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
                onContext={onContext}
                dragSource={dragFrom ? { id, from: dragFrom } : undefined}
                badge={badgeFor?.(id)}
                compact={minTile < 90}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
