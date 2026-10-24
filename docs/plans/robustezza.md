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

**L'unico caso: Vendread** (deck `153-106-revendread_slayer`, mani 0 e 1). Risolto il 24/10, sezione seguente.
- **Cosa succedeva:** 263 e 38 linee interrotte, tutte con lo stesso errore: «a card script ran too long (DUELFORGE_LUA_LIMIT)».
- **Non era un ciclo infinito.** Con `DUELFORGE_LUA_LIMIT=20000000` la mano 1 non aveva più linee interrotte, ma la ricerca scendeva a 5 nodi al secondo.
- **Riproduzione:**
  ```
  duelforge-cli random-decks --count 400 --seed 1 --out $S/rob/decks
  duelforge-cli combos --deck $S/rob/decks/153-106-revendread_slayer.ydk --hand '76871889;2266498;30650147;76871889;7986397' --nodes 2000 --threads 1 --top 5 --seed 7 --probe
  ```

## Il caso Vendread risolto (24/10/2026)
**Causa.** Revendread Evolution evoca per Rituale con la somma esatta dei Livelli, prendendo i materiali anche dal Deck (al massimo uno). Gli script di base fanno il controllo con `aux.SelectUnselectGroup` e `Ritual.Check`:
- `SelectUnselectGroup` prova i gruppi di materiali carta per carta e raggiunge ogni gruppo una volta per ogni ordine delle sue carte; per ogni carta provata entra in Lua dal motore (`Group.IsExists`);
- la condizione di Revendread Evolution («al massimo una carta dal Deck») non ferma mai la ricerca, quindi si provano tutti i gruppi con somma fino al Livello, tra una ventina di mostri Vendread nel Deck;
- `Ritual.Check` somma i Livelli con `Group.GetSum`, che entra in Lua una volta per carta di ogni gruppo.

Il controllo si ripete a ogni decisione con la carta in mano: richieste da 912 000 ingressi, 0,4 s l'una, oltre il tetto di 500 000.

**Correzione.** Due copie delle funzioni upstream, con le stesse risposte, caricate dopo gli script di base (`crates/ygo-engine/lua/`):
- `select_unselect.lua`: ogni gruppo si prova una volta per chiamata (si ricorda per gruppo se si può completare) e le carte si scorrono in Lua;
- `ritual_check.lua`: la somma dei Livelli in Lua, con il Livello di ogni carta chiesto una volta per controllo.

Perché le risposte sono le stesse: se un gruppo si può completare dipende solo dal gruppo (rescon riceve il gruppo e la carta appena aggiunta, che ne fa parte), e durante un controllo lo stato del duello non cambia. La memoria vale per una sola chiamata dall'esterno.

Le copie si caricano solo se la funzione upstream è ancora quella da cui sono state scritte (`lua/upstream/`, confronto del testo in `scripts.rs`): se CardScripts la cambia, la copia resta fuori e il test `the_lua_replacements_answer_as_upstream` lo segnala. `DUELFORGE_UPSTREAM_LUA=1` le toglie per i confronti. Il test confronta copie e originali su gruppi di mostri del Deck (12 condizioni, 6 coppie min/max, Rituale per somma esatta e maggiore); due errori introdotti apposta nelle copie li trova.

Nessuna regola per nome di carta: vale per ogni carta che usa queste due funzioni.

**Numeri** (stesso binario, con e senza `DUELFORGE_UPSTREAM_LUA=1`):

| Misura | Prima | Dopo |
|---|---|---|
| Mano Vendread 1 (`combos` sopra): linee interrotte | 38 | 0 |
| Stessa mano: tempo totale (ricerca e turno di prova) | 24,5 s | 14,2 s |
| Stessa mano senza tetto: richiesta più grande | 912 182 ingressi | 98 362 ingressi |
| Stessa mano senza tetto: richieste oltre 100 000 ingressi | 63 | 0 |
| Test appaiato (92 mani, 4000 nodi) | — | 0 meglio, 0 peggio, 92 pari, ACCEPTED; secondi di ricerca 516 → 529 |
| Misura delle 22 board delle fixture A1b (`a1b_remeasure.py`) | — | identica, tempi a parte |
| Misura delle 396 board della caccia A1b rifatta | — | identica (3 differenze di 2·10⁻¹⁶, presenti anche tra due giri uguali) |
| Sweep di robustezza (286 deck, 572 mani, 2000 duelli) | 1 caso (Vendread) | nessun problema |

**Prova equa.** La verità del livello 2 del giro A1b non cambia (costa circa 9 ore e non dipende da questa correzione); cambia solo la misura, e solo dove scattava il tetto. Quindi si è rifatta la caccia del giro A1b su `main` (`combo_hunt.py --nodes 4000 --top 8 --seed 7`, poi `a1b_setup.py`: 396 board in 80 file di linee) e si è misurata ogni board con lo stesso binario, con e senza `DUELFORGE_UPSTREAM_LUA=1` (`probe-line --json`, sola misura). Risultato: 393 board identiche; nelle altre 3 cambia solo l'ultima cifra di `countered` (0,9576271186440678 contro …676), che cambia allo stesso modo tra due giri dello stesso binario. La prova equa è quindi la stessa per costruzione.

## Quando rifarlo
- Dopo ogni modifica a `crates/ygo-engine` (risposte, decisioni, replay, arena) e a quello che la ricerca o il turno di prova chiedono al motore.
- Un problema nuovo rispetto a `main` blocca la modifica.
- Il giro costa circa 20 minuti su 4 core.
- Con un seme diverso (`--seed`) si provano altri deck dello stesso archetipo.
