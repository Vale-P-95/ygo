# Piano: Revisione dell'interfaccia, Passo 4 — Passata su tutte le sezioni

## Obiettivo
Eseguire una passata di rifinitura e pulizia visiva su tutte le sezioni di DuelForge (Fucina, Bottega, Laboratorio con tutte le schede, Circolo Arcano, Banditore, Impostazioni), applicando coerentemente i principi di `docs/plans/ui-revisione-2.md`:
1. **Tipografia leggibile**: nessuna scritta sotto i 12 px (salvo badge compatti a 11 px), sostituzione di `text-[9px]`, `text-[10px]`, `text-[11px]` sparsi con token conformi (`text-xs` o `text-caption` / `text-small`).
2. **Eliminazione testi fissi d'aiuto**: spostamento delle istruzioni ingombranti in tooltip, popover contestuali (`HelpPopover`) o banner discreti.
3. **Barre di comando snelle**: riduzione a max 6-7 comandi visibili per barra, raggruppamento delle azioni secondarie in menu e adozione di `lucide-react` per comandi con semantica consolidata (es. Undo, Redo, Ricerca, Confronta).
4. **Didascalie decorative («Ispirato a…»)**: mantenute solo dove non rubano spazio (`hidden xl:block` o rimosse se in conflitto con i comandi del banner) e con dimensione minima a 12 px (`text-xs`).
5. **Schede del Laboratorio**: arricchite con icone esplicative per ciascuna scheda e gestione reattiva a menu «Altro ▾» quando lo spazio orizzontale non basta per tutte e 7 le schede.

---

## Tabella degli interventi per sezione

### 1. Fucina (`DeckBuilder.tsx`, `DeckToolbar.tsx`, `DeckZone.tsx`)

| Elemento | Prima | Intervento | Dettaglio / Destinazione |
| :--- | :--- | :--- | :--- |
| **Comandi Undo/Redo** | Caratteri Unicode `↶` e `↷` | Icona Lucide | `Undo2` e `Redo2` con tooltip chiari e `aria-label` |
| **Pulsante Mano di prova** | Testo esteso in barra principale | Icona + Testo compatto | Icona `Dices` per armonizzare con la toolbar |
| **Stato salvataggio** | `text-[11px] text-faint` | Token conforme | `text-xs text-faint` |
| **Data banlist nel popover** | `text-[11px] text-faint` | Token conforme | `text-xs text-faint` |
| **Note Maybe board** | Testo fisso `Carte in prova: non contano nel deck e non si esportano` | Spostato in `HelpPopover` / tooltip | Libera spazio nella testata della Maybe board; testo accessibile nel popover `?` |
| **Didascalia Ispirato a...** | `text-[10px]` in SectionHeader | `text-xs text-faint`, `hidden xl:block` | Evita collisioni con i comandi di testata su schermi < 1440 px |

### 2. Bottega (`Shelf.tsx`, `BoxEditor.tsx`)

| Elemento | Prima | Intervento | Dettaglio / Destinazione |
| :--- | :--- | :--- | :--- |
| **Icona ricerca scaffale** | Carattere `⌕` manuale | Icona Lucide | `Search` integrata nel campo |
| **Chip ordinamento** | `!text-[11px]` | Token conforme | `text-xs font-medium` |
| **Pulsante Nuovo deck** | `＋ Nuovo deck` con carattere `＋` | Icona Lucide | `Plus` + `Nuovo deck` con pulsante standard `btn-molten` |
| **Vetrina in BoxEditor** | Testo fisso `senza scelta: le prime del Main` con `text-[11px]` | Spostato in tooltip | Testo sintetico `3/3 carte` con tooltip esplicativo sul fallback automatico |
| **Badge ruoli in BoxEditor** | `text-[10px]` per conteggi | Token conforme | `text-xs` leggibile e allineato |

### 3. Laboratorio (`Lab.tsx` e schede)

| Elemento | Prima | Intervento | Dettaglio / Destinazione |
| :--- | :--- | :--- | :--- |
| **Schede di navigazione** | Solo testo (7 schede affiancate: Panoramica, Collegamenti, Confronto, Calcolatore, Simulatore, Ottimizzatore, Registro partite) | Icone Lucide + raggruppamento «Altro ▾» | Icone dedicate (`BarChart2`, `Network`, `GitCompare`, `Calculator`, `Play`, `SlidersHorizontal`, `ScrollText`); su schermi piccoli o oltre le 4-5 schede principali, le schede secondarie collassano nel menu a tendina «Altro ▾» |
| **Pulsanti turno Primo/Secondo** | Testo esteso `Primo · 5` / `Secondo · 6` | Icone + etichetta compatta | `ArrowRightCircle` / `CircleDot` + numero carte |
| **Scritte statistiche/grafici** | Ricorrenze di `text-[10px]` e `text-[11px]` in `charts.tsx` e `ConditionPicker` | Token conformi | Passaggio a `text-xs` (12 px) garantendo WCAG contrasto |

### 4. Circolo Arcano / Combo Lab (`Combo.tsx`)

| Elemento | Prima | Intervento | Dettaglio / Destinazione |
| :--- | :--- | :--- | :--- |
| **Barra filtri board** | 6-8 elementi affastellati (Contro, select, Rimisura, Ordine vs deck, Pareto, Confronta meta, Pesi) | Riorganizzazione | Raggruppamento coerente: selezione avversario con icona `RefreshCw`, filtri Pareto / vs deck con toggle chiari, meta button prominente |
| **Icone e glifi** | Caratteri `↻`, `⇄`, `✕`, `★`, `⚠` | Icone Lucide | `RefreshCw`, `ArrowLeftRight`, `X`, `Sparkles`, `AlertTriangle` |
| **Testi fissi d'aiuto** | `riprende da dove si è fermata...`, spiegazione peso mani, testo riferimento fissato | Spostamento | In tooltip e `HelpPopover` per ridurre il rumore visivo |
| **Scritte < 12 px** | `text-[10px]`, `text-[11px]` su chip e badge percentuali | Token conformi | Aggiornamento a `text-xs` e badge `text-caption` (11 px minimo per numeri isolati) |

### 5. Il Banditore (`Banditore.tsx`)

| Elemento | Prima | Intervento | Dettaglio / Destinazione |
| :--- | :--- | :--- | :--- |
| **Schede principali** | Unicode `⚖`, `📜`, `🏛` nel testo | Icone Lucide | `Scale` (Confronto), `History` (Cosa è cambiato), `FolderHeart` / `Boxes` (Impatto deck) |
| **Barra filtri confronto** | Ricerca con `🔍` emoji e `✕` testo; chip divergenze con `⇄` testo | Icone Lucide | `Search`, `X`, `ArrowLeftRight` |
| **Badge formati e statistiche** | `text-[9px]`, `text-[10px]`, `text-[11px]` | Token conformi | Normalizzazione a `text-xs` (12 px) e `text-caption` per badge compatti |
| **Dettaglio divergenze** | Etichette ridondanti nei chip di stato | Semplificazione | Layout tabellare pulito con contrasto > 4.5:1 |

### 6. Impostazioni (`Settings.tsx`)

| Elemento | Prima | Intervento | Dettaglio / Destinazione |
| :--- | :--- | :--- | :--- |
| **Pulsanti test suoni** | Carattere Unicode `▶ {label}` | Icona Lucide | `Play` da 14 px prima del nome del suono |
| **Pulsante prova scintille** | Solo testo `Prova le scintille` | Icona Lucide | `Sparkles` per coerenza visiva |
| **Didascalie e note** | Layout standard già pulito | Rifinitura spaziature | Assicurata scala `text-xs` per note e hint |

---

## Piano di attuazione progressivo

1. **SectionHeader**: normalizzazione didascalia `Ispirato a...` (dimensione minima `text-xs`, reattività con visibilità su schermi ampi).
2. **Fucina (DeckToolbar & DeckZone)**: icone Lucide per Undo/Redo, note Maybe board spostate in popover/tooltip, rimozione `text-[11px]` sparsi.
3. **Bottega (Shelf & BoxEditor)**: icone Search e Plus, rimozione testi ridondanti nella vetrina, pulizia dimensioni font.
4. **Laboratorio (Lab)**: icone Lucide per le 7 schede e menu collassabile «Altro ▾» per schermi medi/piccoli.
5. **Circolo Arcano (Combo)**: icone Lucide per azioni (RefreshCw, ArrowLeftRight, Sparkles), rimozione testi fissi prolissi, normalizzazione font.
6. **Il Banditore (Banditore)**: sostituzione emoji/glifi con Lucide (Scale, History, Boxes, Search, ArrowLeftRight), pulizia `text-[9px]`/`text-[10px]`.
7. **Impostazioni (Settings)**: icone Play e Sparkles, controllo contrasti e token.
8. **Test e controlli**: esecuzione suite completa `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build`.
