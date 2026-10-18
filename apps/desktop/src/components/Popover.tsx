import { useEffect, useRef, type ReactNode } from "react";

/** Closes a popup on a click outside `ref` or on Escape. */
export function useOutsideClose(open: boolean, close: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && close();
    addEventListener("pointerdown", down);
    addEventListener("keydown", key);
    return () => {
      removeEventListener("pointerdown", down);
      removeEventListener("keydown", key);
    };
  }, [open, close]);
  return ref;
}

export function Popover({ open, children, align = "left", width = 340 }: { open: boolean; children: ReactNode; align?: "left" | "right"; width?: number }) {
  if (!open) return null;
  return (
    <div className={`panel absolute top-full z-30 mt-2 max-h-[70vh] overflow-y-auto p-2 anim-pop ${align === "right" ? "right-0" : "left-0"}`} style={{ width }}>
      {children}
    </div>
  );
}
