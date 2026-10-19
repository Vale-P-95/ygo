import { useSyncExternalStore } from "react";
import type { ComboScore } from "./api";

/**
 * The score of a board: the metrics the probe turn measures, each times a
 * weight the player can change. Interruptions, floodgates and resilience
 * are in "interruptions" (1 = negating any activation once, without a
 * target); resources and follow-up in cards; cost in cards of the hand
 * beyond the first.
 */
export type MetricKey = "interruptions" | "handTraps" | "floodgates" | "resilience" | "followup" | "resources" | "cost";

export interface Metric {
  key: MetricKey;
  label: string;
  hint: string;
  color: string;
}

export const METRICS: Metric[] = [
  { key: "interruptions", label: "Interruzioni", hint: "Effetti sul campo, nel Cimitero o banditi che fermano le azioni dell'avversario, per quanto le coprono e quante volte", color: "var(--metric-interruptions)" },
  { key: "handTraps", label: "Dalla mano", hint: "Le stesse interruzioni, ma dalle carte rimaste in mano (handtrap)", color: "var(--metric-hand)" },
  { key: "floodgates", label: "Floodgate", hint: "Azioni che l'avversario fa su un campo vuoto e non contro questa board", color: "var(--metric-floodgates)" },
  { key: "resilience", label: "Resilienza", hint: "Interruzioni che restano in media dopo Dark Hole, Raigeki, Harpie's, Kaiju e gli altri board breaker", color: "var(--metric-resilience)" },
  { key: "followup", label: "Follow-up", hint: "Starter ed estensori rimasti in mano (dai ruoli del Laboratorio)", color: "var(--metric-followup)" },
  { key: "resources", label: "Risorse", hint: "Carte in mano, e un po' il Cimitero", color: "var(--metric-resources)" },
  { key: "cost", label: "Costo", hint: "Carte della mano usate oltre la prima (peso negativo)", color: "var(--df-faint)" },
];

export type Weights = Record<MetricKey, number>;

export interface Profile {
  id: string;
  label: string;
  weights: Weights;
}

export const PROFILES: Profile[] = [
  { id: "balanced", label: "Bilanciato", weights: { interruptions: 1, handTraps: 0.9, floodgates: 0.8, resilience: 0.6, followup: 0.4, resources: 0.3, cost: -0.1 } },
  { id: "negates", label: "Più negazioni", weights: { interruptions: 1.5, handTraps: 1.2, floodgates: 0.5, resilience: 0.4, followup: 0.2, resources: 0.2, cost: -0.05 } },
  { id: "control", label: "Controllo e floodgate", weights: { interruptions: 0.8, handTraps: 0.7, floodgates: 1.6, resilience: 0.6, followup: 0.3, resources: 0.2, cost: -0.1 } },
  { id: "resilient", label: "A prova di board breaker", weights: { interruptions: 0.8, handTraps: 0.9, floodgates: 0.6, resilience: 1.5, followup: 0.5, resources: 0.3, cost: -0.1 } },
  { id: "grind", label: "Risorse e follow-up", weights: { interruptions: 0.7, handTraps: 0.7, floodgates: 0.5, resilience: 0.6, followup: 1.2, resources: 0.8, cost: -0.2 } },
];

export function score(s: ComboScore, w: Weights): number {
  return METRICS.reduce((t, m) => t + w[m.key] * s[m.key], 0);
}

/** Each metric's part of the score (the cost is negative). */
export function parts(s: ComboScore, w: Weights): { metric: Metric; value: number }[] {
  return METRICS.map((metric) => ({ metric, value: w[metric.key] * s[metric.key] }));
}

interface Choice {
  profile: string;
  weights: Weights;
}

const KEY = "duelforge.combo.weights";

function loadChoice(): Choice {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const saved = JSON.parse(raw) as Choice;
      return { profile: saved.profile, weights: { ...PROFILES[0].weights, ...saved.weights } };
    }
  } catch {
    // A missing or broken choice falls back to the default.
  }
  return { profile: PROFILES[0].id, weights: PROFILES[0].weights };
}

let choice = loadChoice();
const listeners = new Set<() => void>();

function set(next: Choice) {
  choice = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // Only a convenience.
  }
  listeners.forEach((l) => l());
}

export function useWeights(): Choice {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => choice,
  );
}

export function pickProfile(id: string) {
  const p = PROFILES.find((x) => x.id === id);
  if (p) set({ profile: p.id, weights: p.weights });
}

/** A weight changed by hand: the profile becomes "Personalizzato". */
export function setWeight(key: MetricKey, value: number) {
  set({ profile: "custom", weights: { ...choice.weights, [key]: value } });
}
