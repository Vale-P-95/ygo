# Piano: Test automatici dell'interfaccia (Frontend Unit Tests)

## Obiettivo
Aggiungere una suite di test unitari automatizzati per le funzioni logiche pure di `apps/desktop/src/lib/` e `apps/desktop/src/pages/Banditore.tsx`, integrata nello script npm `"test": "vitest run"` e nella CI GitHub Actions sotto il job `Frontend (TypeScript)`.

## Ambito e Moduli da testare
Solo funzioni pure: nessun mock di Tauri, nessuna chiamata a backend reale o finto, nessun rendering DOM completo.

1. **`apps/desktop/src/lib/probability.test.ts`** (test di `probability.ts`):
   - `pNone(size, hits, n)`: probabilità ipergeometrica di pescare 0 copie.
   - `pAtLeastOne(size, hits, n)`: probabilità di pescare almeno 1 copia.
     - Caso calcolato a mano: deck di 40 carte, 3 copie, mano da 5.
       $$P(\ge 1) = 1 - \frac{\binom{37}{5}}{\binom{40}{5}} = 1 - \frac{435897}{658008} \approx 0.33756...$$
     - Caso 1 copia su 40, mano da 5: $1 - \frac{39}{40}\frac{38}{39}\frac{37}{38}\frac{36}{37}\frac{35}{36} = 1 - \frac{35}{40} = 0.125$.
     - Casi limite: 0 copie ($P=0$), mano grande quanto l'intero deck ($n = size$), $n > size$, $n = 0$.
   - `pBoth(size, a, b, either, n)`: probabilità congiunta con principio di inclusione-esclusione.

2. **`apps/desktop/src/lib/query.test.ts`** (test di `query.ts`):
   - `tokenize`: tokenizzazione con spazi, stringhe con virgolette (`"Ash Blossom"`), token multipli consecutivi.
   - `splitToken`: estrazione chiave, operatore (`=`, `:`, `>=`, `<=`, `>`, `<`), valore non quotato e gestione chiavi con caratteri accentati (es. `rarità`).
   - `getKey` e `setKey`: lettura e impostazione di filtri singoli con alias italiani (`formato:md`, `ban:limitata`, `tipo:mostro`). Rimozione filtro quando valore è `null`.
   - `toggleToken`: aggiunta e rimozione idempotente / toggle di token.
   - `getRange` e `setRange`: parsing e composizione di range numerici (es. `atk:1500-2000`, `lv>=4`, `def<=1000`).

3. **`apps/desktop/src/lib/conditions.test.ts`** (test di `conditions.ts`):
   - `toCond` e `fromCond`: reversibilità e invarianza strutturale fra albero dell'interfaccia (`CondNode`: `Clause`, `Group`) e tipo del contratto `Cond`.
   - `usedRoles`: estrazione ricorsiva dei ruoli referenziati senza duplicati (es. `starter`, `extender`, `brick`, `handtrap`).
   - `describe`: generazione delle descrizioni testuali in italiano per clausole singole, operatori e gruppi annidati con congiunzioni `e` / `o`.

4. **`apps/desktop/src/lib/deck.test.ts`** (test di `deck.ts`):
   - `copyLimit`: limiti per formato Master Duel, TCG e OCG (Forbidden/Banned -> 0, Limited -> 1, Semi-Limited -> 2, Unlimited / non specificato -> 3).
   - `addCard`:
     - Inserimento automatico in Main vs Extra Deck (Fusion, Synchro, Xyz, Link).
     - Rifiuto carte vietate/limitate oltre quota per formato.
     - Rifiuto carte speciali (`token`, `skill`).
     - Rispetto limiti di capienza delle zone (`main: 60`, `extra: 15`, `side: 15`, `maybe: 30`).
     - Inserimento in `maybe` board senza limite di copie della singola carta ma con limite di capienza zona.
   - `removeAt`: rimozione per indice da zona specificata.
   - `counts`: conteggio accurato mostri, magie, trappole.

5. **`apps/desktop/src/lib/scoring.test.ts`** (test di `scoring.ts`):
   - `score`: calcolo ponderato del punteggio complessivo rispetto a pesi personalizzati e profili predefiniti.
   - Segno delle metriche negative: `NEGATIVE` include `["cost", "exposure"]` che con peso negativo riducono il punteggio all'aumentare del valore.
   - *Nota vincolante*: nessun test sui valori specifici o interpretazione di `exposure`, in quanto sotto modifica attiva (D12) nella corsia 4.

6. **`apps/desktop/src/pages/Banditore.test.ts`** (test di `Banditore.tsx`):
   - `normalizeBanStatus`: normalizzazione di status grezzi in inglese e italiano ("Forbidden", "Banned", "Proibita", "Limited", "Limitata", "Semi-Limited", "Semi", "Unlimited", null, undefined).
   - `getChangeType`: classificazione variazioni banlist (`restriction`, `loosening`, `neutral`) tra stato precedente e successivo (es. Unlimited -> Forbidden è `restriction`, Forbidden -> Limited è `loosening`, Forbidden -> Unlimited è sbandita/loosening).

## Dipendenze e Strumenti
- Installazione di `vitest` in `apps/desktop/package.json` come `devDependency` (compatibile con Vite 8.3.4).
- Script `"test": "vitest run"` aggiunto a `apps/desktop/package.json`.
- Aggiornamento di `apps/desktop/pnpm-lock.yaml` via `pnpm install` fuori sandbox (`BypassSandbox: true` su Windows).
- Aggiunta dello step `- run: pnpm test` in `.github/workflows/ci.yml` dopo `pnpm typecheck` nel job `Frontend (TypeScript)`.

## Procedura e Regole
1. Lock in `docs/TASKS.md` e push del piano su `gemini/frontend-tests`.
2. Installazione dipendenze e configurazione package/CI.
3. Scrittura ed esecuzione dei test unitari (`pnpm test`).
4. Se emerge un bug logico reale nel codice testato, correggerlo in un commit separato e documentato; non indebolire i test.
5. Controlli completi locali: `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `cargo fmt --all -- --check`.
6. Apertura PR verso `main` con riepilogo conteggio test per modulo e verifiche.
