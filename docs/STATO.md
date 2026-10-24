# Stato del progetto (24/10/2026)

Breve indice per chi arriva. Il dettaglio è in `docs/PIANO.md` (v1.11, §30–§32), le regole di lavoro in `AGENTS.md`, i lavori in corso in `docs/TASKS.md`.

## Fatto
| Area | Cosa c'è | PIANO |
|---|---|---|
| Dati | Carte, banlist (Master Duel, TCG, OCG), immagini scaricate a runtime, ricerca full-text con un linguaggio di filtri | §3, §17 |
| Fucina (deck builder) | Editor in stile Master Duel, Maybe board, mano di prova (5 ruoli, collegata al Circolo Arcano), barra dei ruoli e distribuzione della curva, import/export `.ydk`/`ydke://`, immagine del deck | §4, §18 |
| Bottega | Scaffale di deck box 3D personalizzabili, con collegamenti diretti al Laboratorio e al Circolo Arcano | §6, §19 |
| Laboratorio | Probabilità esatte per ruoli, Monte Carlo, ottimizzatore, registro partite, ruoli automatici, confronto tra deck; verificato contro riferimenti indipendenti (18 difetti corretti). Mappa dei collegamenti tra le carte letta dal motore: quasi in tempo reale, carte chiave, percorso più corto, confronto tra versioni del deck | §7, §21, §29.7, §30.7, §30.11 |
| Combo Lab (motore) | Ricerca automatica delle linee del primo turno con il motore EDOPro, obiettivi, ripresa; 12 difetti trovati con una caccia ai bug, 10 corretti; dei punti in coda, il 3 è misurato e non tenuto, il 5 e il 6 sono entrati (`caccia-bug-motore.md` §11) | §9, §23–§25, §29, §30.6 |
| Combo Lab (punteggio) | Turno di prova contro un avversario con carte vere: interruzioni, handtrap, floodgate, resilienza (anche contro i breaker che i deck giocano davvero), turno dopo, risorse, costo, esposizione; risposta alla tua prima interruzione e protezione dei pezzi; profili e pesi; scheda del punteggio spiegata; la board #1 è sempre la migliore misurata. Contro un deck della Bottega: resilienza contro i suoi breaker, handtrap che fermano la linea, Maxx "C" e Mulcharmy; livello 2, l'avversario rigioca le sue linee vere | §26, §29.9, §30.10, §31.1, §31.2 |
| Combo Lab (interfaccia) | Confronto affiancato tra due board, pesi imparati dalle tue scelte e frontiera di Pareto, riproduttore passo-passo della linea con il campo a ogni passo, frecce Link, copia della guida | — |
| Il Banditore | Confronto delle liste Master Duel, TCG e OCG e impatto sui deck salvati; storico dei cambi di lista | §31.3 |
| App | Guida di benvenuto, palette dei comandi (`Ctrl+K`), scorciatoie, tema chiaro e scuro a 1280×800 | — |
| Sistema di design | Galleria `/galleria` (solo in modo mock) e anteprime generate in `apps/desktop/design/` con `pnpm design`, pronte per `/design-sync` | `docs/plans/sistema-di-design.md` |
| Qualità | CI per pull request (solo le parti toccate), ESLint dell'interfaccia, test di regressione della ricerca, bench su GitHub Actions, installer Windows su `main`; strumento di copertura delle meccaniche | §28, §30 |
| Lavoro del 23/10 | Oracolo a gradini (#108); turno di prova senza etichette, −12% di tempo (#110); banco di generalità (#111); sweep di robustezza su 286 deck, 0 crash (#112); «Confronta con il meta» nell'interfaccia (#113); nuova base del banco oggettivo (#114); fasi proporzionali (#115); passo 4 dell'interfaccia (#116); taratura dei pesi §7 (#117) | `docs/TASKS.md` |
| Velocità | Partite riusate, script caricati una volta per thread, mimalloc per Lua e per il C++: ricerca −37% con uscite identiche (PR #33) | `docs/plans/velocita-ricerca.md` |

## A metà
- **Classificazione delle board** (#47): fatti D0, D2–D4, D10, D13, D14 e D15; coppie sicure giuste 22 su 29, da confermare 16 su 46. D12 (esposizione come valore perso) alla corsia 4; poi D11, D16 e i pesi (D17), che aspettano i giudizi del proprietario. Piano: `docs/plans/classificazione-board.md`.
- **Interfaccia**: passo 5 (Bottega) unito con la #118, l'ultima PR di Gemini; dal passo 6 in poi lo fa Claude Code con il mockup prima (`docs/plans/ui-revisione-2.md`).
- **Anteprima dell'app**: `record-fixtures` e il caricamento in `mock.ts` ci sono; la prima anteprima Artifact è pubblicata. Manca la registrazione dei dati veri (serve `cards.db`, nel cloud ygoprodeck è bloccata).
- **Pesi §7** (resilienza 0,6→0, floodgate 0,8→0,4): da confermare sul banco di generalità, poi da cambiare insieme in `bench.py`, CLI, `lib.rs` e `scoring.ts`.
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
- **Una sola famiglia di agenti dal 24/10/2026**: sessioni di Claude Code per motore, backend e interfaccia; mockup in Claude Design; decisioni nel Progetto Claude; un compito per sessione; il merge lo decide l'utente (§30.5, §32, `docs/plans/piano-ambiente-e-risorse.md`, `AGENTS.md`).
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
4. Interfaccia: primo `/design-sync` del sistema di design (lo lancia l'utente), poi dal passo 6 in poi con Claude Code (mockup in Claude Design, poi una sessione Code); B8 (le board `cheap: true` sono nella scheda del punteggio).
5. Mappa dei collegamenti (probabilità sature), simulatore (#36), valutatore imparato.
