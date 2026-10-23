# Piano: Mappa dei collegamenti F2 — Rifiniture percorso, carte chiave e mano in confronto

Questo piano definisce i ritocchi finali della fase F2 per la mappa dei collegamenti (`docs/plans/mappa-collegamenti.md`, `docs/TASKS.md`).
La maggior parte dell'infrastruttura è già stata completata con le PR #22 e #31 (cerchio SVG interattivo, vista a colonne, bozza immediata e progresso in tempo reale, cruscotto di confronto versioni con differenze dinamiche, ricerca percorso BFS e ispettore dettagli).

Il lavoro è circoscritto esclusivamente a ciò che manca davvero, senza inventare funzionalità non richieste.

---

## 1. Percorso più corto e ponte verso il Combo Lab

### Cosa c'è già:
- `LinksCardDetail.tsx` include la funzione BFS `findShortestPath` e un selettore della carta d'arrivo, mostrando la sequenza di passi `A → ... → B` e il pulsante per avviare la ricerca nel Combo Lab.

### Cosa manca e cosa fare:
1. **Ponte completo Starter → Obiettivo:**
   - In `LinksTab.tsx` e `LinksCardDetail.tsx`, aggiornare la firma della prop `onGoCombo` da `(cardId: number) => void` a `(targetGoalCardId: number, startCardId?: number) => void`.
   - Quando l'utente clicca sul pulsante «⚡ Esplora combo con obiettivo [carta d'arrivo]», passare sia `targetCode` (l'obiettivo) sia `card.code` (lo starter selezionato).
   - `Lab.tsx` (`exploreComboAsGoal`) riceve già entrambi gli argomenti e memorizza in `localStorage`: `mode: "card"`, `card: startCardId`, `goal: [{ code: targetGoalCardId, zone: "any" }]`. In questo modo il Combo Lab si apre già configurato per cercare la combo che parte esattamente da quella carta per arrivare a quell'obiettivo.
2. **Navigazione rapida sui passi intermedi:**
   - Nella lista visiva dei passi `A → B → C`, rendere cliccabili i chip delle carte intermedie (con feedback audio e tooltip) per permettere all'utente di saltare all'ispezione della carta intermedia.
3. **Evidenziazione del percorso nel cerchio SVG:**
   - In `LinksCircle.tsx`, quando è attivo un percorso verso un'altra carta, evidenziare sul cerchio la catena di archi e i nodi intermedi del percorso con tratto dedicato e opacità piena, offrendo una comprensione spaziale immediata della rotta.

---

## 2. Visibilità delle carte chiave (`LinkCard.key`)

### Cosa c'è già:
- `LinkCard.key` (perdita media di carte raggiunte se la carta viene negata) è calcolata dal backend, mostrata nell'ispettore `LinksCardDetail` e riassunta nei primi 4 chip di `LinksStats.tsx`.

### Cosa manca e cosa fare:
1. **Dimensione dinamica dei nodi nel cerchio (`LinksCircle.tsx`):**
   - Attualmente la dimensione varia solo di 2px (`c.key > 0.2 ? 14 : 12`).
   - Scalare dinamicamente il raggio dei nodi in base a `c.key`: da un raggio base di 11px fino a 18px per i colli di bottiglia critici (es. `11 + Math.min(7, Math.round(c.key * 6))`).
   - I nodi con impatto rilevante (`key >= 0.2`) avranno un anello dorato di enfasi (`var(--df-gold)`) per risaltare immediatamente come "hub" nevralgici del mazzo.
2. **Riconoscimento nella vista a colonne (`LinksColumns.tsx`):**
   - Mostrare accanto alle statistiche di ciascuna carta il badge `★ −X.XX` per le carte con `key > 0.05`, rendendo visibile la vulnerabilità alle negazioni anche nella suddivisione Starter / Ponti / Extra.

---

## 3. Mano di prova sovrapposta e modalità confronto

### Cosa c'è già:
- Sovrapposizione di 5 o 6 carte casuali con calcolo delle carte raggiungibili (`reachableFromHand`) ed evidenziazione dei nodi/frecce attraversate.

### Cosa manca e cosa fare:
1. **Estrazione dal mazzo corretto in confronto:**
   - In `LinksTab.tsx`, se la modalità confronto è attiva ed è impostata su `isReversed`, la mano deve essere estratta dalle carte del mazzo effettivamente mostrato (`activeData.cards` non Extra) invece che rigidamente da `deck.main`.
2. **Armonizzazione con le frecce di confronto (`LinksCircle.tsx`):**
   - Quando la mano di prova è sovrapposta durante un confronto, filtrare i collegamenti guadagnati (`gained`, verde) e persi (`lost`, rosso):
     - I collegamenti guadagnati raggiungibili dalla mano attuale restano illuminati in `var(--df-ok)`.
     - Quelli non raggiungibili dalla mano attuale vengono attenuati (`opacity: 0.05`).
     - In questo modo l'utente vede all'istante: *"con questa specifica mano iniziale, quali nuove opzioni/frecce guadagnate posso sfruttare?"*.
3. **Dettaglio nel banner della mano:**
   - Nel banner riassuntivo della mano di prova, se il confronto è aperto, mostrare il numero di carte raggiunte distinguendo quante sono nuove (+).

---

## 4. File coinvolti e assenza di modifiche vietate
- **File da modificare**:
  - `apps/desktop/src/components/lab/links/LinksTab.tsx`
  - `apps/desktop/src/components/lab/links/LinksCircle.tsx`
  - `apps/desktop/src/components/lab/links/LinksCardDetail.tsx`
  - `apps/desktop/src/components/lab/links/LinksColumns.tsx`
  - `apps/desktop/src/components/lab/links/LinksStats.tsx`
- **Nessuna modifica** a `crates/`, `src-tauri/`, `vendor/` o ai tipi condivisi in `api.ts`.
- **Colori**: solo token CSS (`var(--df-ok)`, `var(--df-danger)`, `var(--df-gold)`, `var(--sec)`, ecc.).
- **Protocollo**: verifiche complete con `pnpm lint`, `pnpm typecheck`, `pnpm build`.
