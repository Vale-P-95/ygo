# Esposizione come valore perso (D12 = B6 V1)

Corsia 4, terzo task. Il piano è B6 di `docs/plans/classificazione-board.md`. Branch `claude/placed-cards`, ripartito da `main` (dbd7c58), con il merge di 5875e2a.

## Cosa cambia
1. **Punti critici con la posizione** (`explain::exposure_points`, `ChokePoint { kind, answer }`).
   - Per ogni punto critico c'è la risposta della linea data alla tua ultima scelta libera prima dell'evento. È lì che comincia quello che la handtrap ferma.
   - I tipi sono quattro:
     - `DeckTouch`: Ash Blossom;
     - `FieldEffect`: Effect Veiler, Infinite Impermanence, Ghost Ogre;
     - `FifthSummon`: Nibiru;
     - `SecondSearch`: Droll & Lock Bird.
   - I punti sono tanti quanti ne conta `Exposure::choke_points`.
2. **`Prelude::Cut(&Line, usize)`** in `probe.rs`. La linea si rigioca fino a quella risposta, esclusa. Alla tua prima scelta libera il turno finisce, poi si gioca la End Phase. Il taglio cade sempre su una `IdleCmd`: lo controlla il test.
3. **`Prober::quick_value`**: due run.
   - La prima senza risposte: dà le finestre e i floodgate.
   - La seconda con tutte le tue risposte usate quando sono offerte.
   - Il valore è la somma dell'effetto di ogni uso che ferma qualcosa (`Effect::value`), più il peso dei floodgate.
4. **`Prober::expected_loss`**: per ogni tipo, `loss_h` = valore pieno − il minimo di `quick_value` sui suoi tagli (l'avversario sceglie il punto peggiore per te).
   - Maxx "C" e Mulcharmy contano 0,1 per ogni Evocazione Speciale dopo la prima.
   - Si somma `E = Σ_h p_h · loss_h`.
   - I tagli uguali si misurano una volta sola.
5. **`p_h`**: la probabilità di almeno una copia in 6 carte su 40.
   - Le copie medie vengono dalle liste MDM con `scripts/handtrap_rates.py <cartella>`. Lo script ha l'elenco delle carte di ogni tipo, il motore no.
   - L'app non ha le liste e usa una **tabella di default** (`HandTrapRates::default`), che viene dalle 41 liste del 22/10:

     | tipo | copie medie | probabilità |
     |---|---|---|
     | Ash Blossom | 2,46 | 0,335 |
     | Veiler, Impermanence, Ghost Ogre | 2,71 | 0,363 |
     | Nibiru | 0,22 | 0,035 |
     | Droll & Lock Bird | 1,12 | 0,167 |
     | Maxx "C", Mulcharmy | 3,76 | 0,47 |

     Sulle 87 liste di `--limit 300` i valori sono quasi uguali (Ash 2,47 copie). Si rifà quando le liste cambiano.
6. **Nell'app** (`score_view`): `exposure = E`, in interruzioni.
   - `exposureDetail` tiene i conteggi di prima, più `chokePoints` e, per ogni tipo, la perdita e la probabilità (`losses`), e `maxx` con `maxxChance`. I campi nuovi sono opzionali (`ExposureLoss` in `api.ts`); il backend finto è aggiornato.
   - `E` si calcola per **tutte** le board misurate, non solo per le prime 8. Le board si ordinano tutte insieme: senza l'esposizione, una board dopo l'ottava salirebbe. Costa circa 0,1 s a board.
   - C'è una cache per `(mano, board)`, che resta valida anche quando si cambia il deck avversario.
7. **CLI**: `combos --probe` e `probe-line` stampano `expected loss to handtraps` e lo scrivono nel JSON (`expected_loss`).
8. **Per Gemini** (in TASKS):
   - i pesi di `scoring.ts`: oggi −0,04 per punto critico, da portare a −0,5 nel Bilanciato e −1,0 in "Linee sicure";
   - la scheda con le perdite per tipo.

   Finché i pesi non cambiano, l'esposizione pesa meno di prima.

## Numeri
`main` (dbd7c58) contro questo branch. Liste MDM del 22/10.

| | prima | dopo |
|---|---|---|
| Banco `--nodes 4000`, somma best / first | 18,26 / 18,18 | 18,26 / 18,18 (identico) |
| `combo_hunt.py` (41 deck, una mano, 3000 nodi, top 8) | 41 ok, 0 problemi | uguale |
| Tempo di `expected_loss` per board (320 board) | — | mediana 0,12 s, p95 0,45 s, max 1,24 s |
| Tagli per board | — | mediana 4 |
| `E` per board | — | mediana 0,91, p95 2,25, max 3,39 |

La ricerca non cambia: né `quick_score` né `value.rs` usano l'esposizione. Il budget di B6 (≤ 2 s per board) è rispettato.

Esempi (`combos --probe`, 2000 nodi):

| deck | punti critici | `E` |
|---|---|---|
| blue_eyes | 10 | 1,78 |
| tearlaments | 13–15 | 1,04–1,32 |
| invoked, mano con "Sword" | 5–6 | 0,14–0,86 |

Per le linee lunghe `E` non cresce in modo lineare con i punti critici: conta il taglio peggiore per tipo, non il loro numero.

**Coppie (`board_pairs.py`): non riproducibili qui.**
- Le coppie di `pairs.tsv` sono state fatte con le liste del 21/10. Con quelle del 22/10 le mani della regola "-lines" e le board sono altre: se ne ritrovano 3 su 83.
- Gli 8 archetipi `x_*` e elfnote_kewl_tune non ci sono o sono troppo lenti per `make`.
- Le 3 ritrovate danno lo stesso esito con e senza `E`.
- Per il confronto vero serve la cartella delle coppie del coordinatore: `board_pairs.py eval <pairs> --cli <questo branch>`, poi `scripts/exposure_pairs.py <pairs>`, che confronta gli esiti senza e con `E` (peso −0,5).

## Limiti (V1)
- È ottimista: la carta fermata non risulta spesa, e la linea si ferma al momento libero prima, senza cercare un'altra strada (quella è V2, D16).
- `quick_value` conta gli usi delle risposte nella run congiunta, non la copertura come la misura completa: serve per le differenze tra tagli della stessa linea, non come punteggio.
- Le probabilità sono di un mazzo da 40 carte che va secondo con 6 carte, mescolando le liste: non dipendono dal deck avversario scelto.
