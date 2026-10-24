import { describe, expect, it } from "vitest";
import type { ComboScore } from "./api";
import { isCheap, keepFull, METRICS, NEGATIVE, parts, PROFILES, score, migrateLegacyWeights, loadChoice, WEIGHTS_STORAGE_VERSION, type Weights } from "./scoring";

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
    cheap: partial.cheap,
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

  describe("PROFILES exposure weights (D17)", () => {
    it("assigns calibrated exposure weights in the new scale", () => {
      const getExposure = (id: string) => PROFILES.find((p) => p.id === id)!.weights.exposure;

      expect(getExposure("balanced")).toBeCloseTo(-0.5, 6);
      expect(getExposure("negates")).toBeCloseTo(-0.4, 6);
      expect(getExposure("control")).toBeCloseTo(-0.4, 6);
      expect(getExposure("resilient")).toBeCloseTo(-0.6, 6);
      expect(getExposure("grind")).toBeCloseTo(-0.3, 6);
      expect(getExposure("safe")).toBeCloseTo(-1.0, 6);
    });

    it("ranks board with lower exposure higher when other metrics are equal", () => {
      const balanced = PROFILES.find((p) => p.id === "balanced")!;
      const saferBoard = makeScore({ interruptions: 2, resilience: 1, exposure: 0.5 });
      const riskierBoard = makeScore({ interruptions: 2, resilience: 1, exposure: 1.8 });

      expect(score(saferBoard, balanced.weights)).toBeGreaterThan(score(riskierBoard, balanced.weights));
    });
  });

  describe("migrateLegacyWeights", () => {
    const baseWeights: Weights = {
      interruptions: 1,
      handTraps: 0.9,
      floodgates: 0.8,
      resilience: 0.6,
      followup: 0.4,
      resources: 0.1,
      cost: -0.1,
      exposure: -0.04,
    };

    it("multiplies legacy exposure (|exposure| < 0.2) by 12.5", () => {
      const migrated = migrateLegacyWeights({ ...baseWeights, exposure: -0.04 });
      expect(migrated.exposure).toBeCloseTo(-0.5, 2);

      const migratedNegates = migrateLegacyWeights({ ...baseWeights, exposure: -0.03 });
      expect(migratedNegates.exposure).toBeCloseTo(-0.375, 3);

      const migratedCustom = migrateLegacyWeights({ ...baseWeights, exposure: -0.08 });
      expect(migratedCustom.exposure).toBeCloseTo(-1.0, 2);
    });

    it("leaves already-migrated exposure (>= 0.2 in magnitude) unchanged", () => {
      const current = migrateLegacyWeights({ ...baseWeights, exposure: -0.5 });
      expect(current.exposure).toBe(-0.5);

      const safe = migrateLegacyWeights({ ...baseWeights, exposure: -1.0 });
      expect(safe.exposure).toBe(-1.0);
    });

    it("leaves zero exposure unchanged", () => {
      const zero = migrateLegacyWeights({ ...baseWeights, exposure: 0 });
      expect(zero.exposure).toBe(0);
    });
  });

  describe("loadChoice with versioning", () => {
    function createMockStorage(initialData?: string) {
      let store: string | null = initialData ?? null;
      return {
        getItem: (key: string) => (key === "duelforge.combo.weights" ? store : null),
        setItem: (key: string, val: string) => {
          if (key === "duelforge.combo.weights") store = val;
        },
      };
    }

    it("converts legacy data without version (-0.04 -> -0.5) and writes back version 2", () => {
      const storage = createMockStorage(
        JSON.stringify({
          profile: "custom",
          weights: { ...PROFILES[0].weights, exposure: -0.04 },
        })
      );

      const choice = loadChoice(storage);
      expect(choice.version).toBe(WEIGHTS_STORAGE_VERSION);
      expect(choice.weights.exposure).toBeCloseTo(-0.5, 2);

      // Verify it was written back to storage with version: 2
      const stored = JSON.parse(storage.getItem("duelforge.combo.weights")!);
      expect(stored.version).toBe(2);
      expect(stored.weights.exposure).toBeCloseTo(-0.5, 2);
    });

    it("leaves data with version 2 and exposure -0.1 unchanged", () => {
      const storage = createMockStorage(
        JSON.stringify({
          version: 2,
          profile: "custom",
          weights: { ...PROFILES[0].weights, exposure: -0.1 },
        })
      );

      const choice = loadChoice(storage);
      expect(choice.version).toBe(2);
      expect(choice.weights.exposure).toBe(-0.1);
    });

    it("loading legacy data twice in a row gives -0.5, not -6.25", () => {
      const storage = createMockStorage(
        JSON.stringify({
          profile: "custom",
          weights: { ...PROFILES[0].weights, exposure: -0.04 },
        })
      );

      const firstLoad = loadChoice(storage);
      expect(firstLoad.weights.exposure).toBeCloseTo(-0.5, 2);

      const secondLoad = loadChoice(storage);
      expect(secondLoad.weights.exposure).toBeCloseTo(-0.5, 2);
      expect(secondLoad.weights.exposure).not.toBeCloseTo(-6.25, 2);
    });

    it("leaves learned weights with version 2 unchanged", () => {
      const storage = createMockStorage(
        JSON.stringify({
          version: 2,
          profile: "learned",
          priorProfile: "balanced",
          weights: { ...PROFILES[0].weights, exposure: -0.08 },
        })
      );

      const choice = loadChoice(storage);
      expect(choice.version).toBe(2);
      expect(choice.profile).toBe("learned");
      expect(choice.weights.exposure).toBe(-0.08);
    });
  });
});

describe("cheap scores", () => {
  it("never replace a full score of the same board", () => {
    const full = makeScore({ interruptions: 3 });
    const cheap = makeScore({ interruptions: 5, cheap: true });
    expect(isCheap(cheap)).toBe(true);
    expect(keepFull(full, cheap)).toBe(full);
    expect(keepFull(cheap, full)).toBe(full);
    expect(keepFull(undefined, cheap)).toBe(cheap);
    expect(keepFull(makeScore({ error: "x" }), cheap)).toBe(cheap);
  });
});
