import { describe, expect, it } from "vitest";
import {
  calculateCircleGeometry,
  getCardBorderColor,
  getEdgePath,
  getNodeDimensions,
  getRadialLabelLayout,
} from "./layout";

describe("layout.ts - pure circle layout geometry and edge calculations", () => {
  it("determina correttamente i token CSS dei bordi per tipo di carta Yu-Gi-Oh", () => {
    expect(getCardBorderColor({ frame: "spell", kind: "spell" })).toBe("var(--frame-spell, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "trap", kind: "trap" })).toBe("var(--frame-trap, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "fusion", kind: "monster" })).toBe("var(--frame-fusion, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "synchro", kind: "monster" })).toBe("var(--frame-synchro, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "xyz", kind: "monster" })).toBe("var(--frame-xyz, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "xyz_pendulum", kind: "monster" })).toBe("var(--frame-xyz, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "link", kind: "monster" })).toBe("var(--frame-link, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "normal", kind: "monster" })).toBe("var(--frame-normal, var(--df-border-strong))");
    expect(getCardBorderColor({ frame: "effect", kind: "monster" })).toBe("var(--frame-effect, var(--df-border-strong))");
    expect(getCardBorderColor(undefined)).toBe("var(--df-border-strong)");
  });

  it("calcola percorsi archi che terminano precisamente sul bordo del nodo di destinazione", () => {
    const pFrom = { x: 0, y: -200 };
    const pTo = { x: 200, y: 0 };
    const rFrom = 24;
    const rTo = 28;

    const edge = getEdgePath(pFrom, pTo, rFrom, rTo);

    expect(edge.pathD).toContain("M ");
    expect(edge.pathD).toContain("Q ");

    // Distance from pTo to edge.endPoint must equal rTo + 1 (border boundary)
    const distToEnd = Math.hypot(edge.endPoint.x - pTo.x, edge.endPoint.y - pTo.y);
    expect(Math.abs(distToEnd - (rTo + 1))).toBeLessThan(0.2);

    // Distance from pFrom to edge.startPoint must equal rFrom
    const distFromStart = Math.hypot(edge.startPoint.x - pFrom.x, edge.startPoint.y - pFrom.y);
    expect(Math.abs(distFromStart - rFrom)).toBeLessThan(0.2);
  });

  it("gestisce N <= 1 senza errori né diametri nulli", () => {
    const single = getNodeDimensions(0, 0.4);
    expect(single.diameter).toBeGreaterThan(40);
    expect(single.radius).toBe(single.diameter / 2);

    const geo = calculateCircleGeometry({
      containerWidth: 600,
      containerHeight: 600,
      cardCount: 1,
      cards: [{ code: 1, key: 0.2 }],
    });
    expect(geo.positions.get(1)).toBeDefined();
    expect(geo.positions.get(1)!.radius).toBeGreaterThan(0);
  });

  it("garantisce che nodi adiacenti NON si sovrappongono con 40 carte a 1280x800", () => {
    const cardCount = 40;
    const cards = Array.from({ length: cardCount }, (_, i) => ({
      code: 1000 + i,
      key: i % 5 === 0 ? 0.45 : 0.05,
    }));

    const result = calculateCircleGeometry({
      containerWidth: 860,
      containerHeight: 600,
      cardCount,
      cards,
    });

    expect(result.radius).toBeGreaterThan(140);
    expect(result.spacingChord).toBeGreaterThan(0);

    for (let i = 0; i < cardCount; i++) {
      const c1 = cards[i];
      const c2 = cards[(i + 1) % cardCount];
      const pos1 = result.positions.get(c1.code)!;
      const pos2 = result.positions.get(c2.code)!;

      const dist = Math.hypot(pos1.x - pos2.x, pos1.y - pos2.y);
      const sumRadii = pos1.radius + pos2.radius;

      expect(dist).toBeGreaterThan(sumRadii);
    }
  });

  it("garantisce che nodi adiacenti NON si sovrappongono con 60 carte", () => {
    const cardCount = 60;
    const cards = Array.from({ length: cardCount }, (_, i) => ({
      code: 2000 + i,
      key: i % 4 === 0 ? 0.4 : 0,
    }));

    const result = calculateCircleGeometry({
      containerWidth: 740,
      containerHeight: 560,
      cardCount,
      cards,
    });

    for (let i = 0; i < cardCount; i++) {
      const c1 = cards[i];
      const c2 = cards[(i + 1) % cardCount];
      const pos1 = result.positions.get(c1.code)!;
      const pos2 = result.positions.get(c2.code)!;

      const dist = Math.hypot(pos1.x - pos2.x, pos1.y - pos2.y);
      const sumRadii = pos1.radius + pos2.radius;

      expect(dist).toBeGreaterThan(sumRadii);
    }
  });

  it("orienta le etichette radialmente in modo leggibile senza uscire dal viewBox", () => {
    const posRight = {
      x: 200,
      y: 0,
      angle: 0,
      isRight: true,
      radius: 20,
      diameter: 40,
    };
    const labelRight = getRadialLabelLayout(posRight, 200);
    expect(labelRight.textAnchor).toBe("start");
    expect(labelRight.transform).toContain("rotate(0) translate(228, 4)");

    const posLeft = {
      x: -200,
      y: 0,
      angle: Math.PI,
      isRight: false,
      radius: 20,
      diameter: 40,
    };
    const labelLeft = getRadialLabelLayout(posLeft, 200);
    expect(labelLeft.textAnchor).toBe("end");
    expect(labelLeft.transform).toContain("rotate(360) translate(-228, 4)");
  });
});
