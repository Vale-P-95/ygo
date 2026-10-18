import { useSyncExternalStore } from "react";
import { api, isDesktop } from "./api";
import { sound } from "./sound";
import { toast } from "./toast";

/** Favourite cards, shared by every view; stored in user.db. */
let favorites = new Set<number>();
let loaded = false;
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((l) => l());
}

function load() {
  if (loaded || !isDesktop) return;
  loaded = true;
  api
    .listFavorites()
    .then((ids) => {
      favorites = new Set(ids);
      emit();
    })
    .catch(() => {
      loaded = false;
    });
}

function subscribe(listener: () => void) {
  load();
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useFavorites(): Set<number> {
  return useSyncExternalStore(subscribe, () => favorites);
}

export function toggleFavorite(id: number) {
  const on = !favorites.has(id);
  const next = new Set(favorites);
  if (on) next.add(id);
  else next.delete(id);
  favorites = next;
  emit();
  sound.play(on ? "chime" : "toggle");
  api.setFavorite(id, on).catch((e) => {
    toast(`Preferiti non salvati: ${e}`, "error");
  });
}
