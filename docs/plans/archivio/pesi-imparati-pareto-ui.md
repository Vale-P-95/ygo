# Piano: Pesi imparati dalle tue scelte e Frontiera di Pareto (#40)

> **Obiettivo**: Permettere al giocatore di personalizzare la misura delle board nel Circolo Arcano sia esprimendo preferenze dirette tra board ("Preferisco questa"), da cui il sistema calibra automaticamente i pesi con regressione logistica penalizzata (modello Bradley-Terry), sia filtrando le soluzioni tramite la Frontiera di Pareto ("Solo board non dominate").

---

## 1. Architettura e componenti coinvolti

I tipi e i comandi Tauri sono già pronti nel backend (`crates/ygo-stats/src/prefs.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/lib/api.ts`):
- `api.recordBoardChoice(deckId, chosen, other)`: registra la preferenza per la board con metriche `chosen` rispetto a `other`.
- `api.learnedWeights(prior)`: restituisce `LearnedWeights` (`weights`, `choices`, `agreement`, `priorAgreement`).
- `api.clearBoardChoices()`: cancella le scelte registrate nel database utente.

L'implementazione tocca solo l'interfaccia (`apps/desktop/src/`):
1. `apps/desktop/src/components/combo/BoardCompareModal.tsx`:
   - Pulsanti «Preferisco questa» nelle due colonne del confronto;
   - Indicatore del totale scelte registrate e feedback visivo immediato della scelta effettuata.
2. `apps/desktop/src/lib/scoring.ts`:
   - Supporto per il profilo speciale dinamico `learned` ("Imparato dalle tue scelte");
   - Helper per estrarre il dizionario delle metriche da un `ComboScore` (`interruptions`, `handTraps`, `floodgates`, `resilience`, `followup`, `resources`, `cost`, `exposure`);
   - Calcolo della Frontiera di Pareto: funzione pura `isDominated(a, b)` e `paretoFrontier(scores)`.
3. `apps/desktop/src/components/combo/Score.tsx`:
   - Chip del profilo «Imparato dalle tue scelte» nella barra dei profili;
   - Sezione informativa quando il profilo è attivo: numero di scelte registrate, `agreement` vs `priorAgreement`, avviso se le scelte sono poche (< 4);
   - Pulsante «Azzera scelte» con conferma.
4. `apps/desktop/src/pages/Combo.tsx`:
   - Toggle / chip «Solo board non dominate (Pareto)» accanto ai filtri della classifica board;
   - Badge "Pareto" o "Non dominata" sulle board della frontiera.
5. `apps/desktop/src/lib/mock.ts`:
   - Mocking coerente di `recordBoardChoice`, `learnedWeights` e `clearBoardChoices` in ambiente browser.

---

## 2. Dettaglio delle parti

### A. Pulsante «Preferisco questa» in `BoardCompareModal.tsx`
- Sotto i totali e la testata di ciascuna board (Board #A e Board #B):
  - Pulsante primario `★ Preferisco questa board`;
  - Disabilitato o nascosto se una delle due board non ha un `ComboScore` calcolato (`scoreA` o `scoreB` assenti o con errore);
  - Al clic:
    - Estrae le metriche `Record<string, number>` di entrambe le board tramite `metricValues(score)`;
    - Invoca `api.recordBoardChoice(deckId, chosenMetrics, otherMetrics)`;
    - Riproduce il suono di feedback (`sound.play("favorite")` o `"toggle"`);
    - Mostra un feedback visivo immediato (es. badge "Scelta registrata! Totale scelte: N") e aggiorna il contatore interno del modal.

### B. Profilo «Imparato» e statistiche in `Score.tsx` & `scoring.ts`
- In `scoring.ts`:
  - Gestione della memoria/stato dei pesi imparati (`LearnedWeights | null`).
  - Funzione `metricValues(s: ComboScore): Record<MetricKey, number>`:
    ```ts
    {
      interruptions: s.interruptions,
      handTraps: s.handTraps,
      floodgates: s.floodgates,
      resilience: s.resilience,
      followup: s.followup,
      resources: s.resources,
      cost: s.cost,
      exposure: s.exposure,
    }
    ```
- In `Score.tsx` (`WeightsPanel`):
  - Chip profilo: `[Bilanciato] [Più negazioni] ... [★ Imparato]`;
  - Selezionando `Imparato`:
    - Chiama `api.learnedWeights(choice.weights)`;
    - Se `choices === 0`: messaggio "Nessun confronto registrato. Apri «Confronta» tra due board e premi «Preferisco questa» per insegnare le tue preferenze.";
    - Se `choices > 0`:
      - "X scelte registrate";
      - "Accordo: {Math.round(agreement * 100)}% (il profilo di partenza ne spiegava il {Math.round(priorAgreement * 100)}%)";
      - Se `choices < 4`: nota "Con poche scelte i pesi restano vicini al profilo di partenza per stabilità";
      - Pulsante per azzerare `api.clearBoardChoices()`.

### C. Filtro «Frontiera di Pareto» in `Combo.tsx`
- **Definizione formale**:
  Una board $A$ domina $B$ ($A \succ B$) se:
  - $\forall m \in \text{positive}$: $A[m] \ge B[m]$
  - $\forall m \in \{\text{cost}, \text{exposure}\}$: $A[m] \le B[m]$
  - $\exists m$ tale che $A[m]$ è strettamente migliore di $B[m]$ (maggiore se positiva, minore se negativa).
- Se $A \succ B$, la board $B$ è *dominata* (esiste un'altra linea che ottiene almeno lo stesso risultato su tutto spendendo meno o uguale).
- Toggle in `Combo.tsx`:
  - `[ ] Solo non dominate (Pareto)` con contatore `X / Y`;
  - Quando attivo, `boards` viene filtrato per escludere quelle dominate;
  - Se una board ha una stima provvisoria non ancora misurata con `ComboScore`, non viene esclusa a priori finché non è misurata, o viene indicata come non classificata.

---

## 3. Piano di test e verifica
1. `pnpm typecheck` in `apps/desktop` (0 errori di tipo).
2. `pnpm build` in `apps/desktop` (compilazione bundle Vite riuscita).
3. `cargo fmt --all -- --check`.
4. Verifica in mock mode (`VITE_MOCK=1`):
   - Aprire il confronto affiancato di due board, cliccare «Preferisco questa»;
   - Verificare che il contatore aumenti e il feedback compaia;
   - Selezionare il profilo «Imparato» e verificare accordo % e pesi aggiornati;
   - Azionare il filtro «Solo board non dominate (Pareto)» e verificare che le board inferiori vengano filtrate.
