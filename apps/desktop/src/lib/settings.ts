import { useSyncExternalStore } from "react";

export type Theme = "forge" | "light" | "system";

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
  hoverPreview: boolean;
}

const defaults: Settings = {
  theme: "forge",
  sound: true,
  masterVolume: 0.7,
  uiVolume: 0.6,
  fxVolume: 0.8,
  ambient: true,
  cardEffects: true,
  hoverPreview: true,
};

const KEY = "duelforge.settings";

function load(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? { ...defaults, ...JSON.parse(raw) } : defaults;
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
