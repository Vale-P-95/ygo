# Ordine della ricerca e stima rapida (D14 = B11)

Corsia 4, secondo task. Piano: B11, C5.7 e C5.8 di `docs/plans/classificazione-board.md`. Branch `claude/placed-cards`, ripartito da `main` (f61f05d).

## Cosa cambia
1. **Riordino misurato nella CLI.**
   - `combos --probe` misura le board elencate e le stampa dalla migliore misurata (composito Bilanciato, gli stessi pesi di `scripts/bench.py`), con accanto la stima e la posizione nella ricerca.
   - Il JSON e le linee salvate restano nell'ordine della ricerca, con la posizione stampata (`printed`): così le inversioni di `rank_report.py` confrontano ancora la ricerca con la misura.
   - L'app fa già lo stesso: misura le prime `SCORED_BOARDS` board (una per aspetto) e le ordina per punteggio misurato, con le non misurate dopo (`Combo.tsx`, `boards`).
2. **Bonus fissi solo per le carte non misurate.** In `quick_score` un mostro scoperto con un valore misurato vale 0,3 come corpo più il suo valore. Prima prendeva +2,0 (Extra Deck) o +1,0 (Main Deck) anche quando non ferma niente.
3. **Mano vuota.** Mostri e carte scoperte si misurano anche con la mano vuota (`MainNoHand`, `ExtraNoHand`, `FaceUpNoHand`), e `of_field` usa questi valori quando la mano è vuota. Red-Eyes Dark Dragoon e Invoked Mechaba passano da 1,0 a 0: pagano con una carta dalla mano. Nessun nome di carta nel codice.
4. **Seconda copia.** `SetSecond` è quello che una seconda copia settata aggiunge alla prima (la coppia misurata insieme, meno una da sola). Sostituisce il +1,0 fisso per i Set doppi. Per esempio Infinite Impermanence 0,24, Super Polymerization 0,04.
5. **Spot mancanti.** Mostri coperti (`FaceDown`), Scale Pendulum (`Pendulum`), Trappole Continue scoperte (`FaceUp`); le Magie Equipaggiamento c'erano già dalla #42. L'apprendimento alza lo spot in cui la carta era, e la variante a mano vuota quando la mano lo era.
6. **Look key con Cimitero usabile: saltato.**
   - `look_key` decide anche i raggruppamenti e le misure dell'app (`apps/desktop/src-tauri/src/lib.rs`, della corsia 1).
   - Nella CLI il punto 7 copre già il caso: le varianti dello stesso campo, con un altro Cimitero, si misurano e si mostra la migliore.
7. **Un campo per riga.**
   - La top N della CLI elenca ogni campo una volta (`search::field_key`: le carte sul campo, senza zone e senza mano).
   - Delle varianti (altra zona, altra mano, altro Cimitero) si misurano le prime 3 trovate e si mostra la migliore misurata.

## Numeri
41 deck MDM del 22/10 (senza gli 8 archetipi della cartella di studio, che qui non ci sono), una mano per deck con la regola "-lines", 3000 nodi, un thread, seme 1, prime 8 board misurate. `combo_hunt.py --lines-rule --json`, poi `rank_report.py`.

Il "#1 stampato non il migliore" si conta con la posizione stampata (`printed`). Su `main` la posizione stampata era quella della ricerca.

| | `main` | punti 1–3 | punti 1–5 | punti 1–5 e 7 |
|---|---|---|---|---|
| C5.7 coppie invertite di più di 0,1 | 223 su 1110 | 208 | 174 | 174 |
| C5.7 deck con il #1 stampato non il migliore | 19 su 41 | 0 | 0 | 0 |
| C5.8 deck con al massimo 2 campi distinti | 0 | 2 | 3 | 0 |
| campi distinti medi nella top 8 | 6,12 | 5,98 | 5,95 | 7,51 |
| somma del migliore misurato nella top 8 | 53,03 | 58,75 | 57,82 | 59,24 |
| invarianza (gruppi > 0,05) | 11 | 11 | 13 | 3 |
| misura per board, mediana / p95 | 0,77 / 2,68 s | 0,75 / 2,60 s | 0,82 / 3,05 s | 0,78 / 2,64 s |
| `combo_hunt.py` | 41 ok, 0 problemi | uguale | uguale | uguale |

**Banco** (`bench.py --nodes 4000`, 16 mani; `first` è ora la prima board stampata, quindi la migliore misurata):

| | `main` | punti 1–3 | punti 1–5 e 7 |
|---|---|---|---|
| somma `best` | 17,91 | 17,65 (−1,5%) | 18,07 (+0,9%) |
| somma `first` | 14,05 | 17,65 | 18,00 |

Le quattro mani `inv-*` che non partono sono le stesse prima e dopo (carte che la lista Invoked di oggi non ha).

## Cosa resta
**L'obiettivo C5.7 (≤ 60 inversioni) non è raggiunto: 174.**

Le inversioni rimaste sono quasi tutte tra board con stime vicine, a 0,1–0,5 l'una dall'altra. Per esempio snake_eye: 13,47 contro 13,26 di stima, 2,52 contro 2,72 misurato. Lì la misura vede cose che una stima fatta carta per carta non conta:
- la resilienza dopo i breaker;
- le carte che fermano qualcosa solo insieme;
- le domande che cambiano in base alla board.

Abbassarle ancora vuol dire una stima che guarda la board intera. È il valutatore imparato (`docs/plans/valutatore-imparato.md`), oppure si misura di più durante la ricerca.

**Il #1 stampato è sempre il migliore misurato** (prima non lo era in 19 deck su 41), e la top 8 ha più campi diversi.
