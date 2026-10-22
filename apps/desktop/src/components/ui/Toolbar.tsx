import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Filter, X } from "lucide-react";
import { FilterChip } from "./FilterChip";
import { IconButton } from "./IconButton";

export interface ToolbarItem<T = unknown> {
  id: string;
  label: string;
  /** Higher priority items stay visible longer before collapsing (default: 0). */
  priority?: number;
  /** Items marked alwaysVisible never collapse into the popover (e.g. search input). */
  alwaysVisible?: boolean;
  /** Whether the item filter is currently applied/active. */
  active?: boolean;
  /** Optional payload data. */
  data?: T;
}

export interface ToolbarCollapseConfig {
  /** Maximum number of collapsible items visible in the main bar. */
  maxVisibleItems?: number;
  /** Total available width in pixels for collapsible items. */
  availableWidth?: number;
  /** Estimated min width required per item in pixels (default: 120). */
  itemMinWidth?: number;
  /** Width reserved for the «Filtri» button when collapsed (default: 88). */
  filtersButtonWidth?: number;
  /** Force compact mode where all non-alwaysVisible items collapse (e.g. screen < 1440px). */
  isCompact?: boolean;
}

export interface ToolbarCollapseResult<T = unknown> {
  visibleItems: ToolbarItem<T>[];
  collapsedItems: ToolbarItem<T>[];
  isCollapsed: boolean;
  activeCollapsedCount: number;
}

/**
 * Pure calculation determining which items remain visible in the toolbar
 * and which collapse into the «Filtri» popover.
 */
export function resolveToolbarCollapse<T = unknown>(
  items: ToolbarItem<T>[],
  config: ToolbarCollapseConfig = {},
): ToolbarCollapseResult<T> {
  const {
    maxVisibleItems,
    availableWidth,
    itemMinWidth = 120,
    filtersButtonWidth = 88,
    isCompact = false,
  } = config;

  const alwaysVisibleItems = items.filter((i) => i.alwaysVisible);
  const collapsibleItems = items.filter((i) => !i.alwaysVisible);

  if (collapsibleItems.length === 0) {
    return {
      visibleItems: items,
      collapsedItems: [],
      isCollapsed: false,
      activeCollapsedCount: 0,
    };
  }

  // Force all collapsible items into the drawer
  if (isCompact) {
    const activeCount = collapsibleItems.filter((i) => i.active).length;
    return {
      visibleItems: alwaysVisibleItems,
      collapsedItems: collapsibleItems,
      isCollapsed: true,
      activeCollapsedCount: activeCount,
    };
  }

  let capacity = collapsibleItems.length;

  if (maxVisibleItems !== undefined) {
    capacity = Math.max(0, maxVisibleItems);
  } else if (availableWidth !== undefined) {
    const totalNeeded = collapsibleItems.length * itemMinWidth;
    if (totalNeeded <= availableWidth) {
      capacity = collapsibleItems.length;
    } else {
      const spaceForItems = Math.max(0, availableWidth - filtersButtonWidth);
      capacity = Math.max(0, Math.floor(spaceForItems / itemMinWidth));
    }
  }

  if (collapsibleItems.length <= capacity) {
    return {
      visibleItems: items,
      collapsedItems: [],
      isCollapsed: false,
      activeCollapsedCount: 0,
    };
  }

  // Pick top N highest priority items
  const sorted = [...collapsibleItems].sort(
    (a, b) => (b.priority ?? 0) - (a.priority ?? 0),
  );
  const visibleSubset = new Set(sorted.slice(0, capacity).map((i) => i.id));

  const visibleItems = items.filter((i) => i.alwaysVisible || visibleSubset.has(i.id));
  const collapsedItems = collapsibleItems.filter((i) => !visibleSubset.has(i.id));
  const activeCollapsedCount = collapsedItems.filter((i) => i.active).length;

  return {
    visibleItems,
    collapsedItems,
    isCollapsed: collapsedItems.length > 0,
    activeCollapsedCount,
  };
}

export interface ToolbarProps<T = unknown> {
  /** Optional array of items managed by the toolbar layout. */
  items?: ToolbarItem<T>[];
  /** Render function for each item (in main bar or inside the popover). */
  renderItem?: (item: ToolbarItem<T>, inPopover: boolean) => ReactNode;
  /** Content placed on the left (e.g. search input). */
  left?: ReactNode;
  /** Content placed on the right (e.g. view mode switch, action buttons). */
  right?: ReactNode;
  /** Direct children if not using `items`. */
  children?: ReactNode;
  /** Collapse configuration. */
  config?: ToolbarCollapseConfig;
  /** Label for the collapse button (default: "Filtri"). */
  filtersLabel?: string;
  /** Optional custom class name. */
  className?: string;
}

/**
 * Standard compact toolbar supporting single-row presentation with
 * automatic collapsing into «Filtri» popover when space is limited.
 */
export function Toolbar<T = unknown>({
  items,
  renderItem,
  left,
  right,
  children,
  config = {},
  filtersLabel = "Filtri",
  className = "",
}: ToolbarProps<T>) {
  const [popoverOpen, setPopoverOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLDivElement>(null);
  const [popoverCoords, setPopoverCoords] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // Update fixed portal coords and close on Escape, outside click, scroll, or resize
  useEffect(() => {
    if (!popoverOpen) return;

    const updateCoords = () => {
      if (!buttonRef.current) return;
      const rect = buttonRef.current.getBoundingClientRect();
      const left = Math.max(12, Math.min(rect.left, window.innerWidth - 260));
      setPopoverCoords({ top: rect.bottom + 8, left });
    };

    updateCoords();

    const handleDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        popoverRef.current &&
        !popoverRef.current.contains(target) &&
        buttonRef.current &&
        !buttonRef.current.contains(target)
      ) {
        setPopoverOpen(false);
      }
    };

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPopoverOpen(false);
    };

    const handleScrollOrResize = () => {
      setPopoverOpen(false);
    };

    document.addEventListener("mousedown", handleDown);
    document.addEventListener("keydown", handleKey);
    window.addEventListener("scroll", handleScrollOrResize, true);
    window.addEventListener("resize", handleScrollOrResize);

    return () => {
      document.removeEventListener("mousedown", handleDown);
      document.removeEventListener("keydown", handleKey);
      window.removeEventListener("scroll", handleScrollOrResize, true);
      window.removeEventListener("resize", handleScrollOrResize);
    };
  }, [popoverOpen]);

  const collapseResult = items
    ? resolveToolbarCollapse(items, config)
    : {
        visibleItems: [],
        collapsedItems: [],
        isCollapsed: false,
        activeCollapsedCount: 0,
      };

  return (
    <div
      role="toolbar"
      className={`flex items-center justify-between gap-2.5 rounded-xl border border-border/80 bg-surface/75 px-3 py-2 shadow-xs backdrop-blur-xs ${className}`}
    >
      {/* Left controls */}
      <div className="flex flex-1 items-center gap-2 overflow-hidden">
        {left}

        {/* Dynamic items */}
        {items && renderItem && (
          <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar py-0.5">
            {collapseResult.visibleItems.map((item) => (
              <span key={item.id} className="shrink-0">
                {renderItem(item, false)}
              </span>
            ))}

            {/* Collapsed «Filtri» button & fixed portal popover */}
            {collapseResult.isCollapsed && (
              <div ref={buttonRef} className="relative inline-flex items-center shrink-0">
                <FilterChip
                  icon={<Filter size={13} />}
                  label={filtersLabel}
                  count={collapseResult.activeCollapsedCount > 0 ? collapseResult.activeCollapsedCount : undefined}
                  active={popoverOpen || collapseResult.activeCollapsedCount > 0}
                  tooltip={`Altri filtri (${collapseResult.collapsedItems.length})`}
                  onClick={() => setPopoverOpen((p) => !p)}
                  aria-expanded={popoverOpen}
                  aria-haspopup="dialog"
                />

                {popoverOpen && typeof document !== "undefined" && createPortal(
                  <div
                    ref={popoverRef}
                    role="dialog"
                    aria-label="Filtri aggiuntivi"
                    style={{ position: "fixed", top: `${popoverCoords.top}px`, left: `${popoverCoords.left}px` }}
                    className="panel fixed z-50 min-w-56 p-3 shadow-xl border border-border-strong bg-surface anim-pop space-y-2"
                  >
                    <div className="flex items-center justify-between border-b border-border/80 pb-1.5">
                      <span className="font-semibold text-small text-text flex items-center gap-1.5">
                        <Filter size={12} className="text-gold" /> {filtersLabel}
                      </span>
                      <IconButton
                        icon={<X size={13} />}
                        tooltip="Chiudi filtri (Esc)"
                        ariaLabel="Chiudi menu filtri"
                        variant="ghost"
                        size="sm"
                        onClick={() => setPopoverOpen(false)}
                      />
                    </div>
                    <div className="flex flex-col gap-1.5 max-h-60 overflow-y-auto pr-1">
                      {collapseResult.collapsedItems.map((item) => (
                        <div key={item.id} className="w-full">
                          {renderItem(item, true)}
                        </div>
                      ))}
                    </div>
                  </div>,
                  document.body
                )}
              </div>
            )}
          </div>
        )}

        {children}
      </div>

      {/* Right controls */}
      {right && <div className="flex items-center gap-1.5 shrink-0">{right}</div>}
    </div>
  );
}
