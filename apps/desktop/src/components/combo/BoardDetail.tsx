import type { Card, ComboBoard, ComboStep } from "../../lib/api";
import { CardImage, Field, type HoverCard } from "./Field";

interface Props {
  board: ComboBoard;
  /** The hand the line starts from. */
  hand: number[];
  /** `null` while loading; empty when the steps cannot be told yet. */
  steps: ComboStep[] | null;
  stepsNote: string | null;
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
}

function Pile({ title, codes, getCard, onHover, onHoverEnd }: { title: string; codes: number[]; getCard: (id: number) => Card | undefined; onHover: HoverCard; onHoverEnd: () => void }) {
  return (
    <div className="min-w-0">
      <p className="eyebrow mb-1.5 text-faint">
        {title} · {codes.length}
      </p>
      {codes.length === 0 ? (
        <p className="text-xs text-faint">—</p>
      ) : (
        <div className="flex flex-wrap gap-1">
          {codes.map((c, i) => (
            <span key={i} onPointerEnter={(e) => onHover(c, e.currentTarget)} onPointerLeave={onHoverEnd}>
              <CardImage code={c} card={getCard(c)} className="h-[44px] w-[30px]" />
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

/** Turn and phase changes, shown as quiet separators. */
const PHASE = /^(Inizia il |Draw Phase|Standby Phase|Main Phase|Battle Phase|End Phase)/;

/** A board of the search: the field, the other piles and the line that builds it, told in Italian. */
export function BoardDetail({ board, hand, steps, stepsNote, getCard, onHover, onHoverEnd }: Props) {
  return (
    <div className="flex flex-col gap-4">
      <section className="lab-card flex flex-col items-center gap-4 p-4">
        <Field board={board} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
        <div className="grid w-full grid-cols-3 gap-4">
          <Pile title="Mano" codes={board.handCards} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
          <Pile title="Cimitero" codes={board.grave} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
          <Pile title="Bandite" codes={board.banished} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
        </div>
      </section>

      <section className="lab-card flex flex-col gap-3 p-4">
        <div className="flex items-baseline justify-between gap-3">
          <h3 className="text-sm font-semibold">La linea</h3>
          <span className="text-xs text-faint tabular-nums">
            {board.choices} scelte · {board.cardsUsed} {board.cardsUsed === 1 ? "carta usata" : "carte usate"} dalla mano
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
          Dalla mano:
          {hand.map((c, i) => (
            <span key={i} className="chip !py-0.5" onPointerEnter={(e) => onHover(c, e.currentTarget)} onPointerLeave={onHoverEnd}>
              {getCard(c)?.name ?? c}
            </span>
          ))}
        </div>
        {steps === null ? (
          <p className="text-sm text-muted">Il motore rigioca la linea…</p>
        ) : steps.length === 0 ? (
          <p className="text-sm text-faint">{stepsNote ?? "Nessun passo."}</p>
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
