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

### 1. Partite riusate (`crates/ygo-engine/shim/reuse.cpp`)
**Cosa.** Una partita finita non si distrugge: il thread la tiene e la svuota per la partita successiva, con gli script di base già caricati. Così non si rileggono a ogni partita `constant.lua`, `utility.lua` e i 22 file che carica.
- **Fotografia** dopo gli script di base: ogni tabella raggiungibile dalle globali e gli upvalue di ogni funzione Lua; gli effetti creati dagli script di base (alcuni registrano effetti globali, come il rimescolamento della mano di `proc_workaround.lua`) e i contatori del campo.
- **Svuotamento**: tabelle e upvalue tornano come nella fotografia (le classi delle carte spariscono, quindi ogni script di carta si riesegue come in una partita nuova); carte, altri effetti, coroutine e campo si buttano; il campo nuovo prende le opzioni e il seme della partita successiva; gli effetti di base si registrano di nuovo come erano (anche quelli che si cancellano da soli, come quello di `EVENT_STARTUP`).
- **Motore non toccato**: `duel::clear`, che upstream non usa mai, si compila da una copia modificata di `duel.cpp` (come già `field.cpp` e `lauxlib.c`, vedi `build.rs`).
- `DUELFORGE_FRESH_DUELS=1` torna alle partite sempre nuove. `DUELFORGE_CHECK_REUSE=1` gioca ogni partita riusata anche in una partita nuova accanto e si ferma alla prima differenza (passo o stato): serve a controllare.

**Perché.** Gli script di base sono metà del costo di una partita corta, e ogni replay della ricerca, ogni corsa del turno di prova e ogni misura dei valori delle carte crea una partita.

**Risultati invariati.** Le 16 mani danno uscite identiche (riepilogo, board misurate, linee). Con `DUELFORGE_CHECK_REUSE=1` le 16 mani (1500 nodi, ricerca + valori delle carte + turno di prova) non trovano differenze. Nuovo test `crates/ygo-engine/tests/reuse.rs`: 30 partite casuali (entrambi i giocatori) giocate in partite riusate e poi in partite nuove, passo per passo; fallisce se si toglie la registrazione degli effetti di base. Valgrind (memcheck) pulito.
- Due difetti trovati così e corretti prima del commit: senza gli effetti di base la mano non si rimescolava dopo un effetto attivato dalla mano (Ame no Habakiri no Mitsurugi); senza visitare le funzioni di quegli effetti restava lo stato di una catena precedente (la tabella `player_table` di `proc_workaround.lua`).

**Tempo.** Istruzioni eseguite (callgrind, labrynth, 600 nodi): da 62,0 a 45,4 miliardi (−27%). Sulle 16 mani: ricerca da 254,6–262,5 s a 249,7 s, tempo totale da 455–471 s a 428 s. Una mano da sola (labrynth, 2000 nodi, un thread, senza altro sulla macchina): da 12,2–12,4 s a 10,2–10,6 s (−15%).

## Idee scartate
(in corso)

## Limiti
(in corso)
