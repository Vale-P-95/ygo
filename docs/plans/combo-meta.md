# Piano: «Confronta con il meta» (L2-3) nel Combo Lab

Riferimenti: `docs/TASKS.md` (Corsia 8, L2-3 e richiesta per Gemini), PR #105 (backend su `main`).

## Obiettivo
Aggiungere nel Combo Lab il pulsante e la visualizzazione per il confronto delle migliori board con i 3 mazzi più giocati del meta (da Master Duel Meta), riordinando le board per forza contro il meta (`ComboScore.meta.value`) e mostrando il dettaglio per ciascun mazzo avversario (`perDeck`: nome, peso, `left`).

## 1. Contratto (`api.ts` e `mock.ts`)
- In `apps/desktop/src/lib/api.ts`:
  ```ts
  /** Measures the best boards (or given ones) against the 3 most played decks of the meta (L2-3): takes about a minute the first time, then ~20s a board. */
  comboMeta: (boards?: { hand: number; key: string }[]) =>
    invoke<number>("combo_meta", { boards: boards ?? null }),
  ```
- In `apps/desktop/src/lib/mock.ts`:
  Aggiungere `comboMeta`: restituisce un conteggio mockato ed emette eventi `combo-score` arricchiti con `meta: ComboMeta` per le board coinvolte.

## 2. Store `combo.ts`
- Aggiungere al tipo `ComboRun`:
  - `metaLoading: boolean`
  - `metaError: string | null`
- Aggiungere la funzione `compareMeta(boards?: { hand: number; key: string }[]): Promise<number>`:
  - Sottoscrive `api.onComboScore` per aggiornare `run.scores` in tempo reale man mano che le board vengono misurate.
  - Imposta `metaLoading: true` e `metaError: null`.
  - Chiama `api.comboMeta(boards)`.
  - Cattura eventuali errori (di rete, mancanza deck) e li memorizza in italiano.
  - Al termine disiscrive l'evento e reimposta `metaLoading: false`.

## 3. Combo Lab (`apps/desktop/src/pages/Combo.tsx`)
- **Pulsante «Confronta con il meta»:**
  - Posizionato accanto ai controlli di misura (`Contro:`, `Rimisura`, ecc.) quando la valutazione di base è completata (`totalMeasuredCount > 0` e non in ricerca/misura).
  - Chiama `compareMeta(top4Boards)` passando le migliori 4 board secondo i pesi attuali scelti dall'utente.
  - Durante l'esecuzione mostra lo stato di caricamento e disabilita ulteriori chiamate.
- **Messaggio di attesa:**
  - Mostra chiaramente: *"Il confronto con il meta è in corso. La prima volta richiede circa 1 minuto (lettura del meta da Master Duel Meta e delle linee dei 3 deck più giocati), poi circa 20 s per board."*
- **Ordinamento prioritario delle board:**
  - Le board con `meta.value != null` (e senza errori) vengono ordinate prima di tutte le altre, ordinate in modo decrescente per `meta.value`.
  - Le board senza `meta.value` mantengono il loro ordinamento per punteggio pesato / matchup / stima.
- **Badge nelle tessere board:**
  - Mostra il chip compatto `meta: XX%` per le board valutate contro il meta.
- **Protocollo interfaccia e azzeramento:**
  - Id di richiesta (`requestIdRef`) per scartare risposte non attuali.
  - Al cambio deck o a una nuova ricerca, i riferimenti e gli stati meta si azzerano.

## 4. Scheda del punteggio (`apps/desktop/src/components/combo/Score.tsx`)
- Sezione dedicata `Confronto con il meta (L2-3)`:
  - Valore complessivo pesato `meta.value` (in %) e `meta.left` (mosse lasciate all'avversario).
  - Lista dettagliata `perDeck`:
    - Nome del deck e percentuale di popolarità (`weight`).
    - Percentuale mosse fermate (`1 - left`) e residue (`left`).
    - Avviso chiaro se un deck non è stato contato (`left: null` / `made < 0.8`).

## 5. Verifiche
- Unit test con Vitest in `apps/desktop/src/components/combo/Score.test.ts` e `apps/desktop/src/lib/combo.test.ts`.
- `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build` (137 test passati, 0 errori).
- Verifica dell'ordinamento prioritario per `meta.value` e rendering del dettaglio per deck (`perDeck` con gestione `left: null`).

## 6. Risultati dell'implementazione
- `api.ts` e `mock.ts`: aggiunto `comboMeta` conforme al contratto backend di `lib.rs` (PR #105).
- `combo.ts`: gestisce `metaLoading`, `metaError`, `clearMetaError`, `compareMeta` con requestId e isolamento per deck.
- `Score.tsx`: componente `MetaSection` che mostra l'efficacia complessiva del meta, il residuo medio e il breakdown per ciascun deck con etichetta «non conta» per `left: null`.
- `Combo.tsx`: pulsante «Confronta con il meta» posizionato nella toolbar post-valutazione, banner esplicito di attesa con stima dei tempi (1 min la prima volta, poi ~20s/board), gestione errori in italiano, azzeramento immediato al cambio deck o avvio di una nuova ricerca, chip `meta: XX%` sulle board misurate, e ordinamento che colloca le board con `meta.value` in cima alla lista ordinate per efficacia decrescente.
