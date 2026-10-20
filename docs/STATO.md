# Stato del progetto (20/10/2026)

Breve indice per chi arriva. Il dettaglio è in `docs/PIANO.md` (v1.9), le regole di lavoro in `AGENTS.md`, i lavori in corso in `docs/TASKS.md`.

## Fatto
| Area | Cosa c'è | PIANO |
|---|---|---|
| Dati | Carte, banlist (Master Duel, TCG, OCG), immagini scaricate a runtime, ricerca full-text con un linguaggio di filtri | §3, §17 |
| Fucina (deck builder) | Editor in stile Master Duel, Maybe board, mano di prova, import/export `.ydk`/`ydke://`, immagine del deck | §4, §18 |
| Bottega | Scaffale di deck box 3D personalizzabili | §6, §19 |
| Laboratorio | Probabilità esatte per ruoli, Monte Carlo, ottimizzatore, registro partite, ruoli automatici, confronto tra deck | §7, §21, §29.7 |
| Combo Lab (motore) | Ricerca automatica delle linee del primo turno con il motore EDOPro, obiettivi, ripresa | §9, §23–§25, §29 |
| Combo Lab (punteggio) | Turno di prova contro un avversario con carte vere: interruzioni, handtrap, floodgate, resilienza ai board breaker, esposizione alle handtrap; profili e pesi; scheda del punteggio spiegata | §26, §29.9 |
| Qualità | CI con test e installer Windows; caccia ai bug su 37 deck MDM e 8 archetipi; strumento di copertura delle meccaniche | §28, §30 |

## A metà
- **Classificazione delle board** (#47): l'esposizione è il primo criterio nuovo; mancano la protezione della board e le risorse per il turno dopo. File: `crates/ygo-solver/src/probe.rs`, `value.rs`, `apps/desktop/src-tauri/src/lib.rs` (`score_view`).
- **Linee del video Invoked** (#45): la ricerca trova le board; la classifica va rivista insieme alla #47.
- **Audit della copertura** (§30): l'analisi dei 45 deck è in corso.

## Bug e limiti noti
- **Carte piazzate** (ruoli, valori): gli equipaggiamenti non hanno bersaglio e i mostri dell'Extra Deck non hanno materiali (Apollousa a 0 ATK) (§30.4).
- **Ruoli**: la prova dell'estensore riparte da un turno nuovo (§30.4).
- **Geometria dei Link**: alcune disposizioni si trovano solo variando le zone alla fine (§29.10).
- **Board breaker**: riconosciuti dal testo, non ancora misurati con il motore (§29.10).

## Decisioni importanti (il perché è nel PIANO)
- **Motore EDOPro** (AGPL) invece di un motore scritto da zero: applica già tutti i ruling e gli script delle carte (§8.1, §15).
- **Master Duel come primo formato**; nomi e testi delle carte in inglese, interfaccia in italiano (§0, §1).
- **Nessun asset Konami nel repository**: carte e immagini si scaricano (§15.2).
- **Ricerca con replay**: il motore non copia una partita in corso, quindi ogni nodo si raggiunge rigiocando la linea. Tutte le ottimizzazioni puntano a ridurre i replay (§23, §25, §29.4).
- **Punteggio misurato** con un turno di prova vero invece che stimato dal testo delle carte (§22, §26).
- **Correzioni sempre generali**, validate su molti deck, mai per nome di carta (§28, §29.3).

## Richieste dell'utente non ancora fatte
- Turno di prova contro un deck avversario a scelta (#44).
- Simulatore passo per passo di una combo (#36, alla fine).
- Preferenze apprese e frontiera di Pareto (#40).

## Prossimi passi
1. Chiudere l'audit della copertura (§30) e correggere quello che emerge.
2. Backend finto per l'interfaccia (Gemini), così l'interfaccia si sviluppa senza compilare il motore.
3. Classificazione delle board (#47), poi i pesi (#43).
4. Turno di prova contro un deck scelto (#44).
5. Pareto e confronto tra board (#40).
6. Simulatore (#36).
