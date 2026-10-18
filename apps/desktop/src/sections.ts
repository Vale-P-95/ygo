export type SectionId = "overview" | "deck-builder" | "shelf" | "stats" | "combo" | "news" | "settings";

export interface Section {
  id: SectionId;
  label: string;
  /** Roadmap phase that delivers the section (see docs/PIANO.md §11). */
  phase: number;
  summary: string;
  planned: string[];
  /** SVG path data, 24x24 viewBox, stroked. */
  icon: string;
}

export const sections: Section[] = [
  {
    id: "overview",
    label: "Panoramica",
    phase: 0,
    summary: "Stato del motore di gioco e dei dati, e avanzamento del progetto.",
    planned: [],
    icon: "M3 12l9-8 9 8M5 10v10h14V10",
  },
  {
    id: "deck-builder",
    label: "Deck Builder",
    phase: 2,
    summary: "Tutte le carte con ricerca avanzata, banlist aggiornate e validazione per formato.",
    planned: [
      "Ricerca con filtri e linguaggio di query",
      "Drag & drop tra Main, Extra e Side Deck",
      "Validazione Master Duel, TCG, OCG e Genesys",
      "Import/export .ydk e ydke://",
    ],
    icon: "M4 5h7v14H4zM13 5h7v6h-7zM13 13h7v6h-7z",
  },
  {
    id: "shelf",
    label: "Scaffale",
    phase: 3,
    summary: "I tuoi deck in deck box 3D personalizzabili, con le tre carte vetrina.",
    planned: ["Colore, materiale e forma del deck box", "Carte vetrina animate", "Badge di legalità sempre aggiornato"],
    icon: "M4 20V8l4-3 4 3v12M12 20V6l4-2 4 2v14M3 20h18",
  },
  {
    id: "stats",
    label: "Lab Statistico",
    phase: 4,
    summary: "Probabilità esatte, simulazioni Monte Carlo e ottimizzazione del deck.",
    planned: ["Calcolatore con condizioni composte", "Monte Carlo con Pot e ricerche", "Ottimizzatore delle copie", "Registro partite"],
    icon: "M4 20V10M10 20V4M16 20v-7M22 20H2",
  },
  {
    id: "combo",
    label: "Combo Lab",
    phase: 6,
    summary: "Il motore trova le combo di una mano, le classifica e le spiega passo per passo.",
    planned: ["Ricerca delle combo dalla mano", "Punteggio della board finale", "Stress test con le handtrap", "Spiegazione animata"],
    icon: "M13 2L4 14h7l-1 8 9-12h-7z",
  },
  {
    id: "news",
    label: "Novità",
    phase: 2,
    summary: "Nuove carte, banlist e punti Genesys, con l'impatto sui tuoi deck.",
    planned: ["Changelog delle carte", "Differenze tra banlist", "Avvisi sui deck non più legali"],
    icon: "M6 8a6 6 0 1112 0c0 7 3 9 3 9H3s3-2 3-9M10 21h4",
  },
  {
    id: "settings",
    label: "Impostazioni",
    phase: 1,
    summary: "Lingua, tema, audio, aggiornamenti e cartella dei dati.",
    planned: ["Tema chiaro/scuro", "Volumi per categoria", "Frequenza degli aggiornamenti"],
    icon: "M12 15a3 3 0 100-6 3 3 0 000 6zM19 12l2-1-1-3-2 .5-1.5-1.5.5-2-3-1-1 2h-2l-1-2-3 1 .5 2L6 8.5 4 8l-1 3 2 1v0l-2 1 1 3 2-.5L7.5 17 7 19l3 1 1-2h2l1 2 3-1-.5-2 1.5-1.5 2 .5 1-3z",
  },
];
