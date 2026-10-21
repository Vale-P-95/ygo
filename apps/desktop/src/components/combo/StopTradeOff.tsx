import { useEffect, useRef, useState } from "react";
import { api, type Card, type ComboBoard, type ComboDrawTradeOff, type ComboMatchup } from "../../lib/api";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { CardImage, type HoverCard } from "./Field";

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

interface StopTradeOffProps {
  board: ComboBoard;
  matchup: ComboMatchup;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
  onSelectStep?: (stepNumber: number) => void;
}

/**
 * Trade-off between board value and cards drawn by rival handtraps (Maxx "C", Mulcharmy).
 * Queries api.comboStops and identifies the optimal stopping point along the combo line.
 */
export function StopTradeOff({ board, matchup, getCard, onHover, onHoverEnd, onSelectStep }: StopTradeOffProps) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tradeOffs, setTradeOffs] = useState<ComboDrawTradeOff[] | null>(null);
  const [selectedIdx, setSelectedIdx] = useState(0);

  const requestIdRef = useRef(0);

  // Invalidate in-flight and cached trade-off data when board or rival matchup changes.
  useEffect(() => {
    requestIdRef.current++;
    setLoading(false);
    setError(null);
    setTradeOffs(null);
    setSelectedIdx(0);
  }, [board.key, matchup.deckId]);

  const loadStops = async () => {
    const reqId = ++requestIdRef.current;
    setLoading(true);
    setError(null);
    try {
      sound.play("select");
      const res = await api.comboStops(board.hand, board.key, matchup.deckId);
      if (reqId !== requestIdRef.current) return;
      setTradeOffs(res);
      setSelectedIdx(0);
      setOpen(true);
    } catch (err) {
      if (reqId !== requestIdRef.current) return;
      const msg = err instanceof Error ? err.message : String(err);
      const userMsg = `Impossibile calcolare i punti di arresto contro la pesca: ${msg}`;
      setError(userMsg);
      toast(userMsg, "error");
    } finally {
      if (reqId === requestIdRef.current) {
        setLoading(false);
      }
    }
  };

  const activeTradeOff = tradeOffs && tradeOffs.length > 0 ? tradeOffs[selectedIdx] ?? tradeOffs[0] : null;

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border/70 bg-surface-2/40 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="text-base" aria-hidden>
            🛑
          </span>
          <div>
            <h4 className="text-xs font-semibold text-text">Punti di arresto sotto pesca</h4>
            <p className="text-[11px] text-muted">
              Trova dove fermare la linea per massimizzare la board senza regalare pescate a Maxx "C" o alle Mulcharmy.
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={() => {
            if (!tradeOffs && !loading) {
              void loadStops();
            } else {
              setOpen((v) => !v);
            }
          }}
          disabled={loading}
          className={`btn ${open && tradeOffs ? "btn-ghost" : "btn-sec"} !py-1 !px-3 text-xs flex items-center gap-1.5`}
        >
          {loading ? (
            <>
              <span className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent" />
              <span>Calcolo in corso…</span>
            </>
          ) : tradeOffs ? (
            open ? (
              "Nascondi compromesso"
            ) : (
              "Mostra compromesso"
            )
          ) : (
            "Dove fermarsi contro Maxx \"C\""
          )}
        </button>
      </div>

      {error && (
        <div className="mt-2 rounded border border-danger/40 bg-danger/10 p-2 text-xs text-danger flex items-center justify-between gap-2">
          <span>{error}</span>
          <button type="button" onClick={() => void loadStops()} className="underline hover:text-danger/80">
            Riprova
          </button>
        </div>
      )}

      {open && activeTradeOff && (
        <div className="mt-2 flex flex-col gap-3 border-t border-border/50 pt-3 anim-fade">
          {/* Multiple draw handtraps selector */}
          {tradeOffs && tradeOffs.length > 1 && (
            <div className="flex flex-wrap items-center gap-1.5" role="tablist" aria-label="Handtrap che fa pescare">
              <span className="text-xs text-faint">Handtrap:</span>
              {tradeOffs.map((t, idx) => {
                const c = getCard(t.code);
                const name = c?.name ?? String(t.code);
                const isSelected = idx === selectedIdx;
                return (
                  <button
                    key={t.code}
                    type="button"
                    role="tab"
                    aria-selected={isSelected}
                    onClick={() => setSelectedIdx(idx)}
                    className={`chip !py-0.5 !px-2 text-xs ${isSelected ? "chip-active" : ""}`}
                  >
                    {name}
                  </button>
                );
              })}
            </div>
          )}

          {/* Handtrap summary banner */}
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <span className="text-xs font-semibold text-text">
              Contro: <b className="text-sec">{getCard(activeTradeOff.code)?.name ?? activeTradeOff.code}</b>
            </span>
            {activeTradeOff.standardBreakers && (
              <span className="text-[11px] text-muted italic">
                (Deck senza breaker giocabili: stima standard su ~5 breaker in 40 carte)
              </span>
            )}
          </div>

          {/* Recommended best stop callout */}
          {activeTradeOff.stops[activeTradeOff.best] && (
            <div className="rounded-md border border-ok/40 bg-ok/10 p-2.5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
              <div className="flex items-center gap-2">
                <span className="text-ok text-base" aria-hidden>
                  ★
                </span>
                <div>
                  <span className="text-xs font-bold text-ok">Miglior compromesso raccomandato:</span>
                  <p className="text-xs text-text">
                    Fermarsi al{" "}
                    <b>
                      Passo {activeTradeOff.stops[activeTradeOff.best].steps}
                    </b>
                    {activeTradeOff.stops[activeTradeOff.best].playing ? (
                      <>
                        {" "}
                        (dopo <i>{getCard(activeTradeOff.stops[activeTradeOff.best].playing!)?.name ?? activeTradeOff.stops[activeTradeOff.best].playing}</i>)
                      </>
                    ) : null}
                    {" "}— regala solo <b>{activeTradeOff.stops[activeTradeOff.best].drawn}</b> {plural(activeTradeOff.stops[activeTradeOff.best].drawn, "pescata", "pescate")} mantenendo un valore atteso di{" "}
                    <b>{activeTradeOff.stops[activeTradeOff.best].expected.toFixed(2)}</b>.
                  </p>
                </div>
              </div>
              {onSelectStep && (
                <button
                  type="button"
                  onClick={() => onSelectStep(activeTradeOff.stops[activeTradeOff.best].steps)}
                  className="btn btn-ghost !py-1 !px-2.5 text-xs text-ok hover:bg-ok/20 shrink-0 self-start sm:self-auto"
                >
                  Vai al passo →
                </button>
              )}
            </div>
          )}

          {/* Stops table */}
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-border/70 text-faint text-[11px]">
                  <th className="py-1.5 px-2">Passo</th>
                  <th className="py-1.5 px-2">Ultima carta</th>
                  <th className="py-1.5 px-2 text-right">Interruzioni</th>
                  <th className="py-1.5 px-2 text-right">Floodgate</th>
                  <th className="py-1.5 px-2 text-right">Carte pescate</th>
                  <th className="py-1.5 px-2 text-right">Valore atteso</th>
                  <th className="py-1.5 px-2 text-center">Azione</th>
                </tr>
              </thead>
              <tbody>
                {activeTradeOff.stops.map((stop, idx) => {
                  const isBest = idx === activeTradeOff.best;
                  const isLast = idx === activeTradeOff.stops.length - 1;
                  const card = stop.playing ? getCard(stop.playing) : undefined;
                  const cardName = card?.name ?? (stop.playing ? String(stop.playing) : "Inizio linea");
                  return (
                    <tr
                      key={stop.steps}
                      className={`border-b border-border/40 transition-colors ${
                        isBest ? "bg-sec/10 font-medium" : "hover:bg-surface-2/60"
                      }`}
                    >
                      <td className="py-2 px-2 tabular-nums">
                        <span className="flex items-center gap-1.5">
                          {isBest && (
                            <span className="text-ok font-bold text-xs" title="Miglior compromesso">
                              ★
                            </span>
                          )}
                          <span>Passo {stop.steps}</span>
                          {isLast && (
                            <span className="text-[10px] text-faint font-normal">(linea intera)</span>
                          )}
                        </span>
                      </td>
                      <td className="py-2 px-2">
                        <span
                          className="flex items-center gap-1 cursor-default text-text"
                          title={cardName}
                          onPointerEnter={stop.playing ? (e) => onHover?.(stop.playing!, e.currentTarget) : undefined}
                          onPointerLeave={onHoverEnd}
                        >
                          {stop.playing != null && (
                            <CardImage code={stop.playing} card={card} className="h-4 w-3 inline-block" />
                          )}
                          <span className="truncate max-w-[140px]">{cardName}</span>
                        </span>
                      </td>
                      <td className="py-2 px-2 text-right tabular-nums text-muted">{stop.fieldValue.toFixed(2)}</td>
                      <td className="py-2 px-2 text-right tabular-nums text-muted">{stop.floodgateValue.toFixed(2)}</td>
                      <td className="py-2 px-2 text-right tabular-nums font-semibold text-warn">
                        +{stop.drawn}
                      </td>
                      <td className="py-2 px-2 text-right tabular-nums">
                        <span className={`font-bold ${isBest ? "text-ok" : "text-text"}`}>
                          {stop.expected.toFixed(2)}
                        </span>
                      </td>
                      <td className="py-2 px-2 text-center">
                        {onSelectStep && (
                          <button
                            type="button"
                            onClick={() => onSelectStep(stop.steps)}
                            className="btn btn-ghost !py-0.5 !px-1.5 text-[11px] text-muted hover:text-text"
                            title={`Visualizza il campo al passo ${stop.steps}`}
                          >
                            Passo {stop.steps}
                          </button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
