import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** False when the UI runs in a plain browser (vite dev without Tauri). */
export const isDesktop = isTauri();

export type CardKind = "monster" | "spell" | "trap" | "skill" | "token";

/** Mirrors `ygo_data::Card`. */
export interface Card {
  id: number;
  name: string;
  kind: CardKind;
  frame: string;
  humanType: string;
  typeLine: string[];
  desc: string;
  pendDesc: string | null;
  race: string;
  attribute: string | null;
  archetype: string | null;
  level: number | null;
  atk: number | null;
  def: number | null;
  scale: number | null;
  linkRating: number | null;
  linkMarkers: string[];
  mdRarity: string | null;
  formats: string[];
  tcgDate: string | null;
  ocgDate: string | null;
  banTcg: string | null;
  banOcg: string | null;
  /** Master Duel status, from DuelForge's own list. */
  banMd: string | null;
  imageIds: number[];
  views: number;
}

export interface Chip {
  token: string;
  label: string;
}

export interface SearchResult {
  ids: number[];
  chips: Chip[];
  errors: string[];
}

export interface Facets {
  attributes: string[];
  races: string[];
  archetypes: string[];
  formats: string[];
}

export interface DataStatus {
  dataDir: string;
  cards: { version: string; count: number } | null;
  engine: {
    apiVersion: string;
    scripts: number | null;
    scriptsVersion: string | null;
    databases: number;
    cdbVersion: string | null;
  };
  imagesCached: number;
  /** Engine data download failure after an update (cards may still be fine). */
  engineError: string | null;
  /** Effective date (YYYY-MM-DD) of the Master Duel Forbidden & Limited list in use. */
  banlistMd: string | null;
}

export type ProgressItem = "cards" | "scripts" | "cdb";
export type ProgressStage = "checking" | "downloading" | "extracting" | "indexing" | "done" | "upToDate";

export interface Progress {
  item: ProgressItem;
  stage: ProgressStage;
  done: number;
  total: number | null;
}

export type DeckFormat = "md" | "tcg" | "ocg";

/** How a deck looks on the shelf (`ygo_data::DeckBox`); empty means default. */
export interface DeckBox {
  color: string;
  material: string;
  shape: string;
  showcase: number[];
}

/** Mirrors `ygo_data::Deck`. */
export interface Deck {
  id: number | null;
  name: string;
  format: DeckFormat;
  main: number[];
  extra: number[];
  side: number[];
  /** Cards being considered: not part of the deck. */
  maybe: number[];
  /** Cards by role ("starter", "handtrap", ...). */
  roles: Record<string, number[]>;
  box: DeckBox;
  updatedAt: string | null;
}

export interface DeckSummary {
  id: number;
  name: string;
  format: DeckFormat;
  mainCount: number;
  extraCount: number;
  sideCount: number;
  updatedAt: string;
  cover: number | null;
  /** Showcase cards, else the first three Main Deck cards. */
  preview: number[];
  box: DeckBox;
}

/** A deck on the shelf, with its legality under the current lists. */
export interface ShelfDeck extends DeckSummary {
  legal: boolean | null;
  errors: number;
}

export interface Issue {
  severity: "error" | "warning";
  message: string;
  cards: number[];
}

export interface Legality {
  legal: boolean;
  issues: Issue[];
  /** Effective date of the list used (Master Duel). */
  banlist: string | null;
}

/** "2026-10-06" → "6 ottobre 2026". */
export function formatDate(iso: string): string {
  const d = new Date(`${iso}T12:00:00`);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString("it-IT", { day: "numeric", month: "long", year: "numeric" });
}

export interface SavedSearch {
  name: string;
  query: string;
}

export interface Imported {
  main: number[];
  extra: number[];
  side: number[];
  unknown: number[];
}

export const api = {
  dataStatus: () => invoke<DataStatus>("data_status"),
  updateData: (force = false) => invoke<DataStatus>("update_data", { force }),
  search: (query: string) => invoke<SearchResult>("search_cards", { query }),
  getCards: (ids: number[]) => invoke<Card[]>("get_cards", { ids }),
  facets: () => invoke<Facets>("card_facets"),
  listFavorites: () => invoke<number[]>("list_favorites"),
  setFavorite: (id: number, favorite: boolean) => invoke<void>("set_favorite", { id, favorite }),
  listSavedSearches: () => invoke<SavedSearch[]>("list_saved_searches"),
  saveSearch: (name: string, query: string) => invoke<SavedSearch[]>("save_search", { name, query }),
  deleteSavedSearch: (name: string) => invoke<SavedSearch[]>("delete_saved_search", { name }),
  onProgress: (handler: (p: Progress) => void): Promise<UnlistenFn> => listen<Progress>("data-progress", (e) => handler(e.payload)),
  listDecks: () => invoke<DeckSummary[]>("list_decks"),
  shelfDecks: () => invoke<ShelfDeck[]>("shelf_decks"),
  setDeckBox: (id: number, deckBox: DeckBox) => invoke<void>("set_deck_box", { id, deckBox }),
  duplicateDeck: (id: number) => invoke<Deck>("duplicate_deck", { id }),
  getDeck: (id: number) => invoke<Deck | null>("get_deck", { id }),
  saveDeck: (deck: Deck) => invoke<Deck>("save_deck", { deck }),
  deleteDeck: (id: number) => invoke<void>("delete_deck", { id }),
  validateDeck: (deck: Deck) => invoke<Legality>("validate_deck", { deck }),
  importDeck: (text: string) => invoke<Imported>("import_deck", { text }),
  exportDeck: (deck: Deck) => invoke<{ ydk: string; ydke: string }>("export_deck", { deck }),
};

export type ImageSize = "small" | "full" | "cropped";

const windowsHost = typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

/** URL of a card image served (and cached on first use) by the backend. */
export function cardImage(id: number, size: ImageSize): string {
  return windowsHost ? `http://cardimg.localhost/${size}/${id}` : `cardimg://localhost/${size}/${id}`;
}
