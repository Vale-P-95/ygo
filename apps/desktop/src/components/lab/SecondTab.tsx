import { useEffect, useRef, useState } from "react";
import { api, cardImage, type Card, type Deck, type SecondHand, type SecondOpponent, type SecondResult } from "../../lib/api";
import { sound } from "../../lib/sound";
import { pct } from "../../lib/tooltip";

interface Props {
  deck: Deck;
  getCard: (id: number) => Card | undefined;
  /** Loads the images and names of these cards. */
  request: (ids: number[]) => void;
}

const HAND_CHOICES = [6, 10, 16];
/** Nodes the search spends on a hand: the CLI's default for `break-board`. */
const NODES = 4000;

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

function HandRow({ row, getCard }: { row: SecondHand; getCard: Props["getCard"] }) {
  const [open, setOpen] = useState(false);
  const lethal = row.share <= 0;
  return (
    <li className="border-t border-border first:border-t-0">
      <button type="button" className="flex w-full flex-wrap items-center gap-x-4 gap-y-2 px-4 py-2.5 text-left" onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <span className="flex gap-1">
          {row.hand.map((c, i) => (
            <Thumb key={i} code={c} getCard={getCard} />
          ))}
        </span>
        <span
          className="w-16 shrink-0 text-xs text-muted"
          title="Carte della tua mano che il motore sa usare per rompere una board: negazioni, rimozioni, bandire"
        >
          {row.breakers > 0 ? `${row.breakers} breaker` : "nessun breaker"}
        </span>
        <span className="flex min-w-40 flex-1 items-center gap-2" title="Quota del valore della board avversaria che resta dopo il tuo turno, meno i suoi Life Points tolti con la Battle Phase">
          <span className="h-1.5 w-28 shrink-0 overflow-hidden rounded-full bg-bg-2">
            <span className="block h-full rounded-full bg-sec" style={{ width: `${Math.round(Math.min(1, row.share) * 100)}%` }} />
          </span>
          <span className={`tabular-nums ${lethal ? "font-semibold text-text" : ""}`}>{lethal ? "Letale" : `resta ${pct(row.share)}`}</span>
        </span>
        <span className="w-24 shrink-0 text-right text-xs text-muted tabular-nums" title="Life Points che l'avversario perde nella Battle Phase">
          {row.damage > 0 ? `−${row.damage.toLocaleString("it-IT")} LP` : "0 LP"}
        </span>
      </button>
      {open && (
        <div className="flex flex-col gap-2 px-4 pb-3 text-sm">
          <div className="flex items-center gap-2">
            <span className="w-28 shrink-0 text-xs text-muted">Gli resta in campo</span>
            <span className="flex flex-wrap gap-1">
              {row.kept.length === 0 ? <span className="text-muted">Niente</span> : row.kept.map((c, i) => <Thumb key={i} code={c} getCard={getCard} />)}
            </span>
          </div>
          <div className="flex items-start gap-2">
            <span className="w-28 shrink-0 text-xs text-muted">La tua linea</span>
            {row.line ? (
              <ol className="list-inside list-decimal text-muted">
                {row.line.map((step, i) => (
                  <li key={i}>{step}</li>
                ))}
              </ol>
            ) : (
              <span className="text-muted">Il motore non ha trovato nessuna linea per questa mano.</span>
            )}
          </div>
        </div>
      )}
    </li>
  );
}

/** Per secondo: how much of a meta deck's best board each of the deck's hands of 6 breaks, with its answers and a Battle Phase. */
export function SecondTab({ deck, getCard, request }: Props) {
  const [opponents, setOpponents] = useState<SecondOpponent[] | null>(null);
  const [vs, setVs] = useState("");
  const [hands, setHands] = useState(HAND_CHOICES[0]);
  const [result, setResult] = useState<SecondResult | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const reqId = useRef(0);
  // A new deck (the tab is keyed by it) never shows another's result: an answer after unmounting is dropped, and a run in
  // progress is stopped.
  const alive = useRef(true);
  const active = useRef<{ id: number; vs: string } | null>(null);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (active.current) void api.secondCancel().catch(() => {});
    };
  }, []);

  useEffect(() => {
    let live = true;
    api
      .secondOpponents()
      .then((list) => {
        if (!live) return;
        setOpponents(list);
        setVs((v) => v || list[0]?.name || "");
      })
      .catch((e) => live && setError(`Deck del meta non letti: ${e}`));
    return () => {
      live = false;
    };
  }, []);

  // Hands arrive one by one while the run goes on; only the running request's are kept.
  useEffect(() => {
    let off: (() => void) | null = null;
    let gone = false;
    void api.onSecondProgress((p) => {
      const now = active.current;
      if (!now || !p.result || p.deckId !== deck.id || p.vs !== now.vs) return;
      setResult(p.result);
      request([...new Set([...p.result.rival.hand, ...p.result.rival.worth.map((w) => w.code), ...p.result.hands.flatMap((h) => [...h.hand, ...h.kept])])]);
    }).then((un) => {
      if (gone) un();
      else off = un;
    });
    return () => {
      gone = true;
      off?.();
    };
  }, [deck.id, request]);

  const run = async () => {
    if (deck.id == null || !vs) return;
    const id = ++reqId.current;
    sound.play("select");
    active.current = { id, vs };
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await api.secondRun({ deckId: deck.id, vs, hands, nodes: NODES });
      if (!alive.current || reqId.current !== id || r.deckId !== deck.id) return;
      setResult(r);
    } catch (e) {
      if (alive.current && reqId.current === id) setError(`Misura non completata: ${e}`);
    } finally {
      if (reqId.current === id) {
        active.current = null;
        if (alive.current) setRunning(false);
      }
    }
  };

  const mean = (rows: SecondHand[]) => (rows.length ? rows.reduce((a, r) => a + r.share, 0) / rows.length : null);
  const withBreaker = result?.hands.filter((h) => h.breakers > 0) ?? [];
  const without = result?.hands.filter((h) => h.breakers === 0) ?? [];
  const rival = result?.rival;

  return (
    <div className="flex flex-col gap-4">
      <div className="lab-card flex flex-wrap items-center gap-3 px-4 py-3">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold">Quanta board del meta rompe la tua mano, andando secondo</p>
          <p className="text-xs text-muted">
            Il motore costruisce la migliore board di un deck del meta, poi le mani da 6 del tuo deck giocano il loro turno contro di essa: lei risponde
            con le sue interruzioni e tu hai la Battle Phase. Ogni mano vale la quota della board che resta, meno i Life Points che le togli.
          </p>
        </div>
        <label className="flex items-center gap-2 text-xs text-muted">
          Contro
          <select className="field !w-auto !py-1 text-sm" value={vs} disabled={running || !opponents} onChange={(e) => setVs(e.target.value)} aria-label="Deck avversario">
            {(opponents ?? []).map((o) => (
              <option key={o.name} value={o.name}>
                {o.name} ({o.weight.toLocaleString("it-IT", { maximumFractionDigits: 1 })}%)
              </option>
            ))}
          </select>
        </label>
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
        {running ? (
          <button type="button" className="btn shrink-0" onClick={() => void api.secondCancel().catch((e) => setError(`Impossibile fermare: ${e}`))}>
            Ferma
          </button>
        ) : (
          <button type="button" className="btn btn-molten shrink-0" disabled={deck.id == null || !vs} onClick={() => void run()}>
            {result ? "Ricalcola" : "Calcola"}
          </button>
        )}
      </div>

      {error && <p className="lab-card border-danger px-4 py-3 text-sm text-danger">{error}</p>}

      {!result && !running && !error && <p className="text-sm text-muted">Scegli un deck del meta e premi «Calcola»: servono alcuni minuti.</p>}
      {running && !rival && <p className="text-sm text-muted">Costruisco la board dell'avversario dalle sue mani migliori: ci vuole circa un minuto.</p>}

      {rival && result && (
        <section className="lab-card flex flex-col gap-3 px-4 py-3">
          <p className="text-sm font-semibold">La board di {result.vs}</p>
          <div className="flex flex-wrap items-start gap-x-6 gap-y-3">
            <div className="flex flex-col gap-1">
              <span className="text-xs text-muted">Sul campo (quanto vale ogni carta per la board)</span>
              <span className="flex flex-wrap gap-2">
                {rival.worth.map((w, i) => (
                  <span key={i} className="flex flex-col items-center gap-0.5">
                    <Thumb code={w.code} getCard={getCard} />
                    <span className="text-[11px] text-muted tabular-nums">{w.worth.toLocaleString("it-IT", { maximumFractionDigits: 1 })}</span>
                  </span>
                ))}
              </span>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs text-muted">Risponde al tuo turno con</span>
              <span className="flex flex-wrap gap-1">
                {rival.responders.length === 0 ? <span className="text-sm text-muted">niente</span> : rival.responders.map((c, i) => <Thumb key={i} code={c} getCard={getCard} />)}
              </span>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs text-muted">I tuoi breaker</span>
              <span className="flex flex-wrap gap-1">
                {result.breakers.length === 0 ? <span className="text-sm text-muted">nessuno</span> : result.breakers.map((c, i) => <Thumb key={i} code={c} getCard={getCard} />)}
              </span>
            </div>
          </div>
        </section>
      )}

      {result && result.hands.length > 0 && (
        <section className="lab-card">
          <div className="flex flex-wrap items-baseline gap-x-6 gap-y-1 px-4 py-3">
            <p className="text-sm font-semibold">
              {result.hands.length} {result.hands.length === 1 ? "mano" : "mani"}
              {running && ` su ${result.asked}`}
              {result.stopped && ` su ${result.asked} (fermato)`}
            </p>
            <p className="text-xs text-muted tabular-nums">
              Resta in media {pct(mean(result.hands) ?? 0)}
              {withBreaker.length > 0 && without.length > 0 && ` · con un breaker ${pct(mean(withBreaker) ?? 0)} (${withBreaker.length}), senza ${pct(mean(without) ?? 0)} (${without.length})`}
              {" · "}
              {result.seconds.toLocaleString("it-IT", { maximumFractionDigits: 0 })} s
            </p>
          </div>
          <ul>
            {result.hands.map((h, i) => (
              <HandRow key={i} row={h} getCard={getCard} />
            ))}
          </ul>
        </section>
      )}

      {result && (
        <p className="text-xs text-faint">
          Le risposte dell'avversario sono ingenue: usa un'interruzione alla prima occasione, non al momento migliore. È una misura a parte, non quella del primo turno: i numeri di mani diverse si confrontano tra loro, non con il punteggio del Combo Lab.
        </p>
      )}
    </div>
  );
}
