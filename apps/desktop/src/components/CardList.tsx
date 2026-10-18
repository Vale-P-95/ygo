import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef } from "react";
import { cardImage, type Card } from "../lib/api";
import { pointerDownDraggable, type DropZone } from "../lib/drag";
import { ATTRIBUTE_COLORS, frameColor } from "./CardDetails";
import { rarityCode, type TileBadge } from "./CardTile";

const ROW = 62;
const GAP = 4;

interface Props {
  ids: number[];
  getCard: (id: number) => Card | undefined;
  requestCards: (ids: number[]) => void;
  /** Bumps when new card data arrives, to re-render visible rows. */
  version: number;
  onAdd: (id: number, el: HTMLElement, toSide: boolean) => void;
  onOpen: (id: number) => void;
  onHover: (id: number, el: HTMLElement) => void;
  onHoverEnd: () => void;
  dragFrom: DropZone;
  /** Labels on the right of a row (copies in the deck, list status). */
  badgesFor?: (id: number) => TileBadge[];
}

function subtitle(card: Card): string {
  if (card.kind !== "monster") return card.humanType;
  const parts = [card.typeLine.join("/")];
  if (card.linkRating != null) parts.unshift(`LINK-${card.linkRating}`);
  else if (card.level != null) parts.unshift(`${card.typeLine.includes("Xyz") ? "☆" : "★"}${card.level}`);
  return parts.join(" · ");
}

/** Virtualized list of cards: thumbnail, name, type and stats on one row. */
export function CardList({ ids, getCard, requestCards, version, onAdd, onOpen, onHover, onHoverEnd, dragFrom, badgesFor }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: ids.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW + GAP,
    overscan: 6,
  });

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [ids]);

  const items = virtualizer.getVirtualItems();
  const first = items[0]?.index ?? 0;
  const last = items[items.length - 1]?.index ?? -1;
  useEffect(() => {
    if (last >= first) requestCards(ids.slice(first, last + 1));
  }, [first, last, ids, requestCards]);

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto px-2 pt-2 pb-8" onScroll={onHoverEnd}>
      <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }} data-version={version}>
        {items.map((item) => {
          const id = ids[item.index];
          const card = getCard(id);
          const rarity = rarityCode(card);
          return (
            <div
              key={id}
              role="button"
              tabIndex={0}
              aria-label={card?.name ?? "Carta"}
              className="group absolute left-0 flex w-full cursor-grab items-center gap-2.5 overflow-hidden rounded-lg border border-transparent bg-surface-2/60 py-1 pr-2 pl-1 hover:border-border-strong hover:bg-surface-3"
              style={{ height: ROW, transform: `translateY(${item.start}px)` }}
              onPointerEnter={(e) => onHover(id, e.currentTarget)}
              onPointerLeave={onHoverEnd}
              onPointerDown={(e) => pointerDownDraggable(e, { id, from: dragFrom })}
              onDoubleClick={() => onOpen(id)}
              onKeyDown={(e) => e.key === "Enter" && onAdd(id, e.currentTarget, e.shiftKey)}
              onContextMenu={(e) => {
                e.preventDefault();
                onAdd(id, e.currentTarget, e.shiftKey);
              }}
            >
              <span className="h-full w-[3px] shrink-0 rounded-full" style={{ background: card ? frameColor(card.frame) : "var(--df-border)" }} />
              <span className="relative h-[54px] w-[37px] shrink-0 overflow-hidden rounded-[3px] bg-surface-3">
                <img src={cardImage(id, "small")} alt="" draggable={false} loading="lazy" decoding="async" className="h-full w-full object-cover" />
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="flex items-center gap-1.5">
                  <span className="truncate text-[13px] leading-tight font-semibold">{card?.name ?? "…"}</span>
                  {rarity && (
                    <span className="rarity-gem !static shrink-0 !text-[8px] !leading-[12px]" data-rarity={rarity}>
                      {rarity.toUpperCase()}
                    </span>
                  )}
                </span>
                {card && (
                  <span className="truncate text-[11px] text-muted">
                    {card.attribute && <span style={{ color: ATTRIBUTE_COLORS[card.attribute] }}>{card.attribute} · </span>}
                    {subtitle(card)}
                  </span>
                )}
                {card?.kind === "monster" && (
                  <span className="text-[11px] font-semibold text-text/80 tabular-nums">
                    ATK {card.atk ?? "?"}
                    {card.linkRating == null && ` / DEF ${card.def ?? "?"}`}
                  </span>
                )}
              </span>
              <span className="flex shrink-0 items-center gap-1">
                {badgesFor?.(id).map((b) => (
                  <span key={b.text} className="tile-badge !static" data-tone={b.tone}>
                    {b.text}
                  </span>
                ))}
              </span>
              <button
                type="button"
                className="btn btn-ghost shrink-0 !px-2 !py-0.5 text-base opacity-0 group-hover:opacity-100 focus:opacity-100"
                title="Aggiungi al deck (Maiusc: Side)"
                aria-label="Aggiungi al deck"
                onPointerDown={(e) => e.stopPropagation()}
                onClick={(e) => onAdd(id, e.currentTarget, e.shiftKey)}
              >
                +
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
