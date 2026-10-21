import { useEffect, useState, type ReactNode } from "react";
import { api, type Card, ComboBreaker, ComboInterruption, ComboRivalBreaker, ComboRivalHandTrap, ComboScore, type LearnedWeights } from "../../lib/api";
import { METRICS, NEGATIVE, parts, pickProfile, PROFILES, score, setWeight, applyLearnedWeights, useWeights, type Metric, type MetricKey, type Weights } from "../../lib/scoring";
import { sound } from "../../lib/sound";
import { InfoButton } from "../InfoButton";
import { CardImage, type HoverCard } from "./Field";

const fmt = (n: number) => n.toLocaleString("it-IT", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
/** One decimal, two for a small amount that would read as zero. */
const num = (n: number) => (Math.abs(n) > 0.005 && Math.abs(n) < 0.1 ? fmt2(n) : fmt(n));
const fmt2 = (n: number) => n.toLocaleString("it-IT", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const signed = (n: number) => (n > 0.005 ? `+${fmt2(n)}` : n < -0.005 ? `−${fmt2(-n)}` : "0");
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
const cap = (t: string) => t.charAt(0).toUpperCase() + t.slice(1);

/** Where an effect is used from, as the probe turn names the place. */
const FROM: Record<string, string> = { mano: "dalla mano", campo: "dal campo", Cimitero: "dal Cimitero", bandita: "dalle carte bandite" };

/** A board breaker "holds" when at least this share of the field's interruptions is left. */
const HOLDS = 0.5;

/** The positive parts of a score as colored segments; `scale` is the best score shown. */
export function ScoreBar({ s, weights, scale }: { s: ComboScore; weights: Weights; scale: number }) {
  const shown = parts(s, weights).filter((p) => p.value > 0.005);
  const title = shown.map((p) => `${p.metric.label}: ${fmt2(p.value)}`).join(" · ");
  return (
    <span className="score-bar w-full" title={title} role="img" aria-label={`Punteggio ${fmt(score(s, weights))}: ${title}`}>
      {shown.map((p) => (
        <span key={p.metric.key} style={{ width: `${(100 * p.value) / Math.max(scale, 0.01)}%`, background: p.metric.color }} />
      ))}
    </span>
  );
}

function Paragraphs({ lines }: { lines: string[] }) {
  return (
    <>
      {lines.map((l, i) => (
        <p key={i}>{l}</p>
      ))}
    </>
  );
}

/** What the profiles are, for the "i" next to them. */
const PROFILES_INFO = [
  "Il punteggio somma le voci della board (interruzioni, floodgate, resilienza...), ognuna moltiplicata per un peso. Il profilo sceglie i pesi, cioè cosa conta di più per te.",
  "«Bilanciato» va bene quasi sempre. «Più negazioni» premia le board con tante negazioni, «Controllo e floodgate» quelle che bloccano l'avversario senza attivare nulla, «A prova di board breaker» quelle che restano in piedi dopo un Dark Hole, «Risorse e follow-up» quelle che ti lasciano carte in mano, «Linee sicure» le linee che le handtrap fermano meno.",
  "Cambiando profilo le board si riordinano subito. Con «Personalizza i pesi» scegli tu ogni peso.",
];

/** Profiles and one slider per metric; a change ranks the boards again at once. */
export function WeightsPanel({
  profile,
  weights,
  open,
  compact = false,
  deckId: _deckId,
  onToggle,
  onChoicesChanged,
}: {
  profile: string;
  weights: Weights;
  open: boolean;
  compact?: boolean;
  deckId?: number | null;
  onToggle: () => void;
  onChoicesChanged?: () => void;
}) {
  const [learned, setLearned] = useState<LearnedWeights | null>(null);
  const [loadingLearned, setLoadingLearned] = useState(false);
  const [clearing, setClearing] = useState(false);

  useEffect(() => {
    let active = true;
    api.learnedWeights(weights)
      .then((lw) => {
        if (active) setLearned(lw);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [profile, open, weights]);
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        <span className="text-muted">Classifica</span>
        <InfoButton title="Come si ordinano le board">
          <Paragraphs lines={PROFILES_INFO} />
        </InfoButton>
        {PROFILES.map((p) => (
          <button
            key={p.id}
            type="button"
            className={`chip !py-0.5 ${profile === p.id ? "chip-active" : ""}`}
            onClick={() => {
              sound.play("toggle");
              pickProfile(p.id);
            }}
          >
            {p.label}
          </button>
        ))}
        {profile === "custom" && <span className="chip chip-active !py-0.5">Personalizzato</span>}
        <button
          type="button"
          disabled={loadingLearned}
          className={`chip !py-0.5 ${profile === "learned" ? "chip-active" : ""}`}
          onClick={async () => {
            sound.play("toggle");
            setLoadingLearned(true);
            try {
              const res = await api.learnedWeights(weights);
              setLearned(res);
              applyLearnedWeights(res.weights as Weights);
            } finally {
              setLoadingLearned(false);
            }
          }}
          title="Pesi calibrati automaticamente sulle tue preferenze tra board registrate nel confronto"
        >
          ★ Imparato dalle tue scelte
          {learned != null && learned.choices > 0 && <span className="ml-1 opacity-80 font-semibold">({learned.choices})</span>}
        </button>
        <button type="button" className="ml-auto text-[11px] text-muted hover:text-text" onClick={onToggle} aria-expanded={open}>
          {open ? "Chiudi i pesi" : "Personalizza i pesi"}
        </button>
      </div>
      {open && (
        <div className={`lab-card grid ${compact ? "grid-cols-1 2xl:grid-cols-2" : "grid-cols-1 xl:grid-cols-2"} gap-x-5 gap-y-2 p-3 anim-fade`}>
          <p className={`${compact ? "2xl:col-span-2" : "xl:col-span-2"} text-[11px] text-muted`}>
            Più il peso è alto, più la voce conta. Le voci in rosso tolgono punti: più il peso è vicino a −1, più pesano contro la board.
          </p>
          {profile === "learned" && learned && (
            <div className={`${compact ? "2xl:col-span-2" : "xl:col-span-2"} rounded-lg border border-sec/40 bg-sec/10 p-2.5 text-xs anim-fade`}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex flex-col gap-0.5">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-text">
                      ★ Profilo calibrato sulle tue scelte ({learned.choices} {learned.choices === 1 ? "confronto" : "confronti"})
                    </span>
                    {learned.choices > 0 && (
                      <span className="chip !py-0 !px-1.5 text-[10px] text-sec font-medium">
                        Accordo: {Math.round(learned.agreement * 100)}%
                      </span>
                    )}
                  </div>
                  {learned.choices === 0 ? (
                    <p className="text-[11px] text-muted">
                      Nessun confronto registrato finora. Apri «Confronta» tra due board e premi «★ Preferisco questa» per insegnare le tue preferenze.
                    </p>
                  ) : (
                    <p className="text-[11px] text-muted">
                      I pesi imparati spiegano il <b>{Math.round(learned.agreement * 100)}%</b> delle tue scelte (il profilo di partenza ne spiegava il {Math.round(learned.priorAgreement * 100)}%).
                      {learned.choices < 4 && " Con poche scelte i pesi restano vicini al profilo di partenza per garantire stabilità."}
                    </p>
                  )}
                </div>
                {learned.choices > 0 && (
                  <button
                    type="button"
                    disabled={clearing}
                    onClick={async () => {
                      if (!window.confirm("Vuoi azzerare tutte le preferenze di board registrate?")) return;
                      setClearing(true);
                      try {
                        await api.clearBoardChoices();
                        sound.play("knock");
                        const cleared = await api.learnedWeights(PROFILES[0].weights);
                        setLearned(cleared);
                        pickProfile("balanced");
                        onChoicesChanged?.();
                      } finally {
                        setClearing(false);
                      }
                    }}
                    className="btn btn-ghost !py-1 !px-2 text-[11px] text-danger hover:bg-danger/10 border border-danger/30"
                    title="Azzera le scelte registrate e ripristina il profilo bilanciato"
                  >
                    🗑 Azzera scelte
                  </button>
                )}
              </div>
            </div>
          )}
          {METRICS.map((m) => (
            <div key={m.key} className="flex items-center gap-2 text-xs">
              <span className="h-2.5 w-2.5 shrink-0 rounded-sm" style={{ background: m.color }} />
              <span className={`w-24 shrink-0 ${NEGATIVE.includes(m.key) ? "text-danger" : "text-muted"}`}>{m.label}</span>
              <InfoButton title={m.label}>
                <Paragraphs lines={m.explain} />
              </InfoButton>
              <input
                type="range"
                min={NEGATIVE.includes(m.key) ? -1 : 0}
                max={NEGATIVE.includes(m.key) ? 0 : 2}
                step={NEGATIVE.includes(m.key) ? 0.01 : 0.05}
                value={weights[m.key]}
                onChange={(e) => setWeight(m.key, Number(e.target.value))}
                aria-label={`Peso di ${m.label}`}
                className="min-w-[80px] flex-1 accent-[var(--sec)]"
              />
              <span className="w-9 shrink-0 text-right tabular-nums">{weights[m.key].toFixed(2)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** What the probe turn measured for a metric, in words. */
function measured(key: MetricKey, s: ComboScore): string {
  switch (key) {
    case "interruptions":
      if (s.interruptions > 0.005) return `come ${num(s.interruptions)} negazioni piene`;
      return s.details.some((d) => d.place !== "mano") ? "nessuna che si possa usare con questa mano (vedi «Cosa ferma l'avversario»)" : "nessuna carta sul campo ferma l'avversario";
    case "handTraps":
      return s.handTraps > 0.005 ? `come ${num(s.handTraps)} negazioni piene` : "nessuna handtrap utile in mano";
    case "floodgates":
      return s.floodgateList.length > 0 ? `blocca ${plural(s.floodgateList.length, "tipo di giocata", "tipi di giocata")}` : "non blocca nulla da sola";
    case "resilience":
      return s.interruptions > 0.005 ? `in media restano ${num(s.resilience)} su ${num(s.interruptions)}` : "niente da proteggere";
    case "followup":
      return s.followupNote;
    case "resources":
      return s.resourcesNote;
    case "cost":
      return s.cost > 0 ? `usa ${plural(s.cost, "carta", "carte")} della mano oltre la prima` : "nessuna carta oltre la prima";
    case "exposure":
      return s.exposure > 0 ? plural(s.exposure, "punto in cui una handtrap la ferma", "punti in cui una handtrap la ferma") : "nessuna handtrap comune la ferma";
  }
}

/** A titled part of the breakdown, with its "i". */
function Section({ title, info, children }: { title: string; info: ReactNode; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1.5">
      <div className="flex items-center gap-1.5">
        <h4 className="eyebrow text-[10px] text-faint">{title}</h4>
        <InfoButton title={title}>{info}</InfoButton>
      </div>
      {children}
    </section>
  );
}

function MetricRow({ metric, s, weight, points, profile }: { metric: Metric; s: ComboScore; weight: number; points: number; profile: string }) {
  const zero = Math.abs(points) < 0.005;
  return (
    <li className={`grid grid-cols-[auto_1fr_auto] items-start gap-x-2 py-1 ${zero ? "opacity-60" : ""}`}>
      <span className="mt-[5px] h-2 w-2 rounded-sm" style={{ background: metric.color }} />
      <div className="min-w-0">
        <div className="flex items-center gap-1.5 text-xs font-medium">
          {metric.label}
          <InfoButton title={metric.label}>
            <Paragraphs lines={metric.explain} />
            <p className="border-t border-border pt-1.5 text-faint">
              {profile}, questa voce pesa {fmt2(weight)}: {fmt2(s[metric.key])} × {fmt2(weight)} = {signed(points)} punti.
            </p>
          </InfoButton>
        </div>
        <p className="text-[11px] leading-snug text-muted">{measured(metric.key, s)}</p>
      </div>
      <span className={`text-xs font-semibold tabular-nums ${points < -0.005 ? "text-danger" : ""}`}>{signed(points)}</span>
    </li>
  );
}

/** The board in a few plain sentences. */
function summary(s: ComboScore): string[] {
  const out: string[] = [];
  const field = s.details.filter((d) => d.place !== "mano");
  const used = field.filter((d) => d.value > 0.005);
  if (used.length === 1) {
    out.push(`Ferma l'avversario con un effetto, che vale come ${num(s.interruptions)} negazioni piene.`);
  } else if (used.length > 1) {
    out.push(`Ferma l'avversario con ${used.length} effetti: insieme valgono come ${num(s.interruptions)} negazioni piene.`);
  } else if (field.length > 0) {
    out.push("Sul campo c'è chi potrebbe fermare l'avversario, ma con le carte che hai in mano non può farlo (vedi sotto).");
  } else {
    out.push("Nessuna carta sul campo o nel Cimitero ferma le giocate dell'avversario.");
  }
  if (s.handTraps > 0.005) out.push(`Le handtrap che tieni in mano valgono come ${num(s.handTraps)} negazioni.`);
  if (s.floodgateList.length > 0) out.push(`Senza attivare nulla impedisce all'avversario di ${s.floodgateList.join(", ")}.`);
  const tried = s.breakers.filter((b) => !b.notApplicable);
  if (s.interruptions > 0.005 && tried.length > 0) {
    const held = tried.filter((b) => holds(b, s)).length;
    out.push(`Contro i board breaker più giocati regge ${held} volte su ${tried.length}.`);
  }
  if (s.exposureDetail || s.exposureNote) {
    out.push(
      s.exposure > 0
        ? `Mentre la giochi, le handtrap più comuni possono fermarla in ${plural(s.exposure, "punto", "punti")}.`
        : "Mentre la giochi, nessuna delle handtrap più comuni può fermarla.",
    );
  }
  return out;
}

function holds(b: ComboBreaker, s: ComboScore): boolean {
  return b.notApplicable || b.stoppedBy != null || b.remaining >= HOLDS * s.interruptions - 1e-9;
}

function Stopper({ d, getCard, onHover, onHoverEnd }: { d: ComboInterruption; getCard: (id: number) => Card | undefined; onHover?: HoverCard; onHoverEnd?: () => void }) {
  const card = getCard(d.code);
  const color = d.place === "mano" ? "var(--metric-hand)" : "var(--metric-interruptions)";
  const coverage = Math.round(d.coverage * 100);
  return (
    <li className="flex gap-2.5 rounded-lg bg-surface-2 p-2">
      <span className="shrink-0" onPointerEnter={(e) => onHover?.(d.code, e.currentTarget)} onPointerLeave={onHoverEnd}>
        <CardImage code={d.code} card={card} className="h-[52px] w-[36px]" />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex items-baseline gap-2">
          <b className="min-w-0 flex-1 truncate text-xs">{card?.name ?? d.code}</b>
          <span className="shrink-0 text-xs font-semibold tabular-nums" title="Quanto vale (1 = una negazione piena)">
            {fmt2(d.value)}
          </span>
        </div>
        <p className="text-[11px] leading-snug text-muted">
          {cap(d.effect)} {FROM[d.place] ?? d.place} · {d.targeted ? "con bersaglio" : "senza bersaglio"} · {d.uses === 1 ? "una volta" : `${d.uses} volte`}
        </p>
        <div className="flex items-center gap-2">
          <span className="meter flex-1" role="img" aria-label={`Copre il ${coverage}% delle giocate`}>
            <span style={{ width: `${coverage}%`, background: color }} />
          </span>
          <span className="shrink-0 text-[11px] tabular-nums text-muted">copre il {coverage}%</span>
        </div>
        <div className="flex flex-wrap gap-1">
          {d.covers.map((c) => (
            <span key={c} className="chip !px-1.5 !py-0 text-[10px] font-normal text-muted">
              {c}
            </span>
          ))}
        </div>
        {d.value < d.alone - 0.005 && (
          <p className="text-[11px] leading-snug text-warn">
            ⚠ Da sola varrebbe {fmt2(d.alone)}. Insieme alle altre vale meno: un'altra carta ferma prima la stessa giocata, oppure usano lo stesso costo o lo stesso limite di attivazioni.
          </p>
        )}
      </div>
    </li>
  );
}

function BreakerRow({ b, s, name }: { b: ComboBreaker; s: ComboScore; name: (code: number) => string }) {
  const kept = s.interruptions > 0 ? Math.max(0, Math.min(1, b.remaining / s.interruptions)) : 0;
  let status: string;
  if (b.notApplicable) status = "non si può usare contro questa board";
  else if (b.stoppedBy != null) status = `lo neghi con ${name(b.stoppedBy)} · restano ${num(b.remaining)}`;
  else if (kept >= 0.995) status = "non toglie nulla";
  else if (b.remaining < 0.005) status = "toglie tutto";
  else status = `restano ${num(b.remaining)}`;
  const ok = holds(b, s);
  return (
    <li className="grid grid-cols-[minmax(0,9rem)_1fr] items-center gap-x-3 gap-y-0.5 text-xs">
      <span className="truncate" title={b.name}>
        <span className={ok ? "text-ok" : "text-danger"} aria-hidden>
          {ok ? "✓ " : "✗ "}
        </span>
        {b.name}
      </span>
      <span className="flex min-w-0 items-center gap-2">
        <span className="meter w-16 shrink-0" aria-hidden>
          <span style={{ width: `${b.notApplicable ? 100 : Math.round(kept * 100)}%`, background: "var(--metric-resilience)" }} />
        </span>
        <span className={`min-w-0 truncate text-[11px] ${b.notApplicable ? "text-faint" : "text-muted"}`} title={status}>
          {status}
        </span>
      </span>
    </li>
  );
}

/** One handtrap and where it stops the line. */
function Risk({ name, text, counts }: { name: string; text: string; counts: boolean }) {
  return (
    <li className="flex items-baseline gap-2 text-xs">
      <span className={counts ? "text-danger" : "text-faint"} aria-hidden>
        {counts ? "●" : "○"}
      </span>
      <span className="w-32 xl:w-36 shrink-0 font-medium">{name}</span>
      <span className={`min-w-0 text-[11px] ${counts ? "text-muted" : "text-faint"}`}>{text}</span>
    </li>
  );
}


function RivalBreakerRow({
  b,
  s,
  getCard,
  onHover,
  onHoverEnd,
}: {
  b: ComboRivalBreaker;
  s: ComboScore;
  getCard: (code: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
}) {
  const card = getCard(b.code);
  const cardName = card?.name ?? String(b.code);
  const kept = s.interruptions > 0 ? Math.max(0, Math.min(1, b.remaining / s.interruptions)) : 0;
  let status: string;
  if (b.notApplicable) status = "non si può usare contro questa board";
  else if (b.stoppedBy != null) {
    const stopperName = getCard(b.stoppedBy)?.name ?? String(b.stoppedBy);
    status = `lo neghi con ${stopperName} · restano ${num(b.remaining)}`;
  } else if (kept >= 0.995) status = "non toglie nulla";
  else if (b.remaining < 0.005) status = "toglie tutto";
  else status = `restano ${num(b.remaining)}`;
  const ok = b.notApplicable || b.stoppedBy != null || b.remaining >= HOLDS * s.interruptions - 1e-9;
  const chancePct = Math.round(b.chance * 100);

  return (
    <li className="grid grid-cols-[minmax(0,11rem)_1fr] items-center gap-x-3 gap-y-0.5 text-xs py-0.5">
      <span
        className="flex items-center gap-1.5 min-w-0 cursor-default"
        onPointerEnter={(e) => onHover?.(b.code, e.currentTarget)}
        onPointerLeave={onHoverEnd}
      >
        <span className={ok ? "text-ok" : "text-danger"} aria-hidden>
          {ok ? "✓ " : "✗ "}
        </span>
        <span className="truncate font-medium text-text" title={cardName}>
          {cardName}
        </span>
        <span className="shrink-0 text-[10px] text-faint tabular-nums" title="Probabilità tra le 6 carte avversarie">
          ({chancePct}%)
        </span>
      </span>
      <span className="flex min-w-0 items-center gap-2">
        <span className="meter w-16 shrink-0" aria-hidden>
          <span style={{ width: `${b.notApplicable ? 100 : Math.round(kept * 100)}%`, background: "var(--metric-resilience)" }} />
        </span>
        <span className={`min-w-0 truncate text-[11px] ${b.notApplicable ? "text-faint" : "text-muted"}`} title={status}>
          {status}
        </span>
      </span>
    </li>
  );
}

function RivalHandTrapRow({
  h,
  getCard,
  onHover,
  onHoverEnd,
}: {
  h: ComboRivalHandTrap;
  getCard: (code: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
}) {
  const card = getCard(h.code);
  const cardName = card?.name ?? String(h.code);
  const stopsLine = h.stops > 0;
  const chancePct = Math.round(h.chance * 100);
  const targets = h.at.map((c) => getCard(c)?.name ?? String(c));

  return (
    <li className="flex flex-col gap-0.5 text-xs py-1.5 border-b border-border/40 last:border-0">
      <div className="flex items-baseline justify-between gap-2">
        <span
          className="flex items-center gap-1.5 min-w-0 cursor-default"
          onPointerEnter={(e) => onHover?.(h.code, e.currentTarget)}
          onPointerLeave={onHoverEnd}
        >
          <span className={stopsLine ? "text-danger" : "text-faint"} aria-hidden>
            {stopsLine ? "●" : "○"}
          </span>
          <b className="truncate text-text" title={cardName}>
            {cardName}
          </b>
          <span className="shrink-0 text-[10px] text-faint tabular-nums" title="Probabilità tra le 5 carte iniziali">
            ({chancePct}%)
          </span>
        </span>
        <span className={`shrink-0 text-[11px] font-medium tabular-nums ${stopsLine ? "text-warn" : "text-faint"}`}>
          {stopsLine ? `ferma in ${plural(h.stops, "momento", "momenti")} su ${h.windows}` : `usata in ${h.windows} momenti (0 fermate)`}
        </span>
      </div>
      {targets.length > 0 && (
        <p className="text-[11px] text-muted pl-3.5">
          Agisce su: <span className="text-text font-medium">{targets.join(", ")}</span>
        </p>
      )}
    </li>
  );
}

function Exposure({ s }: { s: ComboScore }) {
  const x = s.exposureDetail;
  if (!x) return <p className="text-xs text-muted">{s.exposureNote}</p>;
  return (
    <ul className="flex flex-col gap-1">
      <Risk name="Ash Blossom" counts={x.ash > 0} text={x.ash > 0 ? `${plural(x.ash, "effetto", "effetti")} che prendono carte dal Deck` : "nessun effetto da negare"} />
      <Risk
        name="Effect Veiler, Impermanence"
        counts={x.onField > 0}
        text={x.onField > 0 ? `${plural(x.onField, "effetto", "effetti")} di mostri attivati sul campo` : "nessun effetto di mostro sul campo"}
      />
      <Risk name="Nibiru" counts={x.summons >= 5} text={x.summons >= 5 ? `arriva: ${x.summons} Evocazioni (basta la quinta)` : `non arriva: ${x.summons} Evocazioni su 5`} />
      <Risk
        name="Droll & Lock Bird"
        counts={x.searches >= 2}
        text={x.searches >= 2 ? `ferma ${plural(x.searches - 1, "aggiunta", "aggiunte")} dal Deck dopo la prima` : x.searches === 1 ? "una sola carta aggiunta dal Deck: non ferma nulla" : "nessuna carta aggiunta dal Deck"}
      />
      <Risk
        name={'Maxx "C"'}
        counts={false}
        text={`${plural(x.specialSummons, "Evocazione Speciale", "Evocazioni Speciali")}: l'avversario pesca, ma non ti ferma (non toglie punti)`}
      />
    </ul>
  );
}

/** Why a board has its score: a summary, each metric, then what the probe turn saw. */
export function ScoreBreakdown({
  s,
  weights,
  getCard,
  onHover,
  onHoverEnd,
}: {
  s: ComboScore;
  weights: Weights;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
}) {
  const { profile } = useWeights();
  const name = (code: number) => getCard(code)?.name ?? String(code);
  if (s.error) {
    return <p className="text-xs text-faint">Il turno di prova non si è potuto giocare su questa board: {s.error}</p>;
  }
  const profileName = PROFILES.find((p) => p.id === profile)?.label;
  const profileText = profileName ? `Nel profilo «${profileName}»` : "Con i tuoi pesi";
  const all = parts(s, weights);
  const total = score(s, weights);
  const positive = all.filter((p) => p.value > 0.005).reduce((t, p) => t + p.value, 0);
  const gains = all.filter((p) => !NEGATIVE.includes(p.metric.key));
  const losses = all.filter((p) => NEGATIVE.includes(p.metric.key));
  const tried = s.breakers.filter((b) => !b.notApplicable);
  const held = tried.filter((b) => holds(b, s)).length;
  const metric = (key: MetricKey) => METRICS.find((m) => m.key === key)!;
  const row = (p: { metric: Metric; value: number }) => (
    <MetricRow key={p.metric.key} metric={p.metric} s={s} weight={weights[p.metric.key]} points={p.value} profile={profileText} />
  );

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1.5">
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-semibold tabular-nums">{fmt2(total)}</span>
          <span className="text-xs text-muted">punti{profileName ? ` · profilo «${profileName}»` : " · pesi personalizzati"}</span>
          <InfoButton title="Come si calcola il punteggio" className="ml-auto self-center">
            <p>Il motore mette alla prova la board: gioca il turno dell'avversario con le giocate più comuni e con i board breaker più usati, e guarda cosa riesci a fermare.</p>
            <p>Ogni voce qui sotto dà punti (misura × peso); quelle in rosso li tolgono. I pesi vengono dal profilo scelto sopra la lista delle board.</p>
            <p>Il punteggio serve a confrontare le board della stessa ricerca: non è un voto assoluto.</p>
          </InfoButton>
        </div>
        <ScoreBar s={s} weights={weights} scale={positive} />
      </div>

      <Section title="In breve" info={<p>La board riassunta in poche frasi. I dettagli e i numeri sono più sotto; ogni «i» spiega la sua voce.</p>}>
        <ul className="flex list-disc flex-col gap-1 pl-4 text-xs leading-relaxed marker:text-faint">
          {summary(s).map((t, i) => (
            <li key={i}>{t}</li>
          ))}
        </ul>
      </Section>

      <Section
        title="Da dove vengono i punti"
        info={
          <>
            <p>Ogni voce misura una cosa della board e la moltiplica per il suo peso. A destra i punti che dà; le voci in grigio non danno nulla a questa board.</p>
            <p>Tocca la «i» di una voce per sapere cosa misura e quanto pesa.</p>
          </>
        }
      >
        <ul className="flex flex-col">{gains.map(row)}</ul>
        <p className="mt-1 text-[10px] font-semibold tracking-[0.12em] text-faint uppercase">Tolgono punti</p>
        <ul className="flex flex-col">{losses.map(row)}</ul>
        <div className="flex items-baseline justify-between border-t border-border pt-1.5 text-xs font-semibold">
          <span>Totale</span>
          <span className="tabular-nums">{fmt2(total)}</span>
        </div>
      </Section>

      <Section
        title="Cosa ferma l'avversario"
        info={
          <>
            <p>Le carte che, nel turno di prova, hanno fermato almeno una giocata dell'avversario.</p>
            <p>«Copre»: quante delle giocate di prova riesce a fermare, pesate per quanto sono comuni. Un effetto che nega qualsiasi attivazione copre il 100%, uno che nega solo le Magie molto meno.</p>
            <p>Il numero a destra è quanto vale: 1 è una negazione piena senza bersaglio. Negare e togliere la carta vale più che distruggerla; con un bersaglio vale un po' meno; un effetto che si usa due volte vale il doppio.</p>
            <p>Le carte si provano anche tutte insieme, la più forte per prima: se un'altra ferma già la stessa giocata, o se finisce il costo da pagare, una carta vale meno di quanto varrebbe da sola (è segnato in giallo).</p>
          </>
        }
      >
        {s.details.length === 0 ? (
          <p className="text-xs text-faint">Nessun effetto ferma le giocate di prova dell'avversario.</p>
        ) : (
          <ul className="flex flex-col gap-1.5">
            {s.details.map((d, i) => (
              <Stopper key={i} d={d} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
            ))}
          </ul>
        )}
      </Section>

      {s.floodgateList.length > 0 && (
        <Section title="Cosa l'avversario non può fare" info={<Paragraphs lines={metric("floodgates").explain} />}>
          <div className="flex flex-wrap gap-1">
            {s.floodgateList.map((f) => (
              <span key={f} className="chip !py-0.5 text-[11px] font-normal">
                {cap(f)}
              </span>
            ))}
          </div>
        </Section>
      )}

      <Section
        title={s.matchup ? `Contro i board breaker · «${s.matchup.deckName}»` : "Contro i board breaker"}
        info={
          <>
            <Paragraphs lines={metric("resilience").explain} />
            <p>«Regge» vuol dire che resta almeno metà delle interruzioni dal campo, o che lo neghi. «Non si può usare»: con questa board l'avversario non può attivarlo, quindi non fa danni.</p>
            {s.matchup && (
              <p>Con un deck avversario selezionato, si misurano i breaker specifici presenti nel suo Main Deck contro la tua board.</p>
            )}
          </>
        }
      >
        {s.interruptions < 0.005 ? (
          <p className="text-xs text-faint">Non ci sono interruzioni sul campo da proteggere.</p>
        ) : (
          <div className="flex flex-col gap-2.5">
            {s.matchup && (
              s.matchup.error ? (
                <div className="rounded-lg border border-danger/40 bg-danger/10 p-2.5 text-xs text-danger">
                  Misura contro «{s.matchup.deckName}» non riuscita: {s.matchup.error}
                </div>
              ) : (
                <div className="rounded-lg border border-border/80 bg-surface-2/60 p-2.5 flex flex-col gap-2">
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="text-xs font-semibold text-text">
                      Resilienza contro «{s.matchup.deckName}»
                    </span>
                    <span className="text-xs font-bold tabular-nums text-sec">
                      in media restano {num(s.matchup.resilience)} su {num(s.interruptions)}
                    </span>
                  </div>
                  {s.matchup.standardBreakers ? (
                    <p className="text-[11px] text-muted">
                      Questo deck non ha board breaker giocabili dalla mano: valgono i breaker standard del kit.
                    </p>
                  ) : (
                    <>
                      <p className="text-[11px] text-muted">
                        Probabilità che l'avversario abbia almeno un breaker giocabile tra 6 carte:{" "}
                        <b className="text-text tabular-nums">{Math.round(s.matchup.breakerChance * 100)}%</b>.
                      </p>
                      {s.matchup.breakers.length > 0 && (
                        <ul className="flex flex-col gap-0.5">
                          {s.matchup.breakers.map((b) => (
                            <RivalBreakerRow key={b.code} b={b} s={s} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                          ))}
                        </ul>
                      )}
                    </>
                  )}
                </div>
              )
            )}

            <details className="group" open={!s.matchup}>
              <summary className="cursor-pointer text-xs select-none">
                {s.matchup ? "Breaker del kit standard: regge a" : "Regge a"} <b className="tabular-nums">{held}</b> su <b className="tabular-nums">{tried.length}</b> board breaker
                <span className="ml-1.5 text-[11px] text-muted group-open:hidden">· mostra quali</span>
                <span className="ml-1.5 hidden text-[11px] text-muted group-open:inline">· nascondi</span>
              </summary>
              <ul className="mt-2 flex flex-col gap-1">
                {s.breakers.map((b) => (
                  <BreakerRow key={b.name} b={b} s={s} name={name} />
                ))}
              </ul>
            </details>
          </div>
        )}
      </Section>

      {(s.exposureDetail || s.exposureNote || s.matchup) && (
        <Section
          title={s.matchup ? `Dove si può fermare la linea · «${s.matchup.deckName}»` : "Dove si può fermare la linea"}
          info={<Paragraphs lines={metric("exposure").explain} />}
        >
          <div className="flex flex-col gap-2.5">
            {s.matchup && (
              <div className="rounded-lg border border-border/80 bg-surface-2/60 p-2.5 flex flex-col gap-2">
                <div className="flex items-baseline justify-between gap-2">
                  <span className="text-xs font-semibold text-text">
                    Contro «{s.matchup.deckName}»
                  </span>
                  <span className="text-xs font-bold tabular-nums text-warn">
                    {Math.round(s.matchup.handTrapChance * 100)}% probabilità di fermata
                  </span>
                </div>
                <p className="text-[11px] text-muted">
                  Probabilità che l'avversario peschi tra le sue 5 carte iniziali almeno una handtrap che altera o blocca la linea.
                </p>
                {s.matchup.handTraps.filter((h) => h.stops > 0).length > 0 ? (
                  <ul className="flex flex-col">
                    {s.matchup.handTraps
                      .filter((h) => h.stops > 0)
                      .map((h) => (
                        <RivalHandTrapRow key={h.code} h={h} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                      ))}
                  </ul>
                ) : (
                  <p className="text-xs text-ok">Nessuna handtrap del deck avversario è in grado di fermare questa linea.</p>
                )}

                {s.matchup.handTraps.some((h) => h.stops === 0) && (
                  <div className="mt-1 border-t border-border/50 pt-2 flex flex-col gap-1">
                    <p className="text-[11px] text-faint">Si usano ma non fermano la linea (fanno solo pescare o non alterano il risultato):</p>
                    <div className="flex flex-wrap gap-1.5">
                      {s.matchup.handTraps
                        .filter((h) => h.stops === 0)
                        .map((h) => {
                          const cName = getCard(h.code)?.name ?? String(h.code);
                          return (
                            <span
                              key={h.code}
                              className="chip !py-0.5 !px-2 text-[10px] text-muted"
                              title={`${cName}: usata in ${h.windows} momenti ma la board finale rimane identica`}
                              onPointerEnter={(e) => onHover?.(h.code, e.currentTarget)}
                              onPointerLeave={onHoverEnd}
                            >
                              {cName} ({Math.round(h.chance * 100)}%)
                            </span>
                          );
                        })}
                    </div>
                  </div>
                )}
              </div>
            )}

            <details className="group" open={!s.matchup}>
              <summary className="cursor-pointer text-xs select-none">
                Handtrap più comuni (kit generico)
                <span className="ml-1.5 text-[11px] text-muted group-open:hidden">· mostra</span>
                <span className="ml-1.5 hidden text-[11px] text-muted group-open:inline">· nascondi</span>
              </summary>
              <div className="mt-2">
                <Exposure s={s} />
              </div>
            </details>
          </div>
        </Section>
      )}
    </div>
  );
}
