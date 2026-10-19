import { useSyncExternalStore } from "react";
import { api, type ComboBoard, type ComboOutcome, type ComboProgress, type ComboRequest } from "./api";

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
}

const IDLE: ComboRun = { running: false, request: null, progress: null, live: {}, outcome: null, error: null };

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
  const unlisten = await api.onComboProgress((p) => set({ progress: p, live: { ...run.live, [p.handIndex]: p.top } }));
  try {
    const outcome = await api.comboSearch(request);
    set({ outcome });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    unlisten();
    set({ running: false });
  }
  return run;
}

export function stopCombo() {
  if (run.running) void api.comboCancel();
}
