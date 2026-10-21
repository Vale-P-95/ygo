import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Card, Deck, DeckLinks, DeckSummary, LinksComparison } from "../../../lib/api";
import { api } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { toast } from "../../../lib/toast";
import { LinksCardDetail } from "./LinksCardDetail";
import { LinksCircle } from "./LinksCircle";
import { LinksColumns } from "./LinksColumns";
import { LinksCompare } from "./LinksCompare";
import { LinksFilters } from "./LinksFilters";
import { LinksStats } from "./LinksStats";
import type { LinksFilterState, LinksLayoutMode } from "./types";

interface Props {
  deck: Deck;
  decks?: DeckSummary[];
  data: DeckLinks | null;
  loading: boolean;
  progress: { done: number; total: number } | null;
  error: string | null;
  onReload: () => void;
  getCard: (code: number) => Card | undefined;
  requestCards?: (codes: number[]) => void;
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
  decks = [],
  data,
  loading,
  progress,
  error,
  onReload,
  getCard,
  requestCards,
  onGoCombo,
  hand = "first",
}: Props) {
  const [filters, setFilters] = useState<LinksFilterState>(DEFAULT_FILTERS);
  const [layout, setLayout] = useState<LinksLayoutMode>("circle");
  const [selectedCardCode, setSelectedCardCode] = useState<number | null>(null);
  const [hoveredCardCode, setHoveredCardCode] = useState<number | null>(null);
  const [sampleHand, setSampleHand] = useState<number[] | null>(null);

  // Comparison mode states
  const [isCompareOpen, setIsCompareOpen] = useState(false);
  const [compareDeckId, setCompareDeckId] = useState<number | null>(null);
  const [comparison, setComparison] = useState<LinksComparison | null>(null);
  const [compareLoading, setCompareLoading] = useState(false);
  const [compareError, setCompareError] = useState<string | null>(null);
  const [showLostLinks, setShowLostLinks] = useState(true);

  const compareReqIdRef = useRef(0);

  // Reset comparison when current deck changes
  useEffect(() => {
    setCompareDeckId(null);
    setComparison(null);
    setCompareError(null);
    setCompareLoading(false);
  }, [deck.id]);

  const handleSelectCompareDeck = useCallback(
    async (targetId: number | null) => {
      setCompareDeckId(targetId);
      setCompareError(null);
      if (targetId == null) {
        setComparison(null);
        setCompareLoading(false);
        return;
      }
      if (deck.id == null) return;

      const reqId = ++compareReqIdRef.current;
      setCompareLoading(true);
      try {
        const res = await api.deckLinksCompare(deck.id, targetId);
        if (compareReqIdRef.current === reqId) {
          setComparison(res);
          const cardCodes = [
            ...res.added,
            ...res.removed,
            ...res.changed.map((c) => c.code),
            ...res.after.cards.map((c) => c.code),
          ];
          requestCards?.(cardCodes);
        }
      } catch (err) {
        if (compareReqIdRef.current === reqId) {
          const msg = String(err);
          setCompareError(msg);
          toast(`Errore nel confronto dei deck: ${msg}`, "error");
        }
      } finally {
        if (compareReqIdRef.current === reqId) {
          setCompareLoading(false);
        }
      }
    },
    [deck.id, requestCards],
  );

  // Active data to display: if comparison is active, use comparison.after
  const activeData = comparison ? comparison.after : data;

  const selectedCard = activeData?.cards.find((c) => c.code === selectedCardCode) ?? null;

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
    if (!sampleHand || !activeData) return null;
    const handSet = new Set<number>(sampleHand);
    const reached = new Set<number>();
    const queue = [...sampleHand];
    while (queue.length > 0) {
      const curr = queue.shift()!;
      for (const l of activeData.links) {
        if (l.from === curr && (l.zone === "deck" || l.zone === "extra")) {
          if (!handSet.has(l.to) && !reached.has(l.to)) {
            reached.add(l.to);
            queue.push(l.to);
          }
        }
      }
    }
    return reached;
  }, [sampleHand, activeData]);

  return (
    <div className="flex flex-col gap-4 text-xs anim-fade">
      {/* Header and top info */}
      <div className="flex flex-wrap items-center justify-between gap-3 px-1">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <h3 className="text-base font-semibold text-text">Mappa dei collegamenti</h3>
            {activeData && (
              <span className="chip !py-0.5 !px-2 text-[11px] text-muted font-medium">
                {activeData.cards.length} carte · {activeData.links.length} collegamenti
              </span>
            )}
            {comparison && (
              <span className="chip !py-0.5 !px-2 text-[11px] text-sec font-semibold border-sec/40">
                Confronto attivo
              </span>
            )}
          </div>
          <p className="text-[11px] text-muted">
            Come le carte del deck conducono l'una all'altra: ricerche, evocazioni, invio al Cimitero, recuperi e materiali.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {activeData && (
            <span className="text-[11px] text-faint tabular-nums hidden sm:inline">
              Mani campionate: {activeData.hands} · Linee: {activeData.lines}
            </span>
          )}

          {/* Toggle comparison view button */}
          {decks.length > 1 && (
            <button
              type="button"
              onClick={() => {
                sound.play("toggle");
                setIsCompareOpen((open) => {
                  if (open) {
                    setCompareDeckId(null);
                    setComparison(null);
                  }
                  return !open;
                });
              }}
              className={`chip !py-1 !px-2.5 text-xs flex items-center gap-1.5 transition-colors ${
                isCompareOpen ? "chip-active" : "!bg-surface-2 hover:!bg-surface-3 text-text"
              }`}
              title="Confronta i collegamenti con un altro deck della Bottega"
            >
              <span>⚖ Confronta versioni</span>
            </button>
          )}

          {activeData &&
            (sampleHand ? (
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
            ))}

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

      {/* Comparison Drawer / Panel */}
      {isCompareOpen && (
        <LinksCompare
          decks={decks}
          currentDeckId={deck.id!}
          compareDeckId={compareDeckId}
          onSelectCompareDeckId={handleSelectCompareDeck}
          comparison={comparison}
          loading={compareLoading}
          error={compareError}
          showLostLinks={showLostLinks}
          onToggleShowLostLinks={setShowLostLinks}
          getCard={getCard}
          selectedCardCode={selectedCardCode}
          onSelectCard={(code) => setSelectedCardCode(code)}
        />
      )}

      {/* Progressive loading banner when data is already visible */}
      {loading && activeData && (
        <div className="rounded-lg border border-sec/40 bg-sec/10 p-2.5 flex flex-col gap-1.5 anim-fade">
          <div className="flex items-center justify-between text-xs">
            <span className="font-semibold text-sec flex items-center gap-1.5">
              <span className="animate-spin text-[11px]">↻</span>
              {progress?.done === 0
                ? "Bozza immediata dal testo delle carte · Analisi delle mani del motore in corso…"
                : progress
                  ? `Aggiornamento progressivo: ${progress.done} / ${progress.total} mani analizzate…`
                  : "Calcolo della mappa dei collegamenti in corso…"}
            </span>
            {progress && progress.total > 0 && (
              <span className="text-[11px] text-muted tabular-nums">
                {Math.round((progress.done / progress.total) * 100)}%
              </span>
            )}
          </div>
          <div className="h-1.5 w-full rounded-full bg-surface-3 overflow-hidden">
            <div
              className="h-full bg-[var(--sec)] transition-all duration-300"
              style={{
                width: progress && progress.total > 0 ? `${Math.max(5, (progress.done / progress.total) * 100)}%` : "20%",
              }}
            />
          </div>
        </div>
      )}

      {/* Initial loading state before any map data arrives */}
      {loading && !activeData && (
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

      {/* Error state */}
      {error && !loading && (
        <div className="rounded-lg border border-danger/40 bg-danger/10 p-4 text-xs text-danger">
          <p className="font-semibold">Impossibile calcolare la mappa dei collegamenti</p>
          <p className="text-[11px] mt-1">{error}</p>
        </div>
      )}

      {/* Loaded data view */}
      {activeData && (
        <div className="flex flex-col gap-3.5">
          {/* Summary ribbon */}
          <LinksStats
            cards={activeData.cards}
            links={activeData.links}
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
            links={activeData.links}
          />

          {/* Visualization + Card Detail Panel */}
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] xl:grid-cols-[1fr_360px] gap-4 items-start">
            {/* Visual graph / columns */}
            <div className="lab-card flex flex-col items-center justify-center p-3 overflow-hidden min-h-[520px]">
              {layout === "circle" ? (
                <LinksCircle
                  cards={activeData.cards}
                  links={activeData.links}
                  getCard={getCard}
                  filters={filters}
                  activeCardCode={selectedCardCode}
                  hoveredCardCode={hoveredCardCode}
                  sampleHandCodes={sampleHandCodes}
                  reachedFromHandCodes={reachableFromHand}
                  comparison={comparison}
                  showLostLinks={showLostLinks}
                  onSelectCard={(code) => setSelectedCardCode(code)}
                  onHoverCard={(code) => setHoveredCardCode(code)}
                />
              ) : (
                <LinksColumns
                  cards={activeData.cards}
                  links={activeData.links}
                  getCard={getCard}
                  filters={filters}
                  activeCardCode={selectedCardCode}
                  sampleHandCodes={sampleHandCodes}
                  reachedFromHandCodes={reachableFromHand}
                  comparison={comparison}
                  onSelectCard={(code) => setSelectedCardCode(code)}
                />
              )}
            </div>

            {/* Right card inspector panel */}
            <div className="sticky top-4">
              <LinksCardDetail
                key={selectedCard?.code ?? "none"}
                card={selectedCard}
                allCards={activeData.cards}
                getCard={getCard}
                links={activeData.links}
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
