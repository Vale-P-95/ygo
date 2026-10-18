import { createPortal } from "react-dom";
import { cardImage } from "../lib/api";
import { useDrag } from "../lib/drag";
import { useToasts } from "../lib/toast";

/** The card following the pointer while dragging. */
export function DragLayer() {
  const drag = useDrag();
  if (!drag) return null;
  return createPortal(
    <img
      src={cardImage(drag.source.id, "small")}
      alt=""
      className="pointer-events-none fixed z-[70] w-[92px] rounded-md"
      style={{
        left: drag.x - 46,
        top: drag.y - 67,
        transform: `rotate(${drag.over ? -3 : -8}deg) scale(${drag.over ? 1.08 : 1})`,
        transition: "transform 120ms ease",
        boxShadow: drag.over
          ? "0 0 0 2px var(--df-gold), 0 0 34px -2px var(--df-ember), 0 20px 30px -10px black"
          : "0 18px 30px -10px black, 0 0 20px -6px var(--df-gold)",
        filter: drag.over === "catalog" ? "grayscale(0.8) brightness(0.7)" : undefined,
      }}
    />,
    document.body,
  );
}

const TONES = {
  info: "border-border-strong",
  ok: "border-ok/60",
  error: "border-danger/70",
};

export function Toasts() {
  const toasts = useToasts();
  return createPortal(
    <div className="pointer-events-none fixed bottom-6 left-1/2 z-[80] flex -translate-x-1/2 flex-col items-center gap-2">
      {toasts.map((t) => (
        <div key={t.id} className={`panel anim-pop px-4 py-2.5 text-sm ${TONES[t.tone]}`} role="status">
          {t.tone === "error" ? "⚠ " : t.tone === "ok" ? "✓ " : ""}
          {t.message}
        </div>
      ))}
    </div>,
    document.body,
  );
}
