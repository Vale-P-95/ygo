import { describe, expect, it } from "vitest";
import type { Cond } from "./api";
import {
  BUILTIN_RECIPES,
  clause,
  describe as describeCond,
  fromCond,
  group,
  toCond,
  usedRoles,
} from "./conditions";

describe("conditions", () => {
  describe("toCond and fromCond invertibility", () => {
    it("round-trips Cond -> fromCond -> toCond for group conditions", () => {
      const cond: Cond = {
        type: "all",
        items: [
          { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 1 },
          {
            type: "any",
            items: [
              { type: "count", target: { kind: "role", value: "extender" }, op: "atLeast", n: 1 },
              { type: "count", target: { kind: "card", value: 46986414 }, op: "exactly", n: 0 },
            ],
          },
        ],
      };

      const groupNode = fromCond(cond);
      const reconstructed = toCond(groupNode);
      expect(reconstructed).toEqual(cond);
    });

    it("round-trips negated conditions correctly", () => {
      const cond: Cond = {
        type: "not",
        item: {
          type: "all",
          items: [{ type: "count", target: { kind: "role", value: "brick" }, op: "atLeast", n: 1 }],
        },
      };

      const groupNode = fromCond(cond);
      expect(groupNode.negate).toBe(true);
      const reconstructed = toCond(groupNode);
      expect(reconstructed).toEqual(cond);
    });

    it("round-trips Group -> toCond -> fromCond preserving structure", () => {
      const g = group("all", [
        clause({ kind: "role", value: "starter" }, "atLeast", 2),
        group("any", [
          clause({ kind: "role", value: "handtrap" }, "atLeast", 1),
          clause({ kind: "role", value: "breaker" }, "atLeast", 1),
        ]),
      ]);

      const c = toCond(g);
      const g2 = fromCond(c);

      expect(g2.mode).toBe(g.mode);
      expect(g2.negate).toBe(g.negate);
      expect(g2.items.length).toBe(g.items.length);
      expect(toCond(g2)).toEqual(c);
    });

    it("handles single clause wrapped as a root group", () => {
      const single: Cond = { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 1 };
      const root = fromCond(single);
      expect(root.kind).toBe("group");
      expect(root.mode).toBe("all");
      expect(root.items.length).toBe(1);
    });
  });

  describe("usedRoles", () => {
    it("extracts unique role names from nested condition tree", () => {
      const cond: Cond = {
        type: "all",
        items: [
          { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 1 },
          {
            type: "any",
            items: [
              { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 2 },
              { type: "count", target: { kind: "role", value: "extender" }, op: "atLeast", n: 1 },
              { type: "count", target: { kind: "card", value: 12345 }, op: "atLeast", n: 1 },
            ],
          },
        ],
      };

      const roles = usedRoles(cond);
      expect(roles).toEqual(["starter", "extender"]);
    });

    it("handles negated conditions", () => {
      const cond: Cond = {
        type: "not",
        item: {
          type: "count",
          target: { kind: "role", value: "brick" },
          op: "atLeast",
          n: 1,
        },
      };

      expect(usedRoles(cond)).toEqual(["brick"]);
    });

    it("extracts roles from BUILTIN_RECIPES without error", () => {
      for (const recipe of BUILTIN_RECIPES) {
        const roles = usedRoles(recipe.cond);
        expect(roles.length).toBeGreaterThan(0);
      }
    });
  });

  describe("describe", () => {
    const roleName = (r: string) => (r === "starter" ? "Starter" : r === "extender" ? "Estensore" : r === "brick" ? "Brick" : r);
    const cardName = (id: number) => (id === 1000 ? "Dark Magician" : `Carta #${id}`);

    it("formats simple clause descriptions with operators", () => {
      const c1: Cond = { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 1 };
      expect(describeCond(c1, roleName, cardName)).toBe("almeno 1 Starter");

      const c2: Cond = { type: "count", target: { kind: "role", value: "brick" }, op: "exactly", n: 0 };
      expect(describeCond(c2, roleName, cardName)).toBe("esattamente 0 Brick");

      const c3: Cond = { type: "count", target: { kind: "card", value: 1000 }, op: "atMost", n: 2 };
      expect(describeCond(c3, roleName, cardName)).toBe("al massimo 2 Dark Magician");
    });

    it("formats negated conditions with 'non (...)'", () => {
      const c: Cond = {
        type: "not",
        item: { type: "count", target: { kind: "role", value: "brick" }, op: "atLeast", n: 1 },
      };
      expect(describeCond(c, roleName, cardName)).toBe("non (almeno 1 Brick)");
    });

    it("joins 'all' with ' e ' and 'any' with ' o ', parenting nested groups", () => {
      const c: Cond = {
        type: "all",
        items: [
          { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 1 },
          {
            type: "any",
            items: [
              { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 2 },
              { type: "count", target: { kind: "role", value: "extender" }, op: "atLeast", n: 1 },
            ],
          },
        ],
      };

      expect(describeCond(c, roleName, cardName)).toBe("almeno 1 Starter e (almeno 2 Starter o almeno 1 Estensore)");
    });
  });
});
