# Prossimi passi del motore: cinque proposte

*Thread «Proporre i prossimi miglioramenti del motore», 24/10/2026, branch `claude/project-thread-5uicmc` da `main` 12d6f9d. Solo analisi, nessun codice e nessun calcolo nuovo. Fonti: `STATO.md`, `TASKS.md`, `tempi-motore.md`, `regole-motore.md`, `piano-motore-impatto.md`, `monotonia-budget.md`, `caccia-bug-motore.md`, `revisione-motore.md`, `ricerca-piano-critico.md`, `revisione-bibliografica.md` §4, la PR #173 e il codice di `probe.rs`, `search.rs`, `follow.rs` e `scripts/banco.py`.*

## In breve
- **Dove va il tempo:** il turno di prova (la misura delle board) è l'82% del tempo, la ricerca vera il 14% (`tempi-motore.md`). Le idee sulla ricerca da sola valgono poco sul tempo.
- **Dove va la qualità:** a 4000 nodi la ricerca arriva al 57-62% dell'oracolo (`piano-motore-impatto.md` §5), e la stima che la guida ordina le board con Spearman 0,57 contro la misura (`revisione-motore.md` §2). Le modifiche all'ordine con una costante uguale per tutti (B1a, B1c) sono fallite: il rumore è grande quanto l'effetto.
- **Cosa blocca:** la verità v1 del banco non c'è, quindi ogni PR della misura (classe M, come la #173) la decide Valerio a mano.
- **Consiglio:** i primi tre, in parallelo: non toccano gli stessi file caldi.

| # | Proposta | Cosa risolve | Guadagno atteso | Rischio | Classe | Taglia |
|---|---|---|---|---|---|---|
| 1 | Linee fortunate: ogni linea rigiocata con altri ordini del mazzo | Linee che riescono solo con una pescata fortunata, mostrate come sicure | Ogni linea con pescate o scavi dice «riesce in X su 8» | Basso | E (opzione spenta di base), poi M se entra nel punteggio | M, 15-20 $ |
| 2 | Misura a dimezzamenti fra le finaliste | Le 15 board finaliste misurate tutte per intero anche quando la #1 è già chiara | −15/−20% del tempo totale; più nodi a parità di tempo | Medio | R | M, 15-20 $ |
| 3 | Verità v1 del banco | Le PR della misura non si possono giudicare in automatico | Sblocca ogni modifica della classe M | Basso | 0 (script e fixture) | M, 10-15 $ più 1-2 ore di CPU |
| 4 | Sonda economica dentro la ricerca al posto dei giri di apprendimento | La bussola della ricerca (stima) è storta; i giri di apprendimento costano il 24% e alzano solo i valori delle carte | Il più grande sulla qualità: oracolo dal 57-62% verso il 70% a 4000 nodi | Alto | R | L, 25-35 $ |
| 5 | Giudice più preciso e più rapido per la ricerca | Esperimenti bocciati che costano ore; effetti piccoli nascosti dal rumore | Metà della CPU sulle decisioni chiare; si vedono effetti del 2-3% | Basso | 0 (banco v2) | M, 10-15 $ |

Le stime in dollari sono a occhio, sul consumo delle sessioni del 23-24/10 con compiti simili.

## 1. Linee fortunate: rigiocare ogni linea con altri ordini del mazzo
**Il difetto.** La ricerca gioca con un solo seme (`SearchConfig::seed`, di base 1): l'ordine del mazzo è fisso. Una linea che usa Pot of Extravagance, uno scavo o una pescata vede sempre le stesse carte, e l'app la mostra come se riuscisse sempre. È la priorità 8 di `revisione-motore.md` §7 e il PIANO lo chiede da sempre («una combo che dipende dalla fortuna riporta la sua probabilità di riuscita»), ma nel codice non c'è.

**La proposta.**
- Dopo la ricerca, le prime board si rigiocano con 8 semi diversi, con `follow.rs`, che ritrova ogni risposta da ciò che fa e non dal suo posto nella lista.
- Per ogni linea: in quanti semi arriva alla stessa board, o a una del valore almeno uguale. Le linee senza eventi casuali danno 8 su 8 senza rigiocare niente: basta sapere se la linea ha pescato o scavato.
- Prima solo da mostrare (scheda della board, `combos --seeds 8`); se pesarlo nel punteggio si decide dopo, con i numeri.

**Guadagno.** Nessuna linea fortunata mostrata come sicura. Il primo numero da contare: quante board #1 del banco hanno almeno un evento casuale.

**Rischio.** Basso: la ricerca e la misura non cambiano. Costo: un rigioco per seme e linea, pochi decimi di secondo con le copie del duello.

**Validazione.** Classe E con l'opzione spenta (`banco.py same`, 193 uscite uguali); se un giorno entra nel punteggio, classe M. Il contratto cambia (`lib.rs`, `api.ts`, `mock.ts`, fixture), quindi due file caldi.

## 2. Misura a dimezzamenti fra le finaliste
**Lo spreco.** Oggi la scala di B14 misura per intero le 15 migliori della misura economica: tutte le finestre, i breaker, il turno dopo. La misura delle finaliste è circa metà del tempo (45,6% nel profilo di `tempi-motore.md`), anche quando dopo poche finestre la #1 è già lontana dalle altre.

**La proposta.** Dimezzamenti successivi (Sequential Halving, Karnin et al. 2013; la stessa idea è alla radice di Gumbel MuZero, già in `revisione-bibliografica.md` G3.12):
- tutte le 15 ricevono un pezzo di misura in più (per esempio le finestre 3-8); si tengono le migliori 8;
- le 8 ricevono il pezzo dopo (finestre 9-12); si tengono 4;
- solo le ultime 4 hanno la misura completa, con i breaker e il turno dopo.
Ogni gradino riprende da quello prima, come già fa B14 (`measure_cheap` e `Partial`): nessuna corsa si rigioca. Le board escluse restano in lista con la misura che hanno, segnate come parziali.

**Guadagno.** Circa −30/−40% della misura delle finaliste, quindi −15/−20% del tempo totale. Il tempo risparmiato vale di più dei nodi: sul banco, passare da 2000 a 4000 nodi alza il valore medio del 10,7% con solo +17% di tempo (`regole-motore.md` §6). Rende anche meno cara la monotonia nel budget (`combos --checkpoints` costava +91%, tutto nei turni di prova).

**Rischio.** Medio: in qualche mano la #1 vera può uscire a un dimezzamento. Si tarano le dimensioni dei gradini sulle mani di taratura e si provano sui freschi.

**Validazione.** Classe R, `banco.py judge` sul set `full`; in più, la #1 uguale alla scala di oggi in almeno il 95% delle mani.

**Prova veloce nello stesso thread (facoltativa).** L'Extra Deck del kit ha 7 Flame Swordsman e 7 Gaia Saber (`GENERIC_EXTRA` in `probe.rs`), e il motore controlla l'Evocazione di ognuna a ogni azione dell'avversario: è la decisione più cara nel profilo (3,75 s su 12,2 s di `advance`). Con 2 + 2 copie, se `banco.py same` dà 193 uscite uguali è un guadagno gratis di classe E; se no si lascia, perché tocca la misura. Da coordinare con l'ottavo task della corsia 4, che lavora sull'Extra Deck del kit.

## 3. Verità v1 del banco
**Cosa blocca.** `regole-motore.md` §4 e §7: una PR della misura si giudica con le coppie di board nell'ordine giusto rispetto alla verità (il livello 2 contro i deck più giocati). La verità del banco congelato non è stata costruita, quindi la PR #173 e tutte le prossime di classe M (pesi, kit, D11, D16) non si possono unire in automatico.

**La proposta.** Come scritto nelle regole: le linee di 5 board per deck del banco (63 deck, 315 board) e il loro livello 2 contro i deck più giocati, con le loro linee congelate insieme, committati in `tests/fixtures/banco-motore/v1/verita/`; poi `banco.py judge --measure-cli` calcola le coppie e il test di McNemar.

**Guadagno.** Ogni modifica della misura ha un giudice automatico, in minuti invece che in ore. Si sa finalmente se la misura economica e quella completa si avvicinano alla verità.

**Rischio.** Basso: nessun codice del motore. L'unico punto delicato è scegliere gli avversari (6 come la tabella delle tech, non 3) e congelarne le linee, perché le liste di Master Duel Meta cambiano ogni giorno.

**Validazione.** Classe 0. Controlli: rimisurare la verità con la CLI di `main` dà le stesse coppie; la scala di oggi contro quella di prima del 24/10 rifà, sulle coppie, il verso del risultato dei pesi §7.

**Costo.** Il livello 2 costa circa 38 s a board prima delle copie e il 42% in meno con le copie: per 315 board e 6 avversari, circa 1-2 ore su 4 core, in background o con `bench.yml`.

## 4. Sonda economica dentro la ricerca
**Il difetto.** Ogni 1500 nodi la ricerca misura per intero le 2 board migliori non ancora misurate e alza i valori delle carte (`learn` in `search.rs`). Questi giri costano il 24% del tempo (11,5% durante, 13% alla fine) e correggono la bussola solo di lato. `revisione-motore.md` §2.2 e §8 li mette fra le cose da fermare; B1c ha mostrato che serve un segnale che venga dalla mano stessa, non una costante uguale per tutti.

**La proposta** (`revisione-motore.md` §7, priorità 3; è B10 riformulato):
- a ogni giro, la misura economica (2 finestre per risposta) su 10-20 board nuove, non la completa su 2;
- la differenza con segno fra misura e stima corregge la priorità delle risposte lasciate in coda da quelle linee;
- le board da misurare alla fine si scelgono anche da queste misure, non solo dalla stima.

**Guadagno.** Il più grande sulla qualità: l'obiettivo della revisione è Spearman fra stima e misura da 0,57 a 0,75 e la #1 vera fra le prime 15 nel 95% delle mani. Sul tempo, a pari CPU spesa nei giri.

**Rischio.** Alto: cambia il cuore di `search.rs` (file caldo) e l'ordine della coda, che a 4000 nodi è caotico. Meglio dopo la 5, che rende visibile un effetto piccolo. B11 (novità prima, Lipovetzky & Geffner 2017) resta l'alternativa da provare dopo, con la stessa misura.

**Validazione.** Classe R, `banco.py judge` sul set `full`, più l'oracolo a 4000 nodi con le fixture ricalcolate con la misura di oggi.

## 5. Giudice più preciso e più rapido per la ricerca
**Lo spreco.** Le modifiche della ricerca bocciate del 23-24/10 (B1a, B1c, B2, B3, punto 3 della caccia) sono costate ore di CPU ciascuna (B1c circa 4 ore), e una costante cambiata a caso sposta oltre l'1% fra 34 e 40 mani su 91. Inoltre ogni mano del banco è un processo nuovo che ricalcola i valori delle carte (3-4 s, il 14% del tempo dei banchi), uguali per le 3 mani dello stesso deck.

**La proposta** (banco `v2`, in una PR a parte come chiedono le regole):
- **più ordini del mazzo per mano** (per esempio 3 semi) e il valore di una mano come media: il rumore dell'ordine della coda scende senza mani nuove;
- **arresto anticipato** come in fishtest (test sequenziale, SPRT): le mani girano in ordine casuale e il giro si ferma quando il verdetto è certo; per la classe E, alla prima uscita diversa;
- **una CLI per deck invece che per mano**: i valori delle carte si calcolano una volta per deck e restano in memoria nello stesso processo, come già nell'app. Niente su disco, quindi la decisione di Valerio sui valori delle carte resta.

**Guadagno.** Circa metà della CPU sulle modifiche chiaramente buone o cattive; −9% del tempo di ogni giro per i valori delle carte; si vedono effetti del 2-3%, che oggi il rumore nasconde.

**Rischio.** Basso, ma le soglie vanno ritarate (come al §6 delle regole: `main` contro `main`, 2000 contro 4000 nodi) prima del primo verdetto.

**Validazione.** Classe 0: la taratura del §6 di `regole-motore.md` rifatta su `v2`.

## Già in corso o chiuse (non riproposte)
- Selezioni che girano (tecnica 3 di `tempi-motore.md`): PR #173 aperta, classe M, aspetta la decisione di Valerio o la verità v1.
- Deck lenti e tetto Lua (corsia 2), etichette nel turno di prova (decimo task), Extra Deck del kit contro A1b (ottavo task).
- Valori delle carte su disco: respinti da Valerio. L2-3 di default: respinto, resta a richiesta.
- B1a, B1c, B2, B2b, B3, tabella a penalità minima, punto 3 della caccia, kit concatenato, stima lineare: misurati e non tenuti.
- Monotonia con `--checkpoints`: c'è, spenta di base; diventa più economica con la 2.

## Brief dei primi tre
```
Compito: combos --seeds N: le prime board si rigiocano con N ordini del mazzo (follow.rs); per ogni linea «riesce in X su N», poi nella scheda della board
Perché: docs/plans/prossimi-passi-motore.md §1; revisione-motore.md §7 priorità 8; PIANO, eventi casuali
Branch: claude/lucky-lines, da main
File da toccare: crates/ygo-solver/src/follow.rs, crates/ygo-cli/src/main.rs; poi lib.rs, api.ts, mock.ts, fixture (caldi: lock in TASKS.md) e la scheda della board
Da leggere: docs/STATO.md, regole-motore.md, follow.rs
Verifica: cargo test; banco.py same sul set full con l'opzione spenta; conteggio delle #1 del banco con eventi casuali; pnpm typecheck e build; screenshot nei due temi
Criterio di accettazione: 193 uscite uguali senza --seeds; una linea senza eventi casuali dà N su N senza rigiocare; una con Pot of Extravagance dà meno di N su un deck di prova
PR: template, classe E; Modello: Opus 5.5 alto per follow.rs, Sonnet medio per l'interfaccia
```
```
Compito: la misura delle 15 finaliste a dimezzamenti (15 → 8 → 4), ogni gradino ripreso dal precedente con Partial
Perché: docs/plans/prossimi-passi-motore.md §2; tempi-motore.md (misura delle finaliste 45,6%)
Branch: claude/halving-measure, da main
File da toccare: crates/ygo-solver/src/probe.rs (caldo), crates/ygo-cli/src/main.rs (combos --staged), apps/desktop/src-tauri/src/lib.rs (combo_score; caldo)
Da leggere: docs/STATO.md, regole-motore.md, velocita-ricerca.md «B14»
Verifica: banco.py run --set full e judge, in background; #1 uguale alla scala di oggi; tempo new/old in summary.txt
Criterio di accettazione: ACCEPTED; #1 uguale in almeno il 95% delle mani; tempo almeno −15%
PR: template, classe R; Modello: Opus 5.5 alto
```
```
Compito: costruire la verità v1 del banco: linee di 5 board per deck e livello 2 contro 6 avversari congelati, in tests/fixtures/banco-motore/v1/verita/; McNemar in banco.py judge --measure-cli
Perché: docs/plans/prossimi-passi-motore.md §3; regole-motore.md §4 e §7
Branch: claude/banco-truth, da main
File da toccare: scripts/banco.py, scripts/objective_bench.py, tests/fixtures/banco-motore/v1/verita/
Da leggere: docs/STATO.md, regole-motore.md, banco-oggettivo.md
Verifica: il giro in background (1-2 ore su 4 core) con summary.txt; rimisurare con la CLI di main dà le stesse coppie; la PR #173 giudicata con la verità
Criterio di accettazione: 315 board con verità; judge --measure-cli dà un verdetto senza intervento; controlli verdi
PR: template, classe 0; Modello: Sonnet 5.5 medio (Opus alto per la scelta degli avversari)
```

## Fonti nuove
- Karnin, Koren, Somekh, *Almost Optimal Exploration in Multi-Armed Bandits*, ICML 2013 ([PDF](https://proceedings.mlr.press/v28/karnin13.html)): Sequential Halving, già citato in `ricerca-letteratura.md` n. 48.
- Fishtest, [FAQ](https://github.com/official-stockfish/fishtest/wiki/Fishtest-faq) e [SPRT su chessprogramming](https://chessprogramming.org/Sequential_Probability_Ratio_Test): test sequenziale per accettare le patch di Stockfish.
- Lipovetzky & Geffner 2017, best-first width search; seguiti in [Corrêa & Seipp, ICAPS 2025](https://ai.dmi.unibas.ch/papers/correa-seipp-icaps2025.pdf) e [Approximate Novelty Search](https://arxiv.org/pdf/2105.07691).
