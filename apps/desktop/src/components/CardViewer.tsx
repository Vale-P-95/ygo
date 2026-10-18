import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { createPortal } from "react-dom";
import { cardImage, type Card } from "../lib/api";
import { sound } from "../lib/sound";
import { CardDetails } from "./CardDetails";
import { rarityCode } from "./CardTile";

const MAX_SCALE = 5;
const LENS_SIZE = 180;
const LENS_ZOOM = 2.6;

interface Props {
  card: Card;
  /** "3 / 120" — position in the current results. */
  position?: string;
  onClose: () => void;
  onPrev?: () => void;
  onNext?: () => void;
  onSearch?: (query: string) => void;
}

/**
 * Full-size card viewer: wheel or +/- to zoom around the pointer, drag to
 * pan, double click to zoom in/out, L for a magnifying lens, arrows to move
 * through the results, Esc to close.
 */
export function CardViewer({ card, position, onClose, onPrev, onNext, onSearch }: Props) {
  const [art, setArt] = useState(0);
  const [scale, setScale] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [lens, setLens] = useState(false);
  const [lensAt, setLensAt] = useState<{ x: number; y: number } | null>(null);
  const [loaded, setLoaded] = useState(false);
  const frameRef = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; ox: number; oy: number } | null>(null);
  const imageId = card.imageIds[art] ?? card.id;
  const rarity = rarityCode(card);

  useEffect(() => {
    sound.play("open");
  }, []);

  useEffect(() => {
    setArt(0);
    setScale(1);
    setOffset({ x: 0, y: 0 });
  }, [card.id]);

  useEffect(() => setLoaded(false), [imageId]);

  const close = useCallback(() => {
    sound.play("close");
    onClose();
  }, [onClose]);

  const zoomAt = useCallback(
    (factor: number, clientX?: number, clientY?: number) => {
      const frame = frameRef.current;
      if (!frame) return;
      const r = frame.getBoundingClientRect();
      const px = (clientX ?? r.left + r.width / 2) - (r.left + r.width / 2);
      const py = (clientY ?? r.top + r.height / 2) - (r.top + r.height / 2);
      setScale((s) => {
        const next = Math.min(MAX_SCALE, Math.max(1, s * factor));
        const k = next / s;
        setOffset((o) => (next === 1 ? { x: 0, y: 0 } : { x: px - (px - o.x) * k, y: py - (py - o.y) * k }));
        return next;
      });
    },
    [],
  );

  useEffect(() => {
    const frame = frameRef.current;
    if (!frame) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      zoomAt(Math.pow(1.0018, -e.deltaY), e.clientX, e.clientY);
    };
    frame.addEventListener("wheel", wheel, { passive: false });
    return () => frame.removeEventListener("wheel", wheel);
  }, [zoomAt]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
      else if (e.key === "ArrowLeft" && onPrev) {
        sound.play("page");
        onPrev();
      } else if (e.key === "ArrowRight" && onNext) {
        sound.play("page");
        onNext();
      } else if (e.key.toLowerCase() === "l") {
        sound.play("toggle");
        setLens((v) => !v);
      } else if (e.key === "+" || e.key === "=") zoomAt(1.25);
      else if (e.key === "-") zoomAt(0.8);
      else if (e.key === "0") {
        setScale(1);
        setOffset({ x: 0, y: 0 });
      }
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [close, onPrev, onNext, zoomAt]);

  const pointerDown = (e: ReactPointerEvent) => {
    if (scale === 1) return;
    (e.target as Element).setPointerCapture?.(e.pointerId);
    drag.current = { x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y };
  };
  const pointerMove = (e: ReactPointerEvent) => {
    const r = frameRef.current?.getBoundingClientRect();
    if (r && lens && scale === 1) setLensAt({ x: e.clientX - r.left, y: e.clientY - r.top });
    if (drag.current) setOffset({ x: drag.current.ox + e.clientX - drag.current.x, y: drag.current.oy + e.clientY - drag.current.y });
  };
  const frameRect = frameRef.current?.getBoundingClientRect();

  // Rendered at the end of <body>: above every stacking context of the page.
  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6 anim-fade" role="dialog" aria-modal="true" aria-label={card.name}>
      <div className="absolute inset-0 bg-black/75 backdrop-blur-sm" onClick={close} />
      <div className="panel relative grid max-h-full w-full max-w-6xl grid-cols-[auto_minmax(320px,1fr)] gap-6 overflow-hidden p-6 anim-pop">
        <div className="flex flex-col items-center gap-3">
          <div
            ref={frameRef}
            className="relative overflow-hidden rounded-lg"
            style={{
              height: "min(74vh, 660px)",
              aspectRatio: "421 / 614",
              cursor: lens && scale === 1 ? "none" : scale > 1 ? (drag.current ? "grabbing" : "grab") : "zoom-in",
              boxShadow: rarity === "ur" ? "0 0 40px -6px rgb(255 190 120 / 0.6)" : "0 20px 50px -20px black",
            }}
            onPointerDown={pointerDown}
            onPointerMove={pointerMove}
            onPointerUp={() => (drag.current = null)}
            onPointerLeave={() => {
              drag.current = null;
              setLensAt(null);
            }}
            onDoubleClick={(e) => zoomAt(scale > 1 ? 1 / scale : 2.5, e.clientX, e.clientY)}
          >
            {!loaded && <div className="card-tile__shimmer absolute inset-0 rounded-lg bg-surface-2" />}
            <img
              src={cardImage(imageId, "full")}
              alt={card.name}
              draggable={false}
              onLoad={() => setLoaded(true)}
              className="h-full w-full select-none"
              style={{
                transform: `translate(${offset.x}px, ${offset.y}px) scale(${scale})`,
                transition: drag.current ? "none" : "transform 140ms ease-out",
                opacity: loaded ? 1 : 0,
              }}
            />
            {lens && lensAt && scale === 1 && frameRect && (
              <div
                className="pointer-events-none absolute rounded-full border-2"
                style={{
                  width: LENS_SIZE,
                  height: LENS_SIZE,
                  left: lensAt.x - LENS_SIZE / 2,
                  top: lensAt.y - LENS_SIZE / 2,
                  borderColor: "var(--sec)",
                  boxShadow: "0 0 0 4px rgb(0 0 0 / 0.35), 0 12px 30px rgb(0 0 0 / 0.6), 0 0 24px color-mix(in oklab, var(--sec) 60%, transparent)",
                  backgroundImage: `url("${cardImage(imageId, "full")}")`,
                  backgroundRepeat: "no-repeat",
                  backgroundSize: `${frameRect.width * LENS_ZOOM}px ${frameRect.height * LENS_ZOOM}px`,
                  backgroundPosition: `${-lensAt.x * LENS_ZOOM + LENS_SIZE / 2}px ${-lensAt.y * LENS_ZOOM + LENS_SIZE / 2}px`,
                }}
              />
            )}
          </div>
          <div className="flex items-center gap-2 text-xs">
            <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={() => zoomAt(0.8)} aria-label="Riduci">
              −
            </button>
            <span className="w-12 text-center tabular-nums text-muted">{Math.round(scale * 100)}%</span>
            <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={() => zoomAt(1.25)} aria-label="Ingrandisci">
              +
            </button>
            <button
              type="button"
              className={`btn !px-3 !py-1 ${lens ? "btn-molten" : "btn-ghost"}`}
              onClick={() => {
                sound.play("toggle");
                setLens((v) => !v);
              }}
            >
              Lente (L)
            </button>
            {card.imageIds.length > 1 && (
              <div className="ml-2 flex gap-1">
                {card.imageIds.map((id, i) => (
                  <button
                    key={id}
                    type="button"
                    onClick={() => {
                      sound.play("select");
                      setArt(i);
                    }}
                    className={`h-9 w-6 overflow-hidden rounded border ${i === art ? "border-sec" : "border-border opacity-60 hover:opacity-100"}`}
                    aria-label={`Artwork ${i + 1}`}
                  >
                    <img src={cardImage(id, "small")} alt="" className="h-full w-full object-cover" />
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="flex min-h-0 flex-col">
          <div className="mb-3 flex items-center justify-between">
            <span className="eyebrow">{position ?? ""}</span>
            <div className="flex gap-1">
              {onPrev && (
                <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={onPrev} aria-label="Carta precedente">
                  ←
                </button>
              )}
              {onNext && (
                <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={onNext} aria-label="Carta successiva">
                  →
                </button>
              )}
              <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={close} aria-label="Chiudi">
                ✕
              </button>
            </div>
          </div>
          <div className="min-h-0 overflow-y-auto pr-2">
            <CardDetails card={card} onSearch={onSearch} />
          </div>
          <p className="mt-3 text-[11px] text-faint">Rotella o +/− per lo zoom · trascina per spostarti · doppio clic · L per la lente · ←/→ · Esc</p>
        </div>
      </div>
    </div>,
    document.body,
  );
}
