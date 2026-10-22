import type { Card } from "../../../lib/api";
import { KEY_CARD_THRESHOLD } from "./types";

export interface NodePosition {
  x: number;
  y: number;
  angle: number;
  isRight: boolean;
  radius: number;
  diameter: number;
}

export interface EdgePathResult {
  pathD: string;
  startPoint: { x: number; y: number };
  endPoint: { x: number; y: number };
  cp: { x: number; y: number };
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
 * Returns Yu-Gi-Oh card type frame token for SVG node borders.
 * Uses shared CSS custom properties defined in src/index.css.
 */
export function getCardBorderColor(card?: { frame?: string; kind?: Card["kind"] } | Card): string {
  if (!card) return "var(--df-muted)";
  if (card.frame) {
    const base = card.frame.split("_")[0];
    return `var(--frame-${base}, var(--df-muted))`;
  }
  if (card.kind === "spell") return "var(--frame-spell, var(--df-muted))";
  if (card.kind === "trap") return "var(--frame-trap, var(--df-muted))";
  if (card.kind === "monster") return "var(--frame-effect, var(--df-muted))";
  return "var(--df-muted)";
}

/**
 * Computes node diameter (and radius) dynamically based on available chord distance
 * and card importance (key). Guarantees that nodes never overlap even with 40-60 cards.
 */
export function getNodeDimensions(
  spacingChord: number,
  cardKey: number = 0,
): { radius: number; diameter: number } {
  // Safe single-card fallback when chord is not applicable
  if (spacingChord <= 0) {
    const d = Math.round(48 + Math.min(16, cardKey * 32));
    return { radius: d / 2, diameter: d };
  }

  // Safe ceiling to ensure adjacent nodes never touch (at least 12% safety margin)
  const maxAllowedDiameter = Math.max(8, spacingChord * 0.88);

  // Base diameter for normal cards
  const baseDiameter = Math.max(6, Math.min(40, maxAllowedDiameter * 0.72));

  // Top key cards scale up towards 64px, strictly bounded by maxAllowedDiameter
  const targetMaxDiameter = Math.min(64, maxAllowedDiameter);

  // Scale bonus with key metric threshold
  const keyFactor = Math.min(1, Math.max(0, cardKey / KEY_CARD_THRESHOLD));
  const diameter = Math.round((baseDiameter + (targetMaxDiameter - baseDiameter) * keyFactor) * 10) / 10;
  const radius = Math.round((diameter / 2) * 10) / 10;

  return { radius, diameter };
}

/**
 * Computes exact bezier path between two nodes that starts at the source node's perimeter
 * and ends precisely on the target node's border, leaving the arrow marker completely
 * visible outside the node's artwork and border.
 */
export function getEdgePath(
  pFrom: { x: number; y: number },
  pTo: { x: number; y: number },
  rFrom: number,
  rTo: number,
  kPull: number = 0.35,
): EdgePathResult {
  const cp = {
    x: ((pFrom.x + pTo.x) / 2) * kPull,
    y: ((pFrom.y + pTo.y) / 2) * kPull,
  };

  // Vector leaving pFrom towards cp
  const sdx = cp.x - pFrom.x;
  const sdy = cp.y - pFrom.y;
  const slen = Math.hypot(sdx, sdy) || 1;
  const startPoint = {
    x: Math.round((pFrom.x + (sdx / slen) * rFrom) * 10) / 10,
    y: Math.round((pFrom.y + (sdy / slen) * rFrom) * 10) / 10,
  };

  // Vector entering pTo from cp
  const edx = pTo.x - cp.x;
  const edy = pTo.y - cp.y;
  const elen = Math.hypot(edx, edy) || 1;
  // Stop at border of target node (with 1px margin so arrow tip touches outer border)
  const endPoint = {
    x: Math.round((pTo.x - (edx / elen) * (rTo + 1)) * 10) / 10,
    y: Math.round((pTo.y - (edy / elen) * (rTo + 1)) * 10) / 10,
  };

  const pathD = `M ${startPoint.x} ${startPoint.y} Q ${cp.x} ${cp.y} ${endPoint.x} ${endPoint.y}`;

  return {
    pathD,
    startPoint,
    endPoint,
    cp,
  };
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
  minRadius = 100,
  maxRadius = 300,
}: CircleGeometryOptions): CircleGeometryResult {
  const width = Math.max(320, containerWidth || 740);
  const height = Math.max(320, containerHeight || 620);

  // Radial label length is at most 18 chars (~115px) + max node radius (up to 32px) + 8px gap = ~155px
  // Reserving sufficient radial margin ensures labels at top/bottom (12 and 6 o'clock) are never clipped
  const labelReserve = 145;
  const marginX = Math.max(labelReserve, Math.round(width * 0.18));
  const marginY = Math.max(labelReserve, Math.round(height * 0.22));

  const maxRx = width / 2 - marginX;
  const maxRy = height / 2 - marginY;
  const computedRadius = Math.round(Math.min(maxRx, maxRy));
  const effectiveMinRadius = Math.min(minRadius, Math.max(60, computedRadius));
  const radius = Math.max(effectiveMinRadius, Math.min(maxRadius, computedRadius));

  const N = Math.max(1, cardCount);
  const deltaTheta = N > 1 ? (2 * Math.PI) / N : 0;

  // Exact Euclidean straight-line distance between centers of two adjacent nodes on the circle
  const spacingChord = N > 1 ? 2 * radius * Math.sin(deltaTheta / 2) : 0;

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

  // Ensure viewBox always encompasses the outermost label extent plus a safe margin
  const maxExtentX = radius + labelReserve + 10;
  const maxExtentY = radius + labelReserve + 10;
  const halfW = Math.max(Math.round(width / 2), Math.round(maxExtentX));
  const halfH = Math.max(Math.round(height / 2), Math.round(maxExtentY));
  const viewBox = `${-halfW} ${-halfH} ${halfW * 2} ${halfH * 2}`;

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
 * Computes radial text transform and anchor so card labels radiate cleanly outwards
 * along the ray through the node center without overlapping adjacent labels.
 */
export function getRadialLabelLayout(
  pos: NodePosition,
  radius: number,
): { transform: string; textAnchor: "start" | "end" } {
  const deg = Math.round(((pos.angle * 180) / Math.PI) * 10) / 10;
  const dist = Math.round((radius + pos.radius + 8) * 10) / 10;

  if (pos.isRight) {
    return {
      transform: `rotate(${deg}) translate(${dist}, 4)`,
      textAnchor: "start",
    };
  }

  // Flip text by 180deg on the left hemisphere so it reads upright from left to right
  return {
    transform: `rotate(${deg + 180}) translate(${-dist}, 4)`,
    textAnchor: "end",
  };
}
