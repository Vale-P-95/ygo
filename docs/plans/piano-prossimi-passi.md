# Piano dei prossimi passi (22/10/2026)

Piano per i temi aperti dopo la notte del 21-22/10, con le priorità e il lavoro in parallelo. Lo stato in breve è in `docs/STATO.md`, i lock in `docs/TASKS.md`.

## Dove siamo
- In `main` ci sono:
  - l'interfaccia del turno di prova contro un deck (livello 1), di Maxx "C" e Mulcharmy e della mappa dei collegamenti (F1, tempo reale, confronto, F2);
  - pesi imparati e frontiera di Pareto;
  - la velocità della ricerca (−37%);
  - le correzioni del kit (#22–#34).
- **Aperta: la PR #35**, con la CI verde. Contiene:
  - il livello 2 del turno di prova contro un deck, solo nella CLI;
  - tre correzioni indipendenti: il seme in `probe-line`, la scadenza dei giri di apprendimento, la bozza dal testo della mappa.
- **Livello 2: il collaudo su 37 deck** dice che la misura non è ancora affidabile:
  - il 38% delle mosse tipiche non si gioca nemmeno contro un campo vuoto;
  - le mosse cambiano molto con il seme (43% in comune).
- **Classificazione delle board (#47):** 18 coppie sicure giuste su 29. Il difetto principale è la parità; mancano la protezione della board e le risorse per il turno dopo.
- **In pausa o fermi:** Gemini è in pausa. La sessione sulla velocità ha finito. Quella del collaudo è ferma dal terzo giro, con tutte le sue correzioni applicate.

## Priorità
In ordine. Per ognuna: cosa, perché serve, quanto costa.

### P0. Decisioni e pulizia (oggi, mezz'ora del proprietario)
1. **Unire la PR #35.** Il livello 2 è solo un'aggiunta nella CLI e non cambia la misura generica. Le tre correzioni servono subito anche all'app:
   - i giri di apprendimento non sforano più il tempo di minuti sui deck lenti;
   - la bozza della mappa ha meno collegamenti falsi.
2. **Livello 2:** sì o no alla strada «linee vere» (P1).
3. **Deck lenti** (Elfnote Kewl Tune). Dopo il limite di tempo, `confirm_best` e `zone_variants` aggiungono circa 45 secondi. Si possono:
   - tagliare con la stessa scadenza: più veloce, ma su quei deck alcune board restano non confermate;
   - lasciare così: board sempre confermate, ma la ricerca sfora il tempo scelto.
4. **#47:** saltare D5–D7 (toccano poco le coppie) e passare a D10 (kit) e D13 (risorse), dopo P1.
5. **STATO.md** era fermo a prima delle PR #22–#34: aggiornato con questo piano.

### P1. Livello 2 con le linee vere dell'avversario (Claude, 1-1,5 giorni)
**Perché.** È la domanda della #44: «contro questo deck la mia board regge?». Oggi la risposta non è affidabile, perché il kit non sa rigiocare le mosse:
- effetti in catena;
- due effetti della stessa carta;
- carte scelte;
- costi pagati con carte che non ha in mano.

Le cause sono in `docs/plans/turno-di-prova-matchup.md` («Livello 2: collaudo su 37 deck»). Migliorare il kit arriva al 70% delle mosse. Rigiocare le linee vere toglie le cause alla radice, e rende stabili anche le mosse: non c'è più una sequenza tipica da campionare.

**Passi:**
- **L1. Le linee.** `rival_lines` tiene, per ogni mano, la mano vera di 5 carte e le risposte della linea migliore. L'ordine del Deck viene dal seme.
- **L2. L'avversario che rigioca.** Nel turno 2 l'avversario ha la sua mano vera e il suo Deck nello stesso ordine della ricerca. La pescata del turno 2 è una carta in più in cima.
  - Rigioca le risposte della linea, ritrovandole per identità dove la tua board cambia le domande: `find_again`, esteso alle scelte di carte per carta, come per la tua linea contro le handtrap (R3 e R4 del collaudo).
  - Una mossa che non si ritrova si salta, e la linea va avanti.
- **L3. La misura.** Per ogni linea, le mosse fatte con e senza le tue risposte, ogni linea con peso 1 sul numero di linee. L'uscita resta `MovesMatchup` (mosse, fermate, floodgate, «resta»), più la linea a cui appartiene ogni mossa.
- **L4. Il collaudo** con gli strumenti di stanotte, su 6 board contro 37 deck. Obiettivi:
  - almeno il 90% delle mosse fatte contro un campo vuoto (oggi 62%);
  - 4 secondi al massimo per board;
  - mosse in comune tra due semi almeno al 70% (oggi 43%).
- **L5. Contratto per l'interfaccia:** `ComboMatchup.moves` in `lib.rs` e `api.ts`, calcolato a richiesta per la board aperta: costa 1-4 secondi.
- **L6. Gemini:** interfaccia, accanto alla misura contro il deck.

**Rischi:**
- La linea vera era stata trovata contro un campo vuoto: contro la tua board le domande cambiano prima, e il ritrovare per identità deve reggere. Il collaudo L4 lo misura.
- Il tempo per board cresce di 10 volte rispetto a oggi: per questo resta a richiesta.

### P2. Classificazione delle board (#47) e pesi (#43) (Claude, dopo P1: 2-3 giorni)
**Perché.** La classifica delle board è il cuore del Combo Lab: oggi la board giudicata migliore è quella giusta in 18 casi su 29 tra quelli sicuri.

**Passi** (dal piano `docs/plans/classificazione-board.md`):
- **D10:** azioni nuove del kit (mano senza Deck, Cimitero che resta, Xyz o Synchro, Magia Rapida) e pesi del kit calibrati. Le coperture cambiano tutte: si rifanno i numeri di partenza.
- **D13:** costo nuovo e risorse; turno 3 dopo il breaker peggiore; finestre all'inizio e alla fine del turno.
- **Dati del proprietario.** Senza una classifica fatta da te su almeno 15 deck, i pesi (D17) non si toccano.
  - Il modo più economico: usare il confronto affiancato della #26 nell'app vera. Ogni scelta registrata è una coppia giudicata.
  - Così le prove nell'app della #26 servono due volte.
- **D17:** pesi dei profili calibrati sulle coppie, con il 30% tenuto fuori come verifica. Poi i pesi vanno in TASKS per Gemini (`scoring.ts`).

**Perché dopo P1:** D10 e D13 toccano `probe.rs` come P1. Fatti in parallelo, due branch si scontrerebbero sullo stesso ciclo `run_with`.

### P3. Robustezza dei deck lenti (sessione sorella, mezza giornata)
**Perché.** L'app deve finire entro il tempo scelto dall'utente. Su Kewl Tune una ricerca da 60 secondi ne dura 116 anche dopo la correzione di stanotte, e non sappiamo quali altri deck si comportano così.

**Passi:**
- un giro su 37 deck × 6 mani a 1000 nodi, con il tempo oltre il limite misurato per deck e per fase (ricerca, apprendimento, `confirm_best`, `zone_variants`);
- se il proprietario approva, la scadenza anche per `confirm_best` e `zone_variants`, con il banco prima e dopo;
- in misura, senza committarlo: un limite di istruzioni Lua per ogni richiesta del motore, nello shim (non in `vendor/`). Dirà quanto costa in board perse.

**File:** `search.rs` (il lock passa alla sessione sorella), `scripts/`.

### P4. Interfaccia (Gemini)
- **Il Banditore v1** (pagina «Novità»), senza backend nuovo. Ogni carta ha già il suo stato nei tre formati (`Card.banTcg`, `banOcg` e `banMd` in `api.ts`):
  - confronto tra Master Duel, TCG e OCG;
  - impatto sui deck salvati in Bottega: quali carte sono fuori legge, in quale formato e quante copie vanno tolte.
- **Il Banditore v2:** «cosa è cambiato con l'ultima lista». Serve un backend piccolo, che salvi gli stati di prima a ogni aggiornamento (`ygo-data`, poi `lib.rs` e `api.ts`). Lo fa Claude quando `lib.rs` è libero.
- **TypeScript 7:** quando `typescript-eslint` lo regge.
- **Livello 2 nell'interfaccia:** dopo L5.

### P5. Dopo
- **Mappa dei collegamenti:** probabilità sature (Vaylantz ha 24 carte al 99%). Si potrebbe mostrare la differenza tra 5 e 6 carte, o i passi che servono. Da decidere dopo le tue prove della #31 e della #32 nell'app.
- **#45** (linee del video Invoked): si chiude con la #47.
- **Limiti delle carte piazzate** (PIANO §30.4).
- **Simulatore passo per passo** (#36).

## Lavoro in parallelo
**Le risorse.** Questo container ha 4 core e 15 GB.
- Un giro di collaudo occupa i 4 core per circa un'ora.
- Mentre gira, i tempi del banco non sono affidabili: le prove di stanotte giravano in 4 insieme, con tempi gonfiati di 3-4 volte.
- Ogni sessione sorella ha un container suo, con core suoi: è lavoro e calcolo in più, non diviso.

**Proposta: quattro corsie, non di più.**

| Corsia | Chi | Branch | File (lock) | Cosa | Quando |
|---|---|---|---|---|---|
| 1 | Claude, questa sessione (coordina e rivede) | `claude/yugioh-deck-builder-simulator-cax0ya` | `probe.rs`, `matchup.rs`; per L5 `lib.rs` e `api.ts` | P1, poi P2 | dopo le decisioni P0 |
| 2 | sessione sorella «deck lenti» | `claude/slow-decks` | `search.rs`, `scripts/` | P3 | subito: `search.rs` non serve alla corsia 1 |
| 3 | sessione sorella «collaudo» | `claude/tools-qa` (esiste) | `scripts/`, `docs/plans/collaudo-strumenti.md` | collaudo di P1 (L4) e di P3 sui suoi core; poi le coppie per D10 e D13 | quando L3 è pronto |
| 4 | Gemini | `gemini/banditore` | `apps/desktop/src/` | P4 (Banditore v1, poi livello 2) | subito |
| — | proprietario | — | — | decisioni P0; prove nell'app (#26, #30-#32); scelte nel confronto della #26 | quando vuoi |

**Perché non di più:**
- **File caldi.** P1 e P2 toccano entrambi `probe.rs`, e `lib.rs`/`api.ts` li usa una corsia alla volta. Una quinta corsia sul motore si scontrerebbe con la 1.
- **Quota.** Ogni sessione consuma quota, e ogni PR va rivista: circa mezz'ora di revisione mia.
- **Minuti di GitHub Actions.**
  - Ogni PR costa una CI Rust di circa 11 minuti.
  - Le prove lunghe restano nei container: farle sulle macchine di GitHub brucerebbe i minuti contati del repository privato.
  - L'installer Windows solo su `main`.

**Benefici attesi:**
- P1, P3 e P4 insieme durano circa 2 giorni invece di 4.
- I giri di collaudo girano sui core delle altre sessioni, quindi qui banco e tempi restano puliti.
- Ogni corsia ha file suoi, quindi nessun conflitto di merge oltre a `TASKS.md`. Ognuno ci scrive solo la sua riga.

**Rischi e rimedi:**
- Una sessione sorella può fermarsi a metà (quota, container riavviato, come stanotte). Rimedio: commit piccoli e pushati a ogni passo, e il TASKS aggiornato.
- Due PR che toccano gli stessi documenti. Rimedio: ognuna scrive in un file di piano suo; `STATO.md` lo aggiorna solo Claude.

## Cosa serve dal proprietario
1. Le quattro decisioni di P0.
2. Il via alle corsie 2 e 3 (due sessioni sorelle) e a Gemini sul Banditore v1.
3. Quando puoi: le prove nell'app della #26 (con qualche confronto di board registrato), della #30, della #31 e della #32.
