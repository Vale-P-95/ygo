# Piano: Interfaccia del turno di prova contro un deck scelto (#44, Livello 1)

*Data: 21/10/2026 — Agente: Antigravity / Gemini — Branch: `gemini/matchup-turn`*

Riferimenti:
- Specifiche backend e numeri: [`docs/plans/turno-di-prova-matchup.md`](../turno-di-prova-matchup.md)
- Contratti API: [`apps/desktop/src/lib/api.ts`](../../../apps/desktop/src/lib/api.ts) (`ComboMatchup`, `ComboRivalBreaker`, `ComboRivalHandTrap`, `comboScore(vsDeckId?)`)
- Task list: [`docs/TASKS.md`](../../TASKS.md)

---

## 1. Obiettivi
1. **Selettore avversario nel Circolo Arcano (`Combo.tsx`)**:
   - Selettore posizionato vicino all'area di misurazione delle board: "Contro: Generico / [Deck della Bottega]".
   - Popolato dinamicamente tramite `api.listDecks()`, escludendo il deck attualmente analizzato se presente.
   - Quando viene selezionato un deck avversario, `api.comboScore(vsDeckId)` viene invocato per misurare o rimisurare le board contro quel deck.
   - Selezione memorizzata nella sessione o stato locale del Circolo Arcano.

2. **Dettaglio del punteggio e resilienza in `Score.tsx`**:
   - **Resilienza contro i breaker del deck avversario**:
     - Se `s.matchup` è presente:
       - Mostra la resilienza specifica contro il mazzo: valore atteso delle interruzioni rimaste (es. `0,54 su 1,00`) e probabilità complessiva che l'avversario peschi almeno un breaker giocabile tra 6 carte (`breakerChance`).
       - Se il mazzo avversario non ha breaker giocabili dalla mano (`standardBreakers = true`), mostra una nota esplicativa: *"Il deck avversario non ha board breaker giocabili dalla mano: valgono i breaker standard"*.
       - Lista dei breaker rivali (`breakers`): probabilità tra le 6 carte (`chance`), carta che lo nega (`stoppedBy`), valore residuo della board (`remaining`), e se regge / è negato.
   - **Esposizione alle handtrap del deck avversario ("Dove si può fermare la linea")**:
     - Se `s.matchup` è presente:
       - Box dedicato "Contro {deckName}": probabilità complessiva che l'avversario abbia almeno una handtrap che ferma la linea tra le 5 carte in mano (`handTrapChance`).
       - Handtrap che fermano la linea (`stops > 0`): momento/i in cui fermano (`${stops} su ${windows} momenti`), probabilità in 5 carte, e carte nostre su cui agiscono (`at`, risolte tramite `getCard`).
       - Handtrap che non fermano la linea (`stops === 0`): raggruppate con nota *"Si usano ma non fermano la linea (es. pescate o effetti che non alterano il risultato finale)"*.

3. **Ordinamento e indicatore rapido nella lista board (`Combo.tsx`)**:
   - Quando è attivo un matchup, mostrare nella scheda/striscia di ciascuna board un badge chiaro con la resilienza contro l'avversario scelto.
   - Possibilità di ordinare le board per resilienza "contro X".

4. **Correzione refuso stile in `BoardDetail.tsx`**:
   - Correggere `py-0.2` in `py-0.5` sul badge "campo al passo" (come segnalato da Claude su PR #15).

5. **Backend finto (`mock.ts`)**:
   - Aggiornare `comboScore(vsDeckId?: number | null)` in `mockApi` per accettare `vsDeckId`.
   - Restituire un `ComboMatchup` di esempio completo quando `vsDeckId` è specificato (es. Branded vs Invoked con Forbidden Droplet, Bystial Saronir, Ash Blossom, ecc., replicando fedelmente i dati del test in `matchup.rs`).

---

## 2. File interessati
- [`apps/desktop/src/lib/combo.ts`](../../../apps/desktop/src/lib/combo.ts): supporto per `vsDeckId` in `scoreBoards` e salvataggio nello stato di `ComboRun`.
- [`apps/desktop/src/pages/Combo.tsx`](../../../apps/desktop/src/pages/Combo.tsx): selettore deck rivale, ordinamento e trigger rimisurazione.
- [`apps/desktop/src/components/combo/Score.tsx`](../../../apps/desktop/src/components/combo/Score.tsx): visualizzazione breaker e handtrap rivali.
- [`apps/desktop/src/components/combo/BoardDetail.tsx`](../../../apps/desktop/src/components/combo/BoardDetail.tsx): fix refuso Tailwind `py-0.2` -> `py-0.5`.
- [`apps/desktop/src/lib/mock.ts`](../../../apps/desktop/src/lib/mock.ts): mockup realistico del matchup.
- [`docs/TASKS.md`](../../TASKS.md): aggiornamento bacheca lavori e lock.

---

## 3. Piano di verifica
1. `cd apps/desktop; pnpm typecheck` per verificare i tipi TypeScript.
2. `cd apps/desktop; pnpm build` per verificare la corretta compilazione Vite.
3. Test manuale in ambiente mock (`VITE_MOCK=1`) e verifica rendering del selettore e dei dettagli di punteggio / handtrap.
4. `cargo fmt --all -- --check` per verificare che la formattazione globale sia integra.
