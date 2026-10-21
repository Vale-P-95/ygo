# Mappa dei collegamenti quasi in tempo reale e confronto tra versioni: piano

*Scritto il 21/10/2026 da Gemini (Antigravity). Task 3 di `docs/TASKS.md` (approvato dal proprietario il 21/10).*
*Backend già implementato in `main` da Claude (`api.onLinksProgress` con `map` e `deckId`, `CardLink.text`, `api.deckLinksCompare` → `LinksComparison`).*

---

## 1. Obiettivi

1. **Mappa quasi in tempo reale**:
   - `api.onLinksProgress` invia `{ deckId, done, total, map }`.
   - Con `done === 0`, `map` è la **bozza letta dal testo delle carte**, disponibile all'istante (~0,2s).
   - Mostrare la mappa subito appena arriva la bozza, senza attendere il completamento della simulazione.
   - Aggiornare la mappa a ogni evento di avanzamento progressivo (filtrando per `deckId`), mantenendo una **barra di avanzamento in alto** durante il caricamento senza oscurare il grafico.
2. **Collegamenti «dal testo»**:
   - Quando `CardLink.text` è vero e `possible` è falso: il collegamento proviene esclusivamente dal testo della carta ed è un effetto che le linee combo cercate non sfruttano.
   - Resa grafica più tenue: tratto più sottile e tratteggiato/puntinato con opacità ridotta.
   - Nel pannello dettagli (`LinksCardDetail.tsx`) e nei tooltip: badge o dicitura «dal testo» con spiegazione chiara.
3. **Confronto tra versioni del deck (`api.deckLinksCompare(deckIdA, deckIdB)`)**:
   - Selettore per confrontare il deck attivo con un altro deck della Bottega (es. versione modificata o precedente).
   - Gestione asincrona sicura con `requestIdRef` ed eventi di avanzamento filtrati per `deckId`.
   - Panoramica del confronto:
     - Raggiungibilità media prima → dopo (`reachedBefore` → `reachedAfter`, es. `18,02 → 17,55` con delta colorato).
     - Conteggio e chip delle carte aggiunte (+) e tolte (-).
     - Conteggio dei collegamenti guadagnati (verdi) e persi (rossi).
   - Visualizzazione mappa in modalità confronto:
     - Collegamenti guadagnati in verde (`#22c55e`).
     - Collegamenti persi in rosso (`#ef4444`).
   - Elenco interattivo delle carte modificate (`changed`: `chanceBefore` → `chanceAfter`, `keyBefore` → `keyAfter`, `reachAllBefore` → `reachAllAfter`).
4. **Mock completo per lo sviluppo e i test (`mock.ts`)**:
   - Simulazione di `onLinksProgress` con bozza immediata a `done: 0` e mappe parziali con `deckId`.
   - Implementazione fedele di `deckLinksCompare` con calcolo dinamico delle differenze e delle carte cambiate.

---

## 2. Dettaglio delle modifiche all'interfaccia

### 2.1 Gestione stato e progresso in `Lab.tsx`
- In `runLinks(targetDeckId)`:
  - Salvare l'id richiesta in `linksReqIdRef.current`.
  - Nel listener `api.onLinksProgress(p)`:
    - Verificare `p.deckId == null || p.deckId === targetDeckId`.
    - Verificare `linksReqIdRef.current === reqId`.
    - Se `p.map` è definito:
      - Aggiornare immediatamente `setLinksData(p.map)`.
      - Inviare la richiesta immagini/dati carte `request(p.map.cards.map((c) => c.code))`.
    - Aggiornare `setLinksProgress({ done: p.done, total: p.total })`.
  - All'arrivo della risposta definitiva di `api.deckLinks(targetDeckId)`:
    - Impostare `setLinksData(res)`.
    - Azzerare `linksProgress` e impostare `linksLoading = false`.

### 2.2 Barra di avanzamento contestuale in `LinksTab.tsx`
- Quando `loading` è vero ma `data` è già presente (grazie alla bozza o agli aggiornamenti progressivi):
  - **Non rimpiazzare** la schermata con il loader vuoto.
  - Mostrare invece una striscia di progresso sottile e animata subito sotto l'intestazione:
    - Se `progress.done === 0`: «Bozza istantanea dal testo delle carte · Simulazione mani del motore in corso…»
    - Se `progress.done > 0`: «Aggiornamento progressivo: {progress.done} / {progress.total} mani analizzate…»
  - Consentire all'utente di interagire normalmente con la mappa mentre il motore continua ad arricchirla.

### 2.3 Visualizzazione collegamenti «dal testo»
- In `types.ts` / `LinksCircle.tsx` / `LinksColumns.tsx`:
  - Rilevare `isTextOnly = !l.possible && Boolean(l.text)`.
  - In SVG:
    - `strokeDasharray`: per i collegamenti solo dal testo usare pattern puntinato `"2 3"`.
    - `strokeWidth`: più sottile (1 o 1.2px).
    - `opacity`: più tenue (es. 0.35 quando focalizzato, 0.08 a riposo).
- In `LinksCardDetail.tsx`:
  - Nei blocchi dei collegamenti uscenti ed entranti:
    - Per i collegamenti con `isTextOnly`, mostrare un pill/etichetta `«dal testo»` e un tooltip «Questo effetto è presente nel testo della carta, ma le linee combo cercate non lo utilizzano».

### 2.4 Modalità confronto versioni (`LinksComparisonView`)
- Componente o pannello `LinksCompare.tsx` integrato in `LinksTab.tsx`:
  - Toggle / pulsante di intestazione: «Confronta versione» con selettore del secondo deck (tra i deck disponibili nella Bottega, escludendo quello attuale).
  - Chiamata ad `api.deckLinksCompare(deck.id, compareDeckId)` protetta da `compareReqIdRef`.
  - Barra riassuntiva:
    - Carte aggiunte (+ N) e tolte (- N) con chip cliccabili.
    - Collegamenti guadagnati (+ N, verde) e persi (- N, rosso).
    - Raggiungibilità media: `reachedBefore.toFixed(2)` → `reachedAfter.toFixed(2)` con delta `(Δ +X.XX)`.
  - Tabella delle carte cambiate:
    - Mostra ogni carta in `comparison.changed`, con confronto visivo delle probabilità `chance`, del valore chiave `key` e della raggiungibilità `reachAll`.
    - Cliccando su una carta la si seleziona nell'ispettore della mappa.
  - Opzione di visualizzazione mappa con i link guadagnati/persi evidenziati.

---

## 3. Mock e fixture (`apps/desktop/src/lib/mock.ts`)

1. **`deckLinks` progressivo**:
   - Emissione a `t=0` di un evento `onLinksProgress` con `done: 0, total: 30, map: draftMap`, dove i collegamenti hanno `text: true`, alcuni con `possible: false, used: 0`.
   - Emissione a step successivi di `map` aggiornate e `done: 10, 20, 30`.
2. **`deckLinksCompare` realistico**:
   - Calcolo esatto dei link `gained`, `lost`, delle carte `added`, `removed`.
   - Calcolo delle differenze per le carte in comune (`changed` con delta >= 0.02 su `chance` o `key` o variazione su `reachAll`).
   - Calcolo di `reachedBefore` e `reachedAfter`.

---

## 4. Criteri di accettazione e verifica

- Zero errori TypeScript (`pnpm typecheck`).
- Zero warning ESLint (`pnpm lint`).
- Build frontend completata con successo (`pnpm build`).
- Formattazione Rust intatta (`cargo fmt --all -- --check`).
- Protocollo interfaccia rispettato:
  - `requestIdRef` su tutte le chiamate asincrone.
  - Messaggi di errore in italiano sempre visibili all'utente.
  - Nessun `as any`, `@ts-ignore` o cast forzato.
  - Pulizia listener ed effetti completa.
