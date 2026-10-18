import { createPortal } from "react-dom";
import { cardImage, type Card } from "../lib/api";
import { ATTRIBUTE_COLORS, frameColor } from "./CardDetails";

const WIDTH = 520;

/** Floating preview shown next to a hovered card. */
export function CardPreview({ card, anchor }: { card: Card; anchor: DOMRect }) {
  const placeRight = anchor.right + 16 + WIDTH < innerWidth;
  const left = placeRight ? anchor.right + 16 : Math.max(12, anchor.left - 16 - WIDTH);
  const top = Math.min(Math.max(12, anchor.top + anchor.height / 2 - 170), innerHeight - 352);
  const stats =
    card.kind === "monster"
      ? `ATK ${card.atk ?? "?"}${card.linkRating != null ? ` · LINK-${card.linkRating}` : ` · DEF ${card.def ?? "?"}`}`
      : card.humanType;
  return createPortal(
    <div className="panel pointer-events-none fixed z-40 flex gap-4 p-3 anim-pop" style={{ left, top, width: WIDTH }} role="tooltip">
      <img src={cardImage(card.id, "full")} alt="" className="h-[310px] w-auto rounded-md shadow-lg" />
      <div className="flex min-w-0 flex-col gap-1.5">
        <div className="h-1 w-12 rounded-full" style={{ background: frameColor(card.frame) }} />
        <p className="display text-base leading-tight font-bold">{card.name}</p>
        {card.kind === "monster" && (
          <p className="text-xs text-muted">
            {card.attribute && <span style={{ color: ATTRIBUTE_COLORS[card.attribute] }}>{card.attribute} · </span>}
            {card.typeLine.join(" / ")}
            {card.level != null && ` · ★${card.level}`}
          </p>
        )}
        <p className="text-xs font-semibold tabular-nums">{stats}</p>
        <p className="line-clamp-[11] text-xs leading-relaxed whitespace-pre-line text-text/85">{card.desc}</p>
      </div>
    </div>,
    document.body,
  );
}
