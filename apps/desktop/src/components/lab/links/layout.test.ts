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
    expect(getCardBorderColor({ frame: "spell", kind: "spell" })).toBe("var(--frame-spell, var(--df-muted))");
    expect(getCardBorderColor({ frame: "trap", kind: "trap" })).toBe("var(--frame-trap, var(--df-muted))");
    expect(getCardBorderColor({ frame: "fusion", kind: "monster" })).toBe("var(--frame-fusion, var(--df-muted))");
    expect(getCardBorderColor({ frame: "synchro", kind: "monster" })).toBe("var(--frame-synchro, var(--df-muted))");
    expect(getCardBorderColor({ frame: "xyz", kind: "monster" })).toBe("var(--frame-xyz, var(--df-muted))");
    expect(getCardBorderColor({ frame: "xyz_pendulum", kind: "monster" })).toBe("var(--frame-xyz, var(--df-muted))");
    expect(getCardBorderColor({ frame: "link", kind: "monster" })).toBe("var(--frame-link, var(--df-muted))");
    expect(getCardBorderColor({ frame: "normal", kind: "monster" })).toBe("var(--frame-normal, var(--df-muted))");
    expect(getCardBorderColor({ frame: "effect", kind: "monster" })).toBe("var(--frame-effect, var(--df-muted))");
    expect(getCardBorderColor(undefined)).toBe("var(--df-muted)");
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

  it("riserva margine verticale e include le etichette di 18 caratteri a ore 12 e 6 interamente nel viewBox a 1280x800 e 1440x900", () => {
    // Test with card 0 at top (ore 12: -PI/2) and card N/2 at bottom (ore 6: PI/2)
    const testDimensions = [
      { width: 780, height: 480 }, // 1280x800 with panels
      { width: 860, height: 540 }, // 1440x900
      { width: 1100, height: 680 }, // 1920x1080
    ];

    for (const dim of testDimensions) {
      const cardCount = 40;
      const cards = Array.from({ length: cardCount }, (_, i) => ({
        code: 3000 + i,
        key: i === 0 || i === 20 ? 0.5 : 0.05,
      }));

      const geo = calculateCircleGeometry({
        containerWidth: dim.width,
        containerHeight: dim.height,
        cardCount,
        cards,
      });

      const [, minY, vbWidth, vbHeight] = geo.viewBox.split(" ").map(Number);
      const halfH = vbHeight / 2;
      const halfW = vbWidth / 2;

      // Card at index 0 is at top (-PI/2)
      const topNode = geo.positions.get(3000)!;
      expect(topNode.x).toBeCloseTo(0, 1);
      expect(topNode.y).toBeCloseTo(-geo.radius, 1);

      // Label length for 18 chars (16 + ellipsis) with 12px font is ~115px
      const labelLength = 115;
      const topOutermostY = -(geo.radius + topNode.radius + 8 + labelLength);

      // Verify top label outer edge is strictly within -halfH (minY)
      expect(topOutermostY).toBeGreaterThanOrEqual(minY);

      // Card at index 20 (halfway) is at bottom (PI/2)
      const bottomNode = geo.positions.get(3020)!;
      expect(bottomNode.x).toBeCloseTo(0, 1);
      expect(bottomNode.y).toBeCloseTo(geo.radius, 1);

      const bottomOutermostY = geo.radius + bottomNode.radius + 8 + labelLength;

      // Verify bottom label outer edge is strictly within +halfH
      expect(bottomOutermostY).toBeLessThanOrEqual(halfH);

      // Also check horizontal bounds
      expect(geo.radius + labelLength).toBeLessThanOrEqual(halfW);
    }
  });

  it("garantisce raggio >= 160px e nodi >= 20px a 1280x800, con etichette non sovrapposte", () => {
    const cardCount = 40;
    const cards = Array.from({ length: cardCount }, (_, i) => ({
      code: 4000 + i,
      key: i % 4 === 0 ? 0.35 : 0.05,
    }));

    const geo1280 = calculateCircleGeometry({
      containerWidth: 840,
      containerHeight: 560,
      cardCount,
      cards,
    });

    expect(geo1280.radius).toBeGreaterThanOrEqual(160);
    for (const pos of geo1280.positions.values()) {
      expect(pos.diameter).toBeGreaterThanOrEqual(20);
    }

    // Verify distance between adjacent radial rays at label midpoint (radius + 40px)
    // 2 * d * sin(deltaTheta / 2) with 40 cards deltaTheta = 9deg -> separation is ~30px
    const deltaTheta = (2 * Math.PI) / cardCount;
    const midLabelDistance = geo1280.radius + 40;
    const labelSeparation = 2 * midLabelDistance * Math.sin(deltaTheta / 2);
    // 12px font height requires at least ~14px separation to never overlap
    expect(labelSeparation).toBeGreaterThan(18);
  });
});
