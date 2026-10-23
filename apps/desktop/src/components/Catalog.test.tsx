import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { Catalog } from "./Catalog";
import type { Card, Deck } from "../lib/api";

// Mock api to prevent network calls
vi.mock("../lib/api", () => ({
  api: {
    facets: vi.fn().mockResolvedValue(null),
    search: vi.fn().mockResolvedValue({ ids: [], chips: [], errors: [] }),
    listSavedSearches: vi.fn().mockResolvedValue([]),
    saveSearch: vi.fn().mockResolvedValue([]),
    deleteSavedSearch: vi.fn().mockResolvedValue([]),
  },
}));

// Mock sound
vi.mock("../lib/sound", () => ({
  sound: {
    play: vi.fn(),
  },
}));

// Mock drag to provide static server snapshot
vi.mock("../lib/drag", () => ({
  useDrag: () => null,
}));

// Mock settings
vi.mock("../lib/settings", () => ({
  useSettings: () => ({ catalogView: "m", catalogWidth: 392 }),
  updateSettings: vi.fn(),
}));

// Mock favorites
vi.mock("../lib/favorites", () => ({
  useFavorites: () => new Set<number>(),
  toggleFavorite: vi.fn(),
}));

const MOCK_DECK: Deck = {
  id: 1,
  name: "Deck di Prova",
  format: "md",
  main: [],
  extra: [],
  side: [],
  maybe: [],
  roles: {},
  box: { color: "", material: "", shape: "", showcase: [] },
  updatedAt: null,
};

const DUMMY_GET_CARD = (_id: number): Card | undefined => undefined;

describe("Catalog - Barra di ricerca e filtri ridisegnati", () => {
  it("mostra l'header con titolo 'Catalogo', guida e menu vista a pulsante singolo", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query=""
        onQuery={() => {}}
      />
    );

    // Titolo catalogo
    expect(html).toContain("Catalogo");
    // Guida
    expect(html).toContain('data-tip="Guida del catalogo"');
    // Menu vista a pulsante singolo con menu popover
    expect(html).toContain('aria-haspopup="menu"');
    expect(html).toContain('aria-label="Vista catalogo: Carte medie"');
    expect(html).toContain('data-tip="Vista: Carte medie"');
  });

  it("mostra il campo di ricerca largo con input, scorciatoia e ricerche salvate integrate dentro", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query=""
        onQuery={() => {}}
      />
    );

    // Input con scorciatoia Ctrl+F
    expect(html).toContain('placeholder="Cerca o filtra… (Ctrl+F)"');
    // Pulsante ricerche salvate integrato dentro il campo
    expect(html).toContain('aria-label="Ricerche salvate"');
    expect(html).toContain('data-tip="Ricerche salvate"');
  });

  it("include i 4 interruttori rapidi a icona nella riga (Handtrap, Staple, Board breaker, Preferite)", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query=""
        onQuery={() => {}}
      />
    );

    expect(html).toContain('aria-label="Filtra Handtrap"');
    expect(html).toContain('data-tip="Handtrap (tag:handtrap)"');

    expect(html).toContain('aria-label="Filtra Staple"');
    expect(html).toContain('data-tip="Staple (tag:staple)"');

    expect(html).toContain('aria-label="Filtra Board breaker"');
    expect(html).toContain('data-tip="Board breaker (tag:breaker)"');

    expect(html).toContain('aria-label="Filtra Preferite"');
    expect(html).toContain('data-tip="Preferite (tag:fav)"');
  });

  it("imposta aria-pressed=\"true\" per i filtri rapidi attivi nella query", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query="tag:handtrap tag:fav"
        onQuery={() => {}}
      />
    );

    // Handtrap e Fav attivi
    expect(html).toMatch(/aria-label="Filtra Handtrap"[^>]*aria-pressed="true"/);
    expect(html).toMatch(/aria-label="Filtra Preferite"[^>]*aria-pressed="true"/);

    // Staple e Breaker non attivi
    expect(html).toMatch(/aria-label="Filtra Staple"[^>]*aria-pressed="false"/);
    expect(html).toMatch(/aria-label="Filtra Board breaker"[^>]*aria-pressed="false"/);
  });

  it("mostra il pulsante 'Filtri' con aria-expanded", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query=""
        onQuery={() => {}}
      />
    );

    expect(html).toContain("Filtri</span>");
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('title="Pannello filtri avanzati"');
  });

  it("mostra il pulsante di cancellazione rapida (✕) nel campo quando query non è vuota", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query="ash blossom"
        onQuery={() => {}}
      />
    );

    expect(html).toContain('aria-label="Cancella ricerca"');
  });

  it("non mostra la riga delle pillole rimovibili quando non ci sono filtri attivi o errori", () => {
    const html = renderToStaticMarkup(
      <Catalog
        deck={MOCK_DECK}
        getCard={DUMMY_GET_CARD}
        requestCards={() => {}}
        version={1}
        onAdd={() => {}}
        onOpen={() => {}}
        onHover={() => {}}
        onHoverEnd={() => {}}
        query=""
        onQuery={() => {}}
      />
    );

    // Non deve contenere pulsanti di rimozione filtro
    expect(html).not.toContain("Rimuovi filtro:");
  });
});
