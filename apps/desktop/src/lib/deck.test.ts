import { describe, expect, it } from "vitest";
import type { Card, Deck } from "./api";
import { addCard, copyLimit, counts, emptyDeck, removeAt, ZONE_LIMITS } from "./deck";

function makeCard(partial: Partial<Card> & { id: number; name: string }): Card {
  return {
    id: partial.id,
    name: partial.name,
    kind: partial.kind ?? "monster",
    frame: partial.frame ?? "effect",
    humanType: partial.humanType ?? "Effect Monster",
    typeLine: partial.typeLine ?? ["Effect Monster"],
    race: partial.race ?? "Warrior",
    attribute: partial.attribute ?? "DARK",
    archetype: partial.archetype ?? null,
    desc: partial.desc ?? "",
    pendDesc: null,
    atk: partial.atk ?? 1500,
    def: partial.def ?? 1200,
    level: partial.level ?? 4,
    scale: null,
    linkRating: null,
    linkMarkers: [],
    mdRarity: partial.mdRarity ?? "Common",
    formats: ["md", "tcg", "ocg"],
    tcgDate: null,
    ocgDate: null,
    banMd: partial.banMd ?? null,
    banTcg: partial.banTcg ?? null,
    banOcg: partial.banOcg ?? null,
    imageIds: [partial.id],
    views: 0,
  };
}

describe("deck", () => {
  describe("copyLimit", () => {
    it("returns 0 for Forbidden / Banned cards in MD, TCG and OCG", () => {
      const card = makeCard({
        id: 1,
        name: "Pot of Greed",
        banMd: "Forbidden",
        banTcg: "Banned",
        banOcg: "Forbidden",
      });
      expect(copyLimit(card, "md")).toBe(0);
      expect(copyLimit(card, "tcg")).toBe(0);
      expect(copyLimit(card, "ocg")).toBe(0);
    });

    it("returns 1 for Limited cards in MD, TCG and OCG", () => {
      const card = makeCard({
        id: 2,
        name: "Called by the Grave",
        banMd: "Semi-Limited",
        banTcg: "Limited",
        banOcg: "Limited",
      });
      expect(copyLimit(card, "md")).toBe(2);
      expect(copyLimit(card, "tcg")).toBe(1);
      expect(copyLimit(card, "ocg")).toBe(1);
    });

    it("returns 2 for Semi-Limited cards", () => {
      const card = makeCard({
        id: 3,
        name: "Destiny HERO - Malicious",
        banMd: "Semi-Limited",
        banTcg: "Semi-Limited",
        banOcg: "Semi-Limited",
      });
      expect(copyLimit(card, "md")).toBe(2);
      expect(copyLimit(card, "tcg")).toBe(2);
      expect(copyLimit(card, "ocg")).toBe(2);
    });

    it("returns 3 for Unlimited cards or cards with null ban status", () => {
      const card = makeCard({
        id: 4,
        name: "Dark Magician",
        banMd: null,
        banTcg: null,
        banOcg: null,
      });
      expect(copyLimit(card, "md")).toBe(3);
      expect(copyLimit(card, "tcg")).toBe(3);
      expect(copyLimit(card, "ocg")).toBe(3);
    });
  });

  describe("addCard", () => {
    const cardMap = new Map<number, Card>();
    const getCard = (id: number) => cardMap.get(id);

    const normalMonster = makeCard({ id: 10, name: "Aleister the Invoker", kind: "monster", typeLine: ["Effect Monster"] });
    const fusionMonster = makeCard({ id: 20, name: "Invoked Mechaba", kind: "monster", typeLine: ["Fusion", "Monster"] });
    const forbiddenCard = makeCard({ id: 30, name: "Pot of Greed", banMd: "Forbidden", banTcg: "Forbidden" });
    const limitedCard = makeCard({ id: 40, name: "Terraforming", banMd: "Limited", banTcg: "Limited" });
    const tokenCard = makeCard({ id: 50, name: "Token", kind: "token" });
    const skillCard = makeCard({ id: 60, name: "Skill Card", kind: "skill" });

    [normalMonster, fusionMonster, forbiddenCard, limitedCard, tokenCard, skillCard].forEach((c) => cardMap.set(c.id, c));

    it("automatically places Extra Deck monsters into extra zone", () => {
      const d = emptyDeck("Test", "md");
      const res = addCard(d, fusionMonster, null, getCard);
      expect("deck" in res).toBe(true);
      if ("deck" in res) {
        expect(res.zone).toBe("extra");
        expect(res.deck.extra).toContain(20);
        expect(res.deck.main.length).toBe(0);
      }
    });

    it("redirects want='main' to extra for Extra Deck monsters", () => {
      const d = emptyDeck("Test", "md");
      const res = addCard(d, fusionMonster, "main", getCard);
      expect("deck" in res).toBe(true);
      if ("deck" in res) {
        expect(res.zone).toBe("extra");
        expect(res.deck.extra).toContain(20);
      }
    });

    it("rejects non-Extra cards when want='extra'", () => {
      const d = emptyDeck("Test", "md");
      const res = addCard(d, normalMonster, "extra", getCard);
      expect("error" in res).toBe(true);
      if ("error" in res) {
        expect(res.error).toContain("non è una carta dell'Extra Deck");
      }
    });

    it("rejects tokens and skills from deck", () => {
      const d = emptyDeck("Test", "md");
      const resToken = addCard(d, tokenCard, null, getCard);
      expect("error" in resToken).toBe(true);
      const resSkill = addCard(d, skillCard, null, getCard);
      expect("error" in resSkill).toBe(true);
    });

    it("rejects forbidden cards", () => {
      const d = emptyDeck("Test", "md");
      const res = addCard(d, forbiddenCard, null, getCard);
      expect("error" in res).toBe(true);
      if ("error" in res) {
        expect(res.error).toContain("è proibita in Master Duel");
      }
    });

    it("rejects exceeding copy limits", () => {
      let d = emptyDeck("Test", "md");
      const r1 = addCard(d, limitedCard, null, getCard);
      expect("deck" in r1).toBe(true);
      if ("deck" in r1) d = r1.deck;

      const r2 = addCard(d, limitedCard, null, getCard);
      expect("error" in r2).toBe(true);
      if ("error" in r2) {
        expect(r2.error).toContain("è limitata a 1");
      }
    });

    it("rejects adding more than 3 copies of an unlimited card across main/extra/side", () => {
      let d = emptyDeck("Test", "md");
      for (let i = 0; i < 3; i++) {
        const r = addCard(d, normalMonster, null, getCard);
        expect("deck" in r).toBe(true);
        if ("deck" in r) d = r.deck;
      }

      const r4 = addCard(d, normalMonster, null, getCard);
      expect("error" in r4).toBe(true);
      if ("error" in r4) {
        expect(r4.error).toContain("al massimo 3 copie");
      }
    });

    it("respects zone capacity limits (main 60, extra 15)", () => {
      let d = emptyDeck("Test", "md");
      d = { ...d, main: new Array(ZONE_LIMITS.main).fill(999) };
      const res = addCard(d, normalMonster, null, getCard);
      expect("error" in res).toBe(true);
      if ("error" in res) {
        expect(res.error).toContain("Main Deck è pieno");
      }

      let dExtra = emptyDeck("Test", "md");
      dExtra = { ...dExtra, extra: new Array(ZONE_LIMITS.extra).fill(999) };
      const resExtra = addCard(dExtra, fusionMonster, null, getCard);
      expect("error" in resExtra).toBe(true);
      if ("error" in resExtra) {
        expect(resExtra.error).toContain("Extra Deck è pieno");
      }
    });

    it("allows adding to maybe board beyond 3 copies but respects maybe zone limit (30)", () => {
      let d = emptyDeck("Test", "md");
      for (let i = 0; i < 5; i++) {
        const r = addCard(d, normalMonster, "maybe", getCard);
        expect("deck" in r).toBe(true);
        if ("deck" in r) d = r.deck;
      }
      expect(d.maybe.length).toBe(5);

      d = { ...d, maybe: new Array(ZONE_LIMITS.maybe).fill(999) };
      const res = addCard(d, normalMonster, "maybe", getCard);
      expect("error" in res).toBe(true);
      if ("error" in res) {
        expect(res.error).toContain("Maybe board è piena");
      }
    });
  });

  describe("removeAt", () => {
    it("removes card at specified index from zone", () => {
      const d: Deck = {
        ...emptyDeck(),
        main: [101, 102, 103],
      };
      const updated = removeAt(d, "main", 1);
      expect(updated.main).toEqual([101, 103]);
    });
  });

  describe("counts", () => {
    it("correctly tallies monsters, spells, and traps", () => {
      const cards = new Map<number, Card>([
        [1, makeCard({ id: 1, name: "Monster 1", kind: "monster" })],
        [2, makeCard({ id: 2, name: "Monster 2", kind: "monster" })],
        [3, makeCard({ id: 3, name: "Spell 1", kind: "spell" })],
        [4, makeCard({ id: 4, name: "Trap 1", kind: "trap" })],
      ]);

      const res = counts([1, 2, 3, 4, 1], (id) => cards.get(id));
      expect(res).toEqual({ monsters: 3, spells: 1, traps: 1 });
    });
  });
});
