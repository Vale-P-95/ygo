import type { CardLink, CardLinkKind } from "../../../lib/api";

import type { LinkCard } from "../../../lib/api";

export type CardLinkZone = CardLink["zone"];

/** Threshold above which a card is considered a key bottleneck hub. */
export const KEY_CARD_THRESHOLD = 0.2;

export interface FormattedChance {
  text: string;
  isEstimated: boolean;
  tooltip: string;
}

/** Formats card reach chance consistently as exact or measured estimate (≈ X%). */
export function formatChance(card: Pick<LinkCard, "chance" | "extra" | "measured">): FormattedChance {
  if (card.extra && card.chance === 0) {
    return {
      text: "—",
      isEstimated: false,
      tooltip: "Non raggiunta nelle mani cercate",
    };
  }
  const pct = Math.round(card.chance * 100);
  if (card.measured) {
    return {
      text: `≈ ${pct}%`,
      isEstimated: true,
      tooltip: "Misurata sulle mani cercate dal motore (±5%)",
    };
  }
  return {
    text: `${pct}%`,
    isEstimated: false,
    tooltip: "Prima stima dai testi: la mano contiene la carta, o una carta che la porta in 1 passo",
  };
}

export interface LinkKindMeta {
  id: CardLinkKind;
  label: string;
  verb: string;
  color: string;
  bg: string;
}

export const LINK_KINDS: Record<CardLinkKind, LinkKindMeta> = {
  search: { id: "search", label: "Aggiunge alla mano", verb: "cerca", color: "var(--role-starter)", bg: "color-mix(in srgb, var(--role-starter) 15%, transparent)" },
  summon: { id: "summon", label: "Evoca Specialmente", verb: "evoca", color: "var(--role-extender)", bg: "color-mix(in srgb, var(--role-extender) 15%, transparent)" },
  set: { id: "set", label: "Posiziona dal Deck", verb: "posiziona", color: "var(--role-custom-2)", bg: "color-mix(in srgb, var(--role-custom-2) 15%, transparent)" },
  toGrave: { id: "toGrave", label: "Manda al Cimitero", verb: "manda al Cimitero", color: "var(--role-brick)", bg: "color-mix(in srgb, var(--role-brick) 15%, transparent)" },
  recover: { id: "recover", label: "Recupera", verb: "recupera", color: "var(--role-breaker)", bg: "color-mix(in srgb, var(--role-breaker) 15%, transparent)" },
  material: { id: "material", label: "Materiale di", verb: "è materiale di", color: "var(--role-custom-1)", bg: "color-mix(in srgb, var(--role-custom-1) 15%, transparent)" },
};

export const LINK_ZONES: Record<CardLinkZone, { label: string; short: string }> = {
  deck: { label: "dal Deck", short: "Deck" },
  extra: { label: "dall'Extra Deck", short: "Extra" },
  grave: { label: "dal Cimitero", short: "Cimitero" },
  banished: { label: "dalle bandite", short: "Bandite" },
  hand: { label: "dalla mano", short: "Mano" },
};

export type LinksLayoutMode = "circle" | "columns";

export interface LinksFilterState {
  kind: CardLinkKind | "all";
  zone: CardLinkZone | "all";
  onlyUsed: boolean;
  searchQuery: string;
}

export type LinkDiffStatus = "normal" | "gained" | "lost";

/** Returns true when a link comes from the card's text alone (engine never offered it). */
export function isLinkTextOnly(link: CardLink): boolean {
  return !link.possible && Boolean(link.text);
}

/** Generates unique key for comparison matching: from-to-kind. */
export function linkKey(link: Pick<CardLink, "from" | "to" | "kind">): string {
  return `${link.from}-${link.to}-${link.kind}`;
}

/** Finds shortest directed link path between two card codes via BFS. */
export function findShortestPath(fromCode: number, toCode: number, allLinks: CardLink[]): CardLink[] | null {
  if (fromCode === toCode) return [];
  const queue: { current: number; path: CardLink[] }[] = [{ current: fromCode, path: [] }];
  const visited = new Set<number>([fromCode]);

  while (queue.length > 0) {
    const { current, path } = queue.shift()!;
    const outgoing = allLinks.filter((l) => l.from === current);
    for (const link of outgoing) {
      if (link.to === toCode) {
        return [...path, link];
      }
      if (!visited.has(link.to)) {
        visited.add(link.to);
        queue.push({ current: link.to, path: [...path, link] });
      }
    }
  }
  return null;
}
