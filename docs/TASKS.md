# Bacheca dei lavori

Regole in `AGENTS.md`. Per prendere un task, spostalo in "In corso" con agente, branch e file che tocchi (è il lock), poi committa e pusha questa modifica prima di cominciare.

## In corso
- [ ] Pesi imparati dalle tue scelte e frontiera di Pareto (#40), interfaccia nel Circolo Arcano (piano `docs/plans/pesi-imparati-pareto-ui.md`) — agente: gemini — branch: gemini/learned-weights — file: apps/desktop/src/components/combo/*, apps/desktop/src/pages/Combo.tsx, apps/desktop/src/lib/scoring.ts, apps/desktop/src/lib/mock.ts
- [ ] Mappa dei collegamenti tra le carte, backend (piano `docs/plans/mappa-collegamenti.md`): F0 contratto, F1 collegamenti letti dal motore, carte chiave (`LinkCard.key`) e correzioni del collaudo (tipo `set`, `LinkCard.measured`, tempo massimo) fatti; poi la mappa quasi in tempo reale (bozza dal testo subito, aggiornamento progressivo, ricalcolo solo delle carte cambiate; approvata dal proprietario, dopo Maxx "C"); resta il confronto tra versioni del deck — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/links.rs (nuovo), lib.rs, crates/ygo-cli/src/main.rs, apps/desktop/src-tauri/src/lib.rs, apps/desktop/src/lib/api.ts (solo i tipi)
- [ ] Maxx "C" e Mulcharmy (approvato dal proprietario il 21/10): passo 1, quante carte pesca l'avversario a ogni momento della linea in cui le usa (`RivalHandTrap.drawn` e `moments`); passo 2, il punto migliore dove fermarsi (valore della board contro carte fatte pescare) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/matchup.rs, crates/ygo-cli/src/main.rs, apps/desktop/src-tauri/src/lib.rs (MatchupView), apps/desktop/src/lib/api.ts (solo i tipi)
- [ ] Velocità della ricerca: profilazione e correzioni a risultati invariati (resoconto in `docs/plans/velocita-ricerca.md` sul suo branch) — agente: claude-code, sessione sorella — branch: claude/search-speed — file: crates/ygo-solver/src/search.rs, card.rs, value.rs, probe.rs (solo velocità), crates/ygo-engine/src/, scripts/
- [ ] Collaudo del turno di prova contro un deck e della mappa dei collegamenti su circa 45 deck di Master Duel Meta: resoconto con i problemi e le correzioni proposte (`docs/plans/collaudo-strumenti.md` sul suo branch); le correzioni ai crate le applica claude-code — agente: claude-code, sessione sorella — branch: claude/tools-qa — file: script nuovi in scripts/, docs/plans/collaudo-strumenti.md
- [ ] Classificazione delle board più profonda, con criteri validati sui deck di Master Duel Meta (#47) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/probe.rs, value.rs, explain.rs, tests/probe.rs, crates/ygo-engine/src/message.rs, apps/desktop/src-tauri/src/lib.rs (score_view)
  - Piano: `docs/plans/classificazione-board.md` (passi D0–D17; giudizio dato da un pannello di agenti, non dal proprietario).
  - [x] D0a: `combos --json/--save-lines`, `probe-line`.
  - [x] D0b: `combo_hunt.py --seed/--json`, `board_pairs.py`, `rank_report.py`, `tests/fixtures/boards/pairs.tsv`, numeri di partenza (piano §C7).
  - [x] Pannello di giudici (tre agenti): 74 coppie in 39 deck (30 sicure). Sicure giuste 17 su 33, da confermare 7 su 45; il difetto più frequente è la parità (piano §C7).
  - [ ] D1–D7: correzioni della misura (bersagli, fermate, copertura, copie, run congiunta, floodgate finti). D2, D3 e D4 fatti (piano §C8); D5–D7 toccano poco le coppie: da decidere con l'utente se farli o passare al kit (D10) e alle risorse (D13).
    - [x] D2: una fermata conta solo per il passo giocato, anche per i floodgate (test in `crates/ygo-solver/tests/probe.rs`).
    - [x] D3: un mostro conta come Evocato quando atterra; "nega l'Evocazione" solo prima dell'atterraggio (evento `Summoned` nel motore).
    - [x] Seguito delle risposte: le carte portate da una risposta (un mostro Evocato) rispondono anche loro; si Evoca il mostro più forte; si paga con la carta meno preziosa. Coppie sicure giuste da 14 a 18 su 29 (piano §C8).
    - [x] D4: un mostro girato coperto ha gli effetti negati; una carta attaccata come materiale Xyz è come bandita. Il resto di A4 non serve più dopo D2 e D3 (restavano 3 carte con etichette diverse tra le board).
- [ ] Correzioni dalla caccia ai bug della ricerca (12 problemi confermati) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/search.rs, probe.rs, roles.rs, crates/ygo-engine/src/auto.rs
  - [x] Set in un solo ordine anche dopo una finestra di catena; ricerca fermata a metà che riparte dal punto giusto.
  - [x] Carte rapide tenute per dopo nel dive; handtrap giudicate da ciò che permettono (Labrynth Cooclock); carte da tenere rispettate durante la ricerca.
  - [x] Zone Magie/Trappole centrali prima delle Zone Pendulum; ruoli e copertura con la stessa lista di handtrap del Combo Lab.
  - [x] Turno di prova: bersagli sulla carta che l'avversario usa; costi pagati con carte che non rispondono.
  - [x] Catena non risolta nella chiave dei nodi; test di regressione della ricerca in CI (PIANO §30.6).
  - [ ] Valori delle carte che dipendono dal contesto (mano vuota, equipaggiamenti): passa nella #47.

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
- [x] Mappa dei collegamenti tra le carte, interfaccia (piano `docs/plans/mappa-collegamenti.md`; il proprietario ha scelto una scheda «Collegamenti» nel Laboratorio). Dopo il turno di prova contro un deck. Il backend vero c'è già (`api.deckLinks(deckId)`, circa mezzo minuto la prima volta, avanzamento con `api.onLinksProgress`); per lavorare nel browser, in `mock.ts` lo stub va sostituito con un esempio completo sul deck Invoked (si può partire dall'uscita JSON di `duelforge-cli links --deck tests/fixtures/decks/invoked-siracusa-md.ydk --json <file>`: carte, collegamenti dei cinque tipi con `zone`, `possible` e `used`).
  - Cerchio di carte con frecce colorate per tipo (`CardLink.kind`), spessore per `used`, tratteggio per i solo possibili; filtri per tipo con i conteggi e per zona (dal Cimitero o dalla mano si possono mostrare più tenui: non portano carte nuove); al passaggio o al clic si accendono le frecce della carta e un riquadro mostra `reach`, `reachAll`, `chance` (un trattino quando è 0 su un mostro dell'Extra Deck) e i conteggi per tipo.
  - Dal backend (PR della mappa, correzioni del collaudo): `CardLinkKind` ha un tipo nuovo, `"set"` ("posiziona dal Deck": una carta messa dal Deck sul campo), che va aggiunto a `LINK_KINDS` (altrimenti `pnpm typecheck` fallisce e le frecce di quel tipo rompono la pagina); `LinkCard.measured` dice che `chance` è una stima misurata (mostri dell'Extra Deck senza materiali nominati): mostrarla come «~ 40% (stima)» invece del trattino. La fixture `tests/fixtures/links/invoked.json` è aggiornata (101 collegamenti).
  - Poi (F2): mano di prova sovrapposta (si accende ciò che raggiunge), percorso più corto tra due carte calcolato sui `links` con un pulsante verso il Combo Lab (obiettivo = carta d'arrivo), probabilità `chance` per carta, carte chiave (`LinkCard.key`: carte raggiunte in meno in media se la carta viene negata; ad esempio una classifica o la dimensione della carta nel cerchio), vista a colonne (starter → intermedie → Extra Deck).
- [ ] Pesi imparati dalle tue scelte e frontiera di Pareto (#40; backend pronto). Nel Circolo Arcano:
  - nel confronto affiancato di due board, un pulsante «Preferisco questa» su ognuna: chiama `api.recordBoardChoice(deckId, chosen, other)` con le metriche delle due board per chiave (`MetricKey` di `scoring.ts`: i valori di `ComboScore`), e mostra quante scelte ci sono;
  - nel pannello dei pesi, un profilo «Imparato dalle tue scelte»: `api.learnedWeights(prior)` con `prior` = i pesi del profilo in uso; mostra `choices` e quante scelte spiegano i pesi imparati e quelli di partenza (`agreement`, `priorAgreement`); un pulsante per azzerare (`api.clearBoardChoices`). Con poche scelte i pesi restano vicini al profilo di partenza;
  - un filtro «Solo board non dominate» (frontiera di Pareto, calcolata nell'interfaccia sui `ComboScore` misurati): una board è dominata se un'altra è almeno uguale in tutte le metriche (per costo ed esposizione conta il valore più basso) e migliore in una.
- [ ] Pagina «Il Banditore» (`Novità`): confronto tra banlist (Master Duel, TCG, OCG) e impatto immediato sui deck salvati in bottega.

### Motore e ricerca (Claude)
- [ ] Linee del video Invoked: come si classificano, correzioni generali (#45).
- [ ] Scale e pesi delle metriche del punteggio (#43).
- [ ] Frontiera di Pareto e confronto tra board; preferenze apprese (#40).
- [ ] Simulatore: giocare passo per passo una combo scelta (#36, alla fine).
- [ ] Limiti noti delle carte piazzate: equipaggiamenti, materiali dei mostri dell'Extra Deck (PIANO §30.4).

### Richieste dal backend per l'interfaccia

## Completati (recenti; lo storico è in `docs/PIANO.md`)
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
