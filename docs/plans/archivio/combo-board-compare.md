# Piano: Confronto affiancato tra due board nel Circolo Arcano (`gemini/combo-board-compare`)

## Obiettivo
Permettere al giocatore nel **Circolo Arcano** di fissare una qualsiasi board della classifica come riferimento (`⇄ Confronta`) e confrontarla fianco a fianco con un'altra board, usando esclusivamente i dati già presenti in `ComboBoard` e `ComboScore` e le definizioni dinamiche in `src/lib/scoring.ts` (senza alcuna soglia o peso fisso nell'interfaccia).

## File coinvolti
1. `apps/desktop/src/components/combo/BoardCompareModal.tsx` (nuovo):
   - Finestra modale di confronto affiancato tra **Board A** (fissata) e **Board B** (confrontata), con pulsante per scambiare A ↔ B.
   - Mostra:
     - **Tappetino affiancato (`Field`)**: le due board disposte fianco a fianco (EMZ, Zone Mostri, Zone Magie/Trappole, Terreno, Cimitero, Bandite, Extra Deck) e le carte rimaste in mano.
     - **Differenze tra le carte (Campo, Mano, Cimitero/Bandite)**: quali carte sono presenti **Solo in Board A**, **In entrambe** e **Solo in Board B**.
     - **Confronto delle metriche e del punteggio** (quando misurato dal turno di prova): legge dinamicamente `METRICS`, `NEGATIVE`, `score(s, weights)` e `parts(s, weights)` da `src/lib/scoring.ts` per mostrare i valori grezzi e i punti pesati con le differenze (`Δ`) senza pesi né soglie cablate.
     - **Cosa ferma l'avversario, Floodgate, Board breaker ed Esposizione**: confronto diretto degli effetti d'interruzione (`s.details`), dei blocchi passivi (`s.floodgateList`), dei board breaker (`s.breakers`) e dell'esposizione alle handtrap (`s.exposureDetail`), oltre al costo in carte usate (`cardsUsed`) e lunghezza della linea (`choices`).
2. `apps/desktop/src/pages/Combo.tsx`:
   - Aggiunge lo stato `pinnedBoard: ComboBoard | null` e `compareTarget: ComboBoard | null`.
   - Su ogni riga della classifica delle board e nell'intestazione del pannello di dettaglio della board selezionata aggiunge il pulsante rapido `⇄ Confronta` (o `Fissata` / `⇄ Confronta con #N`).
   - Mostra una barra compatta quando una board è fissata per il confronto, con accesso immediato alla finestra `BoardCompareModal`.

## Vincoli
- Nessuna modifica ai tipi di `apps/desktop/src/lib/api.ts`, né a `crates/` o `apps/desktop/src-tauri/`.
- Nessuna soglia o peso fisso nel componente di confronto: tutto passa da `METRICS`, `NEGATIVE`, `score` e `parts` di `src/lib/scoring.ts` e dai dati già in `ComboBoard` / `ComboScore`.
