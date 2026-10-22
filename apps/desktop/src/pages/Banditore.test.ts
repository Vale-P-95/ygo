import { describe, expect, it } from "vitest";
import { getChangeType, normalizeBanStatus } from "./Banditore";

describe("Banditore", () => {
  describe("normalizeBanStatus", () => {
    it("maps forbidden synonyms including 'Banned' and Italian terms to 'Forbidden'", () => {
      expect(normalizeBanStatus("Forbidden")).toBe("Forbidden");
      expect(normalizeBanStatus("forbidden")).toBe("Forbidden");
      expect(normalizeBanStatus("Banned")).toBe("Forbidden");
      expect(normalizeBanStatus("banned")).toBe("Forbidden");
      expect(normalizeBanStatus("Proibita")).toBe("Forbidden");
      expect(normalizeBanStatus("proibite")).toBe("Forbidden");
    });

    it("maps limited synonyms to 'Limited'", () => {
      expect(normalizeBanStatus("Limited")).toBe("Limited");
      expect(normalizeBanStatus("limited")).toBe("Limited");
      expect(normalizeBanStatus("Limitata")).toBe("Limited");
      expect(normalizeBanStatus("limitate")).toBe("Limited");
    });

    it("maps semi-limited synonyms to 'Semi-Limited'", () => {
      expect(normalizeBanStatus("Semi-Limited")).toBe("Semi-Limited");
      expect(normalizeBanStatus("semi-limited")).toBe("Semi-Limited");
      expect(normalizeBanStatus("Semi-limitata")).toBe("Semi-Limited");
      expect(normalizeBanStatus("semi")).toBe("Semi-Limited");
      expect(normalizeBanStatus("semilimitata")).toBe("Semi-Limited");
    });

    it("maps null, undefined, empty, or unknown strings to 'Unlimited'", () => {
      expect(normalizeBanStatus(null)).toBe("Unlimited");
      expect(normalizeBanStatus(undefined)).toBe("Unlimited");
      expect(normalizeBanStatus("")).toBe("Unlimited");
      expect(normalizeBanStatus("Unlimited")).toBe("Unlimited");
      expect(normalizeBanStatus("illimitata")).toBe("Unlimited");
    });
  });

  describe("getChangeType", () => {
    it("identifies restrictions when copies allowed decrease", () => {
      expect(getChangeType("Unlimited", "Limited")).toEqual({ label: "Restrizione", kind: "restriction" });
      expect(getChangeType("Unlimited", "Forbidden")).toEqual({ label: "Restrizione", kind: "restriction" });
      expect(getChangeType("Limited", "Forbidden")).toEqual({ label: "Restrizione", kind: "restriction" });
      expect(getChangeType("Semi-Limited", "Limited")).toEqual({ label: "Restrizione", kind: "restriction" });
      expect(getChangeType(null, "Banned")).toEqual({ label: "Restrizione", kind: "restriction" });
    });

    it("identifies unbanning / full loosening to Unlimited", () => {
      expect(getChangeType("Forbidden", "Unlimited")).toEqual({ label: "Sbandita (Illimitata)", kind: "loosening" });
      expect(getChangeType("Limited", "Unlimited")).toEqual({ label: "Sbandita (Illimitata)", kind: "loosening" });
      expect(getChangeType("Semi-Limited", null)).toEqual({ label: "Sbandita (Illimitata)", kind: "loosening" });
      expect(getChangeType("Banned", null)).toEqual({ label: "Sbandita (Illimitata)", kind: "loosening" });
    });

    it("identifies intermediate loosening", () => {
      expect(getChangeType("Forbidden", "Limited")).toEqual({ label: "Allentamento", kind: "loosening" });
      expect(getChangeType("Banned", "Limited")).toEqual({ label: "Allentamento", kind: "loosening" });
      expect(getChangeType("Limited", "Semi-Limited")).toEqual({ label: "Allentamento", kind: "loosening" });
      expect(getChangeType("Forbidden", "Semi-Limited")).toEqual({ label: "Allentamento", kind: "loosening" });
    });

    it("identifies neutral/identical status changes", () => {
      expect(getChangeType("Limited", "Limited")).toEqual({ label: "Modifica", kind: "neutral" });
      expect(getChangeType("Forbidden", "Banned")).toEqual({ label: "Modifica", kind: "neutral" });
      expect(getChangeType(null, null)).toEqual({ label: "Modifica", kind: "neutral" });
    });
  });
});
