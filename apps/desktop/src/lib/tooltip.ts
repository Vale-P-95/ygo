import { useSyncExternalStore } from "react";

/**
 * One tooltip for every chart. Marks carry their text in `data-tip` (lines
 * separated by "\n"); line charts call `showTip` for their crosshair.
 */
export interface Tip {
  text: string;
  x: number;
  y: number;
}

let tip: Tip | null = null;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

export function showTip(text: string, x: number, y: number) {
  tip = { text, x, y };
  emit();
}

export function hideTip() {
  if (!tip) return;
  tip = null;
  emit();
}

export function useTip(): Tip | null {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => tip,
  );
}

export const pct = (p: number, digits = 1) => `${(p * 100).toLocaleString("it-IT", { minimumFractionDigits: digits, maximumFractionDigits: digits })}%`;
