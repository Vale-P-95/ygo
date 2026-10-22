# Stato del progetto (22/10/2026)

Breve indice per chi arriva. Il dettaglio è in `docs/PIANO.md` (v1.9, §30), le regole di lavoro in `AGENTS.md`, i lavori in corso in `docs/TASKS.md`.

## Fatto
| Area | Cosa c'è | PIANO |
|---|---|---|
| Dati | Carte, banlist (Master Duel, TCG, OCG), immagini scaricate a runtime, ricerca full-text con un linguaggio di filtri | §3, §17 |
| Fucina (deck builder) | Editor in stile Master Duel, Maybe board, mano di prova (5 ruoli, collegata al Circolo Arcano), barra dei ruoli e distribuzione della curva, import/export `.ydk`/`ydke://`, immagine del deck | §4, §18 |
| Bottega | Scaffale di deck box 3D personalizzabili, con collegamenti diretti al Laboratorio e al Circolo Arcano | §6, §19 |
| Laboratorio | Probabilità esatte per ruoli, Monte Carlo, ottimizzatore, registro partite, ruoli automatici, confronto tra deck; verificato contro riferimenti indipendenti (18 difetti corretti). Mappa dei collegamenti tra le carte letta dal motore: quasi in tempo reale, carte chiave, percorso più corto, confronto tra versioni del deck | §7, §21, §29.7, §30.7, §30.11 |
| Combo Lab (motore) | Ricerca automatica delle linee del primo turno con il motore EDOPro, obiettivi, ripresa; 12 difetti trovati con una caccia ai bug, 10 corretti | §9, §23–§25, §29, §30.6 |
| Combo Lab (punteggio) | Turno di prova contro un avversario con carte vere: interruzioni, handtrap, floodgate, resilienza, esposizione; profili e pesi; scheda del punteggio spiegata. Contro un deck della Bottega: resilienza contro i suoi breaker e handtrap che fermano la linea; Maxx "C" e Mulcharmy (carte fatte pescare, dove fermarsi) | §26, §29.9, §30.10 |
| Combo Lab (interfaccia) | Confronto affiancato tra due board, pesi imparati dalle tue scelte e frontiera di Pareto, riproduttore passo-passo della linea con il campo a ogni passo, frecce Link, copia della guida | — |
| App | Guida di benvenuto, palette dei comandi (`Ctrl+K`), scorciatoie, tema chiaro e scuro a 1280×800 | — |
| Qualità | CI per pull request (solo le parti toccate), ESLint dell'interfaccia, test di regressione della ricerca, bench su GitHub Actions, installer Windows su `main`; strumento di copertura delle meccaniche | §28, §30 |
| Velocità | Partite riusate, script caricati una volta per thread, mimalloc per Lua e per il C++: ricerca −37% con uscite identiche (PR #33) | `docs/plans/velocita-ricerca.md` |

## A metà
- **Turno di prova contro un deck, livello 2** (PR #35, da rivedere): l'avversario gioca le sue mosse tipiche, solo nella CLI. Il collaudo su 37 deck dice che il kit non sa rigiocare il 38% delle mosse: la proposta è rigiocare le linee vere dell'avversario (`docs/plans/piano-prossimi-passi.md`, P1).
- **Classificazione delle board** (#47): strumenti di misura pronti (D0) e prime correzioni della misura fatte (D2–D4: fermate a cascata, Evocazioni negate solo prima dell'atterraggio); sicure giuste 18 su 29 dopo il seguito delle risposte; il difetto principale resta la parità; mancano la protezione della board e le risorse per il turno dopo. Piano: `docs/plans/classificazione-board.md`.
- **Linee del video Invoked** (#45): la ricerca trova le board; la classifica va rivista insieme alla #47.

## Bug e limiti noti
- **Valori delle carte sul campo**: non dipendono dal contesto (mano vuota, equipaggiamenti); passa nella #47 (§30.6).
- **Carte piazzate** (ruoli, valori): gli equipaggiamenti non hanno bersaglio e i mostri dell'Extra Deck non hanno materiali (Apollousa a 0 ATK) (§30.4).
- **Ruoli**: la prova dell'estensore riparte da un turno nuovo (§30.4).
- **Board breaker**: riconosciuti dal testo; contro un deck scelto il motore prova quali si giocano dalla mano, ma quelli dell'Extra Deck o che arrivano con una combo restano fuori (§29.10, §30.10).
- **Chiave dei nodi**: bersagli e nomi dichiarati durante un effetto non ne fanno parte, quindi un'altra scelta dello stesso effetto non si esplora. Una correzione è stata provata e tolta perché peggiorava di poco la caccia sui 45 deck (§30.8).
- **Bench**: alcune mani (branded, mitsurugi) trovano il board migliore solo alla fine del budget, quindi oscillano a ogni cambio della ricerca (§30.6).
- **Deck lenti nel motore** (Elfnote Kewl Tune): una ricerca da 60 secondi ne dura 116; il resto viene da `confirm_best` e `zone_variants` (`docs/plans/velocita-ricerca.md`).

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
- Turno di prova contro un deck avversario a scelta (#44): livello 1 fatto (backend e interfaccia); livello 2 nella PR #35, da rifare con le linee vere; livello 3 dopo.
- Pagina «Il Banditore»: confronto tra banlist e impatto sui deck salvati.
- Simulatore passo per passo di una combo (#36, alla fine).

## Prossimi passi
Il piano con le priorità e il lavoro in parallelo è in `docs/plans/piano-prossimi-passi.md`. In breve:
1. Decisioni del proprietario: merge della PR #35, linee vere per il livello 2, deck lenti, strada della #47.
2. Livello 2 con le linee vere dell'avversario (Claude).
3. Deck lenti (sessione sorella) e Il Banditore (Gemini), in parallelo.
4. Classificazione delle board (#47: D10, D13) e pesi (#43), con le scelte del proprietario nel confronto della #26.
5. Mappa dei collegamenti (probabilità sature), simulatore (#36).
