# Bacheca dei lavori

Regole in `AGENTS.md`. Per prendere un task, spostalo in "In corso" con agente, branch e file che tocchi (è il lock), poi committa e pusha questa modifica prima di cominciare.

## In corso
Corsie di lavoro in parallelo del 22/10/2026 (piano in `docs/plans/piano-prossimi-passi.md`): ogni corsia tocca solo i suoi file.
- [x] **Scheda del punteggio** (Gemini, PR #54, unita il 22/10): testi di follow-up, risorse e costo in `scoring.ts`, «Con una risposta alla tua prima interruzione…» (`ComboScore.countered`) e il pannello «Protezione dei pezzi» (`api.boardProtection`, `PieceProtection.tsx`). Provata con il backend finto, `typecheck`, `lint` e `build`; da provare nell'app vera. Lock tolto.
- [x] **Il Banditore v2** (Gemini, PR #55, unita il 22/10): ricerca con lista OCG (`formato:ocg`) nel `Promise.all` iniziale e in `mock.ts`, vista «Cosa è cambiato con l'ultima lista» (`api.banChanges`), raggruppamento per data/formato, apertura Visore Carta, messaggio esplicito per storico iniziale vuoto. Lock tolto.
- [x] **Test automatici dell'interfaccia** (Gemini, PR #57, unita il 22/10): unit test con Vitest per le funzioni pure in `lib/` (`probability`, `query`, `conditions`, `deck`, `scoring`) e `Banditore.tsx`, script "test" e step nella CI. 75 test passati. Lock tolto.
- [x] **Revisione dell'interfaccia, passo 0: fondamenta** (Gemini, PR #63, unita il 22/10): scala tipografica e spaziature come token in `src/index.css`, `lucide-react`, componenti base (`Toolbar`, `FilterChip`, `IconButton`, `SidePanel`, `HelpPopover`), script Playwright e test Vitest toolbar. Lock tolto.
- [x] **Revisione dell'interfaccia, passo 1: Collegamenti** (Gemini, PR #66, unita il 22/10): cerchio a tutta larghezza, geometria e raggio dinamici in `layout.ts` con nodi ritagliati e frame Yu-Gi-Oh, barra Toolbar monoriga collassabile, pannello laterale arricchito e probabilità stimata. Lock tolto.
- [x] **Esposizione come valore perso (D12)** (Gemini, PR #74, unita il 22/10): «perdita attesa per le handtrap: X interruzioni» nella scheda del punteggio (`Score.tsx`) e nel confronto (`BoardCompareModal.tsx`), con elenco perdite per tipo (`exposureDetail.losses`), Maxx "C" / Mulcharmy (`maxx`, `maxxChance`) e punti critici (`chokePoints`). Formattazione condivisa, contrasto > 4.5:1 e test unitari. Lock tolto.
- [ ] **Pesi dell'esposizione nella scala nuova (D17)**: aggiornamento pesi in `PROFILES` di `scoring.ts` (Bilanciato −0,5, Più negazioni −0,4, Controllo −0,4, A prova di breaker −0,6, Risorse −0,3, Linee sicure −1,0), migrazione con versione dei dati salvati (version: 2, conversione una tantum |exposure| < 0,2 → ×12,5), testo di aiuto e test Vitest. Agente: gemini. Branch: `gemini/exposure-weights`. File: `apps/desktop/src/lib/scoring.ts`, `apps/desktop/src/lib/scoring.test.ts`, `docs/plans/taratura-pesi.md`, `docs/TASKS.md`.
- [x] **Corsia 1.** Turno di prova contro un deck, livello 2, con le linee vere dell'avversario (piano P1, L1–L6):
  - l'avversario rigioca le linee migliori che il motore trova per il suo deck, con la sua mano vera e l'ordine del suo Deck;
  - le risposte si ritrovano per identità dove la tua board cambia le domande.

  - [x] L1–L3: `rival_lines`/`rival_line` (mano e risposte della linea migliore), `Kit::with_line` e il `Follower` (`follow.rs`) nel ciclo del turno di prova, `against_moves` linea per linea; banco identico.
  - [x] L5: contratto per l'interfaccia: `api.comboMoves(hand, key, vsDeckId)` → `ComboMoves` (`lib.rs` `combo_moves`, `api.ts`, `mock.ts`). Le linee del deck avversario si leggono una volta per deck (10-30 s) e restano in memoria; poi pochi secondi per board.
  - [x] L4: collaudo su 37 deck (corsia 3, PR #36): mosse fatte su campo vuoto 96,3% dopo la correzione qui sotto. L6: interfaccia (Gemini), unita con la PR #40.
    - [x] Correzione da L4: le carte scelte dalla linea dell'avversario si ritrovano per identità anche quando lui gioca come giocatore 1. Prima si ripiegava sull'indice, che cadeva sulla carta pescata nel turno 2. Mosse fatte su campo vuoto dall'89% al 96% (Tearlaments e Branded contro 12 deck).
    - [x] Correzione L4-2 (PR #48): la tua linea si rigioca contro un Deck avversario grande come quello del kit, quello contro cui la linea è stata trovata. That Grass Looks Greener mandava un altro numero di carte, e la board Tearlaments non si misurava contro nessun deck.
    - [x] Correzione L4-7: i materiali di un'Evocazione con somma di Livelli (Rituali) si ritrovano per identità. Contro Megalith le mosse fatte passano dall'87% al 98%; sugli altri 40 deck nessuna peggiora.
    - [ ] L4-3 (Lunalight lenta, 85-500 s): alla corsia 2, insieme al tetto di istruzioni Lua. L4-4 (`--seed` con `--moves`): annotato. L4-6 (rigioco dell'avversario senza la sua Fase Finale): alla corsia 4 dopo D12.

  Tutto in `main` (PR #38, #40, #46, #48 e la PR di L4-7). Lock dei file tolto.
- [ ] **Corsia 2.** Deck lenti (piano P3):
  - [x] un giro su 37 deck × 6 mani a 1000 nodi, con il tempo oltre il limite per fase (fatto su 41 deck; più un giro a 20 s senza limite di nodi; numeri in `docs/plans/velocita-ricerca.md`);
  - [x] poi la scadenza anche per `zone_variants` (si perdono solo varianti di zona) e per `confirm_best` (le board non confermate restano stimate, `exact: false`), con il banco prima e dopo.

  Dopo: il backend del Banditore v2, cioè gli stati delle carte prima dell'ultima lista, salvati a ogni aggiornamento (`ygo-data`, poi `lib.rs` e `api.ts`, presi solo se la corsia 1 non li tiene). Fatto, in una PR separata dalla P3: tabella `ban_changes` in `cards.db`, riportata da un aggiornamento delle carte all'altro; comando `ban_changes`, `api.banChanges(format?)` e stub in `mock.ts`.

  - [x] Misura delle istruzioni Lua per richiesta del motore (P3, «in misura»): PR #49, numeri in `docs/plans/velocita-ricerca.md`. Nessuna richiesta normale arriva a 3 milioni di istruzioni; la mano lenta di Kewl Tune ripaga a ogni replay due richieste da 19 e 104 milioni.
  - [x] Tetto per richiesta, chiuso nel motore: 500 000 ingressi in Lua (`DUELFORGE_LUA_LIMIT`; contare le istruzioni costava il 21% del banco). La richiesta fermata è un errore, la linea risulta interrotta e il duello non si riusa. Banco identico e senza costo, `combo_hunt --seed 7` identico; mano lenta 117 → 1003 nodi; Lunalight livello 2 (L4-3) da 297-550 s a 4-19 s, perdendo 2 linee su 11. Numeri in `docs/plans/velocita-ricerca.md`.

  - [x] `combos --probe`: 3 varianti misurate solo per le prime 3 board stampate, 1 per le altre. Il #1 stampato resta il migliore misurato (0 deck su 41 prima e dopo); dracotail-0 da 69,3 a 64,2 s, kewl_tune-0 da 54,3 a 47,4 s. Numeri in `docs/plans/velocita-ricerca.md`.
  - [x] Le fasi dopo la ricerca: nessuna scadenza più corta tiene le board migliori della mano lenta di Kewl Tune (prove in `docs/plans/velocita-ricerca.md`), quindi resta quella di prima. Cambio rimasto: un giro di apprendimento non comincia una board che non finirebbe in tempo.

  Agente: claude-code, sessione sorella della velocità. Branch: `claude/search-speed`. File (lock del 22/10): in `crates/ygo-cli/src/main.rs` solo la scelta delle varianti di `combos` (la stampa dell'esposizione è della corsia 4), `crates/ygo-solver/src/search.rs`, `docs/plans/velocita-ricerca.md`.
- [ ] **Corsia 3.** Collaudo:
  - [x] prima, le coppie di board da far giudicare al proprietario (circa 15 deck, le coppie dove la classifica è più incerta), per la #47 e i pesi (#43): 46 coppie su 16 deck in `tests/fixtures/boards/owner-pairs.tsv` e `.json` (22/10);
  - [x] il collaudo del livello 2 con le linee vere (L4) su `48efae0`: 88% delle mosse fatte su campo vuoto, 71% di mosse in comune tra semi, problemi L4-1…L4-5 in `docs/plans/collaudo-strumenti.md` (22/10);
  - [x] il collaudo dei deck lenti (corsia 2, 22/10): con `combo_hunt --seed 7` su `main` e424904 resta sopra 60 s solo dracotail-0 (70,7 s da solo), per i turni di prova delle varianti di `--probe` (21c8885); il tetto Lua lì non scatta. Numeri in `docs/plans/velocita-ricerca.md`.
  - [x] il banco oggettivo della forza delle board (per D17 e #43): board di tutti i deck MDM contro i 15-20 avversari più diffusi, livello 1 e 2, misure generiche, divisione taratura/verifica:
    - [x] script e selezione delle board (205 board di 41 deck, 20 avversari da popRank);
    - [x] il giro completo (dopo la #53): 817 invocazioni, nessun timeout, 200 board con la verità;
    - [x] il documento con metodo, pesi del meta, tempi, coppie dubbie e primi numeri;
    - [x] le 90 board delle coppie del proprietario nel banco (77 trovate, 76 con la verità) e la tabella per coppia.

  Agente: claude-code, sessione sorella del collaudo. Branch: `claude/tools-qa`. File: script nuovi in `scripts/` (`objective_bench.py`), `docs/plans/collaudo-strumenti.md`, `docs/plans/banco-oggettivo.md`, `tests/fixtures/boards/`.
- [ ] **Corsia 4, quinto task.** B5a, attrito avversario (`docs/plans/ricerca-piano-critico.md`, `ricerca-letteratura.md` §1, «Il caso Aeon»). Controllo: su Invoked la linea di Aeon con il bando dall'Extra Deck avversario sta sopra quella col «No», nella misura e nella stima; banco, `combo_hunt` e `search_regression` non peggiorano oltre il 2% (test appaiato per mano).

  Agente: claude-code, sessione della corsia 4. Branch: `claude/placed-cards`. File: `crates/ygo-solver/src/probe.rs` (kit, `Measure`, Observer, `run_with`), `follow.rs`; in `search.rs` solo `quick_score` e il passaggio dell'avversario di partenza; `crates/ygo-engine/src/italian.rs` (un'etichetta); la parte di `crates/ygo-cli/src/main.rs` che stampa la misura.
  - [x] Il sì opzionale di Aeon: nessuna regola lo rifiuta; la ricerca lo trova, ma la stima non vede l'avversario e a parità vince la linea più corta.
  - [x] Extra Deck dell'avversario del kit da 15 carte; risorse tolte all'avversario (`Measure::taken`, `attrition`); scelte tra le carte avversarie rigiocate per identità; Observer (controllo, Zona Magie e Trappole). Pushati.
  - [x] Il termine in `quick_score` (test `tests/attrition.rs`).
  - [x] L'etichetta italiana «Look at your dell'avversario, Extra Deck».
  - [x] Banco, `combo_hunt` e `search_regression` prima e dopo, appaiati per mano (numeri in `docs/plans/attrito-avversario.md`).
- [ ] **Corsia 8.** B14, valutazione delle board a gradini (`docs/plans/ricerca-piano-critico.md`, `ricerca-esperimenti.md` §1 sul branch `claude/research-lab`): stima → prime 40 → misura economica (2 finestre per risposta, niente ciclo degli usi, breaker e `countered`) → prime 15 → misura completa. Controllo: nessuna mano con un #1 peggiore oltre l'1%, tempo della valutazione almeno −25%.

  Agente: claude-code, sessione della corsia 8. Branch: `claude/staged-measure`. File: in `crates/ygo-solver/src/probe.rs` solo l'aggiunta del modo economico (`Parts`, `Prober::measure_cheap`), senza le parti della corsia 4; `apps/desktop/src-tauri/src/lib.rs` (`combo_score`); `apps/desktop/src/lib/api.ts` (solo il tipo `ScoreView`, se serve); la parte `combos --probe` di `crates/ygo-cli/src/main.rs`; la sezione «B14» di `docs/plans/velocita-ricerca.md`.
  - [x] Modo economico in `probe.rs` (`Prober::measure_cheap`); misura completa identica.
  - [x] Scala nell'app (`combo_score`): 40 economiche → 15 complete; `ScoreView.cheap` (`cheap?: boolean` in `api.ts`). Compilata con clippy; non provata nell'app vera.
  - [x] `combos --staged` (e `combo_hunt.py --staged`).
  - [x] Protocollo: `combo_hunt --seed 7` su 41 deck, tempo dei turni di prova 914 → 653 s (71%), #1 uguale o migliore in 41 mani su 41, nessuna peggiore oltre l'1%, migliore in 3; banco a 4000 nodi identico (12 mani su 16: 4 deck mancano nelle liste MDM del 22/10); `search_regression` verde. Numeri in `docs/plans/velocita-ricerca.md`, «B14». In attesa della PR (la apre la coordinatrice).
  - Richiesta per Gemini (scheda del punteggio): con la valutazione a gradini l'app manda anche board con `cheap: true`, solo con la misura economica. Non hanno breaker (la `resilience` è tutto il loro campo), follow-up, esposizione né confronto con il deck scelto. Vanno messe dopo quelle con la misura completa, con un'etichetta come «valutata in breve», senza confrontarne il punteggio pesato con quello delle altre. Un evento `combo-score` con la misura completa sostituisce quello economico della stessa board.
- [x] **Corsia 4, PR piccola per la corsia 7**, unita il 22/10 (PR #71). In `combos --json`: tempo della misura per parte (run base e risposte, insieme, breaker, follow-up), un'opzione per misurare senza breaker, e, se costa poco, `quick_value`. File: la parte `combos` di `crates/ygo-cli/src/main.rs`, i tempi in `probe.rs` (`measure_from`).
- [x] **Corsia 4, quarto task**, unita il 22/10 (PR #69). L4-6: nel livello 2 il rigioco della linea dell'avversario si ferma all'ultima scelta libera della sua Main Phase e non entra nella Fase Finale, così le sue mosse di Fase Finale contano come perse (Titaniklad, Incredible Ecclesia, Blazing Cartesia, Despian Luluwalilith). Si rigioca anche la Fase Finale.

  Agente: claude-code, sessione della corsia 4. Branch: `claude/placed-cards` (ripartito da `main` 6018809). File: in `crates/ygo-solver/src/probe.rs` solo il ramo `Some(1) if follower.is_some()` di `run_with` e come chiama `turn_three` (d'accordo con la coordinatrice, che fa B5 nel resto); `crates/ygo-solver/src/follow.rs` (`Follower::answer`); `crates/ygo-solver/tests/matchup.rs`.
  - [x] La correzione (nessun test nuovo: le ricerche brevi dei test non trovano linee con mosse di Fase Finale; la prova è il collaudo).
  - [x] `moves_report.py` (invoked e branded contro 12 deck, semi 1 e 7): mosse fatte su una board vuota dal 93,8% al 95,0% (non fatte da 220 a 178 per seme), 0 errori. Banco (18,26 / 18,26) e `combo_hunt` (41 ok, 0 problemi) identici a `main` 6018809; test verdi con `search_regression`.
- [x] **Corsia 4, terzo task**, unita il 22/10 (PR #65). Esposizione come valore perso (#47, D12 = B6 V1 di `docs/plans/classificazione-board.md`): nel punteggio l'esposizione diventa `E = Σ_h p_h · loss_h` in interruzioni, al posto del numero di punti critici, che resta nel dettaglio.

  Agente: claude-code, sessione della corsia 4. Branch: `claude/placed-cards` (ripartito da `main` dbd7c58). File: `crates/ygo-solver/src/probe.rs`, `explain.rs`; `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/lib/api.ts`, `mock.ts`; in `crates/ygo-cli/src/main.rs` la parte di `combos` e `probe-line` che stampa l'esposizione; script miei in `scripts/`; `docs/plans/esposizione.md`.
  - [x] `ChokePoint` con la posizione nella linea.
  - [x] `Prelude::Cut` e `Prober::quick_value`.
  - [x] `loss_h`, il termine di Maxx "C" e Mulcharmy, `p_h` dalle liste e una tabella di default per l'app.
  - [x] `E` nel punteggio (`score_view`), tutte le board misurate, cache; contratto `lib.rs` ↔ `api.ts`; testo per Gemini.
  - [x] Banco, caccia, tempo per board (numeri in `docs/plans/esposizione.md`); le coppie non si riproducono con le liste del 22/10: da rifare con la cartella delle coppie del coordinatore.
- [x] **Corsia 7.** Esperimenti di ricerca, senza codice del motore o della ricerca (`docs/plans/ricerca-piano-critico.md`). Resoconto in `docs/plans/ricerca-esperimenti.md`; la PR la apre la coordinatrice.

  Agente: claude-code, sessione sorella di ricerca. Branch: `claude/research-lab`. File: script nuovi in `scripts/research/`, `docs/plans/ricerca-esperimenti.md`, aggiunte a `docs/plans/ricerca-letteratura.md`, `tests/fixtures/research/`. Lock tolto.
  - [x] 1. Gradini di misura per B14 (82 mani × 60 board).
    - Il 69% della misura è il ciclo delle risposte, quindi la misura «senza breaker» costa il 73%.
    - L'app di oggi perde il #1 vero in 10 mani su 82.
    - Un gradino con 2 finestre per risposta perde fino al 33,7% (correzione della prima versione, su 41 mani). Con 8 finestre e un giro degli usi, la scala «30 → 15» tiene il #1 in 82 mani su 82; il tempo è circa il 71% solo se il gradino è un prefisso della misura completa (corsia 8).
    - La variante sicura senza codice nuovo («30 → senza breaker → 15») costa l'88%.
    - Il livello 2 resta a richiesta.
  - [x] 2. Taratura della stima (Texel). Le caratteristiche della board non aiutano fuori campione. Due run economiche del turno di prova portano le coppie giuste dal 72,2% al 79,6%: stima a due livelli per B9. Elenco dei dati che servirebbero dal motore.
  - [x] 3. Controllo con ygo-combo-solver, e diagnosi.
    - Il solver (compilato su Linux con il nostro core) raggiunge Oceanus + S:P in 4 mani su 4; la nostra ricerca in 0 su 4.
    - Causa: nel modo obiettivo si scelgono per prime le carte dell'obiettivo anche quando la scelta le bandisce o le usa come materiale. Con una patch locale si arriva a 3 su 4. Correzione alla corsia 2 (B4a).
    - Fixture: `tests/fixtures/research/invoked-goals.tsv`.
  - [x] 4. Bibliografia: `ricerca-letteratura.md` §8, con la confidenza di ogni fonte, le correzioni e le aree mancanti.
  - Seguiti, ad altre corsie: il livello 2 dà «0 lines» su Railway e rifiuta una risposta su Elfnote Kewl Tune (corsia 4). Quando il gradino di B14 è in `main`, la misura va rifatta senza la patch.
- [ ] **Corsia 6, secondo task.** Ruoli (piano `docs/plans/ricerca-piano-critico.md`, «Ruoli»): board breaker che usano carte avversarie come materiale o Tributo, starter ed estensore insieme, Invocation «Grail» estensore. Poi l'oracolo A1 e il test appaiato.

  Agente: claude-code, sessione della corsia 6. Branch: `claude/roles-fix` (da `main` cc78f89). File: `crates/ygo-solver/src/roles.rs` e i suoi test, la parte `roles` di `crates/ygo-cli/src/main.rs` se serve, script nuovi miei in `scripts/`; `docs/plans/ricerca-piano-critico.md` (sezioni Ruoli e A1); per A1 anche `tests/fixtures/oracle/`.
  - [x] Ruoli (in attesa della PR, la apre la coordinatrice): numeri nel piano, «Ruoli: fatto».
  - [ ] A1: oracolo e test appaiato, branch `claude/oracle`. Fatti `scripts/oracle.py`, `scripts/paired_test.py` e un pilota sul banco (fixture in `tests/fixtures/oracle`, numeri nel piano). Restano il giro lungo (del proprietario) e la board di Aeon a 3 materiali.
- [x] **Corsia 6.** Probabilità della mappa dei collegamenti: una `chance` che distingua le carte e sia vicina al vero (misurata su mani cercate), senza saturazione. Fatto, in attesa della PR (la apre la coordinatrice): errore medio contro 200 mani a caso da 0,344 a 0,059 su 6 deck; carte a ≥ 0,99 sui 41 deck da 29 + 20 a 6 + 2; nessun collegamento perso; ricerca delle combo invariata. Numeri e cosa resta aperto in `docs/plans/mappa-collegamenti.md`, «Probabilità misurata sulle mani cercate».

  Agente: claude-code, sessione sorella della mappa. Branch: `claude/links-chance`. File: `crates/ygo-solver/src/links.rs` e i suoi test, la parte `links` di `crates/ygo-cli/src/main.rs`, `Rng` di `crates/ygo-solver/src/card.rs` (solo la visibilità), script nuovi miei in `scripts/` (`links_chance.py`) e il default di `links_report.py`, `docs/plans/mappa-collegamenti.md`, `tests/fixtures/links/`; solo i commenti di `LinkCard` in `apps/desktop/src-tauri/src/lib.rs` e `apps/desktop/src/lib/api.ts` (nessun campo nuovo). Lock tolto.
  - Seguito per Gemini: `chance` ora dice «una mano porta in gioco la carta» ed è una stima per quasi tutte le carte (`measured` vero dopo la prima mano cercata): si può mostrare come «circa 45%». Durante l'aggiornamento progressivo le probabilità si muovono un po'.
- [x] **Corsia 4, secondo task**, unita il 22/10 (PR #50). Ordine della ricerca e stima rapida (#47, D14 = B11 di `docs/plans/classificazione-board.md`): il #1 stampato è il migliore misurato; inversioni tra stima e misura nella top 8 (C5.7) ≤ 60; campi distinti nella top 8 (C5.8). Si accetta solo se `best` del banco non scende più del 5%.

  Agente: claude-code, sessione della corsia 4. Branch: `claude/placed-cards` (ripartito da `main`). File: `crates/ygo-solver/src/search.rs`, `value.rs`, `crates/ygo-cli/src/main.rs` (solo la parte `combos`), script miei in `scripts/`, `docs/plans/ordine-ricerca.md`.
  - [x] Linea di base (C5.7, C5.8, banco).
  - [x] 1. Riordino misurato nella CLI dopo `confirm_best`; verifica della top N nell'app.
  - [x] 2. Bonus fissi di `quick_score` solo per le carte non misurate.
  - [x] 3. Mano vuota in `value.rs`.
  - [x] 4. Seconda copia.
  - [x] 5. Spot mancanti.
  - [x] 6. Look key con Cimitero usabile: saltato (tocca `src-tauri/src/lib.rs`; nella CLI lo copre il 7).
  - [x] 7. Varianti nella top N.
  - [x] C5.7: inversioni da 223 a 174 su 1110, obiettivo ≤ 60 non raggiunto; #1 stampato sempre il migliore (19 deck → 0). Numeri in `docs/plans/ordine-ricerca.md`.
- [x] **Corsia 4.** Carte piazzate e ruoli (PIANO §30.4), unita il 22/10 (PR #42):
  - gli equipaggiamenti piazzati con il mostro equipaggiato;
  - i mostri dell'Extra Deck piazzati con i materiali (Apollousa non più a 0 ATK);
  - la prova dell'estensore che non riparte da un turno nuovo (Evocazione Normale ed effetti "una volta per turno" già usati restano usati).

  Con il protocollo del motore (banco prima e dopo). Agente: claude-code, sessione sorella nuova. Branch: `claude/placed-cards`. File: la parte delle carte piazzate di `crates/ygo-engine/src/duel.rs`; `crates/ygo-engine/src/ffi.rs`, `crates/ygo-engine/shim/` e `crates/ygo-engine/build.rs` solo se servono; `crates/ygo-solver/src/roles.rs`, `value.rs`; in `search.rs` solo `SearchConfig.prelude`, `new_duel` e l'uscita iniziale di `learn_round` (d'accordo con il coordinatore); in `matchup.rs` e `tests/probe.rs` solo i letterali di `Placement`; i loro test (`crates/ygo-engine/tests/`, `crates/ygo-solver/tests/` per i file nuovi); `docs/plans/carte-piazzate.md`.
  - [x] Equipaggiamenti piazzati con il mostro equipaggiato.
  - [x] Mostri dell'Extra Deck piazzati con i materiali.
  - [x] Prova dell'estensore nello stesso turno dello starter.
  - [x] Banco, ruoli e valori prima e dopo (`docs/plans/carte-piazzate.md`); PR aperta, in attesa di revisione.
- [x] **Corsia 5.** Unita il 22/10 (PR #37). Pagina «Il Banditore» (Novità) v1: confronto tra formati (Master Duel, TCG, OCG) con ricerca e filtri, apertura Visore Carta, data banlistMd, e impatto sui deck salvati nella Bottega per formato scelto con apertura nella Fucina. Agente: gemini. Branch: `gemini/banditore`. File: `apps/desktop/src/pages/Banditore.tsx`, `apps/desktop/src/App.tsx`, `apps/desktop/src/sections.ts`, `apps/desktop/src/lib/mock.ts`.
  - [x] Seguito per Gemini (PR #55): la ricerca vera ha ora anche la lista OCG (`formato:ocg ban:forbidden ban:limited ban:semi`). Le carte limitate solo in OCG entrano nell'elenco iniziale con una terza ricerca nel `Promise.all` di `Banditore.tsx`; nel backend finto (`searchCatalog` in `mock.ts`) `ban:` con `formato:ocg` deve leggere `banOcg`.
  - [x] Seguito per Gemini, la v2 (PR #55; «cosa è cambiato con l'ultima lista»): backend pronto nella PR della corsia 2, `api.banChanges(format?)` → `BanChange[]` (formato, id, data, stato prima e dopo; `null` è senza limiti), le più recenti prima. Per «l'ultima lista» di un formato si prendono le voci con la data più recente; per TCG e OCG la data è il giorno dell'aggiornamento dei dati di YGOPRODeck, non quello della lista. La storia comincia dal primo aggiornamento dopo questa versione: prima è vuota.
- [x] **Corsia 5b (P1 L6).** Unita il 22/10 (PR #40). Turno di prova contro un deck, livello 2: interfaccia: pulsante «Contro le sue linee», messaggio di attesa 10-30s, riepilogo `ComboMoves` (left, floodgateShare, value, warning madeShare < 0.8), linee dell'avversario con stati delle mosse («fatta», «fermata dalla board», «persa», «non rigiocata») ed evidenziazione al passaggio/clic delle tue risposte (`stops`). Agente: gemini. Branch: `gemini/matchup-moves`. File: `apps/desktop/src/components/combo/RivalMoves.tsx`, `apps/desktop/src/components/combo/BoardDetail.tsx`, `docs/plans/turno-di-prova-livello-2-ui.md`.
- [ ] Classificazione delle board più profonda, con criteri validati sui deck di Master Duel Meta (#47): D10, D13, D14 (corsia 4, PR #50) e D15 (PR #48) in `main`. D12 (esposizione come valore perso, PR #65) e D17 (taratura dei pesi, `docs/plans/taratura-pesi.md`: i quattro pesi restano, l'esposizione passa alla scala nuova) fatti. Restano D11 (esca, solo da mostrare) e D16 (handtrap vera e ripresa). Agente: claude-code (coordinatore). Branch: `claude/yugioh-deck-builder-simulator-cax0ya`. File: nessun lock.
  - Piano: `docs/plans/classificazione-board.md` (passi D0–D17; giudizio dato da un pannello di agenti, non dal proprietario).
  - [x] D0a: `combos --json/--save-lines`, `probe-line`.
  - [x] D0b: `combo_hunt.py --seed/--json`, `board_pairs.py`, `rank_report.py`, `tests/fixtures/boards/pairs.tsv`, numeri di partenza (piano §C7).
  - [x] Pannello di giudici (tre agenti): 74 coppie in 39 deck (30 sicure). Sicure giuste 17 su 33, da confermare 7 su 45; il difetto più frequente è la parità (piano §C7).
  - [ ] D1–D7: correzioni della misura (bersagli, fermate, copertura, copie, run congiunta, floodgate finti). D2, D3 e D4 fatti (piano §C8); D5–D7 saltati il 22/10: toccano poco le coppie, si passa al kit (D10) e alle risorse (D13).
    - [x] D2: una fermata conta solo per il passo giocato, anche per i floodgate (test in `crates/ygo-solver/tests/probe.rs`).
    - [x] D3: un mostro conta come Evocato quando atterra; "nega l'Evocazione" solo prima dell'atterraggio (evento `Summoned` nel motore).
    - [x] Seguito delle risposte: le carte portate da una risposta (un mostro Evocato) rispondono anche loro; si Evoca il mostro più forte; si paga con la carta meno preziosa. Coppie sicure giuste da 14 a 18 su 29 (piano §C8).
    - [x] D4: un mostro girato coperto ha gli effetti negati; una carta attaccata come materiale Xyz è come bandita. Il resto di A4 non serve più dopo D2 e D3 (restavano 3 carte con etichette diverse tra le board).
  - [x] D10: il kit pesato sulle linee vere. `kit-mix` divide le attivazioni per provenienza e per "tocca il Deck" (50% nelle linee vere, 78% del peso nel kit). Due azioni nuove senza Deck (Dian Keto, Bystial Magnamhut) e pesi dalle quote. Ash da 0,71 a 0,53; coppie sicure giuste da 18 a 20 su 29; ricerca +3,2% (piano §C9). Non fatte: l'Evocazione Xyz o Synchro e il trigger d'Evocazione senza Deck.
  - [x] D13, prima parte: il tuo turno dopo il board breaker peggiore. Gli effetti che ripartono da Cimitero e bandite entrano nel follow-up (0,3 l'uno); le risorse non contano più il Cimitero. Coppie sicure giuste da 20 a 22 su 29 (piano §C10).
  - [x] D13, costo (B8): non più le carte della mano usate oltre la prima (quelle rimaste sono già contate), ma 0,05 per carta del Deck pagata come costo, 0,1 per mostro dell'Extra Deck finito nel Cimitero o bandito, 1 ogni 4000 LP pagati. Testo dell'interfaccia: PR #54.
  - [x] D13, finestre prima dei passi del kit (B9, `PRE`): le tue risposte nella Draw e nella Standby avversarie si provano; coppie e banco invariati (piano §C10).
  - [x] D14 (corsia 4, PR #50): il #1 stampato è sempre il migliore misurato (prima no in 19 deck su 41); bonus fissi solo per le carte non misurate, valori a mano vuota, seconda copia, spot nuovi, un campo per riga. Inversioni tra stima e misura da 223 a 174 su 1110: l'obiettivo di 60 vuole una stima della board intera (`docs/plans/ordine-ricerca.md`).
  - [x] D15: i breaker che le liste di Master Duel Meta giocano davvero (Infinite Impermanence, Super Polymerization), i breaker che mirano alla carta che vale di più, la risposta dell'avversario alla tua prima interruzione (`countered`, solo da mostrare) e la protezione di ogni pezzo (`api.boardProtection`, a richiesta). Ricerca identica nel banco, coppie da confermare giuste da 14 a 16 (piano §C11). Interfaccia: PR #54.
- [x] Correzioni dalla caccia ai bug della ricerca (12 problemi confermati): fatte; l'ultima (valori delle carte che dipendono dal contesto) passa nella #47. Lock tolto.
  - [x] Set in un solo ordine anche dopo una finestra di catena; ricerca fermata a metà che riparte dal punto giusto.
  - [x] Carte rapide tenute per dopo nel dive; handtrap giudicate da ciò che permettono (Labrynth Cooclock); carte da tenere rispettate durante la ricerca.
  - [x] Zone Magie/Trappole centrali prima delle Zone Pendulum; ruoli e copertura con la stessa lista di handtrap del Combo Lab.
  - [x] Turno di prova: bersagli sulla carta che l'avversario usa; costi pagati con carte che non rispondono.
  - [x] Catena non risolta nella chiave dei nodi; test di regressione della ricerca in CI (PIANO §30.6).
  - [x] Valori delle carte che dipendono dal contesto (mano vuota, equipaggiamenti): passata nella #47.

- [x] Velocità del motore: mimalloc per Lua (−27% di tempo di ricerca, risultati identici) e meno replay inutili (−9%), sul branch di Claude in attesa di PR (PIANO §30.8).
- [x] Chiave dei nodi con i bersagli e i nomi dichiarati: provata e tolta. Il banco migliorava (best 26,32 → 27,23), ma la caccia sui 45 deck con tre semi peggiorava di poco (somma −2,48, 5 deck peggio e 4 meglio). Numeri e idee per riprovarla nel PIANO §30.8.
- [x] Controllo di salute del motore su 45 deck: nessuna linea interrotta (PIANO §30.8).

## Da fare (in ordine di priorità)

### Interfaccia (Gemini)
- [/] **Per Gemini: pesi dell'esposizione nella scala nuova** (D17, `docs/plans/taratura-pesi.md`). In corso su `gemini/exposure-weights`.
  - In `PROFILES` di `scoring.ts`, `exposure`: Bilanciato −0,5, Più negazioni −0,4, Controllo e floodgate −0,4, A prova di board breaker −0,6, Risorse e follow-up −0,3, Linee sicure −1,0. Gli altri pesi non cambiano.
  - Pesi salvati in `localStorage` (`duelforge.combo.weights`, profilo personalizzato o imparato): se `exposure` ha valore assoluto sotto 0,2 è nella scala vecchia; moltiplicarlo per 12,5 una volta sola al caricamento.
  - Un test Vitest per la conversione e uno che la classifica di due board con la stessa forza e esposizione diversa metta prima quella meno esposta.
  - Il testo d'aiuto della metrica (`scoring.ts`, riga ~110) va aggiornato: ora è la perdita attesa, non il numero di punti critici.
- [ ] **Per Gemini: l'esposizione come valore perso** (D12, backend della corsia 4; piano B6 in `docs/plans/classificazione-board.md`, numeri in `docs/plans/esposizione.md`). `ComboScore.exposure` non è più il numero di punti critici. Ora è la perdita attesa per le handtrap, in interruzioni: per ogni tipo di handtrap, quello che toglie nel momento peggiore per la linea, per la probabilità che l'avversario ne abbia una.
  - **Scheda del punteggio** (`Score.tsx`) e **confronto** (`BoardCompareModal.tsx`). Il valore si mostra come "perdita attesa per le handtrap: X interruzioni". Sotto va una riga per tipo da `exposureDetail.losses`: "Ash Blossom: −1,2 se arriva (34%)", "Veiler/Impermanence", "Nibiru", "Droll & Lock Bird". Poi Maxx "C" (`maxx`, `maxxChance`), e il numero di punti critici (`chokePoints`) come prima.
  - **Tipi.** I campi nuovi sono opzionali (`ExposureLoss` in `api.ts`); il backend finto ha già i tre esempi aggiornati.
- [ ] **Revisione dell'interfaccia, seconda parte** (dalle prove del proprietario nell'app vera, 22/10). Piano: `docs/plans/ui-revisione-2.md`. Un branch e una PR per passo, in quest'ordine:
  - [x] Passo 0, fondamenta: scala tipografica e spaziature, set di icone unico, componenti di base (`Toolbar`, `FilterChip`, `IconButton`, `SidePanel`, `HelpPopover`), script di screenshot.
  - [x] Passo 1, Collegamenti: cerchio a tutta larghezza, nodi con l'immagine della carta, barra dei filtri su una riga, pannello utile senza selezione, probabilità come stima (PR #66 unita il 22/10).
  - [ ] Passo 2, Fucina: ispettore della carta a sinistra (passaggio del mouse, un clic per fissarla, doppio clic come oggi).
  - [ ] Passo 3, ricerca e filtri del catalogo su una riga.
  - [ ] Passo 4, passata su tutte le sezioni: scritte piccole, testi fissi, barre affollate.
  - [ ] Passo 5, Bottega: scatole smussate con materiali veri, mensole di legno.
  - [ ] Passo 6, effetti visivi e suoni nuovi, ognuno con il suo interruttore nelle Impostazioni.
- [x] **Piccola correzione rimasta dalla PR #18** (da fare per prima, anche dentro la PR della mappa): in `apps/desktop/src/pages/Combo.tsx` il chip «vs X: …» (righe ~995-1000) e l'ordinamento «vs deck» (riga ~278) non guardano `s.matchup.error`: una board con il calcolo contro il deck fallito mostra «vs X: 0,00» e viene ordinata come se valesse 0. Escluderla in entrambi i punti (`s.matchup && !s.matchup.error`), e nel chip mostrare «vs X: errore» come in `Score.tsx`.
- [x] Turno di prova contro un deck scelto (#44, livello 1; backend pronto, piano in `docs/plans/turno-di-prova-matchup.md`). Nel Circolo Arcano:
  - un selettore "Contro: generico / deck della Bottega" vicino al pulsante che misura le board;
  - con un deck scelto si chiama `api.comboScore(vsDeckId)`: ogni `ComboScore` ha allora `matchup` (`ComboMatchup` in `api.ts`); cambiare deck rimisura tutte le board;
  - nella scheda del punteggio, accanto alla resilienza generica, la "resilienza contro X" (`matchup.resilience`, con `breakerChance` e la lista `breakers`: carta, probabilità di averla, valore che lascia, chi la nega). Con `standardBreakers` il deck non ha breaker giocabili: va detto che si usano quelli standard;
  - accanto all'esposizione, "fermata da X" (`handTrapChance`) e la lista `handTraps`: carta, probabilità, momenti in cui ferma la linea su quelli in cui si può usare (`stops`/`windows`), le tue carte in quei momenti (`at`). Le handtrap con `stops` a 0 (Maxx "C", Mulcharmy) si possono mostrare a parte: "si usano ma non fermano la linea";
  - una colonna o un ordinamento "contro X" nella classifica delle board, e il backend finto (`mock.ts`) con un `matchup` di esempio.
- [x] Mappa dei collegamenti tra le carte, interfaccia base (PR #22, piano `docs/plans/mappa-collegamenti.md`): scheda «Collegamenti» nel Laboratorio, cerchio carte, frecce colorate per tipo (`CardLink.kind`, incluso `"set"`), spessore per `used`, tratteggio per possibili; filtri per tipo con conteggi e per zona; pannello dettagli con `reach`, `reachAll`, `chance` (e `LinkCard.measured` per stime) e conteggi.
- [x] **Turno di prova contro un deck, livello 2: interfaccia** (unita con la PR #40; dopo Il Banditore v1; backend pronto, piano P1 in `docs/plans/piano-prossimi-passi.md`). Nel Circolo Arcano, nella scheda del punteggio di una board misurata «contro X» (livello 1), un pulsante «Contro le sue linee» che chiama `api.comboMoves(hand, key, vsDeckId)` a richiesta, per la board aperta.
  - **Attese:** la prima volta per un deck servono 10-30 secondi (il backend legge le sue linee migliori da 12 mani), poi pochi secondi per board. Il messaggio di attesa lo deve dire.
  - **Riepilogo** (`ComboMoves`):
    - «con tutte le tue risposte resta il X% delle sue mosse» (`left`);
    - «la tua board da sola ferma il X%» (`floodgateShare`);
    - il valore (`value`);
    - quante linee sono state misurate (`measured` su `lines.length`).
    - Se `madeShare` è sotto 0,8, un avviso: «il turno di prova ha rigiocato solo il X% delle sue mosse».
  - **Le sue linee:**
    - per ogni linea, la mano (`hand`) e le mosse in ordine, con il loro stato da `moves` (stesso indice di linea): fatta, fermata dalla board, persa con una precedente, non rigiocata;
    - una linea senza mosse si mostra come «solo carte Settate».
  - **Le tue risposte** (`stops`):
    - carta, posto, effetto, quota (`coverage`) e valore;
    - le mosse che ferma (`moves`, indici in `moves`) evidenziate nelle linee al passaggio del mouse o al clic.
  - **Regole:** protocollo dell'interfaccia di `AGENTS.md` (id di richiesta, azzeramento al cambio di board o di deck, errori in italiano). Nel backend finto c'è già un esempio (`mock.ts`, `comboMoves`).
- [x] **Testi di follow-up, risorse e costo** (dopo la PR di D13, #47; fatto nella PR #54). Il backend ora conta nel follow-up anche il tuo turno dopo: dopo il board breaker peggiore, ogni effetto che riparte dal Cimitero o dalle bandite vale 0,3. Le risorse non contano più il Cimitero. In `apps/desktop/src/lib/scoring.ts`:
  - `followup`: `hint` ed `explain` dicono anche «effetti da Cimitero e bandite nel tuo turno dopo, dopo il board breaker peggiore (0,3 l'uno)»;
  - `resources`: togliere «e un po' il Cimitero»;
  - `cost`: non più «carte della mano usate oltre la prima», ma «quello che la linea spende oltre alla mano: carte del Deck pagate come costo (0,05 l'una), mostri dell'Extra Deck finiti nel Cimitero o banditi (0,1 l'uno), LP pagati (1 ogni 4000)». Le carte della mano rimaste contano già nelle altre voci;
  - il dettaglio resta in `followupNote` e `resourcesNote`, che arrivano già dal backend. Il contratto (`ComboScore`) non cambia.
- [x] **Risposta alla prima interruzione e protezione dei pezzi** (fatto nella PR #54; dopo la PR di D15, #47; backend pronto, piano `docs/plans/classificazione-board.md` §C11). Nella scheda del punteggio di una board misurata (Circolo Arcano):
  - `ComboScore.countered` (da 0 a 1, o `null`): «con una risposta alla tua prima interruzione (Infinite Impermanence) fermi ancora il X% di quello che fermi». Con `null` non si mostra niente: l'avversario non ha potuto rispondere, o la board non ferma niente. Solo da mostrare, non entra nel punteggio;
  - un pulsante «Protezione dei pezzi» che chiama `api.boardProtection(hand, key)` → `ComboPiece[]` a richiesta, per la board aperta (qualche secondo: si rimisura la board). Per ogni carta che interrompe: «regge a: …» (`holds`) e «cade a: …» (`falls`), con le parole che arrivano dal backend ("distruggere", "bandire", "girare coperto", "negare"). La carta risponde solo con i suoi effetti; contano le protezioni durature della board (Stardust Sifr);
  - id di richiesta e azzeramento quando cambia la board, errori in italiano (protocollo dell'interfaccia in `AGENTS.md`). Il backend finto (`mock.ts`) ha già un esempio di entrambi.
- [ ] Tornare a TypeScript 7 (`apps/desktop/package.json`) quando `typescript-eslint` lo supporta: con la PR #29 è fissato a 6.0.3 perché `typescript-eslint` 8 non regge ancora la versione 7. Poi, già che ci si è, `tseslint.config(...)` → `defineConfig` di `eslint/config` in `eslint.config.js`.

### Motore e ricerca (Claude)
- [ ] **Ricerca delle combo: piano critico** (dalle prove del proprietario, 22/10): board più forti in meno tempo. Piano: `docs/plans/ricerca-piano-critico.md`; letteratura e audit delle carte: `docs/plans/ricerca-letteratura.md`.
  - [x] Ruoli: board breaker che usano carte avversarie come materiale (Super Polymerization), starter ed estensore insieme, Invocation «Grail» estensore. Corsia 6, branch `claude/roles-fix`, in attesa della PR.
  - [ ] A1: oracolo automatico (la stessa ricerca con un budget enorme, le board misurate con il banco oggettivo) e test appaiato per accettare le modifiche. Corsia 6.
    - [x] Strumenti (`scripts/oracle.py`, `scripts/paired_test.py`) e pilota a 40 000 nodi (PR #79).
    - [ ] Il giro vero nel cloud, a 50 volte i nodi, prima con il solo seme della mano. Corsia 6, branch `claude/oracle-run`.
  - [ ] A1b: oltre i limiti dell'oracolo. Livello 2 sulle board dell'oracolo e rapporto dei disaccordi (con la corsia 3); convergenza a 100× e 200× su 12 mani; generalità su deck mai visti (niente ricalcoli quando cambiano le liste); più avanti ricerca esaustiva sulle mani piccole e ygo-combo-solver. Corsia 6, dopo il giro vero.
  - [x] A2: dove va il lavoro della ricerca (PR #67): replay 43% del tempo della ricerca, nodi già visti 26%, nodi senza cambi visibili 24%; turno di prova 58% del tempo di `combo_hunt`.
  - [ ] B0: il replay (prima un prototipo dell'arena di memoria, poi la copia del duello). Corsia 2.
    - [x] B13, PR 1 (corsia 2, PR #72): arena di memoria per duello e `Duel::save`/`restore` col lato Rust. File: `crates/ygo-engine/shim/arena.cpp` (al posto di `new.cpp`), `build.rs`, `src/duel.rs`, `src/ffi.rs`, `src/lib.rs`, `tests/arena.rs`, `.github/workflows/ci.yml` (job Windows del motore).
    - [ ] B13, PR 2 (corsia 2, `claude/search-speed`): le discese della ricerca partono dalla copia più lunga della loro linea (64 MB per thread, `copies_mb`); controllo con `DUELFORGE_COPY_CHECK=n`. Ricerca −32% sul banco, board identiche. File: `crates/ygo-solver/src/search.rs`, `coverage.rs`, `tests/search_regression.rs`, `crates/ygo-engine/shim/arena.cpp`, `shim/compile.cpp`, `src/duel.rs`.
    - [ ] B13, dopo: turno di prova (con la corsia 4), giri di apprendimento e fasi dopo la ricerca (`confirm_best`, `zone_variants`).
  - [ ] B1a, B2, B2b, B3, B1b-c, poi B8 (motore e contratto), B10, B11, B16, B4. Corsia 2, una PR l'una.
  - [x] B4a (corsia 2): nel modo obiettivo una carta dell'obiettivo scelta dove il motore la manda via (costo, materiale, scarto) va per ultima, e perde priorità se era l'ultima copia. Obiettivo Invoked «Oceanus + S:P» da 0/4 a 4/4 mani in 60 s, con Oceanus nella Zona Mostri Extra anche. File: `crates/ygo-solver/src/search.rs`, `tests/search_regression.rs`.
  - [ ] B5a (attrito avversario, caso Aeon), B14 (quali board misurare), B5b-d, B9. Coordinatore, dopo D17.
  - [ ] B8, interfaccia degli obiettivi ricchi. Gemini, dopo il contratto.
- [ ] Linee del video Invoked: come si classificano, correzioni generali (#45).
- [ ] Scale e pesi delle metriche del punteggio (#43).
- [ ] Frontiera di Pareto e confronto tra board; preferenze apprese (#40).
- [ ] Simulatore: giocare passo per passo una combo scelta (#36, alla fine).
- [x] Limiti noti delle carte piazzate: equipaggiamenti, materiali dei mostri dell'Extra Deck (PIANO §30.4, PR #42).
- [ ] **Più avanti, dopo la #47 e la #43:** valutatore imparato. Una rete piccola stima il turno di prova durante la ricerca, al posto di `quick_score`, come NNUE in Stockfish; il motore e le regole restano gli stessi (`docs/plans/valutatore-imparato.md`).

### Richieste dal backend per l'interfaccia

## Completati (recenti; lo storico è in `docs/PIANO.md`)
- [x] Test automatici dell'interfaccia con Vitest nella CI: 75 unit test per funzioni pure (`probability`, `query`, `conditions`, `deck`, `scoring`, `Banditore.tsx`), script `"test": "vitest run"` e step CI nel job Frontend (TypeScript) (PR #57, piano `docs/plans/test-interfaccia.md`).
- [x] Pagina «Il Banditore» (Novità) v1: confronto tra formati (Master Duel, TCG, OCG) con ricerca e filtri, apertura Visore Carta, data banlistMd, e impatto sui deck salvati nella Bottega per formato scelto con apertura nella Fucina (PR #37, piano `docs/plans/banditore-v1.md`).
- [x] Giri di apprendimento della ricerca con una scadenza (10 secondi dopo il limite di tempo): su Elfnote Kewl Tune una ricerca da 60 secondi finiva dopo 170, ora dopo 116; banco identico. `probe-line` rigioca le linee salvate con il loro seme (`docs/plans/velocita-ricerca.md`).
- [x] Velocità della ricerca (sessione sorella): partite riusate, script caricati una volta per thread, solo le corse del turno di prova che servono, mimalloc anche per il C++, limite di nodi senza race. Ricerca −37%, CLI −41% sulle 16 mani con uscite identiche; `combo_hunt.py` −50% (PR #33, resoconto `docs/plans/velocita-ricerca.md`).
- [x] Deck dell'avversario del kit a 35 carte e breaker con il Deck vero dell'avversario (ultime correzioni del collaudo, A4 e M1; numeri in `docs/plans/turno-di-prova-matchup.md`).
- [x] Mappa dei collegamenti F2, interfaccia: percorso più corto evidenziato nel cerchio, con le carte intermedie cliccabili e il pulsante verso il Combo Lab (starter e obiettivo); nodi più grandi per le carte chiave, senza sovrapposizioni nei deck grandi; mano di prova anche in confronto (PR #32, piano `docs/plans/mappa-collegamenti-f2.md`). Da provare nell'app vera dal proprietario.
- [x] Mappa dei collegamenti quasi in tempo reale e confronto tra versioni del deck, interfaccia: bozza dal testo subito e aggiornamento progressivo con la barra sopra la mappa, collegamenti «dal testo» tenui, confronto con la versione precedente (guadagnati, persi, carte tolte come nodi fantasma, carte che cambiano) (PR #31, piano `docs/plans/mappa-collegamenti-tempo-reale-confronto.md`). Da provare nell'app vera dal proprietario.
- [x] Maxx "C" e Mulcharmy: carte fatte pescare a ogni momento della linea e punto migliore dove fermarsi (backend, `api.comboStops`), e la loro interfaccia nel Circolo Arcano: «fa pescare fino a N carte», momenti, tabella «Dove fermarsi contro Maxx "C"» con il salto al passo (PR #25 e #30, piani `docs/plans/turno-di-prova-matchup.md` e `docs/plans/maxx-c-mulcharmy-ui.md`). Da provare nell'app vera dal proprietario.
- [x] Controlli automatici dell'interfaccia nella CI: ESLint con `any` vietato ed `exhaustive-deps`, `pnpm lint` nel job «Frontend (TypeScript)» (PR #29, piano `docs/plans/eslint-interfaccia.md`).
- [x] Mappa dei collegamenti tra le carte, backend (piano `docs/plans/mappa-collegamenti.md`): collegamenti letti dal motore, carte chiave, correzioni del collaudo, mappa quasi in tempo reale (bozza dal testo, aggiornamento progressivo, ricalcolo delle sole carte cambiate) e confronto tra versioni del deck (`api.deckLinksCompare`).
- [x] Pesi imparati dalle tue scelte e frontiera di Pareto (#40) nel Circolo Arcano: confronto affiancato di due board con preferenza registrata, profilo «Imparato dalle tue scelte» e pesi basati su Bradley-Terry con prior preservato e reset, frontiera di Pareto con filtro handFilter e resilient matchup (PR #26, piano `docs/plans/pesi-imparati-pareto-ui.md`).
- [x] Mappa dei collegamenti tra le carte, interfaccia: nuova scheda «Collegamenti» nel Laboratorio, cerchio interattivo SVG, vista a colonne, filtri per azione e zona, ispettore carte con carte chiave e probabilità mano iniziale, ponte rapido al Combo Lab e mock Invoked arricchito (PR #22, piano `docs/plans/mappa-collegamenti-ui.md`).
- [x] Turno di prova contro un deck scelto (#44, livello 1) nel Circolo Arcano: selettore rivale ("Contro: generico / deck Bottega"), resilienza contro X e breaker con probabilità in 6 carte, handtrap rivali che fermano la linea con probabilità in 5 carte e momenti/carte coinvolti, ordinamento "vs deck", mock aggiornato e fix py-0.2 -> py-0.5 (piano `docs/plans/turno-di-prova-matchup-ui.md`).
- [x] Turno di prova contro un deck scelto, livello 1, nel backend: resilienza contro i breaker del deck avversario e handtrap che fermano la linea, con le probabilità di averli in mano; `probe-line --vs`, `comboScore(vsDeckId)` (#44, PIANO §30.10).
- [x] Collegamento del tappetino del Circolo Arcano al riproduttore passo per passo con `ComboStep.field`: campo, mano e pile aggiornati a ogni passo (PR #15).
- [x] Visore Carta (`CardViewer`) interattivo: aggiunta/rimozione copie (+ Main/Extra, + Side, + Maybe, −) con limiti dal formato del deck (MD, TCG, OCG), assegnazione rapida dei ruoli in `deck.roles` e avvio esplorazione combo (PR #13, #14).
- [x] Stato del campo e della mano a ogni passo della linea in `ComboStep.field` (backend; tipo `ComboField` in `api.ts`).
- [x] Testi del Laboratorio allineati ai calcoli dopo la verifica delle statistiche (PIANO §30.7, PR #11).
- [x] Ponte rapido tra Laboratorio, Bottega e Circolo Arcano: avvio combo 1-carta dai ruoli e dalla panoramica del Laboratorio, apertura diretta dei Deck Box in Laboratorio o Circolo Arcano dalla Bottega con riassunto ruoli (PR #10).
- [x] Riproduttore passo-passo della linea (controlli, evidenziazione del passo e delle carte coinvolte), frecce Link sul campo e «Copia guida» (solo nomi e passi) nel Circolo Arcano (PR #8). Il tappetino passo per passo si collega quando arriva lo stato per passo in `ComboStep`.
- [x] Filtri rapidi per ruolo, riconoscimento ruoli, distribuzione della curva e Mano di prova collegata al Circolo Arcano nella Fucina (PR #7).
- [x] Confronto affiancato tra due board nel Circolo Arcano (differenze su campo, interruzioni, resilienza e punteggio, PR #6).
- [x] Navigazione da tastiera e palette dei comandi rapida (`Ctrl+1..7` per le sezioni, `Ctrl+K` per la palette, `?` per la guida e le scorciatoie, PR #5).
- [x] Guida per chi usa l'app per la prima volta (prima apertura: scaricare i dati, creare un deck, cercare le combo), in italiano (`WelcomeModal`, checklist in `Overview`, pulsante `Guida rapida`, PR #4).
- [x] Revisione dell'interfaccia (parte 2): Laboratorio statistico, grafici SVG, Bottega e Fucina su schermi piccoli (`1280×800`, PR #3).
- [x] Revisione dell'interfaccia (parte 1): tema chiaro "Pergamena" e Circolo Arcano su schermi piccoli (`1280×800`, PR #2).
- [x] Backend finto per sviluppare l'interfaccia nel browser (`VITE_MOCK=1` in `src/lib/api.ts` e `src/lib/mock.ts`, PR #1).
- [x] Audit della copertura delle meccaniche su 45 deck: nessuna linea interrotta da errori del motore (PIANO §30.2–30.3).
- [x] Revisione della v1.8: 21 difetti confermati e corretti (PIANO §30.1).
- [x] Strumento `duelforge-cli coverage`; linee Pendulum non più scartate; negazioni delle proprie catene imparate; Set in un solo ordine (PIANO §30.2–30.3).
- [x] Punteggio leggibile con i tasti "i"; handtrap morte come carte da scartare; metrica di esposizione (PIANO §29).
- [x] Regole e file per lavorare con due agenti (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, questa bacheca, `docs/STATO.md`).
