export type SectionId = "overview" | "deck-builder" | "shelf" | "stats" | "combo" | "news" | "settings";

export type Emblem = "gear" | "anvil" | "shelf" | "flask" | "rune" | "bell" | "cog";

export interface Section {
  id: SectionId;
  /** Name in the sidebar. */
  label: string;
  /** Themed title shown in the section banner. */
  title: string;
  summary: string;
  /** Roadmap phase that delivers the section (see docs/PIANO.md §11). */
  phase: number;
  planned: string[];
  emblem: Emblem;
  /**
   * Card whose artwork (downloaded at runtime, never bundled) tints the
   * banner: the monsters that inspired each section's theme.
   */
  themeCard?: { id: number; name: string };
}

export const sections: Section[] = [
  {
    id: "overview",
    label: "Panoramica",
    title: "L'officina",
    summary: "Stato dei dati e del motore di gioco, e avanzamento del progetto.",
    phase: 0,
    planned: [],
    emblem: "gear",
    themeCard: { id: 59811955, name: "Ancient Gear Workshop" },
  },
  {
    id: "deck-builder",
    label: "Deck Builder",
    title: "La Fucina",
    summary: "Tutte le carte, con ricerca avanzata, e il banco dove forgi i tuoi deck.",
    phase: 2,
    planned: [],
    emblem: "anvil",
    themeCard: { id: 60764609, name: "Fiendsmith Engraver" },
  },
  {
    id: "shelf",
    label: "Scaffale",
    title: "La Bottega",
    summary: "I tuoi deck in deck box 3D personalizzabili, con le tre carte vetrina.",
    phase: 3,
    planned: ["Colore, materiale e forma del deck box", "Carte vetrina animate", "Badge di legalità sempre aggiornato"],
    emblem: "shelf",
    themeCard: { id: 21744288, name: "Witchcrafter Schmietta" },
  },
  {
    id: "stats",
    label: "Lab Statistico",
    title: "Il Laboratorio",
    summary: "Probabilità esatte, simulazioni Monte Carlo e ottimizzazione del deck.",
    phase: 4,
    planned: ["Calcolatore con condizioni composte", "Monte Carlo con Pot e ricerche", "Ottimizzatore delle copie", "Registro partite"],
    emblem: "flask",
    themeCard: { id: 85668449, name: "Brain Research Lab" },
  },
  {
    id: "combo",
    label: "Combo Lab",
    title: "Il Circolo Arcano",
    summary: "Da una mano o da una carta, il motore esplora tutte le linee e classifica le board finali.",
    phase: 6,
    planned: [
      "Esplorazione automatica di tutte le linee",
      "Classifica delle board finali per forza",
      "Stress test con le handtrap",
      "Spiegazione animata passo per passo",
    ],
    emblem: "rune",
    themeCard: { id: 86120752, name: "Aleister the Invoker" },
  },
  {
    id: "news",
    label: "Novità",
    title: "Il Banditore",
    summary: "Nuove carte, banlist e punti Genesys, con l'impatto sui tuoi deck.",
    phase: 2,
    planned: ["Changelog delle carte", "Differenze tra banlist", "Avvisi sui deck non più legali"],
    emblem: "bell",
  },
  {
    id: "settings",
    label: "Impostazioni",
    title: "Impostazioni",
    summary: "Aspetto, audio, effetti e dati.",
    phase: 1,
    planned: [],
    emblem: "cog",
  },
];

export function sectionById(id: SectionId): Section {
  return sections.find((s) => s.id === id) ?? sections[0];
}
