import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { INFO_KINDS, Mascot, type InfoKind } from "./Mascot";

/**
 * A small "i" button that opens a short explanation next to it. The panel
 * is put in the page's body (a scrolling or animated column would cut it)
 * and closes on a click elsewhere, on Escape or when the page scrolls.
 * The mascot beside the title takes its face and icon from `kind`.
 */
export function InfoButton({
  title,
  children,
  className = "",
  kind = "what",
}: {
  title: string;
  children: ReactNode;
  className?: string;
  kind?: InfoKind;
}) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);

  useLayoutEffect(() => {
    if (!open || !button.current) return;
    const r = button.current.getBoundingClientRect();
    const width = 300;
    const left = Math.min(Math.max(8, r.right - width), window.innerWidth - width - 8);
    const height = panel.current?.offsetHeight ?? 0;
    // Below the button, or above it when there is no room.
    const below = r.bottom + 6;
    const top = below + height > window.innerHeight - 8 && r.top - 6 - height > 8 ? r.top - 6 - height : below;
    setPos({ top, left });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const close = (e: Event) => {
      if (e.type === "keydown" && (e as KeyboardEvent).key !== "Escape") return;
      if (e.type === "pointerdown" && (panel.current?.contains(e.target as Node) || button.current?.contains(e.target as Node))) return;
      setOpen(false);
    };
    const onScroll = (e: Event) => {
      if (panel.current?.contains(e.target as Node)) return;
      setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", close);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", close);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [open]);

  return (
    <>
      <button
        ref={button}
        type="button"
        className={`info-button ${className}`}
        aria-label={`Che cos'è: ${title}`}
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        i
      </button>
      {open &&
        createPortal(
          <div
            ref={panel}
            role="dialog"
            aria-label={title}
            className="info-panel anim-fade"
            style={pos ? { top: pos.top, left: pos.left } : { visibility: "hidden", top: 0, left: 0 }}
          >
            <div className="mb-2 flex items-center gap-2.5">
              <Mascot kind={kind} />
              <div className="min-w-0">
                <p className="eyebrow text-[9px]" style={{ color: INFO_KINDS[kind].color }}>
                  {INFO_KINDS[kind].label}
                </p>
                <p className="text-[13px] font-semibold leading-tight text-text">{title}</p>
              </div>
            </div>
            <div className="flex flex-col gap-1.5 text-xs leading-relaxed text-muted">{children}</div>
          </div>,
          document.body,
        )}
    </>
  );
}
