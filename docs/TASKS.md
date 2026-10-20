# Bacheca dei lavori

Regole in `AGENTS.md`. Per prendere un task, spostalo in "In corso" con agente, branch e file che tocchi (è il lock), poi committa e pusha questa modifica prima di cominciare.

## In corso
- [ ] Audit della copertura delle meccaniche su 45 deck: analisi dei risultati e correzioni generali (PIANO §30.2–30.3) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/search.rs, coverage.rs

## Da fare (in ordine di priorità)

### Interfaccia (Gemini)
- [ ] Ponte rapido tra Laboratorio, Bottega e Circolo Arcano (avvio ricerca combo 1-carta dal Laboratorio, apertura diretta di un Deck Box in Laboratorio o Circolo Arcano dalla Bottega).
- [ ] Visore Carta (`CardViewer`) interattivo: aggiunta/rimozione copie (`+ Main`, `+ Side`, `+ Maybe`, `−`) e assegnazione rapida dei ruoli senza chiudere il visore.
- [ ] Pagina «Il Banditore» (`Novità`): confronto tra banlist (Master Duel, TCG, OCG) e impatto immediato sui deck salvati in bottega.

### Motore e ricerca (Claude)
- [ ] Classificazione delle board più profonda, con criteri validati sui deck di Master Duel Meta (#47; PIANO §29.10).
- [ ] Linee del video Invoked: come si classificano, correzioni generali (#45).
- [ ] Scale e pesi delle metriche del punteggio (#43).
- [ ] Turno di prova contro un deck avversario scelto dall'utente (#44).
- [ ] Frontiera di Pareto e confronto tra board; preferenze apprese (#40).
- [ ] Simulatore: giocare passo per passo una combo scelta (#36, alla fine).
- [ ] Limiti noti delle carte piazzate: equipaggiamenti, materiali dei mostri dell'Extra Deck (PIANO §30.4).

### Richieste dal backend per l'interfaccia
- [ ] Stato del campo e della mano a ogni passo della linea in `ComboStep` (con la stessa forma del campo della board) per collegare il tappetino al riproduttore passo per passo.

## Completati (recenti; lo storico è in `docs/PIANO.md`)
- [x] Riproduttore passo-passo della linea (controlli, evidenziazione del passo e delle carte coinvolte), frecce Link sul campo e «Copia guida» (solo nomi e passi) nel Circolo Arcano (PR #8). Il tappetino passo per passo si collega quando arriva lo stato per passo in `ComboStep`.
- [x] Filtri rapidi per ruolo, riconoscimento ruoli, distribuzione della curva e Mano di prova collegata al Circolo Arcano nella Fucina (PR #7).
- [x] Confronto affiancato tra due board nel Circolo Arcano (differenze su campo, interruzioni, resilienza e punteggio, PR #6).
- [x] Navigazione da tastiera e palette dei comandi rapida (`Ctrl+1..7` per le sezioni, `Ctrl+K` per la palette, `?` per la guida e le scorciatoie, PR #5).
- [x] Guida per chi usa l'app per la prima volta (prima apertura: scaricare i dati, creare un deck, cercare le combo), in italiano (`WelcomeModal`, checklist in `Overview`, pulsante `Guida rapida`, PR #4).
- [x] Revisione dell'interfaccia (parte 2): Laboratorio statistico, grafici SVG, Bottega e Fucina su schermi piccoli (`1280×800`, PR #3).
- [x] Revisione dell'interfaccia (parte 1): tema chiaro "Pergamena" e Circolo Arcano su schermi piccoli (`1280×800`, PR #2).
- [x] Backend finto per sviluppare l'interfaccia nel browser (`VITE_MOCK=1` in `src/lib/api.ts` e `src/lib/mock.ts`, PR #1).
- [x] Revisione della v1.8: 21 difetti confermati e corretti (PIANO §30.1).
- [x] Strumento `duelforge-cli coverage`; linee Pendulum non più scartate; negazioni delle proprie catene imparate; Set in un solo ordine (PIANO §30.2–30.3).
- [x] Punteggio leggibile con i tasti "i"; handtrap morte come carte da scartare; metrica di esposizione (PIANO §29).
- [x] Regole e file per lavorare con due agenti (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, questa bacheca, `docs/STATO.md`).
