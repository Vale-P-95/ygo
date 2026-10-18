import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { cardImage } from "../lib/api";
import { useDrag } from "../lib/drag";
import { useToasts } from "../lib/toast";
import { hideTip, showTip, useTip } from "../lib/tooltip";

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

/**
 * Chart tooltip: follows the pointer over any element with `data-tip`
 * (keyboard focus shows it too), or wherever a chart's crosshair puts it.
 */
export function ChartTooltip() {
  const tip = useTip();
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: 0, top: 0 });

  useEffect(() => {
    const move = (e: PointerEvent) => {
      const el = (e.target as Element | null)?.closest?.("[data-tip]") as HTMLElement | SVGElement | null;
      if (el?.dataset.tip) showTip(el.dataset.tip, e.clientX, e.clientY);
      else if (!(e.target as Element | null)?.closest?.("[data-crosshair]")) hideTip();
    };
    const focus = (e: FocusEvent) => {
      const el = (e.target as Element | null)?.closest?.("[data-tip]") as HTMLElement | SVGElement | null;
      if (!el?.dataset.tip) return;
      const r = el.getBoundingClientRect();
      showTip(el.dataset.tip, r.right, r.top);
    };
    addEventListener("pointermove", move);
    addEventListener("focusin", focus);
    addEventListener("focusout", hideTip);
    addEventListener("scroll", hideTip, true);
    return () => {
      removeEventListener("pointermove", move);
      removeEventListener("focusin", focus);
      removeEventListener("focusout", hideTip);
      removeEventListener("scroll", hideTip, true);
    };
  }, []);

  useLayoutEffect(() => {
    if (!tip || !ref.current) return;
    const r = ref.current.getBoundingClientRect();
    setPos({ left: Math.min(innerWidth - r.width - 8, tip.x + 14), top: Math.max(8, tip.y - r.height - 12) });
  }, [tip]);

  if (!tip) return null;
  return createPortal(
    <div
      ref={ref}
      role="tooltip"
      className="panel pointer-events-none fixed z-[75] max-w-64 px-3 py-2 text-xs leading-relaxed whitespace-pre-line tabular-nums"
      style={{ left: pos.left, top: pos.top }}
    >
      {tip.text}
    </div>,
    document.body,
  );
}
