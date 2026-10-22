import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CardViewer } from "../components/CardViewer";
import { SectionHeader } from "../components/SectionHeader";
import { api, cardImage, formatDate, type Card, type DataStatus, type Deck, type DeckFormat } from "../lib/api";
import { copyLimit, FORMAT_LABELS } from "../lib/deck";
import { sectionById } from "../sections";

export type BanStatus = "Forbidden" | "Limited" | "Semi-Limited" | "Unlimited";

export function normalizeBanStatus(raw: string | null | undefined): BanStatus {
  if (!raw) return "Unlimited";
  const l = raw.toLowerCase();
  if (l === "forbidden" || l === "banned" || l === "proibita" || l === "proibite") return "Forbidden";
  if (l === "limited" || l === "limitata" || l === "limitate") return "Limited";
  if (l === "semi-limited" || l === "semi-limitata" || l === "semi" || l === "semilimitata") return "Semi-Limited";
  return "Unlimited";
}

export function statusLabel(status: BanStatus): string {
  switch (status) {
    case "Forbidden":
      return "Proibita (0)";
    case "Limited":
      return "Limitata (1)";
    case "Semi-Limited":
      return "Semi-limitata (2)";
    case "Unlimited":
      return "Illimitata (3)";
  }
}

export function hasFormatDivergence(card: Card): boolean {
  const md = normalizeBanStatus(card.banMd);
  const tcg = normalizeBanStatus(card.banTcg);
  const ocg = normalizeBanStatus(card.banOcg);
  return md !== tcg || md !== ocg || tcg !== ocg;
}

export function isCardRestricted(card: Card): boolean {
  return (
    normalizeBanStatus(card.banMd) !== "Unlimited" ||
    normalizeBanStatus(card.banTcg) !== "Unlimited" ||
    normalizeBanStatus(card.banOcg) !== "Unlimited"
  );
}

interface Violation {
  card: Card;
  copies: number;
  limit: number;
  excess: number;
}

interface DeckEvaluation {
  violations: Violation[];
  unverifiableIds: number[];
  isLegal: boolean;
}

interface Props {
  artReady?: boolean;
  onOpenDeckInForge: (deckId: number) => void;
}

type TabMode = "confronto" | "impatto";

export function Banditore({ artReady = true, onOpenDeckInForge }: Props) {
  const [activeTab, setActiveTab] = useState<TabMode>("confronto");
  const [status, setStatus] = useState<DataStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);

  // --- Tab 1: Confronto Formati state ---
  const [cards, setCards] = useState<Card[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterDivergentOnly, setFilterDivergentOnly] = useState(false);
  const [selectedFormat, setSelectedFormat] = useState<DeckFormat | "all">("all");
  const [selectedStatus, setSelectedStatus] = useState<BanStatus | "all">("all");
  const [cardsLoading, setCardsLoading] = useState(true);
  const [cardsError, setCardsError] = useState<string | null>(null);
  const [viewerCard, setViewerCard] = useState<Card | null>(null);

  // --- Tab 2: Impatto sui Deck state ---
  const [decks, setDecks] = useState<Deck[]>([]);
  const [deckCardMap, setDeckCardMap] = useState<Map<number, Card>>(new Map());
  const [evalFormats, setEvalFormats] = useState<Record<number, DeckFormat>>({});
  const [decksLoading, setDecksLoading] = useState(false);
  const [decksError, setDecksError] = useState<string | null>(null);
  const [decksWarning, setDecksWarning] = useState<string | null>(null);

  const cardsReqIdRef = useRef(0);
  const decksReqIdRef = useRef(0);

  // Load DataStatus for MD Banlist date
  useEffect(() => {
    api
      .dataStatus()
      .then(setStatus)
      .catch((err) => {
        setStatusError(err instanceof Error ? err.message : String(err));
      });
  }, []);

  // Fetch Banlist Cards / Search for Tab 1
  useEffect(() => {
    const reqId = ++cardsReqIdRef.current;
    setCardsLoading(true);
    setCardsError(null);

    const timer = setTimeout(() => {
      const q = searchQuery.trim();
      if (q.length > 0) {
        // Search query typed by user
        api
          .search(q)
          .then((res) => {
            if (reqId !== cardsReqIdRef.current) return;
            if (res.ids.length === 0) {
              setCards([]);
              setCardsLoading(false);
              return;
            }
            return api.getCards(res.ids).then((loaded) => {
              if (reqId !== cardsReqIdRef.current) return;
              setCards(loaded);
              setCardsLoading(false);
            });
          })
          .catch((err) => {
            if (reqId !== cardsReqIdRef.current) return;
            setCardsError(`Errore durante la ricerca delle carte: ${err instanceof Error ? err.message : String(err)}`);
            setCardsLoading(false);
          });
      } else {
        // Default: load all restricted cards across MD and TCG/OCG
        Promise.all([
          api.search("ban:forbidden ban:limited ban:semi"),
          api.search("formato:md ban:forbidden ban:limited ban:semi"),
        ])
          .then(([tcgRes, mdRes]) => {
            if (reqId !== cardsReqIdRef.current) return;
            const uniqueIds = Array.from(new Set([...tcgRes.ids, ...mdRes.ids]));
            if (uniqueIds.length === 0) {
              setCards([]);
              setCardsLoading(false);
              return;
            }
            return api.getCards(uniqueIds).then((loaded) => {
              if (reqId !== cardsReqIdRef.current) return;
              setCards(loaded);
              setCardsLoading(false);
            });
          })
          .catch((err) => {
            if (reqId !== cardsReqIdRef.current) return;
            setCardsError(`Errore durante il caricamento delle carte limitate: ${err instanceof Error ? err.message : String(err)}`);
            setCardsLoading(false);
          });
      }
    }, 200);

    return () => {
      clearTimeout(timer);
    };
  }, [searchQuery]);

  // Load Decks & Cards for Tab 2
  const loadDecksImpact = useCallback(() => {
    const reqId = ++decksReqIdRef.current;
    setDecksLoading(true);
    setDecksError(null);
    setDecksWarning(null);

    api
      .listDecks()
      .then(async (summaries) => {
        if (reqId !== decksReqIdRef.current) return;
        if (summaries.length === 0) {
          setDecks([]);
          setDecksLoading(false);
          return;
        }

        // Fetch all decks in parallel
        const deckResults = await Promise.allSettled(summaries.map((s) => api.getDeck(s.id)));
        if (reqId !== decksReqIdRef.current) return;

        const loadedDecks: Deck[] = [];
        let failedDecksCount = 0;
        const cardIds = new Set<number>();

        for (const res of deckResults) {
          if (res.status === "fulfilled" && res.value) {
            const d = res.value;
            loadedDecks.push(d);
            for (const id of d.main) cardIds.add(id);
            for (const id of d.extra) cardIds.add(id);
            for (const id of d.side) cardIds.add(id);
          } else {
            failedDecksCount++;
          }
        }

        if (failedDecksCount > 0) {
          setDecksWarning(
            `${failedDecksCount} deck su ${summaries.length} non ${
              failedDecksCount === 1 ? "è stato caricato" : "sono stati caricati"
            }.`
          );
        }

        if (loadedDecks.length === 0 && failedDecksCount > 0) {
          setDecksError("Impossibile caricare i mazzi salvati nella Bottega.");
          setDecksLoading(false);
          return;
        }

        // Fetch full Card metadata for all cards in decks
        let cardMap = new Map<number, Card>();
        if (cardIds.size > 0) {
          try {
            const loadedCards = await api.getCards(Array.from(cardIds));
            cardMap = new Map(loadedCards.map((c) => [c.id, c]));
          } catch (err) {
            if (reqId !== decksReqIdRef.current) return;
            setDecksError(
              `Errore nel caricamento delle carte dei mazzi: ${err instanceof Error ? err.message : String(err)}`
            );
            setDecksLoading(false);
            return;
          }
        }

        if (reqId !== decksReqIdRef.current) return;
        setDecks(loadedDecks);
        setDeckCardMap(cardMap);
        setDecksLoading(false);
      })
      .catch((err) => {
        if (reqId !== decksReqIdRef.current) return;
        setDecksError(`Errore durante il caricamento dei deck della Bottega: ${err instanceof Error ? err.message : String(err)}`);
        setDecksLoading(false);
      });
  }, []);

  useEffect(() => {
    if (activeTab === "impatto") {
      loadDecksImpact();
    }
  }, [activeTab, loadDecksImpact]);

  // Tab 1: Filtered Cards
  const filteredCards = useMemo(() => {
    return cards.filter((card) => {
      // Divergence filter
      if (filterDivergentOnly && !hasFormatDivergence(card)) {
        return false;
      }

      // If user typed search query, show any matching card unless specific status filters are applied
      const isSearchMode = searchQuery.trim().length > 0;
      if (!isSearchMode && !filterDivergentOnly && !isCardRestricted(card)) {
        return false;
      }

      // Format & Status filters
      if (selectedFormat !== "all") {
        const raw = selectedFormat === "tcg" ? card.banTcg : selectedFormat === "ocg" ? card.banOcg : card.banMd;
        const status = normalizeBanStatus(raw);
        if (selectedStatus !== "all") {
          if (status !== selectedStatus) return false;
        } else {
          if (status === "Unlimited") return false;
        }
      } else if (selectedStatus !== "all") {
        const hasStatus =
          normalizeBanStatus(card.banMd) === selectedStatus ||
          normalizeBanStatus(card.banTcg) === selectedStatus ||
          normalizeBanStatus(card.banOcg) === selectedStatus;
        if (!hasStatus) return false;
      }

      return true;
    });
  }, [cards, filterDivergentOnly, searchQuery, selectedFormat, selectedStatus]);

  // Tab 1: Count stats
  const divergenceCount = useMemo(() => cards.filter(hasFormatDivergence).length, [cards]);

  // Tab 2: Evaluate Violations for a deck
  const evaluateDeck = useCallback(
    (deck: Deck, targetFormat: DeckFormat): DeckEvaluation => {
      const counts = new Map<number, number>();
      for (const id of [...deck.main, ...deck.extra, ...deck.side]) {
        counts.set(id, (counts.get(id) ?? 0) + 1);
      }

      const violations: Violation[] = [];
      const unverifiableIds: number[] = [];

      for (const [id, copies] of counts) {
        const card = deckCardMap.get(id);
        if (!card) {
          unverifiableIds.push(id);
          continue;
        }
        const limit = copyLimit(card, targetFormat);
        if (copies > limit) {
          violations.push({
            card,
            copies,
            limit,
            excess: copies - limit,
          });
        }
      }

      violations.sort((a, b) => a.limit - b.limit || b.excess - a.excess);
      const isLegal = violations.length === 0 && unverifiableIds.length === 0;

      return { violations, unverifiableIds, isLegal };
    },
    [deckCardMap],
  );

  return (
    <div className="flex h-full flex-col overflow-hidden bg-bg">
      <SectionHeader section={sectionById("news")} artReady={artReady}>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 rounded-lg border border-border bg-surface-2 px-3 py-1.5 shadow-sm">
            <span className="text-xs text-muted">Lista Master Duel:</span>
            <span className="text-xs font-semibold text-sec">
              {status?.banlistMd
                ? formatDate(status.banlistMd)
                : statusError
                  ? "Data non disponibile"
                  : "Caricamento data..."}
            </span>
          </div>
        </div>
      </SectionHeader>

      {/* Main navigation tabs */}
      <div className="flex items-center justify-between border-b border-border bg-surface px-8 py-3 xl:px-10">
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setActiveTab("confronto")}
            className={`flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold transition-all ${
              activeTab === "confronto"
                ? "border border-sec/40 bg-sec/15 text-sec shadow-sm"
                : "border border-transparent text-muted hover:border-border hover:bg-surface-2 hover:text-text"
            }`}
          >
            <span>⚖ Confronto Formati</span>
            <span className="rounded-full bg-surface-3 px-2 py-0.5 text-xs text-muted">
              {cardsLoading ? "..." : cards.length}
            </span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("impatto")}
            className={`flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold transition-all ${
              activeTab === "impatto"
                ? "border border-sec/40 bg-sec/15 text-sec shadow-sm"
                : "border border-transparent text-muted hover:border-border hover:bg-surface-2 hover:text-text"
            }`}
          >
            <span>📦 Impatto sui Tuoi Deck</span>
            {decks.length > 0 && (
              <span className="rounded-full bg-surface-3 px-2 py-0.5 text-xs text-muted">
                {decks.length}
              </span>
            )}
          </button>
        </div>

        <div className="text-xs text-muted">
          {activeTab === "confronto" && (
            <span>
              {divergenceCount} {divergenceCount === 1 ? "divergenza rilevata" : "divergenze rilevate"} tra MD, TCG e OCG
            </span>
          )}
          {activeTab === "impatto" && <span>Verifica la legalità dei tuoi mazzi nei 3 formati</span>}
        </div>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto p-6 xl:p-8">
        {/* ========================================================================= */}
        {/* TAB 1: CONFRONTO FORMATI                                                  */}
        {/* ========================================================================= */}
        {activeTab === "confronto" && (
          <div className="mx-auto max-w-7xl space-y-6">
            {/* Filter toolbar */}
            <div className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-border bg-surface p-4 shadow-sm">
              {/* Search input */}
              <div className="relative min-w-[280px] flex-1">
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Cerca carta per nome..."
                  className="w-full rounded-lg border border-border bg-surface-2 px-3.5 py-2 pl-9 text-sm text-text placeholder:text-muted focus:border-sec focus:outline-none focus:ring-1 focus:ring-sec"
                />
                <span className="pointer-events-none absolute left-3 top-2.5 text-muted">🔍</span>
                {searchQuery && (
                  <button
                    type="button"
                    onClick={() => setSearchQuery("")}
                    className="absolute right-3 top-2.5 text-xs text-muted hover:text-text"
                    title="Cancella ricerca"
                  >
                    ✕
                  </button>
                )}
              </div>

              {/* Divergence toggle button */}
              <button
                type="button"
                onClick={() => setFilterDivergentOnly((prev) => !prev)}
                className={`flex items-center gap-2 rounded-lg border px-3.5 py-2 text-xs font-medium transition-all ${
                  filterDivergentOnly
                    ? "border-sec/50 bg-sec/15 text-sec shadow-sm"
                    : "border-border bg-surface-2 text-muted hover:border-border-strong hover:text-text"
                }`}
              >
                <span>⇄</span>
                <span>Solo divergenze tra formati</span>
                {divergenceCount > 0 && (
                  <span className="rounded-full bg-sec/20 px-1.5 py-0.5 text-[10px] font-semibold text-sec">
                    {divergenceCount}
                  </span>
                )}
              </button>

              {/* Format filter */}
              <div className="flex items-center gap-1 rounded-lg border border-border bg-surface-2 p-1">
                <span className="px-2 text-xs text-muted">Formato:</span>
                {(["all", "md", "tcg", "ocg"] as const).map((fmt) => (
                  <button
                    key={fmt}
                    type="button"
                    onClick={() => setSelectedFormat(fmt)}
                    className={`rounded px-2.5 py-1 text-xs font-medium transition-colors ${
                      selectedFormat === fmt
                        ? "bg-surface text-text shadow-sm"
                        : "text-muted hover:text-text"
                    }`}
                  >
                    {fmt === "all" ? "Tutti" : FORMAT_LABELS[fmt]}
                  </button>
                ))}
              </div>

              {/* Status filter */}
              <div className="flex items-center gap-1 rounded-lg border border-border bg-surface-2 p-1">
                <span className="px-2 text-xs text-muted">Stato:</span>
                {(["all", "Forbidden", "Limited", "Semi-Limited"] as const).map((st) => (
                  <button
                    key={st}
                    type="button"
                    onClick={() => setSelectedStatus(st)}
                    className={`rounded px-2.5 py-1 text-xs font-medium transition-colors ${
                      selectedStatus === st
                        ? "bg-surface text-text shadow-sm"
                        : "text-muted hover:text-text"
                    }`}
                  >
                    {st === "all"
                      ? "Tutti"
                      : st === "Forbidden"
                        ? "Proibite"
                        : st === "Limited"
                          ? "Limitate"
                          : "Semi-limitate"}
                  </button>
                ))}
              </div>
            </div>

            {/* Error state */}
            {cardsError && (
              <div className="rounded-xl border border-danger/40 bg-danger/10 p-4 text-sm text-danger">
                <p className="font-semibold">Si è verificato un errore:</p>
                <p>{cardsError}</p>
              </div>
            )}

            {/* Loading state */}
            {cardsLoading ? (
              <div className="flex flex-col items-center justify-center py-16 text-muted">
                <div className="h-8 w-8 animate-spin rounded-full border-2 border-sec border-t-transparent" />
                <p className="mt-3 text-sm">Caricamento delle carte in corso...</p>
              </div>
            ) : filteredCards.length === 0 ? (
              /* Empty state */
              <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-border bg-surface/50 py-16 text-center">
                <span className="text-3xl">📭</span>
                <p className="mt-2 text-base font-semibold text-text">Nessuna carta trovata</p>
                <p className="mt-1 max-w-sm text-xs text-muted">
                  Nessuna carta corrisponde ai criteri di ricerca e ai filtri selezionati. Prova a rimuovere i filtri o
                  a cambiare termine di ricerca.
                </p>
                {(searchQuery || filterDivergentOnly || selectedFormat !== "all" || selectedStatus !== "all") && (
                  <button
                    type="button"
                    onClick={() => {
                      setSearchQuery("");
                      setFilterDivergentOnly(false);
                      setSelectedFormat("all");
                      setSelectedStatus("all");
                    }}
                    className="mt-4 rounded-lg border border-border bg-surface-2 px-4 py-2 text-xs font-medium text-text hover:bg-surface-3"
                  >
                    Azzera tutti i filtri
                  </button>
                )}
              </div>
            ) : (
              /* Cards table / matrix */
              <div className="space-y-3">
                <div className="flex items-center justify-between text-xs text-muted px-1">
                  <span>
                    Mostrando <strong className="text-text">{filteredCards.length}</strong> carte
                  </span>
                  <span>Clicca su una carta per aprire il Visore Carta</span>
                </div>

                <div className="grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3">
                  {filteredCards.map((card) => {
                    const mdStatus = normalizeBanStatus(card.banMd);
                    const tcgStatus = normalizeBanStatus(card.banTcg);
                    const ocgStatus = normalizeBanStatus(card.banOcg);
                    const isDivergent = hasFormatDivergence(card);

                    return (
                      <div
                        key={card.id}
                        onClick={() => setViewerCard(card)}
                        className={`group relative flex cursor-pointer flex-col justify-between rounded-xl border bg-surface p-4 transition-all hover:-translate-y-0.5 hover:shadow-md ${
                          isDivergent
                            ? "border-sec/30 hover:border-sec/70"
                            : "border-border hover:border-border-strong"
                        }`}
                      >
                        <div className="flex items-start gap-3">
                          {/* Thumbnail */}
                          <div className="relative h-16 w-11 shrink-0 overflow-hidden rounded border border-border bg-surface-2">
                            <img
                              src={cardImage(card.id, "cropped")}
                              alt={card.name}
                              loading="lazy"
                              className="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
                            />
                          </div>

                          {/* Card info */}
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-1.5">
                              <h3 className="truncate text-sm font-bold text-text group-hover:text-sec">
                                {card.name}
                              </h3>
                              {isDivergent && (
                                <span
                                  className="shrink-0 rounded border border-sec/40 bg-sec/10 px-1.5 py-0.5 text-[10px] font-semibold text-sec"
                                  title="Stato differente tra i formati"
                                >
                                  ⇄ Divergenza
                                </span>
                              )}
                            </div>

                            <p className="mt-0.5 truncate text-[11px] text-muted">
                              {card.humanType}
                              {card.archetype ? ` • ${card.archetype}` : ""}
                            </p>

                            {card.kind === "monster" && (
                              <p className="mt-0.5 text-[10px] text-faint">
                                {card.level != null ? `Liv. ${card.level}` : card.linkRating != null ? `Link-${card.linkRating}` : ""}
                                {card.atk != null ? ` • ATK ${card.atk}` : ""}
                                {card.def != null ? ` / DEF ${card.def}` : ""}
                              </p>
                            )}
                          </div>
                        </div>

                        {/* Format Badges Matrix */}
                        <div className="mt-4 grid grid-cols-3 gap-2 border-t border-border pt-3">
                          <FormatBadge format="MD" status={mdStatus} />
                          <FormatBadge format="TCG" status={tcgStatus} />
                          <FormatBadge format="OCG" status={ocgStatus} />
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        )}

        {/* ========================================================================= */}
        {/* TAB 2: IMPATTO SUI TUOI DECK                                              */}
        {/* ========================================================================= */}
        {activeTab === "impatto" && (
          <div className="mx-auto max-w-5xl space-y-6">
            {/* Header info */}
            <div className="rounded-xl border border-border bg-surface p-5 shadow-sm">
              <h2 className="text-base font-bold text-text">Conformità dei mazzi della Bottega</h2>
              <p className="mt-1 text-xs text-muted">
                DuelForge analizza ogni mazzo salvato nella tua Bottega verificando la conformità delle copie rispetto
                alla Forbidden & Limited List nel formato originale o negli altri formati supportati.
              </p>
            </div>

            {/* Warning if some decks failed to load */}
            {decksWarning && (
              <div className="rounded-xl border border-warn/40 bg-warn/10 p-4 text-xs text-warn">
                <p className="font-semibold">Avviso sul caricamento:</p>
                <p>{decksWarning}</p>
              </div>
            )}

            {/* Error state */}
            {decksError && (
              <div className="rounded-xl border border-danger/40 bg-danger/10 p-4 text-sm text-danger">
                <p className="font-semibold">Errore nel caricamento dei mazzi:</p>
                <p>{decksError}</p>
                <button
                  type="button"
                  onClick={loadDecksImpact}
                  className="mt-2 rounded bg-danger/20 px-3 py-1 text-xs font-medium text-danger hover:bg-danger/30"
                >
                  Riprova
                </button>
              </div>
            )}

            {/* Loading state */}
            {decksLoading ? (
              <div className="flex flex-col items-center justify-center py-16 text-muted">
                <div className="h-8 w-8 animate-spin rounded-full border-2 border-sec border-t-transparent" />
                <p className="mt-3 text-sm">Analisi dei mazzi in corso...</p>
              </div>
            ) : decks.length === 0 ? (
              /* No decks saved */
              <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-border bg-surface/50 py-16 text-center">
                <span className="text-3xl">🎴</span>
                <p className="mt-2 text-base font-semibold text-text">Nessun mazzo salvato</p>
                <p className="mt-1 max-w-sm text-xs text-muted">
                  Non hai ancora salvato alcun mazzo nella Bottega. Creane uno nella Fucina per verificarne la legalità!
                </p>
              </div>
            ) : (
              /* Decks list */
              <div className="space-y-4">
                {decks.map((deck) => {
                  const deckId = deck.id!;
                  const evalFormat = evalFormats[deckId] ?? deck.format;
                  const { violations, unverifiableIds, isLegal } = evaluateDeck(deck, evalFormat);
                  const totalExcess = violations.reduce((acc, v) => acc + v.excess, 0);

                  return (
                    <div
                      key={deckId}
                      className="rounded-xl border border-border bg-surface p-5 shadow-sm transition-all"
                    >
                      {/* Deck Header */}
                      <div className="flex flex-wrap items-center justify-between gap-4 border-b border-border pb-4">
                        <div>
                          <div className="flex items-center gap-2">
                            <h3 className="text-base font-bold text-text">{deck.name}</h3>
                            <span className="rounded bg-surface-2 px-2 py-0.5 text-xs text-muted">
                              Formato salvato: {FORMAT_LABELS[deck.format]}
                            </span>
                          </div>
                          <p className="mt-1 text-xs text-muted">
                            Main: {deck.main.length} • Extra: {deck.extra.length} • Side: {deck.side.length} • Totale:{" "}
                            {deck.main.length + deck.extra.length + deck.side.length} carte
                          </p>
                        </div>

                        {/* Format selector buttons */}
                        <div className="flex items-center gap-1 rounded-lg border border-border bg-surface-2 p-1">
                          <span className="px-2 text-xs text-muted">Valuta in:</span>
                          {(["md", "tcg", "ocg"] as const).map((fmt) => (
                            <button
                              key={fmt}
                              type="button"
                              onClick={() => setEvalFormats((prev) => ({ ...prev, [deckId]: fmt }))}
                              className={`rounded px-2.5 py-1 text-xs font-medium transition-colors ${
                                evalFormat === fmt
                                  ? "bg-sec text-bg font-bold shadow-sm"
                                  : "text-muted hover:text-text"
                              }`}
                            >
                              {FORMAT_LABELS[fmt]}
                              {deck.format === fmt && " ★"}
                            </button>
                          ))}
                        </div>
                      </div>

                      {/* Legality status banner */}
                      <div className="mt-4 space-y-3">
                        {unverifiableIds.length > 0 && (
                          <div className="flex items-center gap-2 rounded-lg border border-warn/40 bg-warn/10 px-4 py-2.5 text-xs text-warn">
                            <span className="text-base font-bold">⚠️</span>
                            <span>
                              <strong>Non completamente verificabile:</strong> {unverifiableIds.length}{" "}
                              {unverifiableIds.length === 1
                                ? "carta non è stata trovata nel database"
                                : "carte non sono state trovate nel database"}{" "}
                              (ID: {unverifiableIds.join(", ")}).
                            </span>
                          </div>
                        )}

                        {isLegal ? (
                          <div className="flex items-center justify-between rounded-lg border border-ok/40 bg-ok/10 px-4 py-3 text-ok">
                            <div className="flex items-center gap-2">
                              <span className="text-base font-bold">✓</span>
                              <span className="text-xs font-medium">
                                <strong>100% Legale in {FORMAT_LABELS[evalFormat]}:</strong> Nessuna carta supera il limite
                                consentito.
                              </span>
                            </div>
                            <button
                              type="button"
                              onClick={() => onOpenDeckInForge(deckId)}
                              className="rounded-lg border border-ok/40 bg-ok/20 px-3 py-1.5 text-xs font-semibold text-ok transition-colors hover:bg-ok/30"
                            >
                              Apri nella Fucina
                            </button>
                          </div>
                        ) : (
                          <div className="space-y-3">
                            {violations.length > 0 && (
                              <div className="flex items-center justify-between rounded-lg border border-danger/40 bg-danger/10 px-4 py-3 text-danger">
                                <div className="flex items-center gap-2">
                                  <span className="text-base font-bold">⚠</span>
                                  <span className="text-xs font-medium">
                                    <strong>Fuori legge in {FORMAT_LABELS[evalFormat]}:</strong> {violations.length}{" "}
                                    {violations.length === 1 ? "carta supera" : "carte superano"} il limite ({totalExcess}{" "}
                                    {totalExcess === 1 ? "copia in eccesso" : "copie in eccesso"}).
                                  </span>
                                </div>
                                <button
                                  type="button"
                                  onClick={() => onOpenDeckInForge(deckId)}
                                  className="flex items-center gap-1.5 rounded-lg border border-danger/40 bg-danger/20 px-3 py-1.5 text-xs font-semibold text-danger transition-colors hover:bg-danger/30"
                                >
                                  <span>⚡ Modifica nella Fucina</span>
                                </button>
                              </div>
                            )}

                            {/* Offending cards breakdown */}
                            {violations.length > 0 && (
                              <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-3">
                                {violations.map(({ card, copies, limit, excess }) => (
                                  <div
                                    key={card.id}
                                    onClick={() => setViewerCard(card)}
                                    className="group flex cursor-pointer items-center justify-between rounded-lg border border-border bg-surface-2 p-3 transition-colors hover:border-danger/40 hover:bg-surface-3"
                                  >
                                    <div className="flex items-center gap-2.5 min-w-0">
                                      <div className="h-10 w-7 shrink-0 overflow-hidden rounded border border-border bg-surface">
                                        <img
                                          src={cardImage(card.id, "cropped")}
                                          alt={card.name}
                                          loading="lazy"
                                          className="h-full w-full object-cover"
                                        />
                                      </div>
                                      <div className="min-w-0">
                                        <p className="truncate text-xs font-semibold text-text group-hover:text-sec">
                                          {card.name}
                                        </p>
                                        <p className="text-[11px] text-muted">
                                          {limit === 0
                                            ? "Proibita (limite: 0)"
                                            : limit === 1
                                              ? "Limitata (limite: 1)"
                                              : "Semi-limitata (limite: 2)"}
                                        </p>
                                      </div>
                                    </div>

                                    <div className="text-right shrink-0">
                                      <span className="rounded bg-danger/15 px-2 py-0.5 text-[11px] font-bold text-danger">
                                        {copies} in lista
                                      </span>
                                      <p className="mt-0.5 text-[10px] font-medium text-danger">
                                        - {excess} {excess === 1 ? "copia" : "copie"}
                                      </p>
                                    </div>
                                  </div>
                                ))}
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}
      </div>

      {/* CardViewer Modal */}
      {viewerCard && (
        <CardViewer
          card={viewerCard}
          onClose={() => setViewerCard(null)}
        />
      )}
    </div>
  );
}

/** Chip showing format name and ban status */
function FormatBadge({ format, status }: { format: "MD" | "TCG" | "OCG"; status: BanStatus }) {
  let badgeClasses = "";
  let icon = "";

  switch (status) {
    case "Forbidden":
      badgeClasses = "border-danger/40 bg-danger/10 text-danger";
      icon = "🚫";
      break;
    case "Limited":
      badgeClasses = "border-sec/40 bg-sec/10 text-sec";
      icon = "⚠️";
      break;
    case "Semi-Limited":
      badgeClasses = "border-warn/40 bg-warn/10 text-warn";
      icon = "🟡";
      break;
    case "Unlimited":
      badgeClasses = "border-border/60 bg-surface-2 text-muted";
      icon = "✓";
      break;
  }

  return (
    <div className={`flex flex-col items-center justify-center rounded-lg border p-1.5 text-center ${badgeClasses}`}>
      <span className="text-[10px] font-bold tracking-wider">{format}</span>
      <span className="mt-0.5 text-[11px] font-medium">
        {icon} {status === "Unlimited" ? "3" : status === "Forbidden" ? "0" : status === "Limited" ? "1" : "2"}
      </span>
      <span className="text-[9px] text-muted truncate w-full">
        {status === "Unlimited" ? "Legale" : status}
      </span>
    </div>
  );
}
