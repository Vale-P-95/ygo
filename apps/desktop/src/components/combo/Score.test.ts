import { describe, expect, it } from "vitest";
import type { ComboScore } from "../../lib/api";
import {
  formatExposureText,
  formatExposureValue,
  formatHandtrapLoss,
  formatMaxxImpact,
  isLegacyExposure,
} from "./Score";

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

describe("Exposure format and losses variants", () => {
  describe("isLegacyExposure", () => {
    it("returns true when exposureDetail is present and losses is absent (legacy)", () => {
      const s = makeScore({
        exposure: 3,
        exposureDetail: {
          ash: 2,
          onField: 1,
          summons: 4,
          specialSummons: 3,
          searches: 1,
        },
      });
      expect(isLegacyExposure(s)).toBe(true);
    });

    it("returns false when exposureDetail has an empty losses array (new format)", () => {
      const s = makeScore({
        exposure: 0.28,
        exposureDetail: {
          ash: 0,
          onField: 0,
          summons: 2,
          specialSummons: 1,
          searches: 0,
          losses: [],
          maxx: 0.28,
          maxxChance: 0.47,
        },
      });
      expect(isLegacyExposure(s)).toBe(false);
    });

    it("returns false when exposureDetail has a non-empty losses array (new format)", () => {
      const s = makeScore({
        exposure: 1.42,
        exposureDetail: {
          ash: 1,
          onField: 1,
          summons: 4,
          specialSummons: 3,
          searches: 1,
          losses: [{ kind: "ash", loss: 1.42, chance: 0.85 }],
        },
      });
      expect(isLegacyExposure(s)).toBe(false);
    });

    it("returns false when exposureDetail is null (new format without detail)", () => {
      const s = makeScore({
        exposure: 1.5,
        exposureDetail: null,
      });
      expect(isLegacyExposure(s)).toBe(false);
    });
  });

  describe("formatExposureText", () => {
    it("case 1: losses absent -> renders legacy choke points wording", () => {
      const s3 = makeScore({
        exposure: 3,
        exposureDetail: {
          ash: 2,
          onField: 1,
          summons: 4,
          specialSummons: 3,
          searches: 1,
        },
      });
      expect(formatExposureText(s3)).toBe("3 punti in cui una handtrap la ferma");

      const s1 = makeScore({
        exposure: 1,
        exposureDetail: {
          ash: 1,
          onField: 0,
          summons: 2,
          specialSummons: 1,
          searches: 0,
        },
      });
      expect(formatExposureText(s1)).toBe("1 punto in cui una handtrap la ferma");

      const s0 = makeScore({
        exposure: 0,
        exposureDetail: {
          ash: 0,
          onField: 0,
          summons: 1,
          specialSummons: 0,
          searches: 0,
        },
      });
      expect(formatExposureText(s0)).toBe("nessun punto critico per le handtrap comuni");
    });

    it("case 2: losses empty -> renders expected loss from other sources or zero loss", () => {
      const sMaxx = makeScore({
        exposure: 0.28,
        exposureDetail: {
          ash: 0,
          onField: 0,
          summons: 2,
          specialSummons: 1,
          searches: 0,
          losses: [],
          maxx: 0.28,
          maxxChance: 0.47,
        },
      });
      expect(formatExposureText(sMaxx)).toBe("perdita attesa per le handtrap: 0,28 interruzioni");

      const sZero = makeScore({
        exposure: 0,
        exposureDetail: {
          ash: 0,
          onField: 0,
          summons: 1,
          specialSummons: 0,
          searches: 0,
          losses: [],
        },
      });
      expect(formatExposureText(sZero)).toBe("nessuna perdita attesa per le handtrap comuni");
    });

    it("case 3: losses populated -> renders expected loss with proper pluralization", () => {
      const sPopulated = makeScore({
        exposure: 1.42,
        exposureDetail: {
          ash: 1,
          onField: 1,
          summons: 4,
          specialSummons: 3,
          searches: 1,
          losses: [{ kind: "ash", loss: 1.42, chance: 0.85 }],
        },
      });
      expect(formatExposureText(sPopulated)).toBe("perdita attesa per le handtrap: 1,42 interruzioni");

      const sOne = makeScore({
        exposure: 1.0,
        exposureDetail: {
          ash: 1,
          onField: 0,
          summons: 3,
          specialSummons: 2,
          searches: 1,
          losses: [{ kind: "ash", loss: 1.0, chance: 0.85 }],
        },
      });
      expect(formatExposureText(sOne)).toBe("perdita attesa per le handtrap: 1,00 interruzione");
    });
  });

  describe("formatters", () => {
    it("formatExposureValue formats singular and plural consistently with 2 decimals", () => {
      expect(formatExposureValue(1)).toBe("1,00 interruzione");
      expect(formatExposureValue(1.42)).toBe("1,42 interruzioni");
      expect(formatExposureValue(0.5)).toBe("0,50 interruzioni");
    });

    it("formatHandtrapLoss formats negative sign, 2 decimals and percent", () => {
      expect(formatHandtrapLoss(1.42, 0.85)).toBe("−1,42 se arriva (85%)");
    });

    it("formatMaxxImpact formats plus sign, 2 decimals, interruzioni, and percent", () => {
      expect(formatMaxxImpact(0.6, 0.47)).toBe("+0,60 interruzioni all'avversario (47%)");
      expect(formatMaxxImpact(1.0, 0.5)).toBe("+1,00 interruzione all'avversario (50%)");
    });
  });
});
