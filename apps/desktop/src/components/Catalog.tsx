import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
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

const VIEWS: { view: CatalogView; label: string; icon: string }[] = [
  { view: "l", label: "Carte grandi", icon: "M1 2h4v12H1zM6 2h4v12H6zM11 2h4v12h-4z" },
  { view: "m", label: "Carte medie", icon: "M1 2h4v5H1zM6 2h4v5H6zM11 2h4v5h-4zM1 9h4v5H1zM6 9h4v5H6zM11 9h4v5h-4z" },
  { view: "s", label: "Carte piccole", icon: "M1 1h3v4H1zM5 1h3v4H5zM9 1h3v4H9zM13 1h2v4h-2zM1 6h3v4H1zM5 6h3v4H5zM9 6h3v4H9zM13 6h2v4h-2zM1 11h3v4H1zM5 11h3v4H5zM9 11h3v4H9zM13 11h2v4h-2z" },
  { view: "list", label: "Elenco", icon: "M1 2h3v3H1zM6 3h9v1H6zM1 7h3v3H1zM6 8h9v1H6zM1 12h3v3H1zM6 13h9v1H6z" },
];

/** Quick filters: hand-picked groups (crates/ygo-data/src/tags.rs) and favourites. */
const QUICK: { token: string; label: string }[] = [
  { token: "tag:handtrap", label: "Handtrap" },
  { token: "tag:staple", label: "Staple" },
  { token: "tag:breaker", label: "Board breaker" },
  { token: "tag:fav", label: "★ Preferite" },
];

const isQuick = (token: string) => QUICK.some((q) => q.token === token.toLowerCase());

/** Saved searches: apply, save the current one, delete. */
function SavedSearches({ query, onQuery }: { query: string; onQuery: (q: string) => void }) {
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
    <div ref={ref} className="relative shrink-0">
      <button
        type="button"
        className={`btn h-full !px-2.5 ${open ? "btn-molten" : "btn-ghost"}`}
        aria-expanded={open}
        title="Ricerche salvate"
        aria-label="Ricerche salvate"
        onClick={() => {
          sound.play(open ? "close" : "open");
          setOpen((v) => !v);
        }}
      >
        <svg viewBox="0 0 16 16" className="h-4 w-4" fill="currentColor" aria-hidden="true">
          <path d="M3.5 1.5h9a1 1 0 0 1 1 1v12.2a.4.4 0 0 1-.64.32L8 11.6l-4.86 3.42a.4.4 0 0 1-.64-.32V2.5a1 1 0 0 1 1-1z" />
        </svg>
      </button>
      <Popover open={open} align="right" width={320}>
        <form
          className="flex gap-1.5 p-1"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <input className="field !py-1 text-xs" placeholder="Nome della ricerca attuale" value={name} onChange={(e) => setName(e.target.value)} />
          <button type="submit" className="btn btn-molten shrink-0 !px-2.5 !py-1 text-xs" disabled={!name.trim() || !query.trim()}>
            Salva
          </button>
        </form>
        {saved.length === 0 && <p className="px-2 py-2 text-xs text-faint">Nessuna ricerca salvata: scrivi una ricerca, dalle un nome e premi Salva.</p>}
        {saved.length > 0 && <div className="my-1.5 border-t border-border" />}
        {saved.map((s) => (
          <div key={s.name} className="group flex items-center gap-1 rounded-lg hover:bg-surface-3">
            <button
              type="button"
              className="min-w-0 flex-1 px-2 py-1.5 text-left"
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
              className="px-2 text-faint opacity-0 group-hover:opacity-100 hover:text-danger"
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
export function Catalog({ deck, getCard, requestCards, version, onAdd, onOpen, onHover, onHoverEnd, query, onQuery, selectedId, onSelect }: Props) {
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

  return (
    <aside
      data-dropzone="catalog"
      className="relative flex min-h-0 flex-col border-l border-border bg-surface/70 transition-colors"
      style={{ background: drag?.over === "catalog" && removing ? "color-mix(in oklab, var(--df-danger) 12%, var(--df-surface))" : undefined }}
    >
      <div
        className="group absolute inset-y-0 -left-[5px] z-20 w-[9px] cursor-col-resize"
        onPointerDown={(e) => startResize(e, catalogWidth)}
        onDoubleClick={() => updateSettings({ catalogWidth: 392 })}
        title="Trascina per allargare il catalogo (doppio clic: larghezza iniziale)"
      >
        <span className="absolute inset-y-0 left-1 w-px bg-transparent transition-colors group-hover:bg-sec" />
      </div>
      <div className="flex flex-col gap-2 px-3 pt-3 pb-2">
        <div className="flex items-center gap-2">
          <p className="display text-sm font-bold tracking-[0.12em] uppercase">Catalogo</p>
          <HelpPopover title="Guida al catalogo e scorciatoie" tooltip="Guida del catalogo" align="left">
            <div className="flex flex-col gap-2 text-xs text-text/90">
              <p className="font-semibold text-text">Azioni rapide sulle carte</p>
              <ul className="list-disc pl-4 space-y-1 text-muted">
                <li><strong>Clic singolo:</strong> fissa la carta nell'Ispettore.</li>
                <li><strong>Doppio clic:</strong> visore a schermo intero.</li>
                <li><strong>Clic destro:</strong> aggiungi 1 copia al Main (o Extra) Deck.</li>
                <li><strong>Maiusc + clic destro:</strong> aggiungi 1 copia al Side Deck.</li>
                <li><strong>Trascina:</strong> sposta la carta nella zona del deck desiderata.</li>
              </ul>
              <div className="border-t border-border/60 pt-1 text-muted">
                <p>Trascina le carte del deck qui sopra per toglierle dal deck.</p>
              </div>
            </div>
          </HelpPopover>
          <span className="flex items-center gap-1.5 text-xs text-muted tabular-nums">
            {searching && <EmblemIcon emblem="gear" className="h-3 w-3 animate-[spin-slow_1.6s_linear_infinite]" />}
            {result.ids.length.toLocaleString("it-IT")} carte
          </span>
          <div className="ml-auto flex gap-0.5 rounded-lg border border-border bg-bg-2 p-0.5" role="radiogroup" aria-label="Vista del catalogo">
            {VIEWS.map((v) => (
              <button
                key={v.view}
                type="button"
                role="radio"
                aria-checked={view === v.view}
                title={v.label}
                aria-label={v.label}
                onClick={() => {
                  sound.play("toggle");
                  updateSettings({ catalogView: v.view });
                }}
                className={`grid h-6 w-7 place-items-center rounded-md ${view === v.view ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
              >
                <svg viewBox="0 0 16 16" className="h-3.5 w-3.5" fill="currentColor" aria-hidden="true">
                  <path d={v.icon} />
                </svg>
              </button>
            ))}
          </div>
        </div>
        <div className="flex gap-2">
          <label className="relative block flex-1">
            <span className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-faint">⌕</span>
            <input
              ref={inputRef}
              className="field !rounded-lg !pl-8"
              placeholder="Cerca o filtra…"
              value={query}
              spellCheck={false}
              onChange={(e) => onQuery(e.target.value)}
            />
          </label>
          <SavedSearches query={query} onQuery={onQuery} />
          <button
            type="button"
            className={`btn shrink-0 !px-3 ${filters ? "btn-molten" : "btn-ghost"}`}
            onClick={() => {
              sound.play(filters ? "close" : "open");
              setFilters((v) => !v);
            }}
            aria-expanded={filters}
          >
            Filtri{result.chips.length > 0 && <span className="rounded-full bg-black/25 px-1.5 text-[11px]">{result.chips.length}</span>}
          </button>
        </div>
        <div className="flex flex-wrap items-center gap-1">
          {QUICK.map((q) => {
            const on = hasToken(query, q.token);
            return (
              <button
                key={q.token}
                type="button"
                aria-pressed={on}
                className={`chip !px-2 !py-0.5 !text-[11px] ${on ? "chip-active" : ""}`}
                onClick={() => {
                  sound.play("toggle");
                  onQuery(toggleToken(query, q.token));
                }}
              >
                {q.label}
              </button>
            );
          })}
        </div>
        {(result.chips.some((c) => !isQuick(c.token)) || result.errors.length > 0) && (
          <div className="flex flex-wrap gap-1">
            {result.chips.filter((c) => !isQuick(c.token)).map((chip) => (
              <button
                key={chip.token}
                type="button"
                className="chip chip-active !px-2 !py-0.5 !text-[11px]"
                onClick={() => {
                  sound.play("toggle");
                  onQuery(removeToken(query, chip.token));
                }}
              >
                {chip.label} <span className="text-muted">✕</span>
              </button>
            ))}
            {result.errors.map((e) => (
              <span key={e} className="chip !px-2 !py-0.5 !text-[11px] text-danger">
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
