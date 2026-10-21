# Velocità della ricerca delle combo

Lavoro della sessione Claude sul branch `claude/search-speed` (21/10/2026). Obiettivo: ricerca più veloce, risultati identici.

## Come si misura
- **Mani**: le 16 mani fisse del banco (`tests/fixtures/hands/bench.tsv`), 4000 nodi, un thread per mano, 4 mani alla volta su 4 core.
- **Deck**: liste di Master Duel Meta scaricate il 21/10 (`mdm_decks.py --limit 300`, per avere anche mitsurugi, yummy_mimighoul e ignister). La lista Invoked di oggi non ha Droll & Lock Bird, The Book of the Law e Super Polymerization, che servono alle mani `inv-*`: nella copia locale 2 Effect Veiler, Nibiru e Sorcerer of the Swamp sono sostituiti da queste. I numeri quindi non si confrontano con quelli del PIANO, solo prima/dopo qui.
- **Risultati invariati**: `combos --json --save-lines --threads 1 --nodes 4000 --top 10` per ogni mano; si confrontano riga di riepilogo (nodi, replay, duplicati, board), board misurate dal turno di prova e linee, togliendo solo i tempi. Due esecuzioni della stessa CLI danno uscite identiche; il rumore sui tempi è circa ±3%.

## Misura di partenza (`main`, 20281db)
| Misura | Valore |
|---|---|
| Tempo di ricerca, somma delle 16 mani | 254,6–262,5 s |
| Tempo totale della CLI (valori delle carte + ricerca + turno di prova delle prime 10 board) | 455–471 s |
| Partita vuota (solo gli script di base) | 2,7–2,9 ms |
| Partita con il deck fino alla prima scelta | 3,4–3,6 ms |
| Rigiocare una linea casuale (17,6 scelte in media) | 5,0 ms |

## Dove va il tempo
Profilo con callgrind (istruzioni eseguite) di `combos` su labrynth, 600 nodi, un thread, CLI con i simboli:
- **Creare le partite: circa il 50% di tutto.** Il motore EDOPro carica in ogni partita nuova gli script di base (`constant.lua`, `utility.lua` e i 22 file che questo carica, circa 500 KB di sorgente): 30% del totale. Poi lo script di ogni carta del deck (13-17%).
  - Leggere il bytecode Lua (`luaU_undump`, interning delle stringhe in `luaS_newlstr`) è da solo il 23%.
- **Giocare le scelte** nel motore (`OCG_DuelProcess`): 36%.
- **Distruggere la partita**: 5%.
- **Lato Rust** (stato, impronte, chiavi, coda): pochi punti percentuali (`Duel::state` 4,7%, `Duel::actions` 2,5%).
- **Chi crea le partite**: il turno di prova (`Prober::measure`) è l'85% di questo profilo breve: i valori delle carte prima della ricerca (56%) e l'apprendimento durante la ricerca (29%). Ogni misura di una board gioca da 20 a 60 partite (una per risposta provata e per ogni board breaker). Sulla ricerca da 4000 nodi i giri di apprendimento sono 3 (8 board misurate).

Quindi il guadagno grande sta nel non ripagare gli script di base a ogni partita: vale per la ricerca, il turno di prova e i valori delle carte insieme.

## Correzioni
(in corso)

## Idee scartate
(in corso)

## Limiti
(in corso)
