import { useEffect, useState, type ReactNode } from "react";
import type { Facets } from "../lib/api";
import { getKey, getRange, hasToken, KEYS, KIND_VALUES, removeKeys, setKey, setRange, toggleToken, type Range } from "../lib/query";
import { sound } from "../lib/sound";
import { ATTRIBUTE_COLORS } from "./CardDetails";

const KINDS = [
  { value: null, label: "Tutte" },
  { value: "monster", label: "Mostri" },
  { value: "spell", label: "Magie" },
  { value: "trap", label: "Trappole" },
] as const;

const SUBTYPES: Record<string, { token: string; label: string }[]> = {
  monster: [
    { token: "effect", label: "Effetto" },
    { token: "normal", label: "Normale" },
    { token: "fusion", label: "Fusion" },
    { token: "synchro", label: "Synchro" },
    { token: "xyz", label: "Xyz" },
    { token: "link", label: "Link" },
    { token: "ritual", label: "Rituale" },
    { token: "pendulum", label: "Pendulum" },
    { token: "tuner", label: "Tuner" },
    { token: "flip", label: "Scoperta" },
  ],
  spell: [
    { token: "quick-play", label: "Rapida" },
    { token: "continuous", label: "Continua" },
    { token: "field", label: "Terreno" },
    { token: "equip", label: "Equipaggiamento" },
    { token: "ritual", label: "Rituale" },
  ],
  trap: [
    { token: "continuous", label: "Continua" },
    { token: "counter", label: "Contro-trappola" },
  ],
};

const ATTRIBUTES = ["DARK", "LIGHT", "EARTH", "WATER", "FIRE", "WIND"];
const RARITIES = [
  { token: "n", label: "N" },
  { token: "r", label: "R" },
  { token: "sr", label: "SR" },
  { token: "ur", label: "UR" },
];
const SORTS = [
  { value: "", label: "Rilevanza" },
  { value: "popular", label: "Più cercate" },
  { value: "name", label: "Nome" },
  { value: "atk", label: "ATK" },
  { value: "def", label: "DEF" },
  { value: "level", label: "Livello" },
  { value: "new", label: "Più recenti" },
];

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="eyebrow">{title}</h3>
      {children}
    </section>
  );
}

/** Two number fields bound to a range token (committed on blur/Enter). */
function RangeFields({ query, keys, onChange, placeholder }: { query: string; keys: string[]; onChange: (q: string) => void; placeholder: [string, string] }) {
  const current = getRange(query, keys);
  const [draft, setDraft] = useState<Range>(current);
  useEffect(() => setDraft(getRange(query, keys)), [query, keys]);
  const commit = () => {
    const next = setRange(query, keys, draft);
    if (next !== query) {
      sound.play("toggle");
      onChange(next);
    }
  };
  return (
    <div className="flex items-center gap-2">
      {(["min", "max"] as const).map((side, i) => (
        <input
          key={side}
          className="field !py-1.5 tabular-nums"
          inputMode="numeric"
          placeholder={placeholder[i]}
          value={draft[side]}
          onChange={(e) => setDraft({ ...draft, [side]: e.target.value.replace(/[^0-9]/g, "") })}
          onBlur={commit}
          onKeyDown={(e) => e.key === "Enter" && commit()}
        />
      ))}
    </div>
  );
}

export function FilterPanel({ query, onChange, facets }: { query: string; onChange: (q: string) => void; facets: Facets | null }) {
  const apply = (next: string) => {
    sound.play("toggle");
    onChange(next);
  };
  const kindToken = KINDS.find((k) => k.value && hasToken(query, `type:${k.value}`))?.value ?? null;
  const [archDraft, setArchDraft] = useState(getKey(query, KEYS.arch) ?? "");
  useEffect(() => setArchDraft(getKey(query, KEYS.arch) ?? ""), [query]);

  return (
    <div className="flex flex-col gap-5 px-5 py-5">
      <Group title="Tipo di carta">
        <div className="grid grid-cols-4 gap-1 rounded-xl border border-border bg-bg-2 p-1">
          {KINDS.map((k) => (
            <button
              key={k.label}
              type="button"
              onClick={() => apply(k.value ? toggleToken(removeKeys(query, KEYS.type, (v) => KIND_VALUES.includes(v)), `type:${k.value}`) : removeKeys(query, KEYS.type, (v) => KIND_VALUES.includes(v)))}
              className={`rounded-lg px-1 py-1.5 text-xs font-semibold transition-colors ${kindToken === k.value ? "bg-sec text-bg" : "text-muted hover:bg-surface-3 hover:text-text"}`}
            >
              {k.label}
            </button>
          ))}
        </div>
        {kindToken && (
          <div className="flex flex-wrap gap-1.5">
            {SUBTYPES[kindToken].map((s) => {
              const token = `type:${s.token}`;
              const active = hasToken(query, token);
              return (
                <button key={s.token} type="button" className={`chip ${active ? "chip-active" : "text-muted hover:text-text"}`} onClick={() => apply(toggleToken(query, token))}>
                  {s.label}
                </button>
              );
            })}
          </div>
        )}
      </Group>

      <Group title="Attributo">
        <div className="grid grid-cols-3 gap-1.5">
          {ATTRIBUTES.map((a) => {
            const token = `attr:${a.toLowerCase()}`;
            const active = hasToken(query, token);
            return (
              <button
                key={a}
                type="button"
                onClick={() => apply(toggleToken(query, token))}
                className="flex items-center gap-1.5 rounded-lg border px-2 py-1 text-caption font-bold tracking-wide transition-all hover:brightness-125"
                style={{
                  borderColor: active ? ATTRIBUTE_COLORS[a] : "var(--df-border)",
                  background: active ? `color-mix(in oklab, ${ATTRIBUTE_COLORS[a]} 22%, var(--df-surface-2))` : "var(--df-bg-2)",
                  boxShadow: active ? `0 0 12px -3px ${ATTRIBUTE_COLORS[a]}` : undefined,
                }}
              >
                <span className="h-2 w-2 shrink-0 rounded-full" style={{ background: ATTRIBUTE_COLORS[a] }} />
                {a}
              </button>
            );
          })}
        </div>
      </Group>

      <Group title="Livello / Rango">
        <RangeFields query={query} keys={KEYS.level} onChange={onChange} placeholder={["min", "max"]} />
      </Group>
      <Group title="ATK">
        <RangeFields query={query} keys={KEYS.atk} onChange={onChange} placeholder={["da", "a"]} />
      </Group>
      <Group title="DEF">
        <RangeFields query={query} keys={KEYS.def} onChange={onChange} placeholder={["da", "a"]} />
      </Group>

      <Group title="Tipo di mostro">
        <select className="field" value={getKey(query, KEYS.race) ?? ""} onChange={(e) => apply(setKey(query, KEYS.race, e.target.value || null))}>
          <option value="">Tutti</option>
          {facets?.races.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>
      </Group>

      <Group title="Archetipo">
        <input
          className="field"
          list="archetypes"
          placeholder="es. Invoked"
          value={archDraft}
          onChange={(e) => setArchDraft(e.target.value)}
          onBlur={() => archDraft !== (getKey(query, KEYS.arch) ?? "") && apply(setKey(query, KEYS.arch, archDraft.trim() || null))}
          onKeyDown={(e) => e.key === "Enter" && apply(setKey(query, KEYS.arch, archDraft.trim() || null))}
        />
        <datalist id="archetypes">
          {facets?.archetypes.map((a) => (
            <option key={a} value={a} />
          ))}
        </datalist>
      </Group>

      <Group title="Master Duel">
        <button type="button" className={`chip w-fit ${hasToken(query, "format:md") ? "chip-active" : "text-muted"}`} onClick={() => apply(toggleToken(query, "format:md"))}>
          Solo carte disponibili in Master Duel
        </button>
        <div className="flex gap-2">
          {RARITIES.map((r) => {
            const token = `rarity:${r.token}`;
            const active = hasToken(query, token);
            return (
              <button
                key={r.token}
                type="button"
                onClick={() => apply(toggleToken(query, token))}
                className={`rarity-gem !static !px-2 !text-[11px] !leading-6 transition-all ${active ? "" : "opacity-35 grayscale hover:opacity-70"}`}
                data-rarity={r.token}
              >
                {r.label}
              </button>
            );
          })}
        </div>
      </Group>

      <Group title="Ordina per">
        <select className="field" value={getKey(query, KEYS.sort) ?? ""} onChange={(e) => apply(setKey(query, KEYS.sort, e.target.value || null))}>
          {SORTS.map((s) => (
            <option key={s.value} value={s.value}>
              {s.label}
            </option>
          ))}
        </select>
      </Group>

      <button type="button" className="btn btn-ghost justify-center" onClick={() => apply("")}>
        Azzera filtri
      </button>
    </div>
  );
}
