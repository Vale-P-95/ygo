import { useEffect, useState } from "react";
import { api, cardImage, type Card, type Deck, type DeckSummary, type HandStats, type LabOverview } from "../../lib/api";
import { BUILTIN_ROLES } from "../../lib/roles";
import { toast } from "../../lib/toast";
import { pct } from "../../lib/tooltip";
import { BarChart, ChartCard, Legend } from "../charts";

interface Props {
  deck: Deck;
  overview: LabOverview;
  decks: DeckSummary[];
  hand: "first" | "second";
  getCard: (id: number) => Card | undefined;
  /** Loads the images and names of these cards. */
  request: (ids: number[]) => void;
}

/** The odds compared, with which way is better. */
const METRICS: { key: keyof HandStats; name: string; lowerIsBetter?: boolean }[] = [
  { key: "starter", name: "Apri almeno uno starter" },
  { key: "starter2", name: "Doppio starter" },
  { key: "handtrap", name: "Almeno una handtrap" },
  { key: "ideal", name: "Mano ideale" },
  { key: "resist", name: "Resiste a un'interruzione" },
  { key: "solid", name: "Mano solida" },
  { key: "breaker", name: "Board breaker in mano" },
  { key: "brick", name: "Mano bloccata", lowerIsBetter: true },
  { key: "garnet", name: "Brick in mano", lowerIsBetter: true },
];

const DIST_LABELS = ["0", "1", "2", "3", "4", "5+"];
const A_COLOR = "var(--sec)";
const B_COLOR = "var(--role-extender)";

function counts(ids: number[]): Map<number, number> {
  const m = new Map<number, number>();
  for (const id of ids) m.set(id, (m.get(id) ?? 0) + 1);
  return m;
}

/** A change of a probability, coloured by whether it is better for the deck compared. */
function Delta({ value, lowerIsBetter }: { value: number; lowerIsBetter?: boolean }) {
  if (Math.abs(value) < 0.0005) return <span className="text-faint">=</span>;
  const better = lowerIsBetter ? value < 0 : value > 0;
  return (
    <span className={better ? "text-ok" : "text-danger"}>
      {value > 0 ? "▲ +" : "▼ "}
      {pct(value)}
    </span>
  );
}

function CardList({ title, items, getCard }: { title: string; items: { id: number; label: string }[]; getCard: (id: number) => Card | undefined }) {
  return (
    <div className="min-w-0">
      <p className="eyebrow mb-1.5 text-faint">
        {title} · {items.length}
      </p>
      {items.length === 0 ? (
        <p className="text-xs text-faint">—</p>
      ) : (
        <ul className="flex flex-col gap-1">
          {items.map((c) => (
            <li key={c.id} className="flex items-center gap-2 text-xs">
              <img src={cardImage(c.id, "small")} alt="" loading="lazy" className="h-[30px] w-[22px] shrink-0 rounded-[2px] object-cover" />
              <span className="min-w-0 flex-1 truncate">{getCard(c.id)?.name ?? c.id}</span>
              <span className="shrink-0 tabular-nums text-muted">{c.label}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Confronto: the odds of this deck next to another one's, and which cards differ. */
export function CompareTab({ deck, overview: a, decks, hand, getCard, request }: Props) {
  const others = decks.filter((d) => d.id !== deck.id);
  const [otherId, setOtherId] = useState<number | null>(others[0]?.id ?? null);
  const [other, setOther] = useState<Deck | null>(null);
  const [b, setB] = useState<LabOverview | null>(null);

  useEffect(() => {
    if (otherId == null) return;
    let cancelled = false;
    setB(null);
    (async () => {
      const d = await api.getDeck(otherId);
      if (!d || cancelled) return;
      setOther(d);
      request([...new Set(d.main)]);
      const o = await api.labOverview(d.main, d.roles ?? {});
      if (!cancelled) setB(o);
    })().catch((e) => toast(`Deck non caricato: ${e}`, "error"));
    return () => {
      cancelled = true;
    };
  }, [otherId, request]);

  if (others.length === 0) {
    return <p className="lab-card px-4 py-3 text-sm text-muted">Serve un altro deck da confrontare: costruiscine uno nella Fucina.</p>;
  }

  const handLabel = hand === "first" ? "per primo (5 carte)" : "per secondo (6 carte)";
  const nameA = deck.name;
  const nameB = other?.name ?? "…";
  const legend = <Legend items={[{ name: nameA, color: A_COLOR }, { name: nameB, color: B_COLOR }]} />;

  const ca = counts(deck.main);
  const cb = counts(other?.main ?? []);
  const onlyA = [...ca].filter(([id]) => !cb.has(id)).map(([id, n]) => ({ id, label: `×${n}` }));
  const onlyB = [...cb].filter(([id]) => !ca.has(id)).map(([id, n]) => ({ id, label: `×${n}` }));
  const changed = [...ca]
    .filter(([id, n]) => cb.has(id) && cb.get(id) !== n)
    .map(([id, n]) => ({ id, label: `${n} → ${cb.get(id)}` }));
  const noRoles = b && !b.roles.starter;

  return (
    <div className="flex flex-col gap-4">
      <div className="lab-card flex flex-wrap items-center gap-3 px-4 py-3">
        <span className="flex items-center gap-2 text-sm font-semibold">
          <i className="h-2.5 w-2.5 rounded-full" style={{ background: A_COLOR }} />
          {nameA}
        </span>
        <span className="text-faint">contro</span>
        <i className="h-2.5 w-2.5 rounded-full" style={{ background: B_COLOR }} />
        <select className="field !w-auto !py-1 text-sm" value={otherId ?? ""} onChange={(e) => setOtherId(Number(e.target.value))} aria-label="Deck da confrontare">
          {others.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
            </option>
          ))}
        </select>
        <span className="ml-auto text-xs text-faint">Probabilità {handLabel}, con i ruoli di ciascun deck.</span>
      </div>

      {noRoles && (
        <p className="lab-card px-4 py-3 text-sm text-muted">
          {nameB} non ha Starter segnati: aprilo nel Laboratorio e premi <b className="text-text">✦ Riconosci</b> nel pannello Ruoli, poi torna qui.
        </p>
      )}

      {!b ? (
        <p className="text-sm text-muted">Calcolo le probabilità di {nameB}…</p>
      ) : (
        <>
          <section className="lab-card overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-[11px] tracking-[0.12em] text-faint uppercase">
                  <th className="px-4 py-2.5 font-semibold">Metrica</th>
                  <th className="px-4 py-2.5 text-right font-semibold" style={{ color: A_COLOR }}>
                    {nameA}
                  </th>
                  <th className="px-4 py-2.5 text-right font-semibold" style={{ color: B_COLOR }}>
                    {nameB}
                  </th>
                  <th className="px-4 py-2.5 text-right font-semibold">Differenza</th>
                </tr>
              </thead>
              <tbody>
                {METRICS.map((m) => {
                  const va = a[hand][m.key] as number;
                  const vb = b[hand][m.key] as number;
                  return (
                    <tr key={m.key} className="border-t border-border">
                      <td className="px-4 py-2 font-medium">{m.name}</td>
                      <td className="px-4 py-2 text-right tabular-nums">{pct(va)}</td>
                      <td className="px-4 py-2 text-right tabular-nums">{pct(vb)}</td>
                      <td className="px-4 py-2 text-right tabular-nums">
                        <Delta value={vb - va} lowerIsBetter={m.lowerIsBetter} />
                      </td>
                    </tr>
                  );
                })}
                {BUILTIN_ROLES.filter((r) => a.roles[r.key] || b.roles[r.key]).map((r) => (
                  <tr key={r.key} className="border-t border-border">
                    <td className="px-4 py-2 font-medium">{r.label} nel deck</td>
                    <td className="px-4 py-2 text-right tabular-nums">{a.roles[r.key] ?? 0}</td>
                    <td className="px-4 py-2 text-right tabular-nums">{b.roles[r.key] ?? 0}</td>
                    <td className="px-4 py-2 text-right tabular-nums text-muted">{(b.roles[r.key] ?? 0) - (a.roles[r.key] ?? 0) || "="}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>

          <div className="grid grid-cols-2 gap-4">
            <ChartCard
              title="Quanti starter apri"
              note={`Starter nella mano iniziale, ${handLabel}.`}
              legend={legend}
              table={{ head: ["Starter", nameA, nameB], rows: DIST_LABELS.map((l, i) => [l, pct(a[hand].starterDist[i]), pct(b[hand].starterDist[i])]) }}
            >
              <BarChart
                labels={DIST_LABELS}
                series={[
                  { name: nameA, color: A_COLOR, values: a[hand].starterDist },
                  { name: nameB, color: B_COLOR, values: b[hand].starterDist },
                ]}
                ariaLabel="Starter nella mano iniziale, due deck a confronto"
              />
            </ChartCard>
            <ChartCard
              title="Quante handtrap apri"
              note={`Handtrap nella mano iniziale, ${handLabel}.`}
              legend={legend}
              table={{ head: ["Handtrap", nameA, nameB], rows: DIST_LABELS.map((l, i) => [l, pct(a[hand].handtrapDist[i]), pct(b[hand].handtrapDist[i])]) }}
            >
              <BarChart
                labels={DIST_LABELS}
                series={[
                  { name: nameA, color: A_COLOR, values: a[hand].handtrapDist },
                  { name: nameB, color: B_COLOR, values: b[hand].handtrapDist },
                ]}
                ariaLabel="Handtrap nella mano iniziale, due deck a confronto"
              />
            </ChartCard>
          </div>

          <ChartCard title="Carte diverse" note="Main Deck: le carte che ha solo uno dei due, e quelle con un numero di copie diverso.">
            <div className="grid grid-cols-3 gap-6">
              <CardList title={`Solo in ${nameA}`} items={onlyA} getCard={getCard} />
              <CardList title={`Solo in ${nameB}`} items={onlyB} getCard={getCard} />
              <CardList title="Copie diverse" items={changed} getCard={getCard} />
            </div>
          </ChartCard>
        </>
      )}
    </div>
  );
}
