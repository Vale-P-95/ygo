import type { MatchEntry, WinRecord } from "./api";
import { pct } from "./tooltip";

/** Outcomes are stored as W/L/D and shown in Italian. */
export const OUTCOME: Record<string, { short: string; long: string; tone: string }> = {
  W: { short: "V", long: "Vittoria", tone: "var(--df-ok)" },
  L: { short: "S", long: "Sconfitta", tone: "var(--df-danger)" },
  D: { short: "P", long: "Pareggio", tone: "var(--df-faint)" },
};

export const decided = (r: WinRecord) => r.wins + r.losses;
export const played = (r: WinRecord) => r.wins + r.losses + r.draws;

/** "8 V · 4 S" (and draws, when there are any). */
export function score(r: WinRecord) {
  return `${r.wins} V · ${r.losses} S${r.draws ? ` · ${r.draws} P` : ""}`;
}

/** Tooltip text for a record. */
export function recordTip(title: string, r: WinRecord) {
  const n = played(r);
  if (decided(r) === 0) return `${title}\n${n} ${n === 1 ? "partita" : "partite"}, nessuna decisa`;
  return `${title}\n${n} ${n === 1 ? "partita" : "partite"}: ${score(r)}\nwin-rate ${pct(r.p)}\nintervallo 95% ${pct(r.lo)} – ${pct(r.hi)}\nstima bayesiana ${pct(r.betaLo)} – ${pct(r.betaHi)}`;
}

/** Above or below 50% beyond doubt (the whole 95% interval on one side). */
export function verdict(r: WinRecord): "up" | "down" | null {
  if (decided(r) === 0) return null;
  if (r.lo > 0.5) return "up";
  if (r.hi < 0.5) return "down";
  return null;
}

/** Current run of equal results, newest first: ["W", 3]. */
export function streak(matches: MatchEntry[]): [string, number] | null {
  const results = matches.map((m) => m.result ?? "").filter((r) => r === "W" || r === "L");
  const last = results[results.length - 1];
  if (!last) return null;
  let n = 0;
  for (let i = results.length - 1; i >= 0 && results[i] === last; i--) n++;
  return [last, n];
}

/** The result of a match from its games: more wins than losses is a win. */
export function resultOf(games: string) {
  const w = [...games].filter((g) => g === "W").length;
  const l = [...games].filter((g) => g === "L").length;
  return w > l ? "W" : l > w ? "L" : games ? "D" : "";
}

/** "12 ott 2026, 20:30" from "2026-10-12 20:30". */
export function playedLabel(at: string | undefined, time = true) {
  if (!at) return "";
  const d = new Date(at.replace(" ", "T"));
  if (Number.isNaN(d.getTime())) return at;
  return d.toLocaleString("it-IT", { day: "numeric", month: "short", year: "numeric", ...(time && at.length > 10 ? { hour: "2-digit", minute: "2-digit" } : {}) });
}

/** "12 ott" from "2026-10-12 20:30". */
export function shortDate(at: string) {
  const d = new Date(`${at.slice(0, 10)}T12:00`);
  return Number.isNaN(d.getTime()) ? at.slice(0, 10) : d.toLocaleDateString("it-IT", { day: "numeric", month: "short" });
}

/** "p = 0,042" with the usual reading. */
export function pValue(p: number) {
  if (p < 0.001) return "p < 0,001";
  if (p < 0.05) {
    const formatted = p.toLocaleString("it-IT", { maximumFractionDigits: 3, minimumFractionDigits: 3 });
    if (formatted === "0,050") return "p < 0,05";
    return `p = ${formatted}`;
  }
  return `p = ${p.toLocaleString("it-IT", { maximumFractionDigits: 3, minimumFractionDigits: p < 0.1 ? 3 : 2 })}`;
}
