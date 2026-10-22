import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
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
  /** Exact or measured width of the item in pixels. */
  width?: number;
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

  // If item widths are provided on items, use exact greedy packing
  const hasItemWidths = collapsibleItems.some((i) => (i.width ?? 0) > 0);

  if (hasItemWidths && availableWidth !== undefined) {
    const totalCollapsibleWidth = collapsibleItems.reduce((sum, item) => sum + (item.width || itemMinWidth), 0);
    if (totalCollapsibleWidth <= availableWidth) {
      return {
        visibleItems: items,
        collapsedItems: [],
        isCollapsed: false,
        activeCollapsedCount: 0,
      };
    }

    const spaceForItems = Math.max(0, availableWidth - filtersButtonWidth);
    const sorted = [...collapsibleItems].sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));

    let accumulatedWidth = 0;
    const visibleIds = new Set<string>();

    for (const item of sorted) {
      const itemW = item.width || itemMinWidth;
      if (accumulatedWidth + itemW <= spaceForItems) {
        visibleIds.add(item.id);
        accumulatedWidth += itemW;
      }
    }

    const visibleItems = items.filter((i) => i.alwaysVisible || visibleIds.has(i.id));
    const collapsedItems = collapsibleItems.filter((i) => !visibleIds.has(i.id));
    const activeCollapsedCount = collapsedItems.filter((i) => i.active).length;

    return {
      visibleItems,
      collapsedItems,
      isCollapsed: collapsedItems.length > 0,
      activeCollapsedCount,
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

interface PopoverCoords {
  top: number;
  left: number;
  maxHeight: number;
  openUpwards: boolean;
}

const calcPopoverCoords = (rect: DOMRect): PopoverCoords => {
  const popoverW = 260;
  const left = Math.max(12, Math.min(rect.left, window.innerWidth - popoverW - 12));
  const spaceBelow = window.innerHeight - rect.bottom - 12;
  const spaceAbove = rect.top - 12;

  if (spaceBelow < 220 && spaceAbove > spaceBelow) {
    const maxHeight = Math.max(160, Math.min(360, spaceAbove));
    const top = Math.max(8, rect.top - maxHeight - 8);
    return { top, left, maxHeight, openUpwards: true };
  } else {
    const maxHeight = Math.max(160, Math.min(360, spaceBelow));
    const top = rect.bottom + 8;
    return { top, left, maxHeight, openUpwards: false };
  }
};

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
  const [popoverCoords, setPopoverCoords] = useState<PopoverCoords>({
    top: 0,
    left: 0,
    maxHeight: 240,
    openUpwards: false,
  });

  const closePopover = () => {
    setPopoverOpen(false);
    buttonRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
  };

  const handleTogglePopover = () => {
    if (!popoverOpen && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setPopoverCoords(calcPopoverCoords(rect));
    }
    setPopoverOpen((p) => !p);
  };

  // Synchronously compute portal position, handle Escape, outside click, and viewport scrolling
  useLayoutEffect(() => {
    if (!popoverOpen) return;

    const updateCoords = () => {
      if (!buttonRef.current) return;
      const rect = buttonRef.current.getBoundingClientRect();
      if (rect.bottom < 0 || rect.top > window.innerHeight) {
        setPopoverOpen(false);
        return;
      }
      setPopoverCoords(calcPopoverCoords(rect));
    };

    updateCoords();

    // Focus the first interactive element inside the popover dialog on open
    const timer = setTimeout(() => {
      if (popoverRef.current) {
        const focusable = popoverRef.current.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
        );
        if (focusable.length > 0) {
          focusable[0].focus();
        }
      }
    }, 0);

    const handleDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        popoverRef.current &&
        !popoverRef.current.contains(target) &&
        buttonRef.current &&
        !buttonRef.current.contains(target)
      ) {
        closePopover();
      }
    };

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        closePopover();
      }
    };

    const handleScroll = (e: Event) => {
      // Allow internal scrolling inside popover list without triggering close or jump
      if (popoverRef.current && popoverRef.current.contains(e.target as Node)) {
        return;
      }
      updateCoords();
    };

    const handleResize = () => {
      updateCoords();
    };

    document.addEventListener("mousedown", handleDown);
    document.addEventListener("keydown", handleKey);
    window.addEventListener("scroll", handleScroll, true);
    window.addEventListener("resize", handleResize);

    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handleDown);
      document.removeEventListener("keydown", handleKey);
      window.removeEventListener("scroll", handleScroll, true);
      window.removeEventListener("resize", handleResize);
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
      {left && <div className="flex items-center gap-2 shrink-0">{left}</div>}

      {/* Dynamic items and collapse button */}
      {items && renderItem && (
        <div className="flex flex-1 items-center gap-1.5 min-w-0">
          <div className="flex items-center gap-1.5 overflow-hidden py-0.5 shrink min-w-0">
            {collapseResult.visibleItems.map((item) => (
              <span key={item.id} className="shrink-0">
                {renderItem(item, false)}
              </span>
            ))}
          </div>

          {/* Collapsed «Filtri» button ALWAYS OUTSIDE the overflow list! */}
          {collapseResult.isCollapsed && (
            <div ref={buttonRef} className="relative inline-flex items-center shrink-0">
              <FilterChip
                icon={<Filter size={13} />}
                label={filtersLabel}
                count={collapseResult.activeCollapsedCount > 0 ? collapseResult.activeCollapsedCount : undefined}
                active={popoverOpen || collapseResult.activeCollapsedCount > 0}
                tooltip={`Altri filtri (${collapseResult.collapsedItems.length})`}
                onClick={handleTogglePopover}
                aria-expanded={popoverOpen}
                aria-haspopup="dialog"
              />

              {popoverOpen &&
                typeof document !== "undefined" &&
                createPortal(
                  <div
                    ref={popoverRef}
                    role="dialog"
                    aria-label="Filtri aggiuntivi"
                    style={{
                      position: "fixed",
                      top: `${popoverCoords.top}px`,
                      left: `${popoverCoords.left}px`,
                      maxHeight: `${popoverCoords.maxHeight}px`,
                    }}
                    className="panel fixed z-50 min-w-56 p-3 shadow-xl border border-border-strong bg-surface anim-pop flex flex-col gap-2"
                  >
                    <div className="flex items-center justify-between border-b border-border/80 pb-1.5 shrink-0">
                      <span className="font-semibold text-small text-text flex items-center gap-1.5">
                        <Filter size={12} className="text-gold" /> {filtersLabel}
                      </span>
                      <IconButton
                        icon={<X size={13} />}
                        tooltip="Chiudi filtri (Esc)"
                        ariaLabel="Chiudi menu filtri"
                        variant="ghost"
                        size="sm"
                        onClick={closePopover}
                      />
                    </div>
                    <div className="flex flex-col gap-1.5 overflow-y-auto pr-1">
                      {collapseResult.collapsedItems.map((item) => (
                        <div key={item.id} className="w-full">
                          {renderItem(item, true)}
                        </div>
                      ))}
                    </div>
                  </div>,
                  document.body,
                )}
            </div>
          )}
        </div>
      )}

      {children}

      {/* Right controls */}
      {right && <div className="flex items-center gap-1.5 shrink-0">{right}</div>}
    </div>
  );
}
