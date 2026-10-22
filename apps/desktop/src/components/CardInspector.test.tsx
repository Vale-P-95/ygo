import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CardInspector } from "./CardInspector";
import type { Card, Deck } from "../lib/api";

const MOCK_DECK: Deck = {
  id: 1,
  name: "Deck di Prova",
  format: "md",
  main: [1000, 1000],
  extra: [2000],
  side: [1000],
  maybe: [],
  roles: {
    starter: [1000],
    handtrap: [3000],
  },
  box: { color: "", material: "", shape: "", showcase: [] },
  updatedAt: null,
};

const MOCK_MONSTER: Card = {
  id: 1000,
  name: "Aleister the Invoker",
  kind: "monster",
  frame: "effect",
  race: "Spellcaster",
  attribute: "DARK",
  typeLine: ["Spellcaster", "Effect"],
  humanType: "Effetto Mostro",
  level: 4,
  atk: 1000,
  def: 1800,
  desc: "Se questa carta viene Evocata Normalmente: puoi aggiungere 1 «Invocazione» dal tuo Deck alla tua mano.",
  pendDesc: null,
  scale: null,
  linkRating: null,
  linkMarkers: [],
  archetype: "Invoked",
  mdRarity: "Super Rare",
  formats: ["Master Duel", "TCG", "OCG"],
  banMd: null,
  banTcg: null,
  banOcg: "Limited",
  imageIds: [1000],
  views: 0,
  tcgDate: "2017-02-17",
  ocgDate: "2016-11-12",
};

const MOCK_EXTRA_MONSTER: Card = {
  id: 2000,
  name: "Invoked Mechaba",
  kind: "monster",
  frame: "fusion",
  race: "Machine",
  attribute: "LIGHT",
  typeLine: ["Fusion", "Effect"],
  humanType: "Mostro Fusione",
  level: 9,
  atk: 2500,
  def: 2100,
  desc: "1 mostro «Invoker» + 1 mostro LUCE.",
  pendDesc: null,
  scale: null,
  linkRating: null,
  linkMarkers: [],
  archetype: "Invoked",
  mdRarity: "Ultra Rare",
  formats: ["Master Duel", "TCG", "OCG"],
  banMd: null,
  banTcg: null,
  banOcg: null,
  imageIds: [2000],
  views: 0,
  tcgDate: "2017-02-17",
  ocgDate: "2016-11-12",
};

const MOCK_OCG_EXCLUSIVE: Card = {
  id: 3000,
  name: "OCG Exclusive Card",
  kind: "monster",
  frame: "effect",
  race: "Warrior",
  attribute: "EARTH",
  typeLine: ["Warrior", "Effect"],
  humanType: "Effetto Mostro",
  level: 3,
  atk: 1200,
  def: 1000,
  desc: "Carta rilasciata soltanto in OCG.",
  pendDesc: null,
  scale: null,
  linkRating: null,
  linkMarkers: [],
  archetype: null,
  mdRarity: null,
  formats: ["OCG"],
  banMd: null,
  banTcg: null,
  banOcg: null,
  imageIds: [3000],
  views: 0,
  tcgDate: null,
  ocgDate: "2024-01-01",
};

describe("CardInspector", () => {
  it("mostra il messaggio di stato vuoto quando non c'è nessuna carta", () => {
    const html = renderToStaticMarkup(
      <CardInspector deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain("Nessuna carta selezionata");
    expect(html).toContain("Passa il mouse su una carta o fai clic");
  });

  it("mostra il nome, i tipi, le statistiche ATK/DEF e la rarità della carta", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain("Aleister the Invoker");
    expect(html).toContain("Spellcaster / Effect");
    expect(html).toContain("1000"); // ATK
    expect(html).toContain("1800"); // DEF
    expect(html).toContain("SR"); // Super Rare
    expect(html).toContain("DARK");
    expect(html).toContain("Livello 4");
  });

  it("mostra lo stato fissato quando pinned è true", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={true} onUnpin={() => {}} />,
    );

    expect(html).toContain("Carta fissata");
    expect(html).toContain("Sblocca carta (Esc)");
  });

  it("mostra lo stato di anteprima al passaggio quando pinned è false", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain("Anteprima passaggio");
    expect(html).not.toContain("Carta fissata");
  });

  it("mostra il conteggio copie nel deck (Main, Extra, Side)", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    // MOCK_MONSTER has 2 in main, 1 in side = 3 total
    expect(html).toContain("Copie nel deck");
    expect(html).toContain("3");
    expect(html).toContain("Main: 2");
    expect(html).toContain("Side: 1");
  });

  it("mostra i ruoli della carta assegnati nel deck", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    // 1000 is marked as starter
    expect(html).toContain("Starter");
  });

  it("mostra la banlist nei tre formati (MD, TCG, OCG) in righe leggibili", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain("Master Duel");
    expect(html).toContain("TCG");
    expect(html).toContain("OCG");
    expect(html).toContain("Limitata (1)"); // OCG is Limited
    expect(html).toContain("3 copie"); // MD and TCG are legal
  });

  it("gestisce coerentemente le carte non presenti in un formato (Punto 9)", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_OCG_EXCLUSIVE} deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain("Non in MD");
    expect(html).toContain("Non in TCG");
    expect(html).toContain("3 copie"); // in OCG is legal
    expect(html).toContain("—"); // non in format icon dash
  });

  it("renderizza l'immagine con l'id carta come chiave per azzeramento al cambio carta (Punto 8)", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain('cardimg://localhost/full/1000');
  });

  it("adatta l'azione rapida a Extra per mostri dell'Extra Deck", () => {
    const html = renderToStaticMarkup(
      <CardInspector card={MOCK_EXTRA_MONSTER} deck={MOCK_DECK} pinned={false} />,
    );

    expect(html).toContain("Extra");
    expect(html).toContain("Side");
  });
});
