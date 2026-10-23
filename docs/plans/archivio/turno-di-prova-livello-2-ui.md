# Piano: Turno di Prova contro un Deck (Livello 2: Interfaccia)

Questo piano definisce l'architettura e l'implementazione dell'interfaccia per il **Livello 2 del turno di prova contro un deck della Bottega** (analisi contro le linee vere dell'avversario), in conformità con `docs/plans/piano-prossimi-passi.md` (P1 L6), `docs/TASKS.md` e `AGENTS.md`.

---

## Obiettivi

1. **Pulsante «Contro le sue linee» a richiesta:**
   - Nella scheda del punteggio di una board nel Circolo Arcano (`Combo.tsx` / `BoardDetail.tsx`), quando la board ha un `score.matchup` valido (Livello 1).
   - Invocazione su richiesta di `api.comboMoves(hand, key, vsDeckId)`.
   - Feedback di attesa chiaro ed esplicito per l'utente: la prima volta richiede 10-30 secondi (il backend legge le 12 mani migliori del deck avversario), mentre per le board successive richiede pochi secondi.

2. **Riepilogo delle metriche (`ComboMoves`):**
   - «Con tutte le tue risposte resta il **X%** delle sue mosse» (`left`).
   - «La tua board da sola ferma il **X%**» (`floodgateShare`).
   - Valore interruzioni (`value`).
   - Linee misurate: `measured` su `lines.length`.
   - Avviso contestuale se `madeShare < 0.8`: «Il turno di prova ha rigiocato solo il X% delle sue mosse».

3. **Le tue risposte (`stops`):**
   - Visualizzazione dettagliata di ogni risposta della board: carta (miniatura, nome, hover), posizione (`place`: campo, Cimitero, ecc.), effetto (`effect`), quota (`coverage`) e valore (`value`).
   - **Interattività:** al passaggio del mouse o al clic su una risposta, vengono evidenziate immediatamente nelle linee le mosse dell'avversario che quella risposta ferma (`stop.moves`).

4. **Le linee dell'avversario (`lines`):**
   - Per ciascuna linea: visualizzazione delle 5 carte della mano iniziale (`line.hand`) con anteprima e hover.
   - Se una linea non ha mosse: segnalazione chiara «Solo carte Settate».
   - Sequenza ordinata di mosse con il loro stato calcolato (`moves[i].state`):
     - `made`: **✓ Fatta** (riuscita senza risposte)
     - `stopped`: **🚫 Fermata dalla board** (bloccata dalla board come floodgate)
     - `lost`: **↷ Persa** (persa a causa di una mossa precedente interrotta)
     - `notMade`: **— Non rigiocata** (non effettuata neanche a campo vuoto)
   - Evidenziazione visiva reattiva se una mossa è bersagliata/fermata dalla risposta correntemente selezionata.

---

## Architettura e Componenti

### 1. `apps/desktop/src/components/combo/RivalMoves.tsx`
Nuovo componente modulare dedicato al Livello 2:
- Props:
  - `board: ComboBoard`: board correntemente aperta
  - `matchup: ComboMatchup`: matchup livello 1 associato al mazzo rivale
  - `getCard: (id: number) => Card | undefined`
  - `onHover?: HoverCard`
  - `onHoverEnd?: () => void`
- Gestione dello stato asincrono protetto da `requestIdRef`:
  - Reset di `data`, `loading`, `error`, `selectedStop` a ogni cambio di `board.key` o `matchup.deckId`.
  - Nessuna chiamata duplicata o accavallata.

### 2. `apps/desktop/src/components/combo/BoardDetail.tsx`
- Inserimento di `<RivalMoves />` nella sezione «Il punteggio», subito sotto a `ScoreBreakdown` e accanto a `StopTradeOff`.
- Mostrato unicamente quando `score.matchup && !score.matchup.error`.

---

## Protocollo Interfaccia (AGENTS.md)
- Nessun tipo manuale o `any`: uso esclusivo dei contratti tipizzati in `api.ts` (`ComboMoves`, `ComboRivalLine`, `ComboMeasuredMove`, `ComboMoveStop`).
- Colori da token CSS (`var(--df-ok)`, `var(--df-danger)`, `var(--df-warn)`, `var(--sec)`, `var(--df-surface-2)`, ecc.).
- Gestione errori in italiano con avvisi espliciti e toast.
- Verifiche pre-push: `pnpm lint`, `pnpm typecheck`, `pnpm build`.
