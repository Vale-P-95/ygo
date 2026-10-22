import { useEffect, useRef, useState, type ReactNode } from "react";
import { HelpCircle, X } from "lucide-react";
import { IconButton } from "./IconButton";

export interface HelpPopoverProps {
  /** Optional header title for the popover card. */
  title?: string;
  /** Content (tips, shortcuts, instructions) replacing fixed screen text. */
  children: ReactNode;
  /** Tooltip for the trigger button. */
  tooltip?: string;
  /** Accessible label for the trigger button. */
  ariaLabel?: string;
  /** Alignment of the popover dialog relative to the trigger button. */
  align?: "left" | "right";
  /** Optional custom CSS classes for the trigger wrapper. */
  className?: string;
}

/**
 * Accessible «?» help button with an interactive popover dialog.
 * Principle: replaces fixed instructional text on screen with on-demand guidance.
 */
export function HelpPopover({
  title = "Suggerimenti e guida",
  children,
  tooltip = "Guida e suggerimenti",
  ariaLabel = "Mostra suggerimenti e guida",
  align = "right",
  className = "",
}: HelpPopoverProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on Escape or click outside
  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div ref={containerRef} className={`relative inline-flex items-center ${className}`}>
      <IconButton
        icon={<HelpCircle size={15} aria-hidden="true" />}
        tooltip={tooltip}
        ariaLabel={ariaLabel}
        variant="ghost"
        size="sm"
        active={open}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((prev) => !prev)}
      />

      {open && (
        <div
          role="dialog"
          aria-modal="false"
          aria-label={title}
          className={`panel absolute top-full mt-2 z-50 w-72 sm:w-80 p-3.5 shadow-xl border border-border-strong bg-surface anim-pop text-body-sm text-text ${
            align === "right" ? "right-0" : "left-0"
          }`}
        >
          <div className="flex items-center justify-between gap-2 border-b border-border/80 pb-2 mb-2.5">
            <h4 className="font-semibold text-small tracking-wide text-text flex items-center gap-1.5">
              <span className="text-gold font-bold">ℹ</span> {title}
            </h4>
            <IconButton
              icon={<X size={13} aria-hidden="true" />}
              tooltip="Chiudi guida (Esc)"
              ariaLabel="Chiudi guida"
              variant="ghost"
              size="sm"
              onClick={() => setOpen(false)}
            />
          </div>
          <div className="text-small text-muted leading-relaxed space-y-2 max-h-80 overflow-y-auto pr-1">
            {children}
          </div>
        </div>
      )}
    </div>
  );
}
