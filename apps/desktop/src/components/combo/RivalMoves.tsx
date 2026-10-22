import { useEffect, useMemo, useRef, useState } from "react";
import { api, cardImage, type Card, type ComboBoard, type ComboMatchup, type ComboMeasuredMove, type ComboMoves } from "../../lib/api";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import type { HoverCard } from "./Field";

interface RivalMovesProps {
  board: ComboBoard;
  matchup: ComboMatchup;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
}

export function RivalMoves({ board, matchup, getCard, onHover, onHoverEnd }: RivalMovesProps) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<ComboMoves | null>(null);

  // Selected or hovered response to highlight stopped moves
  const [selectedStopIdx, setSelectedStopIdx] = useState<number | null>(null);
  const [hoveredStopIdx, setHoveredStopIdx] = useState<number | null>(null);

  const requestIdRef = useRef(0);

  // Invalidate in-flight and cached data when board or rival matchup changes
  useEffect(() => {
    requestIdRef.current++;
    setLoading(false);
    setError(null);
    setData(null);
    setSelectedStopIdx(null);
    setHoveredStopIdx(null);
  }, [board.hand, board.key, matchup.deckId]);

  const loadMoves = async () => {
    const reqId = ++requestIdRef.current;
    setLoading(true);
    setError(null);
    try {
      sound.play("select");
      const res = await api.comboMoves(board.hand, board.key, matchup.deckId);
      if (reqId !== requestIdRef.current) return;
      setData(res);
      setOpen(true);
    } catch (err) {
      if (reqId !== requestIdRef.current) return;
      const msg = err instanceof Error ? err.message : String(err);
      const userMsg = `Impossibile misurare le linee avversarie: ${msg}`;
      setError(userMsg);
      toast(userMsg, "error");
    } finally {
      if (reqId === requestIdRef.current) {
        setLoading(false);
      }
    }
  };

  // Group moves by line index
  const movesByLine = useMemo(() => {
    if (!data) return new Map<number, { move: ComboMeasuredMove; globalIdx: number }[]>();
    const map = new Map<number, { move: ComboMeasuredMove; globalIdx: number }[]>();
    data.moves.forEach((move, globalIdx) => {
      const list = map.get(move.line) ?? [];
      list.push({ move, globalIdx });
      map.set(move.line, list);
    });
    return map;
  }, [data]);

  const activeStopIdx = hoveredStopIdx ?? selectedStopIdx;
  const activeStop = activeStopIdx != null && data ? data.stops[activeStopIdx] : null;

  return (
    <div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-surface-2/40 p-3.5 transition-all">
      {/* Header action button */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-col">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-text">Turno 2: Linee dell'avversario</span>
            <span className="rounded bg-sec/15 px-1.5 py-0.5 text-[10px] font-bold text-sec">
              Livello 2
            </span>
          </div>
          <p className="mt-0.5 text-[11px] text-muted">
            Mette alla prova la board contro le migliori linee giocate da «{matchup.deckName}».
          </p>
        </div>

        <button
          type="button"
          onClick={() => {
            if (!data) {
              loadMoves();
            } else {
              setOpen((v) => !v);
            }
          }}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-lg border border-sec/40 bg-sec/15 px-3 py-1.5 text-xs font-semibold text-sec transition-all hover:border-sec hover:bg-sec/25 disabled:opacity-50"
        >
          {loading ? (
            <>
              <span className="h-3 w-3 animate-spin rounded-full border-2 border-sec border-t-transparent" />
              <span>Misura in corso…</span>
            </>
          ) : !data ? (
            <>
              <span>⚔</span>
              <span>Contro le sue linee</span>
            </>
          ) : open ? (
            <>
              <span>▲</span>
              <span>Nascondi linee ({data.lines.length})</span>
            </>
          ) : (
            <>
              <span>▼</span>
              <span>Mostra linee ({data.lines.length})</span>
            </>
          )}
        </button>
      </div>

      {/* Loading informative message */}
      {loading && (
        <div className="flex items-start gap-3 rounded-lg border border-sec/30 bg-sec/10 p-3 text-xs text-text anim-fade">
          <div className="mt-0.5 h-4 w-4 shrink-0 animate-spin rounded-full border-2 border-sec border-t-transparent" />
          <div className="space-y-1">
            <p className="font-semibold text-sec">Analisi in corso contro «{matchup.deckName}»…</p>
            <p className="text-[11px] text-muted leading-relaxed">
              La prima volta per questo deck richiede <strong>10–30 secondi</strong> (il motore estrae e analizza le linee
              migliori da 12 mani casuali), poi solo pochi secondi per ogni board successiva.
            </p>
          </div>
        </div>
      )}

      {/* Error banner */}
      {error && (
        <div className="flex items-center justify-between rounded-lg border border-danger/40 bg-danger/10 p-2.5 text-xs text-danger anim-fade">
          <span>{error}</span>
          <button
            type="button"
            onClick={loadMoves}
            className="rounded bg-danger/20 px-2 py-0.5 text-xs font-semibold hover:bg-danger/30"
          >
            Riprova
          </button>
        </div>
      )}

      {/* Level 2 Results View */}
      {data && open && (
        <div className="mt-1 space-y-4 border-t border-border/60 pt-3.5 anim-rise">
          {/* Summary metrics row */}
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            <div className="rounded-lg border border-border bg-surface p-2.5 text-center shadow-xs">
              <span className="text-[10px] text-muted">Mosse residue</span>
              <p className="text-sm font-bold text-sec tabular-nums">
                {Math.round(data.left * 100)}%
              </p>
              <span className="text-[9px] text-faint">con tutte le risposte</span>
            </div>

            <div className="rounded-lg border border-border bg-surface p-2.5 text-center shadow-xs">
              <span className="text-[10px] text-muted">Fermate da board</span>
              <p className="text-sm font-bold text-ok tabular-nums">
                {Math.round(data.floodgateShare * 100)}%
              </p>
              <span className="text-[9px] text-faint">da sola (floodgate)</span>
            </div>

            <div className="rounded-lg border border-border bg-surface p-2.5 text-center shadow-xs">
              <span className="text-[10px] text-muted">Valore interruzioni</span>
              <p className="text-sm font-bold text-text tabular-nums">
                {data.value.toFixed(2)}
              </p>
              <span className="text-[9px] text-faint">contro queste linee</span>
            </div>

            <div className="rounded-lg border border-border bg-surface p-2.5 text-center shadow-xs">
              <span className="text-[10px] text-muted">Linee misurate</span>
              <p className="text-sm font-bold text-text tabular-nums">
                {data.measured} <span className="text-xs text-faint font-normal">/ {data.lines.length}</span>
              </p>
              <span className="text-[9px] text-faint">mani analizzate</span>
            </div>
          </div>

          {/* Warning if madeShare is below 0.8 */}
          {data.madeShare < 0.8 && (
            <div className="flex items-start gap-2 rounded-lg border border-warn/40 bg-warn/10 p-2.5 text-xs text-warn">
              <span className="text-sm leading-none">⚠️</span>
              <div className="space-y-0.5">
                <p className="font-semibold">Affidabilità parziale del turno di prova</p>
                <p className="text-[11px] leading-relaxed text-muted">
                  Il turno di prova ha rigiocato solo il <strong>{Math.round(data.madeShare * 100)}%</strong> delle mosse
                  tipiche dell'avversario su campo vuoto (il resto non è stato possibile rigiocare).
                </p>
              </div>
            </div>
          )}

          {/* Le tue risposte (stops) */}
          {data.stops.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-text">
                  Le tue risposte ({data.stops.length})
                </span>
                <span className="text-[10px] text-muted">
                  Passa o tocca una risposta per evidenziare le mosse che ferma
                </span>
              </div>

              <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                {data.stops.map((stop, sIdx) => {
                  const card = getCard(stop.code);
                  const isSelected = selectedStopIdx === sIdx;
                  const isHovered = hoveredStopIdx === sIdx;
                  const isActive = isSelected || isHovered;

                  return (
                    <div
                      key={`${stop.code}-${sIdx}`}
                      onClick={() => setSelectedStopIdx(isSelected ? null : sIdx)}
                      onPointerEnter={() => setHoveredStopIdx(sIdx)}
                      onPointerLeave={() => setHoveredStopIdx(null)}
                      className={`group flex cursor-pointer items-center justify-between rounded-lg border p-2.5 transition-all ${
                        isActive
                          ? "border-sec bg-sec/15 shadow-sm"
                          : "border-border bg-surface hover:border-sec/50 hover:bg-surface-2"
                      }`}
                    >
                      <div className="flex items-center gap-2.5 min-w-0">
                        {/* Mini thumbnail */}
                        <div
                          className="relative h-10 w-7 shrink-0 overflow-hidden rounded border border-border bg-surface-2"
                          onPointerEnter={(e) => {
                            e.stopPropagation();
                            onHover?.(stop.code, e.currentTarget);
                          }}
                          onPointerLeave={(e) => {
                            e.stopPropagation();
                            onHoverEnd?.();
                          }}
                        >
                          <img
                            src={cardImage(stop.code, "cropped")}
                            alt=""
                            loading="lazy"
                            className="h-full w-full object-cover"
                          />
                        </div>

                        <div className="min-w-0">
                          <p className="truncate text-xs font-semibold text-text group-hover:text-sec">
                            {card?.name ?? `#${stop.code}`}
                          </p>
                          <div className="flex items-center gap-1.5 text-[10px] text-muted">
                            <span className="rounded bg-surface-3 px-1.5 py-0.5">{stop.place}</span>
                            <span className="truncate">{stop.effect}</span>
                          </div>
                        </div>
                      </div>

                      <div className="text-right shrink-0 pl-2">
                        <span className="text-[11px] font-bold text-sec tabular-nums">
                          {Math.round(stop.coverage * 100)}% quota
                        </span>
                        <p className="text-[10px] text-faint tabular-nums">
                          valore {stop.value.toFixed(2)}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Le sue linee (lines) */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold text-text">
                Linee provate dell'avversario ({data.lines.length})
              </span>
              {activeStop && (
                <span className="rounded bg-sec/20 px-2 py-0.5 text-[10px] font-bold text-sec">
                  Evidenziate le mosse fermate da «{getCard(activeStop.code)?.name ?? activeStop.code}»
                </span>
              )}
            </div>

            <div className="space-y-3">
              {data.lines.map((line, lineIdx) => {
                const lineMoves = movesByLine.get(lineIdx) ?? [];

                return (
                  <div
                    key={lineIdx}
                    className="rounded-lg border border-border bg-surface p-3 space-y-2.5 shadow-xs"
                  >
                    {/* Line Header & Hand */}
                    <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border/50 pb-2">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-bold text-text">
                          Linea #{lineIdx + 1}
                        </span>
                        <span className="text-[10px] text-muted">
                          {line.moves.length === 0
                            ? "Solo carte Settate"
                            : lineMoves.length === 0
                              ? "Non misurata"
                              : `${lineMoves.length} ${lineMoves.length === 1 ? "mossa" : "mosse"}`}
                        </span>
                      </div>

                      {/* Initial hand thumbnails */}
                      <div className="flex items-center gap-1">
                        <span className="text-[10px] text-muted mr-1">Mano:</span>
                        {line.hand.map((cardId, hIdx) => {
                          const card = getCard(cardId);
                          return (
                            <div
                              key={`${cardId}-${hIdx}`}
                              className="relative h-8 w-5.5 shrink-0 overflow-hidden rounded border border-border bg-surface-2 transition-transform hover:scale-110 cursor-pointer"
                              onPointerEnter={(e) => onHover?.(cardId, e.currentTarget)}
                              onPointerLeave={onHoverEnd}
                              title={card?.name ?? `#${cardId}`}
                            >
                              <img
                                src={cardImage(cardId, "cropped")}
                                alt=""
                                loading="lazy"
                                className="h-full w-full object-cover"
                              />
                            </div>
                          );
                        })}
                      </div>
                    </div>

                    {/* Moves sequence */}
                    {lineMoves.length === 0 ? (
                      <p className="py-1 text-center text-xs italic text-faint">
                        {line.moves.length === 0
                          ? "Solo carte Settate (nessuna mossa attiva tentata nel turno)"
                          : "Linea non misurata: la partita rigiocata non è arrivata in fondo"}
                      </p>
                    ) : (
                      <div className="flex flex-col gap-1.5">
                        {lineMoves.map(({ move, globalIdx }, mIdx) => {
                          const card = getCard(move.code);
                          const isStoppedByActive = activeStop?.moves.includes(globalIdx) ?? false;

                          let stateBadge = "";
                          let stateLabel = "";

                          switch (move.state) {
                            case "made":
                              stateBadge = "border-ok/30 bg-ok/10 text-ok";
                              stateLabel = "✓ Fatta";
                              break;
                            case "stopped":
                              stateBadge = "border-danger/40 bg-danger/10 text-danger";
                              stateLabel = "🚫 Fermata";
                              break;
                            case "lost":
                              stateBadge = "border-warn/30 bg-warn/10 text-warn";
                              stateLabel = "↷ Persa";
                              break;
                            case "notMade":
                              stateBadge = "border-border bg-surface-3 text-faint";
                              stateLabel = "— Non rigiocata";
                              break;
                          }

                          let kindLabel = "";
                          switch (move.kind) {
                            case "normalSummon":
                              kindLabel = "Evocazione Normale";
                              break;
                            case "specialSummon":
                              kindLabel = "Evocazione Speciale";
                              break;
                            case "activate":
                              kindLabel = "Attivazione";
                              break;
                          }

                          return (
                            <div
                              key={globalIdx}
                              className={`flex items-center justify-between rounded-lg border px-2.5 py-1.5 text-xs transition-all ${
                                isStoppedByActive
                                  ? "border-sec bg-sec/15 ring-2 ring-sec/40 shadow-xs"
                                  : "border-border/60 bg-surface-2/60 hover:border-border"
                              }`}
                            >
                              <div className="flex items-center gap-2 min-w-0">
                                <span className="text-[10px] font-mono text-faint w-4 shrink-0">
                                  {mIdx + 1}.
                                </span>

                                {/* Card mini art */}
                                <div
                                  className="h-6 w-4.5 shrink-0 overflow-hidden rounded border border-border bg-surface cursor-pointer"
                                  onPointerEnter={(e) => onHover?.(move.code, e.currentTarget)}
                                  onPointerLeave={onHoverEnd}
                                >
                                  <img
                                    src={cardImage(move.code, "cropped")}
                                    alt=""
                                    loading="lazy"
                                    className="h-full w-full object-cover"
                                  />
                                </div>

                                <span
                                  className="truncate font-medium text-text hover:text-sec cursor-pointer"
                                  onPointerEnter={(e) => onHover?.(move.code, e.currentTarget)}
                                  onPointerLeave={onHoverEnd}
                                >
                                  {card?.name ?? `#${move.code}`}
                                </span>

                                <span className="text-[10px] text-muted truncate">
                                  ({kindLabel} da {move.place})
                                </span>

                                {isStoppedByActive && (
                                  <span className="shrink-0 rounded bg-sec/20 px-1.5 py-0.5 text-[9px] font-bold text-sec">
                                    ⚡ Fermata da questa risposta
                                  </span>
                                )}
                              </div>

                              <div className="shrink-0 pl-2">
                                <span
                                  className={`rounded border px-2 py-0.5 text-[10px] font-semibold ${stateBadge}`}
                                >
                                  {stateLabel}
                                </span>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
