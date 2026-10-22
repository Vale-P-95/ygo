# Banco oggettivo della forza delle board

*Scritto il 22/10/2026 da Claude Code (corsia 3, branch `claude/tools-qa`). Serve a tarare i pesi del punteggio (#47 D17, #43) sul risultato contro il meta, non sulle opinioni dei giudici.*

## Idea
La forza di una board si misura con quanto ferma il turno vero degli avversari del meta. Il turno di prova contro un deck, livello 2 (`probe-line --vs X --moves`), lo dà: l'avversario rigioca le sue linee migliori, e si conta quanta parte delle sue mosse fa ancora con tutte le tue risposte (`left`).

## Metodo
- **Script:** `scripts/objective_bench.py`.
  ```
  python3 scripts/mdm_decks.py --out $S/mdm
  python3 scripts/combo_hunt.py $S/mdm --hands 2 --top 8 --seed 1 --json $S/hunt
  python3 scripts/objective_bench.py $S/bench --decks $S/mdm --hunt $S/hunt --skip lunalight --jobs 3
  python3 scripts/objective_bench.py $S/bench --decks $S/mdm --hunt $S/hunt --skip-run    # solo le tabelle
  ```
  Il giro riparte da dove era: un'invocazione già fatta non si rifà.
- **Codice:** `main` con la #53 (`5df8595`, materiali dei Rituali per identità).
- **Deck:** le 41 liste di Master Duel Meta del 22/10 (`mdm_decks.py`, non committate).
- **Avversari e pesi del meta:**
  - Master Duel Meta dà la diffusione di ogni tipo di deck in `/api/v1/deck-types`, campo `popRank`: la quota dei top deck classificati, in percentuale. Il campo `tier` è vuoto per tutti.
  - I tipi con `popRank` sopra zero sono esattamente 20, e tutti hanno una lista tra le 41.
  - Il peso è `popRank` normalizzato. Lunalight è saltato: contro di lui una sola invocazione supera il limite di 300 s. Le medie si fanno sui pesi degli avversari misurati.

| Avversario | popRank | Peso |
|---|---|---|
| Elfnote | 14,1 | 20,0% |
| Dracotail | 6,16 | 8,8% |
| Invoked | 5,51 | 7,8% |
| DoomZ | 4,54 | 6,5% |
| Branded | 4,38 | 6,2% |
| Clown Crew | 3,73 | 5,3% |
| Elfnote Kewl Tune | 3,57 | 5,1% |
| Artmage | 3,08 | 4,4% |
| Phantom Knights | 2,76 | 3,9% |
| Sky Striker | 2,76 | 3,9% |
| Power Patron | 2,76 | 3,9% |
| Maliss | 2,27 | 3,2% |
| Resonators | 2,27 | 3,2% |
| Orcust | 2,11 | 3,0% |
| Lunalight (saltato) | 2,11 | 3,0% |
| Yummy | 1,94 | 2,8% |
| Blue-Eyes | 1,78 | 2,5% |
| Magistus Fairy Tail | 1,78 | 2,5% |
| Tearlaments | 1,46 | 2,1% |
| Vanquish Soul K9 | 1,30 | 1,8% |

- **Board:** per ognuno dei 41 deck, le prime 5 board con campi distinti (le zone non contano), nell'ordine stampato da `combo_hunt.py --hands 2 --top 8 --seed 1` (D14). Prima la mano 0, poi la 1. In tutto 205 board, in 43 file di linee.
- **Misure:** un'invocazione di `probe-line --vs <avversario> --moves --json` per ogni file di linee e ogni avversario, con tutte le sue board: le linee dell'avversario si leggono una volta. Limite 300 s per invocazione. Per ogni board e avversario:
  - **livello 2:** `left` (quota delle sue mosse fatte ancora con tutte le tue risposte), quota delle sue mosse fatte su campo vuoto (`made`), `floodgates stop`, `value`, linee lette;
  - **livello 1:** resilienza contro i suoi breaker, probabilità che una sua handtrap fermi la tua linea, breaker in mano;
  - **misure generiche della board**, dal JSON: campo, mano, floodgate, resilienza, `follow_up`, `countered`, esposizione. Il costo e le risorse non ci sono ancora nel JSON.
- **Coppie dubbie:** una coppia board-avversario con meno dell'80% delle mosse fatte su campo vuoto è dubbia (L4-6: le mosse della sua Fase Finale non si rigiocano). Resta nei risultati, ma esce dalla "verità".
- **Verità per board:** 1 − `left`, media pesata sugli avversari, senza le coppie dubbie. È la parte delle mosse del meta che la board ferma.
- **Divisione:** per deck (le board di un deck restano insieme), 70% taratura e 30% verifica, con il seme 47.

## Risultati
- I risultati parziali sono in `tests/fixtures/boards/objective-bench.json`: solo nomi, codici e numeri. Lo stato del giro è in `objective-bench.status`. Si aggiornano ogni 45 minuti durante il giro.
- *(Numeri finali: da completare a fine giro.)*
