import { memo, useRef, useState, type PointerEvent } from "react";
import { cardImage, type Card } from "../lib/api";
import { pointerDownDraggable, type DragSource } from "../lib/drag";

export function rarityCode(card: Card | undefined): "n" | "r" | "sr" | "ur" | undefined {
  switch (card?.mdRarity) {
    case "Common":
      return "n";
    case "Rare":
      return "r";
    case "Super Rare":
      return "sr";
    case "Ultra Rare":
      return "ur";
    default:
      return undefined;
  }
}

export interface TileBadge {
  text: string;
  tone?: "ban" | "limit";
}

interface Props {
  id: number;
  card?: Card;
  selected?: boolean;
  onSelect?: (id: number, el: HTMLElement) => void;
  onOpen?: (id: number) => void;
  onHover?: (id: number, el: HTMLElement) => void;
  onHoverEnd?: () => void;
  /** Right click; `shift` when Shift is held. */
  onContext?: (id: number, el: HTMLElement, shift: boolean) => void;
  /** Makes the tile draggable. */
  dragSource?: DragSource;
  /** Small label in the top-left corner (copies, Forbidden/Limited status). */
  badge?: TileBadge;
  /** Red outline: the card breaks a deck rule. */
  invalid?: boolean;
  /** Plays the "just forged" drop animation. */
  fresh?: boolean;
  /** Smaller tiles get smaller decorations. */
  compact?: boolean;
}

/**
 * A card in a grid. Follows the pointer with a 3D tilt and a glare; Super
 * and Ultra Rare cards (Master Duel rarity) get a silver or rainbow foil.
 * Pointer tracking writes CSS variables directly, without re-rendering.
 */
export const CardTile = memo(function CardTile(props: Props) {
  const { id, card, selected, onSelect, onOpen, onHover, onHoverEnd, onContext, dragSource, badge, invalid, fresh, compact } = props;
  const ref = useRef<HTMLButtonElement>(null);
  const [loaded, setLoaded] = useState(false);
  const rarity = rarityCode(card);

  const move = (e: PointerEvent<HTMLButtonElement>) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width;
    const py = (e.clientY - r.top) / r.height;
    el.dataset.tilting = "true";
    el.style.setProperty("--ry", `${(px - 0.5) * 16}deg`);
    el.style.setProperty("--rx", `${(0.5 - py) * 16}deg`);
    el.style.setProperty("--mx", `${px * 100}%`);
    el.style.setProperty("--my", `${py * 100}%`);
  };

  const leave = () => {
    const el = ref.current;
    if (!el) return;
    el.dataset.tilting = "false";
    el.style.setProperty("--rx", "0deg");
    el.style.setProperty("--ry", "0deg");
    onHoverEnd?.();
  };

  return (
    <button
      ref={ref}
      type="button"
      className={`card-tile${fresh ? " card-drop" : ""}`}
      data-rarity={rarity}
      data-invalid={invalid || undefined}
      data-compact={compact || undefined}
      aria-pressed={selected}
      aria-label={card?.name ?? "Carta"}
      onPointerMove={move}
      onPointerEnter={() => ref.current && onHover?.(id, ref.current)}
      onPointerLeave={leave}
      onClick={() => ref.current && onSelect?.(id, ref.current)}
      onDoubleClick={() => onOpen?.(id)}
      onPointerDown={dragSource ? (e) => pointerDownDraggable(e, dragSource) : undefined}
      onContextMenu={
        onContext
          ? (e) => {
              e.preventDefault();
              if (ref.current) onContext(id, ref.current, e.shiftKey);
            }
          : undefined
      }
    >
      <span className="card-tile__clip">
        {!loaded && <span className="card-tile__shimmer" />}
        <img
          className="card-tile__img"
          data-loaded={loaded}
          src={cardImage(id, "small")}
          alt=""
          draggable={false}
          loading="lazy"
          decoding="async"
          onLoad={() => setLoaded(true)}
        />
        <span className="card-tile__foil" />
        <span className="card-tile__glare" />
      </span>
      {rarity && !compact && (
        <span className="rarity-gem" data-rarity={rarity}>
          {rarity.toUpperCase()}
        </span>
      )}
      {badge && (
        <span className="tile-badge" data-tone={badge.tone}>
          {badge.text}
        </span>
      )}
    </button>
  );
});
