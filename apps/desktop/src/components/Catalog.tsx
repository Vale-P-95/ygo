import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import {
  Bookmark,
  Check,
  ChevronDown,
  Grid2X2,
  Grid3X3,
  Hammer,
  LayoutGrid,
  List,
  Search,
  ShieldAlert,
  SlidersHorizontal,
  Sparkles,
  Star,
  X,
} from "lucide-react";
import { api, type Card, type Facets, type SavedSearch, type SearchResult } from "../lib/api";
import type { Deck } from "../lib/api";
import { copiesInDeck, copyLimit } from "../lib/deck";
import { useDrag } from "../lib/drag";
import { toggleFavorite, useFavorites } from "../lib/favorites";
import { hasToken, removeToken, toggleToken } from "../lib/query";
import { updateSettings, useSettings, type CatalogView } from "../lib/settings";
import { sound } from "../lib/sound";
import { toast } from "../lib/toast";
import { CardGrid } from "./CardGrid";
import { CardList } from "./CardList";
import type { TileBadge } from "./CardTile";
import { EmblemIcon } from "./Emblems";
import { FilterPanel } from "./FilterPanel";
import { Popover, useOutsideClose } from "./Popover";
import { HelpPopover } from "./ui/HelpPopover";
import { IconButton } from "./ui/IconButton";

interface Props {
  deck: Deck;
  getCard: (id: number) => Card | undefined;
  requestCards: (ids: number[]) => void;
  version: number;
  onAdd: (id: number, el: HTMLElement, toSide: boolean) => void;
  onOpen: (id: number, ids: number[]) => void;
  onHover: (id: number, el: HTMLElement) => void;
  onHoverEnd: () => void;
  query: string;
  onQuery: (q: string) => void;
  selectedId?: number | null;
  onSelect?: (id: number) => void;
}

/** Smallest tile width per grid size: about 3, 4 or 5 cards per row at the default width. */
const TILE: Record<Exclude<CatalogView, "list">, number> = { l: 100, m: 78, s: 58 };

interface ViewOption {
  view: CatalogView;
  label: string;
  icon: typeof LayoutGrid;
}

const VIEW_OPTIONS: ViewOption[] = [
  { view: "l", label: "Carte grandi", icon: LayoutGrid },
  { view: "m", label: "Carte medie", icon: Grid2X2 },
  { view: "s", label: "Carte piccole", icon: Grid3X3 },
  { view: "list", label: "Elenco", icon: List },
];

/**
 * Dropdown menu for selecting the catalog view mode (large, medium, small, list).
 */
function CatalogViewMenu({ view, onViewChange }: { view: CatalogView; onViewChange: (v: CatalogView) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useOutsideClose(open, () => setOpen(false));
  const current = VIEW_OPTIONS.find((v) => v.view === view) ?? VIEW_OPTIONS[1];
  const CurrentIcon = current.icon;

  return (
    <div ref={ref} className="relative shrink-0">
      <button
        type="button"
        className={`inline-flex items-center gap-1 h-7 px-2 rounded-lg text-xs font-medium transition cursor-pointer select-none border ${
          open
            ? "border-gold/70 bg-surface-3 text-text"
            : "border-border bg-surface-2 text-muted hover:border-border-strong hover:bg-surface-3 hover:text-text"
        }`}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`Vista catalogo: ${current.label}`}
        data-tip={`Vista: ${current.label}`}
        onClick={() => {
          sound.play(open ? "close" : "open");
          setOpen((v) => !v);
        }}
      >
        <CurrentIcon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        <ChevronDown className="h-3 w-3 shrink-0 text-muted" aria-hidden="true" />
      </button>

      <Popover open={open} align="right" width={180}>
        <div className="p-1 flex flex-col gap-0.5" role="menu" aria-label="Opzioni vista catalogo">
          <div className="px-2 py-1 text-[11px] font-semibold text-faint uppercase tracking-wider">
            Vista catalogo
          </div>
          {VIEW_OPTIONS.map((opt) => {
            const Icon = opt.icon;
            const isSelected = opt.view === view;
            return (
              <button
                key={opt.view}
                type="button"
                role="menuitemradio"
                aria-checked={isSelected}
                className={`flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-xs text-left transition-colors cursor-pointer ${
                  isSelected
                    ? "bg-sec/15 text-sec font-medium"
                    : "text-text hover:bg-surface-3"
                }`}
                onClick={() => {
                  sound.play("toggle");
                  onViewChange(opt.view);
                  setOpen(false);
                }}
              >
                <Icon className="h-3.5 w-3.5 shrink-0 text-muted" aria-hidden="true" />
                <span className="flex-1 truncate">{opt.label}</span>
                {isSelected && <Check className="h-3.5 w-3.5 shrink-0 text-sec" aria-hidden="true" />}
              </button>
            );
          })}
        </div>
      </Popover>
    </div>
  );
}

/**
 * Saved searches dropdown menu embedded inside the search field.
 */
function SavedSearchesDropdown({ query, onQuery }: { query: string; onQuery: (q: string) => void }) {
  const [open, setOpen] = useState(false);
  const [saved, setSaved] = useState<SavedSearch[]>([]);
  const [name, setName] = useState("");
  const ref = useOutsideClose(open, () => setOpen(false));

  useEffect(() => {
    api.listSavedSearches().then(setSaved).catch(() => setSaved([]));
  }, []);

  const save = async () => {
    try {
      setSaved(await api.saveSearch(name, query));
      setName("");
      sound.play("chime");
    } catch (e) {
      sound.play("error");
      toast(String(e), "error");
    }
  };

  return (
    <div ref={ref} className="relative flex items-center pr-1 shrink-0">
      <button
        type="button"
        className={`p-1 rounded-md text-muted hover:text-text transition-colors cursor-pointer ${
          open ? "bg-surface-3 text-gold" : ""
        }`}
        aria-expanded={open}
        data-tip="Ricerche salvate"
        aria-label="Ricerche salvate"
        onClick={() => {
          sound.play(open ? "close" : "open");
          setOpen((v) => !v);
        }}
      >
        <Bookmark className="h-3.5 w-3.5" aria-hidden="true" />
      </button>
      <Popover open={open} align="right" width={320}>
        <form
          className="flex gap-1.5 p-1"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <input
            className="field !py-1 text-xs"
            placeholder="Nome per salvare la ricerca…"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <button
            type="submit"
            className="btn btn-molten shrink-0 !px-2.5 !py-1 text-xs cursor-pointer"
            disabled={!name.trim() || !query.trim()}
          >
            Salva
          </button>
        </form>
        {saved.length === 0 && (
          <p className="px-2 py-2 text-xs text-faint">
            Nessuna ricerca salvata: scrivi una ricerca, dalle un nome e premi Salva.
          </p>
        )}
        {saved.length > 0 && <div className="my-1.5 border-t border-border" />}
        {saved.map((s) => (
          <div key={s.name} className="group flex items-center gap-1 rounded-lg hover:bg-surface-3">
            <button
              type="button"
              className="min-w-0 flex-1 px-2 py-1.5 text-left cursor-pointer"
              onClick={() => {
                sound.play("page");
                onQuery(s.query);
                setOpen(false);
              }}
            >
              <span className="block truncate text-sm font-medium">{s.name}</span>
              <span className="block truncate font-mono text-[10px] text-faint">{s.query}</span>
            </button>
            <button
              type="button"
              className="px-2 text-faint opacity-0 group-hover:opacity-100 hover:text-danger cursor-pointer"
              title="Elimina"
              aria-label={`Elimina ${s.name}`}
              onClick={async () => {
                sound.play("close");
                setSaved(await api.deleteSavedSearch(s.name));
              }}
            >
              ✕
            </button>
          </div>
        ))}
      </Popover>
    </div>
  );
}

const CATALOG_MIN = 300;
const CATALOG_MAX = 760;

/**
 * Drag handle on the catalog's left edge. While dragging it only updates
 * the layout's CSS variable; the width is saved when the pointer is released.
 */
function startResize(e: ReactPointerEvent<HTMLDivElement>, width: number) {
  if (e.button !== 0) return;
  e.preventDefault();
  const layout = e.currentTarget.closest<HTMLElement>("[data-deck-layout]");
  const startX = e.clientX;
  let next = width;
  const move = (ev: PointerEvent) => {
    next = Math.round(Math.min(CATALOG_MAX, Math.max(CATALOG_MIN, width - (ev.clientX - startX))));
    layout?.style.setProperty("--catalog-w", `${next}px`);
  };
  const up = () => {
    removeEventListener("pointermove", move);
    removeEventListener("pointerup", up);
    document.body.style.cursor = "";
    updateSettings({ catalogWidth: next });
  };
  document.body.style.cursor = "col-resize";
  addEventListener("pointermove", move);
  addEventListener("pointerup", up);
}

/**
 * The card catalog column of the deck editor. Dropping deck cards here
 * removes them from the deck.
 */
export function Catalog({
  deck,
  getCard,
  requestCards,
  version,
  onAdd,
  onOpen,
  onHover,
  onHoverEnd,
  query,
  onQuery,
  selectedId,
  onSelect,
}: Props) {
  const [result, setResult] = useState<SearchResult>({ ids: [], chips: [], errors: [] });
  const [searching, setSearching] = useState(false);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [filters, setFilters] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const drag = useDrag();
  const removing = drag && drag.source.from !== "catalog";
  const { catalogView: view, catalogWidth } = useSettings();
  const favorites = useFavorites();
  // Re-run a favourites search when the favourites change.
  const favKey = hasToken(query, "tag:fav") ? favorites : null;

  const listBadges = (id: number): TileBadge[] => {
    const badges: TileBadge[] = [];
    const card = getCard(id);
    const limit = card ? copyLimit(card, deck.format) : 3;
    if (limit === 0) badges.push({ text: "Proibita", tone: "ban" });
    else if (limit < 3) badges.push({ text: limit === 1 ? "Limitata" : "Semi-lim.", tone: "limit" });
    const n = copiesInDeck(deck, id);
    if (n) badges.push({ text: `×${n}` });
    return badges;
  };

  useEffect(() => {
    api.facets().then(setFacets).catch(() => setFacets(null));
  }, []);

  useEffect(() => {
    setSearching(true);
    const timer = window.setTimeout(() => {
      api
        .search(query)
        .then((r) => {
          setResult(r);
          requestCards(r.ids.slice(0, 45));
        })
        .catch(() => sound.play("error"))
        .finally(() => setSearching(false));
    }, 160);
    return () => window.clearTimeout(timer);
  }, [query, requestCards, favKey]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, []);

  const activeFilterCount = result.chips.length;
  const hasRemovableChips = result.chips.length > 0 || result.errors.length > 0;

  return (
    <aside
      data-dropzone="catalog"
      className="relative flex min-h-0 flex-col border-l border-border bg-surface/70 transition-colors"
      style={{
        background:
          drag?.over === "catalog" && removing
            ? "color-mix(in oklab, var(--df-danger) 12%, var(--df-surface))"
            : undefined,
      }}
    >
      <div
        className="group absolute inset-y-0 -left-[5px] z-20 w-[9px] cursor-col-resize"
        onPointerDown={(e) => startResize(e, catalogWidth)}
        onDoubleClick={() => updateSettings({ catalogWidth: 392 })}
        title="Trascina per allargare il catalogo (doppio clic: larghezza iniziale)"
      >
        <span className="absolute inset-y-0 left-1 w-px bg-transparent transition-colors group-hover:bg-sec" />
      </div>

      {/* Header and Toolbar area */}
      <div className="flex flex-col gap-2 px-3 pt-3 pb-2">
        {/* Title row with discrete counter and single-button view menu */}
        <div className="flex items-center gap-2">
          <p className="display text-sm font-bold tracking-[0.12em] uppercase">Catalogo</p>
          <HelpPopover title="Guida al catalogo e scorciatoie" tooltip="Guida del catalogo" align="left">
            <div className="flex flex-col gap-2 text-xs text-text/90">
              <p className="font-semibold text-text">Azioni rapide sulle carte</p>
              <ul className="list-disc pl-4 space-y-1 text-muted">
                <li>
                  <strong>Clic singolo:</strong> fissa la carta nell'Ispettore.
                </li>
                <li>
                  <strong>Doppio clic:</strong> visore a schermo intero.
                </li>
                <li>
                  <strong>Clic destro:</strong> aggiungi 1 copia al Main (o Extra) Deck.
                </li>
                <li>
                  <strong>Maiusc + clic destro:</strong> aggiungi 1 copia al Side Deck.
                </li>
                <li>
                  <strong>Trascina:</strong> sposta la carta nella zona del deck desiderata.
                </li>
              </ul>
              <div className="border-t border-border/60 pt-1 text-muted">
                <p>Trascina le carte del deck qui sopra per toglierle dal deck.</p>
              </div>
            </div>
          </HelpPopover>

          {/* Discrete card counter */}
          <span className="flex items-center gap-1 text-xs text-muted tabular-nums">
            {searching && <EmblemIcon emblem="gear" className="h-3 w-3 animate-[spin-slow_1.6s_linear_infinite] text-gold" />}
            <span>
              {result.ids.length.toLocaleString("it-IT")} <span className="text-faint">carte</span>
            </span>
          </span>

          {/* Catalog view selector as a single menu button */}
          <div className="ml-auto">
            <CatalogViewMenu
              view={view}
              onViewChange={(v) => updateSettings({ catalogView: v })}
            />
          </div>
        </div>

        {/* Single row: search field with embedded saved searches, quick filter icon toggles, and filters button */}
        <div className="flex items-center gap-1.5">
          {/* Wide search input with embedded saved searches dropdown */}
          <div className="relative flex min-w-0 flex-1 items-center rounded-lg border border-border bg-bg-2 focus-within:border-gold/70 focus-within:ring-1 focus-within:ring-gold/70 transition-colors">
            <Search className="pointer-events-none ml-2.5 h-3.5 w-3.5 shrink-0 text-muted" aria-hidden="true" />
            <input
              ref={inputRef}
              className="w-full min-w-0 bg-transparent px-2 py-1.5 text-xs text-text placeholder:text-faint focus:outline-none"
              placeholder="Cerca o filtra… (Ctrl+F)"
              value={query}
              spellCheck={false}
              onChange={(e) => onQuery(e.target.value)}
            />
            {query && (
              <button
                type="button"
                className="mr-0.5 p-1 text-muted hover:text-text rounded transition-colors cursor-pointer"
                onClick={() => {
                  onQuery("");
                  inputRef.current?.focus();
                }}
                title="Cancella ricerca"
                aria-label="Cancella ricerca"
              >
                <X className="h-3 w-3" />
              </button>
            )}
            <SavedSearchesDropdown query={query} onQuery={onQuery} />
          </div>

          {/* Quick filter icon toggle buttons */}
          <div className="flex items-center gap-0.5 shrink-0" role="group" aria-label="Filtri rapidi">
            <IconButton
              size="sm"
              variant="ghost"
              active={hasToken(query, "tag:handtrap")}
              icon={<ShieldAlert className="h-3.5 w-3.5" />}
              tooltip="Handtrap (tag:handtrap)"
              ariaLabel="Filtra Handtrap"
              onClick={() => {
                sound.play("toggle");
                onQuery(toggleToken(query, "tag:handtrap"));
              }}
            />
            <IconButton
              size="sm"
              variant="ghost"
              active={hasToken(query, "tag:staple")}
              icon={<Sparkles className="h-3.5 w-3.5" />}
              tooltip="Staple (tag:staple)"
              ariaLabel="Filtra Staple"
              onClick={() => {
                sound.play("toggle");
                onQuery(toggleToken(query, "tag:staple"));
              }}
            />
            <IconButton
              size="sm"
              variant="ghost"
              active={hasToken(query, "tag:breaker")}
              icon={<Hammer className="h-3.5 w-3.5" />}
              tooltip="Board breaker (tag:breaker)"
              ariaLabel="Filtra Board breaker"
              onClick={() => {
                sound.play("toggle");
                onQuery(toggleToken(query, "tag:breaker"));
              }}
            />
            <IconButton
              size="sm"
              variant="ghost"
              active={hasToken(query, "tag:fav")}
              icon={<Star className="h-3.5 w-3.5" />}
              tooltip="Preferite (tag:fav)"
              ariaLabel="Filtra Preferite"
              onClick={() => {
                sound.play("toggle");
                onQuery(toggleToken(query, "tag:fav"));
              }}
            />
          </div>

          {/* Filtri button with active filter count badge */}
          <button
            type="button"
            className={`inline-flex items-center gap-1.5 h-7 px-2.5 rounded-lg text-xs font-medium shrink-0 transition-colors cursor-pointer select-none border ${
              filters
                ? "btn-molten border-gold/60 text-text"
                : activeFilterCount > 0
                  ? "border-sec/80 bg-sec/15 text-sec hover:bg-sec/25"
                  : "border-border bg-surface-2 text-muted hover:border-border-strong hover:bg-surface-3 hover:text-text"
            }`}
            onClick={() => {
              sound.play(filters ? "close" : "open");
              setFilters((v) => !v);
            }}
            aria-expanded={filters}
            title="Pannello filtri avanzati"
          >
            <SlidersHorizontal className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
            <span>Filtri</span>
            {activeFilterCount > 0 && (
              <span className="rounded-full bg-sec/30 text-sec px-1.5 py-0.2 text-[10px] font-bold tabular-nums">
                {activeFilterCount}
              </span>
            )}
          </button>
        </div>

        {/* Active filter removable pills - rendered ONLY when filters or errors exist */}
        {hasRemovableChips && (
          <div className="flex flex-wrap items-center gap-1 pt-0.5 anim-fade">
            {result.chips.map((chip) => (
              <button
                key={chip.token}
                type="button"
                className="chip chip-active inline-flex items-center gap-1 !px-2 !py-0.5 !text-[11px] group cursor-pointer"
                onClick={() => {
                  sound.play("toggle");
                  onQuery(removeToken(query, chip.token));
                }}
                title={`Rimuovi filtro: ${chip.label}`}
                aria-label={`Rimuovi filtro ${chip.label}`}
              >
                <span>{chip.label}</span>
                <X className="h-3 w-3 text-muted group-hover:text-danger transition-colors" />
              </button>
            ))}
            {result.errors.map((e) => (
              <span key={e} className="chip !px-2 !py-0.5 !text-[11px] text-danger border-danger/40 bg-danger/10">
                ? {e}
              </span>
            ))}
          </div>
        )}
      </div>

      <div className="relative min-h-0 flex-1">
        {view === "list" ? (
          <CardList
            ids={result.ids}
            getCard={getCard}
            requestCards={requestCards}
            version={version}
            onAdd={onAdd}
            onOpen={(id) => onOpen(id, result.ids)}
            onHover={onHover}
            onHoverEnd={onHoverEnd}
            dragFrom="catalog"
            badgesFor={listBadges}
            favorites={favorites}
            onFavorite={toggleFavorite}
            selectedId={selectedId}
            onSelect={onSelect}
          />
        ) : (
          <CardGrid
            ids={result.ids}
            getCard={getCard}
            requestCards={requestCards}
            version={version}
            selected={selectedId ?? null}
            onSelect={(id) => {
              sound.play("select");
              onSelect?.(id);
            }}
            onOpen={(id) => onOpen(id, result.ids)}
            onHover={onHover}
            onHoverEnd={onHoverEnd}
            onContext={(id, el, shift) => onAdd(id, el, shift)}
            dragFrom="catalog"
            minTile={TILE[view] ?? TILE.m}
            favorites={favorites}
            onFavorite={toggleFavorite}
            badgeFor={(id) => {
              const n = copiesInDeck(deck, id);
              const card = getCard(id);
              const limit = card ? copyLimit(card, deck.format) : 3;
              if (limit === 3) return n ? { text: `×${n}` } : undefined;
              const tone = limit === 0 ? "ban" : "limit";
              return { text: n ? `×${n}/${limit}` : limit === 0 ? "✕" : String(limit), tone };
            }}
          />
        )}
        {filters && (
          <div className="absolute inset-0 z-10 overflow-y-auto border-t border-border bg-surface anim-fade">
            <FilterPanel query={query} onChange={onQuery} facets={facets} />
          </div>
        )}
        {removing && (
          <div className="pointer-events-none absolute inset-x-3 bottom-3 rounded-xl border border-danger/60 bg-surface/90 p-3 text-center text-xs text-danger anim-fade">
            Rilascia qui per togliere la carta dal deck
          </div>
        )}
      </div>
    </aside>
  );
}
