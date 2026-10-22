import { describe, expect, it } from "vitest";
import {
  calculateCircleGeometry,
  getCardBorderColor,
  getLabelCoordinates,
  getNodeDimensions,
} from "./layout";

describe("layout.ts - pure circle layout geometry", () => {
  it("determina correttamente i colori del bordo per tipo di carta Yu-Gi-Oh", () => {
    expect(getCardBorderColor({ frame: "spell", kind: "spell" })).toBe("#1d9e74");
    expect(getCardBorderColor({ frame: "trap", kind: "trap" })).toBe("#b8457f");
    expect(getCardBorderColor({ frame: "fusion", kind: "monster" })).toBe("#8f5cbf");
    expect(getCardBorderColor({ frame: "synchro", kind: "monster" })).toBe("#d9d9d9");
    expect(getCardBorderColor({ frame: "xyz", kind: "monster" })).toBe("#3b3b3b");
    expect(getCardBorderColor({ frame: "link", kind: "monster" })).toBe("#2f72b8");
    expect(getCardBorderColor({ frame: "ritual", kind: "monster" })).toBe("#5a86c9");
    expect(getCardBorderColor({ frame: "normal", kind: "monster" })).toBe("#d8b46e");
    expect(getCardBorderColor({ frame: "effect", kind: "monster" })).toBe("#cf7a43");
    expect(getCardBorderColor(undefined)).toBe("var(--df-border-strong)");
  });

  it("calcola diametri nodi scalati da 40 a 64px quando c'è spazio abbondante", () => {
    // Spacing chord of 120px gives plenty of room
    const normal = getNodeDimensions(120, 0);
    const keyCard = getNodeDimensions(120, 0.5);

    expect(normal.diameter).toBeLessThanOrEqual(44);
    expect(normal.diameter).toBeGreaterThanOrEqual(28);

    expect(keyCard.diameter).toBe(64);
    expect(keyCard.radius).toBe(32);
    expect(keyCard.diameter).toBeGreaterThan(normal.diameter);
  });

  it("garantisce che nodi adiacenti NON si sovrappongono con 40 carte a 1280x800", () => {
    const cardCount = 40;
    const cards = Array.from({ length: cardCount }, (_, i) => ({
      code: 1000 + i,
      key: i % 5 === 0 ? 0.45 : 0.05, // Some key cards
    }));

    const result = calculateCircleGeometry({
      containerWidth: 860,
      containerHeight: 640,
      cardCount,
      cards,
    });

    expect(result.radius).toBeGreaterThan(160);
    expect(result.spacingChord).toBeGreaterThan(0);

    // Verify distance between all adjacent pairs is strictly greater than sum of radii
    for (let i = 0; i < cardCount; i++) {
      const c1 = cards[i];
      const c2 = cards[(i + 1) % cardCount];
      const pos1 = result.positions.get(c1.code)!;
      const pos2 = result.positions.get(c2.code)!;

      const dist = Math.hypot(pos1.x - pos2.x, pos1.y - pos2.y);
      const sumRadii = pos1.radius + pos2.radius;

      // Distance must exceed sum of radii (no overlapping nodes)
      expect(dist).toBeGreaterThan(sumRadii);
    }
  });

  it("garantisce che nodi adiacenti NON si sovrappongono con 60 carte (mazzo massimale)", () => {
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

  it("adatta radius e viewBox al contenitore reale (dinamico, non fisso)", () => {
    const small = calculateCircleGeometry({
      containerWidth: 500,
      containerHeight: 500,
      cardCount: 20,
    });

    const large = calculateCircleGeometry({
      containerWidth: 1200,
      containerHeight: 900,
      cardCount: 20,
    });

    expect(large.radius).toBeGreaterThan(small.radius);
    expect(small.viewBox).toBe("-250 -250 500 500");
    expect(large.viewBox).toBe("-600 -450 1200 900");
    expect(large.width).toBe(1200);
    expect(large.height).toBe(900);
  });

  it("posiziona i testi delle etichette all'esterno del cerchio e del nodo", () => {
    const pos = {
      x: 100,
      y: 0,
      angle: 0,
      isRight: true,
      radius: 25,
      diameter: 50,
    };

    const label = getLabelCoordinates(pos, 10);
    expect(label.lx).toBe(100 + 25 + 10); // Center dist + radius + offset
    expect(label.textAnchor).toBe("start");

    const posLeft = {
      x: -100,
      y: 0,
      angle: Math.PI,
      isRight: false,
      radius: 25,
      diameter: 50,
    };
    const labelLeft = getLabelCoordinates(posLeft, 10);
    expect(labelLeft.lx).toBe(-(100 + 25 + 10));
    expect(labelLeft.textAnchor).toBe("end");
  });
});
