import { useMemo, useState } from "react";
import type { Card, Deck, DeckLinks } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { LinksCardDetail } from "./LinksCardDetail";
import { LinksCircle } from "./LinksCircle";
import { LinksColumns } from "./LinksColumns";
import { LinksFilters } from "./LinksFilters";
import { LinksStats } from "./LinksStats";
import type { LinksFilterState, LinksLayoutMode } from "./types";

interface Props {
  deck: Deck;
  data: DeckLinks | null;
  loading: boolean;
  progress: { done: number; total: number } | null;
  error: string | null;
  onReload: () => void;
  getCard: (code: number) => Card | undefined;
  onGoCombo?: (cardId: number) => void;
  hand?: "first" | "second";
}

const DEFAULT_FILTERS: LinksFilterState = {
  kind: "all",
  zone: "all",
  onlyUsed: false,
  searchQuery: "",
};

export function LinksTab({
  deck,
  data,
  loading,
  progress,
  error,
  onReload,
  getCard,
  onGoCombo,
  hand = "first",
}: Props) {
  const [filters, setFilters] = useState<LinksFilterState>(DEFAULT_FILTERS);
  const [layout, setLayout] = useState<LinksLayoutMode>("circle");
  const [selectedCardCode, setSelectedCardCode] = useState<number | null>(null);
  const [hoveredCardCode, setHoveredCardCode] = useState<number | null>(null);
  const [sampleHand, setSampleHand] = useState<number[] | null>(null);

  const selectedCard = data?.cards.find((c) => c.code === selectedCardCode) ?? null;

  // Draw or clear sample hand overlay (5 cards going first, 6 going second)
  const drawSampleHand = () => {
    if (!deck.main || deck.main.length === 0) return;
    sound.play("page");
    const count = hand === "second" ? 6 : 5;
    const array = [...deck.main];
    for (let i = array.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [array[i], array[j]] = [array[j], array[i]];
    }
    setSampleHand(array.slice(0, count));
  };

  const clearSampleHand = () => {
    sound.play("close");
    setSampleHand(null);
  };

  const sampleHandCodes = useMemo(() => (sampleHand ? new Set(sampleHand) : null), [sampleHand]);

  // Find all cards reachable from current sample hand in any number of steps
  // Traverses only deck and extra deck target zones. Reached set excludes cards already in sample hand.
  const reachableFromHand = useMemo(() => {
    if (!sampleHand || !data) return null;
    const handSet = new Set<number>(sampleHand);
    const reached = new Set<number>();
    const queue = [...sampleHand];
    while (queue.length > 0) {
      const curr = queue.shift()!;
      for (const l of data.links) {
        if (l.from === curr && (l.zone === "deck" || l.zone === "extra")) {
          if (!handSet.has(l.to) && !reached.has(l.to)) {
            reached.add(l.to);
            queue.push(l.to);
          }
        }
      }
    }
    return reached;
  }, [sampleHand, data]);

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
            Come le carte del deck conducono l'una all'altra: ricerche, evocazioni, invio al Cimitero, recuperi e materiali.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {data && (
            <span className="text-[11px] text-faint tabular-nums hidden sm:inline">
              Mani campionate: {data.hands} · Linee: {data.lines}
            </span>
          )}
          {data && (
            sampleHand ? (
              <button
                type="button"
                onClick={clearSampleHand}
                className="chip chip-active !py-1 !px-2 text-xs flex items-center gap-1.5"
                title="Rimuovi la mano di prova sovrapposta"
              >
                <span>✕ Chiudi mano ({sampleHand.length} carte)</span>
              </button>
            ) : (
              <button
                type="button"
                onClick={drawSampleHand}
                className="btn btn-ghost !py-1 !px-2 text-xs flex items-center gap-1.5"
                title="Sovrapponi una mano di prova casuale e illumina ciò che raggiunge"
              >
                <span>⚡ Sovrapponi mano ({hand === "second" ? "6" : "5"} carte)</span>
              </button>
            )
          )}
          <button
            type="button"
            onClick={() => {
              sound.play("select");
              onReload();
            }}
            disabled={loading}
            className="btn btn-ghost !py-1 !px-2.5 text-xs flex items-center gap-1.5"
            title="Ricalcola la mappa dei collegamenti tramite il motore"
          >
            {loading ? "Analisi in corso…" : "↻ Ricalcola"}
          </button>
        </div>
      </div>

      {/* Hand overlay summary banner */}
      {sampleHand && (
        <div className="rounded-lg border border-sec/40 bg-sec/10 p-2.5 flex flex-wrap items-center justify-between gap-2 anim-fade">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="font-semibold text-sec text-xs">Mano di prova:</span>
            {sampleHand.map((code, idx) => (
              <button
                key={`${code}-${idx}`}
                type="button"
                onClick={() => {
                  sound.play("toggle");
                  setSelectedCardCode(code);
                }}
                className={`chip !py-0.5 !px-1.5 text-[11px] font-medium ${selectedCardCode === code ? "chip-active" : ""}`}
              >
                {getCard(code)?.name ?? code}
              </button>
            ))}
          </div>
          <span className="text-[11px] text-muted">
            Raggiunge <b>{reachableFromHand ? reachableFromHand.size : 0}</b> carte del mazzo
          </span>
        </div>
      )}

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
            links={data.links}
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
                  sampleHandCodes={sampleHandCodes}
                  reachedFromHandCodes={reachableFromHand}
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
                  sampleHandCodes={sampleHandCodes}
                  reachedFromHandCodes={reachableFromHand}
                  onSelectCard={(code) => setSelectedCardCode(code)}
                />
              )}
            </div>

            {/* Right card inspector panel */}
            <div className="sticky top-4">
              <LinksCardDetail
                key={selectedCard?.code ?? "none"}
                card={selectedCard}
                allCards={data.cards}
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
