import { useEffect, useMemo, useRef, useState } from "react";
import type { Card, ComboBoard, ComboField, ComboHappening, ComboHappeningKind, ComboStep, ComboStepKind } from "../../lib/api";
import { sound } from "../../lib/sound";
import { CardImage, formatLinkArrows, type HoverCard } from "./Field";

/** Line-art icons, 24×24, drawn with the current color. */
const ICONS: Record<ComboStepKind | ComboHappeningKind, string[]> = {
  normalSummon: ["M12 18V6", "M7 11l5-5 5 5", "M5 21h14"],
  specialSummon: ["M12 3l2 6.2L20.5 11l-6.5 2L12 19.5 10 13l-6.5-2L10 9.2z"],
  summon: ["M12 18V6", "M7 11l5-5 5 5", "M5 21h14"],
  activate: ["M13 2L4.5 13.5H11L10 22l8.5-11.5H12z"],
  negate: ["M12 3a9 9 0 100 18 9 9 0 000-18z", "M8.5 8.5l7 7", "M15.5 8.5l-7 7"],
  set: ["M6 3.5h12v17H6z", "M6 3.5l12 17"],
  choose: ["M5 12.5l4.2 4.2L19 7"],
  place: ["M4 4h7v7H4z", "M13 4h7v7h-7z", "M4 13h7v7H4z", "M13 13h7v7h-7z"],
  pass: ["M6 12h12"],
  endTurn: ["M5 4v16", "M9 12h11", "M16 8l4 4-4 4"],
  toHand: ["M12 3v12", "M7 10l5 5 5-5", "M4 20h16"],
  toGrave: ["M7 21V10a5 5 0 0110 0v11z", "M5 21h14", "M12 11v6", "M10 13h4"],
  banish: ["M12 3a9 9 0 100 18 9 9 0 000-18z", "M12 7a5 5 0 100 10 5 5 0 000-10z"],
  toDeck: ["M4 8l8-4 8 4-8 4z", "M4 12l8 4 8-4", "M4 16l8 4 8-4"],
  toExtra: ["M4 8l8-4 8 4-8 4z", "M4 12l8 4 8-4", "M4 16l8 4 8-4"],
  material: ["M10 14a4 4 0 005.7 0l3-3a4 4 0 00-5.7-5.7l-1 1", "M14 10a4 4 0 00-5.7 0l-3 3a4 4 0 005.7 5.7l1-1"],
  toField: ["M4 5h16v14H4z", "M12 9v6", "M9 12h6"],
  draw: ["M6 3.5h12v17H6z", "M12 9v6", "M9 12h6"],
  phase: [],
  other: [],
};

export function StepIcon({ kind, className = "h-3.5 w-3.5" }: { kind: ComboStepKind | ComboHappeningKind; className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      {ICONS[kind].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}

const STEP_LABEL: Record<ComboStepKind, string> = {
  normalSummon: "Evocazione Normale",
  specialSummon: "Evocazione Speciale",
  activate: "Attivazione",
  set: "Posizionamento",
  choose: "Scelta",
  place: "Zona",
  pass: "Nessuna risposta",
  endTurn: "Fine del turno",
  other: "Scelta",
};

const HAPPENING_LABEL: Record<ComboHappeningKind, string> = {
  summon: "Evocato",
  activate: "Attivato",
  negate: "Annullato",
  set: "Posizionato",
  toHand: "In mano",
  toGrave: "Al Cimitero",
  banish: "Bandito",
  toDeck: "Nel Deck",
  toExtra: "Nell'Extra Deck",
  material: "Materiale Xyz",
  toField: "Sul Terreno",
  draw: "Pescato",
  phase: "",
  other: "",
};

type StepRowItem = { type: "step"; number: number | null; step: ComboStep | null; happened: ComboHappening[]; playIndex: number };
type Row = { type: "phase"; text: string } | StepRowItem;

/** What is worth a picture: a summon already says the card is on the field. */
function shown(happened: ComboHappening[]): ComboHappening[] {
  const placed = new Set(happened.filter((h) => h.kind === "summon" || h.kind === "set").map((h) => h.code));
  return happened.filter((h) => {
    if (h.kind === "other" || h.kind === "phase") return false;
    if (h.kind === "toField" && placed.has(h.code)) return false;
    return h.code !== 0 || h.kind === "negate";
  });
}

/**
 * The steps as rows: each choice with what followed it, the turns and
 * phases as separators (consecutive ones merged). Choices that do nothing
 * ("don't chain") are left out unless `all`.
 */
function rows(steps: ComboStep[], all: boolean): { list: Row[]; stepList: StepRowItem[]; quiet: number } {
  const out: Row[] = [];
  const stepList: StepRowItem[] = [];
  let number = 0;
  let quiet = 0;
  for (const step of steps) {
    if (step.choice) number += 1;
    let current: StepRowItem = {
      type: "step",
      number: step.choice ? number : null,
      step: step.choice ? step : null,
      happened: [],
      playIndex: -1,
    };
    const flush = () => {
      const silent = current.step?.kind === "pass" && shown(current.happened).length === 0;
      if (silent) quiet += 1;
      if ((current.step && (all || !silent)) || shown(current.happened).length > 0) {
        current.playIndex = stepList.length;
        stepList.push(current);
        out.push(current);
      }
    };
    for (const h of step.happened) {
      if (h.kind === "phase") {
        flush();
        const last = out[out.length - 1];
        if (last?.type === "phase") last.text += ` · ${h.text}`;
        else out.push({ type: "phase", text: h.text });
        current = { type: "step", number: null, step: null, happened: [], playIndex: -1 };
      } else {
        current.happened.push(h);
      }
    }
    flush();
  }
  return { list: out, stepList, quiet };
}

/**
 * Formats a plain-text combo guide with card names and steps only
 * (never official Konami card effect texts).
 */
export function formatComboGuide(
  board: ComboBoard,
  hand: number[],
  steps: ComboStep[],
  getCard: (id: number) => Card | undefined,
): string {
  const cardName = (code: number) => getCard(code)?.name ?? `#${code}`;
  const lines: string[] = [];

  if (hand.length > 0) {
    lines.push(`Mano iniziale: ${hand.map(cardName).join(", ")}`);
  }

  const monsters = board.monsters
    .map((z, i) => {
      if (!z) return null;
      const zoneTag = i >= 5 ? "EMZ" : `M${i + 1}`;
      const arrows = z.link > 0 && z.arrows > 0 ? ` [${formatLinkArrows(z.arrows)}]` : "";
      const pos = !z.faceUp ? "coperta" : z.link ? `L${z.link} ${z.attack}${arrows}` : z.defense ? `DEF ${z.def}` : `ATK ${z.attack}`;
      return `${cardName(z.code)} (${zoneTag}, ${pos})`;
    })
    .filter((x): x is string => x !== null);

  const spells = board.spells
    .slice(0, 6)
    .map((z, i) => {
      if (!z) return null;
      const zoneTag = i === 5 ? "Terreno" : `S${i + 1}`;
      return `${cardName(z.code)} (${zoneTag}${z.faceUp ? "" : ", coperta"})`;
    })
    .filter((x): x is string => x !== null);

  if (monsters.length > 0) lines.push(`Mostri a fine turno: ${monsters.join(" · ")}`);
  if (spells.length > 0) lines.push(`Magie/Trappole a fine turno: ${spells.join(" · ")}`);
  if (board.handCards.length > 0) lines.push(`Carte in mano a fine turno: ${board.handCards.map(cardName).join(", ")}`);
  if (board.grave.length > 0) lines.push(`Cimitero (${board.grave.length}): ${board.grave.map(cardName).join(", ")}`);

  const { list } = rows(steps, false);
  if (list.length > 0) {
    lines.push("");
    lines.push(`Linea (${board.choices} scelte · ${board.cardsUsed} ${board.cardsUsed === 1 ? "carta usata" : "carte usate"} dalla mano):`);
    for (const r of list) {
      if (r.type === "phase") {
        lines.push(`--- ${r.text} ---`);
      } else {
        const prefix = r.number != null ? `${r.number}. ` : "• ";
        const head = r.step?.choice ?? "Effetti risolti";
        const sub = shown(r.happened)
          .map((h) => {
            const label = HAPPENING_LABEL[h.kind] || h.kind;
            const cName = h.code !== 0 ? cardName(h.code) : "";
            const chain = h.chainLink > 0 ? ` [C${h.chainLink}]` : "";
            return cName ? `${label}: ${cName}${chain}` : `${label}${chain}`;
          })
          .join(" · ");
        lines.push(sub ? `${prefix}${head} (${sub})` : `${prefix}${head}`);
      }
    }
  }

  return lines.join("\n");
}

function Happened({ h, getCard, onHover, onHoverEnd }: { h: ComboHappening; getCard: (id: number) => Card | undefined; onHover: HoverCard; onHoverEnd: () => void }) {
  const label = HAPPENING_LABEL[h.kind];
  return (
    <span
      className={`combo-happened ${h.mine ? "" : "combo-happened--theirs"}`}
      data-kind={h.kind}
      title={h.text}
      onPointerEnter={h.code ? (e) => onHover(h.code, e.currentTarget) : undefined}
      onPointerLeave={h.code ? onHoverEnd : undefined}
    >
      {h.code !== 0 && <CardImage code={h.code} card={getCard(h.code)} className="h-[38px] w-[26px]" />}
      <span className="combo-happened-icon" aria-label={label}>
        <StepIcon kind={h.kind} className="h-2.5 w-2.5" />
      </span>
      {h.chainLink > 0 && <span className="combo-happened-link">C{h.chainLink}</span>}
    </span>
  );
}

export interface ActiveStepInfo {
  index: number;
  total: number;
  number: number | null;
  choice: string | null;
  kind?: string | null;
  field: ComboField | null;
}

/** The line told with pictures and an interactive step-by-step player. */
export function ComboSteps({
  steps,
  getCard,
  onHover,
  onHoverEnd,
  onActiveCards,
  onActiveStep,
  resetNonce,
}: {
  steps: ComboStep[];
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
  onActiveCards?: (codes: Set<number> | null) => void;
  onActiveStep?: (step: ActiveStepInfo | null) => void;
  resetNonce?: number;
}) {
  const [all, setAll] = useState(false);
  const [activeIdx, setActiveIdx] = useState<number | null>(null);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState<1 | 2>(1);
  const stepRefs = useRef<Map<number, HTMLLIElement>>(new Map());

  const { list, stepList, quiet } = useMemo(() => rows(steps, all), [steps, all]);
  const totalSteps = stepList.length;

  // Reset player when the line changes.
  useEffect(() => {
    setActiveIdx(null);
    setPlaying(false);
  }, [steps]);

  // Handle external reset request (e.g. clicking "Board finale" on the field).
  useEffect(() => {
    if (resetNonce) {
      setActiveIdx(null);
      setPlaying(false);
    }
  }, [resetNonce]);

  // Auto-advance timer when playing.
  useEffect(() => {
    if (!playing || totalSteps === 0) return;
    const intervalMs = speed === 2 ? 950 : 1800;
    const timer = window.setTimeout(() => {
      setActiveIdx((cur) => {
        const next = cur == null ? 0 : cur + 1;
        if (next >= totalSteps) {
          setPlaying(false);
          return totalSteps - 1;
        }
        sound.play("select");
        return next;
      });
    }, intervalMs);
    return () => window.clearTimeout(timer);
  }, [playing, activeIdx, totalSteps, speed]);

  // Notify parent of cards involved in the active step, step field, and keep the active row in view.
  useEffect(() => {
    if (activeIdx == null || !stepList[activeIdx]) {
      onActiveCards?.(null);
      onActiveStep?.(null);
      return;
    }
    const row = stepList[activeIdx];
    const codes = new Set<number>();
    if (row.step?.card) codes.add(row.step.card);
    for (const h of row.happened) {
      if (h.code !== 0) codes.add(h.code);
    }
    onActiveCards?.(codes.size > 0 ? codes : null);

    // Find the field after this step: either row.step.field, or the nearest previous step's field.
    let field: ComboField | null = null;
    if (row.step?.field) {
      field = row.step.field;
    } else {
      for (let i = activeIdx; i >= 0; i--) {
        const f = stepList[i]?.step?.field;
        if (f) {
          field = f;
          break;
        }
      }
    }

    onActiveStep?.({
      index: activeIdx,
      total: totalSteps,
      number: row.number,
      choice: row.step?.choice ?? (row.happened[0]?.text ?? null),
      kind: row.step?.kind ?? null,
      field,
    });

    const el = stepRefs.current.get(activeIdx);
    el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [activeIdx, stepList, onActiveCards, onActiveStep, totalSteps]);

  // Keyboard shortcuts [ and ] to step backward/forward.
  useEffect(() => {
    if (totalSteps === 0) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.closest("input, textarea, select")) return;
      if (e.key === "[") {
        e.preventDefault();
        setPlaying(false);
        sound.play("select");
        setActiveIdx((cur) => (cur == null ? 0 : Math.max(0, cur - 1)));
      } else if (e.key === "]") {
        e.preventDefault();
        setPlaying(false);
        sound.play("select");
        setActiveIdx((cur) => (cur == null ? 0 : Math.min(totalSteps - 1, cur + 1)));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [totalSteps]);

  const activeRow = activeIdx != null ? stepList[activeIdx] : null;

  const togglePlay = () => {
    if (totalSteps === 0) return;
    sound.play("toggle");
    if (playing) {
      setPlaying(false);
    } else {
      if (activeIdx == null || activeIdx >= totalSteps - 1) setActiveIdx(0);
      setPlaying(true);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      {totalSteps > 0 && (
        <div className="flex flex-col gap-2 rounded-xl border border-border bg-surface-2/80 p-2.5">
          <div className="flex flex-wrap items-center gap-1.5 text-xs">
            <button
              type="button"
              className="btn btn-ghost !px-2 !py-1 text-xs"
              disabled={activeIdx === 0}
              onClick={() => {
                sound.play("select");
                setPlaying(false);
                setActiveIdx(0);
              }}
              title="Primo passo"
              aria-label="Primo passo"
            >
              ⏮
            </button>
            <button
              type="button"
              className="btn btn-ghost !px-2 !py-1 text-xs"
              disabled={activeIdx === 0}
              onClick={() => {
                sound.play("select");
                setPlaying(false);
                setActiveIdx((cur) => (cur == null ? 0 : Math.max(0, cur - 1)));
              }}
              title="Passo precedente ([)"
              aria-label="Passo precedente"
            >
              ◀
            </button>
            <button
              type="button"
              className={`btn !px-3 !py-1 text-xs font-semibold ${playing ? "btn-molten" : "border-sec/50 bg-sec/15 text-text hover:border-sec"}`}
              onClick={togglePlay}
              title={playing ? "Metti in pausa il replay" : "Riproduci la linea passo per passo"}
            >
              {playing ? "⏸ Pausa" : "▶ Passo a passo"}
            </button>
            <button
              type="button"
              className="btn btn-ghost !px-2 !py-1 text-xs"
              disabled={activeIdx != null && activeIdx >= totalSteps - 1}
              onClick={() => {
                sound.play("select");
                setPlaying(false);
                setActiveIdx((cur) => (cur == null ? 0 : Math.min(totalSteps - 1, cur + 1)));
              }}
              title="Passo successivo (])"
              aria-label="Passo successivo"
            >
              ▶
            </button>
            <button
              type="button"
              className="btn btn-ghost !px-2 !py-1 text-xs"
              disabled={activeIdx === totalSteps - 1}
              onClick={() => {
                sound.play("select");
                setPlaying(false);
                setActiveIdx(totalSteps - 1);
              }}
              title="Ultimo passo"
              aria-label="Ultimo passo"
            >
              ⏭
            </button>

            <button
              type="button"
              onClick={() => {
                sound.play("toggle");
                setSpeed((s) => (s === 1 ? 2 : 1));
              }}
              className="chip !px-2 !py-0.5 text-[11px] tabular-nums"
              title="Velocità di riproduzione"
            >
              {speed}×
            </button>

            <span className="ml-auto text-[11px] font-semibold tabular-nums text-muted" title="Posizione nella linea, contando anche gli effetti che si risolvono da soli">
              {activeIdx != null ? `${activeIdx + 1} / ${totalSteps}` : `${totalSteps} momenti`}
            </span>

            {activeIdx != null && (
              <button
                type="button"
                onClick={() => {
                  sound.play("close");
                  setPlaying(false);
                  setActiveIdx(null);
                }}
                className="text-[11px] text-faint hover:text-text"
                title="Mostra tutta la linea senza filtro passo"
              >
                ✕ Tutti
              </button>
            )}
          </div>

          {/* Interactive step scrubber */}
          <div className="grid gap-1" style={{ gridTemplateColumns: `repeat(${totalSteps}, minmax(0, 1fr))` }}>
            {stepList.map((s, idx) => {
              const isCurrent = activeIdx === idx;
              const isDone = activeIdx != null && idx < activeIdx;
              return (
                <button
                  key={idx}
                  type="button"
                  onClick={() => {
                    sound.play("select");
                    setPlaying(false);
                    setActiveIdx(idx);
                  }}
                  title={s.step?.choice ? `${idx + 1}. ${s.step.choice}` : `Passo ${idx + 1}`}
                  aria-label={`Vai al passo ${idx + 1}`}
                  className={`h-1.5 rounded-full transition-all ${
                    isCurrent ? "bg-sec shadow-[0_0_8px_var(--sec)]" : isDone ? "bg-sec/45" : "bg-surface-3 hover:bg-border-strong"
                  }`}
                />
              );
            })}
          </div>

          {/* Active step spotlight */}
          {activeRow && (
            <div className="flex items-start gap-2.5 rounded-lg border border-sec/40 bg-bg-2/80 p-2.5 text-xs anim-fade">
              {activeRow.step?.card ? (
                <span className="shrink-0" onPointerEnter={(e) => onHover(activeRow.step!.card!, e.currentTarget)} onPointerLeave={onHoverEnd}>
                  <CardImage code={activeRow.step.card} card={getCard(activeRow.step.card)} className="h-[54px] w-[38px] shadow-md" />
                </span>
              ) : null}
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <div className="flex items-center gap-1.5">
                  <span className="chip chip-active !px-1.5 !py-0 text-[10px]">
                    {activeRow.number != null ? `Scelta ${activeRow.number}` : "Effetti risolti"}
                  </span>
                  {activeRow.step?.kind && <span className="text-[11px] text-muted">{STEP_LABEL[activeRow.step.kind]}</span>}
                </div>
                {activeRow.step?.choice && <p className="font-semibold leading-snug text-text">{activeRow.step.choice}</p>}
                {shown(activeRow.happened).length > 0 && (
                  <div className="mt-0.5 flex flex-wrap gap-1.5">
                    {shown(activeRow.happened).map((h, j) => (
                      <span
                        key={j}
                        className="inline-flex items-center gap-1 rounded bg-surface-3 px-1.5 py-0.5 text-[11px] text-muted"
                        onPointerEnter={h.code ? (e) => onHover(h.code, e.currentTarget) : undefined}
                        onPointerLeave={h.code ? onHoverEnd : undefined}
                      >
                        <StepIcon kind={h.kind} className="h-3 w-3 text-sec" />
                        <span>{HAPPENING_LABEL[h.kind]}:</span>
                        <b className="text-text">{h.code ? (getCard(h.code)?.name ?? h.code) : ""}</b>
                        {h.chainLink > 0 && <span className="text-[10px] text-sec">C{h.chainLink}</span>}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      )}

      <ol className="combo-timeline">
        {list.map((row, i) => {
          if (row.type === "phase") {
            return (
              <li key={i} className="combo-timeline-phase">
                {row.text}
              </li>
            );
          }
          const isCurrent = activeIdx === row.playIndex;
          const isFuture = activeIdx != null && row.playIndex > activeIdx;
          return (
            <li
              key={i}
              ref={(el) => {
                if (el) stepRefs.current.set(row.playIndex, el);
                else stepRefs.current.delete(row.playIndex);
              }}
              onClick={() => {
                sound.play("select");
                setPlaying(false);
                setActiveIdx((cur) => (cur === row.playIndex ? null : row.playIndex));
              }}
              className={`combo-timeline-step cursor-pointer rounded-lg transition-all anim-rise ${
                isCurrent
                  ? "bg-sec/15 ring-1 ring-sec/60"
                  : isFuture
                    ? "opacity-50 hover:opacity-85"
                    : "hover:bg-surface-2/60"
              }`}
              style={{ animationDelay: `${Math.min(i, 20) * 20}ms`, paddingLeft: "2rem", paddingRight: "0.5rem", paddingTop: "0.35rem", paddingBottom: "0.35rem" }}
            >
              <span
                className={`combo-timeline-dot ${isCurrent ? "ring-2 ring-[var(--sec)]" : ""}`}
                data-kind={row.step?.kind ?? "other"}
                title={row.step ? STEP_LABEL[row.step.kind ?? "other"] : undefined}
              >
                {row.step ? <StepIcon kind={row.step.kind ?? "other"} /> : null}
              </span>
              <div className="flex min-w-0 flex-1 items-start gap-2.5">
                {row.step?.card ? (
                  <span className="shrink-0" onPointerEnter={(e) => onHover(row.step!.card!, e.currentTarget)} onPointerLeave={onHoverEnd}>
                    <CardImage code={row.step.card} card={getCard(row.step.card)} className="h-[52px] w-[36px] shadow-md" />
                  </span>
                ) : null}
                <div className="flex min-w-0 flex-1 flex-col gap-1">
                  {row.step && (
                    <p className="text-[13px] leading-snug text-text">
                      {row.number !== null && <span className="mr-1.5 text-[11px] font-semibold tabular-nums text-faint">{row.number}.</span>}
                      {row.step.choice}
                    </p>
                  )}
                  {shown(row.happened).length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {shown(row.happened).map((h, j) => (
                        <Happened key={j} h={h} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </li>
          );
        })}
      </ol>
      {quiet > 0 && (
        <button type="button" className="self-start text-[11px] text-faint underline-offset-2 hover:text-muted hover:underline" onClick={() => setAll((v) => !v)}>
          {all ? "Nascondi" : "Mostra"} {quiet === 1 ? "la scelta di non rispondere" : `le ${quiet} scelte di non rispondere`}
        </button>
      )}
    </div>
  );
}
