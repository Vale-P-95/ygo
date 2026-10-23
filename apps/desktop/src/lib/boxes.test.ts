import { describe, expect, it } from "vitest";
import { MATERIALS, PALETTE, resolveLook, SHAPES, SIZES, type Material, type Shape } from "./boxes";

describe("boxes", () => {
  it("defines palettes, materials and shapes with valid metadata", () => {
    expect(PALETTE.length).toBeGreaterThanOrEqual(12);
    for (const p of PALETTE) {
      expect(p.color).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(p.name.length).toBeGreaterThan(0);
    }

    const materialIds = MATERIALS.map((m) => m.id);
    expect(materialIds).toContain("leather");
    expect(materialIds).toContain("matte");
    expect(materialIds).toContain("metal");
    expect(materialIds).toContain("wood");
    expect(materialIds).toContain("cardboard");
    expect(materialIds).toContain("holo");

    const shapeIds = SHAPES.map((s) => s.id);
    expect(shapeIds).toEqual(["box", "tin", "chest"]);
    for (const s of shapeIds) {
      expect(SIZES[s as Shape]).toBeDefined();
      expect(SIZES[s as Shape].w).toBeGreaterThan(0);
      expect(SIZES[s as Shape].h).toBeGreaterThan(0);
      expect(SIZES[s as Shape].d).toBeGreaterThan(0);
    }
  });

  it("resolveLook uses stable palette fallback when box is undefined", () => {
    const look0 = resolveLook(undefined, 0);
    expect(look0.color).toBe(PALETTE[0].color);
    expect(look0.material).toBe("leather");
    expect(look0.shape).toBe("box");

    const look1 = resolveLook(undefined, 1);
    expect(look1.color).toBe(PALETTE[1].color);

    const lookNeg = resolveLook(undefined, -2);
    expect(lookNeg.color).toBe(PALETTE[2].color);
  });

  it("resolveLook preserves custom box settings and validates materials including cardboard", () => {
    const custom = resolveLook(
      {
        color: "#123456",
        material: "cardboard",
        shape: "chest",
        showcase: [12345],
      },
      99,
    );
    expect(custom.color).toBe("#123456");
    expect(custom.material).toBe("cardboard");
    expect(custom.shape).toBe("chest");
  });

  it("resolveLook falls back to defaults for invalid material or shape", () => {
    const invalid = resolveLook(
      {
        color: "#abcdef",
        material: "titanium" as Material,
        shape: "pyramid" as Shape,
        showcase: [],
      },
      5,
    );
    expect(invalid.color).toBe("#abcdef");
    expect(invalid.material).toBe("leather");
    expect(invalid.shape).toBe("box");
  });
});
