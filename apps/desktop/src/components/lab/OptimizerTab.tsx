import { useEffect, useMemo, useState } from "react";
import { api, type Card, type Cond, type Deck, type LabAnalysis, type OptimizeResult } from "../../lib/api";
import { copyLimit } from "../../lib/deck";
import { roleDefs } from "../../lib/roles";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { pct } from "../../lib/tooltip";
import { ChartCard, HeatmapChart, LineChart, TornadoChart } from "../charts";

interface Props {
  deck: Deck;
  roles: Record<string, number[]>;
  cond: Cond;
  hand: "first" | "second";
  getCard: (id: number) => Card | undefined;
  /** Applies the optimizer's changes: copies taken out go to the Maybe board. */
  onApply: (changes: { id: number; from: number; to: number }[]) => void;
}

const points = (v: number) => `${v >= 0 ? "+" : "−"}${Math.abs(v * 100).toLocaleString("it-IT", { minimumFractionDigits: 1, maximumFractionDigits: 1 })}`;

/** Ottimizzatore: what each copy is worth, and a search for better copies. */
export function OptimizerTab({ deck, roles, cond, hand, getCard, onApply }: Props) {
  const defs = roleDefs(roles);
  const used = defs.filter((d) => deck.main.some((id) => roles[d.key]?.includes(id)));
  const [roleA, setRoleA] = useState("starter");
  const [roleB, setRoleB] = useState("handtrap");
  const [analysis, setAnalysis] = useState<LabAnalysis | null>(null);
  const [weight, setWeight] = useState(60);
  const [useMaybe, setUseMaybe] = useState(true);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<OptimizeResult | null>(null);
  const handSize = hand === "first" ? 5 : 6;
  const name = (id: number) => getCard(id)?.name ?? String(id);
  const limits = useMemo(() => {
    const out: Record<number, number> = {};
    for (const id of new Set([...deck.main, ...deck.maybe])) {
      const card = getCard(id);
      out[id] = card ? copyLimit(card, deck.format) : 3;
    }
    return out;
  }, [deck, getCard]);
  const key = JSON.stringify([cond, deck.main, roles, handSize, roleA, roleB, limits]);

  useEffect(() => {
    let cancelled = false;
    const t = window.setTimeout(() => {
      api
        .labAnalyze(
          deck.main,
          roles,
          cond,
          handSize,
          used.map((d) => d.key),
          limits,
          roleA,
          roleB,
        )
        .then((a) => !cancelled && setAnalysis(a));
    }, 120);
    return () => {
      cancelled = true;
      window.clearTimeout(t);
    };
  }, [key]); // eslint-disable-line react-hooks/exhaustive-deps

  const optimize = async () => {
    setBusy(true);
    sound.play("open");
    try {
      const r = await api.labOptimize({
        main: deck.main,
        maybe: useMaybe ? deck.maybe : [],
        roles,
        limits,
        locked: [],
        cond,
        weightFirst: weight / 100,
        minSize: Math.max(40, deck.main.length),
        maxSize: Math.max(40, deck.main.length),
        seed: Math.floor(Math.random() * 1e9),
      });
      setResult(r);
      sound.play(r.changes.length ? "glass" : "toggle");
    } catch (e) {
      sound.play("error");
      toast(`Ottimizzazione non riuscita: ${e}`, "error");
    } finally {
      setBusy(false);
    }
  };

  const s = analysis?.sensitivity;
  const cardRows = (s?.cards ?? [])
    .map((d) => ({ label: name(Number(d.key)), plus: d.plus, minus: d.minus }))
    .sort((a, b) => Math.abs(b.plus ?? 0) + Math.abs(b.minus ?? 0) - (Math.abs(a.plus ?? 0) + Math.abs(a.minus ?? 0)))
    .slice(0, 10);
  const roleName = (k: string) => defs.find((d) => d.key === k)?.label ?? k;

  return (
    <div className="flex flex-col gap-4">
      {analysis?.error && <p className="lab-card px-4 py-3 text-sm text-warn">{analysis.error}</p>}
      <div className="grid grid-cols-2 gap-4">
        <ChartCard title="Quanto vale una copia in più o in meno, per ruolo" note={s ? `Condizione ${hand === "first" ? "per primo" : "per secondo"}: ${pct(s.base)}. Una copia del ruolo prende o lascia il posto a una carta senza ruolo.` : "…"}>
          {s && <TornadoChart rows={s.roles.map((d) => ({ label: roleName(d.key), plus: d.plus, minus: d.minus }))} ariaLabel="Valore di una copia per ruolo" />}
          {s?.grew && <p className="text-[11px] text-faint">Il deck non ha carte senza ruolo: «+1 copia» lo porta a {deck.main.length + 1} carte.</p>}
        </ChartCard>
        <ChartCard title="Le carte che contano di più" note="Le dieci carte la cui copia in più o in meno cambia di più la condizione.">
          {s && <TornadoChart rows={cardRows} ariaLabel="Valore di una copia per carta" />}
        </ChartCard>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <ChartCard
          title="40 carte o di più?"
          note="La condizione aggiungendo carte senza ruolo."
          table={analysis ? { head: ["Carte", "Probabilità"], rows: analysis.size.map((v, i) => [deck.main.length + i, v == null ? "—" : pct(v)]) } : undefined}
        >
          {analysis && analysis.size.every((v) => v != null) && (
            <LineChart
              xs={analysis.size.map((_, i) => deck.main.length + i)}
              series={[{ name: "Condizione", color: "var(--sec)", values: analysis.size as number[] }]}
              ymax={Math.min(1, Math.ceil(Math.max(...(analysis.size as number[])) * 4 + 0.5) / 4)}
              xlabel="carte nel Main Deck"
              mark={{ x: deck.main.length, y: analysis.size[0] as number, label: pct(analysis.size[0] as number) }}
              ariaLabel="Probabilità al crescere del deck"
            />
          )}
        </ChartCard>
        <ChartCard title="Combinazione di due ruoli" note="La condizione al variare delle copie dei due ruoli (carte con solo quel ruolo), il resto senza ruolo.">
          <div className="flex items-center gap-2 text-xs text-muted">
            Righe
            <select className="field !w-auto !py-1 text-xs" value={roleA} onChange={(e) => setRoleA(e.target.value)}>
              {defs.map((d) => (
                <option key={d.key} value={d.key}>
                  {d.label}
                </option>
              ))}
            </select>
            colonne
            <select className="field !w-auto !py-1 text-xs" value={roleB} onChange={(e) => setRoleB(e.target.value)}>
              {defs.map((d) => (
                <option key={d.key} value={d.key}>
                  {d.label}
                </option>
              ))}
            </select>
          </div>
          {analysis?.heatmap ? (
            <HeatmapChart
              rows={analysis.heatmap.a}
              cols={analysis.heatmap.b}
              values={analysis.heatmap.p}
              current={analysis.heatmap.current}
              rowLabel={roleName(roleA)}
              colLabel={roleName(roleB)}
              ariaLabel="Mappa di due ruoli"
            />
          ) : (
            <p className="py-6 text-center text-xs text-faint">Scegli due ruoli diversi.</p>
          )}
        </ChartCard>
      </div>

      <section className="lab-card flex flex-col gap-3 p-4">
        <div className="flex flex-wrap items-center gap-4">
          <h3 className="text-sm font-semibold">Cerca il deck migliore</h3>
          <label className="flex items-center gap-2 text-xs text-muted">
            Peso primo / secondo
            <input type="range" min={0} max={100} value={weight} onChange={(e) => setWeight(Number(e.target.value))} className="w-32 accent-[var(--sec)]" />
            <b className="w-20 text-text tabular-nums">
              {weight}% / {100 - weight}%
            </b>
          </label>
          <label className="flex items-center gap-2 text-xs text-muted">
            <input type="checkbox" checked={useMaybe} onChange={(e) => setUseMaybe(e.target.checked)} className="accent-[var(--sec)]" />
            Prova anche le carte della Maybe board ({new Set(deck.maybe).size})
          </label>
          <button type="button" className="btn btn-molten ml-auto" disabled={busy} onClick={() => void optimize()}>
            {busy ? "Sto cercando…" : "Ottimizza"}
          </button>
        </div>
        <p className="text-[11px] text-faint">Prova migliaia di varianti del Main Deck con le stesse carte (e quelle della Maybe board), rispettando banlist e numero di carte. Usa il calcolo esatto: gli effetti del simulatore non sono considerati.</p>
        {result && (
          <div className="flex flex-col gap-2 border-t border-border pt-3">
            <div className="flex flex-wrap items-baseline gap-4 text-sm">
              <span>
                Primo {pct(result.before.first)} → <b>{pct(result.after.first)}</b>
              </span>
              <span>
                Secondo {pct(result.before.second)} → <b>{pct(result.after.second)}</b>
              </span>
              <span className={result.after.objective > result.before.objective ? "text-ok" : "text-muted"}>{points(result.after.objective - result.before.objective)} punti sull'obiettivo</span>
            </div>
            {result.changes.length === 0 ? (
              <p className="text-sm text-muted">Nessuna modifica migliora il deck per questa condizione.</p>
            ) : (
              <div className="flex flex-wrap items-center gap-1.5">
                {result.changes
                  .slice()
                  .sort((a, b) => b.to - b.from - (a.to - a.from))
                  .map((c) => (
                    <span key={c.id} className={`chip !px-2 !py-0.5 !text-xs ${c.to > c.from ? "text-ok" : "text-danger"}`}>
                      {c.to > c.from ? "+" : "−"}
                      {Math.abs(c.to - c.from)} {name(c.id)}
                    </span>
                  ))}
                <button type="button" className="btn btn-ghost ml-auto !py-1 text-xs" onClick={() => onApply(result.changes)}>
                  Applica al deck
                </button>
              </div>
            )}
          </div>
        )}
      </section>
    </div>
  );
}
