import type { Card, ComboScore } from "../../lib/api";
import { METRICS, parts, pickProfile, PROFILES, score, setWeight, type Weights } from "../../lib/scoring";
import { sound } from "../../lib/sound";

const fmt = (n: number) => n.toLocaleString("it-IT", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const fmt2 = (n: number) => n.toLocaleString("it-IT", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

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

/** Profiles and one slider per metric; a change ranks the boards again at once. */
export function WeightsPanel({ profile, weights, open, onToggle }: { profile: string; weights: Weights; open: boolean; onToggle: () => void }) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        <span className="text-muted">Classifica</span>
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
        <button type="button" className="ml-auto text-[11px] text-muted hover:text-text" onClick={onToggle} aria-expanded={open}>
          {open ? "Chiudi i pesi" : "Personalizza i pesi"}
        </button>
      </div>
      {open && (
        <div className="lab-card grid grid-cols-2 gap-x-5 gap-y-2 p-3 anim-fade">
          {METRICS.map((m) => (
            <label key={m.key} className="flex items-center gap-2 text-xs" title={m.hint}>
              <span className="h-2.5 w-2.5 shrink-0 rounded-sm" style={{ background: m.color }} />
              <span className="w-28 shrink-0 text-muted">{m.label}</span>
              <input
                type="range"
                min={m.key === "cost" ? -1 : 0}
                max={m.key === "cost" ? 0 : 2}
                step={0.05}
                value={weights[m.key]}
                onChange={(e) => setWeight(m.key, Number(e.target.value))}
                className="min-w-0 flex-1 accent-[var(--sec)]"
              />
              <span className="w-9 text-right tabular-nums">{weights[m.key].toFixed(2)}</span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
}

/** Why a board has its score: each metric, then what the probe turn saw. */
export function ScoreBreakdown({ s, weights, getCard }: { s: ComboScore; weights: Weights; getCard: (id: number) => Card | undefined }) {
  const name = (code: number) => getCard(code)?.name ?? String(code);
  if (s.error) {
    return <p className="text-xs text-faint">Il turno di prova non si è potuto giocare su questa board: {s.error}</p>;
  }
  return (
    <div className="flex flex-col gap-3">
      <table className="w-full text-xs tabular-nums">
        <thead className="sr-only">
          <tr>
            <th>Metrica</th>
            <th>Misura</th>
            <th>Peso</th>
            <th>Punti</th>
          </tr>
        </thead>
        <tbody>
          {parts(s, weights).map(({ metric, value }) => (
            <tr key={metric.key} title={metric.hint}>
              <td className="py-0.5">
                <span className="mr-1.5 inline-block h-2 w-2 rounded-sm align-middle" style={{ background: metric.color }} />
                {metric.label}
              </td>
              <td className="text-right text-muted">{fmt2(s[metric.key])}</td>
              <td className="text-right text-faint">× {weights[metric.key].toFixed(2)}</td>
              <td className="w-14 text-right font-semibold">{fmt2(value)}</td>
            </tr>
          ))}
          <tr className="border-t border-border">
            <td className="pt-1 font-semibold" colSpan={3}>
              Totale
            </td>
            <td className="pt-1 text-right font-semibold">{fmt2(score(s, weights))}</td>
          </tr>
        </tbody>
      </table>

      <div className="flex flex-col gap-1.5">
        <p className="eyebrow text-[10px] text-faint">Cosa ferma</p>
        {s.details.length === 0 ? (
          <p className="text-xs text-faint">Nessun effetto ferma le azioni di prova dell'avversario.</p>
        ) : (
          s.details.map((d, i) => (
            <p key={i} className="text-xs leading-relaxed">
              <b>{name(d.code)}</b> <span className="text-faint">({d.place})</span>: {d.effect}
              {d.targeted ? ", con bersaglio" : ", senza bersaglio"}
              {d.uses > 1 ? `, ${d.uses} volte` : ""} · copre il {Math.round(d.coverage * 100)}% · <b className="tabular-nums">{fmt2(d.value)}</b>
              <span className="block text-[11px] text-muted">{d.covers.join(", ")}</span>
              {d.value < d.alone && (
                <span className="block text-[11px] text-warn">
                  Da sola varrebbe {fmt2(d.alone)}: insieme agli altri effetti non sempre si può usare (il costo o il limite di attivazioni è già stato speso).
                </span>
              )}
            </p>
          ))
        )}
      </div>

      {s.floodgateList.length > 0 && (
        <div className="flex flex-col gap-1">
          <p className="eyebrow text-[10px] text-faint">Floodgate: l'avversario non riesce a</p>
          <p className="text-xs">{s.floodgateList.join(", ")}</p>
        </div>
      )}

      <div className="flex flex-col gap-1">
        <p className="eyebrow text-[10px] text-faint">Contro i board breaker (interruzioni che restano)</p>
        <div className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-0.5 text-xs">
          {s.breakers.map((b) => (
            <div key={b.name} className="contents">
              <span className="text-muted">{b.name}</span>
              <span className="text-right tabular-nums">
                {b.notApplicable ? <span className="text-faint">non si attiva</span> : b.stoppedBy != null ? <span>negato da {name(b.stoppedBy)} · {fmt2(b.remaining)}</span> : fmt2(b.remaining)}
              </span>
            </div>
          ))}
        </div>
      </div>

      <p className="text-[11px] text-faint">
        Risorse: {s.resourcesNote}. Follow-up: {s.followupNote}.
      </p>
    </div>
  );
}
