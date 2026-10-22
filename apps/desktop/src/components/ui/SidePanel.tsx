import { useCallback, useEffect, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { ChevronLeft, ChevronRight, X } from "lucide-react";
import { IconButton } from "./IconButton";

export interface SidePanelProps {
  /** Identifier used to persist open/closed preference in localStorage. */
  id: string;
  /** Title shown in panel header. */
  title?: string;
  /** Optional icon shown before title. */
  icon?: ReactNode;
  /** Controlled open state. If omitted, the panel operates uncontrolled using localStorage. */
  isOpen?: boolean;
  /** Default state when no stored value exists (default: true). */
  defaultOpen?: boolean;
  /** Callback fired when the open state changes. */
  onToggle?: (open: boolean) => void;
  /** Docking position: left or right side of the container. */
  position?: "left" | "right";
  /** Panel width in pixels or CSS value (default: 300). */
  width?: number | string;
  /** Width threshold under which the panel renders as a floating overlay with backdrop (default: 1440). */
  overlayBelow?: number;
  /** Header action buttons or badges. */
  actions?: ReactNode;
  /** Content rendered inside the side panel body. */
  children: ReactNode;
  /** Optional container class name. */
  className?: string;
}

/**
 * Standard collapsible side panel remembering open state in localStorage,
 * with adaptive density (docked above 1440px, modal overlay below).
 */
export function SidePanel({
  id,
  title,
  icon,
  isOpen: controlledOpen,
  defaultOpen = true,
  onToggle,
  position = "right",
  width = 300,
  overlayBelow = 1440,
  actions,
  children,
  className = "",
}: SidePanelProps) {
  const storageKey = `df_panel_${id}`;

  const [uncontrolledOpen, setUncontrolledOpen] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      if (saved !== null) return saved === "true";
    } catch {
      // Fallback if localStorage is unavailable
    }
    return defaultOpen;
  });

  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : uncontrolledOpen;

  const [isOverlay, setIsOverlay] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return window.innerWidth < overlayBelow;
  });

  useEffect(() => {
    const handleResize = () => {
      setIsOverlay(window.innerWidth < overlayBelow);
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [overlayBelow]);

  const setOpen = useCallback(
    (nextState: boolean) => {
      if (!isControlled) {
        setUncontrolledOpen(nextState);
        try {
          localStorage.setItem(storageKey, String(nextState));
        } catch {
          // Ignore storage errors
        }
      }
      onToggle?.(nextState);
    },
    [isControlled, storageKey, onToggle],
  );

  // Handle closing with Escape when open in overlay mode
  useEffect(() => {
    if (!open || !isOverlay) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (e.defaultPrevented) return;
        // Do not close if a modal dialog (e.g. CardViewer, CardModal, ImportDialog) is open above
        if (document.querySelector('[role="dialog"], [aria-modal="true"]')) {
          return;
        }
        setOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, isOverlay, setOpen]);

  const widthStyle = typeof width === "number" ? `${width}px` : width;

  // Render closed toggle tab when docked and closed
  if (!open && !isOverlay) {
    return (
      <div className={`shrink-0 flex items-center ${className}`}>
        <IconButton
          icon={position === "left" ? <ChevronRight size={16} /> : <ChevronLeft size={16} />}
          tooltip={title ? `Apri ${title}` : "Apri pannello"}
          ariaLabel={title ? `Apri pannello ${title}` : "Apri pannello laterale"}
          variant="surface"
          size="sm"
          onClick={() => setOpen(true)}
          className="shadow-sm"
        />
      </div>
    );
  }

  // Overlay container with backdrop
  if (isOverlay) {
    if (!open) return null;

    const overlayContent = (
      <div className="fixed inset-0 z-[60] flex overflow-hidden">
        {/* Backdrop */}
        <div
          className="fixed inset-0 bg-black/60 backdrop-blur-xs transition-opacity"
          aria-hidden="true"
          onClick={() => setOpen(false)}
        />

        {/* Floating drawer - shifted past compact sidebar (68px) when docked on the left */}
        <aside
          role="region"
          aria-label={title ?? "Pannello laterale"}
          style={{ width: widthStyle }}
          className={`relative z-50 flex h-full max-w-[85vw] flex-col border-border-strong bg-surface shadow-2xl transition-transform duration-200 ease-out ${
            position === "left" ? "mr-auto ml-[68px] border-r" : "ml-auto border-l"
          } ${className}`}
        >
          {/* Header */}
          <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/80 px-3.5 bg-surface-2/60">
            <div className="flex items-center gap-2 truncate">
              {icon && <span className="text-sec shrink-0">{icon}</span>}
              {title && <h3 className="truncate font-semibold text-small tracking-wide text-text">{title}</h3>}
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              {actions}
              <IconButton
                icon={<X size={15} />}
                tooltip="Chiudi pannello (Esc)"
                ariaLabel="Chiudi pannello laterale"
                variant="ghost"
                size="sm"
                onClick={() => setOpen(false)}
              />
            </div>
          </div>

          {/* Body */}
          <div className="flex-1 overflow-y-auto p-3.5">{children}</div>
        </aside>
      </div>
    );

    return typeof document !== "undefined" ? createPortal(overlayContent, document.body) : overlayContent;
  }

  // Docked container
  return (
    <aside
      role="region"
      aria-label={title ?? "Pannello laterale"}
      style={{ width: widthStyle }}
      className={`shrink-0 flex h-full flex-col border-border bg-surface/90 shadow-md ${
        position === "left" ? "border-r" : "border-l"
      } ${className}`}
    >
      {/* Header */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/80 px-3 bg-surface-2/40">
        <div className="flex items-center gap-2 truncate">
          {icon && <span className="text-sec shrink-0">{icon}</span>}
          {title && <h3 className="truncate font-semibold text-small tracking-wide text-text">{title}</h3>}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {actions}
          <IconButton
            icon={position === "left" ? <ChevronLeft size={15} /> : <ChevronRight size={15} />}
            tooltip="Chiudi pannello"
            ariaLabel="Comprimi pannello laterale"
            variant="ghost"
            size="sm"
            onClick={() => setOpen(false)}
          />
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto p-3">{children}</div>
    </aside>
  );
}
