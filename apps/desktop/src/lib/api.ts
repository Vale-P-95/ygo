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
}

export interface Issue {
  severity: "error" | "warning";
  message: string;
  cards: number[];
}

export interface Legality {
  legal: boolean;
  issues: Issue[];
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
  onProgress: (handler: (p: Progress) => void): Promise<UnlistenFn> => listen<Progress>("data-progress", (e) => handler(e.payload)),
  listDecks: () => invoke<DeckSummary[]>("list_decks"),
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
