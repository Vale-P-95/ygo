import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AnatomyEffects, AnatomyLegend, AnatomyText } from "./CardAnatomy";
import type { CardAnatomy } from "../lib/api";

// A text written for the test in the style of the cards, split as `ygo_data::psct` does.
const ANATOMY: CardAnatomy = {
  text: [
    { text: "If this card is Summoned:", part: "condition", effect: 0 },
    { text: " ", part: null, effect: null },
    { text: "You can discard 1 card;", part: "cost", effect: 0 },
    { text: ' add 1 "Test" card from your Deck to your hand.', part: "action", effect: 0 },
    { text: " ", part: null, effect: null },
    { text: "You can only use this effect once per turn.", part: "limit", effect: null },
  ],
  pendulum: [],
  effects: [
    {
      kind: "trigger",
      mandatory: false,
      zones: ["field"],
      limit: { count: 1, sharedWith: [], perDuel: false },
      categories: ["search", "to hand"],
      textOnly: ["banish"],
      targets: false,
      cost: true,
      condition: false,
      description: "",
      textEffect: 0,
    },
  ],
  scriptError: null,
};

describe("CardAnatomy", () => {
  it("colours the parts of the text and keeps card names as links", () => {
    const html = renderToStaticMarkup(<AnatomyText text="" pieces={ANATOMY.text} onSearch={() => {}} />);
    expect(html).toContain('title="Condizione"');
    expect(html).toContain('title="Costo"');
    expect(html).toContain("var(--anat-limit)");
    expect(html).toContain("<button");
    expect(html).toContain("&quot;Test&quot;");
  });

  it("shows the plain text without pieces", () => {
    const html = renderToStaticMarkup(<AnatomyText text="Plain text." />);
    expect(html).not.toContain("--anat-");
    expect(html).toContain("Plain text.");
  });

  it("counts the parts in the legend", () => {
    const html = renderToStaticMarkup(<AnatomyLegend pieces={ANATOMY.text} />);
    expect(html).toContain("Restrizione");
    expect(html).not.toContain("Materiali");
  });

  it("lists each effect in Italian with the parts of its text", () => {
    const html = renderToStaticMarkup(<AnatomyEffects anatomy={ANATOMY} />);
    expect(html).toContain("Innescato · facoltativo");
    expect(html).toContain("sul campo");
    expect(html).toContain("1 volta per turno");
    expect(html).toContain("cerca nel Deck");
    expect(html).toContain("bandisce · solo dal testo");
    expect(html).toContain("You can discard 1 card;");
    expect(html).not.toContain("non hanno gli stessi effetti");
  });

  it("says when the script could not be read", () => {
    const html = renderToStaticMarkup(<AnatomyEffects anatomy={{ ...ANATOMY, effects: [], scriptError: "no engine data" }} />);
    expect(html).toContain("non si è potuto leggere");
  });
});
