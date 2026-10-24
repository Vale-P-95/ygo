import { useEffect, useMemo, useRef, useState } from "react";
import { api, type Card, type Checkpoint, type Cond, type SimEffect, type SimResult, type SimRule } from "../../lib/api";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { pct } from "../../lib/tooltip";
import { ChartCard, ConvergenceChart, FlaskMeter } from "../charts";

/** Draw spells the simulator recognises by name. */
const KNOWN: Record<string, SimEffect> = {
  "Pot of Desires": { type: "draw", n: 2, banishTop: 10 },
  "Pot of Extravagance": { type: "draw", n: 2, banishTop: 0 },
  "Pot of Prosperity": { type: "excavate", n: 6 },
  "Pot of Duality": { type: "excavate", n: 3 },
  "Upstart Goblin": { type: "draw", n: 1, banishTop: 0 },
  "Allure of Darkness": { type: "draw", n: 2, banishTop: 0 },
  "Pot of Greed": { type: "draw", n: 2, banishTop: 0 },
};

const effectText = (e: SimEffect): string =>
  e.type === "draw" ? `${e.banishTop ? `bandisce ${e.banishTop}, ` : ""}pesca ${e.n}` : e.type === "excavate" ? `guarda ${e.n}, ne aggiunge 1` : `cerca ${e.targets.length} ${e.targets.length === 1 ? "carta" : "carte"}`;

const SIZES = [100_000, 1_000_000, 5_000_000];

function loadRules(deckId: number): SimRule[] | null {
  try {
    const raw = localStorage.getItem(`duelforge.simRules.${deckId}`);
    return raw ? (JSON.parse(raw) as SimRule[]) : null;
  } catch {
    return null;
  }
}

function storeRules(deckId: number, rules: SimRule[]) {
  try {
    localStorage.setItem(`duelforge.simRules.${deckId}`, JSON.stringify(rules));
  } catch {
    // Only a convenience: the rules come back next time.
  }
}

interface Props {
  deckId: number;
  main: number[];
  roles: Record<string, number[]>;
  cond: Cond;
  hand: "first" | "second";
  getCard: (id: number) => Card | undefined;
}

/** Simulatore: Monte Carlo with draw spells, searchers and summon limits. */
export function SimulatorTab({ deckId, main, roles, cond, hand, getCard }: Props) {
  const ids = useMemo(() => [...new Set(main)], [main]);
  const [rules, setRules] = useState<SimRule[]>(() => loadRules(deckId) ?? []);
  const [size, setSize] = useState(1_000_000);
  const [seed, setSeed] = useState(() => Math.floor(Math.random() * 1e9));
  const [running, setRunning] = useState(false);
  const [points, setPoints] = useState<Checkpoint[]>([]);
  const [result, setResult] = useState<SimResult | null>(null);
  const [exact, setExact] = useState<number | null>(null);
  const [searcher, setSearcher] = useState<number | "">("");
  const handSize = hand === "first" ? 5 : 6;
  const started = useRef(0);

  // Until the player edits the rules, draw spells in the deck get theirs
  // automatically (once the card names are known).
  const names = ids.map((id) => getCard(id)?.name ?? "").join("|");
  useEffect(() => {
    if (loadRules(deckId)) return;
    const auto: SimRule[] = ids.flatMap((id) => {
      const effect = KNOWN[getCard(id)?.name ?? ""];
      return effect ? [{ id, effect, oncePerTurn: true, normalSummon: false }] : [];
    });
    setRules(auto);
  }, [deckId, names]); // eslint-disable-line react-hooks/exhaustive-deps

  const condKey = JSON.stringify(cond);
  useEffect(() => {
    let stale = false;
    api
      .labProbability(main, roles, cond, handSize)
      .then((r) => !stale && setExact(r.p))
      .catch((e) => !stale && toast(`Probabilità esatta non calcolata: ${e}`, "error"));
    return () => {
      stale = true;
    };
  }, [main, roles, condKey, handSize]); // eslint-disable-line react-hooks/exhaustive-deps

  const rule = (id: number) => rules.find((r) => r.id === id);
  const setRule = (id: number, change: Partial<SimRule>) =>
    setRules((list) => {
      const current = list.find((r) => r.id === id) ?? { id, effect: null, oncePerTurn: false, normalSummon: false };
      const next = { ...current, ...change };
      const rest = list.filter((r) => r.id !== id);
      const out = next.effect || next.oncePerTurn || next.normalSummon ? [...rest, next] : rest;
      storeRules(deckId, out);
      return out;
    });

  const run = async () => {
    sound.play("open");
    setRunning(true);
    setPoints([]);
    setResult(null);
    started.current = performance.now();
    const unlisten = await api.onSimProgress((c) => setPoints((p) => [...p, c]));
    try {
      const r = await api.labSimulate(main, roles, cond, handSize, rules, size, seed);
      setResult(r);
      setPoints(r.checkpoints);
      sound.play(r.cancelled ? "close" : "glass");
    } catch (e) {
      sound.play("error");
      toast(`Simulazione non riuscita: ${e}`, "error");
    } finally {
      unlisten();
      setRunning(false);
    }
  };

  const last = result ?? points[points.length - 1];
  const name = (id: number) => getCard(id)?.name ?? String(id);
  const searchers = rules.filter((r) => r.effect?.type === "search");
  const draws = rules.filter((r) => r.effect && r.effect.type !== "search");
  const seconds = result ? (performance.now() - started.current) / 1000 : 0;

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_320px] gap-4">
      <div className="flex min-w-0 flex-col gap-4">
        <ChartCard title="Convergenza" note="La stima si stringe intorno al valore vero man mano che le mani aumentano; la banda è l'intervallo al 95%.">
          {points.length ? (
            <ConvergenceChart points={points} exact={exact} ariaLabel="Convergenza della simulazione" />
          ) : (
            <p className="py-10 text-center text-sm text-faint">Premi «Simula» per vedere la stima convergere.</p>
          )}
        </ChartCard>

        <section className="lab-card flex flex-col gap-3 p-4">
          <div className="flex items-baseline justify-between">
            <h3 className="text-sm font-semibold">Pescate e scavi</h3>
            <span className="text-xs text-faint">riconosciuti per nome, una volta per turno</span>
          </div>
          {draws.length === 0 && <p className="text-xs text-faint">Nessun Pot o carta che pesca nel Main Deck.</p>}
          {draws.map((r) => (
            <div key={r.id} className="flex items-center gap-3 text-sm">
              <span className="min-w-0 flex-1 truncate">{name(r.id)}</span>
              <span className="text-xs text-muted">{r.effect && effectText(r.effect)}</span>
              <button type="button" className="text-xs text-faint hover:text-danger" onClick={() => setRule(r.id, { effect: null })}>
                Ignora
              </button>
            </div>
          ))}
        </section>

        <section className="lab-card flex flex-col gap-3 p-4">
          <div className="flex items-baseline justify-between">
            <h3 className="text-sm font-semibold">Ricerche</h3>
            <span className="text-xs text-faint">la carta vale come quella che prende, se è ancora nel deck</span>
          </div>
          {searchers.map((r) => (
            <div key={r.id} className="flex flex-col gap-1.5 rounded-lg border border-border p-2.5">
              <div className="flex items-center gap-2 text-sm">
                <b className="min-w-0 flex-1 truncate">{name(r.id)}</b>
                <span className="text-xs text-muted">cerca</span>
                <button type="button" className="text-xs text-faint hover:text-danger" onClick={() => setRule(r.id, { effect: null })}>
                  Togli
                </button>
              </div>
              <div className="flex flex-wrap gap-1">
                {ids
                  .filter((id) => id !== r.id)
                  .map((id) => {
                    const on = r.effect?.type === "search" && r.effect.targets.includes(id);
                    return (
                      <button
                        key={id}
                        type="button"
                        className={`chip !px-2 !py-0.5 !text-[11px] ${on ? "chip-active" : "opacity-70"}`}
                        onClick={() => {
                          const targets = r.effect?.type === "search" ? r.effect.targets : [];
                          setRule(r.id, { effect: { type: "search", targets: on ? targets.filter((t) => t !== id) : [...targets, id] } });
                        }}
                      >
                        {name(id)}
                      </button>
                    );
                  })}
              </div>
            </div>
          ))}
          <div className="flex gap-2">
            <select className="field !py-1 text-sm" value={searcher} onChange={(e) => setSearcher(e.target.value ? Number(e.target.value) : "")} aria-label="Carta che cerca">
              <option value="">Scegli la carta che cerca…</option>
              {ids
                .filter((id) => rule(id)?.effect?.type !== "search")
                .map((id) => (
                  <option key={id} value={id}>
                    {name(id)}
                  </option>
                ))}
            </select>
            <button
              type="button"
              className="btn btn-ghost shrink-0 !py-1 text-xs"
              disabled={searcher === ""}
              onClick={() => {
                if (searcher === "") return;
                sound.play("select");
                setRule(searcher, { effect: { type: "search", targets: [] } });
                setSearcher("");
              }}
            >
              ＋ Ricerca
            </button>
          </div>
        </section>

        <section className="lab-card flex flex-col gap-2 p-4">
          <div className="flex items-baseline justify-between">
            <h3 className="text-sm font-semibold">Limiti</h3>
            <span className="text-xs text-faint">1/T: una copia per turno · NS: richiede la Normal Summon (ne conta una)</span>
          </div>
          <div className="grid grid-cols-2 gap-x-6 gap-y-1">
            {ids.map((id) => {
              const r = rule(id);
              return (
                <div key={id} className="flex items-center gap-2 text-xs">
                  <span className="min-w-0 flex-1 truncate">{name(id)}</span>
                  <button type="button" onClick={() => setRule(id, { oncePerTurn: !r?.oncePerTurn })} className={`chip !px-1.5 !py-0 !text-[10px] ${r?.oncePerTurn ? "chip-active" : ""}`}>
                    1/T
                  </button>
                  <button type="button" onClick={() => setRule(id, { normalSummon: !r?.normalSummon })} className={`chip !px-1.5 !py-0 !text-[10px] ${r?.normalSummon ? "chip-active" : ""}`}>
                    NS
                  </button>
                </div>
              );
            })}
          </div>
        </section>
      </div>

      <div className="flex flex-col gap-4">
        <section className="lab-card flex flex-col items-center gap-2 p-5 text-center">
          <FlaskMeter value={last?.p ?? 0} size={70} bubbling={running} />
          <p className="text-4xl font-semibold">{last ? pct(last.p) : "—"}</p>
          <p className="text-xs text-muted">{last ? `intervallo 95%: ${pct(last.lo, 2)} – ${pct(last.hi, 2)}` : `${handSize} carte, ${hand === "first" ? "per primo" : "per secondo"}`}</p>
          {last && <p className="text-[11px] text-faint">{last.hands.toLocaleString("it-IT")} mani{result && !result.cancelled ? ` in ${seconds.toLocaleString("it-IT", { maximumFractionDigits: 1 })} s` : ""}</p>}
          {result && exact != null && (
            <p className="text-xs">
              Senza effetti {pct(exact)} ·{" "}
              <b className={result.p >= exact ? "text-ok" : "text-danger"}>
                {result.p >= exact ? "+" : "−"}
                {Math.abs((result.p - exact) * 100).toLocaleString("it-IT", { maximumFractionDigits: 1 })} punti
              </b>
            </p>
          )}
        </section>
        <section className="lab-card flex flex-col gap-3 p-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-muted">Mani da simulare</span>
            <div className="flex gap-1 rounded-lg border border-border bg-bg-2 p-0.5">
              {SIZES.map((s) => (
                <button key={s} type="button" onClick={() => setSize(s)} className={`flex-1 rounded-md py-1 text-xs font-semibold ${size === s ? "bg-sec text-bg" : "text-muted hover:text-text"}`}>
                  {s >= 1e6 ? `${s / 1e6} mln` : `${s / 1e3}k`}
                </button>
              ))}
            </div>
          </div>
          <label className="flex items-center gap-2 text-xs text-muted">
            Seed
            <input
              className="field !py-1 font-mono text-xs"
              value={seed}
              onChange={(e) => setSeed(Math.min(Number.MAX_SAFE_INTEGER, Number(e.target.value.replace(/\D/g, "")) || 0))}
            />
            <button type="button" className="btn btn-ghost shrink-0 !px-2 !py-1" onClick={() => setSeed(Math.floor(Math.random() * 1e9))} title="Nuovo seed" aria-label="Nuovo seed">
              ⟳
            </button>
          </label>
          {running ? (
            <button type="button" className="btn btn-ghost" onClick={() => void api.labCancelSimulation()}>
              Ferma
            </button>
          ) : (
            <button type="button" className="btn btn-molten" onClick={() => void run()}>
              Simula
            </button>
          )}
          <p className="text-[11px] text-faint">Stesso seed, stesso risultato. Una carta che risolve il suo effetto non conta più per i suoi ruoli: è diventata quella che ha pescato o cercato.</p>
        </section>
        {result && result.effects.length > 0 && (
          <section className="lab-card flex flex-col gap-1.5 p-4">
            <h3 className="text-sm font-semibold">Effetti usati</h3>
            {result.effects.map(([id, share]) => (
              <div key={id} className="flex items-center gap-2 text-xs" data-tip={`${name(id)}\nha risolto il suo effetto nel ${pct(share)} delle mani`}>
                <span className="min-w-0 flex-1 truncate">{name(id)}</span>
                <span className="text-muted tabular-nums">{pct(share)}</span>
              </div>
            ))}
          </section>
        )}
      </div>
    </div>
  );
}
