import { useState } from "react";
import { cardImage, type Card } from "../../lib/api";
import { MAX_CUSTOM_ROLES, roleDefs, BUILTIN_ROLES } from "../../lib/roles";
import { sound } from "../../lib/sound";

interface Props {
  main: number[];
  roles: Record<string, number[]>;
  getCard: (id: number) => Card | undefined;
  onChange: (roles: Record<string, number[]>) => void;
  onSuggest: () => void;
  /** Progress of the automatic roles, while the engine plays the cards. */
  guessing: { done: number; total: number } | null;
  /** Why each card got its automatic roles. */
  reasons: Record<number, string[]>;
  /** Puts back the roles from before the automatic ones. */
  onUndo: (() => void) | null;
}

/** Side panel: every Main Deck card with a toggle per role. */
export function RolesPanel({ main, roles, getCard, onChange, onSuggest, guessing, reasons, onUndo }: Props) {
  const [custom, setCustom] = useState("");
  const defs = roleDefs(roles);
  const ids = [...new Set(main)];
  const copies = (id: number) => main.filter((x) => x === id).length;
  const count = (key: string) => main.filter((id) => roles[key]?.includes(id)).length;
  const customCount = defs.length - BUILTIN_ROLES.length;

  const toggle = (key: string, id: number) => {
    const current = roles[key] ?? [];
    const on = current.includes(id);
    sound.play(on ? "toggle" : "select");
    onChange({ ...roles, [key]: on ? current.filter((x) => x !== id) : [...current, id] });
  };

  const addRole = () => {
    const key = custom.trim();
    if (!key || defs.some((d) => d.key.toLowerCase() === key.toLowerCase()) || customCount >= MAX_CUSTOM_ROLES) return;
    sound.play("chime");
    onChange({ ...roles, [key]: [] });
    setCustom("");
  };

  const removeRole = (key: string) => {
    const next = { ...roles };
    delete next[key];
    sound.play("close");
    onChange(next);
  };

  return (
    <aside className="flex min-h-0 flex-col border-l border-border bg-surface/80 backdrop-blur">
      <div className="flex flex-col gap-2.5 px-4 pt-4 pb-3">
        <div className="flex items-center justify-between">
          <p className="display text-sm font-bold tracking-[0.12em] uppercase">Ruoli</p>
          <span className="flex gap-1">
            {onUndo && !guessing && (
              <button type="button" className="btn btn-ghost !px-2 !py-1 text-xs" onClick={onUndo} title="Rimetti i ruoli di prima">
                Annulla
              </button>
            )}
            <button
              type="button"
              className="btn btn-ghost !px-2.5 !py-1 text-xs"
              onClick={onSuggest}
              disabled={guessing !== null}
              title="Il motore gioca ogni carta: da sola, accanto allo starter migliore, dalla mano nel turno avversario. I ruoli che trova si possono poi cambiare a mano."
            >
              {guessing ? `Analisi ${guessing.done}/${guessing.total || "…"}` : "✦ Riconosci"}
            </button>
          </span>
        </div>
        {guessing && (
          <div className="h-1 overflow-hidden rounded-full bg-surface-3">
            <div className="h-full bg-[var(--sec)] transition-[width]" style={{ width: `${guessing.total ? (100 * guessing.done) / guessing.total : 4}%` }} />
          </div>
        )}
        <div className="flex flex-wrap gap-1.5">
          {defs.map((d) => (
            <span key={d.key} className="chip group !gap-1.5 !px-2 !py-0.5 !text-[11px]" title={d.hint}>
              <i className="grid h-3.5 w-3.5 place-items-center rounded-full text-[8px] font-bold text-white not-italic" style={{ background: d.color }}>
                {d.letter}
              </i>
              {d.label} <b className="tabular-nums">{count(d.key)}</b>
              {!BUILTIN_ROLES.some((b) => b.key === d.key) && (
                <button type="button" className="text-faint hover:text-danger" onClick={() => removeRole(d.key)} aria-label={`Elimina il ruolo ${d.label}`}>
                  ✕
                </button>
              )}
            </span>
          ))}
        </div>
        {customCount < MAX_CUSTOM_ROLES && (
          <form
            className="flex gap-1.5"
            onSubmit={(e) => {
              e.preventDefault();
              addRole();
            }}
          >
            <input className="field !py-1 text-xs" placeholder="Nuovo ruolo (es. Engine Fiendsmith)" value={custom} onChange={(e) => setCustom(e.target.value)} maxLength={24} />
            <button type="submit" className="btn btn-ghost shrink-0 !px-2.5 !py-1 text-xs" disabled={!custom.trim()}>
              ＋
            </button>
          </form>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
        {ids.length === 0 && <p className="px-2 py-6 text-center text-xs text-faint">Il Main Deck è vuoto.</p>}
        {ids.map((id) => {
          const card = getCard(id);
          return (
            <div key={id} className="flex items-center gap-1.5 xl:gap-2 rounded-lg px-1.5 xl:px-2 py-1 hover:bg-surface-3">
              <img src={cardImage(id, "small")} alt="" loading="lazy" className="h-9 w-[25px] shrink-0 rounded-[2px] object-cover" />
              <span className="min-w-0 flex-1 truncate text-xs" title={[card?.name, ...(reasons[id] ?? [])].filter(Boolean).join("\n")}>
                {card?.name ?? id} <span className="text-faint">×{copies(id)}</span>
                {reasons[id]?.length ? <span className="ml-1 text-[9px] text-faint" aria-label="ruoli trovati dal motore">✦</span> : null}
              </span>
              <span className="flex shrink-0 gap-0.5 xl:gap-1">
                {defs.map((d) => {
                  const on = roles[d.key]?.includes(id) ?? false;
                  return (
                    <button
                      key={d.key}
                      type="button"
                      onClick={() => toggle(d.key, id)}
                      aria-pressed={on}
                      title={`${d.label}${on ? " ✓" : ""}`}
                      className="grid h-[18px] w-[18px] xl:h-5 xl:w-5 place-items-center rounded-full border text-[8.5px] xl:text-[9px] font-bold transition-transform hover:scale-110"
                      style={on ? { background: d.color, borderColor: d.color, color: "#fff" } : { borderColor: "var(--df-border-strong)", color: "var(--df-faint)" }}
                    >
                      {d.letter}
                    </button>
                  );
                })}
              </span>
            </div>
          );
        })}
      </div>
    </aside>
  );
}
