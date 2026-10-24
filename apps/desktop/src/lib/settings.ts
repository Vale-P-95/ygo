import { useSyncExternalStore } from "react";

export type Theme = "forge" | "light" | "system";

/** Deck editor catalog: large, medium or small cards, or a list. */
export type CatalogView = "l" | "m" | "s" | "list";

export interface Settings {
  theme: Theme;
  sound: boolean;
  /** 0..1 */
  masterVolume: number;
  uiVolume: number;
  fxVolume: number;
  /** Rising embers behind the content. */
  ambient: boolean;
  /** Tilt, holo foil and sparks on cards. */
  cardEffects: boolean;
  /** Fade between pages. */
  pageTransitions: boolean;
  /** Staggered entrances of lists, boards and results. */
  listAnimations: boolean;
  /** Sway, foil and lift of the deck boxes on the shelf. */
  shelfEffects: boolean;
  /** Pulsing links on the links map. */
  linksEffects: boolean;
  /** Hover, click, tab and page sounds. */
  soundUi: boolean;
  /** Search done, legal deck, errors, lab sounds. */
  soundEvents: boolean;
  /** Cards and boxes set down on the shelf. */
  soundShelf: boolean;
  hoverPreview: boolean;
  catalogView: CatalogView;
  /** Width of the deck editor catalog column, in pixels. */
  catalogWidth: number;
}

const defaults: Settings = {
  theme: "forge",
  sound: true,
  masterVolume: 0.7,
  uiVolume: 0.6,
  fxVolume: 0.8,
  ambient: true,
  cardEffects: true,
  pageTransitions: true,
  listAnimations: true,
  shelfEffects: true,
  linksEffects: true,
  soundUi: true,
  soundEvents: true,
  soundShelf: true,
  hoverPreview: true,
  catalogView: "m",
  catalogWidth: 392,
};

const KEY = "duelforge.settings";

/** Saved settings over the defaults; a field of the wrong type falls back, so older saves keep working. */
export function mergeSettings(saved: unknown): Settings {
  if (!saved || typeof saved !== "object") return defaults;
  const out: Record<string, unknown> = { ...defaults };
  for (const [k, v] of Object.entries(saved as Record<string, unknown>)) {
    if (k in defaults && typeof v === typeof (defaults as unknown as Record<string, unknown>)[k]) out[k] = v;
  }
  return out as unknown as Settings;
}

function load(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? mergeSettings(JSON.parse(raw)) : defaults;
  } catch {
    return defaults;
  }
}

let current = load();
const listeners = new Set<() => void>();

export function getSettings(): Settings {
  return current;
}

export function updateSettings(patch: Partial<Settings>) {
  current = { ...current, ...patch };
  try {
    localStorage.setItem(KEY, JSON.stringify(current));
  } catch {
    // Storage may be unavailable; settings then last for the session.
  }
  listeners.forEach((l) => l());
}

export function subscribeSettings(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useSettings(): Settings {
  return useSyncExternalStore(subscribeSettings, getSettings);
}

export function prefersReducedMotion(): boolean {
  return typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}
