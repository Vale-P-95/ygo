import { describe, expect, it } from "vitest";
import {
  getKey,
  getRange,
  KEYS,
  quote,
  setKey,
  setRange,
  splitToken,
  toggleToken,
  tokenize,
} from "./query";

describe("query", () => {
  describe("tokenize", () => {
    it("splits query by whitespace", () => {
      expect(tokenize("t:monster lv:4 atk>=1500")).toEqual(["t:monster", "lv:4", "atk>=1500"]);
    });

    it("preserves quotes with spaces as a single token", () => {
      expect(tokenize('arch:"Dark Magician" "Ash Blossom & Joyous Spring"')).toEqual([
        'arch:"Dark Magician"',
        '"Ash Blossom & Joyous Spring"',
      ]);
    });

    it("handles multiple consecutive spaces and empty strings", () => {
      expect(tokenize("   ")).toEqual([]);
      expect(tokenize("  a   b   c  ")).toEqual(["a", "b", "c"]);
    });
  });

  describe("splitToken", () => {
    it("returns null for quoted phrases without key prefix", () => {
      expect(splitToken('"Dark Magician"')).toBeNull();
      expect(splitToken("monster")).toBeNull();
    });

    it("parses colon and equality operators as op '='", () => {
      expect(splitToken("formato:md")).toEqual({ key: "formato", op: "=", value: "md" });
      expect(splitToken("ban:limitata")).toEqual({ key: "ban", op: "=", value: "limitata" });
      expect(splitToken("type=spell")).toEqual({ key: "type", op: "=", value: "spell" });
    });

    it("parses comparison operators (>=, <=, >, <)", () => {
      expect(splitToken("atk>=1500")).toEqual({ key: "atk", op: ">=", value: "1500" });
      expect(splitToken("def<=2000")).toEqual({ key: "def", op: "<=", value: "2000" });
      expect(splitToken("lv>4")).toEqual({ key: "lv", op: ">", value: "4" });
      expect(splitToken("rank<8")).toEqual({ key: "rank", op: "<", value: "8" });
    });

    it("strips outer quotes from token value", () => {
      expect(splitToken('arch:"Blue-Eyes"')).toEqual({ key: "arch", op: "=", value: "Blue-Eyes" });
      expect(splitToken('nome:"Mirror Force"')).toEqual({ key: "nome", op: "=", value: "Mirror Force" });
    });

    it("supports Italian accented keys like rarità", () => {
      expect(splitToken("rarità:ultra")).toEqual({ key: "rarità", op: "=", value: "ultra" });
    });
  });

  describe("quote", () => {
    it("wraps strings containing spaces in quotes", () => {
      expect(quote("Dark Magician")).toBe('"Dark Magician"');
      expect(quote("dragon")).toBe("dragon");
    });
  });

  describe("getKey and setKey", () => {
    it("retrieves value for matching key alias", () => {
      const q = "formato:md ban:limitata lv>=4";
      expect(getKey(q, KEYS.format)).toBe("md");
      expect(getKey(q, ["ban"])).toBe("limitata");
      expect(getKey(q, ["arch"])).toBeNull();
    });

    it("returns null when operator is not '='", () => {
      expect(getKey("lv>=4", KEYS.level)).toBeNull();
    });

    it("sets key replacing existing alias tokens", () => {
      const initial = "f:tcg t:monster";
      const updated = setKey(initial, KEYS.format, "md");
      expect(updated).toBe("t:monster format:md");
      expect(getKey(updated, KEYS.format)).toBe("md");
    });

    it("quotes value with spaces when setting key", () => {
      const updated = setKey("t:monster", KEYS.arch, "Dark Magician");
      expect(updated).toBe('t:monster arch:"Dark Magician"');
      expect(getKey(updated, KEYS.arch)).toBe("Dark Magician");
    });

    it("removes key token when value is null", () => {
      const initial = "t:monster ban:limitata formato:md";
      const cleared = setKey(initial, ["ban"], null);
      expect(cleared).toBe("t:monster formato:md");
      expect(getKey(cleared, ["ban"])).toBeNull();
    });
  });

  describe("toggleToken", () => {
    it("adds token if not present", () => {
      expect(toggleToken("t:monster", "t:spell")).toBe("t:monster t:spell");
    });

    it("removes token if present (case-insensitive)", () => {
      expect(toggleToken("t:monster t:spell", "t:monster")).toBe("t:spell");
      expect(toggleToken("T:MONSTER t:spell", "t:monster")).toBe("t:spell");
    });
  });

  describe("getRange and setRange", () => {
    it("extracts range from 'key:min-max'", () => {
      expect(getRange("atk:1500-2500", KEYS.atk)).toEqual({ min: "1500", max: "2500" });
    });

    it("extracts range from separate >= and <= tokens", () => {
      expect(getRange("lv>=4 lv<=8", KEYS.level)).toEqual({ min: "4", max: "8" });
      expect(getRange("atk>=2000", KEYS.atk)).toEqual({ min: "2000", max: "" });
      expect(getRange("def<=1000", KEYS.def)).toEqual({ min: "", max: "1000" });
    });

    it("sets combined range when both min and max are provided", () => {
      const res = setRange("t:monster", KEYS.atk, { min: "1000", max: "2000" });
      expect(res).toBe("t:monster atk:1000-2000");
    });

    it("sets single bound operator when only min or max is provided", () => {
      expect(setRange("t:monster", KEYS.atk, { min: "1500", max: "" })).toBe("t:monster atk>=1500");
      expect(setRange("t:monster", KEYS.atk, { min: "", max: "3000" })).toBe("t:monster atk<=3000");
    });

    it("removes range tokens when both bounds are empty", () => {
      expect(setRange("t:monster atk:1000-2000", KEYS.atk, { min: "", max: "" })).toBe("t:monster");
    });
  });
});
