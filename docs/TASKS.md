# Bacheca dei lavori

Regole in `AGENTS.md`. Per prendere un task, spostalo in "In corso" con agente, branch e file che tocchi (è il lock), poi committa e pusha questa modifica prima di cominciare.

## In corso
- [ ] Audit della copertura delle meccaniche su 45 deck: analisi dei risultati e correzioni generali (PIANO §30.2–30.3) — agente: claude-code — branch: claude/yugioh-deck-builder-simulator-cax0ya — file: crates/ygo-solver/src/search.rs, coverage.rs
- [ ] Revisione dell'interfaccia del Combo Lab e del Laboratorio su schermi piccoli (1280×800) e nel tema chiaro: elenco dei problemi, poi le correzioni. — agente: antigravity — branch: gemini/ui-review — file: docs/plans/ui-review.md, apps/desktop/src/index.css, apps/desktop/src/pages/Combo.tsx, apps/desktop/src/pages/Lab.tsx, apps/desktop/src/components/SectionHeader.tsx, apps/desktop/src/components/BoxEditor.tsx, apps/desktop/src/components/charts.tsx, apps/desktop/src/components/combo/BoardDetail.tsx, apps/desktop/src/components/combo/Field.tsx, apps/desktop/src/components/combo/Score.tsx, apps/desktop/src/components/combo/Steps.tsx, apps/desktop/src/components/lab/OverviewTab.tsx, apps/desktop/src/components/lab/CompareTab.tsx, apps/desktop/src/components/lab/CalculatorTab.tsx, apps/desktop/src/components/lab/SimulatorTab.tsx, apps/desktop/src/components/lab/OptimizerTab.tsx, apps/desktop/src/components/lab/LogTab.tsx, apps/desktop/src/components/lab/RolesPanel.tsx, apps/desktop/src/components/lab/MatchEntryPanel.tsx

## Da fare (in ordine di priorità)

### Interfaccia (Gemini)
- [ ] Guida per chi usa l'app per la prima volta (prima apertura: scaricare i dati, creare un deck, cercare le combo), in italiano.

### Motore e ricerca (Claude)
- [ ] Classificazione delle board più profonda, con criteri validati sui deck di Master Duel Meta (#47; PIANO §29.10).
- [ ] Linee del video Invoked: come si classificano, correzioni generali (#45).
- [ ] Scale e pesi delle metriche del punteggio (#43).
- [ ] Turno di prova contro un deck avversario scelto dall'utente (#44).
- [ ] Frontiera di Pareto e confronto tra board; preferenze apprese (#40).
- [ ] Simulatore: giocare passo per passo una combo scelta (#36, alla fine).
- [ ] Limiti noti delle carte piazzate: equipaggiamenti, materiali dei mostri dell'Extra Deck (PIANO §30.4).

### Richieste dal backend per l'interfaccia
(Gemini scrive qui i dati nuovi che gli servono; li aggiunge Claude in backend e `api.ts`.)

## Completati (recenti; lo storico è in `docs/PIANO.md`)
- [x] Backend finto per sviluppare l'interfaccia nel browser (`VITE_MOCK=1` in `src/lib/api.ts` e `src/lib/mock.ts`, PR #1).
- [x] Revisione della v1.8: 21 difetti confermati e corretti (PIANO §30.1).
- [x] Strumento `duelforge-cli coverage`; linee Pendulum non più scartate; negazioni delle proprie catene imparate; Set in un solo ordine (PIANO §30.2–30.3).
- [x] Punteggio leggibile con i tasti "i"; handtrap morte come carte da scartare; metrica di esposizione (PIANO §29).
- [x] Regole e file per lavorare con due agenti (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, questa bacheca, `docs/STATO.md`).
