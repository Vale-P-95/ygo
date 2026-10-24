# Monte Carlo guidato dal motore (idea 4, PIANO §7.4)

Thread «Guidare l'ottimizzatore con il motore», branch `claude/project-thread-ecxt66`, 24/10/2026. Non cambia la misura: la usa come obiettivo.

## Cosa c'è
- **`duelforge-cli engine-mc --deck <ydk>`**: pesca N mani da 5 carte (andando primo), ognuna la cerca il Combo Lab con un budget di nodi (2000 di base, un thread per mano, quattro mani alla volta) e la misura a gradini come `combos --staged`: misura economica dei primi 10 campi, misura completa dei 3 migliori per quella. Il valore della mano è il valore misurato (profilo «Bilanciato», `composite`) della board migliore. Scrive `out/engine-mc/summary.txt` (dieci righe) e `hands.json` (ogni mano con valore e carte usate).
- **Obiettivo**: P(board ≥ soglia) con l'intervallo al 95%, più il valore medio. Senza `--soglia` (`--threshold`) la soglia è la mediana del deck, poi fissa per ogni variante: è la quota che reagisce di più a un cambio. Una soglia fissa per tutti i deck non funziona: le scale dei deck sono diverse (Invoked mediana 1,43, Branded 0,71; con 1,5 Branded dà 0,4%).
- **`--swap "Out>In;..."`**: misura gli scambi di una copia sulle stesse mani. **`--climb N`**: l'ottimizzatore sceglie da solo fino a N scambi (Side Deck del `.ydk` e `--candidates` come carte che possono entrare, banlist di Master Duel). **`--roles`**: confronto con i ruoli automatici.
- Codice: `crates/ygo-solver/src/engine_mc.rs` (pesca, cache, stime), `climb` in `crates/ygo-stats/src/optimize.rs`, `crates/ygo-cli/src/engine_mc.rs` (il comando).

## Tecniche per spendere meno ricerche
1. **Numeri casuali comuni.** Le mani sono posizioni del Main Deck; una variante tiene ogni carta che resta al suo posto (`respell`). Cambiare una copia cambia solo le mani che contengono quella posizione (circa 5/40 = 12,5%), e la differenza tra due deck si misura sulle stesse mani: la varianza della differenza crolla. Su Invoked, uno scambio si legge a ±2,3-3 punti con 200 mani, contro i ±10 di due campioni indipendenti.
2. **Cache per le carte che contano.** La chiave di una mano sono le carte che la linea migliore ha tolto dalla mano, più le handtrap (contano tenute in mano); una carta già cercata in qualche mano e mai usata da nessuna linea migliore vale «vuota». Una carta mai cercata non è vuota. Quando una carta diventa viva le chiavi si fanno più fini e le mani che condividevano la chiave si cercano di nuovo, quindi il risultato non dipende dai thread. Una prima versione con «tolta dalla mano in una board qualsiasi» rendeva viva ogni carta (in qualche linea anche Ash Blossom va al Cimitero come costo) e non serviva a niente.
3. **Variabili di controllo.** La presenza di ogni carta nella mano ha probabilità esatta (ipergeometrica); lo stimatore di regressione corregge la quota del campione per quanto le carte del campione si scostano dalle loro probabilità, e l'intervallo viene dai residui più l'incertezza della regressione (`[(XᵀX)⁻¹]₀₀`). Un test con la verità nota controlla che l'intervallo copra la verità. Sui deck veri aiuta poco (Invoked ±6,7 → ±6,4 punti): la presenza delle carte, anche con le coppie o con un vicino più prossimo (prova fuori dal codice), spiega meno del 15% della varianza; il resto è come gioca la mano.

Il valore di una mano dipende anche dal resto del deck (cosa si cerca, cosa resta da pescare): la cache lo ignora, come fa il calcolo esatto dei ruoli.

## L'ottimizzatore (`climb`)
La ricottura di `optimize` fa migliaia di valutazioni: col motore sono giorni. `climb` prova uno scambio alla volta (una copia fuori, una dentro, la taglia non cambia), nell'ordine dato dal motore: fuori le carte che più di rado fanno una board sopra soglia quando sono in mano e la linea le usa, dentro prima le candidate, poi le carte del deck che lo fanno più spesso. Tiene il primo scambio la cui differenza appaiata ha l'estremo basso dell'intervallo sopra zero, e riparte dal deck nuovo.

Attenzione al confronto multiplo: con 8 scambi provati al 95% uno può passare per caso. Uno scambio tenuto per poco (Invoked: +2,4 ± 2,4) va riconfermato con più mani.

## Numeri (24/10, 4 core, seed 1)
| | Invoked (fixture) | Branded (MDM 24/10) |
|---|---:|---:|
| tempo per mano, un thread, 2000 nodi | 4,7 s | 9,0 s |
| 200 mani: P(board ≥ mediana) | 48,9% ± 6,6 punti | 50,4% ± 5,9 punti |
| intervallo di Wilson senza correzione | ±6,9 | ±6,9 |
| valore medio | 1,33 ± 0,07 | 0,71 ± 0,04 |
| scambi di una copia: ricerche evitate | 89-92% (15-21 mani cambiate su 200) | 87-92% (tranne una variante nuova: 2 e 1 mani) |
| `--climb 8` | +Harpie's Feather Duster −Crowley: 48,9 → 51,3% (+2,4 ± 2,4) | +Droll & Lock Bird −The Golden Swordsoul: 50,4 → 55,6% (+4,9 ± 3,0) |
| ruoli automatici: P(≥1 Starter) esatta | 91,9% | 12,5% (solo Albion) |
| board sopra mediana con / senza Starter | 54% / 10% | 44% / 51% |

- **Criterio del brief, intervallo sotto ±5 punti con 200 mani: non raggiunto** quando la quota è vicina al 50%. Non è raggiungibile da nessuno stimatore che usi solo 200 esiti indipendenti: il limite binomiale è ±6,9 a p = 0,5, e servono circa 385 mani (`--hands 400`; vedi sotto). Con una soglia che dà p sotto il 16% o sopra l'84% basta 200, ma sarebbe scegliere la soglia per il numero. Le differenze tra deck, che sono ciò che l'ottimizzatore usa, stanno già sotto ±5 con 200 mani grazie alle mani comuni.
- **Criterio della cache, almeno 50% delle ricerche evitate cambiando una copia: raggiunto** (87-92%).
- **Confronto con i ruoli**: su Invoked i ruoli e il motore concordano (con uno Starter il 54% delle mani supera la mediana, senza il 10%). Su Branded i ruoli automatici trovano un solo Starter e la quota con o senza è la stessa: lì i ruoli non dicono quanto gioca la mano, il Monte Carlo sì.

### Con 400 mani (stesse prime 200 mani)
| | Invoked | Branded |
|---|---:|---:|
| P(board ≥ mediana) | 49,9% **± 4,5 punti** (Wilson ±4,9) | 50,5% **± 3,8 punti** (Wilson ±4,9) |
| tempo | 481 s per 386 mani cercate | 953 s per 400 |
| scambio tenuto da `--climb` con 200 mani | −Crowley +Harpie's Feather Duster: **+1,1 ± 1,8, non confermato** | −The Golden Swordsoul +Droll & Lock Bird: **+4,8 ± 2,1, confermato** |
| ricerche evitate dallo scambio | 89% | 87% |

Con 400 mani l'intervallo sta sotto ±5 su entrambi, e le variabili di controllo stringono di più (Branded ±4,9 → ±3,8). Il secondo Droll piace alla misura perché un'handtrap tenuta in mano vale 0,9 nel profilo «Bilanciato»: è il giudizio della misura, non un consiglio di gioco.

## Prossimi passi
- **Nell'app (24/10, fatto in parte)**: scheda «Monte Carlo» del Laboratorio (`McTab.tsx`). Si sceglie il numero di mani (40, 100, 200) e, volendo, uno scambio (una carta del Main Deck fuori, una del Side dentro); il comando `mc_run` (`lib.rs`) chiama `ygo_solver::mc_run::run` e manda una `mc-progress` a ogni mano, `mc_cancel` ferma e restituisce le mani finite (numeri meno precisi). Mostra P(board ≥ mediana) con l'intervallo corretto e quello del campione, il valore medio, l'istogramma, la differenza dello scambio sulle stesse mani con il verdetto, e le mani dalla migliore con le carte usate dalla linea. Il seme è 1, i nodi 2000, come nella CLI.
- Resta: `--climb` nell'app (l'ottimizzatore sceglie gli scambi; serve spostare `rank_swaps` in `ygo-solver` e dare a `ygo-solver` la dipendenza da `ygo-stats`) e «Quante copie?» con l'obiettivo «board misurata».
- Andando secondo (6 carte) serve la misura dell'idea 5.
- Più mani per scambio solo dove serve: misurare prima le mani cambiate, e fermarsi quando l'intervallo della differenza esclude lo zero (test sequenziale).
