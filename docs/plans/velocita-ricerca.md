# Velocità della ricerca delle combo

Lavoro della sessione Claude sul branch `claude/search-speed` (21/10/2026). Obiettivo: ricerca più veloce, risultati identici.

## Come si misura
- **Mani**: le 16 mani fisse del banco (`tests/fixtures/hands/bench.tsv`), 4000 nodi, un thread per mano, 4 mani alla volta su 4 core.
- **Deck**: liste di Master Duel Meta scaricate il 21/10 (`mdm_decks.py --limit 300`, per avere anche mitsurugi, yummy_mimighoul e ignister). La lista Invoked di oggi non ha Droll & Lock Bird, The Book of the Law e Super Polymerization, che servono alle mani `inv-*`: nella copia locale 2 Effect Veiler, Nibiru e Sorcerer of the Swamp sono sostituiti da queste. I numeri quindi non si confrontano con quelli del PIANO, solo prima/dopo qui.
- **Risultati invariati**: `combos --json --save-lines --threads 1 --nodes 4000 --top 10` per ogni mano; si confrontano riga di riepilogo (nodi, replay, duplicati, board), board misurate dal turno di prova e linee, togliendo solo i tempi. Due esecuzioni della stessa CLI danno uscite identiche; il rumore sui tempi è circa ±3%.

## Coordinamento
- `probe.rs`: `Kit::standard` e `Kit::with_breakers` lasciati alla coordinatrice. I miei commit non li toccano, e il lavoro sulla velocità è finito.

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

### 2. Script delle carte caricati una volta per thread
**Cosa.** Prima di aggiungere le carte di una partita, gli script non ancora caricati si caricano e la fotografia si rifà. Le partite successive li tengono come sono appena caricati ed eseguono solo `initial_effect` di ogni carta.
- Se uno script, caricandosi, fa più che definire la sua carta (registra un effetto, crea una carta), quella partita non sarebbe come una nuova: si rifà nuova, e il thread smette di caricare in anticipo.

**Perché.** Con le partite riusate, rileggere il bytecode degli script delle carte restava circa un decimo del tempo.

### 3. Niente raccolta completa della memoria a ogni svuotamento
**Cosa.** Lo svuotamento non fa più la raccolta completa di Lua (upstream la fa in `duel::clear`): il collector arriva ai rifiuti della partita precedente man mano; i gruppi ancora vivi si svuotano come fa upstream.

**Perché.** La raccolta completa ripercorre ogni volta tutti gli oggetti degli script di base: −3% di istruzioni.

### 4. Turno di prova: solo le corse che servono
**Cosa.** I valori delle carte leggono il valore del campo, della mano e dei floodgate; l'apprendimento della ricerca legge solo il valore di ogni interruzione da sola. Ogni misura però giocava anche i 10 board breaker e la corsa con tutte le risposte insieme: circa un terzo delle corse. `Prober::measure_placed_unbroken` e `Prober::measure_alone` le saltano; `measure` e `measure_placed` (classifica, app, test) sono come prima.

**Altro, più piccolo.** Un passo del turno di prova confronta la mano e il Cimitero dell'avversario invece di leggere tutta la partita; le risposte della linea del tuo turno si danno senza costruire l'elenco delle azioni; un nodo della ricerca calcola l'impronta canonica una volta sola.

### Insieme (correzioni 1–4)
Le 16 mani danno uscite identiche; `DUELFORGE_CHECK_REUSE=1` (16 mani, 1500 nodi) non trova differenze; i test passano.

| Misura (16 mani, 4000 nodi) | Partenza | Dopo 1–4 |
|---|---|---|
| Tempo di ricerca | 254,6–262,5 s | 205,7 s (−20%) |
| Tempo totale della CLI | 455–471 s | 337 s (−27%) |

Il tempo totale scende più della ricerca perché i valori delle carte (prima della ricerca) e la misura delle prime 10 board sono quasi tutto turno di prova.

### 5. Si rimettono a posto solo le tabelle toccate
**Cosa.** Rimettere la fotografia ripercorreva ogni tabella degli script di base chiave per chiave: un quarto di ogni svuotamento. Ora di ogni tabella si tiene un'immagine grezza (parte array e parte hash, con il layout di Lua, `shim/table.cpp`) presa quando è stata rimessa a posto l'ultima volta. Una tabella con gli stessi byte non è stata toccata e si salta; le altre si rimettono come prima e si fotografano di nuovo.

**Tempo.** Uno svuotamento costa circa un quarto di prima (callgrind: da 2,3 a 0,5 milioni di istruzioni per la parte delle tabelle). Sulle 16 mani la ricerca passa da 205,7 a 196,5 s; uscite identiche, controllo con `DUELFORGE_CHECK_REUSE=1` senza differenze.

### 6. Anche gli oggetti C++ del motore con mimalloc
**Cosa.** Carte, effetti, gruppi e i contenitori del campo si allocano con `new` e `delete`: milioni di blocchi piccoli per ricerca. `shim/new.cpp` include le sostituzioni degli operatori globali che mimalloc stesso fornisce (supportate anche su Windows).

**Perché.** `malloc` e `free` della libreria C erano un ottavo delle istruzioni rimaste (callgrind).

**Tempo.** Una mano Labrynth da sola: da 7,6 a 6,7 s. Uscite identiche sulle 16 mani.

**Da verificare:** la build Windows (MSVC). mimalloc documenta queste sostituzioni anche per MSVC, ma qui non si può compilare per Windows; la CI compila l'installer solo su `main`.

### 7. Il limite di nodi lo controllano solo i worker
**Cosa.** Il thread che chiama la ricerca controllava tutti i limiti quattro volte al secondo, anche il budget di nodi. Se il controllo cadeva durante un'immersione oltre il budget, l'immersione si fermava lì; altrimenti arrivava in fondo. Ora quel thread controlla solo l'annullamento e il tempo; i nodi li controllano i worker dopo ogni lato, come già facevano.

**Perché.** Era la causa dei risultati non riproducibili su `main`: inv-r6 a 1500 nodi trovava 366 o 369 board invece di 371 circa una volta su sedici. Dopo la correzione 24 esecuzioni su 24 coincidono, e le 16 mani danno le uscite solite di `main`. Non è una correzione di velocità, ma senza di essa i confronti prima/dopo sbagliavano a caso.

### Insieme (correzioni 1–7)
Le 16 mani danno uscite identiche a `main` (riepilogo, board misurate dal turno di prova, linee); i test passano.

| Misura (16 mani, 4000 nodi, 4 mani alla volta) | Partenza | Dopo 1–7 |
|---|---|---|
| Tempo di ricerca | 254,6–262,5 s | 164,2 s (−36%) |
| Tempo totale della CLI (valori delle carte, ricerca, prime 10 board misurate) | 455–471 s | 269,2 s (−42%) |
| Con anche la correzione 8 (versione finale): ricerca / totale | | 158,9 s (−38%) / 260,6 s (−43%) |
| Istruzioni (callgrind, labrynth, 1000 nodi) | 44,7 miliardi | 27,1 miliardi dopo la correzione 4 |

Altre misure, `main` contro la versione finale:

| Misura | Prima | Dopo |
|---|---|---|
| `card-values` invoked / labrynth | 1,8 / 2,0 s | 0,7 / 0,7 s |
| `roles` invoked / labrynth (4 thread) | 15,2 / 14,6 s | 8,6 / 8,9 s |
| "Carta + resto", 6 mani in 30 s su 4 thread: nodi esplorati, invoked (Aleister the Invoker) | 48.298 | 62.063 (+28%) |
| Lo stesso, labrynth (carta 1225009) | 21.905 | 41.217 (+88%) |
| `combo_hunt.py` (82 deck, 2 mani, 3000 nodi): tempo di ricerca sommato | 2824 s | 1401 s (−50%) |

`card-values` e `roles` danno lo stesso elenco (a parte l'ordine tra valori uguali, che già cambiava da un'esecuzione all'altra). In `combo_hunt.py` le 164 righe sono identiche (fine della ricerca, nodi, replay, board, board migliore misurata) e i problemi restano 0. In "carta + resto" il tempo è fisso, quindi più nodi danno board diverse: media pesata delle migliori da 16,21 a 16,89 (invoked) e da 9,82 a 9,87 (labrynth).

### 8. Le immagini delle tabelle senza i byte che nessuno scrive
Memcheck vedeva il confronto delle immagini leggere i byte di riempimento di Lua (e il valore di uno slot nil o booleano), mai scritti. La risposta era giusta comunque (una differenza lì faceva solo rimettere a posto una tabella per niente), ma leggeva memoria non inizializzata. Ora un'immagine tiene i tag di ogni slot e il valore solo quando il tag ne ha uno. Memcheck su una ricerca Mitsurugi con 2 thread e il turno di prova: nessun errore (resta un falso allarme nell'avvio di mimalloc, uguale su `main`).

## Idee scartate
- **Garbage collector generazionale di Lua** dopo la fotografia (−1% di istruzioni in più). Il motore chiama un passo del collector a ogni gruppo creato quando ne esistono più di 2048; in modalità generazionale ogni passo è una raccolta intera, e una ricerca Invoked restava bloccata per mezz'ora.

- **mimalloc come allocatore globale di Rust** nella CLI (provato dalla sessione principale dopo la PR #33, 21/10). Uscite identiche, ma la ricerca sulle 16 mani passa da 192,5 a 207,7 s (+8%, più lenta in 14 mani su 16): con questo carico l'allocatore di sistema va meglio per gli oggetti di Rust. Tolto.

## Trovato per strada
- `combos --card "Aleister the Invoker"` rispondeva "not in the Main Deck" se il deck ha l'artwork alternativo (86120752). Corretto nella PR #34: si cerca la copia del deck con lo stesso codice canonico.

## Limiti
- **Con più thread** il guadagno è minore: Labrynth in 30 s su 4 thread fa +16% di nodi al secondo con il riuso (+29% su un thread). Ogni thread tiene la sua partita con gli script di base (qualche MB), e più memoria si contende la cache.
- **Il turno di prova resta la parte più cara:** in una ricerca da 4000 nodi gioca circa 37 partite per board misurata, di cui il 90% sono le corse "usa questa risposta a questa finestra" (fino a 12 finestre per risposta). Ognuna rigioca tutto il tuo turno (il 44% del tempo di una corsa) e il turno avversario. Il motore non sa copiare una partita, quindi non si può ripartire dal punto in cui le corse si separano.
- **Valgrind e i controlli `DUELFORGE_CHECK_REUSE` coprono le 16 mani del banco e 30 partite casuali del test.** Un deck con script che cambiano lo stato di base in modi non visti qui potrebbe comportarsi diversamente in una partita riusata: `DUELFORGE_FRESH_DUELS=1` torna alle partite nuove, `DUELFORGE_CHECK_REUSE=1` lo controlla.

## Dopo il merge di `main` (21/10, sera)
Conflitti in `probe.rs` risolti: il filtro dei board breaker ora avvolge `self.breakers(...)`, e il `Prelude::Own` di `main` resta com'era. Base rifatta con `origin/main` (4a786d0): le 16 mani danno uscite identiche. Ricerca da 266,7 a 168,4 s (−37%), tempo totale da 470,2 a 276,6 s (−41%). Fmt, clippy e test passano.

## Revisione della PR #33
Cinque punti corretti:
1. `Debug.ReloadFieldBegin` chiama `duel::clear` fuori da uno svuotamento: ora lì fa quello che fa upstream, invece di leggere un puntatore nullo.
2. Le funzioni di un effetto di base cambiate durante la partita si riconoscono confrontando il valore nel registro, non il numero del riferimento: `Set*` libera il vecchio e spesso riprende lo stesso numero.
3. La fotografia tiene i flag della partita: con flag diversi la partita non si riusa (alcuni script di base si caricano secondo i flag).
4. Alcuni riferimenti del registro si liberavano solo con `lua_close` (contatori di attività, limiti di catena, condizioni di `SetUniqueOnField`): dopo 1000 svuotamenti la partita si rifà nuova, così non crescono senza limite sui thread lunghi dell'app.
5. Le dichiarazioni `extern "C"` sono fuori dal namespace anonimo.

Controlli: le 16 mani danno uscite identiche a `main` (ricerca 154,4 s, totale 256,8 s); `DUELFORGE_CHECK_REUSE=1` sulle 16 mani senza differenze; test, fmt e clippy verdi.

## Conclusione
- **Guadagno totale** sulle 16 mani del banco (4000 nodi): ricerca −38% (da 254,6–262,5 a 158,9 s), tempo totale della CLI −43% (da 455–471 a 260,6 s), con uscite identiche a `main` (16 su 16). Su `combo_hunt.py` la ricerca dimezza; `roles` −42%, `card-values` −60%; su 4 thread "carta + resto" esplora dal 28% all'88% di nodi in più nello stesso tempo.
- **Da dove viene:** soprattutto dal non ricaricare gli script del motore a ogni partita (correzioni 1, 2, 5), poi dalle corse inutili del turno di prova (4) e dall'allocatore per il C++ (6).
- **Da verificare prima del merge:** la build Windows (correzione 6 e il resto dello shim C++), con la CI dell'installer.
- **Cosa resta da provare:**
  - mimalloc come allocatore globale di Rust (`#[global_allocator]` nella CLI e nell'app: file non miei): il lato Rust fa ancora circa un quarto delle allocazioni;
  - nel turno di prova, il 90% delle corse sono "usa questa risposta a questa finestra": se si trovasse un criterio esatto per riconoscere finestre equivalenti se ne giocherebbero meno;
  - la scalabilità su più thread (+16% su 4 thread contro +29% su uno): ogni thread tiene una partita con gli script di base, e la cache si satura;
  - il resto è il motore che gioca le scelte (circa due terzi del tempo), in `vendor/` che non si tocca.

## Un deck lento: Elfnote Kewl Tune (22/10/2026, notte)
Leggendo le mosse tipiche di Elfnote Kewl Tune con 24 mani (seme 7) sono serviti 463 secondi: le altre letture stanno sotto i 70.
- **La mano:** Kewl Tune Mix, Ash Blossom, Ghost Mourner, Kewl Tune Cue, Kewl Tune Reco; 1000 nodi, un thread, `--seconds 60`. La ricerca arriva a 1000 nodi in 50 secondi, ma finisce dopo 170.
- **Perché:** il motore è lento su questi stati (6-9 nodi al secondo contro circa 100). Lo stack del motore mostra `check_matching` che chiama se stesso decine di volte: gli effetti rapidi dei Kewl Tune Evocano Synchro e controllano ogni combinazione di materiali. Una sola richiesta del motore usa fino a 27 milioni di istruzioni Lua.
- **Dove va il tempo dopo il limite:**
  - il giro di apprendimento finale (4 board al turno di prova): circa 75 secondi, perché a ogni finestra di catena il motore riprova quelle combinazioni;
  - `confirm_best` (Fase Finale vera delle board migliori) e `zone_variants`: circa 45 secondi.
- **Correzione:** i giri di apprendimento hanno una scadenza, 10 secondi dopo il limite di tempo della ricerca (o dopo il loro inizio, se cominciano più tardi). Se una board non finisce entro la scadenza, la sua misura non si usa e il giro si ferma.
  - Sulla mano lenta: da 170 a 116 secondi, stessa board migliore (14,4).
  - Banco (16 mani, 4000 nodi): uscite identiche in tutto tranne i tempi. Sulle mani normali una board si misura in meno di un secondo e la scadenza non scatta.
  - `combo_hunt.py --seed 7` (37 deck, 74 mani): 73 identiche. In `radiant_typhoon_zoodiac-1` le board passano da 584 a 585, ma la build di prima rifatta tre volte dà sempre 585, e la nuova 24 volte su 24 (stesse linee). La differenza non si ripete e non viene dalla scadenza.
- **Resta:** `confirm_best` e `zone_variants` non guardano il limite di tempo. Tagliarli cambia le board confermate, quindi è da decidere. L'alternativa è un limite di istruzioni Lua per ogni richiesta del motore: si potrebbe mettere nello shim, ma cambierebbe le linee trovate su questi deck.
