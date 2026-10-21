# Stato del progetto (21/10/2026)

Breve indice per chi arriva. Il dettaglio è in `docs/PIANO.md` (v1.9, §30), le regole di lavoro in `AGENTS.md`, i lavori in corso in `docs/TASKS.md`.

## Fatto
| Area | Cosa c'è | PIANO |
|---|---|---|
| Dati | Carte, banlist (Master Duel, TCG, OCG), immagini scaricate a runtime, ricerca full-text con un linguaggio di filtri | §3, §17 |
| Fucina (deck builder) | Editor in stile Master Duel, Maybe board, mano di prova (5 ruoli, collegata al Circolo Arcano), barra dei ruoli e distribuzione della curva, import/export `.ydk`/`ydke://`, immagine del deck | §4, §18 |
| Bottega | Scaffale di deck box 3D personalizzabili, con collegamenti diretti al Laboratorio e al Circolo Arcano | §6, §19 |
| Laboratorio | Probabilità esatte per ruoli, Monte Carlo, ottimizzatore, registro partite, ruoli automatici, confronto tra deck; verificato contro riferimenti indipendenti (18 difetti corretti) | §7, §21, §29.7, §30.7 |
| Combo Lab (motore) | Ricerca automatica delle linee del primo turno con il motore EDOPro, obiettivi, ripresa; 12 difetti trovati con una caccia ai bug, 10 corretti | §9, §23–§25, §29, §30.6 |
| Combo Lab (punteggio) | Turno di prova contro un avversario con carte vere: interruzioni, handtrap, floodgate, resilienza, esposizione; profili e pesi; scheda del punteggio spiegata | §26, §29.9 |
| Combo Lab (interfaccia) | Confronto affiancato tra due board, riproduttore passo-passo della linea, frecce Link, copia della guida | — |
| App | Guida di benvenuto, palette dei comandi (`Ctrl+K`), scorciatoie, tema chiaro e scuro a 1280×800 | — |
| Qualità | CI per pull request (solo le parti toccate), test di regressione della ricerca, bench su GitHub Actions, installer Windows su `main`; strumento di copertura delle meccaniche | §28, §30 |

## A metà
- **Velocità del motore**: allocatore mimalloc per Lua (ricerca dal 22 al 50% più veloce a parità di risultati) e altri prototipi in misura; la patch finale va su un branch e poi in una PR.
- **Classificazione delle board** (#47): strumenti di misura e linea di base pronti (D0: 83 coppie giudicate, sicure giuste 17 su 33, il difetto principale è la parità); mancano la protezione della board e le risorse per il turno dopo. Piano: `docs/plans/classificazione-board.md`.
- **Linee del video Invoked** (#45): la ricerca trova le board; la classifica va rivista insieme alla #47.

## Bug e limiti noti
- **Valori delle carte sul campo**: non dipendono dal contesto (mano vuota, equipaggiamenti); passa nella #47 (§30.6).
- **Carte piazzate** (ruoli, valori): gli equipaggiamenti non hanno bersaglio e i mostri dell'Extra Deck non hanno materiali (Apollousa a 0 ATK) (§30.4).
- **Ruoli**: la prova dell'estensore riparte da un turno nuovo (§30.4).
- **Board breaker**: riconosciuti dal testo, non ancora misurati con il motore (§29.10).
- **Bench**: alcune mani (branded, mitsurugi) trovano il board migliore solo alla fine del budget, quindi oscillano a ogni cambio della ricerca (§30.6).

## Decisioni importanti (il perché è nel PIANO)
- **Motore EDOPro** (AGPL) invece di un motore scritto da zero: applica già tutti i ruling e gli script delle carte (§8.1, §15).
- **Master Duel come primo formato**; nomi e testi delle carte in inglese, interfaccia in italiano (§0, §1).
- **Nessun asset Konami nel repository**: carte e immagini si scaricano (§15.2).
- **Ricerca con replay**: il motore non copia una partita in corso, quindi ogni nodo si raggiunge rigiocando la linea. Tutte le ottimizzazioni puntano a ridurre i replay o a renderli più economici (§23, §25, §29.4).
- **Punteggio misurato** con un turno di prova vero invece che stimato dal testo delle carte (§22, §26).
- **Correzioni sempre generali**, validate su molti deck, mai per nome di carta (§28, §29.3).
- **Due agenti**: Claude Code sul motore e sul backend, Gemini (Antigravity) sull'interfaccia; ogni PR la controlla l'altro, il merge lo decide l'utente (§30.5, `AGENTS.md`).
- **Minuti di GitHub Actions contati** (repository privato): CI solo sulle parti toccate, bench solo a mano.

## Richieste dell'utente non ancora fatte
- Turno di prova contro un deck avversario a scelta (#44).
- Simulatore passo per passo di una combo (#36, alla fine).
- Preferenze apprese e frontiera di Pareto (#40).

## Prossimi passi
1. Velocità del motore: patch finale verificata, poi PR.
2. Stato del campo a ogni passo della linea, per il riproduttore dell'interfaccia (richiesta di Gemini in `docs/TASKS.md`).
3. Classificazione delle board (#47), poi i pesi (#43).
4. Interfaccia (Gemini): Visore Carta interattivo, poi la pagina «Il Banditore».
5. Turno di prova contro un deck scelto (#44), Pareto e confronto (#40), simulatore (#36).
