# Monotonia nel budget: l'idea scelta dalla revisione bibliografica

*Thread «Scegliere l'idea più promettente dalla bibliografia», 24/10/2026, branch `claude/project-thread-8wknbr` da `main` 9b82704. Solo analisi, nessun codice. Fonti: `revisione-bibliografica-2.md` §0 e §4.3, `STATO.md`, `TASKS.md`, `ricerca-esperimenti.md`, `ricerca-piano-critico.md` (A1), `caccia-bug-motore.md`, `velocita-ricerca.md`.*

## Decisione
1. Si fa per prima l'idea 2 della seconda revisione: **ciò che è stato misurato a un budget resta misurato a ogni budget più grande, e la migliore board misurata non scende** (inclusione dell'insieme misurato, con l'annidamento verificato).
2. È l'unico difetto documentato che mostra all'utente un risultato peggiore quando spende di più: **27 mani su 88 scendono di oltre l'1%** passando a un budget più grande (A1 con l'oracolo a gradini). La cura vale per costruzione, non cambia la misura né l'ordine della ricerca, e toglie rumore ai test appaiati delle idee che vengono dopo.
3. Forma: punti di controllo a **500·2^k nodi** (valori assoluti, non frazioni del budget); a ognuno la ricerca annota le prime N board per stima; `combos --staged` misura l'unione tenendo in cache le misure; il risultato finale è la migliore misura completa di sempre.
4. Giudice: cadute fra 500, 1000, 2000, 4000 e 8000 nodi da 27 su 88 a 0 (un thread); `paired_test.py` a 4000 non peggiore; tempo in più al massimo +15%; prova equa sulle 406 board identica, perché la misura non cambia.
5. Dopo questa: l'idea 6 (novità e LTS, B11), poi la 5. Le idee 1, 4 e 7 sono chiuse dalle prove già fatte; la 3 è in gran parte superata da L2-3; la 8 è un test di regressione piccolo, da fare quando capita.

## Le otto idee contro lo stato del 24/10
| # | Idea (§0 della seconda revisione) | Cosa c'è già | Verdetto |
|---|---|---|---|
| 1 | Riaprire la tabella delle trasposizioni per penalità minore (NRR2) | Provata il 23/10 nella forma «riapri subito» (`velocita-ricerca.md`): 15-159 riarrivi su 3000 nodi, `paired_test` 3 meglio, 5 peggio, 171 pari (p = 0,73). La chiave con i bersagli (B3) è stata provata e tolta (PIANO §30.8). | **Chiusa.** NRR2 rimanda le stesse riaperture: con riarrivi così rari il guadagno atteso sta sotto la soglia del test. Si riapre solo se il contatore E-c3 li mostrerà frequenti dopo B11. |
| 2 | Monotonia nel budget: inclusione e annidamento | Dentro una ricerca la #1 è la migliore misurata (#50); le fasi dopo la ricerca crescono in proporzione; il rappresentante del campo con un criterio fisso è stato respinto (6 meglio, 3 peggio, −27%). L'inclusione **fra** budget non c'è: 27 cadute su 88. | **Scelta.** |
| 3 | Kit del turno di prova a doppio oracolo, 4-10 continuazioni | L2-3 (livello 2 contro le linee vere dei tre deck più giocati, 94% delle coppie, §6b) è la forma che la revisione stessa preferisce: continuazioni vere, non generate. | **Superata in gran parte.** Resta far crescere gli avversari oltre tre (E-p1), con B5e/f e il valutatore imparato. |
| 4 | Quale passo interrompere, calcolato dalla traccia | Il kit concatenato è stato provato sulle 406 board (§6c): nessuno dei sette interruttori migliora, PERUSE porta le coppie dal 79% al 73%. | **Chiusa** nella forma provata. Gli strati 2 e 3 (punto fisso, landmark) non hanno un caso che li giustifichi. |
| 5 | Ripianificare dopo una handtrap con `resume()` | Non fatta. `countered` (D15) è solo da mostrare. | **Terza.** Cambia la misura (prova equa) e costa una piccola ricerca per run. |
| 6 | Novità prima, stima come spareggio; coda per `η·g/π` | Non fatta (B11, B1c aperti). A2: 24% dei nodi senza cambi visibili, 26% già visti. | **Seconda.** Il potenziale più alto, ma tocca il cuore di `search.rs` e il suo test appaiato è rumoroso finché la 2 non c'è. |
| 7 | Stima lineare tarata sulla misura | Provata (§2 e §5 di `ricerca-esperimenti.md`): fuori campione le inversioni della top 8 salgono da 14,7% a 15,6-16,5%; servono caratteristiche che il motore oggi non esporta. | **Superata per ora.** Torna con quei dati (B9). |
| 8 | Turno illimitato: cicli, contatori, mazzo vuoto | Il budget di nodi e il tetto per linea ci sono; manca la fixture dell'articolo come test di terminazione. | **Piccola.** Non cambia le board: un test di regressione da fare quando capita. |

## Perché la 2 prima della 6
- **Il difetto è misurato, la cura è certa.** Con un thread i giri di apprendimento cadono agli stessi nodi (commento di `learn` in `search.rs`), quindi i primi 1000 nodi di una ricerca a 4000 sono quelli di una ricerca a 1000. Le cadute vengono dalle scelte fatte alla fine: le prime 10 per la misura completa, il giro di apprendimento finale, le 40 di `confirm_best`, il rappresentante del campo, i 30 → 15 di `combos --staged` e i 300 → 30 dell'oracolo (caccia ai bug, punto 1). Se ciò che si è misurato prima non si butta, la caduta sparisce per costruzione.
- **Il distacco dall'oracolo è grande e in parte già trovato.** A 4000 nodi la quota mediana dell'oracolo è il 73%; sky_striker-1 trova a 1000 nodi uno stato che a 200 000 non ritrova. Quante delle board perse sono state trovate e non misurate lo dirà E-c2: è il primo numero di questa sessione.
- **Costa poco e non tocca i file più caldi in profondità.** In `search.rs` serve solo un'annotazione ai punti di controllo, che non cambia la ricerca (stesse board, `search_regression` invariato); il lavoro sta in `combos` (`ygo-cli`). La 6 cambia l'ordine della coda e si giudica con un `paired_test` che oggi oscilla anche per questo difetto.

## Cosa costruire
- **Punti di controllo assoluti.** A 500, 1000, 2000, 4000… nodi (con un thread, al nodo esatto) la ricerca annota le linee delle prime N board per stima (N = `staged_cheap`, oggi 30 campi in `combos`, 300 nell'oracolo). I valori sono assoluti perché una ricerca a 4000 deve contenere tutto ciò che misura una a 2000, compreso il punto a 500.
- **Misure in cache.** `combos --staged` misura con la misura economica l'unione delle annotate e delle finali; ogni board già misurata non si rimisura. La misura completa va alle migliori `staged_full` per misura economica **di ogni punto di controllo**, così anche l'insieme della completa è annidato.
- **Rappresentante e varianti.** La cache è per linea, non per campo: un rappresentante misurato a un punto resta misurato anche se a fine ricerca il campo ne sceglie un altro.
- **Board provvisorie.** Un'annotata che a fine ricerca non è fra le 40 di `confirm_best` va portata alla End Phase prima della misura.
- **Più thread.** La garanzia vale con un thread (bench, `paired_test`, oracolo). Con più thread i punti di controllo restano utili, ma l'annidamento non è garantito (potature condivise, caccia ai bug punto 3): lo dice la PR.
- **L'app** (`lib.rs`, `look_key`) in una seconda PR, dopo questa, con la stessa funzione.

## L'esperimento che la giudica
1. **E-c1, annidamento, senza codice:** 88 mani dell'oracolo, un thread, `combos --json --top 5000` a 1000 e a 4000 nodi; bersaglio: le board trovate a 1000 sono tutte fra quelle trovate a 4000. Se non regge, si trova la causa prima di tutto il resto.
2. **E-c2, inclusione:** le stesse mani a 500, 1000, 2000, 4000 e 8000 nodi; si contano le cadute di oltre l'1% fra un budget e il successivo. Oggi 27 su 88; **bersaglio 0**. Si registra anche quante volte la migliore misurata a un budget non è nell'insieme misurato del budget dopo.
3. **`paired_test.py` a 4000 nodi** (179 mani, modalità `--staged`): peggiori non più delle migliori, nessuna mano oltre −5%.
4. **Costo:** secondi totali di `combos --staged` sulle 179 mani al massimo +15%.
5. **Prova equa sulle 406 board** (`scripts/research/a1b_fair_measure.py`): la misura non cambia, quindi coppie e numeri identici; si lancia per la regola del merge.

## Costo stimato
- **Sessione:** una, Opus 5.5 alto per il disegno e Sonnet 5.5 medio per gli script; circa 15-20 $ (stima, sotto i 30 $ per PR).
- **Calcolo:** E-c1 ed E-c2 circa 2-3 ore di CPU, `paired_test` circa 1 ora; tutto in background nel container, niente `bench.yml`.
- **Per l'utente:** al massimo +15% di tempo della misura a parità di nodi; nessun cambio nell'interfaccia in questa PR.

## Brief per la sessione
```
Compito: in `combos --staged` la migliore board misurata non scende mai con il budget (punti di controllo a 500·2^k nodi, misure in cache, finale = migliore misura completa di sempre).
Perché: docs/plans/monotonia-budget.md (27 mani su 88 scendono con più nodi).
Branch: claude/monotonia-budget, da main
File da toccare: crates/ygo-solver/src/search.rs (solo le annotazioni ai punti di controllo: lock in TASKS.md), crates/ygo-cli/src/main.rs (combos), scripts/oracle.py o uno script nuovo per contare le cadute
Da leggere: docs/STATO.md, docs/plans/monotonia-budget.md, revisione-bibliografica-2.md §4.3, caccia-bug-motore.md §1, ricerca-piano-critico.md «A1: il giro con la misura a gradini»
Verifica: E-c1 (prima di scrivere codice), E-c2 a 500-8000 nodi, paired_test.py a 4000 --staged, a1b_fair_measure.py, cargo test + search_regression; i calcoli lunghi in background
Criterio di accettazione: cadute 0 su 88 (un thread), paired_test non peggiore, nessuna mano oltre −5%, tempo al massimo +15%, prova equa identica
PR: template di .github/pull_request_template.md; tabella delle cadute e del tempo prima/dopo
Modello: Opus 5.5, effort alto (script e documenti a Sonnet 5.5 medio)
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

## Esito (thread «Rendere la ricerca monotona nel budget», 24/10, branch `claude/project-thread-g65a3w`)
**In breve:** la garanzia si ottiene (cadute 21 → 1 su 91, nessuna mano peggio di `main`), ma costa +91% di tempo a 4000 nodi, non +15%. Entra come opzione `combos --checkpoints`, spenta di base: senza l'opzione l'uscita è identica a `main`.

**Come è fatta.** `SearchConfig::checkpoints` (nodi) e `checkpoint_top` (board): fra due archi, dove si controlla il budget e prima del giro di apprendimento, la ricerca copia le sue migliori board in `SearchResult::checkpoints`. La ricerca non cambia; con un thread una ricerca più lunga prende le stesse copie per prime (test `checkpoints_change_nothing_and_a_longer_search_takes_the_same` in `search_regression`). `combos --checkpoints` mette i punti a 500·2^k nodi sotto il budget e al budget stesso, misura i campi di ogni copia come quelli della fine (`--checkpoint-cheap`, `--checkpoint-full`, 30/15) con una cache per linea, e tiene per ogni campo la misura migliore.

**Prova (E-c2).** 91 mani delle fixture dell'oracolo con le liste esatte della loro ricerca (`oracle.py decks`: 42 deck non erano più nelle liste del giorno), un thread, `oracle.py measure --staged 30,15`, budget 500-8000. Le mani a 4000 nodi sono le stesse di `paired_test.py` (banco più `combo_hunt --seed 7`), quindi la colonna «contro main a 4000» è il test appaiato.

| | cadute oltre l'1% | contro main a 4000 (meglio / peggio, p) | peggiore | secondi a 4000 | secondi a 8000 |
|---|---|---|---|---|---|
| `main`, `--staged` 30/15 | 21 su 91 | — | — | 1736 | 2227 |
| prima forma: la ricerca riparte a ogni punto (`resume`) | 0 | 23 / 18, p = 0,53 | −44% | 3159 (+82%) | 4152 |
| la stessa con 15/8 per punto | 0 | 18 / 25, p = 0,36 | −44% | 2379 (+37%) | 3174 |
| **istantanee, 30/15 per punto** | **1** (doomz-0 −10%) | **10 / 0, p = 0,002** | 0% | 3312 (+91%) | 4342 |
| istantanee, 15/8 alla fine e per punto | 4 | 7 / 8, p = 1 | −47% | 2499 (+44%) | 3337 |

- **Perché costa tanto.** Per non scendere, un budget deve misurare tutto ciò che misura ogni budget più piccolo. Le prime 30 di 500, 1000, 2000 e 4000 nodi si sovrappongono poco: a 4000 nodi i campi con la misura completa passano da 14,5 a 34 per mano. Il tempo in più è tutto nei turni di prova (950 → 2506 s); la ricerca resta uguale (428 → 434 s).
- **La prima forma** (`resume` a ogni punto, senza toccare `search.rs`) dà la garanzia esatta, ma le fasi dopo la ricerca girano a ogni punto e la ricerca prende un'altra strada: mani fino a −44% contro `main`. Scartata.
- **L'unica caduta delle istantanee** viene dalle fasi dopo la ricerca: a 4000 nodi la board migliore è una che solo il gioco della End Phase o le varianti di zona producono, e la ricerca a 8000 nodi copia le board di 4000 prima di quelle fasi.
- **Con lo stesso tempo conviene spendere nodi:** `main` a 8000 nodi (2227 s) vale quanto le istantanee a 4000 (3312 s): 18 mani meglio, 22 peggio.
- **Prova equa sulle 406 board:** identica per costruzione. La misura (`probe.rs`, `probe-line`) non cambia, e senza l'opzione `combos` dà le stesse board e le stesse misure di `main` (12 mani su 12 a 2000 nodi, obiettivo Invoked compreso).

**Seguito.**
- L'app (`lib.rs`) può avere la monotonia a costo quasi nullo dove conta per l'utente: tenere la migliore board misurata fra una ripresa e l'altra della stessa ricerca.
- `--checkpoints` serve ai test appaiati che devono confrontare budget diversi senza il rumore delle cadute, pagandone il tempo.
- Le idee dopo restano la 6 (novità e LTS, B11) e la 5.
