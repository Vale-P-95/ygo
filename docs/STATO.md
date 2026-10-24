# Stato del progetto (24/10/2026)

Breve indice per chi arriva. Il dettaglio è in `docs/PIANO.md` (v1.11, §30–§32), le regole di lavoro in `AGENTS.md`, i lavori in corso in `docs/TASKS.md`.

## Fatto
| Area | Cosa c'è | PIANO |
|---|---|---|
| Dati | Carte, banlist (Master Duel, TCG, OCG), immagini scaricate a runtime, ricerca full-text con un linguaggio di filtri | §3, §17 |
| Fucina (deck builder) | Editor in stile Master Duel con l'ispettore della carta come colonna a sinistra, Maybe board, mano di prova (5 ruoli, collegata al Circolo Arcano), barra dei ruoli e distribuzione della curva, import/export `.ydk`/`ydke://`, immagine del deck | §4, §18 |
| Bottega | Scaffale di deck box 3D personalizzabili, con collegamenti diretti al Laboratorio e al Circolo Arcano | §6, §19 |
| Laboratorio | Probabilità esatte per ruoli, Monte Carlo, ottimizzatore, registro partite, ruoli automatici, confronto tra deck; verificato contro riferimenti indipendenti (18 difetti corretti). Mappa dei collegamenti tra le carte letta dal motore: quasi in tempo reale, carte chiave, percorso più corto, confronto tra versioni del deck | §7, §21, §29.7, §30.7, §30.11 |
| Combo Lab (motore) | Ricerca automatica delle linee del primo turno con il motore EDOPro, obiettivi (con il posto di ogni carta, anche Cimitero, bandite, mano e carte coperte, duri o morbidi), ripresa; 12 difetti trovati con una caccia ai bug, 10 corretti; dei punti in coda, il 3 è misurato e non tenuto, il 5 e il 6 sono entrati (`caccia-bug-motore.md` §11). B1a (ordine dei fratelli nella coda) misurato il 24/10 e non tenuto: risultato neutro e una mano −42%; resta il contatore, l'ordinamento passa a B1b e B1c (`ricerca-piano-critico.md`) | §9, §23–§25, §29, §30.6 |
| Combo Lab (punteggio) | Turno di prova contro un avversario con carte vere: interruzioni, handtrap, floodgate, resilienza (anche contro i breaker che i deck giocano davvero), turno dopo, risorse, costo, esposizione; risposta alla tua prima interruzione e protezione dei pezzi; profili e pesi; scheda del punteggio spiegata; la board #1 è sempre la migliore misurata. Contro un deck della Bottega: resilienza contro i suoi breaker, handtrap che fermano la linea, Maxx "C" e Mulcharmy; livello 2, l'avversario rigioca le sue linee vere | §26, §29.9, §30.10, §31.1, §31.2 |
| Combo Lab (interfaccia) | Confronto affiancato tra due board, pesi imparati dalle tue scelte e frontiera di Pareto, riproduttore passo-passo della linea con il campo a ogni passo, frecce Link, copia della guida | — |
| Tech contro il meta | Laboratorio → «Contro il meta» e CLI `tech-table`: per ogni handtrap del Main e del Side, quanto toglie alle linee vere dei 6 deck più giocati di Master Duel Meta, rigiocate dal motore; non cambia la misura | `docs/plans/tech-table.md` |
| Il Banditore | Confronto delle liste Master Duel, TCG e OCG e impatto sui deck salvati; storico dei cambi di lista | §31.3 |
| App | Guida di benvenuto, palette dei comandi (`Ctrl+K`), scorciatoie, tema chiaro e scuro a 1280×800 | — |
| Sistema di design | Galleria `/galleria` (solo in modo mock) e anteprime generate in `apps/desktop/design/` con `pnpm design`, pronte per `/design-sync` | `docs/plans/sistema-di-design.md` |
| Qualità | CI per pull request (solo le parti toccate), ESLint dell'interfaccia, test di regressione della ricerca, bench su GitHub Actions, installer Windows su `main`; strumento di copertura delle meccaniche | §28, §30 |
| Lavoro del 23/10 | Oracolo a gradini (#108); turno di prova senza etichette, −12% di tempo (#110); banco di generalità (#111); sweep di robustezza su 286 deck, 0 crash (#112); «Confronta con il meta» nell'interfaccia (#113); nuova base del banco oggettivo (#114); fasi proporzionali (#115); passo 4 dell'interfaccia (#116); taratura dei pesi §7 (#117) | `docs/TASKS.md` |
| Velocità | Partite riusate, script caricati una volta per thread, mimalloc per Lua e per il C++: ricerca −37% con uscite identiche (PR #33). Arena di memoria e copie del duello (B13): la ricerca, le fasi dopo e ora anche il turno di prova e i giri di apprendimento partono da una copia invece di rigiocare la linea; turni di prova −46% e ricerca −23% su 80 mani, board identiche; il livello 2 contro un deck −42%, uscite identiche (24/10) | `docs/plans/velocita-ricerca.md` |

## A metà
- **Classificazione delle board** (#47): fatti D0, D2–D4, D10, D13, D14 e D15; coppie sicure giuste 22 su 29, da confermare 16 su 46. D12 (esposizione come valore perso) alla corsia 4; poi D11, D16 e i pesi (D17), che aspettano i giudizi del proprietario. Piano: `docs/plans/classificazione-board.md`.
- **Interfaccia**: passo 5 (Bottega) unito con la #118, l'ultima PR di Gemini; dal passo 6 in poi lo fa Claude Code con il mockup prima (`docs/plans/ui-revisione-2.md`).
- **Anteprima dell'app**: `record-fixtures` e il caricamento in `mock.ts` ci sono; la prima anteprima Artifact è pubblicata. `recorded.json` è registrato con i dati veri (24/10) e, dal 24/10 sera, ha anche le carte mostrate (460, senza testi), la legalità e la panoramica delle probabilità del deck Invoked, che nell'anteprima è il deck «Invoked (registrato)»; la ricerca di default del Deck builder mostra carte. Mancano i punteggi delle board (servono il motore e `fetch-data`).
- **Pesi §7** (resilienza 0,6→0, floodgate 0,8→0,4): confermati il 24/10 sui 20 deck freschi del banco di generalità, mai usati per tarare. Coppie giuste 80,4% → 86,0% (+40/−8, p < 0,001); con il turno dopo, come ordina l'app, 78,7% → 82,0% (+32/−14, p = 0,011). Cambiati insieme in `bench.py`, CLI, `lib.rs` e `scoring.ts`. Dal 24/10 anche la misura economica pesa il floodgate 0,4: #1 identico in 126 mani su 126, la migliore sale di posto in 5 mani e scende in 1 (`velocita-ricerca.md`, «B14, floodgate 0,4»).
- **Deck lenti**: misura delle istruzioni Lua fatta, tetto di 500 000 ingressi per richiesta. Il caso Vendread dello sweep di robustezza è risolto (24/10): la selezione dei materiali per gruppi degli script di base ha copie più veloci con le stesse risposte (`docs/plans/robustezza.md`).
- **Linee del video Invoked** (#45): la ricerca trova le board; la classifica va rivista insieme alla #47.

## Bug e limiti noti
- **Collaudo dell'app vera del 24/10** (Fucina, Collegamenti, Combo Lab con `combo_meta` e obiettivo ricco: tutto funziona con motore e dati reali). Difetti trovati: «usa 0.7000000000000001 carte» nella scheda del punteggio (corretto: `plural` arrotonda); incollando in «Importa» un testo senza carte nasceva un deck vuoto senza errore (corretto: errore nella finestra, anche quando tutti i passcode sono sconosciuti); un nodo della mappa dei Collegamenti mostrava il codice 86120751 al posto del nome (corretto: YGOPRODeck a volte mette per prima un'altra illustrazione, 86120752 per Aleister the Invoker, e `get_many` ora trova la carta anche dal passcode di un'illustrazione, sotto il passcode chiesto). Secondo giro, stesso giorno: Combo Lab nel tema scuro e `combo_meta` con le board scelte dal profilo (non «Bilanciato»): funzionano, screenshot in `docs/screens/combo-lab-real/`. Corretti quattro difetti piccoli: «Ricalcola meta» con un altro profilo riprendeva le board già misurate, perché la lista le mette prima (ora prende le 4 migliori per i pesi scelti); il pannello di destra apriva la migliore per la stima, che dopo la misura non era la prima della lista (ora segue la prima finché non si sceglie una board a mano); «1 carte»; punteggi della lista col punto invece della virgola. Per cliccare sotto Xvfb c'è `xdotool` (`docs/plans/piano-ambiente-e-risorse.md`).
- **Carte piazzate**: gli effetti "se usato come materiale" delle carte materiale non si giocano. Equipaggiamenti, materiali e prova dell'estensore nello stesso turno sono corretti (PR #42, §30.4).
- **Board breaker**: riconosciuti dal testo; contro un deck scelto il motore prova quali si giocano dalla mano, ma quelli dell'Extra Deck o che arrivano con una combo restano fuori (§29.10, §30.10).
- **Chiave dei nodi**: bersagli e nomi dichiarati durante un effetto non ne fanno parte, quindi un'altra scelta dello stesso effetto non si esplora. Una correzione è stata provata e tolta perché peggiorava di poco la caccia sui 45 deck (§30.8).
- **Bench**: alcune mani (branded, mitsurugi) trovano il board migliore solo alla fine del budget, quindi oscillano a ogni cambio della ricerca (§30.6).
- **Deck lenti nel motore**: due richieste Lua lunghissime, ripagate a ogni replay, fanno quasi tutto il tempo della mano lenta; Lunalight impiega fino a 500 s nel turno di prova contro un deck (`docs/plans/velocita-ricerca.md`, §31.4).
- **Monotonia nel budget**: con più nodi la migliore board misurata può scendere (21 mani su 91 dell'oracolo, `--staged`). `combos --checkpoints` la tiene ferma (1 su 91), ma costa +91% di tempo a 4000 nodi, quindi è spenta di base, per scelta di Valerio del 24/10 (`docs/plans/monotonia-budget.md`). Nell'app la monotonia c'è a costo zero fra una ripresa e l'altra: dopo «Continua» le board già misurate restano in lista con il loro punteggio.
- **Stima rapida contro misura**: 174 coppie su 1110 della top 8 hanno l'ordine invertito di più di 0,1. La board #1 è comunque la migliore misurata; per il resto serve una stima della board intera (§31.2).

## Decisioni importanti (il perché è nel PIANO)
- **Motore EDOPro** (AGPL) invece di un motore scritto da zero: applica già tutti i ruling e gli script delle carte (§8.1, §15).
- **Master Duel come primo formato**; nomi e testi delle carte in inglese, interfaccia in italiano (§0, §1).
- **Nessun asset Konami nel repository**: carte e immagini si scaricano (§15.2).
- **Ricerca con replay**: il motore non copia una partita in corso, quindi ogni nodo si raggiunge rigiocando la linea. Tutte le ottimizzazioni puntano a ridurre i replay o a renderli più economici (§23, §25, §29.4).
- **Punteggio misurato** con un turno di prova vero invece che stimato dal testo delle carte (§22, §26).
- **Correzioni sempre generali**, validate su molti deck, mai per nome di carta (§28, §29.3).
- **Una sola famiglia di agenti dal 24/10/2026**: sessioni di Claude Code per motore, backend e interfaccia; mockup in Claude Design; decisioni nel Progetto Claude; un compito per sessione; il merge lo fa la coordinatrice in automatico a CI verde, per il motore con la prova equa non peggiorata (decisione del 23/10; §30.5, §32, `docs/plans/piano-ambiente-e-risorse.md`, `AGENTS.md`).
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
4. Interfaccia: primo `/design-sync` del sistema di design (lo lancia l'utente), poi dal passo 6 in poi con Claude Code (mockup in Claude Design, poi una sessione Code); B8 (motore e contratto degli obiettivi ricchi fatti il 24/10: posto per carta, duri e morbidi; l'editor dell'obiettivo (posto per carta, obbligatoria o preferita) c'è in prima versione senza mockup, da rifinire col mockup; le board `cheap: true` sono nella scheda del punteggio).
5. Motore: le cinque modifiche con più beneficio per il costo, in ordine, in `docs/plans/piano-motore-impatto.md` (il livello 2 dalle copie è fatto, −42%; il floodgate della misura economica a 0,4 è fatto; L2-3 resta a richiesta; dopo la monotonia il rappresentante del campo e B1c).
6. Mappa dei collegamenti (probabilità sature), simulatore (#36), valutatore imparato.
