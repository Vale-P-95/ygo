import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CardDetails } from "../components/CardDetails";
import { CardGrid } from "../components/CardGrid";
import { CardPreview } from "../components/CardPreview";
import { CardViewer } from "../components/CardViewer";
import { DataSetup } from "../components/DataSetup";
import { EmblemIcon } from "../components/Emblems";
import { FilterPanel } from "../components/FilterPanel";
import { SectionHeader } from "../components/SectionHeader";
import { api, cardImage, isDesktop, type DataStatus, type Facets, type SearchResult } from "../lib/api";
import { useCardCache } from "../lib/cards";
import { removeToken } from "../lib/query";
import { useSettings } from "../lib/settings";
import { sound } from "../lib/sound";
import { burstFrom } from "../lib/sparks";
import { sectionById } from "../sections";

const EXAMPLES = ['arch:invoked type:fusion', '"negate the activation" attr:light', "type:tuner lv<=3 format:md", "tipo:magia rapida"];

export function DeckBuilder({ status, onStatus }: { status: DataStatus | null; onStatus: (s: DataStatus) => void }) {
  const section = sectionById("deck-builder");
  const ready = !!status?.cards;
  const settings = useSettings();
  const [query, setQuery] = useState("sort:popular");
  const [result, setResult] = useState<SearchResult>({ ids: [], chips: [], errors: [] });
  const [searching, setSearching] = useState(false);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [viewer, setViewer] = useState<number | null>(null);
  const [hover, setHover] = useState<{ id: number; rect: DOMRect } | null>(null);
  const [filtersOpen, setFiltersOpen] = useState(() => innerWidth >= 1500);
  const hoverTimer = useRef<number>(undefined);
  const cards = useCardCache();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!ready) return;
    api.facets().then(setFacets).catch(() => setFacets(null));
  }, [ready]);

  useEffect(() => {
    if (!ready) return;
    setSearching(true);
    const timer = window.setTimeout(() => {
      api
        .search(query)
        .then((r) => {
          setResult(r);
          cards.request(r.ids.slice(0, 60));
        })
        .catch(() => sound.play("error"))
        .finally(() => setSearching(false));
    }, 160);
    return () => window.clearTimeout(timer);
  }, [query, ready, cards.request]);

  useEffect(() => {
    const focus = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    addEventListener("keydown", focus);
    return () => removeEventListener("keydown", focus);
  }, []);

  const select = useCallback((id: number, el: HTMLElement) => {
    sound.play("select");
    burstFrom(el, 14, 0.8);
    setSelected(id);
  }, []);

  const startHover = useCallback(
    (id: number, el: HTMLElement) => {
      sound.play("hover");
      window.clearTimeout(hoverTimer.current);
      if (!settings.hoverPreview) return;
      hoverTimer.current = window.setTimeout(() => setHover({ id, rect: el.getBoundingClientRect() }), 380);
    },
    [settings.hoverPreview],
  );
  const endHover = useCallback(() => {
    window.clearTimeout(hoverTimer.current);
    setHover(null);
  }, []);

  const open = useCallback((id: number) => {
    window.clearTimeout(hoverTimer.current);
    setHover(null);
    setSelected(id);
    setViewer(id);
  }, []);

  const searchFor = useCallback((q: string) => {
    sound.play("page");
    setViewer(null);
    setQuery(q);
  }, []);

  const viewerIndex = viewer == null ? -1 : result.ids.indexOf(viewer);
  const step = (delta: number) => {
    const next = result.ids[viewerIndex + delta];
    if (next == null) return;
    cards.request([next, ...result.ids.slice(viewerIndex + delta + 1, viewerIndex + delta + 4)]);
    setViewer(next);
    setSelected(next);
  };

  const selectedCard = selected != null ? cards.get(selected) : undefined;
  const viewerCard = viewer != null ? cards.get(viewer) : undefined;
  const hoverCard = hover ? cards.get(hover.id) : undefined;
  const count = useMemo(() => result.ids.length.toLocaleString("it-IT"), [result.ids.length]);

  if (!ready) {
    return (
      <div className="flex h-full flex-col">
        <SectionHeader section={section} artReady={false} />
        <div className="flex-1 overflow-y-auto px-10">
          <DataSetup onReady={onStatus} />
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section}>
        <span className="chip chip-active tabular-nums">
          {count} carte {searching && <EmblemIcon emblem="gear" className="h-3 w-3 animate-[spin-slow_1.6s_linear_infinite]" />}
        </span>
      </SectionHeader>

      <div className={`grid min-h-0 flex-1 ${filtersOpen ? "grid-cols-[240px_minmax(0,1fr)_320px]" : "grid-cols-[minmax(0,1fr)_320px]"}`}>
        {filtersOpen && (
          <aside className="min-h-0 overflow-y-auto border-r border-border bg-surface/60 anim-fade">
            <FilterPanel query={query} onChange={setQuery} facets={facets} />
          </aside>
        )}

        <section className="bg-blueprint flex min-h-0 flex-col">
          <div className="flex flex-col gap-2 px-4 pt-4 pb-2">
            <div className="flex gap-2">
              <button
                type="button"
                className={`btn shrink-0 ${filtersOpen ? "btn-molten" : "btn-ghost"}`}
                onClick={() => {
                  sound.play("toggle");
                  setFiltersOpen((v) => !v);
                }}
                aria-expanded={filtersOpen}
              >
                Filtri{result.chips.length > 0 && <span className="rounded-full bg-black/25 px-1.5 text-[11px]">{result.chips.length}</span>}
              </button>
              <label className="relative block flex-1">
                <span className="pointer-events-none absolute top-1/2 left-3.5 -translate-y-1/2 text-faint">⌕</span>
                <input
                  ref={inputRef}
                  className="field !rounded-xl !py-2.5 !pl-9 text-[15px] shadow-inner"
                  placeholder="Cerca per nome o testo, oppure filtra: attr:dark type:fusion atk>=2500 …"
                  value={query}
                  spellCheck={false}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </label>
            </div>
            <div className="flex min-h-7 flex-wrap items-center gap-1.5">
              {result.chips.map((chip) => (
                <button
                  key={chip.token}
                  type="button"
                  className="chip chip-active group"
                  title="Rimuovi filtro"
                  onClick={() => {
                    sound.play("toggle");
                    setQuery(removeToken(query, chip.token));
                  }}
                >
                  {chip.label}
                  <span className="text-muted group-hover:text-text">✕</span>
                </button>
              ))}
              {result.errors.map((e) => (
                <span key={e} className="chip border-danger/60 text-danger" title="Filtro non riconosciuto">
                  ? {e}
                </span>
              ))}
              {result.chips.length === 0 && result.errors.length === 0 && (
                <span className="text-xs text-faint">
                  Prova:{" "}
                  {EXAMPLES.map((ex) => (
                    <button key={ex} type="button" className="mr-2 font-mono text-muted hover:text-sec" onClick={() => searchFor(ex)}>
                      {ex}
                    </button>
                  ))}
                </span>
              )}
            </div>
          </div>
          {status?.engineError && (
            <p className="mx-4 mb-2 rounded-lg border border-warn/40 bg-warn/10 px-3 py-2 text-xs text-warn">
              Script del motore non scaricati: servono dal Combo Lab in poi. Riprova da Impostazioni → Dati. ({status.engineError})
            </p>
          )}
          <div className="min-h-0 flex-1">
            {result.ids.length === 0 && !searching ? (
              <div className="grid h-full place-items-center text-center text-muted anim-fade">
                <div>
                  <EmblemIcon emblem="anvil" className="mx-auto mb-3 h-10 w-10 text-faint" />
                  <p className="display text-lg">Nessuna carta nella forgia</p>
                  <p className="text-sm">Allenta qualche filtro o cerca un'altra parola.</p>
                </div>
              </div>
            ) : (
              <CardGrid
                ids={result.ids}
                getCard={cards.get}
                requestCards={cards.request}
                version={cards.version}
                selected={selected}
                onSelect={select}
                onOpen={open}
                onHover={startHover}
                onHoverEnd={endHover}
              />
            )}
          </div>
        </section>

        <aside className="flex min-h-0 flex-col gap-4 overflow-y-auto border-l border-border bg-surface/60 p-5">
          {selectedCard ? (
            <div key={selectedCard.id} className="flex flex-col gap-4 anim-rise">
              <button type="button" className="group relative mx-auto w-44" onClick={() => open(selectedCard.id)} title="Apri nella lente">
                <img src={cardImage(selectedCard.id, "full")} alt={selectedCard.name} className="w-full rounded-md shadow-[0_18px_40px_-14px_black] transition-transform group-hover:scale-[1.03]" />
                <span className="chip absolute -bottom-3 left-1/2 -translate-x-1/2 bg-surface opacity-0 transition-opacity group-hover:opacity-100">Apri nella lente</span>
              </button>
              <CardDetails card={selectedCard} onSearch={searchFor} />
            </div>
          ) : (
            <div className="flex flex-col gap-3 text-sm text-muted">
              <p className="display text-lg text-text">Il banco del fabbro</p>
              <p>Clicca una carta per esaminarla qui. Doppio clic per aprirla nella lente, con zoom e lente d'ingrandimento.</p>
              <p className="text-xs text-faint">Ctrl+F porta il cursore nella ricerca. Ogni filtro scelto a sinistra diventa testo nella barra: puoi anche scriverlo a mano.</p>
            </div>
          )}
          <div className="mt-auto rounded-xl border border-dashed border-border-strong p-4 text-center text-xs text-faint">
            <EmblemIcon emblem="anvil" className="mx-auto mb-2 h-5 w-5" />
            Qui forgerai il tuo deck: arriva nella Fase 2.
          </div>
        </aside>
      </div>

      {hover && hoverCard && !viewer && <CardPreview card={hoverCard} anchor={hover.rect} />}
      {viewerCard && (
        <CardViewer
          card={viewerCard}
          position={`${viewerIndex + 1} / ${count}`}
          onClose={() => setViewer(null)}
          onPrev={viewerIndex > 0 ? () => step(-1) : undefined}
          onNext={viewerIndex < result.ids.length - 1 ? () => step(1) : undefined}
          onSearch={searchFor}
        />
      )}
      {!isDesktop && <p className="p-4 text-warn">Anteprima nel browser: i dati sono disponibili solo nell'app desktop.</p>}
    </div>
  );
}
