import { useCallback, useEffect, useState } from "react";
import { api, type Card, type Deck, type DeckLinks } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { LinksCardDetail } from "./LinksCardDetail";
import { LinksCircle } from "./LinksCircle";
import { LinksColumns } from "./LinksColumns";
import { LinksFilters } from "./LinksFilters";
import { LinksStats } from "./LinksStats";
import type { LinksFilterState, LinksLayoutMode } from "./types";

interface Props {
  deck: Deck;
  getCard: (code: number) => Card | undefined;
  requestCards: (codes: number[]) => void;
  onGoCombo?: (cardId: number) => void;
}

const DEFAULT_FILTERS: LinksFilterState = {
  kind: "all",
  zone: "all",
  onlyUsed: false,
  searchQuery: "",
};

export function LinksTab({ deck, getCard, requestCards, onGoCombo }: Props) {
  const [data, setData] = useState<DeckLinks | null>(null);
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [filters, setFilters] = useState<LinksFilterState>(DEFAULT_FILTERS);
  const [layout, setLayout] = useState<LinksLayoutMode>("circle");
  const [selectedCardCode, setSelectedCardCode] = useState<number | null>(null);
  const [hoveredCardCode, setHoveredCardCode] = useState<number | null>(null);

  const loadLinks = useCallback(
    async (deckId: number) => {
      setLoading(true);
      setError(null);
      setProgress(null);

      let unlisten: (() => void) | undefined;
      try {
        unlisten = await api.onLinksProgress((p) => {
          setProgress(p);
        });

        const res = await api.deckLinks(deckId);
        setData(res);

        // Preload card metadata for all cards in the map
        const codes = res.cards.map((c) => c.code);
        requestCards(codes);
      } catch (err) {
        setError(String(err));
      } finally {
        unlisten?.();
        setLoading(false);
        setProgress(null);
      }
    },
    [requestCards]
  );

  useEffect(() => {
    if (deck?.id != null) {
      void loadLinks(deck.id);
    }
  }, [deck?.id, loadLinks]);

  const selectedCard = data?.cards.find((c) => c.code === selectedCardCode) ?? null;

  return (
    <div className="flex flex-col gap-4 text-xs anim-fade">
      {/* Header and top info */}
      <div className="flex flex-wrap items-center justify-between gap-3 px-1">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <h3 className="text-base font-semibold text-text">Mappa dei collegamenti</h3>
            {data && (
              <span className="chip !py-0.5 !px-2 text-[11px] text-muted font-medium">
                {data.cards.length} carte · {data.links.length} collegamenti
              </span>
            )}
          </div>
          <p className="text-[11px] text-muted">
            Come le carte del deck conducono l'una all'altra: ricerche, evocazioni, invio al cimitero, recuperi e materiali.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {data && (
            <span className="text-[11px] text-faint tabular-nums hidden sm:inline">
              Mani campionate: {data.hands} · Linee: {data.lines}
            </span>
          )}
          <button
            type="button"
            onClick={() => {
              sound.play("select");
              if (deck?.id != null) void loadLinks(deck.id);
            }}
            disabled={loading}
            className="btn btn-ghost !py-1 !px-2.5 text-xs flex items-center gap-1.5"
            title="Ricalcola la mappa dei collegamenti tramite il motore"
          >
            {loading ? "Analisi in corso…" : "↻ Ricalcola"}
          </button>
        </div>
      </div>

      {/* Loading state with progress */}
      {loading && (
        <div className="lab-card flex flex-col items-center justify-center p-8 gap-3 anim-fade text-center">
          <div className="h-2 w-48 rounded-full bg-surface-3 overflow-hidden">
            <div
              className="h-full bg-[var(--sec)] transition-all duration-300"
              style={{
                width: progress && progress.total > 0 ? `${(progress.done / progress.total) * 100}%` : "40%",
              }}
            />
          </div>
          <div className="flex flex-col gap-1">
            <p className="font-medium text-text text-xs">
              {progress ? `Analisi delle mani del deck: ${progress.done} / ${progress.total}…` : "Lettura dei collegamenti tramite il motore EDOPro…"}
            </p>
            <p className="text-[11px] text-muted">
              Il motore gioca le mani del deck per rilevare le scelte offerte e la frequenza nelle combo migliori.
            </p>
          </div>
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <div className="rounded-lg border border-danger/40 bg-danger/10 p-4 text-xs text-danger">
          <p className="font-semibold">Impossibile calcolare la mappa dei collegamenti</p>
          <p className="text-[11px] mt-1">{error}</p>
        </div>
      )}

      {/* Loaded data view */}
      {data && !loading && (
        <div className="flex flex-col gap-3.5">
          {/* Summary ribbon */}
          <LinksStats
            cards={data.cards}
            getCard={getCard}
            selectedCode={selectedCardCode}
            onSelectCard={(code) => setSelectedCardCode(code)}
          />

          {/* Interactive Filters */}
          <LinksFilters
            filters={filters}
            onChange={setFilters}
            layout={layout}
            onLayoutChange={setLayout}
            links={data.links}
            
          />

          {/* Visualization + Card Detail Panel */}
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] xl:grid-cols-[1fr_360px] gap-4 items-start">
            {/* Visual graph / columns */}
            <div className="lab-card flex flex-col items-center justify-center p-3 overflow-hidden min-h-[520px]">
              {layout === "circle" ? (
                <LinksCircle
                  cards={data.cards}
                  links={data.links}
                  getCard={getCard}
                  filters={filters}
                  activeCardCode={selectedCardCode}
                  hoveredCardCode={hoveredCardCode}
                  onSelectCard={(code) => setSelectedCardCode(code)}
                  onHoverCard={(code) => setHoveredCardCode(code)}
                />
              ) : (
                <LinksColumns
                  cards={data.cards}
                  links={data.links}
                  getCard={getCard}
                  filters={filters}
                  activeCardCode={selectedCardCode}
                  onSelectCard={(code) => setSelectedCardCode(code)}
                />
              )}
            </div>

            {/* Right card inspector panel */}
            <div className="sticky top-4">
              <LinksCardDetail
                card={selectedCard}
                getCard={getCard}
                links={data.links}
                
                onSelectCard={(code) => setSelectedCardCode(code)}
                onGoCombo={onGoCombo}
                onClose={() => setSelectedCardCode(null)}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
