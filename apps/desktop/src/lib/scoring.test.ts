import { describe, expect, it } from "vitest";
import type { ComboScore } from "./api";
import { METRICS, NEGATIVE, parts, PROFILES, score, type Weights } from "./scoring";

function makeScore(partial: Partial<ComboScore> = {}): ComboScore {
  return {
    hand: partial.hand ?? 1,
    key: partial.key ?? "test-key",
    interruptions: partial.interruptions ?? 0,
    handTraps: partial.handTraps ?? 0,
    floodgates: partial.floodgates ?? 0,
    resilience: partial.resilience ?? 0,
    followup: partial.followup ?? 0,
    resources: partial.resources ?? 0,
    cost: partial.cost ?? 0,
    details: partial.details ?? [],
    floodgateList: partial.floodgateList ?? [],
    breakers: partial.breakers ?? [],
    resourcesNote: partial.resourcesNote ?? "",
    followupNote: partial.followupNote ?? "",
    exposure: partial.exposure ?? 0,
    exposureNote: partial.exposureNote ?? "",
    exposureDetail: partial.exposureDetail ?? null,
    countered: partial.countered ?? null,
    error: partial.error ?? null,
  };
}

describe("scoring", () => {
  it("identifies negative metrics as cost and exposure", () => {
    expect(NEGATIVE).toEqual(["cost", "exposure"]);
  });

  describe("score", () => {
    it("computes weighted sum of metrics", () => {
      const s = makeScore({
        interruptions: 2.0,
        handTraps: 1.0,
        floodgates: 0.5,
        resilience: 1.0,
        followup: 1.0,
        resources: 2.0,
        cost: 3.0,
        exposure: 4.0,
      });

      const w: Weights = {
        interruptions: 1.0,
        handTraps: 0.5,
        floodgates: 0.8,
        resilience: 0.6,
        followup: 0.4,
        resources: 0.1,
        cost: -0.2,
        exposure: -0.1,
      };

      // 2*1 + 1*0.5 + 0.5*0.8 + 1*0.6 + 1*0.4 + 2*0.1 + 3*(-0.2) + 4*(-0.1)
      // = 2 + 0.5 + 0.4 + 0.6 + 0.4 + 0.2 - 0.6 - 0.4 = 3.1
      expect(score(s, w)).toBeCloseTo(3.1, 6);
    });

    it("reduces score when negative metrics increase (with negative weights)", () => {
      const base = makeScore({ interruptions: 2, cost: 1, exposure: 1 });
      const higherCost = makeScore({ interruptions: 2, cost: 5, exposure: 1 });
      const higherExposure = makeScore({ interruptions: 2, cost: 1, exposure: 5 });

      const w: Weights = {
        interruptions: 1,
        handTraps: 0,
        floodgates: 0,
        resilience: 0,
        followup: 0,
        resources: 0,
        cost: -0.5,
        exposure: -0.2,
      };

      expect(score(higherCost, w)).toBeLessThan(score(base, w));
      expect(score(higherExposure, w)).toBeLessThan(score(base, w));
      expect(score(higherCost, w)).toBeCloseTo(score(base, w) - 4 * 0.5, 6);
      expect(score(higherExposure, w)).toBeCloseTo(score(base, w) - 4 * 0.2, 6);
    });

    it("evaluates to 0 when all weights are 0", () => {
      const s = makeScore({ interruptions: 5, cost: 10 });
      const zeroWeights: Weights = {
        interruptions: 0,
        handTraps: 0,
        floodgates: 0,
        resilience: 0,
        followup: 0,
        resources: 0,
        cost: 0,
        exposure: 0,
      };
      expect(score(s, zeroWeights)).toBe(0);
    });

    it("works with builtin presets such as 'balanced'", () => {
      const balanced = PROFILES.find((p) => p.id === "balanced")!;
      expect(balanced).toBeDefined();
      expect(balanced.weights.cost).toBeLessThan(0);
      expect(balanced.weights.exposure).toBeLessThan(0);

      const s = makeScore({
        interruptions: 1,
        handTraps: 1,
        floodgates: 1,
        resilience: 1,
        followup: 1,
        resources: 1,
        cost: 1,
        exposure: 1,
      });

      const expected =
        balanced.weights.interruptions +
        balanced.weights.handTraps +
        balanced.weights.floodgates +
        balanced.weights.resilience +
        balanced.weights.followup +
        balanced.weights.resources +
        balanced.weights.cost +
        balanced.weights.exposure;

      expect(score(s, balanced.weights)).toBeCloseTo(expected, 6);
    });
  });

  describe("parts", () => {
    it("returns each metric contribution matching total score", () => {
      const s = makeScore({ interruptions: 2, cost: 3 });
      const w: Weights = {
        interruptions: 1.5,
        handTraps: 0,
        floodgates: 0,
        resilience: 0,
        followup: 0,
        resources: 0,
        cost: -0.2,
        exposure: 0,
      };

      const breakdown = parts(s, w);
      expect(breakdown).toHaveLength(METRICS.length);
      const totalFromParts = breakdown.reduce((acc, p) => acc + p.value, 0);
      expect(totalFromParts).toBeCloseTo(score(s, w), 6);
    });
  });
});
