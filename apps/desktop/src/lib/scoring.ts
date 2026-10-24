import { useSyncExternalStore } from "react";
import type { ComboScore } from "./api";

/**
 * The score of a board: the metrics the probe turn measures, each times a
 * weight the player can change. Interruptions, floodgates and resilience
 * are in "interruptions" (1 = negating any activation once, without a
 * target); resources and follow-up in cards; cost in deck cards, extra deck monsters and LP spent beyond the hand.
 * A card in hand counts once: a measured handtrap in
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
    hint: "Starter ed estensori rimasti in mano (dai ruoli del Laboratorio), ed effetti da Cimitero e bandite nel tuo turno dopo, dopo il board breaker peggiore (0,3 l'uno)",
    color: "var(--metric-followup)",
    explain: [
      "Starter ed estensori rimasti in mano: le carte per ripartire al turno dopo, se l'avversario smonta la board.",
      "I ruoli vengono dal Laboratorio (pannello Ruoli, «Riconosci»): uno starter vale 1, un estensore mezzo.",
      "Conta anche gli effetti da Cimitero e bandite nel tuo turno dopo, dopo il board breaker peggiore (0,3 l'uno).",
    ],
  },
  {
    key: "resources",
    label: "Risorse",
    hint: "Le altre carte rimaste in mano (non handtrap misurate, starter o estensori)",
    color: "var(--metric-resources)",
    explain: ["Le altre carte rimaste in mano. Pesano poco: una carta qualsiasi conta molto meno di un'interruzione."],
  },
  {
    key: "cost",
    label: "Costo",
    hint: "Quello che la linea spende oltre alla mano: carte del Deck pagate come costo (0,05 l'una), mostri dell'Extra Deck finiti nel Cimitero o banditi (0,1 l'uno), LP pagati (1 ogni 4000) (peso negativo)",
    color: "var(--df-faint)",
    explain: [
      "Quello che la linea spende oltre alla mano: carte del Deck pagate come costo (0,05 l'una), mostri dell'Extra Deck finiti nel Cimitero o banditi (0,1 l'uno), LP pagati (1 ogni 4000).",
      "Le carte della mano rimaste contano già nelle altre voci.",
      "Toglie punti: a parità di board, è meglio la linea che spende meno risorse e te ne lascia di più.",
    ],
  },
  {
    key: "exposure",
    label: "Esposizione",
    hint: "Perdita attesa in interruzioni causata dalle handtrap più comuni mentre giochi la linea (peso negativo)",
    color: "var(--df-danger)",
    explain: [
      "La perdita attesa in interruzioni che la linea subisce per mano delle handtrap più usate:",
      "• per ogni tipo di handtrap (Ash Blossom, Effect Veiler / Infinite Impermanence, Nibiru, Droll & Lock Bird), quanto toglie nel momento peggiore per la probabilità che l'avversario ne abbia una;",
      "• l'impatto di Maxx \"C\" e Mulcharmy in base alle Evocazioni Speciali concesse.",
      "Toglie punti: due linee che arrivano alla stessa board non valgono uguale se una si espone a perdere le sue interruzioni. Il profilo «Linee sicure» la pesa di più.",
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
  { id: "balanced", label: "Bilanciato", weights: { interruptions: 1, handTraps: 0.9, floodgates: 0.8, resilience: 0.6, followup: 0.4, resources: 0.1, cost: -0.1, exposure: -0.5 } },
  { id: "negates", label: "Più negazioni", weights: { interruptions: 1.5, handTraps: 1.2, floodgates: 0.5, resilience: 0.4, followup: 0.2, resources: 0.05, cost: -0.05, exposure: -0.4 } },
  { id: "control", label: "Controllo e floodgate", weights: { interruptions: 0.8, handTraps: 0.7, floodgates: 1.6, resilience: 0.6, followup: 0.3, resources: 0.05, cost: -0.1, exposure: -0.4 } },
  { id: "resilient", label: "A prova di board breaker", weights: { interruptions: 0.8, handTraps: 0.9, floodgates: 0.6, resilience: 1.5, followup: 0.5, resources: 0.1, cost: -0.1, exposure: -0.6 } },
  { id: "grind", label: "Risorse e follow-up", weights: { interruptions: 0.7, handTraps: 0.7, floodgates: 0.5, resilience: 0.6, followup: 1.2, resources: 0.3, cost: -0.2, exposure: -0.3 } },
  {
    id: "safe",
    label: "Linee sicure",
    weights: { interruptions: 0.9, handTraps: 0.9, floodgates: 0.7, resilience: 0.6, followup: 0.4, resources: 0.1, cost: -0.15, exposure: -1.0 },
  },
];

export function score(s: ComboScore, w: Weights): number {
  return METRICS.reduce((t, m) => t + w[m.key] * s[m.key], 0);
}

/** Scored with the cheap measure only: no breakers, follow-up, exposure nor matchup, so it is not comparable with a full score. */
export function isCheap(s: ComboScore | null | undefined): boolean {
  return s?.cheap === true;
}

/** The score to keep when `next` arrives: a cheap one never replaces a full one of the same board. */
export function keepFull(prev: ComboScore | undefined, next: ComboScore): ComboScore {
  return prev && !isCheap(prev) && !prev.error && isCheap(next) ? prev : next;
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
export function dominates(a: ComboScore, b: ComboScore, includeMatchup: boolean = false): boolean {
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

  if (includeMatchup) {
    const aRes = a.matchup && !a.matchup.error ? a.matchup.resilience : null;
    const bRes = b.matchup && !b.matchup.error ? b.matchup.resilience : null;
    if (aRes == null || bRes == null) return false;
    if (aRes < bRes - EPS) return false;
    if (aRes > bRes + EPS) strictlyBetter = true;
  }

  return strictlyBetter;
}


export interface Choice {
  version?: number;
  profile: string;
  priorProfile: string;
  weights: Weights;
  priorWeights: Weights;
}

export const WEIGHTS_STORAGE_VERSION = 2;
const KEY = "duelforge.combo.weights";

/** Converts legacy exposure weights (|exposure| < 0.2) to the new scale (x12.5). */
export function migrateLegacyWeights(weights: Weights): Weights {
  if (weights.exposure !== undefined && Math.abs(weights.exposure) > 1e-6 && Math.abs(weights.exposure) < 0.2) {
    return { ...weights, exposure: weights.exposure * 12.5 };
  }
  return weights;
}

export function loadChoice(storage: Pick<Storage, "getItem" | "setItem"> | null = typeof localStorage !== "undefined" ? localStorage : null): Choice {
  try {
    const raw = storage?.getItem(KEY);
    if (raw) {
      const saved = JSON.parse(raw) as Partial<Choice>;
      const isLegacy = (saved.version ?? 1) < 2;
      const baseProfile = PROFILES.find((p) => p.id === saved.priorProfile) ?? PROFILES[0];

      let priorWeights = saved.priorWeights
        ? { ...baseProfile.weights, ...saved.priorWeights }
        : baseProfile.weights;
      if (isLegacy) {
        priorWeights = migrateLegacyWeights(priorWeights);
      }

      let profileResult: Choice;

      if (saved.profile === "learned") {
        let weights = { ...baseProfile.weights, ...saved.weights };
        if (isLegacy) {
          weights = migrateLegacyWeights(weights);
        }
        profileResult = {
          version: WEIGHTS_STORAGE_VERSION,
          profile: "learned",
          priorProfile: saved.priorProfile ?? baseProfile.id,
          weights,
          priorWeights,
        };
      } else {
        const profile = PROFILES.find((p) => p.id === saved.profile);
        if (profile) {
          profileResult = {
            version: WEIGHTS_STORAGE_VERSION,
            profile: profile.id,
            priorProfile: profile.id,
            weights: profile.weights,
            priorWeights: profile.weights,
          };
        } else {
          let weights = { ...PROFILES[0].weights, ...saved.weights };
          if (isLegacy) {
            weights = migrateLegacyWeights(weights);
          }
          profileResult = {
            version: WEIGHTS_STORAGE_VERSION,
            profile: saved.profile ?? "custom",
            priorProfile: saved.priorProfile ?? "custom",
            weights,
            priorWeights,
          };
        }
      }

      // If legacy data was migrated, persist immediately with version: 2
      if (isLegacy && storage) {
        try {
          storage.setItem(KEY, JSON.stringify(profileResult));
        } catch {
          // ignore
        }
      }

      return profileResult;
    }
  } catch {
    // A missing or broken choice falls back to the default.
  }
  return {
    version: WEIGHTS_STORAGE_VERSION,
    profile: PROFILES[0].id,
    priorProfile: PROFILES[0].id,
    weights: PROFILES[0].weights,
    priorWeights: PROFILES[0].weights,
  };
}

let choice = loadChoice();
const listeners = new Set<() => void>();

function set(next: Choice) {
  choice = { ...next, version: WEIGHTS_STORAGE_VERSION };
  try {
    localStorage.setItem(KEY, JSON.stringify(choice));
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
  if (p) {
    set({
      profile: p.id,
      priorProfile: p.id,
      weights: p.weights,
      priorWeights: p.weights,
    });
  }
}

/** A weight changed by hand: the profile becomes "Personalizzato", keeping the original prior intact. */
export function setWeight(key: MetricKey, value: number) {
  const priorProfile = choice.priorProfile ?? "custom";
  const priorWeights = choice.priorWeights ?? PROFILES[0].weights;
  set({
    profile: "custom",
    priorProfile,
    weights: { ...choice.weights, [key]: value },
    priorWeights,
  });
}

/** Activates the weights learned from the player's choices, keeping the original prior intact. */
export function applyLearnedWeights(learned: Partial<Weights>) {
  const isLearned = choice.profile === "learned";
  const priorProfile = isLearned ? choice.priorProfile : choice.profile;
  const priorWeights = isLearned ? choice.priorWeights : choice.weights;
  set({
    profile: "learned",
    priorProfile,
    weights: { ...choice.weights, ...learned },
    priorWeights,
  });
}

/** Returns the prior baseline weights used for Bradley-Terry regression. */
export function getPriorWeights(): Weights {
  return choice.priorWeights ?? PROFILES[0].weights;
}

