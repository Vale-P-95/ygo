import { useEffect, useRef, useState } from "react";
import { api, cardImage, type Card, type Deck, type TechRow, type TechTable } from "../../lib/api";
import { sound } from "../../lib/sound";
import { pct } from "../../lib/tooltip";

interface Props {
  deck: Deck;
  getCard: (id: number) => Card | undefined;
  /** Loads the images and names of these cards. */
  request: (ids: number[]) => void;
}

/** A share of the board taken as a tinted cell: the more it takes, the stronger the tint. */
function Share({ share, usable, title }: { share: number; usable: boolean; title: string }) {
  if (!usable) {
    return (
      <span className="text-faint" title={title}>
        —
      </span>
    );
  }
  return (
    <span
      className="inline-block min-w-11 rounded-md px-1.5 py-0.5 text-center tabular-nums"
      style={{ background: `color-mix(in oklab, var(--sec) ${Math.round(share * 60)}%, transparent)` }}
      title={title}
    >
      {pct(share)}
    </span>
  );
}

function Row({ row, table, getCard }: { row: TechRow; table: TechTable; getCard: (id: number) => Card | undefined }) {
  const name = getCard(row.code)?.name ?? `#${row.code}`;
  const none = row.lost === 0;
  return (
    <tr className={`border-t border-border ${none ? "text-muted" : ""}`}>
      <td className="max-w-64 py-2 pr-3 pl-4">
        <span className="flex min-w-0 items-center gap-2">
          <img src={cardImage(row.code, "small")} alt="" loading="lazy" className="h-[34px] w-[25px] shrink-0 rounded-[2px] object-cover" />
          <span className="flex min-w-0 flex-col">
            <span className="truncate font-medium" title={name}>
              {name}
            </span>
            {row.at.length > 0 && (
              <span
                className="truncate text-xs text-muted"
                title={`Risponde a: ${row.at.map((a) => `${getCard(a.code)?.name ?? a.code} (${a.lines} linee)`).join(", ")}`}
              >
                di solito su {getCard(row.at[0].code)?.name ?? `#${row.at[0].code}`}
              </span>
            )}
          </span>
        </span>
      </td>
      <td className="px-2 py-2 text-right whitespace-nowrap tabular-nums" title="Copie nel Main e nel Side">
        {row.main}
        {row.side > 0 && <span className="text-faint"> +{row.side} side</span>}
      </td>
      <td className="px-2 py-2 text-right tabular-nums" title="Probabilità di averne almeno una nelle 5 carte che tieni nel primo turno avversario">
        {row.main > 0 ? pct(row.chance) : <span className="text-faint">—</span>}
      </td>
      <td className="px-2 py-2">
        <span className="flex items-center gap-2">
          <span className="h-1.5 w-20 shrink-0 overflow-hidden rounded-full bg-bg-2">
            <span className="block h-full rounded-full bg-sec" style={{ width: `${Math.round(row.lost * 100)}%` }} />
          </span>
          <span className={`tabular-nums ${none ? "" : "font-semibold text-text"}`}>{pct(row.lost)}</span>
        </span>
      </td>
      <td className="px-2 py-2 text-right tabular-nums text-muted" title="Linee del meta di cui cambia il campo o la mano finale, pesate per diffusione">
        {pct(row.value)}
      </td>
      {table.decks.map((d, i) => {
        const a = row.perDeck[i];
        return (
          <td key={d.name} className="px-1.5 py-2 text-center text-xs">
            <Share
              share={a?.lost ?? 0}
              usable={(a?.usable ?? 0) > 0}
              title={
                a && a.usable > 0
                  ? `${d.name}: toglie in media il ${pct(a.lost)} della board finale; cambia ${a.stopped} linee su ${d.measured} (usabile in ${a.usable})`
                  : `${d.name}: il motore non la lascia usare in nessuna linea`
              }
            />
          </td>
        );
      })}
      <td className="py-2 pr-4 pl-2 text-right tabular-nums" title="Mosse della linea già giocate quando la ferma, in media">
        {row.step == null ? <span className="text-faint">—</span> : row.step.toLocaleString("it-IT", { maximumFractionDigits: 1 })}
      </td>
    </tr>
  );
}

/** Contro il meta: which of the deck's handtraps stop the most real lines of the decks played the most. */
export function TechTab({ deck, getCard, request }: Props) {
  const [table, setTable] = useState<TechTable | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const reqId = useRef(0);
  // A new deck (the tab is keyed by it) never shows another's table: an answer after unmounting is dropped.
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const run = async () => {
    const id = ++reqId.current;
    sound.play("select");
    setLoading(true);
    setError(null);
    setTable(null);
    try {
      const t = await api.techTable(deck.id!);
      if (!alive.current || reqId.current !== id || t.deckId !== deck.id) return;
      setTable(t);
      request([...new Set(t.rows.flatMap((r) => [r.code, ...r.at.map((a) => a.code)]))]);
    } catch (e) {
      if (alive.current && reqId.current === id) setError(`Tabella non calcolata: ${e}`);
    } finally {
      if (alive.current && reqId.current === id) setLoading(false);
    }
  };

  const left = table?.decks.filter((d) => d.measured < d.lines) ?? [];

  return (
    <div className="flex flex-col gap-4">
      <div className="lab-card flex flex-wrap items-center gap-3 px-4 py-3">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold">Le tue handtrap contro il meta</p>
          <p className="text-xs text-muted">
            Il motore rigioca le linee migliori dei deck più giocati su Master Duel Meta con ogni handtrap del tuo Main e Side in mano, usata al
            momento migliore: misura quanto del campo e della mano finali toglie, pesando ogni deck per quanto è giocato.
          </p>
        </div>
        <button type="button" className="btn btn-molten shrink-0" disabled={loading || deck.id == null} onClick={() => void run()}>
          {loading ? "Calcolo…" : table ? "Ricalcola" : "Calcola"}
        </button>
      </div>

      {error && <p className="lab-card border-danger px-4 py-3 text-sm text-danger">{error}</p>}

      {loading && (
        <p className="text-sm text-muted">
          Leggo le linee del meta e le rigioco con le tue handtrap: la prima volta in una sessione servono alcuni minuti, poi circa mezzo minuto.
        </p>
      )}

      {!table && !loading && !error && (
        <p className="text-sm text-muted">Premi «Calcola» per vedere quali handtrap tolgono di più alle linee vere del meta.</p>
      )}

      {table && table.rows.length === 0 && (
        <p className="lab-card px-4 py-3 text-sm text-muted">
          Nel Main e nel Side di {deck.name} il motore non trova handtrap: carte che si usano dalla mano nel turno dell'avversario.
        </p>
      )}

      {table && table.rows.length > 0 && (
        <section className="lab-card overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[11px] tracking-[0.12em] text-faint uppercase">
                <th className="py-2.5 pr-3 pl-4 font-semibold">Carta</th>
                <th className="px-2 py-2.5 text-right font-semibold">Copie</th>
                <th className="px-2 py-2.5 text-right font-semibold">In mano</th>
                <th className="px-2 py-2.5 font-semibold" title="Quota del campo e della mano finali delle linee del meta che toglie, al suo momento migliore, pesata per diffusione">
                  Board tolta
                </th>
                <th className="px-2 py-2.5 text-right font-semibold" title="Linee di cui cambia il campo o la mano finale">
                  Linee
                </th>
                {table.decks.map((d) => (
                  <th key={d.name} className="max-w-24 px-1.5 py-2.5 text-center font-semibold normal-case tracking-normal" title={`${d.name}: ${d.weight.toLocaleString("it-IT")}% dei top deck, ${d.measured} linee contate su ${d.lines}`}>
                    <span className="block truncate text-[11px] text-muted">{d.name}</span>
                    <span className="block text-[10px] font-normal text-faint tabular-nums">{d.weight.toLocaleString("it-IT", { maximumFractionDigits: 1 })}%</span>
                  </th>
                ))}
                <th className="py-2.5 pr-4 pl-2 text-right font-semibold">Passo</th>
              </tr>
            </thead>
            <tbody>
              {table.rows.map((r) => (
                <Row key={r.code} row={r} table={table} getCard={getCard} />
              ))}
            </tbody>
          </table>
        </section>
      )}

      {table && (
        <p className="text-xs text-faint">
          «Board tolta»: la quota del campo e della mano con cui finiscono le linee che manca usando la carta al momento migliore. «Linee»: quelle di
          cui cambia la fine. «Passo»: le mosse della linea già giocate in quel momento, in media. Sotto il nome, la carta avversaria a cui risponde più
          spesso.
          {left.length > 0 &&
            ` Non contate, perché il rigioco senza handtrap non torna uguale: ${left.map((d) => `${d.name} ${d.lines - d.measured} su ${d.lines}`).join(", ")}.`}
        </p>
      )}
    </div>
  );
}
