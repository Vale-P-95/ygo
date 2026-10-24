import { describe, expect, it } from "vitest";
import type { ComboBoard, ComboGoal } from "./api";
import { describeGoal, goalCards, reaches } from "./goal";

const name = (c: number) => `C${c}`;
const board = (goalHits: number, goalSoft = 0) => ({ goalHits, goalSoft }) as ComboBoard;

describe("goal", () => {
  it("loads goals saved as plain codes or without hard", () => {
    expect(goalCards([5, { code: 7, zone: "grave" }])).toEqual([
      { code: 5, zone: "any" },
      { code: 7, zone: "grave" },
    ]);
    expect(goalCards("x")).toEqual([]);
  });

  it("counts only the hard cards to reach the goal", () => {
    const goal: ComboGoal = {
      cards: [
        { code: 1, zone: "set" },
        { code: 2, zone: "grave" },
        { code: 3, zone: "any", hard: false },
      ],
      all: true,
      first: true,
    };
    expect(reaches(board(1, 1), goal)).toBe(false);
    expect(reaches(board(2), goal)).toBe(true);
    expect(reaches(board(1), { ...goal, all: false })).toBe(true);
    expect(reaches(board(0, 1), { ...goal, cards: [{ code: 3, zone: "any", hard: false }] })).toBe(false);
  });

  it("describes places and soft cards", () => {
    const goal: ComboGoal = {
      cards: [
        { code: 1, zone: "set" },
        { code: 2, zone: "grave" },
        { code: 3, zone: "any", hard: false },
      ],
      all: true,
      first: false,
    };
    expect(describeGoal(goal, name)).toBe("C1 (M/T coperta) + C2 (Cimitero), meglio se anche C3");
    expect(describeGoal({ ...goal, all: false }, name)).toContain("una tra");
  });
});
