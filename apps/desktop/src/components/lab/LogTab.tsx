import { useEffect, useRef, useState } from "react";
import { api, type Card, type Deck, type MatchLog, type WinRecord } from "../../lib/api";
import { decided, OUTCOME, played, playedLabel, pValue, recordTip, score, shortDate, streak } from "../../lib/matches";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { pct } from "../../lib/tooltip";
import { ChartCard, FlaskMeter, ForestPlot, Legend, LineChart, WinMatrix, type ForestRow } from "../charts";

interface Props {
  deck: Deck;
  log: MatchLog | null;
  scope: "deck" | "all";
  setScope: (s: "deck" | "all") => void;
  getCard: (id: number) => Card | undefined;
  onChanged: () => void;
}

const TOP = 10;

/** Registro partite: how the deck does, by matchup, turn, version and over time. */
export function LogTab({ deck, log, scope, setScope, getCard, onChanged }: Props) {
  const file = useRef<HTMLInputElement>(null);
  const [all, setAll] = useState(false);
  const [confirm, setConfirm] = useState<number | null>(null);
  const r = log?.report;
  const name = (id: number) => getCard(id)?.name ?? String(id);

  const exportCsv = async () => {
    try {
      const path = await api.exportMatches(scope === "deck" ? deck.id : null, scope === "deck" ? deck.name : "Tutti i deck");
      sound.play("chime");
      toast(`Registro salvato in ${path}`, "ok");
    } catch (e) {
      sound.play("error");
      toast(`Registro non salvato: ${e}`, "error");
    }
  };

  const importCsv = async (f: File) => {
    try {
      const n = await api.importMatches(deck.id!, await f.text());
      sound.play("bubble");
      toast(`${n} ${n === 1 ? "partita importata" : "partite importate"} in ${deck.name}`, "ok");
      onChanged();
    } catch (e) {
      sound.play("error");
      toast(`CSV non importato: ${e}`, "error");
    }
  };

  const remove = async (id: number) => {
    if (confirm !== id) {
      sound.play("toggle");
      setConfirm(id);
      return;
    }
    await api.deleteMatch(id);
    sound.play("close");
    setConfirm(null);
    onChanged();
  };

  const header = (
    <div className="flex flex-wrap items-center gap-3">
      <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1" role="group" aria-label="Partite di">
        {(["deck", "all"] as const).map((s) => (
          <button
            key={s}
            type="button"
            aria-pressed={scope === s}
            onClick={() => {
              sound.play("toggle");
              setScope(s);
            }}
            className={`max-w-56 truncate rounded-lg px-2.5 py-1 text-xs font-semibold ${scope === s ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
          >
            {s === "deck" ? deck.name : "Tutti i deck"}
          </button>
        ))}
      </div>
      {r && <span className="text-xs text-faint tabular-nums">{r.matches === 1 ? "1 partita" : `${r.matches} partite`}</span>}
      <span className="ml-auto flex gap-2">
        {scope === "deck" && (
          <>
            <input ref={file} type="file" accept=".csv,text/csv,text/plain" hidden onChange={(e) => e.target.files?.[0] && void importCsv(e.target.files[0]).finally(() => (e.target.value = ""))} />
            <button type="button" className="btn btn-ghost !py-1 text-xs" onClick={() => file.current?.click()} title="Aggiunge a questo deck le partite di un file CSV">
              Importa CSV
            </button>
          </>
        )}
        <button type="button" className="btn btn-ghost !py-1 text-xs" disabled={!r?.matches} onClick={() => void exportCsv()}>
          Esporta CSV
        </button>
      </span>
    </div>
  );

  if (!log || !r) return <div className="flex flex-col gap-4">{header}</div>;

  if (r.matches === 0) {
    return (
      <div className="flex flex-col gap-4">
        {header}
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-4">
          <section className="lab-card flex flex-col items-center justify-center gap-3 p-8 text-center anim-rise">
            <FlaskMeter value={0} size={56} />
            <p className="display text-lg">Nessuna partita registrata</p>
            <p className="max-w-sm text-sm text-muted">
              Registra le partite dal pannello a destra: avversario, turno e risultato bastano. Il laboratorio calcola win-rate con i loro intervalli, matchup, andamento e, a ogni modifica del deck, confronta le versioni.
            </p>
          </section>
          <SampleSize decidedNow={0} />
        </div>
      </div>
    );
  }

  const st = streak(log.matches);
  const recent = log.matches.slice(-10);
  const opponents = r.opponents.slice(0, TOP);
  const othersCount = r.opponents.length - opponents.length;
  const matrixRows = r.matrix.slice(0, TOP).map(([name, first, second]) => ({ name, first, second, total: r.opponents.find((o) => o.name === name)!.record }));
  const turnRows: ForestRow[] = [
    { label: "Per primo", record: r.first },
    { label: "Per secondo", record: r.second },
    ...(decided(r.postSide) > 0
      ? [
          { label: "Game 1", record: r.game1 },
          { label: "Dopo il side", sub: "game 2 e 3", record: r.postSide },
        ]
      : []),
  ].filter((row) => played(row.record) > 0);
  const versionInfo = new Map(log.versions.map((v) => [v.version, v]));
  const versionRows: ForestRow[] = r.versions.map((v) => ({
    label: v.name,
    sub: v.firstPlayed.slice(0, 10) === v.lastPlayed.slice(0, 10) ? shortDate(v.firstPlayed) : `${shortDate(v.firstPlayed)} – ${shortDate(v.lastPlayed)}`,
    record: v.record,
  }));
  const cmp = r.latestComparison;
  const [older, newer] = r.versions.slice(-2);
  const kpi = "lab-card flex items-center gap-3 px-4 py-3 anim-rise";
  const shown = all ? [...log.matches].reverse() : [...log.matches].reverse().slice(0, 25);

  return (
    <div className="flex flex-col gap-4">
      {header}
      <div className="grid grid-cols-4 gap-3">
        <div className={kpi} tabIndex={0} data-tip={recordTip("Tutte le partite", r.total)}>
          <FlaskMeter value={decided(r.total) ? r.total.p : 0} />
          <div className="min-w-0">
            <p className="text-2xl leading-tight font-semibold">{decided(r.total) ? pct(r.total.p) : "—"}</p>
            <p className="text-xs leading-tight text-muted">Win-rate</p>
            <p className="text-[11px] text-faint tabular-nums">{decided(r.total) ? `95%: ${pct(r.total.lo, 0)} – ${pct(r.total.hi, 0)}` : "nessuna partita decisa"}</p>
          </div>
        </div>
        <div className={kpi}>
          <div className="min-w-0 flex-1">
            <p className="text-2xl leading-tight font-semibold tabular-nums">{r.matches}</p>
            <p className="text-xs leading-tight text-muted">{score(r.total)}</p>
            <div className="mt-1.5 flex gap-[3px]" aria-label="Ultime partite">
              {recent.map((m, i) => (
                <i
                  key={m.id ?? i}
                  className="h-2.5 w-2.5 rounded-full"
                  style={{ background: OUTCOME[m.result ?? "D"]?.tone }}
                  data-tip={`${OUTCOME[m.result ?? "D"]?.long}${m.opponent ? ` contro ${m.opponent}` : ""}\n${playedLabel(m.playedAt)}`}
                />
              ))}
            </div>
            {st && st[1] > 1 && (
              <p className="mt-1 text-[11px] text-faint">
                {st[1]} {st[0] === "W" ? "vittorie" : "sconfitte"} di fila
              </p>
            )}
          </div>
        </div>
        <div className={kpi}>
          <div className="grid min-w-0 flex-1 grid-cols-2 gap-2">
            {(
              [
                ["Per primo", r.first],
                ["Per secondo", r.second],
              ] as const
            ).map(([label, rec]) => (
              <div key={label} tabIndex={0} data-tip={recordTip(label, rec)}>
                <p className="text-xl leading-tight font-semibold">{decided(rec) ? pct(rec.p, 0) : "—"}</p>
                <p className="text-xs leading-tight text-muted">{label}</p>
                <p className="text-[11px] text-faint tabular-nums">{played(rec)} partite</p>
              </div>
            ))}
          </div>
        </div>
        <div
          className={kpi}
          tabIndex={0}
          data-tip={"Il win-rate da aspettarsi se le prossime partite\nsono contro gli avversari delle ultime 30:\nil win-rate di ogni matchup, pesato su quanto\nspesso è uscito di recente. I matchup con poche\npartite contano come se fossero vicini alla media."}
        >
          <FlaskMeter value={r.metaExpected} tone="var(--lab-second)" />
          <div className="min-w-0">
            <p className="text-2xl leading-tight font-semibold">{pct(r.metaExpected)}</p>
            <p className="text-xs leading-tight text-muted">Atteso col meta recente</p>
            <p className="text-[11px] text-faint">ultime 30 partite</p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-[minmax(0,3fr)_minmax(0,2fr)] gap-4">
        <ChartCard
          title="Win-rate per avversario"
          note={`Il punto è il win-rate, la linea il suo intervallo al 95%: più partite, più stretta. ▲/▼ quando tutto l'intervallo sta sopra o sotto il 50%.${othersCount > 0 ? ` I ${TOP} più incontrati; altri ${othersCount} nei dati.` : ""}`}
          table={{ head: ["Avversario", "Partite", "V", "S", "P", "Win-rate", "Intervallo 95%"], rows: r.opponents.map((o) => [o.name, played(o.record), o.record.wins, o.record.losses, o.record.draws, decided(o.record) ? pct(o.record.p) : "—", decided(o.record) ? `${pct(o.record.lo)} – ${pct(o.record.hi)}` : "—"]) }}
        >
          <ForestPlot width={520} rows={opponents.map((o) => ({ label: o.name, record: o.record }))} reference={decided(r.total) ? { value: r.total.p, label: `media ${pct(r.total.p, 0)}` } : undefined} ariaLabel="Win-rate per avversario" />
        </ChartCard>
        <ChartCard title="Meta incontrato" note="Quota di partite contro ciascun avversario." table={{ head: ["Avversario", "Partite", "Quota"], rows: r.opponents.map((o) => [o.name, played(o.record), pct(o.share)]) }}>
          <ShareBars rows={r.opponents} />
        </ChartCard>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <ChartCard
          title="Andamento"
          note="Win-rate sulle ultime 20 partite decise e dall'inizio."
          legend={
            <Legend
              items={[
                { name: "Ultime 20", color: "var(--sec)" },
                { name: "Dall'inizio", color: "var(--df-muted)" },
              ]}
            />
          }
          table={{ head: ["Partita", "Data", "Ultime 20", "Dall'inizio"], rows: r.trend.map((t) => [t.index, playedLabel(t.playedAt), pct(t.rolling), pct(t.cumulative)]) }}
        >
          {r.trend.length >= 3 ? (
            <LineChart
              xs={r.trend.map((t) => t.index)}
              series={[
                { name: "Ultime 20", color: "var(--sec)", values: r.trend.map((t) => t.rolling) },
                { name: "Dall'inizio", color: "var(--df-muted)", values: r.trend.map((t) => t.cumulative) },
              ]}
              xlabel="partite decise"
              ariaLabel="Andamento del win-rate"
            />
          ) : (
            <p className="py-8 text-center text-xs text-faint">Servono almeno 3 partite decise.</p>
          )}
        </ChartCard>
        <ChartCard title="Avversario e turno" note="Win-rate e partite per avversario, per primo e per secondo. Verde sopra il 50%, rosso sotto; più intenso con più partite.">
          <WinMatrix rows={matrixRows} />
        </ChartCard>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <ChartCard
          title="Turno e side"
          note={
            r.orderFisherP == null
              ? "Per primo e per secondo, e nei Bo3 il game 1 contro i game dopo il side."
              : `Per primo contro per secondo: ${pValue(r.orderFisherP)} (test esatto di Fisher), ${r.orderFisherP < 0.05 ? "una differenza netta" : "differenza non ancora chiara"}.`
          }
        >
          {turnRows.length > 0 ? <ForestPlot rows={turnRows} ariaLabel="Win-rate per turno e side" /> : <p className="py-6 text-center text-xs text-faint">Segna il turno per vedere la differenza.</p>}
        </ChartCard>
        <ChartCard
          title={scope === "deck" ? "Versioni del deck" : "Per deck"}
          note={scope === "deck" ? "Ogni modifica della lista è una nuova versione: le partite si confrontano tra versioni." : "Win-rate di ciascun deck."}
        >
          <ForestPlot rows={versionRows} reference={decided(r.total) ? { value: r.total.p, label: `media ${pct(r.total.p, 0)}` } : undefined} ariaLabel={scope === "deck" ? "Win-rate per versione" : "Win-rate per deck"} />
          {scope === "deck" && (
            <div className="flex flex-col gap-1 text-xs">
              {r.versions.map((v) => {
                const changes = versionInfo.get(v.version)?.changes;
                if (!changes?.length) return null;
                return (
                  <p key={v.version} className="text-muted">
                    <b className="text-text">{v.name}</b>:{" "}
                    {changes.map((c, i) => (
                      <span key={`${c.id}-${c.zone}`}>
                        {i > 0 && ", "}
                        <span className={c.delta > 0 ? "text-ok" : "text-danger"}>
                          {c.delta > 0 ? "+" : "−"}
                          {Math.abs(c.delta)}
                        </span>{" "}
                        {name(c.id)}
                        {c.zone !== "main" && <span className="text-faint"> ({c.zone})</span>}
                      </span>
                    ))}
                  </p>
                );
              })}
              {cmp && older && newer && (
                <p className="mt-1 rounded-lg bg-surface-3/60 px-3 py-2 text-muted">
                  <b className="text-text">
                    {newer.name} contro {older.name}
                  </b>
                  : probabilità che la {newer.name} vinca più spesso <b className="text-text">{pct(cmp.probNewerBetter, 0)}</b> · {pValue(cmp.fisherP)}{" "}
                  {cmp.fisherP < 0.05 ? "— una differenza netta." : "— servono altre partite per esserne sicuri."}
                </p>
              )}
            </div>
          )}
        </ChartCard>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <SampleSize decidedNow={decided(r.total)} start={decided(r.total) >= 20 ? Math.round(r.total.p * 20) / 20 : 0.5} />
        {r.tags.length > 0 ? (
          <ChartCard title="Per tag" note="Win-rate delle partite con ciascun tag.">
            <ForestPlot rows={r.tags.slice(0, TOP).map((t) => ({ label: t.name, record: t.record }))} reference={{ value: r.total.p, label: `media ${pct(r.total.p, 0)}` }} ariaLabel="Win-rate per tag" />
          </ChartCard>
        ) : (
          <section className="lab-card flex flex-col justify-center gap-1 p-4 text-sm text-muted">
            <h3 className="text-sm font-semibold text-text">Per tag</h3>
            <p>Aggiungi tag alle partite (ranked, evento, brick…) per confrontarne il win-rate.</p>
          </section>
        )}
      </div>

      <section className="lab-card flex flex-col gap-2 p-4">
        <div className="flex items-baseline gap-3">
          <h3 className="text-sm font-semibold">Partite</h3>
          <span className="text-xs text-faint">dalla più recente</span>
        </div>
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-border text-left text-faint">
              <th className="py-1.5 pr-3 font-medium">Quando</th>
              {scope === "all" && <th className="py-1.5 pr-3 font-medium">Deck</th>}
              <th className="py-1.5 pr-3 font-medium">Avversario</th>
              <th className="py-1.5 pr-3 font-medium">Turno</th>
              <th className="py-1.5 pr-3 font-medium">Game</th>
              {scope === "deck" && <th className="py-1.5 pr-3 font-medium">Versione</th>}
              <th className="py-1.5 pr-3 font-medium">Tag e note</th>
              <th className="w-8" />
            </tr>
          </thead>
          <tbody>
            {shown.map((m) => (
              <tr key={m.id} className="group border-b border-border/50 hover:bg-surface-3/40">
                <td className="py-1.5 pr-3 whitespace-nowrap text-muted tabular-nums">{playedLabel(m.playedAt)}</td>
                {scope === "all" && <td className="max-w-40 truncate py-1.5 pr-3">{m.deckName}</td>}
                <td className="max-w-40 truncate py-1.5 pr-3">{m.opponent || <span className="text-faint">—</span>}</td>
                <td className="py-1.5 pr-3 text-muted">{m.first == null ? "—" : m.first ? "primo" : "secondo"}</td>
                <td className="py-1.5 pr-3">
                  <span className="flex items-center gap-1">
                    {[...m.games].map((g, i) => (
                      <i key={i} className="grid h-5 w-5 place-items-center rounded-md text-[10px] font-bold text-white not-italic" style={{ background: OUTCOME[g]?.tone }} title={`Game ${i + 1}: ${OUTCOME[g]?.long}`}>
                        {OUTCOME[g]?.short}
                      </i>
                    ))}
                    {m.games.length > 1 && <span className="ml-1 font-semibold" style={{ color: OUTCOME[m.result ?? "D"]?.tone }}>{OUTCOME[m.result ?? "D"]?.long}</span>}
                  </span>
                </td>
                {scope === "deck" && (
                  <td className="max-w-36 truncate py-1.5 pr-3 whitespace-nowrap text-muted" title={versionInfo.get(m.version ?? "")?.name}>
                    {versionInfo.get(m.version ?? "")?.name ?? "—"}
                  </td>
                )}
                <td className="max-w-72 py-1.5 pr-3">
                  <span className="flex flex-wrap items-center gap-1">
                    {m.tags.map((t) => (
                      <span key={t} className="rounded-full bg-surface-3 px-1.5 text-[10px] text-muted">
                        {t}
                      </span>
                    ))}
                    {m.notes && (
                      <span className="truncate text-faint" title={m.notes}>
                        {m.notes}
                      </span>
                    )}
                  </span>
                </td>
                <td className="py-1.5 text-right">
                  <button
                    type="button"
                    onClick={() => void remove(m.id!)}
                    onBlur={() => setConfirm(null)}
                    className={`rounded-md px-1.5 text-[11px] ${confirm === m.id ? "bg-danger/15 text-danger" : "text-faint opacity-0 group-hover:opacity-100 hover:text-danger focus-visible:opacity-100"}`}
                    aria-label="Elimina la partita"
                  >
                    {confirm === m.id ? "Elimina?" : "✕"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {log.matches.length > shown.length && (
          <button type="button" className="self-center text-xs text-sec hover:underline" onClick={() => setAll(true)}>
            Mostra tutte le {log.matches.length} partite
          </button>
        )}
      </section>
    </div>
  );
}

/** Horizontal bars of how often each opponent was met. */
function ShareBars({ rows }: { rows: { name: string; share: number; record: WinRecord }[] }) {
  const top = rows.slice(0, 8);
  const rest = rows.slice(8);
  const items = rest.length ? [...top, { name: `Altri ${rest.length}`, share: rest.reduce((s, r) => s + r.share, 0), record: null }] : top;
  const max = Math.max(...items.map((i) => i.share), 0.01);
  return (
    <div className="flex flex-col gap-1.5">
      {items.map((i, k) => (
        <div key={i.name} className="grid grid-cols-[8.5rem_minmax(0,1fr)_3rem] items-center gap-2 text-xs" tabIndex={0} data-tip={`${i.name}\n${pct(i.share)} delle partite${i.record ? `\n${score(i.record)}` : ""}`}>
          <span className="truncate text-muted" title={i.name}>{i.name}</span>
          <span className="h-3.5 overflow-hidden rounded-r-[4px]">
            <span className="share-bar block h-full rounded-r-[4px]" style={{ width: `${(i.share / max) * 100}%`, background: i.record ? "var(--sec)" : "var(--df-faint)", animationDelay: `${k * 40}ms` }} />
          </span>
          <span className="text-right tabular-nums">{pct(i.share, 0)}</span>
        </div>
      ))}
    </div>
  );
}

/** How many matches it takes to tell two win rates apart. */
function SampleSize({ decidedNow, start = 0.5 }: { decidedNow: number; start?: number }) {
  const [p0, setP0] = useState(Math.min(0.7, Math.max(0.3, start)));
  const [gap, setGap] = useState(0.05);
  const [n, setN] = useState<[number, number] | null>(null);
  const p1 = Math.min(0.95, p0 + gap);

  useEffect(() => {
    let cancelled = false;
    api.sampleSize(p0, p1).then((v) => !cancelled && setN(v));
    return () => {
      cancelled = true;
    };
  }, [p0, p1]);

  const fmt = (v: number) => v.toLocaleString("it-IT");
  const progress = n ? Math.min(1, decidedNow / n[0]) : 0;
  return (
    <section className="lab-card flex flex-col gap-3 p-4">
      <div>
        <h3 className="text-sm font-semibold">Quante partite servono?</h3>
        <p className="text-xs text-muted">Per distinguere due win-rate con il 95% di confidenza e l'80% di potenza.</p>
      </div>
      <div className="grid grid-cols-2 gap-4 text-xs text-muted">
        <label className="flex flex-col gap-1">
          <span>
            Win-rate di partenza <b className="text-text tabular-nums">{pct(p0, 0)}</b>
          </span>
          <input type="range" min={30} max={70} step={1} value={Math.round(p0 * 100)} onChange={(e) => setP0(Number(e.target.value) / 100)} className="accent-[var(--sec)]" />
        </label>
        <label className="flex flex-col gap-1">
          <span>
            Differenza da riconoscere <b className="text-text tabular-nums">+{Math.round(gap * 100)} punti</b>
          </span>
          <input type="range" min={2} max={20} step={1} value={Math.round(gap * 100)} onChange={(e) => setGap(Number(e.target.value) / 100)} className="accent-[var(--sec)]" />
        </label>
      </div>
      {n && (
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className="text-2xl leading-tight font-semibold tabular-nums">{fmt(n[0])}</p>
            <p className="text-xs text-muted">
              partite decise per capire se vinci il {pct(p1, 0)} e non il {pct(p0, 0)}
            </p>
          </div>
          <div>
            <p className="text-2xl leading-tight font-semibold tabular-nums">{fmt(n[1])}</p>
            <p className="text-xs text-muted">per versione, per confrontare due versioni del deck</p>
          </div>
        </div>
      )}
      {n && decidedNow > 0 && (
        <div className="flex items-center gap-2 text-[11px] text-faint">
          <span className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-3">
            <span className="block h-full rounded-full bg-sec" style={{ width: `${progress * 100}%` }} />
          </span>
          {fmt(decidedNow)} di {fmt(n[0])} ({pct(progress, 0)})
        </div>
      )}
    </section>
  );
}
