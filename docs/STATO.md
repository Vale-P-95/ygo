# Stato del progetto (22/10/2026)

Breve indice per chi arriva. Il dettaglio è in `docs/PIANO.md` (v1.10, §30–§31), le regole di lavoro in `AGENTS.md`, i lavori in corso in `docs/TASKS.md`.

## Fatto
| Area | Cosa c'è | PIANO |
|---|---|---|
| Dati | Carte, banlist (Master Duel, TCG, OCG), immagini scaricate a runtime, ricerca full-text con un linguaggio di filtri | §3, §17 |
| Fucina (deck builder) | Editor in stile Master Duel, Maybe board, mano di prova (5 ruoli, collegata al Circolo Arcano), barra dei ruoli e distribuzione della curva, import/export `.ydk`/`ydke://`, immagine del deck | §4, §18 |
| Bottega | Scaffale di deck box 3D personalizzabili, con collegamenti diretti al Laboratorio e al Circolo Arcano | §6, §19 |
| Laboratorio | Probabilità esatte per ruoli, Monte Carlo, ottimizzatore, registro partite, ruoli automatici, confronto tra deck; verificato contro riferimenti indipendenti (18 difetti corretti). Mappa dei collegamenti tra le carte letta dal motore: quasi in tempo reale, carte chiave, percorso più corto, confronto tra versioni del deck | §7, §21, §29.7, §30.7, §30.11 |
| Combo Lab (motore) | Ricerca automatica delle linee del primo turno con il motore EDOPro, obiettivi, ripresa; 12 difetti trovati con una caccia ai bug, 10 corretti | §9, §23–§25, §29, §30.6 |
| Combo Lab (punteggio) | Turno di prova contro un avversario con carte vere: interruzioni, handtrap, floodgate, resilienza (anche contro i breaker che i deck giocano davvero), turno dopo, risorse, costo, esposizione; risposta alla tua prima interruzione e protezione dei pezzi; profili e pesi; scheda del punteggio spiegata; la board #1 è sempre la migliore misurata. Contro un deck della Bottega: resilienza contro i suoi breaker, handtrap che fermano la linea, Maxx "C" e Mulcharmy; livello 2, l'avversario rigioca le sue linee vere | §26, §29.9, §30.10, §31.1, §31.2 |
| Combo Lab (interfaccia) | Confronto affiancato tra due board, pesi imparati dalle tue scelte e frontiera di Pareto, riproduttore passo-passo della linea con il campo a ogni passo, frecce Link, copia della guida | — |
| Il Banditore | Confronto delle liste Master Duel, TCG e OCG e impatto sui deck salvati; storico dei cambi di lista | §31.3 |
| App | Guida di benvenuto, palette dei comandi (`Ctrl+K`), scorciatoie, tema chiaro e scuro a 1280×800 | — |
| Qualità | CI per pull request (solo le parti toccate), ESLint dell'interfaccia, test di regressione della ricerca, bench su GitHub Actions, installer Windows su `main`; strumento di copertura delle meccaniche | §28, §30 |
| Velocità | Partite riusate, script caricati una volta per thread, mimalloc per Lua e per il C++: ricerca −37% con uscite identiche (PR #33) | `docs/plans/velocita-ricerca.md` |

## A metà
- **Classificazione delle board** (#47): fatti D0, D2–D4, D10, D13, D14 e D15; coppie sicure giuste 22 su 29, da confermare 16 su 46. D12 (esposizione come valore perso) alla corsia 4; poi D11, D16 e i pesi (D17), che aspettano i giudizi del proprietario. Piano: `docs/plans/classificazione-board.md`.
- **Interfaccia** (Gemini, `docs/TASKS.md`): testi di follow-up, risorse e costo; risposta alla prima interruzione e protezione dei pezzi; Banditore v2 e lista OCG.
- **Deck lenti**: misura delle istruzioni Lua fatta; il tetto per richiesta è alla corsia 2.
- **Linee del video Invoked** (#45): la ricerca trova le board; la classifica va rivista insieme alla #47.

## Bug e limiti noti
- **Carte piazzate**: gli effetti "se usato come materiale" delle carte materiale non si giocano. Equipaggiamenti, materiali e prova dell'estensore nello stesso turno sono corretti (PR #42, §30.4).
- **Board breaker**: riconosciuti dal testo; contro un deck scelto il motore prova quali si giocano dalla mano, ma quelli dell'Extra Deck o che arrivano con una combo restano fuori (§29.10, §30.10).
- **Chiave dei nodi**: bersagli e nomi dichiarati durante un effetto non ne fanno parte, quindi un'altra scelta dello stesso effetto non si esplora. Una correzione è stata provata e tolta perché peggiorava di poco la caccia sui 45 deck (§30.8).
- **Bench**: alcune mani (branded, mitsurugi) trovano il board migliore solo alla fine del budget, quindi oscillano a ogni cambio della ricerca (§30.6).
- **Deck lenti nel motore**: due richieste Lua lunghissime, ripagate a ogni replay, fanno quasi tutto il tempo della mano lenta; Lunalight impiega fino a 500 s nel turno di prova contro un deck (`docs/plans/velocita-ricerca.md`, §31.4).
- **Stima rapida contro misura**: 174 coppie su 1110 della top 8 hanno l'ordine invertito di più di 0,1. La board #1 è comunque la migliore misurata; per il resto serve una stima della board intera (§31.2).

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
- Turno di prova contro un deck avversario a scelta (#44): livelli 1 e 2 fatti; livello 3 dopo.
- Il valutatore imparato (rete neurale), più avanti: `docs/plans/valutatore-imparato.md`.
- Simulatore passo per passo di una combo (#36, alla fine).

## Prossimi passi
Il piano con le priorità e il lavoro in parallelo è in `docs/plans/piano-prossimi-passi.md`; i lavori in corso, corsia per corsia, in `docs/TASKS.md`. In breve:
1. Classificazione delle board: D12 (corsia 4), poi D11 e D16; i pesi (D17, #43) con i giudizi del proprietario.
2. Deck lenti: tetto di istruzioni Lua per richiesta (corsia 2).
3. Collaudo del livello 2 dopo le correzioni (corsia 3).
4. Interfaccia delle misure nuove (Gemini).
5. Mappa dei collegamenti (probabilità sature), simulatore (#36), valutatore imparato.
