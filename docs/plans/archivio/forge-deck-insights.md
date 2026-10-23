# Piano: La Fucina connessa — Ruoli sul banco, Mano di prova a 5 ruoli e Ponte col Circolo Arcano (`gemini/forge-deck-insights`)

## Obiettivo
Collegare **La Fucina (`DeckBuilder`)**, **La Mano di prova (`TestHand`)**, **Il Laboratorio** e **Il Circolo Arcano**:
1. Mostrare sul banco della Fucina i conteggi dei ruoli (`Starter`, `Estensore`, `Handtrap`, `Board breaker`, `Brick` + ruoli personalizzati), permettere di cliccare un ruolo per evidenziarne le carte nel Main/Side/Maybe Deck e lanciare `✦ Riconosci` direttamente dal banco.
2. Aggiungere il pannello a comparsa `📊 Distribuzione` con la curva dei Livelli (1–12) dei mostri del Main Deck, i sottotipi di Magie/Trappole e la curva Link/Ranghi/Livelli dell'Extra Deck.
3. Estendere `TestHand.tsx` a tutti i 5 ruoli di `BUILTIN_ROLES` (inclusa la probabilità esatta di aprire **Senza Brick**) e aggiungere il pulsante **`✦ Prova nel Circolo Arcano`** che salva la mano pescata nel formato `Setup` di `Combo.tsx` e apre direttamente il Circolo Arcano.

## Avvertenze tecniche importanti
1. **Clic su `✦ Riconosci` (`api.guessDeckRoles`)**:
   - `guessDeckRoles(id)` lavora sul deck salvato nel database. Nella Fucina il salvataggio automatico ha un debounce di 700 ms (`dirty.current`) e un deck appena creato potrebbe avere `idRef.current == null`.
   - Prima di invocare `api.guessDeckRoles(id)`, se `dirty.current` è vero o `idRef.current == null`, salviamo immediatamente il deck con `api.saveDeck({ ...deckRef.current, id: idRef.current })` e aggiorniamo `idRef.current` e `rememberDeck(saved.id)`.
   - Quando arrivano i `RoleGuess[]`, applichiamo la stessa logica di `Lab.tsx`: sostituiamo solo i `BUILTIN_ROLES` delle carte provate (`tried`), preserviamo eventuali ruoli personalizzati, salviamo con `api.setDeckRoles(id, next)` e teniamo `previousRoles` per permettere `Annulla riconoscimento`.
2. **Formato della mano salvata per `Combo.tsx` (`duelforge.combo.${deckId}`)**:
   - `Combo.tsx` legge `localStorage.getItem(\`duelforge.combo.\${id}\`)` con la struttura `Setup`:
     `{ mode: "hand", hand: number[], card: number | null, keep: number[], forbid: number[], goal: GoalCard[], goalAll: boolean, goalFirst: boolean }`.
   - `hand` in `Combo.tsx` accetta al massimo 6 carte (di norma le prime 5 carte d'apertura del primo turno: `hand.slice(0, 6)`), preservando eventuali `forbid`, `goal`, `goalAll`, `goalFirst` già salvati per quel deck e filtrando `keep` alle sole carte presenti nella nuova `hand` (o pre-impostando `keep` sulle handtrap pescate se presenti).
   - Se il deck nella Fucina non era ancora salvato o aveva modifiche pendenti, viene salvato prima di navigare a `combo` così `Combo.tsx` carica lo stesso `deck.id`.

## File coinvolti
- `apps/desktop/src/components/DeckInsightsBar.tsx` (nuovo)
- `apps/desktop/src/components/DeckZone.tsx`
- `apps/desktop/src/components/TestHand.tsx`
- `apps/desktop/src/pages/DeckBuilder.tsx`
- `apps/desktop/src/App.tsx`
