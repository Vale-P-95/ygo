import { describe, expect, it } from "vitest";
import { mockApi } from "./mock";

describe("mock search", () => {
  it("shows cards for the Deck builder's default search", async () => {
    const r = await mockApi.search("format:md sort:popular");
    expect(r.ids.length).toBeGreaterThan(0);
    expect(r.errors).toEqual([]);
  });

  it("orders by ATK with sort:atk", async () => {
    const r = await mockApi.search("type:monster sort:atk");
    const cards = await mockApi.getCards(r.ids);
    const atks = cards.map((c) => c.atk ?? -1);
    expect(atks).toEqual([...atks].sort((a, b) => b - a));
  });

  it("reports an unknown sort instead of matching no card", async () => {
    const r = await mockApi.search("sort:boh");
    expect(r.errors.length).toBe(1);
    expect(r.ids.length).toBeGreaterThan(0);
  });
});

describe("recorded deck", () => {
  it("answers legality and odds with the recorded backend's numbers", async () => {
    const decks = await mockApi.listDecks();
    const summary = decks.find((d) => d.name === "Invoked (registrato)");
    if (!summary) return; // no recorded.json
    const deck = await mockApi.getDeck(summary.id);
    if (!deck) throw new Error("recorded deck missing");
    const legality = await mockApi.validateDeck(deck);
    expect(legality.legal).toBe(true);
    const overview = await mockApi.labOverview(deck.main, deck.roles);
    expect(overview.size).toBe(deck.main.length);
    expect(overview.roles.starter).toBeGreaterThan(0);
    // Changing the roles leaves the recorded answer for the mock's own.
    const other = await mockApi.labOverview(deck.main, {});
    expect(other.roles.starter).toBeUndefined();
  });
});
