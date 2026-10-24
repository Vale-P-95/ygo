import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, FlaskConical, RotateCcw, Waypoints } from "lucide-react";
import { api, cardImage, type Card, type Deck, type LabTarget, type SpecialFilter, type SpecialKnown, type SpecialMechanism, type SpecialReport, type SpecialResult } from "../../lib/api";
import { roleDefs } from "../../lib/roles";
import { sound } from "../../lib/sound";
import { pct } from "../../lib/tooltip";
import { FlaskMeter } from "../charts";

interface Props {
  deck: Deck;
  roles: Record<string, number[]>;
  hand: "first" | "second";
  getCard: (id: number) => Card | undefined;
  request: (ids: number[]) => void;
}

const KIND: Record<string, string> = { monster: "mostro", spell: "Magia", trap: "Trappola" };

function filterText(f: SpecialFilter): string {
  const parts = [KIND[f.kind ?? ""] ?? "carta", f.attribute, f.race].filter(Boolean);
  return `${parts.join(" ")}${f.maxLevel != null ? ` di Livello ${f.maxLevel} o inferiore` : ""}`;
}

/** What the card does, in the terms the calculation uses. */
function mechanismText(m: SpecialMechanism): string {
  switch (m.type) {
    case "draw":
      return `${m.banishTop ? `Bandisce coperte le prime ${m.banishTop} carte del Deck, poi pesca ${m.n}` : `Pesca ${m.n}`}${m.minExtra ? `; costo: almeno ${m.minExtra} carte nell'Extra Deck` : ""}.`;
    case "excavate":
      return `Scava ${m.n.join(" o ")} carte e ne aggiunge 1${m.banishExtra ? "; bandisce dall'Extra Deck tante carte quante ne scava" : ""}.`;
    case "drawThenBanish":
      return `Pesca ${m.n}, poi bandisce un ${filterText(m.cost)} dalla mano; se non ne hai, perdi tutta la mano.`;
    case "bridge":
      return `Ponte: un mostro in mano, uno nel Deck con esattamente ${m.share} proprietà in comune (Tipo, Attributo, Livello, ATK, DEF) e il mostro da aggiungere, con ${m.share} in comune con quello.`;
    case "search":
      return `Aggiunge dal Deck un ${filterText(m.filter)}.`;
    case "revealSearch":
      return `Rivela un mostro dell'Extra Deck (${names(m.reveal)}) e aggiunge un mostro con Livello pari al suo Rango e lo stesso Tipo o Attributo${m.putBack ? `; poi rimette ${m.putBack} ${m.putBack === 1 ? "carta" : "carte"} dalla mano in cima al Deck` : ""}.`;
  }
}

const points = (d: number) => (Math.abs(d) < 0.0005 ? "nessun aiuto" : `${d > 0 ? "+" : "−"}${Math.abs(d * 100).toLocaleString("it-IT", { maximumFractionDigits: 1 })} punti`);

/** "Number 101, …, Number 107" for a long list of names. */
const names = (list: string[]) => {
  const short = list.map((r) => r.replace(/:$/, ""));
  return short.length > 3 ? `${short[0]}, …, ${short[short.length - 1]}` : short.join(", ");
};

/** Laboratorio, "Carte speciali": draw spells, excavators, bridges and searches against one target. */
export function SpecialTab({ deck, roles, hand, getCard, request }: Props) {
  const handSize = hand === "first" ? 5 : 6;
  const ids = useMemo(() => [...new Set(deck.main)].sort((a, b) => (getCard(a)?.name ?? "").localeCompare(getCard(b)?.name ?? "")), [deck.main, getCard]);
  const copiesIn = (key: string) => deck.main.filter((id) => roles[key]?.includes(id)).length;
  const defs = roleDefs(roles).filter((d) => copiesIn(d.key) > 0);
  const [target, setTarget] = useState<LabTarget>(() =>
    deck.main.some((id) => roles.starter?.includes(id)) ? { kind: "role", value: "starter" } : { kind: "card", value: deck.main[0] ?? 0 },
  );
  // Copies of known cards added only to the calculation, to try one the deck lacks.
  const [trial, setTrial] = useState<Record<number, number>>({});
  const [known, setKnown] = useState<SpecialKnown[] | null>(null);
  const [knownError, setKnownError] = useState<string | null>(null);
  const [report, setReport] = useState<SpecialReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const reqId = useRef(0);

  useEffect(() => {
    let stale = false;
    api
      .labSpecialKnown()
      .then((k) => {
        if (stale) return;
        setKnown(k);
        request(k.map((c) => c.id));
      })
      .catch((e) => !stale && setKnownError(String(e)));
    return () => {
      stale = true;
    };
  }, [request]);

  const main = useMemo(() => [...deck.main, ...Object.entries(trial).flatMap(([id, n]) => Array<number>(n).fill(Number(id)))], [deck.main, trial]);
  const targetKey = JSON.stringify(target);
  useEffect(() => {
    const id = ++reqId.current;
    setLoading(true);
    setError(null);
    api
      .labSpecial(main, deck.extra, roles, target, handSize)
      .then((r) => {
        if (reqId.current !== id) return;
        setReport(r);
        request([...new Set(r.cards.flatMap((c) => [...c.links, ...c.chains.flatMap((ch) => [ch.from, ch.via, ch.to])]))]);
      })
      .catch((e) => {
        if (reqId.current !== id) return;
        setReport(null);
        setError(String(e));
      })
      .finally(() => reqId.current === id && setLoading(false));
    // `targetKey` stands for the target.
  }, [main, deck.extra, roles, targetKey, handSize, request]); // eslint-disable-line react-hooks/exhaustive-deps

  const hits = (id: number) => (target.kind === "card" ? id === target.value : (roles[target.value]?.includes(id) ?? false));
  const name = (id: number) => getCard(id)?.name ?? `#${id}`;
  const targetName = target.kind === "card" ? name(target.value) : (defs.find((d) => d.key === target.value)?.label ?? target.value);
  const cards = [...(report?.cards ?? [])].sort((a, b) => b.with - a.with);
  const bridge = cards.find((c) => c.mechanism.type === "bridge");
  const trialCount = Object.values(trial).reduce((a, b) => a + b, 0);
  const missing = (known ?? []).filter((k) => !deck.main.includes(k.id));

  const pickTarget = (v: string) => {
    sound.play("select");
    setTarget(v.startsWith("card:") ? { kind: "card", value: Number(v.slice(5)) } : { kind: "role", value: v.slice(5) });
  };

  return (
    <div className="flex flex-col gap-4">
      <section className="lab-card flex flex-wrap items-center gap-x-6 gap-y-3 px-4 py-3">
        <label className="flex min-w-0 flex-1 items-center gap-3 text-sm">
          <span className="shrink-0 font-semibold">Che cosa vuoi in mano?</span>
          <select className="field min-w-0 flex-1 !py-1 text-sm" value={`${target.kind}:${target.value}`} onChange={(e) => pickTarget(e.target.value)} aria-label="Bersaglio">
            {defs.length > 0 && (
              <optgroup label="Ruoli">
                {defs.map((d) => (
                  <option key={d.key} value={`role:${d.key}`}>
                    {d.label} ({copiesIn(d.key)} {copiesIn(d.key) === 1 ? "carta" : "carte"})
                  </option>
                ))}
              </optgroup>
            )}
            <optgroup label="Carte">
              {ids.map((id) => (
                <option key={id} value={`card:${id}`}>
                  {name(id)}
                </option>
              ))}
            </optgroup>
          </select>
        </label>
        {report && (
          <p className="text-sm text-muted tabular-nums">
            Senza aiuti <b className="text-text">{pct(report.base)}</b>
            <span className="text-faint">
              {" "}
              · {report.targetCopies} {report.targetCopies === 1 ? "copia" : "copie"} su {report.size}, {handSize} carte
            </span>
          </p>
        )}
      </section>

      {defs.length === 0 && (
        <p className="lab-card px-4 py-2.5 text-sm text-muted">
          Nessun ruolo segnato: scegli la carta che vuoi in mano, oppure segna gli <b className="text-text">Starter</b> nel pannello Ruoli per calcolare su tutti insieme.
        </p>
      )}

      {trialCount > 0 && (
        <p className="lab-card flex items-center gap-3 px-4 py-2.5 text-sm text-muted">
          <FlaskConical size={15} className="shrink-0 text-sec" />
          <span className="flex-1">
            In prova: {Object.entries(trial).map(([id, n]) => `${n}× ${name(Number(id))}`).join(", ")}, aggiunte al Main Deck solo per il calcolo ({main.length} carte).
          </span>
          <button
            type="button"
            className="btn btn-ghost !px-2 !py-1 text-xs"
            onClick={() => {
              sound.play("toggle");
              setTrial({});
            }}
          >
            <RotateCcw size={13} /> Togli le prove
          </button>
        </p>
      )}

      {error ? (
        <p className="lab-card border-danger/40 px-4 py-3 text-sm text-danger">Calcolo non riuscito: {error}</p>
      ) : !report ? (
        <p className="text-sm text-muted">{loading ? "Calcolo…" : ""}</p>
      ) : cards.length === 0 ? (
        <section className="lab-card flex flex-col items-center gap-2 px-6 py-8 text-center">
          <p className="display text-lg">Nessuna carta speciale nel deck</p>
          <p className="max-w-lg text-sm text-muted">
            Qui si calcolano le carte che cambiano le probabilità: pescate, scavi, ponti come Small World e ricerche. Provane una qui sotto: entra solo nel calcolo, non nel deck.
          </p>
        </section>
      ) : (
        <div className={`flex flex-col gap-3 ${loading ? "opacity-70" : ""}`}>
          {cards.map((c) => (
            <SpecialRow key={c.id} result={c} base={report.base} targetName={targetName} isTarget={hits(c.id)} trial={trial[c.id] ?? 0} name={name} />
          ))}
        </div>
      )}

      {bridge && report && <BridgeMap result={bridge} hits={hits} name={name} targetName={targetName} />}

      <section className="lab-card flex flex-col gap-2 px-4 py-3">
        <div className="flex items-baseline justify-between gap-3">
          <h3 className="text-sm font-semibold">Prova una carta</h3>
          <p className="text-xs text-faint">Una copia in più nel calcolo, fino a 3; il deck non cambia.</p>
        </div>
        {knownError ? (
          <p className="text-sm text-danger">Elenco delle carte non letto: {knownError}</p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {missing.map((k) => {
              const n = trial[k.id] ?? 0;
              return (
                <button
                  key={k.id}
                  type="button"
                  title={mechanismText(k.mechanism)}
                  disabled={n >= 3}
                  onClick={() => {
                    sound.play("select");
                    setTrial((t) => ({ ...t, [k.id]: Math.min(3, (t[k.id] ?? 0) + 1) }));
                  }}
                  className={`chip flex items-center gap-1.5 !py-1 text-xs ${n > 0 ? "border-sec text-text" : "text-muted hover:text-text"}`}
                >
                  <img src={cardImage(k.id, "small")} alt="" loading="lazy" className="h-5 w-[14px] rounded-[1px] object-cover" />
                  {k.name}
                  {n > 0 && <b className="text-sec tabular-nums">×{n}</b>}
                </button>
              );
            })}
            {known && missing.length === 0 && <p className="text-sm text-muted">Il deck ha già tutte le carte che il laboratorio conosce.</p>}
          </div>
        )}
      </section>
    </div>
  );
}

function SpecialRow({ result: c, base, targetName, isTarget, trial, name }: { result: SpecialResult; base: number; targetName: string; isTarget: boolean; trial: number; name: (id: number) => string }) {
  const delta = c.with - base;
  const facts: { label: string; value: string; warn?: boolean }[] = [{ label: "Apri la carta", value: pct(c.open) }];
  if (c.variants.length > 1) for (const v of c.variants) facts.push({ label: `Scavando ${v.n}`, value: pct(v.with) });
  if (c.banishRisk != null) facts.push({ label: `Senza ${targetName} in mano, ne bandisce almeno una copia`, value: pct(c.banishRisk), warn: true });
  if (c.loseHand != null) facts.push({ label: "Con la carta in mano, perdi la mano", value: pct(c.loseHand), warn: true });
  if (c.payers != null) facts.push({ label: "Carte che pagano il costo", value: String(c.payers) });
  const found = c.mechanism.type === "search" || c.mechanism.type === "revealSearch" ? c.links : [];

  return (
    <section className="lab-card flex gap-4 p-4 anim-rise">
      <img src={cardImage(c.id, "small")} alt="" loading="lazy" className="h-[70px] w-12 shrink-0 rounded-[3px] object-cover shadow" />
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="flex items-start gap-4">
          <div className="min-w-0 flex-1">
            <h3 className="truncate font-semibold">
              {name(c.id)} <span className="text-sm font-normal text-faint">×{c.copies}</span>
              {trial > 0 && <span className="ml-2 rounded bg-sec/15 px-1.5 py-0.5 text-[11px] font-semibold text-sec">in prova</span>}
            </h3>
            <p className="text-xs text-muted">{mechanismText(c.mechanism)}</p>
          </div>
          <div className="flex shrink-0 items-center gap-2.5" title={`${targetName} in mano, oppure raggiunto con ${name(c.id)}`}>
            <FlaskMeter value={c.with} size={38} />
            <div className="text-right">
              <p className="text-xl leading-tight font-semibold tabular-nums">{pct(c.with)}</p>
              <p className={`text-xs tabular-nums ${c.blocked ? "text-faint" : delta > 0.0005 ? "text-ok" : "text-faint"}`}>{c.blocked ? "non si attiva" : isTarget ? "è già il bersaglio" : points(delta)}</p>
            </div>
          </div>
        </div>
        {c.blocked && <p className="text-xs text-warn">Non si attiva: {c.blocked}.</p>}
        <dl className="flex flex-wrap gap-x-5 gap-y-1 text-xs">
          {facts.map((f) => (
            <div key={f.label} className="flex gap-1.5">
              <dt className="text-muted">{f.label}</dt>
              <dd className={`font-semibold tabular-nums ${f.warn ? "text-[var(--role-handtrap)]" : "text-text"}`}>{f.value}</dd>
            </div>
          ))}
        </dl>
        {(c.mechanism.type === "search" || c.mechanism.type === "revealSearch") && (
          <p className="text-xs text-muted">
            {found.length === 0 ? (
              "Nel deck non c'è niente da aggiungere."
            ) : (
              <>
                Aggiunge: <span className="text-text">{found.map(name).join(", ")}</span>
              </>
            )}
          </p>
        )}
      </div>
    </section>
  );
}

/** The bridge paths of the deck: those to the target first, then from any monster. */
function BridgeMap({ result, hits, name, targetName }: { result: SpecialResult; hits: (id: number) => boolean; name: (id: number) => string; targetName: string }) {
  const starts = [...new Set(result.chains.map((c) => c.from))].sort((a, b) => name(a).localeCompare(name(b)));
  const [from, setFrom] = useState<number | null>(null);
  const toTarget = result.chains.filter((c) => result.links.includes(c.from) && hits(c.to));
  const shown = from == null ? toTarget : result.chains.filter((c) => c.from === from);
  // One row per start and arrival, with every monster that bridges them.
  const rows = new Map<string, { from: number; to: number; via: number[] }>();
  for (const c of shown) {
    const key = `${c.from}-${c.to}`;
    const row = rows.get(key) ?? { from: c.from, to: c.to, via: [] };
    if (!row.via.includes(c.via)) row.via.push(c.via);
    rows.set(key, row);
  }
  const list = [...rows.values()].sort((a, b) => name(a.from).localeCompare(name(b.from)) || name(a.to).localeCompare(name(b.to)));

  return (
    <section className="lab-card flex flex-col gap-3 p-4">
      <div className="flex flex-wrap items-center gap-3">
        <Waypoints size={16} className="text-sec" />
        <h3 className="text-sm font-semibold">Ponti di {name(result.id)}</h3>
        <select
          className="field ml-auto !w-auto min-w-0 !py-1 text-sm"
          value={from ?? ""}
          onChange={(e) => {
            sound.play("select");
            setFrom(e.target.value === "" ? null : Number(e.target.value));
          }}
          aria-label="Mostro di partenza"
        >
          <option value="">Verso {targetName}</option>
          {starts.map((id) => (
            <option key={id} value={id}>
              Da {name(id)}
            </option>
          ))}
        </select>
      </div>
      <p className="text-xs text-muted">
        {from == null
          ? `Mostri che in mano portano a ${targetName}: ${result.links.length}. Il mostro in mezzo viene bandito dal Deck, quindi deve esserci ancora.`
          : `Dove arriva ${name(from)}: ${list.length} ${list.length === 1 ? "mostro" : "mostri"}.`}
      </p>
      {list.length === 0 ? (
        <p className="text-sm text-faint">{from == null ? `Nessun mostro del deck arriva a ${targetName} con un ponte.` : "Nessun ponte da questo mostro."}</p>
      ) : (
        <ul className="flex max-h-80 flex-col gap-1 overflow-y-auto pr-1">
          {list.map((r) => (
            <li key={`${r.from}-${r.to}`} className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1.3fr)_auto_minmax(0,1fr)] items-center gap-2 rounded-lg bg-bg-2 px-2.5 py-1.5 text-xs">
              <ChainCard id={r.from} name={name} />
              <ArrowRight size={13} className="text-faint" />
              <span className="truncate text-muted" title={r.via.map(name).join(", ")}>
                {r.via.length === 1 ? name(r.via[0]) : `${name(r.via[0])} e altri ${r.via.length - 1}`}
              </span>
              <ArrowRight size={13} className="text-faint" />
              <ChainCard id={r.to} name={name} strong={hits(r.to)} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function ChainCard({ id, name, strong }: { id: number; name: (id: number) => string; strong?: boolean }) {
  return (
    <span className={`flex min-w-0 items-center gap-1.5 ${strong ? "font-semibold text-text" : "text-text"}`}>
      <img src={cardImage(id, "small")} alt="" loading="lazy" className="h-6 w-[17px] shrink-0 rounded-[1px] object-cover" />
      <span className="truncate">{name(id)}</span>
    </span>
  );
}
