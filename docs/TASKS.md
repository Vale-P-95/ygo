# Bacheca dei lavori

Regole in `AGENTS.md`. Per prendere un task, spostalo in "In corso" con agente, branch e file che tocchi (è il lock), poi committa e pusha questa modifica prima di cominciare.

## In corso
- [ ] Classificazione delle board più profonda, con criteri validati sui deck di Master Duel Meta (#47) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/probe.rs, value.rs, explain.rs, apps/desktop/src-tauri/src/lib.rs (score_view)
  - Piano: `docs/plans/classificazione-board.md` (passi D0–D17; giudizio dato da un pannello di agenti, non dal proprietario).
  - [x] D0a: `combos --json/--save-lines`, `probe-line`.
  - [x] D0b: `combo_hunt.py --seed/--json`, `board_pairs.py`, `rank_report.py`, `tests/fixtures/boards/pairs.tsv`, numeri di partenza (piano §C7).
  - [x] Pannello di giudici (tre agenti): 74 coppie in 39 deck (30 sicure). Sicure giuste 17 su 33, da confermare 7 su 45; il difetto più frequente è la parità (piano §C7).
  - [ ] D1–D7: correzioni della misura (bersagli, fermate, copertura, copie, run congiunta, floodgate finti).
- [ ] Correzioni dalla caccia ai bug della ricerca (12 problemi confermati) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/search.rs, probe.rs, roles.rs, crates/ygo-engine/src/auto.rs
  - [x] Set in un solo ordine anche dopo una finestra di catena; ricerca fermata a metà che riparte dal punto giusto.
  - [x] Carte rapide tenute per dopo nel dive; handtrap giudicate da ciò che permettono (Labrynth Cooclock); carte da tenere rispettate durante la ricerca.
  - [x] Zone Magie/Trappole centrali prima delle Zone Pendulum; ruoli e copertura con la stessa lista di handtrap del Combo Lab.
  - [x] Turno di prova: bersagli sulla carta che l'avversario usa; costi pagati con carte che non rispondono.
  - [x] Catena non risolta nella chiave dei nodi; test di regressione della ricerca in CI (PIANO §30.6).
  - [ ] Valori delle carte che dipendono dal contesto (mano vuota, equipaggiamenti): passa nella #47.

- [x] Velocità del motore: mimalloc per Lua (−27% di tempo di ricerca, risultati identici) e meno replay inutili (−9%), sul branch di Claude in attesa di PR (PIANO §30.8).
- [ ] Chiave dei nodi con i bersagli e i nomi dichiarati finché la catena è aperta (oggi due bersagli diversi sembrano lo stesso nodo) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/search.rs, crates/ygo-engine/src/auto.rs
  - [ ] Chiave corretta; tolto il taglio dei replay che si basava sul limite vecchio.
  - [ ] Bench prima e dopo, copertura, test di regressione; si tiene solo se le board non peggiorano.
- [x] Controllo di salute del motore su 45 deck: nessuna linea interrotta (PIANO §30.8).

## Da fare (in ordine di priorità)

### Interfaccia (Gemini)
- [ ] Visore Carta (`CardViewer`) interattivo: aggiunta/rimozione copie (`+ Main`, `+ Side`, `+ Maybe`, `−`) e assegnazione rapida dei ruoli senza chiudere il visore.
- [ ] Pagina «Il Banditore» (`Novità`): confronto tra banlist (Master Duel, TCG, OCG) e impatto immediato sui deck salvati in bottega.

### Motore e ricerca (Claude)
- [ ] Linee del video Invoked: come si classificano, correzioni generali (#45).
- [ ] Scale e pesi delle metriche del punteggio (#43).
- [ ] Turno di prova contro un deck avversario scelto dall'utente (#44).
- [ ] Frontiera di Pareto e confronto tra board; preferenze apprese (#40).
- [ ] Simulatore: giocare passo per passo una combo scelta (#36, alla fine).
- [ ] Limiti noti delle carte piazzate: equipaggiamenti, materiali dei mostri dell'Extra Deck (PIANO §30.4).

### Richieste dal backend per l'interfaccia
- [ ] Stato del campo e della mano a ogni passo della linea in `ComboStep` (con la stessa forma del campo della board) per collegare il tappetino al riproduttore passo per passo.

## Completati (recenti; lo storico è in `docs/PIANO.md`)
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
