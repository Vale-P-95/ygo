import { useEffect, useState } from "react";
import type { Card, ComboBoard, ComboField, ComboScore, ComboStep } from "../../lib/api";
import type { Weights } from "../../lib/scoring";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { CardImage, Field, type HoverCard, type Pile as PileKind } from "./Field";
import { ScoreBreakdown } from "./Score";
import { RivalMoves } from "./RivalMoves";
import { ComboSteps, formatComboGuide, type ActiveStepInfo } from "./Steps";
import { StopTradeOff } from "./StopTradeOff";

type StepsView = "visual" | "text";
const VIEW_KEY = "duelforge.combo.stepsView";

function savedView(): StepsView {
  try {
    return localStorage.getItem(VIEW_KEY) === "text" ? "text" : "visual";
  } catch {
    return "visual";
  }
}

interface Props {
  board: ComboBoard;
  /** The probe turn's result, when measured. */
  score: ComboScore | null;
  scoring: boolean;
  weights: Weights;
  /** The hand the line starts from. */
  hand: number[];
  /** `null` while loading; empty when the steps cannot be told yet. */
  steps: ComboStep[] | null;
  stepsNote: string | null;
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
}

const PILE_TITLES: Record<PileKind, string> = { grave: "Cimitero", banished: "Bandite", deck: "Deck", extra: "Extra Deck (scoperte)" };

function Pile({
  title,
  codes,
  highlightedCodes,
  getCard,
  onHover,
  onHoverEnd,
}: {
  title: string;
  codes: number[];
  highlightedCodes?: Set<number> | null;
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
}) {
  return (
    <div className="min-w-0">
      <p className="eyebrow mb-1.5 text-faint">
        {title} · {codes.length}
      </p>
      {codes.length === 0 ? (
        <p className="text-xs text-faint">—</p>
      ) : (
        <div className="flex flex-wrap gap-1">
          {codes.map((c, i) => {
            const hl = !!highlightedCodes && highlightedCodes.has(c);
            return (
              <span
                key={i}
                className={hl ? "rounded-[3px] ring-2 ring-[var(--sec)] shadow-[0_0_10px_var(--sec)]" : ""}
                onPointerEnter={(e) => onHover(c, e.currentTarget)}
                onPointerLeave={onHoverEnd}
              >
                <CardImage code={c} card={getCard(c)} className="h-[44px] w-[30px]" />
              </span>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** Turn and phase changes, shown as quiet separators. */
const PHASE = /^(Inizia il |Draw Phase|Standby Phase|Main Phase|Battle Phase|End Phase)/;

/** A board of the search: the field, the other piles and the line that builds it, told in Italian. */
export function BoardDetail({ board, score, scoring, weights, hand, steps, stepsNote, getCard, onHover, onHoverEnd }: Props) {
  const [pile, setPile] = useState<PileKind | null>(null);
  const [view, setView] = useState<StepsView>(savedView);
  const [activeCards, setActiveCards] = useState<Set<number> | null>(null);
  const [activeStep, setActiveStep] = useState<ActiveStepInfo | null>(null);
  const [resetNonce, setResetNonce] = useState(0);
  const [requestedStep, setRequestedStep] = useState<{ step: number; nonce: number } | null>(null);

  // When steps or board changes, reset the active step.
  useEffect(() => {
    setActiveStep(null);
    setActiveCards(null);
    setRequestedStep(null);
  }, [board.key]);

  const pickView = (v: StepsView) => {
    setView(v);
    try {
      localStorage.setItem(VIEW_KEY, v);
    } catch {
      // Only a convenience.
    }
  };

  const copyGuide = async () => {
    if (!steps || steps.length === 0) return;
    const text = formatComboGuide(board, hand, steps, getCard);
    try {
      await navigator.clipboard.writeText(text);
      sound.play("select");
      toast("Guida combo copiata negli appunti", "ok");
    } catch {
      toast("Impossibile copiare negli appunti", "error");
    }
  };

  const currentField: ComboField = activeStep?.field ?? board;
  const pileCodes: Record<PileKind, number[]> = {
    grave: currentField.grave,
    banished: currentField.banished,
    deck: [],
    extra: currentField.extraFaceUp,
  };

  return (
    <div className="flex flex-col gap-4">
      <section className="lab-card flex flex-col items-center gap-3 p-4">
        {activeStep ? (
          <div className="flex w-full items-center justify-between rounded-lg border border-sec/40 bg-sec/10 px-3 py-1.5 text-xs anim-fade">
            <div className="flex min-w-0 items-center gap-2">
              <span className="chip chip-active shrink-0 !px-1.5 !py-0 text-[10px] font-bold">
                {activeStep.number != null ? `Passo ${activeStep.number}` : `Momento ${activeStep.index + 1}`}
              </span>
              <span className="truncate font-medium text-text" title={activeStep.choice ?? undefined}>
                {activeStep.choice ?? "Effetto risolto"}
              </span>
              {activeStep.field && (
                <span className="hidden sm:inline-flex rounded-full border border-sec/30 bg-sec/15 px-1.5 py-0.5 text-[9px] text-sec font-medium">
                  campo al passo
                </span>
              )}
            </div>
            <div className="flex shrink-0 items-center gap-2.5 ml-2">
              <span className="text-[11px] tabular-nums text-muted">
                {activeStep.index + 1} / {activeStep.total}
              </span>
              <button
                type="button"
                onClick={() => {
                  sound.play("toggle");
                  setResetNonce((n) => n + 1);
                }}
                className="btn btn-ghost !px-2 !py-0.5 text-[11px] text-faint hover:text-text"
                title="Torna alla board di fine turno"
              >
                Board finale ✕
              </button>
            </div>
          </div>
        ) : (
          <div className="flex w-full items-center justify-between px-1 text-xs text-muted">
            <span className="font-medium text-text">Fine turno (board finale)</span>
            {!board.exact && (
              <span className="text-[11px] text-faint" title="Effetti della End Phase non ancora giocati">
                Prima della End Phase
              </span>
            )}
          </div>
        )}

        <Field
          board={currentField}
          getCard={getCard}
          onHover={onHover}
          onHoverEnd={onHoverEnd}
          pile={pile}
          onPile={(p) => setPile((cur) => (cur === p ? null : p))}
          highlightedCodes={activeCards}
          label={activeStep ? `Il tuo campo al passo ${activeStep.index + 1}` : "Il tuo campo a fine turno"}
        />

        <div className="w-full border-t border-border pt-3">
          <Pile
            title={activeStep ? `Mano al passo (${currentField.handCards.length})` : `Mano (${currentField.handCards.length})`}
            codes={currentField.handCards}
            highlightedCodes={activeCards}
            getCard={getCard}
            onHover={onHover}
            onHoverEnd={onHoverEnd}
          />
        </div>

        {pile && (
          <div className="w-full anim-fade">
            <Pile
              title={`${PILE_TITLES[pile]}${activeStep ? " al passo" : ""} (${pileCodes[pile].length})`}
              codes={pileCodes[pile]}
              highlightedCodes={activeCards}
              getCard={getCard}
              onHover={onHover}
              onHoverEnd={onHoverEnd}
            />
          </div>
        )}
        {!pile && <p className="w-full text-[11px] text-faint">Tocca Cimitero, bandite o Extra Deck per vedere le carte.</p>}
      </section>

      <section className="lab-card flex flex-col gap-3 p-4">
        <h3 className="text-sm font-semibold">Il punteggio</h3>
        {score ? (
          <>
            {score.cheap && (
              <p className="rounded-md border border-border bg-surface-2 px-3 py-2 text-xs text-muted">
                Valutata in breve: nessun breaker, follow-up, esposizione o confronto con il deck scelto. I numeri qui sotto non si
                confrontano con quelli delle board misurate per intero.
              </p>
            )}
            <ScoreBreakdown s={score} weights={weights} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} board={board} />
            {score.matchup && !score.matchup.error && (
              <RivalMoves
                board={board}
                matchup={score.matchup}
                getCard={getCard}
                onHover={onHover}
                onHoverEnd={onHoverEnd}
              />
            )}
            {score.matchup && !score.matchup.error && score.matchup.handTraps.some((h) => (h.drawn ?? 0) > 0) && (
              <StopTradeOff
                board={board}
                matchup={score.matchup}
                getCard={getCard}
                onHover={onHover}
                onHoverEnd={onHoverEnd}
                onSelectStep={(st) => {
                  setRequestedStep({ step: st, nonce: Date.now() });
                  setView("visual");
                }}
              />
            )}
          </>
        ) : (
          <p className="text-xs text-faint">{scoring ? "Il turno di prova sta mettendo alla prova le board…" : "Questa board non è tra quelle messe alla prova (le migliori 30 della ricerca)."}</p>
        )}
      </section>

      <section className="lab-card flex flex-col gap-3 p-4">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h3 className="text-sm font-semibold">La linea</h3>
          <span className="text-xs text-faint tabular-nums">
            {board.choices} scelte · {board.cardsUsed} {board.cardsUsed === 1 ? "carta usata" : "carte usate"} dalla mano
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <div className="flex gap-1" role="tablist" aria-label="Come mostrare la linea">
            {(["visual", "text"] as const).map((v) => (
              <button key={v} type="button" role="tab" aria-selected={view === v} className={`chip !py-0.5 text-[11px] ${view === v ? "chip-active" : ""}`} onClick={() => pickView(v)}>
                {v === "visual" ? "Visuale" : "Testo"}
              </button>
            ))}
          </div>
          {steps && steps.length > 0 && (
            <button
              type="button"
              onClick={() => void copyGuide()}
              className="btn btn-ghost ml-auto !px-2.5 !py-0.5 text-[11px]"
              title="Copia negli appunti la guida testuale con i nomi delle carte e i passi della linea"
            >
              📋 Copia guida
            </button>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
          Dalla mano:
          {hand.map((c, i) => (
            <span
              key={i}
              className={`chip !py-0.5 ${activeCards?.has(c) ? "border-sec bg-sec/20 text-text" : ""}`}
              onPointerEnter={(e) => onHover(c, e.currentTarget)}
              onPointerLeave={onHoverEnd}
            >
              {getCard(c)?.name ?? c}
            </span>
          ))}
        </div>
        {steps === null ? (
          <p className="text-sm text-muted">Il motore rigioca la linea…</p>
        ) : steps.length === 0 ? (
          <p className="text-sm text-faint">{stepsNote ?? "Nessun passo."}</p>
        ) : view === "visual" ? (
          <ComboSteps
            steps={steps}
            getCard={getCard}
            onHover={onHover}
            onHoverEnd={onHoverEnd}
            onActiveCards={setActiveCards}
            onActiveStep={setActiveStep}
            resetNonce={resetNonce}
            requestedStep={requestedStep}
          />
        ) : (
          <ol className="combo-steps flex flex-col gap-2.5">
            {steps.map((s, i) => (
              <li key={i} className="anim-rise" style={{ animationDelay: `${Math.min(i, 20) * 25}ms` }}>
                {s.choice && <p className="text-sm font-medium text-text">{s.choice}</p>}
                {s.events.length > 0 && (
                  <ul className="mt-0.5 flex flex-col gap-0.5">
                    {s.events.map((e, j) =>
                      PHASE.test(e) ? (
                        <li key={j} className="eyebrow pt-1 text-[10px] text-faint">
                          {e}
                        </li>
                      ) : (
                        <li key={j} className="text-xs text-muted">
                          {e}
                        </li>
                      ),
                    )}
                  </ul>
                )}
              </li>
            ))}
          </ol>
        )}
      </section>
    </div>
  );
}
