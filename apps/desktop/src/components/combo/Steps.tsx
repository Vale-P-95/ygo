import { useState } from "react";
import type { Card, ComboHappening, ComboHappeningKind, ComboStep, ComboStepKind } from "../../lib/api";
import { CardImage, type HoverCard } from "./Field";

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

type Row =
  | { type: "phase"; text: string }
  | { type: "step"; number: number | null; step: ComboStep | null; happened: ComboHappening[] };

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
function rows(steps: ComboStep[], all: boolean): { list: Row[]; quiet: number } {
  const out: Row[] = [];
  let number = 0;
  // The choices left out, counted by the same rule that leaves them out.
  let quiet = 0;
  for (const step of steps) {
    if (step.choice) number += 1;
    let current: Row & { type: "step" } = { type: "step", number: step.choice ? number : null, step: step.choice ? step : null, happened: [] };
    const flush = () => {
      const silent = current.step?.kind === "pass" && shown(current.happened).length === 0;
      if (silent) quiet += 1;
      if ((current.step && (all || !silent)) || shown(current.happened).length > 0) out.push(current);
    };
    for (const h of step.happened) {
      if (h.kind === "phase") {
        flush();
        const last = out[out.length - 1];
        if (last?.type === "phase") last.text += ` · ${h.text}`;
        else out.push({ type: "phase", text: h.text });
        current = { type: "step", number: null, step: null, happened: [] };
      } else {
        current.happened.push(h);
      }
    }
    flush();
  }
  return { list: out, quiet };
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

/** The line told with pictures: each choice, the card it is about, and what happened to which cards. */
export function ComboSteps({ steps, getCard, onHover, onHoverEnd }: { steps: ComboStep[]; getCard: (id: number) => Card | undefined; onHover: HoverCard; onHoverEnd: () => void }) {
  const [all, setAll] = useState(false);
  const { list, quiet } = rows(steps, all);
  return (
    <div className="flex flex-col gap-2">
      <ol className="combo-timeline">
        {list.map((row, i) =>
          row.type === "phase" ? (
            <li key={i} className="combo-timeline-phase">
              {row.text}
            </li>
          ) : (
            <li key={i} className="combo-timeline-step anim-rise" style={{ animationDelay: `${Math.min(i, 20) * 20}ms` }}>
              <span className="combo-timeline-dot" data-kind={row.step?.kind ?? "other"} title={row.step ? STEP_LABEL[row.step.kind ?? "other"] : undefined}>
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
          ),
        )}
      </ol>
      {quiet > 0 && (
        <button type="button" className="self-start text-[11px] text-faint underline-offset-2 hover:text-muted hover:underline" onClick={() => setAll((v) => !v)}>
          {all ? "Nascondi" : "Mostra"} {quiet === 1 ? "la scelta di non rispondere" : `le ${quiet} scelte di non rispondere`}
        </button>
      )}
    </div>
  );
}
