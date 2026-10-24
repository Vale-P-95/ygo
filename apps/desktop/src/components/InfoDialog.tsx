import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Mascot, type InfoKind } from "./Mascot";

/**
 * The Archmage's explanation: a centered dialog with his board on the left
 * and the text on the right. Shared by every «i» and «?» of the app; closes
 * on Escape, on a click outside or on «Ho capito».
 */
export function InfoDialog({ kind, title, onClose, children }: { kind: InfoKind; title: string; onClose: () => void; children: ReactNode }) {
  // Callers pass inline functions: keep the latest without re-running the effect.
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && close.current();
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, []);
  return createPortal(
    <div
      className="fixed inset-0 z-[60] grid place-items-center p-4 anim-fade"
      style={{ background: "color-mix(in oklab, var(--df-bg) 70%, transparent)", backdropFilter: "blur(3px)" }}
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
        className="panel mascot-dialog grid max-h-full w-full max-w-[640px] items-center gap-5 overflow-y-auto py-6 pl-4 pr-6 shadow-2xl"
      >
        <div className="mascot-pop mx-auto">
          <Mascot kind={kind} />
        </div>
        <div className="flex min-w-0 flex-col gap-2.5">
          <span className="eyebrow">L'Arcimago spiega</span>
          <h3 className="text-title-lg display font-bold">{title}</h3>
          <div className="flex flex-col gap-2 text-sm leading-relaxed text-muted [&>p:first-child]:text-text">{children}</div>
          <div className="mt-1.5 flex justify-end">
            <button type="button" className="btn btn-molten" autoFocus onClick={onClose}>
              Ho capito
            </button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
