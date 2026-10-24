# Motore: le modifiche con più beneficio per il costo

*Thread «Pianificare le modifiche più utili al motore», 24/10/2026, branch `claude/project-thread-ymnr67` da `main` 9f18298. Solo analisi; l'unico calcolo nuovo è il controllo di L2-3 sui deck freschi (§1, `scripts/research/l23_fresh.py`, nessuna run nuova). Fonti: `STATO.md`, `TASKS.md`, `ricerca-esperimenti.md` §1, §5, §6b, §6c, §7, `velocita-ricerca.md` («L2-3 come ultimo gradino», «Il campo che cambia con il budget», B13), `ricerca-piano-critico.md` (A1, A1b, B1a), `caccia-bug-motore.md` §11, `monotonia-budget.md`, `revisione-bibliografica-2.md` §0, `extra-deck-a1b.md`.*

## Decisione
1. **Prima la classifica, poi la velocità, poi la ricerca.** Il distacco più grande e più facile da chiudere sta nell'ordine finale: la misura mette nel giusto ordine il 68% delle coppie dei deck freschi, L2-3 il 94%. L'ordine dei lavori è 1 → 2 → 3 subito, 4 e 5 dopo la monotonia nel budget, che è in corso.
2. **1. L2-3 come ultimo gradino di default** sulle 4 migliori board: sui 20 deck freschi il #1 cambia in 10 mani su 39, 8 meglio e 0 peggio. **2. Il livello 2 dalle copie del duello**: le run col tuo script non usano ancora l'arena; il turno di prova con l'arena ha fatto −46%.
3. **3. Il floodgate della misura economica a 0,4**, come nel punteggio: oggi il taglio 30 → 15 sceglie con un peso che il punteggio ha già abbassato. Piccolo, quasi gratis.
4. **4. Il rappresentante del campo scelto dalla misura** (caccia ai bug, punto 1, seconda parte) e **5. B1c e poi B11** (penalità per rango, novità prima): aspettano la monotonia, che tocca gli stessi file e toglie il rumore ai loro test appaiati.
5. **Non ora:** ripianificare dopo una handtrap, i breaker dall'Extra Deck e le risposte non di negazione (B5b) non spostano il punteggio, perché la resilienza pesa 0 e le handtrap avversarie non ci entrano. B1a, B2, B2b, B3, la tabella a penalità minima, il kit concatenato, l'opzione (d) e la stima lineare sono chiusi dalle prove.

## Dove sono i distacchi oggi
| Distacco | Numero | Fonte |
|---|---|---|
| Ordine finale delle board | Coppie giuste: misura 68,5%, L2-3 93,8% (deck freschi, verità sugli altri 16 avversari); su A1b 80% contro 94% | §1 qui sotto, `ricerca-esperimenti.md` §6b |
| Ricerca contro l'oracolo | Quota mediana 73% a 4000 nodi, 79% a 8000, 83% a 16 000; 19 mani su 88 al 95% a 4000 | `ricerca-piano-critico.md`, A1 a gradini |
| Stabilità col budget | 27 mani su 88 scendono di oltre l'1% con più nodi; Labrynth −19,8% per il rappresentante | `monotonia-budget.md`, `caccia-bug-motore.md` §11 |
| Costo del livello 2 | Circa 38 s a board e 77-86 s per le linee dei tre deck; il banco di generalità 9,2 ore di CPU | `velocita-ricerca.md` «L2-3», `ricerca-esperimenti.md` §7 |

## Le cinque modifiche
| # | Modifica | Beneficio atteso | Costo (sessione, CPU) | Giudice | Quando |
|---|---|---|---|---|---|
| 1 | L2-3 come ultimo gradino di default | #1 migliore in circa 1 mano su 5, nessuna peggiore; coppie giuste dal 68% al 94% | 15-20 $; circa 3 ore di CPU | `l23_fresh.py` e `l23_final.py`, giro dal vivo sui freschi, prova equa | Subito |
| 2 | Livello 2 dalle copie del duello | L2-3 e banco oggettivo circa −40% di tempo, uscite identiche | 15-20 $; circa 1 ora | Uscite identiche, `DUELFORGE_COPY_CHECK`, tempi | Subito |
| 3 | Floodgate della misura economica 0,8 → 0,4 | Il taglio 30 → 15 segue il punteggio; meno #1 tagliati prima della misura completa | 5-8 $; circa 2 ore | `paired_test.py --staged` su taratura e freschi | Dopo la 1 (stesso `lib.rs`) |
| 4 | Rappresentante del campo scelto dalla misura economica | Recupera le varianti perse (Labrynth 1,28 → 1,59) | 20-25 $; 3-4 ore | `paired_test.py --staged`, oracolo, banco di generalità | Dopo la monotonia |
| 5 | B1c, poi B11: priorità per rango e novità | Stessa qualità con metà dei nodi (quota a 4000 dal 73% al 79%) | 15 $ (B1c), 25-30 $ (B11); 3-5 ore ciascuna | Oracolo, `paired_test.py`, banco di generalità | Dopo la monotonia |

Le stime in dollari sono a occhio, sul consumo delle sessioni del 23-24/10 con compiti simili.

## 1. L2-3 come ultimo gradino di default
**Evidenza.**
- Su A1b (82 mani, 19 avversari) con N = 4 il #1 cambia in 29 mani su 80: 12 meglio, 16 pari, 1 peggio; la verità del #1 sale da 0,590 a 0,605 (`velocita-ricerca.md`, «L2-3»).
- **Controllo nuovo, fuori campione.** I 20 deck freschi del banco di generalità, su cui nessun peso né L2-3 è stato tarato, hanno già le run del livello 2 contro tutti i 19 avversari (`tests/fixtures/boards/generality-fresh.json`). Con `python3 scripts/research/l23_fresh.py`:

  | N | mani col #1 cambiato | meglio / peggio (soglia 0,02) | verità del #1, misura → L2-3 |
  |---|---:|---:|---|
  | 3 | 9 su 39 | 7 / 0 | 0,562 → 0,576 |
  | **4** | **10 su 39** | **8 / 0** | **0,562 → 0,578** |
  | 5 | 15 su 39 | 10 / 1 | 0,562 → 0,582 |

  Coppie giuste, sulle 5 board di ogni mano: misura 122/178 (68,5%), L2-3 167/178 (93,8%). Senza i parenti dei tre deck nella verità (`--exclude elfnote_kewl_tune,magistus_fairy_tail`) 67,6% contro 92,6%, e il #1 resta 7 meglio, 0 peggio. Dove cambia, la verità del #1 sale in media di 0,061 (mediana 0,040), fino a +0,20 (memento-1).
- **Limite**: la verità è anch'essa il livello 2 contro altri deck, quindi L2-3 ne condivide il meccanismo (§6b). È anche la cura che `extra-deck-a1b.md` chiede per M1 e M2 (rimozioni su mostri appena evocati, prima grande negazione), senza linee salvate.

**Cosa costruire.** Oggi L2-3 c'è nella CLI (`combos --meta-final N`) e nell'app a richiesta (`combo_meta`, pulsante «Confronta con il meta»). Il passo è farlo partire da solo dopo la scala di B14, sulle 4 migliori, in sottofondo: la prima classifica si vede subito, poi le 4 si riordinano. Le linee dei tre deck restano in memoria per la sessione e non si salvano mai. Senza rete o senza linee che si rigiocano, la classifica resta quella della misura con un avviso in italiano.

**Decisione di Valerio** (prima di questo brief): automatico in sottofondo (consigliato) o a richiesta come oggi. Costa 2-4 minuti di CPU in più per valutazione la prima volta, circa 40 s a board dopo; con la 2 circa il 40% in meno.

**Brief.**
```
Compito: nell'app L2-3 parte da solo dopo la scala di B14 sulle 4 migliori board e le riordina in sottofondo; la prima classifica resta visibile subito.
Perché: docs/plans/piano-motore-impatto.md §1 (#1 migliore in 8 mani su 39 sui deck freschi, 0 peggiori).
Branch: claude/l23-default, da main
File da toccare: apps/desktop/src-tauri/src/lib.rs (caldo: lock in TASKS.md), apps/desktop/src/lib/api.ts, mock.ts, store combo.ts, pages/Combo.tsx
Da leggere: docs/STATO.md, questo piano §1, velocita-ricerca.md «L2-3 come ultimo gradino», combo-meta.md
Verifica: l23_fresh.py (numeri del §1); combo_hunt --hands 2 --seed 7 --staged --meta-final 4 sui 20 deck freschi in background (tempo, #1 cambiati); pnpm typecheck, pnpm build, screens.mjs nei due temi; app vera con pnpm tauri dev
Criterio di accettazione: dal vivo nessun #1 peggiore di oltre 0,02 sulla verità del banco; la prima classifica non aspetta L2-3; senza rete un avviso e la classifica della misura; caricamenti con id di richiesta
PR: template; tabella dei #1 e dei tempi, screenshot nei due temi
Modello: Opus 5.5, effort alto per lib.rs; Sonnet 5.5 medio per l'interfaccia
Quando hai finito: TASKS.md, STATO.md e questo piano aggiornati, PR aperta, fermati.
```

## 2. Il livello 2 dalle copie del duello
**Evidenza.** Con l'arena nel turno di prova le run dopo la prima della stessa linea rigiocano solo il turno dell'avversario: turni di prova 1213 → 654 s (−46%) su 80 mani, board identiche. Restano senza copie le run del livello 2 col tuo script (`Prelude::Own`, `--vs --moves`) e i valori delle carte piazzate (`TASKS.md`). Il livello 2 è il costo di L2-3 (circa 38 s a board), del banco oggettivo, della prova equa e del banco di generalità (9,2 ore di CPU l'ultimo giro): ogni prova futura costa meno.

**Cosa costruire.** In `probe.rs` e `matchup.rs` le run di `Prelude::Own` partono dalla copia del duello dove comincia il turno dell'avversario, come le altre run del prober. Prima un profilo di una run del livello 2, per sapere quanto è replay del tuo turno e quanto turno avversario: se il tuo turno è meno di un terzo, il guadagno è sotto il −40% e la PR lo dice.

**Brief.**
```
Compito: le run del livello 2 col tuo script (Prelude::Own, probe-line --vs --moves) partono dalla copia del duello come le altre run del turno di prova, con uscite identiche.
Perché: docs/plans/piano-motore-impatto.md §2 (L2-3 e i banchi pagano il replay del tuo turno a ogni run).
Branch: claude/l2-copies, da main
File da toccare: crates/ygo-solver/src/probe.rs (caldo: lock in TASKS.md), crates/ygo-solver/src/matchup.rs
Da leggere: docs/STATO.md, questo piano §2, velocita-ricerca.md «B13, PR 4», caccia-bug-motore.md §11
Verifica: profilo di una run prima di scrivere codice; probe-line --vs --moves su 32 board contro i 6 deck più giocati, main contro branch (JSON identici a parte i tempi), DUELFORGE_COPY_CHECK=1, a1b_fair_measure.py, cargo test + search_regression
Criterio di accettazione: uscite identiche; tempo del livello 2 per board almeno −30%; prova equa identica
PR: template; tabella dei tempi prima/dopo per deck
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md, velocita-ricerca.md e questo piano aggiornati, PR aperta, fermati.
```

## 3. Il floodgate della misura economica a 0,4
**Evidenza.** Il punteggio pesa il floodgate 0,4 dal 24/10; la misura economica (`cheap_composite` della CLI, `cheap_value` di `lib.rs`) lo pesa ancora 0,8 e decide quali 15 board su 30 arrivano alla misura completa. Sui deck freschi il floodgate segue la verità al contrario (ρ −0,39 e −0,13 nelle due metà, `ricerca-esperimenti.md` §7); in Warrior Combo board con floodgate 0,26 hanno verità 0 (`extra-deck-a1b.md`). La scala di B14 tiene il #1 in 82 mani su 82, ma con una sola board di margine (Paleozoic-0, la migliore è la 14ª per valore economico) e misurata con i pesi vecchi.

**Brief.**
```
Compito: la misura economica pesa il floodgate 0,4 come il punteggio (cheap_composite nella CLI, cheap_value in lib.rs), se la prova appaiata lo accetta.
Perché: docs/plans/piano-motore-impatto.md §3 (il taglio 30 → 15 sceglie con un peso già abbassato nel punteggio).
Branch: claude/cheap-floodgate, da main
File da toccare: crates/ygo-cli/src/main.rs (cheap_composite), apps/desktop/src-tauri/src/lib.rs (cheap_value; caldo: lock in TASKS.md)
Da leggere: docs/STATO.md, questo piano §3, ricerca-esperimenti.md §7, velocita-ricerca.md «B14, seguito», banco-generalita.md «Come si giudica»
Verifica: paired_test.py --staged a 4000 nodi su taratura e freschi (le due versioni giudicate col punteggio di oggi); posto della migliore board nella misura economica sulle 82 mani di B14 (--staged-full 0); cargo test; in background
Criterio di accettazione: meglio ≥ peggio con la regola del segno, nessuna mano oltre −5%, freschi non sotto di oltre 1 punto, margine del taglio non più stretto di oggi; altrimenti «misurato, non tenuto»
PR: template; tabella appaiata e posti della migliore prima/dopo
Modello: Sonnet 5.5, effort medio
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

## 4. Il rappresentante del campo scelto dalla misura economica
**Evidenza.** Le varianti di un campo (stesso `field_key`, mano, Cimitero e bandite diversi) sono rappresentate dalla prima che la ricerca trova. Su hunt-labrynth-0 la variante giusta vale 1,59 e quella scelta 1,28, anche a 8000 e 20 000 nodi (`caccia-bug-motore.md` §11); su sky_striker-1 cambia il rappresentante in 6 campi su 22 fra 1000 e 20 000 nodi. Il criterio fisso (più carte in mano, poi Cimitero) è stato respinto: 6 meglio, 3 peggio, peggiore −27,2%. Resta la strada indicata allora: misurare più varianti. La monotonia tiene in cache le misure per linea, quindi questo lavoro le riusa.

**Cosa costruire.** Per ciascuno dei 30 campi che vanno alla misura economica, fino a 3 varianti diverse per mano e Cimitero (le prime per stima); il rappresentante è la migliore per misura economica. Nell'app lo stesso per `look_key`, in una seconda PR.

**Brief** (dopo il merge della monotonia).
```
Compito: in combos --staged il rappresentante di ogni campo è la migliore di fino a 3 varianti per misura economica, con le misure in cache della monotonia.
Perché: docs/plans/piano-motore-impatto.md §4 (Labrynth 1,59 → 1,28 per la variante sbagliata; il criterio fisso è stato respinto).
Branch: claude/field-representative, da main dopo la monotonia
File da toccare: crates/ygo-cli/src/main.rs (combos), crates/ygo-solver/src/search.rs solo se servono le varianti (caldo: lock in TASKS.md)
Da leggere: docs/STATO.md, questo piano §4, caccia-bug-motore.md §1 e §11, velocita-ricerca.md «Il campo che cambia con il budget», monotonia-budget.md
Verifica: hunt-labrynth-0 a 4000 nodi; paired_test.py --staged a 4000 su taratura e freschi; cadute fra 500 e 8000 nodi (E-c2 della monotonia); a1b_fair_measure.py; cargo test + search_regression; in background
Criterio di accettazione: Labrynth torna a 1,59; meglio ≥ peggio, nessuna mano oltre −5%; cadute ancora 0; tempo della valutazione al massimo +30%
PR: template; tabella appaiata, cadute e tempi prima/dopo
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

## 5. B1c, poi B11: priorità per rango e novità prima
**Evidenza.** A 4000 nodi la ricerca arriva al 73% mediano dell'oracolo, e solo 19 mani su 88 al 95%; da 4000 a 8000 nodi si guadagnano 6 punti. Un quarto dei nodi non cambia nulla di visibile e un quarto è già visto (A2). B1a ha mostrato che l'ordine fra fratelli alla **stessa** priorità non conta (15 meglio, 9 peggio, p = 0,31): conta dare priorità diverse, cioè B1c (la penalità di deviazione cresce col rango in `dive_order`, al posto dello 0,6 uguale per tutte) e B11 (novità come chiave, stima come spareggio; idea 6 della seconda revisione). Il guadagno possibile è il più grande, ma tocca il cuore di `search.rs` e il suo test appaiato è rumoroso finché la monotonia non c'è (`monotonia-budget.md`).

**Ordine.** Prima B1c, che è una costante per rango e si giudica in un giorno; B11 dopo, con un brief scritto sui numeri di B1c.

**Brief di B1c** (dopo il merge della monotonia).
```
Compito: la penalità di deviazione in coda cresce col rango della risposta in dive_order (per esempio 0,4 · rango, tarata su taratura e provata sui freschi).
Perché: docs/plans/piano-motore-impatto.md §5 (quota mediana dell'oracolo 73% a 4000 nodi; B1a ha mostrato che serve una priorità diversa, non un ordine diverso).
Branch: claude/b1c-rank-penalty, da main dopo la monotonia
File da toccare: crates/ygo-solver/src/search.rs (caldo: lock in TASKS.md)
Da leggere: docs/STATO.md, questo piano §5, ricerca-piano-critico.md B1a e B1b-c, revisione-bibliografica-2.md §3.2
Verifica: contatore siblings di DUELFORGE_SEARCH_STATS; paired_test.py --staged a 4000 su taratura e freschi; oracle.py measure a 1000, 4000 e 16 000 nodi; banco di generalità (report --against); cargo test + search_regression; in background
Criterio di accettazione: quota mediana dell'oracolo a 4000 almeno +3 punti; meglio ≥ peggio, nessuna mano oltre −5%; freschi non sotto di oltre 1 punto, divario non oltre +2
PR: template; quote dell'oracolo e tabella appaiata prima/dopo
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

## Non ora
| Idea | Perché no adesso |
|---|---|
| Ripianificare dopo una handtrap (`resume()`, idea 5) | Le handtrap dell'avversario non entrano nel punteggio «Bilanciato» (campo 1, mano 0,9, floodgate 0,4, resilienza 0): cambierebbe solo `countered`, che si mostra. |
| Breaker dall'Extra Deck o arrivati con una combo; risposte non di negazione (B5b) | Lavorano sulla resilienza, che pesa 0 dal 24/10. Tornano se la resilienza riprende peso. |
| Kit con più Evocazioni dall'Extra Deck | Prototipo: la classe resta 18 coppie su 19, `search_regression` peggiora (`extra-deck-a1b.md`). La cura è la 1. |
| Kit concatenato, opzione (d) | Nessun interruttore migliora sulle 406 board; (d) peggiora la ricerca in tre prove su tre (§6c, §7). |
| B1a, B2, B2b, B3, tabella a penalità minima, punto 3 della caccia | Misurati e non tenuti il 23-24/10. |
| Stima lineare tarata (B9) | Fuori campione peggiora; servono caratteristiche che il motore non esporta (materiali Xyz, «una volta per turno» usato, carte tolte all'avversario). |

## Parallelo
Al massimo tre sessioni. Ora: la monotonia (in corso: `search.rs`, `combos`), la 1 (`lib.rs`, interfaccia) e la 2 (`probe.rs`, `matchup.rs`): file diversi. La 3 dopo la 1, perché tocca anche `lib.rs`. La 4 e la 5 dopo la monotonia, una alla volta su `search.rs`.
