# Piano: Backend finto per sviluppare l'interfaccia nel browser (`VITE_MOCK=1`)

## Obiettivo
Permettere lo sviluppo e la revisione dell'interfaccia React direttamente nel browser (`VITE_MOCK=1 pnpm dev` in `apps/desktop`) senza compilare il backend Tauri/Rust, restituendo dati realistici per tutte le sezioni dell'app e mantenendo invariati i tipi esportati da `apps/desktop/src/lib/api.ts`.

## File coinvolti
- `apps/desktop/src/lib/mock.ts` *(nuovo)*: dataset realistico in memoria, implementazione di tutte le funzioni di `api` e generatore di illustrazioni segnaposto SVG (`data:image/svg+xml`) per le carte quando non c'è il protocollo `cardimg.localhost` di Tauri.
- `apps/desktop/src/lib/api.ts`: attivazione della modalità mock quando `import.meta.env.VITE_MOCK === "1"` (o `"true"`), impostando `isDesktop = isTauri() || isMock` e delegando `api` e `cardImage` a `mock.ts`. Nessun tipo di `api.ts` viene modificato.

## Passi di implementazione

### Passo 1 — Catalogo carte, immagini segnaposto, Fucina e Bottega
1. Creare `apps/desktop/src/lib/mock.ts` con:
   - Un catalogo realistico di ~35 carte (archetipo Snake-Eye / Fire King, mostri Extra Deck ricorrenti come *Promethean Princess*, *S:P Little Knight*, *Apollousa*, *Baronne de Fleur*, *Raging Phoenix*, * Amblowhale* e handtrap/staple come *Ash Blossom*, *Maxx "C"*, *Infinite Impermanence*, *Nibiru*, *Called by the Grave*, *Crossout Designator*).
   - Due deck completi in memoria (40 carte Main Deck, 15 Extra Deck, Side Deck, Maybe board, ruoli pre-assegnati e configurazione `DeckBox` 3D).
   - Un generatore `mockCardImage(id, size)` che produce un SVG leggero colorato in base al frame della carta con nome, statistiche e codice, così tutte le anteprime nella UI si vedono senza errori di rete nel browser.
   - Implementazione dei metodi dati, catalogo e deck (`dataStatus`, `updateData`, `onProgress`, `search`, `getCards`, `facets`, `listFavorites`, `setFavorite`, `listSavedSearches`, `saveSearch`, `deleteSavedSearch`, `listDecks`, `shelfDecks`, `getDeck`, `saveDeck`, `deleteDeck`, `duplicateDeck`, `setDeckBox`, `validateDeck`, `importDeck`, `exportDeck`).
2. Collegare `mockApi` e `mockCardImage` in `apps/desktop/src/lib/api.ts` dietro il flag `VITE_MOCK`.
3. Eseguire `pnpm typecheck` e `pnpm build`, committare e pushare.

### Passo 2 — Laboratorio statistico e Registro partite
1. Aggiungere in `apps/desktop/src/lib/mock.ts` le funzioni del Laboratorio:
   - `labOverview`, `labProbability`, `setDeckRoles`, `guessDeckRoles`, `onRolesProgress`.
   - `labSimulate`, `labCancelSimulation`, `onSimProgress`, `labAnalyze`, `labOptimize`, `sampleSize`.
   - `listRecipes`, `saveRecipe`, `deleteRecipe`.
   - `matchLog`, `logMatch`, `updateMatch`, `deleteMatch`, `exportMatches`, `importMatches` con uno storico di partite, versioni del deck, matrici di matchup e trend.
2. Eseguire `pnpm typecheck` e `pnpm build`, committare e pushare.

### Passo 3 — Combo Lab (ricerca, board, punteggi e spiegazione)
1. Aggiungere in `apps/desktop/src/lib/mock.ts` le funzioni del Combo Lab:
   - `comboSearch`, `comboContinue`, `comboCancel`, `onComboProgress` con più mani (`ComboHand`) e più board finali (`ComboBoard` con mostri nelle zone Main/Extra, carte coperte/scoperte nelle zone Magie/Trappole e Terreno, Cimitero, bandite e mano finale).
   - `comboScore`, `comboScores`, `onComboScore` con valutazioni complete (`ComboScore`: interruzioni dettagliate `ComboInterruption[]`, tenuta ai board breaker `ComboBreaker[]`, esposizione `exposureDetail`, risorse, follow-up e note esplicative).
   - `comboExplain` con la sequenza passo-passo (`ComboStep[]` e `ComboHappening[]`) coerente con le carte della board selezionata.
2. Eseguire `pnpm typecheck` e `pnpm build`, committare e pushare.

### Passo 4 — Verifica nel browser e Pull Request
1. Avviare `pnpm dev` con `VITE_MOCK=1` e verificare il funzionamento di tutte le viste nel browser.
2. Aggiornare `docs/TASKS.md`, committare, pushare e aprire la PR con `gh pr create --base main --head gemini/mock-backend --fill`.
