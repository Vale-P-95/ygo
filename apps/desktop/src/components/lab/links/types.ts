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
  search: { id: "search", label: "Aggiunge alla mano", verb: "cerca", color: "#10b981", bg: "rgba(16, 185, 129, 0.15)" },
  summon: { id: "summon", label: "Evoca Specialmente", verb: "evoca", color: "#f59e0b", bg: "rgba(245, 158, 11, 0.15)" },
  toGrave: { id: "toGrave", label: "Manda al Cimitero", verb: "manda al GY", color: "#ef4444", bg: "rgba(239, 68, 68, 0.15)" },
  recover: { id: "recover", label: "Recupera", verb: "recupera", color: "#06b6d4", bg: "rgba(6, 182, 212, 0.15)" },
  material: { id: "material", label: "Materiale di", verb: "è materiale di", color: "#8b5cf6", bg: "rgba(139, 92, 246, 0.15)" },
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
