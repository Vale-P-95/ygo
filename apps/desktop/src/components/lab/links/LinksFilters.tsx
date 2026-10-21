import type { CardLink, CardLinkKind } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { LINK_KINDS, LINK_ZONES, type CardLinkZone, type LinksFilterState, type LinksLayoutMode } from "./types";

interface Props {
  filters: LinksFilterState;
  onChange: (filters: LinksFilterState) => void;
  layout: LinksLayoutMode;
  onLayoutChange: (layout: LinksLayoutMode) => void;
  links: CardLink[];
}

export function LinksFilters({ filters, onChange, layout, onLayoutChange, links }: Props) {
  const countsByKind = links.reduce<Record<string, number>>((acc, l) => {
    acc[l.kind] = (acc[l.kind] || 0) + 1;
    return acc;
  }, {});

  const countsByZone = links.reduce<Record<string, number>>((acc, l) => {
    acc[l.zone] = (acc[l.zone] || 0) + 1;
    return acc;
  }, {});

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

  return (
    <div className="flex flex-col gap-2.5 rounded-lg border border-border bg-surface-2/60 p-3 text-xs anim-fade">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="text-muted font-medium">Azione:</span>
          <button
            type="button"
            className={`chip !py-0.5 ${filters.kind === "all" ? "chip-active" : ""}`}
            onClick={() => setKind("all")}
          >
            Tutte ({links.length})
          </button>
          {(Object.keys(LINK_KINDS) as CardLinkKind[]).map((k) => {
            const meta = LINK_KINDS[k];
            const active = filters.kind === k;
            const count = countsByKind[k] || 0;
            return (
              <button
                key={k}
                type="button"
                className={`chip !py-0.5 flex items-center gap-1.5 ${active ? "chip-active" : ""}`}
                onClick={() => setKind(k)}
              >
                <span className="h-2 w-2 rounded-full shrink-0" style={{ background: meta.color }} />
                <span>{meta.label}</span>
                <span className="text-[10px] text-faint tabular-nums">({count})</span>
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center rounded-lg border border-border bg-surface-2 p-0.5 text-[11px]" role="tablist" aria-label="Modalità di visualizzazione">
            <button
              type="button"
              role="tab"
              aria-selected={layout === "circle"}
              className={`rounded-md px-2.5 py-0.5 font-medium transition-colors ${layout === "circle" ? "bg-surface-3 text-text shadow-xs" : "text-muted hover:text-text"}`}
              onClick={() => {
                sound.play("toggle");
                onLayoutChange("circle");
              }}
            >
              Cerchio
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={layout === "columns"}
              className={`rounded-md px-2.5 py-0.5 font-medium transition-colors ${layout === "columns" ? "bg-surface-3 text-text shadow-xs" : "text-muted hover:text-text"}`}
              onClick={() => {
                sound.play("toggle");
                onLayoutChange("columns");
              }}
            >
              Colonne
            </button>
          </div>
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border/60 pt-2">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="text-muted font-medium">Da zona:</span>
          <button
            type="button"
            className={`chip !py-0.5 !text-[11px] ${filters.zone === "all" ? "chip-active" : ""}`}
            onClick={() => setZone("all")}
          >
            Tutte
          </button>
          {(Object.keys(LINK_ZONES) as CardLinkZone[]).map((z) => {
            const meta = LINK_ZONES[z];
            const active = filters.zone === z;
            const count = countsByZone[z] || 0;
            return (
              <button
                key={z}
                type="button"
                className={`chip !py-0.5 !text-[11px] ${active ? "chip-active" : ""}`}
                onClick={() => setZone(z)}
              >
                {meta.label} <span className="text-[10px] text-faint tabular-nums">({count})</span>
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-2.5">
          <label className="flex items-center gap-1.5 text-[11px] cursor-pointer text-muted hover:text-text select-none">
            <input
              type="checkbox"
              checked={filters.onlyUsed}
              onChange={toggleOnlyUsed}
              className="accent-[var(--sec)]"
            />
            <span>Solo usati nelle combo migliori</span>
          </label>

          <input
            type="search"
            placeholder="Filtra carte…"
            value={filters.searchQuery}
            onChange={(e) => onChange({ ...filters, searchQuery: e.target.value })}
            className="field !py-0.5 !px-2 !text-xs max-w-[140px]"
          />
        </div>
      </div>
    </div>
  );
}
