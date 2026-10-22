import { useEffect, useRef, useState } from "react";
import { api, type Card, type ComboBoard, type ComboPiece } from "../../lib/api";
import { CardImage, type HoverCard } from "./Field";
import { sound } from "../../lib/sound";

interface Props {
  board: ComboBoard;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
}

export function PieceProtection({ board, getCard, onHover, onHoverEnd }: Props) {
  const [pieces, setPieces] = useState<ComboPiece[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(true);
  const requestIdRef = useRef(0);

  // Reset when board changes (board.hand or board.key) and cancel any running measurement
  useEffect(() => {
    requestIdRef.current++;
    setPieces(null);
    setLoading(false);
    setError(null);
    setExpanded(true);
  }, [board.hand, board.key]);

  const handleMeasure = async () => {
    const reqId = ++requestIdRef.current;
    setLoading(true);
    setError(null);
    sound.play("toggle");
    try {
      const res = await api.boardProtection(board.hand, board.key);
      if (reqId !== requestIdRef.current) return;
      setPieces(res);
      setLoading(false);
    } catch (e) {
      if (reqId !== requestIdRef.current) return;
      setError(e instanceof Error ? e.message : String(e));
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <button
          type="button"
          onClick={() => {
            if (pieces === null) {
              void handleMeasure();
            } else {
              setExpanded((v) => !v);
              sound.play("toggle");
            }
          }}
          disabled={loading}
          className="btn btn-ghost !py-1 !px-2.5 text-xs border border-border/80 hover:border-sec/50 flex items-center gap-1.5"
          title="Verifica come ogni carta che interrompe regge alle rimozioni mirate (distruggere, bandire, girare coperto, negare)"
        >
          {loading ? (
            <>
              <span className="inline-block animate-spin">⏳</span>
              <span>Misuro la protezione dei pezzi…</span>
            </>
          ) : pieces === null ? (
            <>
              <span>🛡️ Protezione dei pezzi</span>
              <span className="text-[10px] text-muted">· verifica rimozioni</span>
            </>
          ) : (
            <>
              <span>🛡️ Protezione dei pezzi</span>
              <span className="text-[10px] text-sec font-medium">({pieces.length} {pieces.length === 1 ? "pezzo" : "pezzi"})</span>
              <span className="text-[10px] text-muted ml-0.5">{expanded ? "▲" : "▼"}</span>
            </>
          )}
        </button>

        {pieces !== null && !loading && (
          <button
            type="button"
            onClick={() => void handleMeasure()}
            className="btn btn-ghost !py-0.5 !px-2 text-[11px] text-faint hover:text-text"
            title="Rimisura la protezione dei pezzi"
          >
            Rimisura
          </button>
        )}
      </div>

      {loading && (
        <p className="text-xs text-muted anim-fade pl-1">
          Rigiocata della board per testare ogni rimozione mirata sul pezzo (qualche secondo)…
        </p>
      )}

      {error && (
        <div className="rounded-lg border border-danger/40 bg-danger/10 p-2.5 text-xs text-danger flex flex-col gap-1.5 anim-fade">
          <p>Impossibile misurare la protezione dei pezzi: {error}</p>
          <button
            type="button"
            onClick={() => void handleMeasure()}
            className="btn btn-ghost !py-0.5 !px-2 text-[11px] text-danger border border-danger/30 w-fit"
          >
            Riprova
          </button>
        </div>
      )}

      {pieces !== null && expanded && !loading && (
        <div className="flex flex-col gap-2 anim-fade">
          {pieces.length === 0 ? (
            <p className="text-xs text-faint pl-1">
              Nessuna carta con interruzioni da proteggere trovata su questa board.
            </p>
          ) : (
            <ul className="flex flex-col gap-2">
              {pieces.map((p, idx) => {
                const card = getCard(p.code);
                const cName = card?.name ?? String(p.code);
                return (
                  <li
                    key={`${p.code}-${p.place}-${idx}`}
                    className="flex flex-col gap-1.5 rounded-lg border border-border/80 bg-surface-2/40 p-2 text-xs"
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className="cursor-pointer shrink-0"
                        onPointerEnter={(e) => onHover?.(p.code, e.currentTarget)}
                        onPointerLeave={onHoverEnd}
                      >
                        <CardImage code={p.code} card={card} className="h-[36px] w-[25px]" />
                      </span>
                      <div className="flex min-w-0 flex-1 items-baseline gap-1.5 flex-wrap">
                        <span
                          className="font-medium text-text hover:text-sec cursor-pointer"
                          onPointerEnter={(e) => onHover?.(p.code, e.currentTarget)}
                          onPointerLeave={onHoverEnd}
                        >
                          {cName}
                        </span>
                        <span className="chip !py-0.5 !px-1.5 text-[10px] text-muted">
                          {p.place}
                        </span>
                      </div>
                    </div>

                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-1.5 pl-8 text-[11px]">
                      <div className="flex items-baseline gap-1.5 flex-wrap">
                        <span className="font-semibold text-ok shrink-0">regge a:</span>
                        {p.holds.length > 0 ? (
                          p.holds.map((h) => (
                            <span key={h} className="rounded border border-ok/30 bg-ok/10 px-1.5 py-0.5 text-ok text-[10px]">
                              {h}
                            </span>
                          ))
                        ) : (
                          <span className="text-faint">nessuna rimozione</span>
                        )}
                      </div>
                      <div className="flex items-baseline gap-1.5 flex-wrap">
                        <span className="font-semibold text-danger shrink-0">cade a:</span>
                        {p.falls.length > 0 ? (
                          p.falls.map((f) => (
                            <span key={f} className="rounded border border-danger/30 bg-danger/10 px-1.5 py-0.5 text-danger text-[10px]">
                              {f}
                            </span>
                          ))
                        ) : (
                          <span className="text-faint">nessuna rimozione</span>
                        )}
                      </div>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
