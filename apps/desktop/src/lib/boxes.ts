/** Deck box looks for the shelf (La Bottega). */
import type { DeckBox } from "./api";

export type Material = "leather" | "metal" | "wood" | "holo" | "matte" | "cardboard";
export type Shape = "box" | "tin" | "chest";

export const PALETTE: { color: string; name: string }[] = [
  { color: "#8e2a22", name: "Rosso brace" },
  { color: "#c4571c", name: "Rame" },
  { color: "#d6a53a", name: "Oro" },
  { color: "#5b6b2e", name: "Muschio" },
  { color: "#1f6b5a", name: "Smeraldo" },
  { color: "#23607f", name: "Acciaio blu" },
  { color: "#2b3b8f", name: "Zaffiro" },
  { color: "#5d2f86", name: "Ametista" },
  { color: "#8c2f63", name: "Rubino" },
  { color: "#e6dccb", name: "Avorio" },
  { color: "#6b6f78", name: "Grafite" },
  { color: "#1b1a1f", name: "Ossidiana" },
];

export const MATERIALS: { id: Material; name: string }[] = [
  { id: "leather", name: "Pelle" },
  { id: "metal", name: "Metallo" },
  { id: "wood", name: "Legno" },
  { id: "holo", name: "Olografico" },
  { id: "matte", name: "Opaco" },
  { id: "cardboard", name: "Cartoncino" },
];

export const SHAPES: { id: Shape; name: string; hint: string }[] = [
  { id: "box", name: "Deck box", hint: "verticale, con il coperchio aperto" },
  { id: "tin", name: "Latta", hint: "larga e arrotondata" },
  { id: "chest", name: "Cofanetto", hint: "profondo, con fermaglio" },
];

/** Box size in pixels: width, height, depth. */
export const SIZES: Record<Shape, { w: number; h: number; d: number }> = {
  box: { w: 132, h: 176, d: 64 },
  tin: { w: 176, h: 124, d: 58 },
  chest: { w: 176, h: 130, d: 100 },
};

/** A stable pick from the palette for decks that never chose a color. */
function defaultColor(id: number | null): string {
  return PALETTE[Math.abs(id ?? 0) % PALETTE.length].color;
}

export interface Look {
  color: string;
  material: Material;
  shape: Shape;
}

export function resolveLook(box: DeckBox | undefined, id: number | null): Look {
  return {
    color: box?.color || defaultColor(id),
    material: MATERIALS.some((m) => m.id === box?.material) ? (box!.material as Material) : "leather",
    shape: SHAPES.some((s) => s.id === box?.shape) ? (box!.shape as Shape) : "box",
  };
}
