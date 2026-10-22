import type { Card } from "../../../lib/api";

export interface NodePosition {
  x: number;
  y: number;
  angle: number;
  isRight: boolean;
  radius: number;
  diameter: number;
}

export interface CircleGeometryOptions {
  containerWidth: number;
  containerHeight: number;
  cardCount: number;
  cards?: { code: number; key?: number }[];
  minRadius?: number;
  maxRadius?: number;
}

export interface CircleGeometryResult {
  width: number;
  height: number;
  viewBox: string;
  center: { x: number; y: number };
  radius: number;
  spacingChord: number;
  positions: Map<number, NodePosition>;
}

/**
 * Returns Yu-Gi-Oh card type frame color for SVG node borders.
 */
export function getCardBorderColor(card?: { frame?: string; kind?: Card["kind"] } | Card): string {
  if (!card) return "var(--df-border-strong)";
  if (card.frame === "spell" || card.kind === "spell") return "#1d9e74";
  if (card.frame === "trap" || card.kind === "trap") return "#b8457f";
  if (card.frame === "fusion") return "#8f5cbf";
  if (card.frame === "synchro") return "#d9d9d9";
  if (card.frame === "xyz") return "#3b3b3b";
  if (card.frame === "link") return "#2f72b8";
  if (card.frame === "ritual") return "#5a86c9";
  if (card.frame === "normal") return "#d8b46e";
  if (card.frame === "effect") return "#cf7a43";
  if (card.frame === "token") return "#9a9a9a";
  if (card.kind === "monster") return "#cf7a43";
  return "var(--df-border-strong)";
}

/**
 * Computes node diameter (and radius) dynamically based on available chord distance
 * and card importance (key). Guarantees that nodes never overlap even with 40-60 cards.
 */
export function getNodeDimensions(
  spacingChord: number,
  cardKey: number = 0,
): { radius: number; diameter: number } {
  // Safe ceiling to ensure adjacent nodes never touch
  const maxAllowedDiameter = Math.max(18, spacingChord * 0.9);
  
  // Base diameter ranges between 20px and 44px (normal cards)
  const baseDiameter = Math.max(16, Math.min(40, maxAllowedDiameter * 0.72));
  
  // Top key cards scale up towards 64px, bounded by maxAllowedDiameter
  const targetMaxDiameter = Math.min(64, maxAllowedDiameter);
  
  // Key metric scaling: key values >= 0.2 are significant hubs, 0.4+ max bonus
  const keyFactor = Math.min(1, Math.max(0, cardKey / 0.4));
  const diameter = Math.round((baseDiameter + (targetMaxDiameter - baseDiameter) * keyFactor) * 10) / 10;
  const radius = Math.round((diameter / 2) * 10) / 10;

  return { radius, diameter };
}

/**
 * Pure geometric calculation for the circular links graph.
 * Adapts radius and viewBox to actual container dimensions and distributes cards
 * evenly with non-overlapping node sizes.
 */
export function calculateCircleGeometry({
  containerWidth,
  containerHeight,
  cardCount,
  cards = [],
  minRadius = 160,
  maxRadius = 420,
}: CircleGeometryOptions): CircleGeometryResult {
  const width = Math.max(320, containerWidth || 740);
  const height = Math.max(320, containerHeight || 740);

  // Reserve radial margins for labels (longer on horizontal axis) and outer nodes
  const marginX = Math.min(140, Math.max(90, width * 0.16));
  const marginY = Math.min(70, Math.max(45, height * 0.1));

  const maxRx = width / 2 - marginX;
  const maxRy = height / 2 - marginY;
  const computedRadius = Math.round(Math.min(maxRx, maxRy));
  const radius = Math.max(minRadius, Math.min(maxRadius, computedRadius));

  const N = Math.max(1, cardCount);
  const deltaTheta = (2 * Math.PI) / N;

  // Exact Euclidean straight-line distance between centers of two adjacent nodes on the circle
  const spacingChord = 2 * radius * Math.sin(deltaTheta / 2);

  const positions = new Map<number, NodePosition>();

  cards.forEach((c, i) => {
    // Start at top (-PI/2) and rotate clockwise
    const angle = deltaTheta * i - Math.PI / 2;
    const x = Math.round(radius * Math.cos(angle) * 10) / 10;
    const y = Math.round(radius * Math.sin(angle) * 10) / 10;
    const isRight = Math.cos(angle) >= 0;

    const { radius: r, diameter: d } = getNodeDimensions(spacingChord, c.key ?? 0);

    positions.set(c.code, {
      x,
      y,
      angle,
      isRight,
      radius: r,
      diameter: d,
    });
  });

  const halfW = Math.round(width / 2);
  const halfH = Math.round(height / 2);
  const viewBox = `${-halfW} ${-halfH} ${width} ${height}`;

  return {
    width,
    height,
    viewBox,
    center: { x: 0, y: 0 },
    radius,
    spacingChord,
    positions,
  };
}

/**
 * Computes radial label anchor and coordinate offset outside the node.
 */
export function getLabelCoordinates(
  pos: NodePosition,
  extraOffset: number = 8,
): { lx: number; ly: number; textAnchor: "start" | "end" } {
  const dist = Math.hypot(pos.x, pos.y) + pos.radius + extraOffset;
  const lx = Math.round(dist * Math.cos(pos.angle) * 10) / 10;
  const ly = Math.round(dist * Math.sin(pos.angle) * 10) / 10;
  return {
    lx,
    ly,
    textAnchor: pos.isRight ? "start" : "end",
  };
}
