import { useState } from "react";
import { cardImage, type Card, type HandStats, type LabOverview } from "../../lib/api";
import { roleDefs } from "../../lib/roles";
import { pct } from "../../lib/tooltip";
import { BarChart, ChartCard, FlaskMeter, Legend, LineChart } from "../charts";

interface Props {
  overview: LabOverview;
  hand: "first" | "second";
  roles: Record<string, number[]>;
  getCard: (id: number) => Card | undefined;
}

const METRICS: { key: keyof HandStats; name: string; def: string; count?: boolean }[] = [
  { key: "starter", name: "Apri almeno uno starter", def: "Almeno una carta Starter nella mano iniziale." },
  { key: "starter2", name: "Doppio starter", def: "Almeno due Starter: giochi anche se il primo viene fermato." },
  { key: "handtrap", name: "Almeno una handtrap", def: "Almeno un'interruzione dalla mano." },
  { key: "ideal", name: "Mano ideale", def: "Almeno uno Starter e almeno una Handtrap." },
  { key: "resist", name: "Resiste a un'interruzione", def: "Due Starter, oppure uno Starter e un Estensore." },
  { key: "solid", name: "Mano solida", def: "Resiste a un'interruzione e ha almeno una Handtrap." },
  { key: "breaker", name: "Board breaker in mano", def: "Almeno un Board breaker: conta soprattutto per secondo." },
  { key: "brick", name: "Mano bloccata", def: "Nessuno Starter: la mano non parte." },
  { key: "garnet", name: "Brick in mano", def: "Almeno una carta segnata come Brick." },
];

const DIST_LABELS = ["0", "1", "2", "3", "4", "5+"];

/** Panoramica: the deck's main odds at a glance. */
export function OverviewTab({ overview: o, hand, roles, getCard }: Props) {
  const [oppHandtraps, setOppHandtraps] = useState(10);
  const cur = o[hand];
  const other = hand === "first" ? o.second : o.first;
  const otherLabel = hand === "first" ? "per secondo" : "per primo";
  const noStarters = !o.roles.starter;
  const defs = roleDefs(roles);
  const first = { name: "Primo, 5 carte", color: "var(--sec)" };
  const second = { name: "Secondo, 6 carte", color: "var(--lab-second)" };
  const seenFmt = (n: number) => String(n);
  const copiesColors = ["color-mix(in oklab, var(--sec) 40%, var(--df-surface))", "color-mix(in oklab, var(--sec) 70%, var(--df-surface))", "var(--sec)"];
  const cards = [...o.cards].sort((a, b) => b.copies - a.copies || (getCard(a.id)?.name ?? "").localeCompare(getCard(b.id)?.name ?? ""));

  const kpis: { label: string; key: keyof HandStats; warn?: boolean }[] = [
    { label: "Apri uno starter", key: "starter" },
    { label: "Almeno una handtrap", key: "handtrap" },
    { label: "Mano ideale", key: "ideal" },
    { label: "Mano bloccata", key: "brick", warn: true },
  ];

  return (
    <div className="flex flex-col gap-4">
      {noStarters && (
        <p className="lab-card px-4 py-3 text-sm text-muted">
          Segna gli <b className="text-text">Starter</b> nel pannello Ruoli: quasi tutte le probabilità partono da lì. Le handtrap e i board breaker li trovi con «Suggerisci».
        </p>
      )}
      <div className="grid grid-cols-4 gap-3">
        {kpis.map((k) => {
          const v = cur[k.key] as number;
          return (
            <div key={k.key} className="lab-card flex items-center gap-3 px-4 py-3 anim-rise">
              <FlaskMeter value={v} tone={k.warn ? "var(--role-handtrap)" : "var(--sec)"} />
              <div className="min-w-0">
                <p className="text-2xl leading-tight font-semibold">{pct(v)}</p>
                <p className="text-xs leading-tight text-muted">{k.label}</p>
                <p className="text-[11px] text-faint">
                  {otherLabel} {pct(other[k.key] as number)}
                </p>
              </div>
            </div>
          );
        })}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <ChartCard
          title="Quante handtrap apri"
          note={`Numero di handtrap nella mano iniziale (${o.roles.handtrap ?? 0} nel deck).`}
          legend={<Legend items={[first, second]} />}
          table={{ head: ["Handtrap", "Primo", "Secondo"], rows: DIST_LABELS.map((l, i) => [l, pct(o.first.handtrapDist[i]), pct(o.second.handtrapDist[i])]) }}
        >
          <BarChart labels={DIST_LABELS} series={[{ ...first, values: o.first.handtrapDist }, { ...second, values: o.second.handtrapDist }]} ariaLabel="Handtrap nella mano iniziale" />
        </ChartCard>
        <ChartCard
          title="Quanti starter apri"
          note={`Numero di Starter nella mano iniziale (${o.roles.starter ?? 0} nel deck).`}
          legend={<Legend items={[first, second]} />}
          table={{ head: ["Starter", "Primo", "Secondo"], rows: DIST_LABELS.map((l, i) => [l, pct(o.first.starterDist[i]), pct(o.second.starterDist[i])]) }}
        >
          <BarChart labels={DIST_LABELS} series={[{ ...first, values: o.first.starterDist }, { ...second, values: o.second.starterDist }]} ariaLabel="Starter nella mano iniziale" />
        </ChartCard>
      </div>

      <section className="lab-card overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-[11px] tracking-[0.12em] text-faint uppercase">
              <th className="px-4 py-2.5 font-semibold">Metrica</th>
              <th className="px-4 py-2.5 font-semibold">Definizione</th>
              <th className={`px-4 py-2.5 text-right font-semibold ${hand === "first" ? "text-sec" : ""}`}>Primo (5)</th>
              <th className={`px-4 py-2.5 text-right font-semibold ${hand === "second" ? "text-sec" : ""}`}>Secondo (6)</th>
            </tr>
          </thead>
          <tbody>
            {METRICS.map((m) => (
              <tr key={m.key} className="border-t border-border">
                <td className="px-4 py-2 font-medium">{m.name}</td>
                <td className="px-4 py-2 text-muted">{m.def}</td>
                <td className={`px-4 py-2 text-right tabular-nums ${hand === "first" ? "font-semibold text-sec" : "text-muted"}`}>{pct(o.first[m.key] as number)}</td>
                <td className={`px-4 py-2 text-right tabular-nums ${hand === "second" ? "font-semibold text-sec" : "text-muted"}`}>{pct(o.second[m.key] as number)}</td>
              </tr>
            ))}
            {defs
              .filter((d) => o.roles[d.key])
              .map((d) => (
                <tr key={d.key} className="border-t border-border">
                  <td className="px-4 py-2 font-medium">{d.label} attesi</td>
                  <td className="px-4 py-2 text-muted">Numero medio di carte {d.label} in mano ({o.roles[d.key]} nel deck).</td>
                  <td className={`px-4 py-2 text-right tabular-nums ${hand === "first" ? "font-semibold text-sec" : "text-muted"}`}>{(o.first.expected[d.key] ?? 0).toLocaleString("it-IT", { maximumFractionDigits: 2 })}</td>
                  <td className={`px-4 py-2 text-right tabular-nums ${hand === "second" ? "font-semibold text-sec" : "text-muted"}`}>{(o.second.expected[d.key] ?? 0).toLocaleString("it-IT", { maximumFractionDigits: 2 })}</td>
                </tr>
              ))}
          </tbody>
        </table>
      </section>

      <div className="grid grid-cols-2 gap-4">
        <ChartCard
          title="Pescare una carta entro…"
          note="Almeno una copia tra le carte viste: 5 è la mano per primo, 6 per secondo, poi una per turno."
          legend={<Legend items={[1, 2, 3].map((c, i) => ({ name: `${c} ${c === 1 ? "copia" : "copie"}`, color: copiesColors[i] }))} />}
          table={{ head: ["Carte viste", "1 copia", "2 copie", "3 copie"], rows: o.seen.map((s, i) => [s, ...o.byCopies.map((r) => pct(r[i]))]) }}
        >
          <LineChart
            xs={o.seen}
            series={o.byCopies.map((values, i) => ({ name: `${i + 1} ${i ? "copie" : "copia"}`, color: copiesColors[i], values }))}
            ymax={0.75}
            yTicks={[0, 0.25, 0.5, 0.75]}
            xfmt={seenFmt}
            xlabel="carte viste"
            ariaLabel="Probabilità di pescare almeno una copia"
          />
        </ChartCard>
        <ChartCard title="L'avversario ha almeno una handtrap?" note="Mano da 5 dell'avversario durante il tuo primo turno, in base alle handtrap nel suo deck da 40.">
          <LineChart
            xs={o.opponent.map((_, i) => i)}
            series={[{ name: "Almeno una handtrap", color: "var(--sec)", values: o.opponent }]}
            xlabel="handtrap nel deck avversario"
            mark={{ x: oppHandtraps, y: o.opponent[oppHandtraps], label: pct(o.opponent[oppHandtraps]) }}
            ariaLabel="Probabilità che l'avversario abbia una handtrap"
          />
          <label className="flex items-center gap-3 text-xs text-muted">
            Handtrap dell'avversario
            <input type="range" min={0} max={20} value={oppHandtraps} onChange={(e) => setOppHandtraps(Number(e.target.value))} className="flex-1 accent-[var(--sec)]" />
            <b className="w-6 text-right text-text tabular-nums">{oppHandtraps}</b>
          </label>
        </ChartCard>
      </div>

      <ChartCard title="Probabilità per carta" note={`Almeno una copia nella mano iniziale, ${hand === "first" ? "per primo (5 carte)" : "per secondo (6 carte)"}.`}>
        <div className="grid grid-cols-2 gap-x-8 gap-y-1">
          {cards.map((c) => {
            const v = c[hand];
            const card = getCard(c.id);
            return (
              <div key={c.id} className="grid grid-cols-[22px_minmax(0,1fr)_120px_48px] items-center gap-2 text-xs" data-tip={`${card?.name ?? c.id}\nPer primo ${pct(c.first)} · per secondo ${pct(c.second)}`}>
                <img src={cardImage(c.id, "small")} alt="" loading="lazy" className="h-[30px] w-[22px] rounded-[2px] object-cover" />
                <span className="flex min-w-0 items-center gap-1.5">
                  <span className="truncate">{card?.name ?? c.id}</span>
                  <span className="text-faint">×{c.copies}</span>
                  {defs
                    .filter((d) => roles[d.key]?.includes(c.id))
                    .map((d) => (
                      <i key={d.key} className="grid h-3.5 w-3.5 shrink-0 place-items-center rounded-full text-[8px] font-bold text-white not-italic" style={{ background: d.color }}>
                        {d.letter}
                      </i>
                    ))}
                </span>
                <span className="h-1.5 overflow-hidden rounded-full bg-surface-3">
                  <span className="block h-full rounded-full bg-sec transition-[width] duration-500" style={{ width: `${Math.min(100, (v / 0.65) * 100)}%` }} />
                </span>
                <span className="text-right text-muted tabular-nums">{pct(v)}</span>
              </div>
            );
          })}
        </div>
      </ChartCard>
    </div>
  );
}
