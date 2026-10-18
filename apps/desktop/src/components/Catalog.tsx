import { useEffect, useRef, useState } from "react";
import { api, type Card, type Facets, type SearchResult } from "../lib/api";
import type { Deck } from "../lib/api";
import { copiesInDeck } from "../lib/deck";
import { useDrag } from "../lib/drag";
import { removeToken } from "../lib/query";
import { sound } from "../lib/sound";
import { CardGrid } from "./CardGrid";
import { EmblemIcon } from "./Emblems";
import { FilterPanel } from "./FilterPanel";

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
}

/**
 * The card catalog column of the deck editor. Dropping deck cards here
 * removes them from the deck.
 */
export function Catalog({ deck, getCard, requestCards, version, onAdd, onOpen, onHover, onHoverEnd, query, onQuery }: Props) {
  const [result, setResult] = useState<SearchResult>({ ids: [], chips: [], errors: [] });
  const [searching, setSearching] = useState(false);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [filters, setFilters] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const drag = useDrag();
  const removing = drag && drag.source.from !== "catalog";

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
  }, [query, requestCards]);

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
      <div className="flex flex-col gap-2 px-3 pt-3 pb-2">
        <div className="flex items-center justify-between">
          <p className="display text-sm font-bold tracking-[0.12em] uppercase">Catalogo</p>
          <span className="flex items-center gap-1.5 text-xs text-muted tabular-nums">
            {searching && <EmblemIcon emblem="gear" className="h-3 w-3 animate-[spin-slow_1.6s_linear_infinite]" />}
            {result.ids.length.toLocaleString("it-IT")} carte
          </span>
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
        {(result.chips.length > 0 || result.errors.length > 0) && (
          <div className="flex flex-wrap gap-1">
            {result.chips.map((chip) => (
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
        <CardGrid
          ids={result.ids}
          getCard={getCard}
          requestCards={requestCards}
          version={version}
          selected={null}
          onSelect={() => sound.play("select")}
          onOpen={(id) => onOpen(id, result.ids)}
          onHover={onHover}
          onHoverEnd={onHoverEnd}
          onContext={(id, el, shift) => onAdd(id, el, shift)}
          dragFrom="catalog"
          minTile={92}
          badgeFor={(id) => {
            const n = copiesInDeck(deck, id);
            return n ? { text: `×${n}` } : undefined;
          }}
        />
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
      <p className="border-t border-border px-3 py-2 text-[10px] leading-relaxed text-faint">
        Trascina nel deck · clic destro: aggiungi · Maiusc+clic destro: Side · doppio clic: lente
      </p>
    </aside>
  );
}
