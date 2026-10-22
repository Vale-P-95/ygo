import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, type Card, type Deck, type DeckLinks, type DeckSummary, type LinksComparison } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { toast } from "../../../lib/toast";
import { LinksCardDetail } from "./LinksCardDetail";
import { LinksCircle } from "./LinksCircle";
import { LinksColumns } from "./LinksColumns";
import { LinksCompare } from "./LinksCompare";
import { LinksFilters } from "./LinksFilters";
import { LinksStats } from "./LinksStats";
import { findShortestPath, type LinksFilterState, type LinksLayoutMode } from "./types";

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
  onGoCombo?: (targetGoalCardId: number, startCardId?: number) => void;
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
  const [pathTargetCode, setPathTargetCode] = useState<number | null>(null);
  const [sampleHand, setSampleHand] = useState<number[] | null>(null);

  // Comparison mode states
  const [isCompareOpen, setIsCompareOpen] = useState(false);
  const [compareDeckId, setCompareDeckId] = useState<number | null>(null);
  const [comparison, setComparison] = useState<LinksComparison | null>(null);
  const [compareLoading, setCompareLoading] = useState(false);
  const [compareError, setCompareError] = useState<string | null>(null);
  const [showLostLinks, setShowLostLinks] = useState(true);
  const [isReversed, setIsReversed] = useState(false);

  const compareReqIdRef = useRef(0);

  // Reset comparison and selection when current deck changes
  useEffect(() => {
    compareReqIdRef.current++;
    setCompareDeckId(null);
    setComparison(null);
    setCompareError(null);
    setCompareLoading(false);
    setIsReversed(false);
    setSelectedCardCode(null);
    setPathTargetCode(null);
    setSampleHand(null);
  }, [deck.id]);

  const handleSelectCard = useCallback((code: number | null) => {
    setSelectedCardCode(code);
    setPathTargetCode(null);
  }, []);

  const loadComparison = useCallback(
    async (targetId: number | null, reversed: boolean) => {
      setCompareDeckId(targetId);
      setCompareError(null);
      if (targetId == null) {
        compareReqIdRef.current++;
        setComparison(null);
        setCompareLoading(false);
        setCompareError(null);
        return;
      }
      if (deck.id == null) return;

      const reqId = ++compareReqIdRef.current;
      setCompareLoading(true);
      try {
        // Normal direction: targetId is before (previous version), deck.id is after (current version)
        const beforeId = reversed ? deck.id : targetId;
        const afterId = reversed ? targetId : deck.id;
        const res = await api.deckLinksCompare(beforeId, afterId);
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

  const handleSelectCompareDeck = useCallback(
    (targetId: number | null) => {
      void loadComparison(targetId, isReversed);
    },
    [loadComparison, isReversed],
  );

  const handleToggleReversed = useCallback(() => {
    const nextReversed = !isReversed;
    setIsReversed(nextReversed);
    if (compareDeckId != null) {
      void loadComparison(compareDeckId, nextReversed);
    }
  }, [isReversed, compareDeckId, loadComparison]);

  // Active data to display: if comparison is active, use comparison.after
  const activeData = comparison ? comparison.after : data;

  const selectedCard = activeData?.cards.find((c) => c.code === selectedCardCode) ?? null;

  // Shortest path between selected card and target code
  const shortestPathLinks = useMemo(() => {
    if (!selectedCardCode || !pathTargetCode || !activeData) return null;
    return findShortestPath(selectedCardCode, pathTargetCode, activeData.links);
  }, [selectedCardCode, pathTargetCode, activeData]);

  // Draw or clear sample hand overlay (5 cards going first, 6 going second)
  // Respects copies and Main Deck (!c.extra) of activeData, ensuring correct behavior in comparison mode
  const drawSampleHand = () => {
    if (!activeData || activeData.cards.length === 0) return;
    sound.play("page");
    const count = hand === "second" ? 6 : 5;
    const pool: number[] = [];
    for (const c of activeData.cards) {
      if (!c.extra) {
        for (let i = 0; i < c.copies; i++) {
          pool.push(c.code);
        }
      }
    }
    if (pool.length === 0) return;
    for (let i = pool.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [pool[i], pool[j]] = [pool[j], pool[i]];
    }
    setSampleHand(pool.slice(0, count));
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

  const cardsCount = activeData?.cards.length ?? 0;
  const linksCount = activeData?.links.length ?? 0;
  const usedCount = activeData?.links.filter((l) => l.used > 0).length ?? 0;

  return (
    <div className="flex flex-col gap-3">
      {/* Header bar: title, summary stats, comparison toggle, sample hand, and reload */}
      <div className="flex flex-wrap items-center justify-between gap-2.5">
        <div className="flex flex-wrap items-center gap-2">
          <h3 className="font-semibold text-text text-sm flex items-center gap-1.5">
            <span>Mappa dei collegamenti</span>
            {activeData && (
              <span className="text-xs text-muted font-normal">
                ({cardsCount} carte · {linksCount} collegamenti, {usedCount} usati)
              </span>
            )}
          </h3>
          {activeData && (
            <span
              className="chip !py-0.5 !px-2 text-[10px] text-muted border-border cursor-help"
              title="Calcolata simulando le mani del deck con il motore EDOPro. Lo spessore indica la frequenza nelle linee ottimali, il tratteggio indica collegamenti solo possibili o dal testo."
            >
              Info motore
            </span>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-2">
          {activeData && (
            <span className="text-[11px] text-faint tabular-nums hidden sm:inline">
              Mani campionate: {activeData.hands} · Linee: {activeData.lines}
            </span>
          )}

          {/* Toggle comparison view button: disabled while parent map is loading */}
          {decks.length > 1 && (
            <button
              type="button"
              disabled={loading || !data}
              onClick={() => {
                sound.play("toggle");
                setIsCompareOpen((open) => {
                  if (open) {
                    compareReqIdRef.current++;
                    setCompareDeckId(null);
                    setComparison(null);
                    setCompareError(null);
                    setCompareLoading(false);
                  }
                  return !open;
                });
              }}
              className={`chip !py-1 !px-2.5 text-xs flex items-center gap-1.5 transition-colors ${
                isCompareOpen ? "chip-active" : "!bg-surface-2 hover:!bg-surface-3 text-text"
              } ${loading || !data ? "opacity-40 cursor-not-allowed" : ""}`}
              title={
                loading || !data
                  ? "Attendi il completamento della lettura del deck…"
                  : "Confronta i collegamenti con un altro deck della Bottega"
              }
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
          parentLoading={loading}
          error={compareError}
          showLostLinks={showLostLinks}
          onToggleShowLostLinks={setShowLostLinks}
          isReversed={isReversed}
          onToggleReversed={handleToggleReversed}
          getCard={getCard}
          selectedCardCode={selectedCardCode}
          onSelectCard={handleSelectCard}
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
                  handleSelectCard(code);
                }}
                className={`chip !py-0.5 !px-1.5 text-[11px] font-medium ${selectedCardCode === code ? "chip-active" : ""}`}
              >
                {getCard(code)?.name ?? code}
              </button>
            ))}
          </div>
          <span className="text-[11px] text-muted">
            Raggiunge <b>{reachableFromHand ? reachableFromHand.size : 0}</b> carte del mazzo
            {comparison && reachableFromHand && (
              <span className="ml-1.5 font-normal">
                (di cui{" "}
                <span className="text-ok font-semibold">
                  +{Array.from(reachableFromHand).filter((c) => comparison.added.includes(c)).length} nuove
                </span>
                )
              </span>
            )}
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
            onSelectCard={handleSelectCard}
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
            <div className="lab-card flex flex-col items-stretch justify-start p-3 overflow-hidden min-h-[520px]">
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
                  pathLinks={shortestPathLinks}
                  onSelectCard={handleSelectCard}
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
                  onSelectCard={handleSelectCard}
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
                onSelectCard={handleSelectCard}
                targetCode={pathTargetCode}
                onTargetCodeChange={setPathTargetCode}
                shortestPath={shortestPathLinks}
                onGoCombo={onGoCombo}
                onClose={() => handleSelectCard(null)}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
