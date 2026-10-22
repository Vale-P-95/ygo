import { describe, expect, it } from "vitest";
import { pAtLeastOne, pBoth, percent, pNone } from "./probability";

describe("probability", () => {
  describe("pNone", () => {
    it("matches hand-calculated value for 3 copies in 40 cards with hand of 5", () => {
      // P(none) = (37/40) * (36/39) * (35/38) * (34/37) * (33/36) = 1309 / 1976 ≈ 0.662449
      const expected = 1309 / 1976;
      expect(pNone(40, 3, 5)).toBeCloseTo(expected, 6);
    });

    it("matches single copy probability in 40 cards with hand of 5", () => {
      // P(none) = 35/40 = 0.875
      expect(pNone(40, 1, 5)).toBeCloseTo(0.875, 6);
    });

    it("handles 0 copies in deck", () => {
      expect(pNone(40, 0, 5)).toBe(1);
    });

    it("handles hand size of 0", () => {
      expect(pNone(40, 3, 0)).toBe(1);
    });

    it("handles hand size equal to deck size", () => {
      expect(pNone(40, 3, 40)).toBe(0);
      expect(pNone(40, 0, 40)).toBe(1);
    });

    it("handles hand size larger than deck size", () => {
      expect(pNone(40, 3, 45)).toBe(0);
      expect(pNone(40, 0, 45)).toBe(1);
    });

    it("handles all cards being hits", () => {
      expect(pNone(40, 40, 5)).toBe(0);
    });
  });

  describe("pAtLeastOne", () => {
    it("matches hand-calculated value for 3 copies in 40 cards with hand of 5 (≈ 0.3376)", () => {
      // 1 - C(37,5)/C(40,5) = 1 - 1309/1976 = 667/1976 ≈ 0.33755
      const expected = 667 / 1976;
      expect(pAtLeastOne(40, 3, 5)).toBeCloseTo(expected, 5);
      expect(pAtLeastOne(40, 3, 5)).toBeCloseTo(0.3376, 3);
    });

    it("returns 0 when hits is 0", () => {
      expect(pAtLeastOne(40, 0, 5)).toBe(0);
    });

    it("returns 1 when hand size equals deck size with hits > 0", () => {
      expect(pAtLeastOne(40, 1, 40)).toBe(1);
    });

    it("returns 0 when hand size is 0", () => {
      expect(pAtLeastOne(40, 3, 0)).toBe(0);
    });
  });

  describe("pBoth", () => {
    it("calculates joint probability for disjoint groups", () => {
      // 40 cards, 3 in A, 3 in B, disjoint (either = 6), drawing 5:
      // pBoth = 1 - pNone(40,3,5) - pNone(40,3,5) + pNone(40,6,5)
      const pNone3 = 1309 / 1976;
      const pNone6 = (34 * 33 * 32 * 31 * 30) / (40 * 39 * 38 * 37 * 36);
      const expected = 1 - 2 * pNone3 + pNone6;
      expect(pBoth(40, 3, 3, 6, 5)).toBeCloseTo(expected, 6);
    });

    it("calculates joint probability for overlapping groups", () => {
      // 40 cards, 3 in A, 3 in B, 1 card belongs to both -> either = 5
      const pNoneA = pNone(40, 3, 5);
      const pNoneB = pNone(40, 3, 5);
      const pNoneEither = pNone(40, 5, 5);
      const expected = 1 - pNoneA - pNoneB + pNoneEither;
      expect(pBoth(40, 3, 3, 5, 5)).toBeCloseTo(expected, 6);
    });

    it("returns 0 when either group has 0 copies", () => {
      expect(pBoth(40, 0, 3, 3, 5)).toBe(0);
      expect(pBoth(40, 3, 0, 3, 5)).toBe(0);
    });
  });

  describe("percent", () => {
    it("formats probabilities as Italian percentage string", () => {
      expect(percent(0.3376)).toBe("33,8%");
      expect(percent(1)).toBe("100,0%");
      expect(percent(0)).toBe("0,0%");
    });
  });
});
