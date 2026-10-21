import type { CardLink, CardLinkKind } from "../../../lib/api";

export type CardLinkZone = CardLink["zone"];

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
