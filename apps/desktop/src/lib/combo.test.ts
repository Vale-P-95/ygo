import { describe, expect, it, vi } from "vitest";
import { api, type ComboScore } from "./api";
import { clearMetaError, compareMeta } from "./combo";

function makeScore(hand: number, key: string, partial: Partial<ComboScore> = {}): ComboScore {
  return {
    hand,
    key,
    interruptions: partial.interruptions ?? 1,
    handTraps: partial.handTraps ?? 0,
    floodgates: partial.floodgates ?? 0,
    resilience: partial.resilience ?? 1,
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
    meta: partial.meta ?? null,
    error: partial.error ?? null,
  };
}

describe("combo lib - compareMeta", () => {
  it("calls api.comboMeta with given board refs", async () => {
    const metaSpy = vi.spyOn(api, "comboMeta").mockResolvedValueOnce(2);
    const scoreListenerSpy = vi.spyOn(api, "onComboScore").mockResolvedValueOnce(() => {});

    const boards = [
      { hand: 0, key: "abc" },
      { hand: 1, key: "def" },
    ];
    const count = await compareMeta(boards, 42, 1);
    expect(count).toBe(2);
    expect(metaSpy).toHaveBeenCalledWith(boards);

    metaSpy.mockRestore();
    scoreListenerSpy.mockRestore();
  });

  it("handles errors from api.comboMeta properly", async () => {
    const metaSpy = vi.spyOn(api, "comboMeta").mockRejectedValueOnce(new Error("Network failure"));
    const scoreListenerSpy = vi.spyOn(api, "onComboScore").mockResolvedValueOnce(() => {});

    await expect(compareMeta(undefined, 42, 2)).rejects.toThrow("Network failure");
    clearMetaError();

    metaSpy.mockRestore();
    scoreListenerSpy.mockRestore();
  });

  it("ranks boards with meta.value ahead of other boards", () => {
    const boards = [
      { hand: 0, key: "b1", goalHits: 1, score: makeScore(0, "b1", { meta: null }) },
      { hand: 0, key: "b2", goalHits: 1, score: makeScore(0, "b2", { meta: { value: 0.65, left: 0.35, perDeck: [] } }) },
      { hand: 0, key: "b3", goalHits: 1, score: makeScore(0, "b3", { meta: { value: 0.88, left: 0.12, perDeck: [] } }) },
    ];

    // Sorting comparator matching Combo.tsx:
    const sorted = [...boards].sort((x, y) => {
      const xMeta = x.score.meta?.value ?? null;
      const yMeta = y.score.meta?.value ?? null;
      if (xMeta != null && yMeta == null) return -1;
      if (xMeta == null && yMeta != null) return 1;
      if (xMeta != null && yMeta != null) {
        const diff = yMeta - xMeta;
        if (Math.abs(diff) > 1e-6) return diff;
      }
      return y.goalHits - x.goalHits;
    });

    expect(sorted.map((b) => b.key)).toEqual(["b3", "b2", "b1"]);
  });
});
