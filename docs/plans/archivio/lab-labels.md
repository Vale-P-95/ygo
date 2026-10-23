# Piano: Allineamento testi del Laboratorio alle statistiche (`gemini/lab-labels`)

Questo intervento allinea le etichette, le descrizioni e i controlli del Laboratorio alle correzioni matematiche e algoritmiche verificate in PIANO §30.7.

## File interessati
1. `apps/desktop/src/components/lab/OverviewTab.tsx`
2. `apps/desktop/src/components/lab/OptimizerTab.tsx`
3. `apps/desktop/src/components/lab/CalculatorTab.tsx`
4. `apps/desktop/src/components/lab/SimulatorTab.tsx`
5. `apps/desktop/src/lib/matches.ts`
6. `apps/desktop/src/components/lab/LogTab.tsx`
7. `docs/TASKS.md`

## Modifiche dettagliate

### 1. `OverviewTab.tsx` (Definizioni delle mani)
- «Mano ideale»: `Almeno uno Starter e una Handtrap: due carte diverse.`
- «Resiste a un'interruzione»: `Due Starter, oppure uno Starter e un Estensore: due carte diverse.`
- «Mano solida»: `Resiste a un'interruzione e ha anche una Handtrap: tre carte diverse.`
- Nota esplicativa per i preset in `conditions.ts`: chiarire che nei preset delle condizioni una carta con doppio ruolo può contare per entrambi se non vincolata diversamente.

### 2. `OptimizerTab.tsx` (Dimensione del deck e vincoli)
- Quando `result.size !== deck.main.length`: aggiungere il testo esplicativo «Il deck passa da N a M carte: il confronto include le carte aggiunte.»
- Gestire e visualizzare l'errore di dimensione (es. 40 carte non raggiungibili) chiaramente all'utente.

### 3. `CalculatorTab.tsx` (Gestione differenziata errori per turno)
- Separare la gestione dell'errore tra mano per primo (`first`) e mano per secondo (`second`).
- Mostrare il messaggio di errore/avviso solo per la mano attualmente visualizzata, e mostrare «—» per l'altra anziché nascondere la probabilità calcolata se una delle due mani è valida.

### 4. `SimulatorTab.tsx` (Tetto al seme)
- Nel campo di input del seed, limitare il valore massimo accettato a `Number.MAX_SAFE_INTEGER` (`Math.min(Number.MAX_SAFE_INTEGER, val)`) per prevenire errori di simulazione a runtime.

### 5. `lib/matches.ts` (Formattazione p-value)
- Nella funzione `pValue(p: number)`: sotto 0,05 non mostrare mai `0,050` (che potrebbe far pensare a un valore non significativo per via dell'arrotondamento per eccesso); mostrare `p < 0,05` o arrotondare per difetto.

### 6. `LogTab.tsx` (Grafico per tag e dicitura game)
- Nel grafico «Per tag», mostrare la linea di riferimento `reference` solo se `decided(r.total) > 0`, evitando la riga «media 0%» quando non ci sono partite decise.
- Aggiornare l'etichetta del game iniziale in Bo3: «Game 1 (Bo3)».

## Verifiche
- `pnpm --dir apps/desktop typecheck` (0 errori)
- `pnpm --dir apps/desktop build` (compilazione con successo)
- `cargo fmt --all -- --check`
