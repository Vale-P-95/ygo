import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ArrowDownToLine,
  CircleDot,
  Columns3,
  Combine,
  Flame,
  Layers,
  RotateCcw,
  Search,
  Sparkles,
  Trash2,
} from "lucide-react";
import type { CardLink, CardLinkKind } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { FilterChip } from "../../ui/FilterChip";
import { IconButton } from "../../ui/IconButton";
import { Toolbar, type ToolbarItem } from "../../ui/Toolbar";
import {
  LINK_KINDS,
  LINK_ZONES,
  type CardLinkZone,
  type LinksFilterState,
  type LinksLayoutMode,
} from "./types";

interface Props {
  filters: LinksFilterState;
  onChange: (filters: LinksFilterState) => void;
  layout: LinksLayoutMode;
  onLayoutChange: (layout: LinksLayoutMode) => void;
  links: CardLink[];
}

const KIND_ICONS: Record<CardLinkKind, ReactNode> = {
  search: <Search className="h-3 w-3" />,
  summon: <Sparkles className="h-3 w-3" />,
  set: <ArrowDownToLine className="h-3 w-3" />,
  toGrave: <Trash2 className="h-3 w-3" />,
  recover: <RotateCcw className="h-3 w-3" />,
  material: <Combine className="h-3 w-3" />,
};

export function LinksFilters({ filters, onChange, layout, onLayoutChange, links }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLDivElement>(null);
  const [availableWidth, setAvailableWidth] = useState<number | undefined>(undefined);
  const [measuredWidths, setMeasuredWidths] = useState<Record<string, number>>({});
  const [isBelow1440, setIsBelow1440] = useState(() => {
    return typeof window !== "undefined" && window.innerWidth < 1440;
  });

  useEffect(() => {
    if (typeof window === "undefined") return;
    const mq = window.matchMedia("(max-width: 1439px)");
    const handler = (e: MediaQueryListEvent) => setIsBelow1440(e.matches);
    setIsBelow1440(mq.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  // Measure actual container and controls width to dynamically manage chips
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const update = () => {
      const totalW = el.clientWidth;
      const leftW = leftRef.current ? leftRef.current.getBoundingClientRect().width : 450;
      const rightW = rightRef.current ? rightRef.current.getBoundingClientRect().width : 80;
      // Subtract left, right, and toolbar margins/gaps (~48px)
      const free = Math.max(0, totalW - leftW - rightW - 48);
      setAvailableWidth(free);
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    if (leftRef.current) ro.observe(leftRef.current);
    return () => ro.disconnect();
  }, []);

  const countsByKind = useMemo(() => {
    return links.reduce<Record<string, number>>((acc, l) => {
      acc[l.kind] = (acc[l.kind] || 0) + 1;
      return acc;
    }, {});
  }, [links]);

  const countsByZone = useMemo(() => {
    return links.reduce<Record<string, number>>((acc, l) => {
      acc[l.zone] = (acc[l.zone] || 0) + 1;
      return acc;
    }, {});
  }, [links]);

  const setKind = (k: CardLinkKind | "all") => {
    sound.play("toggle");
    onChange({ ...filters, kind: k });
  };

  const setZone = (z: CardLinkZone | "all") => {
    sound.play("toggle");
    onChange({ ...filters, zone: z });
  };

  const toggleOnlyUsed = () => {
    sound.play("toggle");
    onChange({ ...filters, onlyUsed: !filters.onlyUsed });
  };

  // Prepare candidate items for Toolbar collapsing
  const toolbarItems: ToolbarItem<{ kind: CardLinkKind | "all"; color?: string; count: number }>[] = useMemo(() => {
    const list: ToolbarItem<{ kind: CardLinkKind | "all"; color?: string; count: number }>[] = [
      {
        id: "all",
        label: "Tutte",
        active: false,
        alwaysVisible: false,
        priority: 100,
        width: measuredWidths["all"] || 89,
        data: { kind: "all", count: links.length },
      },
    ];

    (Object.keys(LINK_KINDS) as CardLinkKind[]).forEach((k) => {
      const count = countsByKind[k] || 0;
      if (count > 0 || filters.kind === k) {
        list.push({
          id: k,
          label: LINK_KINDS[k].label,
          active: filters.kind === k,
          priority: count,
          width: measuredWidths[k] || 140,
          data: {
            kind: k,
            color: LINK_KINDS[k].color,
            count,
          },
        });
      }
    });

    return list;
  }, [countsByKind, filters.kind, links.length, measuredWidths]);

  // Hidden offscreen measurement of exact chip widths in real DOM
  useLayoutEffect(() => {
    if (!measureRef.current) return;
    const widths: Record<string, number> = {};
    const children = measureRef.current.children;
    for (let i = 0; i < children.length; i++) {
      const child = children[i] as HTMLElement;
      const id = child.dataset.itemId;
      if (id) {
        widths[id] = Math.ceil(child.getBoundingClientRect().width + 6);
      }
    }
    setMeasuredWidths((prev) => {
      let changed = false;
      for (const [k, v] of Object.entries(widths)) {
        if (prev[k] !== v) {
          changed = true;
          break;
        }
      }
      return changed ? widths : prev;
    });
  }, [countsByKind, links.length]);

  const renderChip = (
    item: ToolbarItem<{ kind: CardLinkKind | "all"; color?: string; count: number }>,
    inPopover: boolean,
  ) => {
    const d = item.data;
    if (!d) return null;
    const isAll = d.kind === "all";
    const icon = isAll ? undefined : KIND_ICONS[d.kind as CardLinkKind];
    const isSelected = isAll ? filters.kind === "all" : filters.kind === d.kind;

    return (
      <FilterChip
        icon={icon}
        label={item.label}
        count={d.count}
        active={isSelected}
        disableWhenZero={!isAll}
        color={d.color}
        size="sm"
        className={inPopover ? "w-full justify-between" : undefined}
        tooltip={isAll ? `Tutti i tipi di collegamento (${d.count})` : `${item.label} (${d.count})`}
        onClick={() => {
          if (isAll) {
            setKind("all");
          } else {
            setKind(filters.kind === d.kind ? "all" : d.kind);
          }
        }}
      />
    );
  };

  return (
    <div ref={containerRef} className="w-full">
      {/* Hidden offscreen container measuring real rendered chip widths */}
      <div
        ref={measureRef}
        aria-hidden="true"
        className="fixed top-0 left-0 invisible pointer-events-none -z-50 flex items-center gap-1.5"
      >
        {toolbarItems.map((item) => (
          <div key={item.id} data-item-id={item.id} className="shrink-0">
            {renderChip(item, false)}
          </div>
        ))}
      </div>

      <Toolbar
        items={toolbarItems}
        config={{
          isCompact: isBelow1440,
          availableWidth,
          filtersButtonWidth: 96,
        }}
        filtersLabel="Filtri"
        left={
          <div ref={leftRef} className="flex items-center gap-2 shrink-0">
            {/* Card search */}
            <div className="relative flex items-center min-w-[130px] max-w-[180px]">
              <Search className="absolute left-2.5 h-3.5 w-3.5 text-muted pointer-events-none" aria-hidden />
              <input
                type="search"
                placeholder="Cerca carta…"
                aria-label="Cerca carta nei collegamenti"
                value={filters.searchQuery}
                onChange={(e) => onChange({ ...filters, searchQuery: e.target.value })}
                className="field !py-1 !pl-8 !pr-2 !text-xs w-full rounded-lg"
              />
            </div>

            {/* Start Zone dropdown with right padding to prevent text overlapping arrow */}
            <div className="flex items-center gap-1.5 text-xs text-muted shrink-0">
              <Layers className="h-3.5 w-3.5 text-muted shrink-0" aria-hidden />
              <select
                aria-label="Filtra per zona di partenza"
                value={filters.zone}
                onChange={(e) => setZone(e.target.value as CardLinkZone | "all")}
                className="field !py-1 !pl-2 !pr-7 !text-xs rounded-lg cursor-pointer"
              >
                <option value="all">Tutte le zone ({links.length})</option>
                {(Object.keys(LINK_ZONES) as CardLinkZone[])
                  .filter((z) => (countsByZone[z] || 0) > 0 || filters.zone === z)
                  .map((z) => (
                    <option key={z} value={z}>
                      {LINK_ZONES[z].label} ({countsByZone[z] || 0})
                    </option>
                  ))}
              </select>
            </div>

            {/* Only used in top combos toggle */}
            <FilterChip
              icon={<Flame className="h-3.5 w-3.5 text-sec" />}
              label="Solo combo migliori"
              active={filters.onlyUsed}
              onClick={toggleOnlyUsed}
              tooltip="Filtra solo le connessioni usate nelle combo con il punteggio più alto"
              size="sm"
            />
          </div>
        }
        renderItem={renderChip}
        right={
          /* Accessible group (eliminates axe aria-required-children error) */
          <div
            ref={rightRef}
            className="flex items-center rounded-lg border border-border bg-surface-2 p-0.5 shrink-0"
            role="group"
            aria-label="Modalità di visualizzazione"
          >
            <IconButton
              icon={<CircleDot className="h-4 w-4" />}
              ariaLabel="Vista mappa cerchio"
              tooltip="Vista mappa a cerchio"
              variant="ghost"
              size="sm"
              active={layout === "circle"}
              onClick={() => {
                sound.play("toggle");
                onLayoutChange("circle");
              }}
            />
            <IconButton
              icon={<Columns3 className="h-4 w-4" />}
              ariaLabel="Vista colonne per zona"
              tooltip="Vista a colonne per zona"
              variant="ghost"
              size="sm"
              active={layout === "columns"}
              onClick={() => {
                sound.play("toggle");
                onLayoutChange("columns");
              }}
            />
          </div>
        }
      />
    </div>
  );
}
