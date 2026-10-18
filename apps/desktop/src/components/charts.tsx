/**
 * Small SVG chart kit for the lab: grouped bars, lines with a crosshair,
 * and the flask meter. Marks take their colors from CSS tokens, axes and
 * text from the theme; every mark has a tooltip (data-tip) and every chart
 * can show its data as a table.
 */
import { useId, useState, type ReactNode } from "react";
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
          <line x1={x0} x2={x1} y1={y(t)} y2={y(t)} className="stroke-[var(--df-surface-3)]" strokeWidth={1} />
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
  const every = xs.length > 12 ? 2 : 1;
  return (
    <svg viewBox={`0 0 ${w} ${height}`} role="img" aria-label={ariaLabel} className="block h-auto w-full overflow-visible">
      <YGrid y={y} ticks={ticks} x0={m.l} x1={w - m.r} fmt={(v) => pct(v, 0)} />
      {xs.map((v, i) =>
        i % every === 0 ? (
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
      {series.map((s) => {
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
            <text x={x(xs[last]) + 8} y={y(s.values[last]) + 4} className="chart-label">
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
          <table className="mt-1.5 tabular-nums">
            <thead>
              <tr>
                {table.head.map((h) => (
                  <th key={h} className="border-b border-border pr-4 pb-1 text-left font-semibold">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((r, i) => (
                <tr key={i}>
                  {r.map((c, j) => (
                    <td key={j} className="border-b border-border/60 py-0.5 pr-4">
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
