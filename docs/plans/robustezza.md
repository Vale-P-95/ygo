# Sweep di robustezza

*Corsia 8, 23/10/2026. Punto 3 dopo il banco di generalità (`revisione-motore.md`, §4.2). La domanda: la ricerca, il turno di prova e il rigioco delle linee si rompono su deck che nessuno ha scelto?*

## Cosa fa
- **Deck:** `duelforge-cli random-decks` scrive un deck per ogni archetipo del pool delle carte con almeno 10 carte del Main Deck (`CardPool::archetype_decks` in `crates/ygo-engine/src/sweep.rs`).
  - Composizione: le carte dell'archetipo ×3 fino a 40, carte a caso per riempire, i suoi mostri dell'Extra Deck e poi carte a caso fino a 15.
  - Lo stesso seme dà gli stessi deck. Nessun deck si sceglie per nome.
- **Per ogni deck, due mani** estratte come in `combo_hunt.py` (seme della mano `<deck>-<k>`, duelli con seme 7):
  - `combos --probe --save-lines`: la ricerca e il turno di prova sulle prime 5 board;
  - poi `probe-line` rigioca e rimisura ogni linea salvata, come fa il Combo Lab per la spiegazione e per il livello 2.
- **Poi `sweep`:** duelli casuali con risposte casuali, per le decisioni a cui non si può rispondere e le risposte che il motore rifiuta.
- **Ogni problema si scrive con il comando che lo riproduce**, in `problems.json`. I tipi:
  - crash (panic o errore);
  - linee interrotte dalla ricerca;
  - turno di prova fallito;
  - linea che non si rigioca;
  - decisione senza risposta, risposta rifiutata.

```
python3 scripts/robustness.py $S/rob --count 400 --hands 2 --nodes 2000 --duels 2000 --jobs 4
python3 scripts/robustness.py $S/rob --report --list
```
Le mani già fatte si tengono: un giro interrotto riprende da dove si era fermato.

## Risultati su `main` 98c3a2d
286 deck (tutti gli archetipi con 10 carte o più nel Main Deck), 572 mani, 2000 nodi a mano. Mediana 5,5 s a mano, massimo 115 s; in tutto 78 minuti di CPU, circa 20 minuti su 4 core.

| controllo | problemi |
|---|---:|
| crash (ricerca, turno di prova, rigioco) | 0 |
| turni di prova falliti | 0 |
| linee salvate che non si rigiocano | 0 |
| mani con linee interrotte | 2, dello stesso deck |
| `sweep`, 2000 duelli, 186 269 decisioni: senza risposta / rifiutate / errori del motore | 0 / 0 / 0 |
| errori degli script registrati dal motore | 0 |

**L'unico caso: Vendread** (deck `153-106-revendread_slayer`, mani 0 e 1).
- **Cosa succede:** 263 e 38 linee interrotte, tutte con lo stesso errore: «a card script ran too long (DUELFORGE_LUA_LIMIT)».
- **Non è un ciclo infinito.** Con `DUELFORGE_LUA_LIMIT=20000000` la mano 1 non ha più linee interrotte, ma la ricerca scende a 5 nodi al secondo e si ferma al limite di tempo.
- **Causa probabile:** è uno script lento. I Rituali Vendread (Vendread Reunion, Revendread Evolution) sommano i Livelli su molte combinazioni.
- **Il tetto fa il suo lavoro:** la linea si interrompe e la ricerca continua.
- **Da guardare** (velocità del motore, corsia 2):
  - quale richiesta conta così tanti ingressi in Lua;
  - se la scelta dei materiali con somma di Livelli si può dare al motore in un modo che non enumeri tutte le combinazioni.
- **Riproduzione:**
  ```
  duelforge-cli random-decks --count 400 --seed 1 --out $S/rob/decks
  duelforge-cli combos --deck $S/rob/decks/153-106-revendread_slayer.ydk --hand '76871889;2266498;30650147;76871889;7986397' --nodes 2000 --threads 1 --top 5 --seed 7 --probe
  ```

## Quando rifarlo
- Dopo ogni modifica a `crates/ygo-engine` (risposte, decisioni, replay, arena) e a quello che la ricerca o il turno di prova chiedono al motore.
- Un problema nuovo rispetto a `main` blocca la modifica.
- Il giro costa circa 20 minuti su 4 core.
- Con un seme diverso (`--seed`) si provano altri deck dello stesso archetipo.
