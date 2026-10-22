# Attrito avversario (B5a)

Corsia 4. Piano: B5a di `docs/plans/ricerca-piano-critico.md` e «Il caso Aeon» in `docs/plans/ricerca-letteratura.md`. Branch `claude/placed-cards`, da `main` bb6c443.

## Cosa cambia
1. **Il sì di Aeon.** Nessuna regola lo rifiutava. Con Sorath, Caliga e Raidjin nel Cimitero, la ricerca esplorava tutto e trovava anche le board con l'Extra Deck avversario bandito. Ma `quick_score` guardava solo il tuo lato: a stima pari (2,01) `rank()` preferiva la linea più corta, quella del «No».
2. **Extra Deck dell'avversario del kit: 15 carte.** Link Spider più due mostri senza effetto, Flame Swordsman e Gaia Saber, 7 copie ciascuno. Togliere una carta ora è un attrito piccolo, non un floodgate da 0,6.
   - Le copie servono a tenere poche le scelte: con 14 carte diverse un Aeon che bandisce fino a 3 carte fa 2319 board, con le copie 79.
3. **Risorse tolte all'avversario** (`Measure::taken`, `Measure::attrition`, `probe::attrition`). Sono le sue carte che il tuo turno manda al Cimitero, bandisce o prende, lette dagli eventi della run senza risposte. Ogni carta pesa secondo la pila da cui viene:

   | pila | peso |
   |---|---|
   | Extra Deck | 0,04 |
   | mano | 0,15 |
   | Deck | 0,01 |
   | campo | 0,3 |

   I pesi sono provvisori: li tara la coordinatrice in D17. `combos` e `probe-line` li stampano («taken from the opponent») e li scrivono nel JSON.
4. **Lo stesso termine in `quick_score`.** Le carte dell'avversario mancanti rispetto all'inizio del turno (`SearchConfig::opponent`), pila per pila, con gli stessi pesi, per `VALUE_WEIGHT`. Ora la board con il bando sta in cima: 2,61 contro 2,01. Lo controlla il test `crates/ygo-solver/tests/attrition.rs`.
5. **Scelte tra le carte dell'avversario rigiocate per identità** (`follow::line_picks`). Se la tua linea si gioca contro un avversario diverso da quello del kit (i breaker che portano carte, il livello 2), le carte avversarie che sceglie si ritrovano per nome e posto, non per posizione nella lista.
6. **Observer.**
   - Un mostro avversario preso sotto il tuo controllo vale almeno come distrutto; prima valeva come un cambio di posizione.
   - Un mostro mandato nella sua Zona Magie e Trappole vale come bandito; prima non contava.
7. **Un'etichetta italiana.** «Look at your dell'avversario, Extra Deck» diventa «Look at your opponent's Extra Deck»: si traducono solo i luoghi tra parentesi.

## Numeri
`main` bb6c443 (o l'equivalente 8f4d837, uguale nella misura) contro questo branch. Liste MDM del 22/10.

| | `main` | B5a |
|---|---|---|
| Banco `--nodes 4000`, somma best / first | 18,26 / 18,26 | 18,26 / 18,26 |
| Banco, mano peggiore | — | branded 1,16 → 1,15 (−0,9%) |
| `combo_hunt` (41 deck, una mano, 3000 nodi, top 8), somma della migliore | 57,48 | 57,98 |
| `combo_hunt`, mani peggiori / migliori di oltre 0,05 | — | 0 / 3 |
| Problemi | 0 | 0 |

Le tre mani che migliorano:
- gimmick_puppet: 1,18 → 1,34;
- vanquish_soul_k9: 1,06 → 1,30;
- white_forest_azamina: 2,52 → 2,62.

`kewl_tune`: le linee migliori tolgono 1–2 carte all'avversario (Extra Deck e Deck), attrito 0,04–0,05. I test completi sono verdi, compreso `search_regression`.

## Limiti
- I pesi per pila sono un punto di partenza (D17).
- L'avversario del kit non ha campo nel tuo turno: il peso «campo» serve per il livello 2 e per le run con un altro avversario.
- `look_key` non distingue ancora le carte tolte all'avversario: nell'app due board con lo stesso campo, una con il bando e una senza, restano una sola voce. Il cambio tocca `lib.rs` (la scelta delle board da misurare) e va concordato.
