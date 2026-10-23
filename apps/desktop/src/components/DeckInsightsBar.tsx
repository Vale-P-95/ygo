import { useMemo, useState } from "react";
import type { Card, Deck } from "../lib/api";
import { roleDefs } from "../lib/roles";
import { sound } from "../lib/sound";

interface Props {
  deck: Deck;
  getCard: (id: number) => Card | undefined;
  activeRole: string | null;
  onSelectRole: (roleKey: string | null) => void;
  guessing: { done: number; total: number } | null;
  onSuggestRoles: () => void;
  onUndoRoles: (() => void) | null;
}

const SPELL_RACES: { key: string; label: string }[] = [
  { key: "Normal", label: "Normali" },
  { key: "Quick-Play", label: "Rapide" },
  { key: "Continuous", label: "Continue" },
  { key: "Field", label: "Terreno" },
  { key: "Equip", label: "Equip." },
  { key: "Ritual", label: "Rituale" },
];

const TRAP_RACES: { key: string; label: string }[] = [
  { key: "Normal", label: "Normali" },
  { key: "Continuous", label: "Continue" },
  { key: "Counter", label: "Contro-Trappola" },
];

export function DeckInsightsBar({
  deck,
  getCard,
  activeRole,
  onSelectRole,
  guessing,
  onSuggestRoles,
  onUndoRoles,
}: Props) {
  const [distOpen, setDistOpen] = useState(false);
  const defs = useMemo(() => roleDefs(deck.roles ?? {}), [deck.roles]);

  const roleCounts = useMemo(() => {
    const out: Record<string, number> = {};
    for (const d of defs) {
      const set = new Set(deck.roles?.[d.key] ?? []);
      out[d.key] = deck.main.filter((id) => set.has(id)).length;
    }
    return out;
  }, [deck.main, deck.roles, defs]);

  // Computed on render (<= 75 cards) so it updates as soon as useCardCache loads card details.
  const levels = new Map<number, number>();
  const spells = new Map<string, number>();
  const traps = new Map<string, number>();
  const extraLinks = new Map<number, number>();
  const extraLevels = new Map<number, number>();

  for (const id of deck.main) {
    const c = getCard(id);
    if (!c) continue;
    if (c.kind === "monster" && c.level != null) {
      levels.set(c.level, (levels.get(c.level) ?? 0) + 1);
    } else if (c.kind === "spell") {
      spells.set(c.race || "Normal", (spells.get(c.race || "Normal") ?? 0) + 1);
    } else if (c.kind === "trap") {
      traps.set(c.race || "Normal", (traps.get(c.race || "Normal") ?? 0) + 1);
    }
  }

  for (const id of deck.extra) {
    const c = getCard(id);
    if (!c) continue;
    if (c.linkRating != null) {
      extraLinks.set(c.linkRating, (extraLinks.get(c.linkRating) ?? 0) + 1);
    } else if (c.level != null) {
      extraLevels.set(c.level, (extraLevels.get(c.level) ?? 0) + 1);
    }
  }

  const levelEntries = [...levels.entries()].sort((a, b) => a[0] - b[0]);
  const maxLevelCount = Math.max(1, ...levelEntries.map(([, n]) => n));
  const dist = { levelEntries, maxLevelCount, spells, traps, extraLinks, extraLevels };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-1.5 rounded-xl border border-border bg-surface/75 px-3 py-2 backdrop-blur">
        <span className="eyebrow mr-1 text-xs">Ruoli</span>
        {defs.map((d) => {
          const n = roleCounts[d.key] ?? 0;
          const active = activeRole === d.key;
          return (
            <button
              key={d.key}
              type="button"
              onClick={() => {
                sound.play("toggle");
                onSelectRole(active ? null : d.key);
              }}
              aria-pressed={active}
              title={`${d.label}: ${d.hint}. Clicca per evidenziare le carte sul banco.`}
              className={`chip !gap-1.5 !px-2.5 !py-0.5 text-xs transition ${
                active ? "border-sec bg-sec/20 text-text shadow-sm" : "hover:border-border-strong"
              }`}
            >
              <i className="grid h-3.5 w-3.5 place-items-center rounded-full text-[8px] font-bold text-white not-italic" style={{ background: d.color }}>
                {d.letter}
              </i>
              <span>{d.label}</span>
              <b className="tabular-nums">{n}</b>
            </button>
          );
        })}

        {activeRole && (
          <button
            type="button"
            onClick={() => {
              sound.play("close");
              onSelectRole(null);
            }}
            className="text-xs text-muted underline hover:text-text"
          >
            Mostra tutte
          </button>
        )}

        <div className="ml-auto flex items-center gap-1.5">
          {onUndoRoles && !guessing && (
            <button type="button" onClick={onUndoRoles} className="btn btn-ghost !px-2 !py-1 text-xs" title="Ripristina i ruoli precedenti">
              Annulla ruoli
            </button>
          )}
          <button
            type="button"
            disabled={guessing !== null || deck.main.length === 0}
            onClick={onSuggestRoles}
            className="btn btn-ghost !px-2.5 !py-1 text-xs"
            title="Il motore gioca ogni carta del Main Deck per riconoscere Starter, Estensori, Handtrap, Board breaker e Brick"
          >
            {guessing ? `Analisi ${guessing.done}/${guessing.total || "…"}` : "✦ Riconosci"}
          </button>
          <button
            type="button"
            onClick={() => {
              sound.play(distOpen ? "close" : "open");
              setDistOpen((v) => !v);
            }}
            aria-expanded={distOpen}
            className={`btn !px-2.5 !py-1 text-xs ${distOpen ? "border-sec bg-surface-3 text-text" : "btn-ghost"}`}
          >
            📊 Distribuzione
          </button>
        </div>
      </div>

      {guessing && (
        <div className="h-1 overflow-hidden rounded-full bg-surface-3">
          <div
            className="h-full bg-[var(--sec)] transition-[width]"
            style={{ width: `${guessing.total ? (100 * guessing.done) / guessing.total : 6}%` }}
          />
        </div>
      )}

      {distOpen && (
        <div className="panel grid grid-cols-3 gap-4 p-3.5 text-xs anim-fade">
          {/* 1. Monster Level Curve */}
          <div className="flex flex-col gap-2">
            <p className="eyebrow text-xs">Curva livelli mostri (Main Deck)</p>
            {dist.levelEntries.length === 0 ? (
              <p className="text-faint">Nessun mostro nel Main Deck.</p>
            ) : (
              <div className="flex items-end gap-2 pt-2" style={{ height: 78 }}>
                {dist.levelEntries.map(([lvl, count]) => {
                  const pct = Math.max(12, Math.round((count / dist.maxLevelCount) * 100));
                  return (
                    <div key={lvl} className="flex flex-1 flex-col items-center gap-1">
                      <span className="text-xs font-semibold tabular-nums text-text">{count}</span>
                      <div className="flex h-11 w-full items-end rounded bg-bg-2 p-0.5">
                        <div className="w-full rounded-sm bg-gold transition-all" style={{ height: `${pct}%` }} />
                      </div>
                      <span className="text-xs text-muted tabular-nums">★{lvl}</span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* 2. Spells & Traps Subtypes */}
          <div className="flex flex-col gap-2 border-l border-border pl-4">
            <p className="eyebrow text-xs">Sottotipi Magie e Trappole</p>
            <div className="flex flex-col gap-1.5">
              <div className="flex flex-wrap items-center gap-1">
                <span className="w-14 shrink-0 text-xs font-semibold text-[#1d9e74]">Magie</span>
                {SPELL_RACES.filter((r) => (dist.spells.get(r.key) ?? 0) > 0).map((r) => (
                  <span key={r.key} className="chip !px-2 !py-0 text-xs">
                    {r.label} <b className="tabular-nums">{dist.spells.get(r.key)}</b>
                  </span>
                ))}
                {[...dist.spells.values()].reduce((a, b) => a + b, 0) === 0 && <span className="text-faint">—</span>}
              </div>
              <div className="flex flex-wrap items-center gap-1">
                <span className="w-14 shrink-0 text-xs font-semibold text-[#b8457f]">Trappole</span>
                {TRAP_RACES.filter((r) => (dist.traps.get(r.key) ?? 0) > 0).map((r) => (
                  <span key={r.key} className="chip !px-2 !py-0 text-xs">
                    {r.label} <b className="tabular-nums">{dist.traps.get(r.key)}</b>
                  </span>
                ))}
                {[...dist.traps.values()].reduce((a, b) => a + b, 0) === 0 && <span className="text-faint">—</span>}
              </div>
            </div>
          </div>

          {/* 3. Extra Deck breakdown */}
          <div className="flex flex-col gap-2 border-l border-border pl-4">
            <p className="eyebrow text-xs">Curva Extra Deck (Link / Livelli / Ranghi)</p>
            {deck.extra.length === 0 ? (
              <p className="text-faint">Extra Deck vuoto.</p>
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {[...dist.extraLinks.entries()]
                  .sort((a, b) => a[0] - b[0])
                  .map(([link, n]) => (
                    <span key={`L${link}`} className="chip !px-2 !py-0.5 text-xs">
                      Link-{link} <b className="tabular-nums">{n}</b>
                    </span>
                  ))}
                {[...dist.extraLevels.entries()]
                  .sort((a, b) => a[0] - b[0])
                  .map(([lvl, n]) => (
                    <span key={`R${lvl}`} className="chip !px-2 !py-0.5 text-xs">
                      ★/R{lvl} <b className="tabular-nums">{n}</b>
                    </span>
                  ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
