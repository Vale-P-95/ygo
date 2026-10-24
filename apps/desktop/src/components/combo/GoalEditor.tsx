import { useState } from "react";
import { CardImage } from "./Field";
import type { Card, GoalCard, GoalZone } from "../../lib/api";
import { GOAL_ZONES, MAX_GOAL_CARDS, isHard } from "../../lib/goal";
import { sound } from "../../lib/sound";

export interface GoalValue {
  cards: GoalCard[];
  all: boolean;
  first: boolean;
}

interface Props {
  value: GoalValue;
  onChange: (v: GoalValue) => void;
  main: number[];
  extra: number[];
  cards: { get: (code: number) => Card | undefined };
  name: (code: number) => string;
  onHover: (code: number, el: HTMLElement) => void;
  onHoverEnd: () => void;
}

/** The goal of a search: a place for each card, hard (needed) or soft (only ranks the boards higher). */
export function GoalEditor({ value, onChange, main, extra, cards, name, onHover, onHoverEnd }: Props) {
  const [picker, setPicker] = useState<"extra" | "main" | null>(null);
  const uniqueMain = [...new Set(main)];
  const uniqueExtra = [...new Set(extra)];
  const copies = (code: number) => main.filter((c) => c === code).length + extra.filter((c) => c === code).length;
  const hardCount = value.cards.filter(isHard).length;

  const patch = (i: number, p: Partial<GoalCard>) => onChange({ ...value, cards: value.cards.map((g, j) => (j === i ? { ...g, ...p } : g)) });
  const add = (code: number) => {
    if (value.cards.filter((g) => g.code === code).length >= copies(code) || value.cards.length >= MAX_GOAL_CARDS) return sound.play("error");
    sound.play("select");
    onChange({ ...value, cards: [...value.cards, { code, zone: "any" }] });
  };

  return (
    <section className="flex flex-col gap-2" aria-label="Obiettivo">
      <div className="flex items-baseline justify-between">
        <h3 className="text-sm font-semibold">Obiettivo</h3>
        <span className="text-xs text-faint">facoltativo</span>
      </div>
      <p className="text-xs leading-relaxed text-faint">
        Carte da avere a fine turno, ognuna nel suo posto. Le obbligatorie decidono se l'obiettivo è raggiunto; le preferite alzano solo la classifica.
      </p>
      {value.cards.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {value.cards.map((g, i) => {
            const hard = isHard(g);
            const late = GOAL_ZONES.find((z) => z.zone === g.zone)?.late;
            return (
              <li key={i} className="flex items-center gap-2 rounded-lg border border-border bg-bg-2 p-1.5" onPointerEnter={(e) => onHover(g.code, e.currentTarget)} onPointerLeave={onHoverEnd}>
                <CardImage code={g.code} card={cards.get(g.code)} className="h-[34px] w-[23px] shrink-0" />
                <div className="flex min-w-0 flex-1 flex-col gap-1">
                  <span className="truncate text-xs font-semibold">{name(g.code)}</span>
                  <div className="flex items-center gap-1.5">
                    <select
                      className="field !w-auto min-w-0 flex-1 !py-0.5 text-xs"
                      aria-label={`Dove vuoi ${name(g.code)}`}
                      title={GOAL_ZONES.find((z) => z.zone === g.zone)?.hint}
                      value={g.zone}
                      onChange={(e) => patch(i, { zone: e.target.value as GoalZone })}
                    >
                      {GOAL_ZONES.map((z) => (
                        <option key={z.zone} value={z.zone}>
                          {z.label}
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={hard}
                      title={hard ? "Obbligatoria: serve per raggiungere l'obiettivo. Tocca per renderla preferita." : "Preferita: alza la classifica ma non serve. Tocca per renderla obbligatoria."}
                      className={`chip !py-0.5 text-xs ${hard ? "chip-active" : ""}`}
                      onClick={() => {
                        sound.play("toggle");
                        patch(i, { hard: !hard });
                      }}
                    >
                      {hard ? "obbligatoria" : "preferita"}
                    </button>
                  </div>
                  {late && <span className="text-[11px] text-faint">Conta solo sulle board con la Fase Finale giocata.</span>}
                </div>
                <button
                  type="button"
                  aria-label={`Togli ${name(g.code)} dall'obiettivo`}
                  className="px-1 text-faint hover:text-danger"
                  onClick={() => {
                    sound.play("close");
                    onChange({ ...value, cards: value.cards.filter((_, j) => j !== i) });
                  }}
                >
                  ×
                </button>
              </li>
            );
          })}
        </ul>
      )}
      <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
        {(
          [
            ["extra", "+ dall'Extra Deck"],
            ["main", "+ dal Main Deck"],
          ] as const
        ).map(([where, label]) => (
          <button
            key={where}
            type="button"
            className={`chip !py-0.5 ${picker === where ? "chip-active" : ""}`}
            onClick={() => {
              sound.play(picker === where ? "close" : "open");
              setPicker((p) => (p === where ? null : where));
            }}
            aria-expanded={picker === where}
          >
            {label}
          </button>
        ))}
      </div>
      {picker && (
        <div className="grid grid-cols-6 gap-1 rounded-lg border border-border bg-bg-2 p-1.5 anim-fade">
          {(picker === "extra" ? uniqueExtra : uniqueMain).map((id) => (
            <button key={id} type="button" title={name(id)} onClick={() => add(id)} onPointerEnter={(e) => onHover(id, e.currentTarget)} onPointerLeave={onHoverEnd}>
              <CardImage code={id} card={cards.get(id)} className="aspect-[59/86] w-full" />
            </button>
          ))}
        </div>
      )}
      {hardCount > 1 && (
        <div className="flex gap-1 self-start rounded-lg border border-border bg-bg-2 p-0.5 text-xs" role="radiogroup" aria-label="Quante carte obbligatorie">
          {(
            [
              [true, "tutte"],
              [false, "almeno una"],
            ] as const
          ).map(([all, label]) => (
            <button
              key={label}
              type="button"
              role="radio"
              aria-checked={value.all === all}
              onClick={() => onChange({ ...value, all })}
              className={`rounded-md px-2 py-0.5 font-semibold ${value.all === all ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
            >
              {label}
            </button>
          ))}
        </div>
      )}
      {value.cards.length > 0 && (
        <label className="flex items-center gap-2 text-xs text-muted">
          <input type="checkbox" className="accent-[var(--sec)]" checked={value.first} onChange={(e) => onChange({ ...value, first: e.target.checked })} />
          Fermati alla prima linea che ci arriva
        </label>
      )}
      {value.cards.length > 0 && hardCount === 0 && <p className="text-xs text-warn">Tutte le carte sono preferite: serve almeno una obbligatoria perché l'obiettivo possa dirsi raggiunto.</p>}
    </section>
  );
}
