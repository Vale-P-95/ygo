# Bacheca dei lavori

Regole in `AGENTS.md`. Per prendere un task, spostalo in "In corso" con agente, branch e file che tocchi (è il lock), poi committa e pusha questa modifica prima di cominciare.

## In corso
Corsie di lavoro in parallelo del 22/10/2026 (piano in `docs/plans/piano-prossimi-passi.md`): ogni corsia tocca solo i suoi file.
- [ ] **Corsia 1.** Turno di prova contro un deck, livello 2, con le linee vere dell'avversario (piano P1, L1–L6):
  - l'avversario rigioca le linee migliori che il motore trova per il suo deck, con la sua mano vera e l'ordine del suo Deck;
  - le risposte si ritrovano per identità dove la tua board cambia le domande.

  - [x] L1–L3: `rival_lines`/`rival_line` (mano e risposte della linea migliore), `Kit::with_line` e il `Follower` (`follow.rs`) nel ciclo del turno di prova, `against_moves` linea per linea; banco identico.
  - [x] L5: contratto per l'interfaccia: `api.comboMoves(hand, key, vsDeckId)` → `ComboMoves` (`lib.rs` `combo_moves`, `api.ts`, `mock.ts`). Le linee del deck avversario si leggono una volta per deck (10-30 s) e restano in memoria; poi pochi secondi per board.
  - [ ] L4: collaudo su 37 deck (corsia 3). L6: interfaccia (Gemini): unita con la PR #40.
    - [x] Correzione da L4: le carte scelte dalla linea dell'avversario si ritrovano per identità anche quando lui gioca come giocatore 1. Prima si ripiegava sull'indice, che cadeva sulla carta pescata nel turno 2. Mosse fatte su campo vuoto dall'89% al 96% (Tearlaments e Branded contro 12 deck).
    - [x] Correzione L4-2 (PR #48): la tua linea si rigioca contro un Deck avversario grande come quello del kit, quello contro cui la linea è stata trovata. That Grass Looks Greener mandava un altro numero di carte, e la board Tearlaments non si misurava contro nessun deck.
    - [ ] L4-3 (Lunalight lenta, 85-500 s): alla corsia 2, insieme al tetto di istruzioni Lua. L4-4 (`--seed` con `--moves`): annotato.

  L1–L3 e L5 sono in `main` (PR #38); restano L4 (corsia 3) e L6 (Gemini). Lock dei file tolto.
- [ ] **Corsia 2.** Deck lenti (piano P3):
  - [x] un giro su 37 deck × 6 mani a 1000 nodi, con il tempo oltre il limite per fase (fatto su 41 deck; più un giro a 20 s senza limite di nodi; numeri in `docs/plans/velocita-ricerca.md`);
  - [x] poi la scadenza anche per `zone_variants` (si perdono solo varianti di zona) e per `confirm_best` (le board non confermate restano stimate, `exact: false`), con il banco prima e dopo.

  Dopo: il backend del Banditore v2, cioè gli stati delle carte prima dell'ultima lista, salvati a ogni aggiornamento (`ygo-data`, poi `lib.rs` e `api.ts`, presi solo se la corsia 1 non li tiene). Fatto, in una PR separata dalla P3: tabella `ban_changes` in `cards.db`, riportata da un aggiornamento delle carte all'altro; comando `ban_changes`, `api.banChanges(format?)` e stub in `mock.ts`.

  - [ ] Misura delle istruzioni Lua per richiesta del motore (P3, «in misura»): conteggio solo con `DUELFORGE_LUA_COUNT`, giro su 41 deck × 2 mani a 20 s, soglia per deck che toglierebbe i secondi oltre il limite di elfnote e kewl_tune e quante richieste normali la supererebbero. Senza la variabile banco e `combo_hunt --seed 7` identici. Il tetto vero, se serve, in una PR a parte.

  Agente: claude-code, sessione sorella della velocità. Branch: `claude/search-speed`. File (lock del 22/10 per la misura Lua): uno shim nuovo in `crates/ygo-engine/shim/`, `crates/ygo-engine/build.rs`, una funzione nuova in `crates/ygo-engine/src/duel.rs`, `scripts/`, `docs/plans/velocita-ricerca.md`. Banditore v2 unito (PR #45): `lib.rs`, `api.ts` e `mock.ts` liberi.
- [ ] **Corsia 3.** Collaudo:
  - [x] prima, le coppie di board da far giudicare al proprietario (circa 15 deck, le coppie dove la classifica è più incerta), per la #47 e i pesi (#43): 46 coppie su 16 deck in `tests/fixtures/boards/owner-pairs.tsv` e `.json` (22/10);
  - [x] il collaudo del livello 2 con le linee vere (L4) su `48efae0`: 88% delle mosse fatte su campo vuoto, 71% di mosse in comune tra semi, problemi L4-1…L4-5 in `docs/plans/collaudo-strumenti.md` (22/10);
  - [ ] il collaudo dei deck lenti (corsia 2).

  Agente: claude-code, sessione sorella del collaudo. Branch: `claude/tools-qa`. File: script nuovi in `scripts/`, `docs/plans/collaudo-strumenti.md`, `tests/fixtures/boards/`.
- [ ] **Corsia 6.** Probabilità della mappa dei collegamenti: una `chance` che distingua le carte e sia vicina al vero (misurata su mani cercate), senza saturazione.

  Agente: claude-code, sessione sorella della mappa. Branch: `claude/links-chance`. File: `crates/ygo-solver/src/links.rs` e i suoi test, la parte `links` di `crates/ygo-cli/src/main.rs`, script nuovi miei in `scripts/`, `docs/plans/mappa-collegamenti.md`, `tests/fixtures/links/`.
- [ ] **Corsia 4, secondo task.** Ordine della ricerca e stima rapida (#47, D14 = B11 di `docs/plans/classificazione-board.md`): il #1 stampato è il migliore misurato; inversioni tra stima e misura nella top 8 (C5.7) ≤ 60; campi distinti nella top 8 (C5.8). Si accetta solo se `best` del banco non scende più del 5%.

  Agente: claude-code, sessione della corsia 4. Branch: `claude/placed-cards` (ripartito da `main`). File: `crates/ygo-solver/src/search.rs`, `value.rs`, `crates/ygo-cli/src/main.rs` (solo la parte `combos`), script miei in `scripts/`, `docs/plans/ordine-ricerca.md`.
  - [x] Linea di base (C5.7, C5.8, banco).
  - [x] 1. Riordino misurato nella CLI dopo `confirm_best`; verifica della top N nell'app.
  - [x] 2. Bonus fissi di `quick_score` solo per le carte non misurate.
  - [x] 3. Mano vuota in `value.rs`.
  - [x] 4. Seconda copia.
  - [x] 5. Spot mancanti.
  - [x] 6. Look key con Cimitero usabile: saltato (tocca `src-tauri/src/lib.rs`; nella CLI lo copre il 7).
  - [x] 7. Varianti nella top N.
  - [ ] C5.7: inversioni da 223 a 174 su 1110, obiettivo ≤ 60 non raggiunto; #1 stampato sempre il migliore (19 deck → 0). Numeri in `docs/plans/ordine-ricerca.md`.
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
  - [ ] Seguito per Gemini: la ricerca vera ha ora anche la lista OCG (`formato:ocg ban:forbidden ban:limited ban:semi`). Le carte limitate solo in OCG entrano nell'elenco iniziale con una terza ricerca nel `Promise.all` di `Banditore.tsx`; nel backend finto (`searchCatalog` in `mock.ts`) `ban:` con `formato:ocg` deve leggere `banOcg`.
  - [ ] Seguito per Gemini, la v2 («cosa è cambiato con l'ultima lista»): backend pronto nella PR della corsia 2, `api.banChanges(format?)` → `BanChange[]` (formato, id, data, stato prima e dopo; `null` è senza limiti), le più recenti prima. Per «l'ultima lista» di un formato si prendono le voci con la data più recente; per TCG e OCG la data è il giorno dell'aggiornamento dei dati di YGOPRODeck, non quello della lista. La storia comincia dal primo aggiornamento dopo questa versione: prima è vuota.
- [ ] **Corsia 5b (P1 L6).** Turno di prova contro un deck, livello 2: interfaccia: pulsante «Contro le sue linee», messaggio di attesa 10-30s, riepilogo `ComboMoves` (left, floodgateShare, value, warning madeShare < 0.8), linee dell'avversario con stati delle mosse («fatta», «fermata dalla board», «persa», «non rigiocata») ed evidenziazione al passaggio/clic delle tue risposte (`stops`). Agente: gemini. Branch: `gemini/matchup-moves`. File: `apps/desktop/src/components/combo/RivalMoves.tsx`, `apps/desktop/src/components/combo/BoardDetail.tsx`, `docs/plans/turno-di-prova-livello-2-ui.md`.
- [ ] Classificazione delle board più profonda, con criteri validati sui deck di Master Duel Meta (#47): D10, D13, D14 (corsia 4, PR #50) e D15 (PR #48) in `main`. D12 (esposizione come valore perso) alla corsia 4, con il suo lock. Restano D11 (esca, solo da mostrare), D16 (handtrap vera e ripresa) e D17 (pesi: aspetta i giudizi del proprietario). Agente: claude-code (coordinatore). Branch: `claude/yugioh-deck-builder-simulator-cax0ya`. File: nessun lock.
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
  - [x] D13, costo (B8): non più le carte della mano usate oltre la prima (quelle rimaste sono già contate), ma 0,05 per carta del Deck pagata come costo, 0,1 per mostro dell'Extra Deck finito nel Cimitero o bandito, 1 ogni 4000 LP pagati. Testo dell'interfaccia chiesto a Gemini sotto.
  - [x] D13, finestre prima dei passi del kit (B9, `PRE`): le tue risposte nella Draw e nella Standby avversarie si provano; coppie e banco invariati (piano §C10).
  - [x] D14 (corsia 4, PR #50): il #1 stampato è sempre il migliore misurato (prima no in 19 deck su 41); bonus fissi solo per le carte non misurate, valori a mano vuota, seconda copia, spot nuovi, un campo per riga. Inversioni tra stima e misura da 223 a 174 su 1110: l'obiettivo di 60 vuole una stima della board intera (`docs/plans/ordine-ricerca.md`).
  - [x] D15: i breaker che le liste di Master Duel Meta giocano davvero (Infinite Impermanence, Super Polymerization), i breaker che mirano alla carta che vale di più, la risposta dell'avversario alla tua prima interruzione (`countered`, solo da mostrare) e la protezione di ogni pezzo (`api.boardProtection`, a richiesta). Ricerca identica nel banco, coppie da confermare giuste da 14 a 16 (piano §C11). Interfaccia chiesta a Gemini sotto.
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
- [x] **Piccola correzione rimasta dalla PR #18** (da fare per prima, anche dentro la PR della mappa): in `apps/desktop/src/pages/Combo.tsx` il chip «vs X: …» (righe ~995-1000) e l'ordinamento «vs deck» (riga ~278) non guardano `s.matchup.error`: una board con il calcolo contro il deck fallito mostra «vs X: 0,00» e viene ordinata come se valesse 0. Escluderla in entrambi i punti (`s.matchup && !s.matchup.error`), e nel chip mostrare «vs X: errore» come in `Score.tsx`.
- [x] Turno di prova contro un deck scelto (#44, livello 1; backend pronto, piano in `docs/plans/turno-di-prova-matchup.md`). Nel Circolo Arcano:
  - un selettore "Contro: generico / deck della Bottega" vicino al pulsante che misura le board;
  - con un deck scelto si chiama `api.comboScore(vsDeckId)`: ogni `ComboScore` ha allora `matchup` (`ComboMatchup` in `api.ts`); cambiare deck rimisura tutte le board;
  - nella scheda del punteggio, accanto alla resilienza generica, la "resilienza contro X" (`matchup.resilience`, con `breakerChance` e la lista `breakers`: carta, probabilità di averla, valore che lascia, chi la nega). Con `standardBreakers` il deck non ha breaker giocabili: va detto che si usano quelli standard;
  - accanto all'esposizione, "fermata da X" (`handTrapChance`) e la lista `handTraps`: carta, probabilità, momenti in cui ferma la linea su quelli in cui si può usare (`stops`/`windows`), le tue carte in quei momenti (`at`). Le handtrap con `stops` a 0 (Maxx "C", Mulcharmy) si possono mostrare a parte: "si usano ma non fermano la linea";
  - una colonna o un ordinamento "contro X" nella classifica delle board, e il backend finto (`mock.ts`) con un `matchup` di esempio.
- [x] Mappa dei collegamenti tra le carte, interfaccia base (PR #22, piano `docs/plans/mappa-collegamenti.md`): scheda «Collegamenti» nel Laboratorio, cerchio carte, frecce colorate per tipo (`CardLink.kind`, incluso `"set"`), spessore per `used`, tratteggio per possibili; filtri per tipo con conteggi e per zona; pannello dettagli con `reach`, `reachAll`, `chance` (e `LinkCard.measured` per stime) e conteggi.
- [ ] **Turno di prova contro un deck, livello 2: interfaccia** (dopo Il Banditore v1; backend pronto, piano P1 in `docs/plans/piano-prossimi-passi.md`). Nel Circolo Arcano, nella scheda del punteggio di una board misurata «contro X» (livello 1), un pulsante «Contro le sue linee» che chiama `api.comboMoves(hand, key, vsDeckId)` a richiesta, per la board aperta.
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
- [ ] **Testi di follow-up, risorse e costo** (dopo la PR di D13, #47). Il backend ora conta nel follow-up anche il tuo turno dopo: dopo il board breaker peggiore, ogni effetto che riparte dal Cimitero o dalle bandite vale 0,3. Le risorse non contano più il Cimitero. In `apps/desktop/src/lib/scoring.ts`:
  - `followup`: `hint` ed `explain` dicono anche «effetti da Cimitero e bandite nel tuo turno dopo, dopo il board breaker peggiore (0,3 l'uno)»;
  - `resources`: togliere «e un po' il Cimitero»;
  - `cost`: non più «carte della mano usate oltre la prima», ma «quello che la linea spende oltre alla mano: carte del Deck pagate come costo (0,05 l'una), mostri dell'Extra Deck finiti nel Cimitero o banditi (0,1 l'uno), LP pagati (1 ogni 4000)». Le carte della mano rimaste contano già nelle altre voci;
  - il dettaglio resta in `followupNote` e `resourcesNote`, che arrivano già dal backend. Il contratto (`ComboScore`) non cambia.
- [ ] **Risposta alla prima interruzione e protezione dei pezzi** (dopo la PR di D15, #47; backend pronto, piano `docs/plans/classificazione-board.md` §C11). Nella scheda del punteggio di una board misurata (Circolo Arcano):
  - `ComboScore.countered` (da 0 a 1, o `null`): «con una risposta alla tua prima interruzione (Infinite Impermanence) fermi ancora il X% di quello che fermi». Con `null` non si mostra niente: l'avversario non ha potuto rispondere, o la board non ferma niente. Solo da mostrare, non entra nel punteggio;
  - un pulsante «Protezione dei pezzi» che chiama `api.boardProtection(hand, key)` → `ComboPiece[]` a richiesta, per la board aperta (qualche secondo: si rimisura la board). Per ogni carta che interrompe: «regge a: …» (`holds`) e «cade a: …» (`falls`), con le parole che arrivano dal backend ("distruggere", "bandire", "girare coperto", "negare"). La carta risponde solo con i suoi effetti; contano le protezioni durature della board (Stardust Sifr);
  - id di richiesta e azzeramento quando cambia la board, errori in italiano (protocollo dell'interfaccia in `AGENTS.md`). Il backend finto (`mock.ts`) ha già un esempio di entrambi.
- [ ] Tornare a TypeScript 7 (`apps/desktop/package.json`) quando `typescript-eslint` lo supporta: con la PR #29 è fissato a 6.0.3 perché `typescript-eslint` 8 non regge ancora la versione 7. Poi, già che ci si è, `tseslint.config(...)` → `defineConfig` di `eslint/config` in `eslint.config.js`.

### Motore e ricerca (Claude)
- [ ] Linee del video Invoked: come si classificano, correzioni generali (#45).
- [ ] Scale e pesi delle metriche del punteggio (#43).
- [ ] Frontiera di Pareto e confronto tra board; preferenze apprese (#40).
- [ ] Simulatore: giocare passo per passo una combo scelta (#36, alla fine).
- [x] Limiti noti delle carte piazzate: equipaggiamenti, materiali dei mostri dell'Extra Deck (PIANO §30.4, PR #42).
- [ ] **Più avanti, dopo la #47 e la #43:** valutatore imparato. Una rete piccola stima il turno di prova durante la ricerca, al posto di `quick_score`, come NNUE in Stockfish; il motore e le regole restano gli stessi (`docs/plans/valutatore-imparato.md`).

### Richieste dal backend per l'interfaccia

## Completati (recenti; lo storico è in `docs/PIANO.md`)
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
