import { useRef, type CSSProperties, type PointerEvent } from "react";
import { cardImage, type DeckFormat } from "../lib/api";
import { SIZES, type Look } from "../lib/boxes";
import { FORMAT_LABELS } from "../lib/deck";

interface Props {
  look: Look;
  name: string;
  format: DeckFormat;
  /** Legality under the current lists (`null` when unknown). */
  legal: boolean | null;
  errors: number;
  /** Showcase cards (up to three) peeking out of the box. */
  cards: number[];
  scale?: number;
  /** Follows the pointer and lifts the cards on hover. */
  interactive?: boolean;
  selected?: boolean;
  /** Slow sway, for the editor's preview. */
  sway?: boolean;
  onClick?: () => void;
  onHover?: () => void;
}

/**
 * A deck box drawn with CSS 3D transforms: five walls (the top is open),
 * a lid leaning back and the showcase cards standing inside. Color,
 * material and shape come from the deck's `box` settings.
 */
export function DeckBox3D({ look, name, format, legal, errors, cards, scale = 1, interactive, selected, sway, onClick, onHover }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const size = SIZES[look.shape];
  const shown = cards.slice(0, 3);

  const move = (e: PointerEvent<HTMLDivElement>) => {
    const el = ref.current;
    if (!el || !interactive) return;
    const r = el.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width - 0.5;
    const py = (e.clientY - r.top) / r.height - 0.5;
    el.style.setProperty("--tilt-y", `${px * 26}deg`);
    el.style.setProperty("--tilt-x", `${-py * 12}deg`);
  };
  const leave = () => {
    ref.current?.style.setProperty("--tilt-y", "0deg");
    ref.current?.style.setProperty("--tilt-x", "0deg");
  };

  const style = {
    "--w": `${size.w}px`,
    "--h": `${size.h}px`,
    "--d": `${size.d}px`,
    "--c": look.color,
    "--s": scale,
    width: (size.w + 70) * scale,
    height: (size.h + size.w * 0.75) * scale,
  } as CSSProperties;

  return (
    <div
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={onClick ? (e) => (e.key === "Enter" || e.key === " ") && onClick() : undefined}
      className="dbox-stage"
      data-interactive={interactive || undefined}
      data-selected={selected || undefined}
      data-sway={sway || undefined}
      style={style}
      onClick={onClick}
      onPointerEnter={onHover}
      onPointerMove={move}
      onPointerLeave={leave}
      aria-label={onClick ? `Deck ${name}` : undefined}
    >
      <div className="dbox-shadow" />
      <div ref={ref} className="dbox" data-shape={look.shape} data-material={look.material}>
        <div className="dbox-cards">
          {shown.map((id, i) => (
            <img
              key={`${id}-${i}`}
              className="dbox-card"
              src={cardImage(id, "small")}
              alt=""
              draggable={false}
              loading="lazy"
              style={{ "--i": i - (shown.length - 1) / 2 } as CSSProperties}
            />
          ))}
        </div>
        <div className="dbox-face dbox-back" />
        <div className="dbox-face dbox-left" />
        <div className="dbox-face dbox-right" />
        <div className="dbox-face dbox-bottom" />
        <div className="dbox-face dbox-lid" />
        <div className="dbox-face dbox-front">
          {look.shape === "chest" && <span className="dbox-clasp" />}
          <div className="dbox-plate">
            <span className="dbox-name" data-long={name.length > 30 ? "2" : name.length > 18 ? "1" : undefined}>
              {name}
            </span>
            <span className="dbox-format">{FORMAT_LABELS[format]}</span>
          </div>
          {legal != null && (
            <span className="dbox-seal" data-ok={legal || undefined} title={legal ? "Legale" : `${errors} problemi`}>
              {legal ? "✓" : `✕${errors > 1 ? errors : ""}`}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
