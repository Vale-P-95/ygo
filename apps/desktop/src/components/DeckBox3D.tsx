import { useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
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
 * a lid leaning back, visible metallic hinge and showcase cards standing inside.
 * Dynamic lighting tracks cursor coordinates for specular highlights and shadows.
 */
export function DeckBox3D({ look, name, format, legal, errors, cards, scale = 1, interactive, selected, sway, onClick, onHover }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const extractingTimer = useRef<number | null>(null);
  const [extracting, setExtracting] = useState(false);
  const size = SIZES[look.shape];
  const shown = cards.slice(0, 3);

  useEffect(() => {
    return () => {
      if (extractingTimer.current != null) {
        window.clearTimeout(extractingTimer.current);
      }
    };
  }, []);

  const handleClick = () => {
    if (!onClick) return;
    if (extracting) return;
    const reduced = typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) {
      onClick();
      return;
    }
    setExtracting(true);
    extractingTimer.current = window.setTimeout(() => {
      setExtracting(false);
      onClick();
    }, 200);
  };

  const move = (e: PointerEvent<HTMLDivElement>) => {
    const el = ref.current;
    if (!el || !interactive) return;
    const r = el.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width - 0.5;
    const py = (e.clientY - r.top) / r.height - 0.5;
    el.style.setProperty("--tilt-y", `${px * 26}deg`);
    el.style.setProperty("--tilt-x", `${-py * 12}deg`);
    el.style.setProperty("--light-x", `${Math.round((px + 0.5) * 100)}%`);
    el.style.setProperty("--light-y", `${Math.round((py + 0.5) * 100)}%`);
    el.style.setProperty("--spec-op", "0.55");
  };

  const leave = () => {
    const el = ref.current;
    if (!el) return;
    el.style.setProperty("--tilt-y", "0deg");
    el.style.setProperty("--tilt-x", "0deg");
    el.style.setProperty("--light-x", "50%");
    el.style.setProperty("--light-y", "30%");
    el.style.setProperty("--spec-op", "0.22");
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
      onKeyDown={onClick ? (e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), handleClick()) : undefined}
      className="dbox-stage"
      data-interactive={interactive || undefined}
      data-selected={selected || undefined}
      data-extracting={extracting || undefined}
      data-sway={sway || undefined}
      style={style}
      onClick={onClick ? handleClick : undefined}
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
        <div className="dbox-face dbox-back">
          <div className="dbox-specular" />
        </div>
        <div className="dbox-face dbox-left">
          <div className="dbox-specular" />
        </div>
        <div className="dbox-face dbox-right">
          <div className="dbox-specular" />
        </div>
        <div className="dbox-face dbox-bottom" />
        <div className="dbox-face dbox-lid">
          <div className="dbox-specular" />
        </div>
        <div className="dbox-hinge" aria-hidden="true" />
        <div className="dbox-face dbox-front">
          <div className="dbox-specular" />
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
