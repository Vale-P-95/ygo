import { useSyncExternalStore } from "react";
import { api, type ComboBoard, type ComboOutcome, type ComboProgress, type ComboRequest, type ComboScore } from "./api";

/**
 * The Combo Lab's search runs in the backend while the player does other
 * things: its state lives here, outside the page, so leaving the section
 * and coming back finds it still going (or finished).
 */
export interface ComboRun {
  running: boolean;
  request: ComboRequest | null;
  progress: ComboProgress | null;
  /** Best boards seen so far for each hand ("card + rest"). */
  live: Record<number, ComboBoard[]>;
  outcome: ComboOutcome | null;
  error: string | null;
  /** Probe turn results, by board (`hand:key`). */
  scores: Record<string, ComboScore>;
  /** The probe turn is running on the best boards. */
  scoring: boolean;
  /** Chosen opponent deck ID for matchup measurement, if any. */
  vsDeckId: number | null;
}

const IDLE: ComboRun = { running: false, request: null, progress: null, live: {}, outcome: null, error: null, scores: {}, scoring: false, vsDeckId: null };

let run: ComboRun = IDLE;
const listeners = new Set<() => void>();

function set(next: Partial<ComboRun>) {
  run = { ...run, ...next };
  listeners.forEach((l) => l());
}

export function useCombo(): ComboRun {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => run,
  );
}

/** Starts a search; resolves when it ends (finished, stopped or failed). */
export async function startCombo(request: ComboRequest): Promise<ComboRun> {
  if (run.running) return run;
  set({ ...IDLE, running: true, request });
  return follow(() => api.comboSearch(request));
}

/**
 * Goes on with the last search for `seconds` more, from where it stopped:
 * the boards found stay, and the new ones join them.
 */
export async function continueCombo(seconds: number): Promise<ComboRun> {
  if (run.running || !run.request || !run.outcome) return run;
  const previous = run.outcome;
  set({ running: true, error: null, request: { ...run.request, seconds: run.request.seconds + seconds }, outcome: null, live: liveOf(previous), scores: run.scores });
  return follow(() => api.comboContinue(run.request!.deckId, seconds));
}

/** The boards of an outcome by hand, as the live view keeps them. */
function liveOf(outcome: ComboOutcome): Record<number, ComboBoard[]> {
  const live: Record<number, ComboBoard[]> = {};
  for (const b of outcome.boards) (live[b.hand] ??= []).push(b);
  return live;
}

/** The boards seen so far for a hand, with the newest best ones. */
function merge(seen: ComboBoard[] = [], top: ComboBoard[]): ComboBoard[] {
  const keys = new Set(top.map((b) => b.key));
  return [...top, ...seen.filter((b) => !keys.has(b.key))].slice(0, 200);
}

async function follow(call: () => Promise<ComboOutcome>): Promise<ComboRun> {
  const unlisten = await api.onComboProgress((p) => set({ progress: p, live: { ...run.live, [p.handIndex]: merge(run.live[p.handIndex], p.top) } }));
  let finished = false;
  try {
    const outcome = await call();
    set({ outcome, live: {} });
    finished = outcome.stop !== "Cancelled";
  } catch (e) {
    set({ error: String(e) });
  } finally {
    unlisten();
    set({ running: false });
  }
  if (finished) void scoreBoards(run.vsDeckId);
  return run;
}

/** Puts the best boards to the probe turn; scores arrive one by one. */
export async function scoreBoards(vsDeckId?: number | null) {
  if (run.scoring || run.running) return;
  const targetVs = vsDeckId !== undefined ? vsDeckId : run.vsDeckId;
  const resetScores = vsDeckId !== undefined && vsDeckId !== run.vsDeckId;
  set({ scoring: true, vsDeckId: targetVs, scores: resetScores ? {} : run.scores });
  const unlisten = await api.onComboScore((s) => set({ scores: { ...run.scores, [`${s.hand}:${s.key}`]: s } }));
  try {
    await api.comboScore(targetVs);
  } catch (e) {
    set({ error: String(e) });
  } finally {
    unlisten();
    set({ scoring: false });
  }
}

export function stopCombo() {
  if (run.running) void api.comboCancel();
}
