import { describe, expect, it } from "vitest";
import { mergeSettings } from "./settings";

describe("mergeSettings", () => {
  it("fills the fields an older save does not have", () => {
    const s = mergeSettings({ theme: "light", sound: false, masterVolume: 0.2 });
    expect(s.theme).toBe("light");
    expect(s.sound).toBe(false);
    expect(s.masterVolume).toBe(0.2);
    expect(s.soundUi).toBe(true);
    expect(s.pageTransitions).toBe(true);
    expect(s.linksEffects).toBe(true);
  });

  it("ignores unknown fields and values of the wrong type", () => {
    const s = mergeSettings({ ambient: "no", shelfEffects: false, gone: 1 });
    expect(s.ambient).toBe(true);
    expect(s.shelfEffects).toBe(false);
    expect("gone" in s).toBe(false);
  });

  it("survives garbage", () => {
    expect(mergeSettings(null).soundEvents).toBe(true);
    expect(mergeSettings("x").soundShelf).toBe(true);
  });
});
