/**
 * Small SVG chart kit for the lab: grouped bars, lines with a crosshair,
 * the flask meter, tornado, heatmap and forest plots. Marks take their colors from CSS tokens, axes and
 * text from the theme; every mark has a tooltip (data-tip) and every chart
 * can show its data as a table.
 */
import { useId, useState, type ReactNode } from "react";
import type { WinRecord } from "../lib/api";
import { decided, played, recordTip, verdict } from "../lib/matches";
import { hideTip, pct, showTip } from "../lib/tooltip";

export interface Series {
  name: string;
  color: string;
  values: number[];
}

const lin = (d0: number, d1: number, r0: number, r1: number) => (v: number) => r0 + ((v - d0) / (d1 - d0 || 1)) * (r1 - r0);

/** Rounded-top bar from the baseline: 4px data end, square at the base. */
function barPath(x: number, y0: number, w: number, y1: number, r = 4) {
  const h = y0 - y1;
  r = Math.max(0, Math.min(r, h, w / 2));
  return `M${x},${y0}V${y1 + r}Q${x},${y1} ${x + r},${y1}H${x + w - r}Q${x + w},${y1} ${x + w},${y1 + r}V${y0}Z`;
}

function YGrid({ y, ticks, x0, x1, fmt }: { y: (v: number) => number; ticks: number[]; x0: number; x1: number; fmt: (v: number) => string }) {
  return (
    <g>
      {ticks.map((t) => (
        <g key={t}>
          <line x1={x0} x2={x1} y1={y(t)} y2={y(t)} className="stroke-[var(--df-border)]" strokeWidth={1} />
          <text x={x0 - 8} y={y(t) + 4} textAnchor="end" className="chart-axis">
            {fmt(t)}
          </text>
        </g>
      ))}
    </g>
  );
}

export function Legend({ items }: { items: { name: string; color: string }[] }) {
  return (
    <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-muted">
      {items.map((i) => (
        <span key={i.name} className="inline-flex items-center gap-1.5">
          <i className="inline-block h-2.5 w-2.5 rounded-[3px]" style={{ background: i.color }} />
          {i.name}
        </span>
      ))}
    </div>
  );
}

/** Grouped columns over categories (e.g. 0, 1, 2… cards in hand). */
export function BarChart({ labels, series, max, height = 220, unit = "", ariaLabel }: { labels: string[]; series: Series[]; max?: number; height?: number; unit?: string; ariaLabel: string }) {
  const w = 460;
  const m = { l: 40, r: 8, t: 18, b: 24 };
  const top = max ?? Math.max(0.1, Math.ceil(Math.max(...series.flatMap((s) => s.values)) * 10) / 10);
  const y = lin(0, top, height - m.b, m.t);
  const ticks = Array.from({ length: Math.round(top / 0.1) + 1 }, (_, i) => i * 0.1).filter((_, i, a) => a.length <= 6 || i % 2 === 0);
  const band = (w - m.l - m.r) / labels.length;
  const bw = Math.min(22, (band - 16) / series.length);
  const xOf = (i: number, k: number) => m.l + band * i + band / 2 - (bw * series.length + 2 * (series.length - 1)) / 2 + k * (bw + 2);
  return (
    <svg viewBox={`0 0 ${w} ${height}`} role="img" aria-label={ariaLabel} className="block h-auto w-full overflow-visible">
      <YGrid y={y} ticks={ticks} x0={m.l} x1={w - m.r} fmt={(v) => pct(v, 0)} />
      {labels.map((lab, i) => (
        <text key={lab} x={m.l + band * i + band / 2} y={height - 6} textAnchor="middle" className="chart-axis">
          {lab}
        </text>
      ))}
      {series.map((s, k) =>
        s.values.map((v, i) =>
          v < 0.0005 ? null : (
            <path
              key={`${k}-${i}`}
              d={barPath(xOf(i, k), y(0), bw, y(Math.min(v, top)))}
              className="chart-bar"
              style={{ fill: s.color, animationDelay: `${i * 50}ms` }}
              tabIndex={0}
              data-tip={`${s.name}\n${labels[i]}${unit}: ${pct(v)}`}
            />
          ),
        ),
      )}
      <line x1={m.l} x2={w - m.r} y1={y(0)} y2={y(0)} className="stroke-[var(--df-border-strong)]" />
      {series.map((s, k) => {
        const i = s.values.indexOf(Math.max(...s.values));
        // Two maxima on the same column would overlap: label the first only.
        if (series.slice(0, k).some((o) => o.values.indexOf(Math.max(...o.values)) === i)) return null;
        return (
          <text key={s.name} x={xOf(i, k) + bw / 2} y={y(Math.min(s.values[i], top)) - 6} textAnchor="middle" className="chart-label">
            {pct(s.values[i], 0)}
          </text>
        );
      })}
    </svg>
  );
}

/** Lines over a numeric x, with a crosshair tooltip showing every series. */
export function LineChart({
  xs,
  series,
  ymax = 1,
  yTicks,
  xfmt = (x) => String(x),
  xlabel,
  mark,
  height = 220,
  width = 460,
  ariaLabel,
}: {
  xs: number[];
  series: Series[];
  ymax?: number;
  yTicks?: number[];
  xfmt?: (x: number) => string;
  xlabel?: string;
  mark?: { x: number; y: number; label: string };
  height?: number;
  width?: number;
  ariaLabel: string;
}) {
  const w = width;
  const m = { l: 40, r: 46, t: 12, b: xlabel ? 38 : 24 };
  const x = lin(xs[0], xs[xs.length - 1], m.l, w - m.r);
  const y = lin(0, ymax, height - m.b, m.t);
  const [cross, setCross] = useState<number | null>(null);
  const ticks = yTicks ?? [0, 0.25, 0.5, 0.75, 1].filter((t) => t <= ymax + 1e-9);
  // About a dozen x labels at most, on round steps.
  const every = [1, 2, 5, 10, 20, 50, 100, 200, 500].find((s) => xs.length / s <= 12) ?? Math.ceil(xs.length / 12);
  // End labels of different series pushed apart when they would overlap.
  const ends = series.map((s) => y(s.values[s.values.length - 1]) + 4);
  series
    .map((_, i) => i)
    .sort((a, b) => ends[a] - ends[b])
    .forEach((i, k, order) => {
      if (k > 0) ends[i] = Math.max(ends[i], ends[order[k - 1]] + 12);
    });
  return (
    <svg viewBox={`0 0 ${w} ${height}`} role="img" aria-label={ariaLabel} className="block h-auto w-full overflow-visible">
      <YGrid y={y} ticks={ticks} x0={m.l} x1={w - m.r} fmt={(v) => pct(v, 0)} />
      {xs.map((v, i) =>
        (every === 1 ? true : v % every === 0 || i === 0) ? (
          <text key={v} x={x(v)} y={height - m.b + 16} textAnchor="middle" className="chart-axis">
            {xfmt(v)}
          </text>
        ) : null,
      )}
      {xlabel && (
        <text x={(m.l + w - m.r) / 2} y={height - 4} textAnchor="middle" className="chart-note">
          {xlabel}
        </text>
      )}
      {cross != null && <line x1={x(xs[cross])} x2={x(xs[cross])} y1={m.t} y2={height - m.b} className="stroke-[var(--df-border-strong)]" />}
      {series.map((s, k) => {
        const last = s.values.length - 1;
        return (
          <g key={s.name}>
            <path
              d={s.values.map((v, i) => `${i ? "L" : "M"}${x(xs[i])},${y(v)}`).join("")}
              pathLength={1}
              className="chart-line"
              style={{ stroke: s.color }}
              fill="none"
              strokeWidth={2}
              strokeLinejoin="round"
              strokeLinecap="round"
            />
            <circle cx={x(xs[last])} cy={y(s.values[last])} r={4} style={{ fill: s.color }} className="stroke-[var(--df-surface)]" strokeWidth={2} />
            <text x={x(xs[last]) + 8} y={ends[k]} className="chart-label">
              {pct(s.values[last], 0)}
            </text>
          </g>
        );
      })}
      {mark && (
        <g>
          <circle cx={x(mark.x)} cy={y(mark.y)} r={5} style={{ fill: series[0]?.color }} className="stroke-[var(--df-surface)]" strokeWidth={2} />
          <text x={x(mark.x) + 10} y={y(mark.y) + 18} className="chart-label">
            {mark.label}
          </text>
        </g>
      )}
      <rect
        x={m.l}
        y={m.t}
        width={w - m.l - m.r}
        height={height - m.t - m.b}
        fill="transparent"
        data-crosshair
        onPointerMove={(e) => {
          const r = e.currentTarget.ownerSVGElement!.getBoundingClientRect();
          const vx = ((e.clientX - r.left) / r.width) * w;
          let i = 0;
          xs.forEach((v, k) => {
            if (Math.abs(x(v) - vx) < Math.abs(x(xs[i]) - vx)) i = k;
          });
          setCross(i);
          showTip(`${xfmt(xs[i])}${xlabel ? ` ${xlabel}` : ""}\n${series.map((s) => `${s.name}: ${pct(s.values[i])}`).join("\n")}`, e.clientX, e.clientY);
        }}
        onPointerLeave={() => {
          setCross(null);
          hideTip();
        }}
      />
    </svg>
  );
}

/** A lab flask filled up to `value` (0..1), with a couple of rising bubbles. */
export function FlaskMeter({ value, size = 44, tone = "var(--sec)", bubbling = false }: { value: number; size?: number; tone?: string; bubbling?: boolean }) {
  const id = useId().replace(/:/g, "");
  const level = 14 + (1 - Math.max(0, Math.min(1, value))) * 38;
  const outline = "M17 3h10v13l13 27a5 5 0 0 1-4.5 7.5h-27A5 5 0 0 1 4 43l13-27z";
  return (
    <svg width={size} height={size * 1.23} viewBox="0 0 44 54" aria-hidden="true" className="shrink-0" data-bubbling={bubbling || undefined}>
      <defs>
        <clipPath id={id}>
          <path d={outline} />
        </clipPath>
      </defs>
      <g clipPath={`url(#${id})`}>
        <rect x={0} y={0} width={44} height={60} className="flask-liquid" style={{ fill: tone, transform: `translateY(${level}px)` }} opacity={0.55} />
        <rect x={0} y={0} width={44} height={1.6} className="flask-liquid" style={{ fill: tone, transform: `translateY(${level}px)` }} />
        <circle cx={16} cy={48} r={1.8} className="flask-bubble" style={{ fill: tone }} />
        <circle cx={26} cy={50} r={1.3} className="flask-bubble" style={{ fill: tone, animationDelay: "1.1s" }} />
        <circle cx={21} cy={49} r={1} className="flask-bubble" style={{ fill: tone, animationDelay: "2.2s" }} />
      </g>
      <path d={outline} fill="none" style={{ stroke: tone }} strokeWidth={1.6} />
      <path d="M14 3h16" style={{ stroke: tone }} strokeWidth={2} strokeLinecap="round" />
    </svg>
  );
}

/** A chart panel: title, note, legend, the chart and its data table. */
export function ChartCard({ title, note, legend, table, children }: { title: string; note?: string; legend?: ReactNode; table?: { head: string[]; rows: (string | number)[][] }; children: ReactNode }) {
  return (
    <section className="lab-card flex min-w-0 flex-col gap-2 p-4">
      <div>
        <h3 className="text-sm font-semibold">{title}</h3>
        {note && <p className="text-xs text-muted">{note}</p>}
      </div>
      {legend}
      {children}
      {table && (
        <details className="text-xs text-muted">
          <summary className="cursor-pointer text-faint hover:text-muted">Dati</summary>
          <table className="mt-1.5 w-full tabular-nums">
            <thead>
              <tr>
                {table.head.map((h) => (
                  <th key={h} className="border-b border-border pr-4 pb-1 text-left font-semibold whitespace-nowrap">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((r, i) => (
                <tr key={i}>
                  {r.map((c, j) => (
                    <td key={j} className="border-b border-border/60 py-1 pr-4 whitespace-nowrap">
                      {c}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </details>
      )}
    </section>
  );
}

/** Simulation estimate over a log scale of hands, with its 95% band. */
export function ConvergenceChart({ points, exact, ariaLabel }: { points: { hands: number; p: number; lo: number; hi: number }[]; exact?: number | null; ariaLabel: string }) {
  const w = 620;
  const h = 230;
  const m = { l: 44, r: 120, t: 12, b: 38 };
  if (points.length === 0) return null;
  const all = points.flatMap((p) => [p.lo, p.hi]).concat(exact != null ? [exact] : []);
  const lo = Math.max(0, Math.floor((Math.min(...all) - 0.02) * 20) / 20);
  const hi = Math.min(1, Math.ceil((Math.max(...all) + 0.02) * 20) / 20);
  const first = Math.log10(Math.min(points[0].hands, 1000));
  const last = Math.log10(Math.max(points[points.length - 1].hands, 10_000));
  const x = (n: number) => lin(first, last, m.l, w - m.r)(Math.log10(Math.max(n, 1)));
  const y = lin(lo, hi, h - m.b, m.t);
  const ticks = Array.from({ length: Math.round((hi - lo) / 0.05) + 1 }, (_, i) => lo + i * 0.05).filter((_, i, a) => a.length <= 6 || i % 2 === 0);
  const decades = [1e3, 1e4, 1e5, 1e6, 1e7].filter((d) => Math.log10(d) >= first - 1e-9 && Math.log10(d) <= last + 1e-9);
  const band = points.map((p, i) => `${i ? "L" : "M"}${x(p.hands)},${y(p.hi)}`).join("") + [...points].reverse().map((p) => `L${x(p.hands)},${y(p.lo)}`).join("") + "Z";
  const end = points[points.length - 1];
  return (
    <svg viewBox={`0 0 ${w} ${h}`} role="img" aria-label={ariaLabel} className="block h-auto w-full overflow-visible">
      <YGrid y={y} ticks={ticks} x0={m.l} x1={w - m.r} fmt={(v) => pct(v, 0)} />
      {decades.map((d) => (
        <text key={d} x={x(d)} y={h - m.b + 16} textAnchor="middle" className="chart-axis">
          {d.toLocaleString("it-IT")}
        </text>
      ))}
      <text x={(m.l + w - m.r) / 2} y={h - 4} textAnchor="middle" className="chart-note">
        mani simulate
      </text>
      <path d={band} style={{ fill: "var(--sec)" }} opacity={0.14} />
      {exact != null && (
        <g>
          <line x1={m.l} x2={w - m.r} y1={y(exact)} y2={y(exact)} className="stroke-[var(--df-faint)]" strokeWidth={1} />
          <text x={w - m.r + 8} y={y(exact) + 14} className="chart-note">
            esatto senza effetti {pct(exact)}
          </text>
        </g>
      )}
      <path d={points.map((p, i) => `${i ? "L" : "M"}${x(p.hands)},${y(p.p)}`).join("")} fill="none" style={{ stroke: "var(--sec)" }} strokeWidth={2} strokeLinejoin="round" />
      {points.map((p) => (
        <circle
          key={p.hands}
          cx={x(p.hands)}
          cy={y(p.p)}
          r={4}
          style={{ fill: "var(--sec)" }}
          className="stroke-[var(--df-surface)]"
          strokeWidth={2}
          tabIndex={0}
          data-tip={`${p.hands.toLocaleString("it-IT")} mani\nstima ${pct(p.p, 2)}\nintervallo ${pct(p.lo, 2)} – ${pct(p.hi, 2)}`}
        />
      ))}
      <text x={x(end.hands) + 8} y={y(end.p) - 8} className="chart-label">
        {pct(end.p)} ± {pct((end.hi - end.lo) / 2, 2)}
      </text>
    </svg>
  );
}

/** Horizontal bars around zero: what one copy more (right) or less (left) changes. */
export function TornadoChart({ rows, ariaLabel }: { rows: { label: string; plus: number | null; minus: number | null }[]; ariaLabel: string }) {
  const w = 490;
  const rowH = 30;
  const m = { l: 184, r: 44, t: 6, b: 22 };
  const h = m.t + rows.length * rowH + m.b;
  // Room past the longest bar for its value label.
  const max = Math.max(0.01, ...rows.flatMap((r) => [Math.abs(r.plus ?? 0), Math.abs(r.minus ?? 0)])) * 1.45;
  const x = lin(-max, max, m.l, w - m.r);
  const pp = (v: number) => `${v >= 0 ? "+" : "−"}${Math.abs(v * 100).toLocaleString("it-IT", { minimumFractionDigits: 1, maximumFractionDigits: 1 })}`;
  return (
    <svg viewBox={`0 0 ${w} ${h}`} role="img" aria-label={ariaLabel} className="block h-auto w-full overflow-visible">
      <line x1={x(0)} x2={x(0)} y1={m.t} y2={h - m.b} className="stroke-[var(--df-border-strong)]" />
      <text x={x(-max)} y={h - 6} className="chart-axis">
        −1 copia
      </text>
      <text x={x(max)} y={h - 6} textAnchor="end" className="chart-axis">
        +1 copia
      </text>
      {rows.map((r, i) => {
        const cy = m.t + i * rowH + rowH / 2;
        return (
          <g key={r.label}>
            <text x={m.l - 10} y={cy + 4} textAnchor="end" className="chart-label">
              <title>{r.label}</title>
              {r.label.length > 27 ? `${r.label.slice(0, 26)}…` : r.label}
            </text>
            {(
              [
                [r.plus, "var(--df-ok)", "+1 copia"],
                [r.minus, "var(--df-danger)", "−1 copia"],
              ] as const
            ).map(([v, color, what]) =>
              v == null || Math.abs(v) < 0.0005 ? null : (
                <g key={what}>
                  <rect
                    x={Math.min(x(0), x(v))}
                    y={cy - 8}
                    width={Math.abs(x(v) - x(0))}
                    height={16}
                    rx={3}
                    style={{ fill: color }}
                    opacity={0.85}
                    className="chart-bar"
                    tabIndex={0}
                    data-tip={`${r.label}, ${what}\n${pp(v)} punti percentuali`}
                  />
                  <text x={v > 0 ? x(v) + 6 : x(v) - 6} y={cy + 4} textAnchor={v > 0 ? "start" : "end"} className="chart-axis">
                    {pp(v)}
                  </text>
                </g>
              ),
            )}
            {(r.plus == null || Math.abs(r.plus) < 0.0005) && (r.minus == null || Math.abs(r.minus) < 0.0005) && (
              <text x={x(0) + 8} y={cy + 4} className="chart-note">
                {r.plus == null ? "già al massimo delle copie" : "nessun effetto"}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

/** Grid of probabilities for two counts, colored on one hue (light to dark). */
export function HeatmapChart({
  rows,
  cols,
  values,
  current,
  rowLabel,
  colLabel,
  ariaLabel,
}: {
  rows: number[];
  cols: number[];
  values: (number | null)[][];
  current: [number, number];
  rowLabel: string;
  colLabel: string;
  ariaLabel: string;
}) {
  const cell = 34;
  const gap = 2;
  const m = { l: 92, t: 22, b: 30 };
  const w = m.l + cols.length * (cell + gap) + 8;
  const h = m.t + rows.length * (cell + gap) + m.b;
  const flat = values.flat().filter((v): v is number => v != null);
  const lo = Math.min(...flat);
  const hi = Math.max(...flat);
  const shade = (v: number) => `color-mix(in oklab, var(--sec) ${Math.round(12 + ((v - lo) / (hi - lo || 1)) * 88)}%, var(--df-surface))`;
  return (
    <div className="flex flex-col gap-2">
      <svg viewBox={`0 0 ${w} ${h}`} role="img" aria-label={ariaLabel} className="block h-auto w-full max-w-[560px] overflow-visible">
        {cols.map((c, j) => (
          <text key={c} x={m.l + j * (cell + gap) + cell / 2} y={m.t - 8} textAnchor="middle" className="chart-axis">
            {c}
          </text>
        ))}
        <text x={m.l + (cols.length * (cell + gap)) / 2} y={h - 6} textAnchor="middle" className="chart-note">
          {colLabel}
        </text>
        {rows.map((r, i) => (
          <g key={r}>
            <text x={m.l - 8} y={m.t + i * (cell + gap) + cell / 2 + 4} textAnchor="end" className="chart-axis">
              {r} {rowLabel}
            </text>
            {cols.map((c, j) => {
              const v = values[i][j];
              if (v == null) return null;
              const here = r === current[0] && c === current[1];
              return (
                <g key={c}>
                  <rect
                    x={m.l + j * (cell + gap)}
                    y={m.t + i * (cell + gap)}
                    width={cell}
                    height={cell}
                    rx={4}
                    style={{ fill: shade(v), stroke: here ? "var(--df-text)" : "none", strokeWidth: here ? 2 : 0 }}
                    tabIndex={0}
                    data-tip={`${r} ${rowLabel}, ${c} ${colLabel}\n${pct(v)}`}
                  />
                  {here && (
                    <text x={m.l + j * (cell + gap) + cell / 2} y={m.t + i * (cell + gap) + cell / 2 + 4} textAnchor="middle" className="chart-label" style={{ fontSize: 10 }}>
                      {pct(v, 0)}
                    </text>
                  )}
                </g>
              );
            })}
          </g>
        ))}
      </svg>
      <Legend
        items={[
          { name: pct(lo, 0), color: shade(lo) },
          { name: pct((lo + hi) / 2, 0), color: shade((lo + hi) / 2) },
          { name: `${pct(hi, 0)} · riquadro: il deck attuale`, color: shade(hi) },
        ]}
      />
    </div>
  );
}

export interface ForestRow {
  label: string;
  /** Second line under the label (dates, changes...). */
  sub?: string;
  record: WinRecord;
}

/**
 * Win rates with their 95% interval, one row each, against the 50% line
 * and an optional reference (the overall win rate). A row whose whole
 * interval is above or below 50% is marked ▲ or ▼, not by color alone.
 */
export function ForestPlot({ rows, reference, width = 460, ariaLabel }: { rows: ForestRow[]; reference?: { value: number; label: string }; width?: number; ariaLabel: string }) {
  const w = width;
  const rowH = 32;
  const m = { l: 128, r: 84, t: 24, b: 6 };
  const h = m.t + rows.length * rowH + m.b;
  const x = lin(0, 1, m.l, w - m.r);
  const most = Math.max(1, ...rows.map((r) => decided(r.record)));
  return (
    <svg viewBox={`0 0 ${w} ${h}`} role="img" aria-label={ariaLabel} className="block h-auto w-full overflow-visible">
      {[0, 0.25, 0.5, 0.75, 1].map((t) => (
        <g key={t}>
          <line x1={x(t)} x2={x(t)} y1={m.t - 4} y2={h - m.b} className={t === 0.5 ? "stroke-[var(--df-border-strong)]" : "stroke-[var(--df-border)]"} strokeDasharray={t === 0.5 ? "4 3" : undefined} strokeWidth={t === 0.5 ? 1.5 : 1} />
          <text x={x(t)} y={m.t - 10} textAnchor="middle" className="chart-axis">
            {pct(t, 0)}
          </text>
        </g>
      ))}
      {reference && (
        <g>
          <line x1={x(reference.value)} x2={x(reference.value)} y1={m.t - 2} y2={h - m.b} style={{ stroke: "var(--sec)" }} strokeOpacity={0.45} strokeWidth={1.5} />
          <text x={x(reference.value) + 4} y={h - m.b - 2} className="chart-note">
            {reference.label}
          </text>
        </g>
      )}
      {rows.map((r, i) => {
        const cy = m.t + i * rowH + rowH / 2;
        const rec = r.record;
        const n = decided(rec);
        const v = verdict(rec);
        const tone = v === "up" ? "var(--df-ok)" : v === "down" ? "var(--df-danger)" : "var(--sec)";
        return (
          <g key={`${r.label}-${i}`} tabIndex={0} data-tip={recordTip(r.sub ? `${r.label} · ${r.sub}` : r.label, rec)} className="forest-row">
            <rect x={0} y={cy - rowH / 2} width={w} height={rowH} rx={6} className="forest-hit" />
            <text x={m.l - 12} y={r.sub ? cy - 1 : cy + 4} textAnchor="end" className="chart-label">
              <title>{r.label}</title>
              {r.label.length > 18 ? `${r.label.slice(0, 17)}…` : r.label}
            </text>
            {r.sub && (
              <text x={m.l - 12} y={cy + 11} textAnchor="end" className="chart-note">
                {r.sub.length > 22 ? `${r.sub.slice(0, 21)}…` : r.sub}
              </text>
            )}
            {n === 0 ? (
              <text x={x(0.5) + 8} y={cy + 4} className="chart-note">
                nessuna partita decisa
              </text>
            ) : (
              <>
                <line x1={x(rec.lo)} x2={x(rec.hi)} y1={cy} y2={cy} style={{ stroke: tone }} strokeWidth={2} strokeLinecap="round" opacity={0.75} className="chart-line" pathLength={1} />
                {[rec.lo, rec.hi].map((e, k) => (
                  <line key={k} x1={x(e)} x2={x(e)} y1={cy - 4} y2={cy + 4} style={{ stroke: tone }} strokeWidth={1.5} opacity={0.75} />
                ))}
                <circle cx={x(rec.p)} cy={cy} r={4 + 3 * Math.sqrt(n / most)} style={{ fill: tone }} className="stroke-[var(--df-surface)]" strokeWidth={2} />
              </>
            )}
            <text x={w - m.r + 12} y={cy + 4} className="chart-label">
              {n ? pct(rec.p, 0) : "—"}
              <tspan className="chart-note" dx={5}>
                {played(rec)}
              </tspan>
              {v && (
                <tspan dx={5} style={{ fill: tone }}>
                  {v === "up" ? "▲" : "▼"}
                </tspan>
              )}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

/** Opponent × going first / second, cells on a red–gray–green scale around 50%, fainter with few matches. */
export function WinMatrix({ rows }: { rows: { name: string; first: WinRecord; second: WinRecord; total: WinRecord }[] }) {
  const cell = (title: string, r: WinRecord) => {
    const n = decided(r);
    if (played(r) === 0) return <td className="px-2 py-1.5 text-center text-faint">—</td>;
    // Stronger with a bigger gap from 50% and with more matches behind it.
    const k = n ? Math.round(Math.min(1, Math.abs(r.p - 0.5) * 2) * Math.sqrt(Math.min(1, n / 10)) * 60) : 0;
    const hue = r.p >= 0.5 ? "var(--df-ok)" : "var(--df-danger)";
    return (
      <td className="px-1 py-0.5">
        <span
          tabIndex={0}
          data-tip={recordTip(title, r)}
          className="flex items-baseline justify-center gap-1 rounded-md px-2 py-1 tabular-nums"
          style={{ background: `color-mix(in oklab, ${hue} ${k}%, var(--df-surface-2))` }}
        >
          <b className="text-[13px] font-semibold text-text">{n ? pct(r.p, 0) : "—"}</b>
          <span className="text-[10px] text-muted">{played(r)}</span>
        </span>
      </td>
    );
  };
  return (
    <table className="w-full text-xs">
      <thead>
        <tr className="text-faint">
          <th className="pb-1 text-left font-medium">Avversario</th>
          <th className="pb-1 font-medium">Per primo</th>
          <th className="pb-1 font-medium">Per secondo</th>
          <th className="pb-1 font-medium">Totale</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <tr key={r.name}>
            <td className="max-w-36 truncate py-0.5 pr-2 text-muted" title={r.name}>
              {r.name}
            </td>
            {cell(`${r.name}, per primo`, r.first)}
            {cell(`${r.name}, per secondo`, r.second)}
            {cell(r.name, r.total)}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
