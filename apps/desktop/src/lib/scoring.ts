import { useSyncExternalStore } from "react";
import type { ComboScore } from "./api";

/**
 * The score of a board: the metrics the probe turn measures, each times a
 * weight the player can change. Interruptions, floodgates and resilience
 * are in "interruptions" (1 = negating any activation once, without a
 * target); resources and follow-up in cards; cost in cards of the hand
 * beyond the first. A card in hand counts once: a measured handtrap in
 * its metric, a starter or extender in the follow-up, any other card in
 * the resources, which weigh far less than an interruption. The exposure
 * counts the moments of the line where a handtrap stops it (Ash Blossom,
 * a negation on the field, Nibiru, Droll): two lines to the same board
 * are not worth the same.
 */
export type MetricKey = "interruptions" | "handTraps" | "floodgates" | "resilience" | "followup" | "resources" | "cost" | "exposure";

export interface Metric {
  key: MetricKey;
  label: string;
  hint: string;
  color: string;
  /** For the "i" button: what it measures and how to read it, in plain words. */
  explain: string[];
}

export const METRICS: Metric[] = [
  {
    key: "interruptions",
    label: "Interruzioni",
    hint: "Effetti sul campo, nel Cimitero o banditi che fermano le azioni dell'avversario, per quanto le coprono e quante volte",
    color: "var(--metric-interruptions)",
    explain: [
      "Quanto le carte sul tuo campo (e nel Cimitero) fermano l'avversario durante il suo turno.",
      "Il motore gioca un turno di prova: un avversario fa le giocate più comuni (attivare una Magia, aggiungere una carta dal Deck, Evocare, attivare effetti dalla mano o dal Cimitero...) e conta quali riesci a fermare.",
      "1 vale una negazione che ferma qualsiasi giocata una volta (come Invoked Mechaba con un costo da pagare). Un effetto che ferma solo alcune giocate, o che distrugge invece di negare, vale meno.",
    ],
  },
  {
    key: "handTraps",
    label: "Dalla mano",
    hint: "Le stesse interruzioni, ma dalle carte rimaste in mano (handtrap)",
    color: "var(--metric-hand)",
    explain: [
      "Come le interruzioni, ma per le carte che tieni in mano a fine turno: Ash Blossom, Droll & Lock Bird, Effect Veiler...",
      "Una handtrap che con la tua board non si può usare vale 0: Mulcharmy Fuwalos e Purulia funzionano solo se non controlli carte, quindi andando per primo sono le carte migliori da scartare.",
    ],
  },
  {
    key: "floodgates",
    label: "Floodgate",
    hint: "Azioni che l'avversario fa su un campo vuoto e non contro questa board",
    color: "var(--metric-floodgates)",
    explain: [
      "Giocate che l'avversario riesce a fare contro un campo vuoto ma non contro questa board, senza che tu debba attivare nulla.",
      "Esempi: Invoked Caliga (dopo il primo effetto di mostro non può attivarne altri), Invoked Oceanus nella Zona Mostri Extra (le sue carte mandate al Cimitero vengono bandite).",
      "Ogni tipo di giocata bloccata conta per quanto pesa (una ricerca dal Deck pesa più di un'Evocazione Normale).",
    ],
  },
  {
    key: "resilience",
    label: "Resilienza",
    hint: "Interruzioni del campo che restano in media dopo Dark Hole, Raigeki, Harpie's, Kaiju, un attacco e gli altri board breaker (le handtrap non contano: hanno la loro voce)",
    color: "var(--metric-resilience)",
    explain: [
      "Quante delle tue interruzioni dal campo restano, in media, dopo che l'avversario prova a smontare la board: Dark Hole, Raigeki, Harpie's Feather Duster, Lightning Storm, Forbidden Droplet, un Kaiju, un attacco da 4200 ATK...",
      "Se una tua carta nega il board breaker, la board resta intera. Le handtrap non contano qui: hanno la loro voce.",
    ],
  },
  {
    key: "followup",
    label: "Follow-up",
    hint: "Starter ed estensori rimasti in mano (dai ruoli del Laboratorio)",
    color: "var(--metric-followup)",
    explain: [
      "Starter ed estensori rimasti in mano: le carte per ripartire al turno dopo, se l'avversario smonta la board.",
      "I ruoli vengono dal Laboratorio (pannello Ruoli, «Riconosci»): uno starter vale 1, un estensore mezzo.",
    ],
  },
  {
    key: "resources",
    label: "Risorse",
    hint: "Le altre carte rimaste in mano (non handtrap misurate, starter o estensori), e un po' il Cimitero",
    color: "var(--metric-resources)",
    explain: ["Le altre carte rimaste in mano, e un po' le carte nel Cimitero. Pesano poco: una carta qualsiasi conta molto meno di un'interruzione."],
  },
  {
    key: "cost",
    label: "Costo",
    hint: "Carte della mano usate oltre la prima (peso negativo)",
    color: "var(--df-faint)",
    explain: [
      "Quante carte della mano la linea usa oltre la prima. Toglie punti: a parità di board, è meglio la linea che usa meno carte e te ne lascia di più.",
    ],
  },
  {
    key: "exposure",
    label: "Esposizione",
    hint: "Punti della linea in cui una handtrap la ferma: effetti che Ash Blossom nega, effetti di mostri sul campo (Veiler, Impermanence), Nibiru dalla quinta Evocazione, Droll dalla seconda carta aggiunta dal Deck (peso negativo)",
    color: "var(--df-danger)",
    explain: [
      "Quante volte l'avversario può fermare la linea mentre la giochi, con le handtrap più usate:",
      "• ogni tuo effetto che aggiunge, evoca o manda carte dal Deck (Ash Blossom lo nega);",
      "• ogni effetto di mostro attivato sul campo (Effect Veiler, Infinite Impermanence, Ghost Ogre);",
      "• Nibiru, se arrivi a 5 Evocazioni;",
      "• Droll & Lock Bird, se aggiungi più di una carta dal Deck.",
      "Toglie punti: due linee che arrivano alla stessa board non valgono uguale se una si ferma alla prima Ash. Il profilo «Linee sicure» le pesa di più.",
    ],
  },
];

/** Metrics that count against a board: their weight goes from -1 to 0. */
export const NEGATIVE: MetricKey[] = ["cost", "exposure"];

export type Weights = Record<MetricKey, number>;

export interface Profile {
  id: string;
  label: string;
  weights: Weights;
}

export const PROFILES: Profile[] = [
  { id: "balanced", label: "Bilanciato", weights: { interruptions: 1, handTraps: 0.9, floodgates: 0.8, resilience: 0.6, followup: 0.4, resources: 0.1, cost: -0.1, exposure: -0.04 } },
  { id: "negates", label: "Più negazioni", weights: { interruptions: 1.5, handTraps: 1.2, floodgates: 0.5, resilience: 0.4, followup: 0.2, resources: 0.05, cost: -0.05, exposure: -0.03 } },
  { id: "control", label: "Controllo e floodgate", weights: { interruptions: 0.8, handTraps: 0.7, floodgates: 1.6, resilience: 0.6, followup: 0.3, resources: 0.05, cost: -0.1, exposure: -0.04 } },
  { id: "resilient", label: "A prova di board breaker", weights: { interruptions: 0.8, handTraps: 0.9, floodgates: 0.6, resilience: 1.5, followup: 0.5, resources: 0.1, cost: -0.1, exposure: -0.04 } },
  { id: "grind", label: "Risorse e follow-up", weights: { interruptions: 0.7, handTraps: 0.7, floodgates: 0.5, resilience: 0.6, followup: 1.2, resources: 0.3, cost: -0.2, exposure: -0.03 } },
  {
    id: "safe",
    label: "Linee sicure",
    weights: { interruptions: 0.9, handTraps: 0.9, floodgates: 0.7, resilience: 0.6, followup: 0.4, resources: 0.1, cost: -0.15, exposure: -0.15 },
  },
];

export function score(s: ComboScore, w: Weights): number {
  return METRICS.reduce((t, m) => t + w[m.key] * s[m.key], 0);
}

/** Each metric's part of the score (the cost is negative). */
export function parts(s: ComboScore, w: Weights): { metric: Metric; value: number }[] {
  return METRICS.map((metric) => ({ metric, value: w[metric.key] * s[metric.key] }));
}

/** Extracts the dictionary of metrics by key for preference recording. */
export function metricValues(s: ComboScore): Record<MetricKey, number> {
  return {
    interruptions: s.interruptions,
    handTraps: s.handTraps,
    floodgates: s.floodgates,
    resilience: s.resilience,
    followup: s.followup,
    resources: s.resources,
    cost: s.cost,
    exposure: s.exposure,
  };
}

/**
 * Checks if board score A Pareto-dominates board score B:
 * A is at least equal to B in all metrics (lower is better for cost and exposure),
 * and strictly better in at least one metric.
 */
export function dominates(a: ComboScore, b: ComboScore): boolean {
  if (a.error || b.error) return false;
  let strictlyBetter = false;
  const EPS = 1e-4;

  const posKeys: MetricKey[] = ["interruptions", "handTraps", "floodgates", "resilience", "followup", "resources"];
  for (const k of posKeys) {
    if (a[k] < b[k] - EPS) return false;
    if (a[k] > b[k] + EPS) strictlyBetter = true;
  }

  const negKeys: MetricKey[] = ["cost", "exposure"];
  for (const k of negKeys) {
    if (a[k] > b[k] + EPS) return false;
    if (a[k] < b[k] - EPS) strictlyBetter = true;
  }

  return strictlyBetter;
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
      // A profile is remembered by name: its weights are today's ones.
      const profile = PROFILES.find((p) => p.id === saved.profile);
      if (profile) return { profile: profile.id, weights: profile.weights };
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

/** Activates the weights learned from the player's choices. */
export function applyLearnedWeights(learned: Weights) {
  set({ profile: "learned", weights: { ...choice.weights, ...learned } });
}

