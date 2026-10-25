import { useEffect, useMemo, useRef, useState } from "react";
import { api, cardImage, type Card, type Deck, type McDeck, type McHand, type McProgress, type McResult } from "../../lib/api";
import { isExtraDeckCard } from "../../lib/deck";
import { sound } from "../../lib/sound";
import { pct } from "../../lib/tooltip";

interface Props {
  deck: Deck;
  getCard: (id: number) => Card | undefined;
  /** Loads the images and names of these cards. */
  request: (ids: number[]) => void;
}

const HAND_CHOICES = [40, 100, 200];
/** Nodes the search spends on a hand: the CLI's default for `engine-mc`. */
const NODES = 2000;
/** Ids are unique across runs, even of a tab that is mounted again. */
let nextRequest = 1;

function Thumb({ code, getCard, dim = false }: { code: number; getCard: Props["getCard"]; dim?: boolean }) {
  const name = getCard(code)?.name ?? `#${code}`;
  return (
    <img
      src={cardImage(code, "small")}
      alt={name}
      title={name}
      loading="lazy"
      className={`h-[42px] w-[30px] shrink-0 rounded-[2px] object-cover ${dim ? "opacity-40" : ""}`}
    />
  );
}

const points = (x: number) => (x * 100).toLocaleString("it-IT", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const sign = (x: number) => (x >= 0 ? "+" : "−");

/** How the values of the hands spread, in 8 equal steps up to the best one; the threshold marks where "a good board" starts. */
function Histogram({ hands, threshold }: { hands: McHand[]; threshold: number }) {
  const max = Math.max(1e-9, ...hands.map((h) => h.value));
  const bins = Array.from({ length: 8 }, () => 0);
  for (const h of hands) bins[Math.min(7, Math.floor((h.value / max) * 8))]++;
  const top = Math.max(1, ...bins);
  return (
    <div className="flex h-16 items-end gap-1" role="img" aria-label="Distribuzione dei valori delle mani">
      {bins.map((n, i) => {
        const from = (i / 8) * max;
        const good = from + max / 16 >= threshold;
        return (
          <span
            key={i}
            title={`Valore da ${from.toFixed(1)}: ${n} ${n === 1 ? "mano" : "mani"}`}
            className={`w-6 rounded-t-[2px] ${good ? "bg-sec" : "bg-bg-2"}`}
            style={{ height: `${Math.max(4, (n / top) * 100)}%` }}
          />
        );
      })}
    </div>
  );
}

function HandRow({ row, threshold, getCard }: { row: McHand; threshold: number; getCard: Props["getCard"] }) {
  const good = row.value >= threshold;
  return (
    <li className="flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-border px-4 py-2 first:border-t-0">
      <span className="flex gap-1">
        {row.hand.map((c, i) => (
          <Thumb key={i} code={c} getCard={getCard} dim={!row.used.includes(c)} />
        ))}
      </span>
      <span className={`w-20 shrink-0 text-right tabular-nums ${good ? "font-semibold text-text" : "text-muted"}`} title="Valore della migliore board della mano">
        {row.value.toLocaleString("it-IT", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
      </span>
      <span className="text-xs text-muted">{row.value <= 0 ? "nessuna board" : good ? "sopra soglia" : "sotto soglia"}</span>
    </li>
  );
}

function Summary({ title, d, threshold }: { title: string; d: McDeck; threshold: number }) {
  return (
    <div className="flex min-w-48 flex-col gap-1">
      <span className="text-xs text-muted">{title}</span>
      <span className="text-2xl font-semibold tabular-nums">{pct(d.p)}</span>
      <span className="text-xs text-muted tabular-nums" title="Intervallo al 95% dopo la correzione con le probabilità esatte delle carte in mano">
        ± {points(d.pHalf)} punti
      </span>
      <span className="text-xs text-faint tabular-nums" title="Quota del solo campione e suo intervallo di Wilson al 95%">
        campione {pct(d.pSample)} ({pct(d.low)}–{pct(d.high)})
      </span>
      <span className="text-xs text-muted tabular-nums" title={`Valore medio della migliore board di una mano; soglia ${threshold.toFixed(2)}`}>
        valore medio {d.mean.toLocaleString("it-IT", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ± {d.meanHalf.toLocaleString("it-IT", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
      </span>
    </div>
  );
}

/** Monte Carlo del motore: how often the deck's opening hands reach a good board, played by the Combo Lab hand by hand. */
export function McTab({ deck, getCard, request }: Props) {
  const [hands, setHands] = useState(HAND_CHOICES[1]);
  const [out, setOut] = useState(0);
  const [into, setInto] = useState(0);
  const [result, setResult] = useState<McResult | null>(null);
  const [progress, setProgress] = useState<Pick<McProgress, "phase" | "done" | "total"> | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // A new deck (the tab is keyed by it) never shows another's result: an answer after unmounting is dropped, and a run in
  // progress is stopped.
  const alive = useRef(true);
  const active = useRef<number | null>(null);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (active.current != null) void api.mcCancel().catch(() => {});
    };
  }, []);

  const mainCards = useMemo(() => [...new Set(deck.main)], [deck.main]);
  const sideCards = useMemo(() => [...new Set(deck.side)].filter((c) => !(getCard(c) && isExtraDeckCard(getCard(c)!))), [deck.side, getCard]);
  useEffect(() => {
    request([...new Set([...deck.main, ...deck.side])]);
  }, [deck.main, deck.side, request]);
  const name = (id: number) => getCard(id)?.name ?? `#${id}`;

  // Only the running request's hands count; the cards of the result are asked for when it arrives.
  useEffect(() => {
    let off: (() => void) | null = null;
    let gone = false;
    void api.onMcProgress((p) => {
      if (p.requestId !== active.current || p.deckId !== deck.id) return;
      setProgress({ phase: p.phase, done: p.done, total: p.total });
    }).then((un) => {
      if (gone) un();
      else off = un;
    });
    return () => {
      gone = true;
      off?.();
    };
  }, [deck.id]);

  const run = async () => {
    if (deck.id == null) return;
    const id = nextRequest++;
    sound.play("select");
    active.current = id;
    setRunning(true);
    setError(null);
    setResult(null);
    setProgress(null);
    const swap = out && into ? { out, into } : null;
    try {
      const r = await api.mcRun({ deckId: deck.id, requestId: id, hands, nodes: NODES, swap });
      if (!alive.current || active.current !== id || r.deckId !== deck.id) return;
      request([...new Set(r.deck.hands.flatMap((h) => h.hand))]);
      setResult(r);
    } catch (e) {
      if (alive.current && active.current === id) setError(`Misura non completata: ${e}`);
    } finally {
      if (active.current === id) {
        active.current = null;
        if (alive.current) setRunning(false);
      }
    }
  };

  const swap = result?.swap ?? null;
  const verdict = swap ? (swap.dp - swap.dpHalf > 0 ? "migliora" : swap.dp + swap.dpHalf < 0 ? "peggiora" : "non si distingue dal caso") : null;
  const sorted = useMemo(() => (result ? [...result.deck.hands].sort((a, b) => b.value - a.value) : []), [result]);

  return (
    <div className="flex flex-col gap-4">
      <div className="lab-card flex flex-wrap items-center gap-3 px-4 py-3">
        <div className="min-w-full">
          <p className="text-sm font-semibold">Quante mani fanno una buona board, giocate dal motore</p>
          <p className="text-xs text-muted">
            Il Combo Lab gioca davvero ogni mano da 5 carte pescata dal tuo Main Deck e misura la sua migliore board. «Buona» vuol dire almeno la mediana del
            deck. Puoi anche provare lo scambio di una copia con una carta del Side Deck: il confronto usa le stesse mani, quindi è molto più preciso.
          </p>
        </div>
        <label className="flex items-center gap-2 text-xs text-muted">
          Mani
          <select className="field !w-auto !py-1 text-sm" value={hands} disabled={running} onChange={(e) => setHands(Number(e.target.value))} aria-label="Numero di mani">
            {HAND_CHOICES.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-xs text-muted">
          Togli
          <select className="field !w-auto max-w-44 !py-1 text-sm" value={out} disabled={running} onChange={(e) => setOut(Number(e.target.value))} aria-label="Carta da togliere">
            <option value={0}>nessuno scambio</option>
            {mainCards.map((c) => (
              <option key={c} value={c}>
                {name(c)}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-xs text-muted">
          Metti
          <select
            className="field !w-auto max-w-44 !py-1 text-sm"
            value={into}
            disabled={running || !out}
            onChange={(e) => setInto(Number(e.target.value))}
            aria-label="Carta da mettere, dal Side Deck"
          >
            <option value={0}>{sideCards.length ? "scegli dal Side" : "Side vuoto"}</option>
            {sideCards.map((c) => (
              <option key={c} value={c}>
                {name(c)}
              </option>
            ))}
          </select>
        </label>
        {running ? (
          <button type="button" className="btn ml-auto shrink-0" onClick={() => void api.mcCancel().catch((e) => setError(`Impossibile fermare: ${e}`))}>
            Ferma
          </button>
        ) : (
          <button type="button" className="btn btn-molten ml-auto shrink-0" disabled={deck.id == null || deck.main.length < 5 || (out !== 0 && into === 0)} onClick={() => void run()}>
            {result ? "Ricalcola" : "Calcola"}
          </button>
        )}
      </div>

      {error && <p className="lab-card border-danger px-4 py-3 text-sm text-danger">{error}</p>}

      {!result && !running && !error && (
        <p className="text-sm text-muted">Scegli quante mani e premi «Calcola»: con 100 mani servono alcuni minuti, e di più con lo scambio.</p>
      )}

      {running && (
        <div className="lab-card flex flex-col gap-2 px-4 py-3" role="status">
          <p className="text-sm">
            {!progress
              ? "Preparo il motore e i valori delle carte: ci vuole circa un minuto."
              : `${progress.phase === "swap" ? "Con lo scambio" : "Deck"}: mano ${progress.done} di ${progress.total}`}
          </p>
          <span className="h-1.5 w-full overflow-hidden rounded-full bg-bg-2">
            <span className="block h-full rounded-full bg-sec transition-all" style={{ width: `${progress ? Math.round((progress.done / Math.max(1, progress.total)) * 100) : 0}%` }} />
          </span>
        </div>
      )}

      {result && (
        <section className="lab-card flex flex-col gap-4 px-4 py-3">
          {result.stopped && (
            <p className="text-xs text-muted">Fermato: i numeri usano solo le {result.deck.hands.length} mani finite, quindi sono meno precisi.</p>
          )}
          <div className="flex flex-wrap items-start gap-x-8 gap-y-4">
            <Summary title={`Mani con una board da almeno ${result.threshold.toLocaleString("it-IT", { maximumFractionDigits: 2 })}`} d={result.deck} threshold={result.threshold} />
            {swap && (
              <>
                <Summary title={`Con −1 ${name(swap.out)} +1 ${name(swap.into)}`} d={swap.deck} threshold={result.threshold} />
                <div className="flex min-w-48 flex-col gap-1">
                  <span className="text-xs text-muted">Differenza sulle stesse mani</span>
                  <span className="text-2xl font-semibold tabular-nums">
                    {sign(swap.dp)}
                    {points(Math.abs(swap.dp))} punti
                  </span>
                  <span className="text-xs text-muted tabular-nums">± {points(swap.dpHalf)} · {verdict}</span>
                  <span className="text-xs text-faint tabular-nums">{swap.changed} mani su {swap.deck.hands.length} cambiate dallo scambio</span>
                </div>
              </>
            )}
            <div className="flex flex-col gap-1">
              <span className="text-xs text-muted">Valori delle mani</span>
              <Histogram hands={result.deck.hands} threshold={result.threshold} />
              <span className="text-xs text-faint">in verde le mani sopra soglia</span>
            </div>
          </div>
          <p className="text-xs text-faint tabular-nums">
            {result.deck.searched} mani cercate dal motore su {result.deck.hands.length} · {result.seconds.toLocaleString("it-IT", { maximumFractionDigits: 0 })} s
          </p>
        </section>
      )}

      {result && sorted.length > 0 && (
        <section className="lab-card">
          <p className="px-4 py-3 text-sm font-semibold">Le mani, dalla migliore</p>
          <ul>
            {sorted.map((h, i) => (
              <HandRow key={i} row={h} threshold={result.threshold} getCard={getCard} />
            ))}
          </ul>
          <p className="px-4 pb-3 text-xs text-faint">Le carte in grigio chiaro non sono state usate dalla linea migliore.</p>
        </section>
      )}

      {result && (
        <p className="text-xs text-faint">
          È il giudizio della misura del Combo Lab (profilo «Bilanciato»), non un consiglio di gioco: una mano senza board vale zero, e le handtrap tenute in mano contano.
          Un intervallo largo vuol dire che servono più mani.
        </p>
      )}
    </div>
  );
}
