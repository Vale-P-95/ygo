import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { sound } from "../lib/sound";

/** Centered dialog with a title; closes on Escape or a click outside. */
export function Modal({ title, onClose, children, wide }: { title: string; onClose: () => void; children: ReactNode; wide?: boolean }) {
  // Callers pass inline functions: keep the latest without re-running the effect.
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    sound.play("open");
    const key = (e: KeyboardEvent) => e.key === "Escape" && close.current();
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, []);
  return createPortal(
    <div className="fixed inset-0 z-50 grid place-items-center p-6 anim-fade" role="dialog" aria-modal="true" aria-label={title}>
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={onClose} />
      <div className={`panel relative flex max-h-full w-full flex-col gap-4 p-6 anim-pop ${wide ? "max-w-6xl" : "max-w-2xl"}`}>
        <div className="flex items-center justify-between">
          <h2 className="display text-xl font-bold">{title}</h2>
          <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={onClose} aria-label="Chiudi">
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>,
    document.body,
  );
}
