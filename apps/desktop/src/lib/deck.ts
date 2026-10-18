import { useCallback, useState } from "react";
import type { Card, Deck, DeckFormat } from "./api";

export type Zone = "main" | "extra" | "side" | "maybe";

export const ZONE_LIMITS: Record<Zone, number> = { main: 60, extra: 15, side: 15, maybe: 30 };
export const ZONE_LABELS: Record<Zone, string> = { main: "Main Deck", extra: "Extra Deck", side: "Side Deck", maybe: "Maybe board" };
export const FORMAT_LABELS: Record<DeckFormat, string> = { md: "Master Duel", tcg: "TCG", ocg: "OCG" };

export function emptyDeck(name = "Nuovo deck", format: DeckFormat = "md"): Deck {
  return { id: null, name, format, main: [], extra: [], side: [], maybe: [], updatedAt: null };
}

export function isExtraDeckCard(card: Card): boolean {
  return card.typeLine.some((t) => t === "Fusion" || t === "Synchro" || t === "Xyz" || t === "Link");
}

/** Copies allowed by the format's Forbidden & Limited list. */
export function copyLimit(card: Card, format: DeckFormat): number {
  const status = format === "tcg" ? card.banTcg : format === "ocg" ? card.banOcg : card.banMd;
  switch (status) {
    case "Banned":
    case "Forbidden":
      return 0;
    case "Limited":
      return 1;
    case "Semi-Limited":
      return 2;
    default:
      return 3;
  }
}

export function copiesInDeck(deck: Deck, id: number): number {
  let n = 0;
  for (const zone of [deck.main, deck.extra, deck.side]) for (const c of zone) if (c === id) n++;
  return n;
}

const SPELL_ORDER = ["Normal", "Quick-Play", "Continuous", "Equip", "Field", "Ritual"];
const TRAP_ORDER = ["Normal", "Continuous", "Counter"];
const EXTRA_ORDER = ["Fusion", "Synchro", "Xyz", "Link"];

/** Master Duel-like order: monsters by level, then Spells and Traps by subtype; Extra by summon type. */
function sortKey(card: Card | undefined): (string | number)[] {
  if (!card) return [9, 0, ""];
  if (card.kind === "monster") {
    const extra = EXTRA_ORDER.findIndex((t) => card.typeLine.includes(t));
    return [0, extra, -(card.level ?? card.linkRating ?? 0), card.name];
  }
  if (card.kind === "spell") return [1, SPELL_ORDER.indexOf(card.race), 0, card.name];
  return [2, TRAP_ORDER.indexOf(card.race), 0, card.name];
}

export function sortIds(ids: number[], getCard: (id: number) => Card | undefined): number[] {
  return [...ids].sort((a, b) => {
    const ka = sortKey(getCard(a));
    const kb = sortKey(getCard(b));
    for (let i = 0; i < ka.length; i++) {
      if (ka[i] < kb[i]) return -1;
      if (ka[i] > kb[i]) return 1;
    }
    return a - b;
  });
}

export type AddResult = { deck: Deck; zone: Zone; index: number } | { error: string };

/**
 * Adds one copy, choosing Main or Extra from the card type unless the Side
 * or the maybe board is asked for. The maybe board has no copy limits.
 */
export function addCard(deck: Deck, card: Card, want: Zone | null, getCard: (id: number) => Card | undefined): AddResult {
  const extraCard = isExtraDeckCard(card);
  if (card.kind === "skill" || card.kind === "token") return { error: `${card.name} non si può mettere in un deck.` };
  if (want === "maybe") {
    if (deck.maybe.length >= ZONE_LIMITS.maybe) return { error: "La Maybe board è piena." };
    const ids = sortIds([...deck.maybe, card.id], getCard);
    return { deck: { ...deck, maybe: ids }, zone: "maybe", index: ids.lastIndexOf(card.id) };
  }
  let zone: Zone = want === "side" ? "side" : extraCard ? "extra" : "main";
  if (want === "extra" && !extraCard) return { error: `${card.name} non è una carta dell'Extra Deck.` };
  if (want === "main" && extraCard) zone = "extra";
  const limit = copyLimit(card, deck.format);
  const copies = copiesInDeck(deck, card.id);
  if (limit === 0) return { error: `${card.name} è proibita in ${FORMAT_LABELS[deck.format]}.` };
  if (copies >= limit) return { error: limit < 3 ? `${card.name} è limitata a ${limit} in ${FORMAT_LABELS[deck.format]}.` : `Puoi avere al massimo 3 copie di ${card.name}.` };
  if (deck[zone].length >= ZONE_LIMITS[zone]) return { error: `Il ${ZONE_LABELS[zone]} è pieno.` };
  const ids = sortIds([...deck[zone], card.id], getCard);
  return { deck: { ...deck, [zone]: ids }, zone, index: ids.lastIndexOf(card.id) };
}

export function removeAt(deck: Deck, zone: Zone, index: number): Deck {
  const ids = [...deck[zone]];
  ids.splice(index, 1);
  return { ...deck, [zone]: ids };
}

export function counts(ids: number[], getCard: (id: number) => Card | undefined) {
  let monsters = 0;
  let spells = 0;
  let traps = 0;
  for (const id of ids) {
    const kind = getCard(id)?.kind;
    if (kind === "monster") monsters++;
    else if (kind === "spell") spells++;
    else if (kind === "trap") traps++;
  }
  return { monsters, spells, traps };
}

/** Master Duel crafting cost: how many cards of each rarity the deck needs. */
export function craftCost(deck: Deck, getCard: (id: number) => Card | undefined) {
  const cost = { ur: 0, sr: 0, r: 0, n: 0 };
  for (const id of [...deck.main, ...deck.extra, ...deck.side]) {
    switch (getCard(id)?.mdRarity) {
      case "Ultra Rare":
        cost.ur++;
        break;
      case "Super Rare":
        cost.sr++;
        break;
      case "Rare":
        cost.r++;
        break;
      case "Common":
        cost.n++;
        break;
    }
  }
  return cost;
}

/** Deck state with undo/redo. */
export function useDeckHistory(initial: Deck) {
  const [state, setState] = useState({ past: [] as Deck[], present: initial, future: [] as Deck[] });
  const set = useCallback((deck: Deck, record = true) => {
    setState((s) => (record ? { past: [...s.past.slice(-99), s.present], present: deck, future: [] } : { ...s, present: deck }));
  }, []);
  const reset = useCallback((deck: Deck) => setState({ past: [], present: deck, future: [] }), []);
  const undo = useCallback(() => setState((s) => (s.past.length ? { past: s.past.slice(0, -1), present: s.past[s.past.length - 1], future: [s.present, ...s.future] } : s)), []);
  const redo = useCallback(() => setState((s) => (s.future.length ? { past: [...s.past, s.present], present: s.future[0], future: s.future.slice(1) } : s)), []);
  return { deck: state.present, set, reset, undo, redo, canUndo: state.past.length > 0, canRedo: state.future.length > 0 };
}
