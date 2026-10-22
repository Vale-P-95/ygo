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

## Deck lenti: la scadenza anche dopo la ricerca (22/10/2026, corsia 2)
**Misura.** `DUELFORGE_DEBUG_PHASES=1` stampa quando finiscono la ricerca, l'ultimo giro di apprendimento, `confirm_best` e `zone_variants`. `combo_hunt.py --seconds` sceglie il limite di tempo. Deck di Master Duel Meta del 22/10 (41 deck), seme 7, un thread, 4 giri insieme sui 4 core: i tempi sono gonfiati.
- **1000 nodi, limite 60 s, 6 mani per deck (246 ricerche):** nessuna arriva al limite, perché tutte finiscono i nodi prima. Dopo la ricerca, le fasi successive superano i 10 s solo in 4 ricerche (kaiju, elfnote, dracotail; al massimo 24 s).
- **Limite di tempo 20 s, senza limite di nodi, 2 mani per deck (82 ricerche):** è quello che fa l'app. Le fasi dopo la ricerca superano il limite di più di 10 s su 2 deck:

| Deck | Oltre il limite, prima | Ricerca | Apprendimento | `confirm_best` | `zone_variants` | Oltre il limite, dopo |
|---|---|---|---|---|---|---|
| elfnote_kewl_tune | 30,3 s | 20,1 s | 10,0 s | 5,3 s | 14,9 s | 10,1 s |
| elfnote | 17,6 s | 20,1 s | 10,0 s | 1,5 s | 6,0 s | 13,5 s (la ricerca stessa si è fermata a 23,5 s) |

Gli altri 39 deck sforano al massimo di 7,4 s (tearlaments, quasi tutto apprendimento). La mano lenta indicata dalla coordinatrice non si gioca con la lista Kewl Tune di oggi (manca Ghost Mourner): nella copia locale un Effect Veiler è sostituito da Ghost Mourner.

**Correzione.** `confirm_best` e `zone_variants` hanno la stessa scadenza dei giri di apprendimento: 10 s dopo il limite di tempo, o dopo la fine della ricerca se è finita più tardi. Una board che non si gioca fino alla fine resta stimata (`exact: false`). L'app la mostra già così: «≈» nella classifica e «Prima della End Phase» nel dettaglio, quindi non serve niente nell'interfaccia. Le varianti di zona di una board si possono perdere.

**Risultati.**
- **Mano lenta** (Kewl Tune Mix, Ash Blossom, Ghost Mourner, Kewl Tune Cue, Kewl Tune Reco; seme 7, 1000 nodi, limite 60 s, da sola sulla macchina):
  - prima: 163,8 s in tutto (ricerca 60,2 s, apprendimento fino a 70,4 s, `confirm_best` fino a 98,5 s, `zone_variants` fino a 162,4 s);
  - dopo: 72,0 s in tutto, con lo stesso punteggio delle prime tre board (12,2), che però restano stimate.
- **Banco** (16 mani, 4000 nodi, limite 900 s): uscite identiche alla versione di prima (ricerca 146,1 s prima, 141,8 s dopo).
- **`combo_hunt.py --seed 7`** (41 deck, 2 mani, 3000 nodi): 82 righe identiche su 82, nessun problema.
- fmt, clippy e test passano, compreso `search_regression`.

**Limiti.**
- La ricerca stessa può fermarsi qualche secondo dopo il limite: una richiesta del motore su questi deck dura fino a qualche secondo, e il limite si controlla tra una richiesta e l'altra.
- Il limite di istruzioni Lua per ogni richiesta del motore (piano P3, «in misura») non è stato provato.

## Istruzioni Lua per richiesta del motore (22/10/2026, corsia 2)
**Misura.** Con `DUELFORGE_LUA_COUNT=<file>` uno shim (`crates/ygo-engine/shim/count.cpp`) mette un hook di conteggio sullo stato Lua del duello; le coroutine del motore lo ereditano. Per ogni richiesta (da una risposta alla decisione dopo) si salvano le istruzioni, in migliaia, e il tempo. Senza la variabile nessun hook. `combo_hunt.py --lua-count <cartella>` salva un file per giro e `scripts/lua_count.py` li legge: per soglia, le richieste sopra, il loro tempo e il massimo che un tetto lì farebbe risparmiare. Con l'hook la mano lenta dura 78,1 s invece di 76,7 s.

**Giro a 20 s** (41 deck × 2 mani, seme 7, senza limite di nodi, 4 giri insieme come per la scadenza): 25,9 milioni di richieste.
- La richiesta più lunga fa 2,96 milioni di istruzioni (elfnote_kewl_tune, mano 1); nessuna arriva a 3 milioni. Il 99,9% delle richieste di ogni giro sta sotto 1,4 milioni.
- Sforano di più di 10 s elfnote (14,3 s), dracotail (10,7 s) ed elfnote_kewl_tune (10,1 s). Sono i 10 s di grazia dell'apprendimento più qualche secondo di ricerca oltre il limite, non richieste Lua lunghe: le loro richieste più lunghe fanno 795 mila, 108 mila e 2,96 milioni di istruzioni.
- Per questi deck nessuna soglia sicura toglie i secondi oltre il limite: sotto 3 milioni si fermerebbero anche richieste normali (61 sopra 2 milioni, 992 sopra 100 mila nei 82 giri), e tutte le richieste sopra 1 milione valgono al massimo 3,3 s nei tre giri lenti.

**Mano lenta** (Kewl Tune Mix, Ash Blossom, Ghost Mourner, Kewl Tune Cue, Kewl Tune Reco; seme 7, 1000 nodi, limite 60 s, da sola): 42 712 richieste in 76,8 s di motore, ricerca finita a 64,1 s, 75,4 s in tutto.
- 25 richieste sopra 10 milioni di istruzioni valgono 69,8 s: 13 da 104-125 milioni (4,1-5,5 s l'una) e 12 da 13-19 milioni (circa 0,8 s l'una).
- Sono sempre le stesse due richieste, una da 18,7 milioni e una da 104,4 milioni, ripetute ogni 5-6 s: ogni volta che la ricerca rigioca quella linea le paga di nuovo. Così la ricerca fa solo 116 nodi.

| Soglia (istruzioni) | Richieste sopra nei 82 giri | Richieste sopra nella mano lenta | Tempo loro | Risparmio massimo |
|---|---|---|---|---|
| 2 milioni | 61 | 48 | 75,0 s | 70,8 s |
| 3 milioni | 0 | 47 | 74,9 s | 68,7 s |
| 5 milioni | 0 | 36 | 72,9 s | 65,1 s |
| 10 milioni | 0 | 25 | 69,8 s | 58,9 s |
| 20 milioni | 0 | 13 | 60,1 s | 48,9 s |
| 50 milioni | 0 | 13 | 60,1 s | 32,1 s |

Il risparmio massimo suppone che il tempo di una richiesta cresca con le istruzioni: un tetto la ferma alla soglia.

**Senza la variabile niente cambia** (stesso binario, confrontato con `main`):
- banco: 16 mani su 16 identiche;
- `combo_hunt.py --seed 7`: 82 righe su 82 identiche, nessun problema.

**Raccomandazione.**
- Un tetto di **10 milioni di istruzioni per richiesta**. Nessuna delle 25,9 milioni di richieste normali ci arriva (la più lunga fa 2,96 milioni, margine di 3,4 volte). Sulla mano lenta toglie fino a 59 s dei 77 s di motore.
- Per gli sforamenti di elfnote, dracotail ed elfnote_kewl_tune nel giro a 20 s il tetto non serve: lì pesano la grazia dell'apprendimento e la ricerca che controlla il limite solo tra una richiesta e l'altra.
- Da fare nella PR del tetto, con banco e `combo_hunt --seed 7`:
  - una richiesta fermata rende la linea interrotta, che si scarta come le altre;
  - il duello non si riusa, perché lo stato Lua resta a metà;
  - meglio ancora ricordare il prefisso di linea che ha superato il tetto e non rigiocarlo, perché la stessa richiesta torna a ogni replay.
- La mano lenta usa una copia locale della lista, quindi va ricontrollata con una lista vera che contenga Ghost Mourner.

## Il tetto per richiesta: 500 000 ingressi in Lua (22/10/2026, corsia 2)
**Cosa si conta.** Il primo tentativo contava le istruzioni con un hook di Lua: il banco costava il 21% in più (192,9 s contro 159,9 s), perché con un hook di conteggio la macchina virtuale controlla prima di ogni istruzione. Un hook sulle chiamate costava ancora l'11% (177,5 s). Il tetto conta quindi gli ingressi del motore in Lua (`interpreter::call_lua`):
- `build.rs` compila una copia di `interpreter.cpp` in cui `call_lua` chiede prima allo shim (`shim/count.cpp`), un incremento per ingresso; `vendor/` non cambia;
- oltre il tetto l'ingresso fallisce come un errore di script, che il motore già gestisce; `Duel::advance` restituisce un errore, la ricerca conta la linea come interrotta e il duello non si riusa;
- il registro del motore tiene al massimo 100 righe per duello, perché ogni ingresso dopo il tetto registra un errore;
- `DUELFORGE_LUA_LIMIT` cambia il tetto (0 lo toglie); gli hook di Lua restano solo per `DUELFORGE_LUA_COUNT`, che ora salva anche chiamate e ingressi.

Le richieste lente sono fatte proprio di ingressi: 4,5-5,4 milioni sulla mano lenta di Kewl Tune, 3,2 milioni contro Lunalight. Un ciclo tutto dentro Lua, senza ingressi dal motore, non si ferma; nei casi misurati non c'è.

**Soglia.** Giro a 20 s (41 deck × 2 mani, 25,6 milioni di richieste): la richiesta con più ingressi ne fa 110 003 (elfnote_kewl_tune, mano 1), la seconda 36 137. Il tetto di 500 000 è 4,5 volte la prima e 14 volte la seconda. Con 1 milione la mano lenta pagava ancora 12 richieste da 0,8 s (tra 500 000 e 1 milione di ingressi) a ogni replay.

**Prima e dopo** (prima: `main` 27c9ba0; dopo: il tetto):
- **Costo:** banco a macchina libera, alternando i due binari: 170,3 s contro 170,8 s, poi 165,2 s contro 165,1 s.
- **Banco** (16 mani, 4000 nodi): 15 mani su 16 identiche. L'ultima, inv-aleister, differisce solo nell'ultima cifra di un decimale (`countered` 0,801418439716312 contro 0,8014184397163119), e lo fa anche tra due giri dello stesso binario di `main`.
- **`combo_hunt.py --seed 7`** (41 deck, 2 mani, 3000 nodi): 82 righe su 82 identiche, nessun problema.
- **Mano lenta di Kewl Tune** (60 s, 1000 nodi, seme 7, da sola):

| | Nodi | Fine della ricerca | Miglior punteggio | Linee interrotte |
|---|---|---|---|---|
| Prima | 117 (in 60 s) | 60,5 s | 11,0 | 0 |
| Tetto 1 milione | 254 (in 60 s) | 60,4 s | 11,4 | 1 |
| Tetto 500 000 | 1003 | 25,6 s | 12,4 | 12 |

Il tempo totale resta circa 72 s: finita la ricerca, i giri di apprendimento arrivano comunque alla scadenza (limite più 10 s). Accorciarli tocca `search.rs` (corsia 4).

- **Turno di prova contro Lunalight, livello 2** (caso L4-3 della corsia 3; board: la prima di `combo_hunt --hands 1 --top 8 --seed 7`), tempo di `against its lines`:

| Board | Prima | Dopo (1 milione) | Valore prima → dopo |
|---|---|---|---|
| Branded | 359,0 s | 6,1 s (4,0 s a 500 000) | 1,99 → 2,02 |
| Dracotail | 524,2 s | 19,4 s | 2,34 → 2,34 |
| Labrynth | 550,2 s | 9,7 s | 1,86 → 1,88 |
| K9 | 297,2 s | 6,9 s | 2,03 → 2,06 |

Il costo: su tutte e quattro le board le linee 9 e 10 di Lunalight (su 11) non si misurano più, perché il loro rigioco supera il tetto. Le altre mosse sono identiche. Invoked e Tearlaments erano già veloci (sotto 1,7 milioni di istruzioni per richiesta) e non cambiano.

**Limiti.**
- Le linee che superano il tetto si perdono: nella ricerca sono linee interrotte, nel livello 2 linee dell'avversario non misurate. Ricordare il prefisso della linea per non rigiocarlo tocca `search.rs` e `matchup.rs`.
- La mano lenta usa ancora la copia locale della lista con Ghost Mourner.

## Collaudo dei deck lenti con il tetto (22/10/2026, corsia 2)
**Metodo.** Le mani e le opzioni di `combo_hunt.py --seed 7` (41 deck di Master Duel Meta, 2 mani, 3000 nodi, limite 600 s, un thread, `--top 5 --probe`), con il tempo totale di ogni mano e le fasi della ricerca (`DUELFORGE_DEBUG_PHASES`). Il resto, dopo la fine di `zone_variants`, è quasi tutto il turno di prova. Tre giri, 4 mani alla volta sui 4 core:
- **prima:** `main` 27c9ba0, senza il tetto;
- **dopo, tetto spento:** `main` e424904 con `DUELFORGE_LUA_LIMIT=0`;
- **dopo:** `main` e424904.

Fra 27c9ba0 ed e424904 sono entrati anche altri cambi della ricerca e di `combos`, quindi il tetto si giudica dal confronto fra gli ultimi due giri.

**Il tetto in questi giri non scatta.** Con e senza il tetto le uscite delle 82 mani sono identiche, tolti i tempi, e i tempi sono gli stessi (1505 s contro 1550 s in tutto, con 4 giri insieme). I casi lenti che il tetto toglie sono quelli già misurati: la mano lenta di Kewl Tune con la lista locale, e il livello 2 contro Lunalight.

**Mani sopra 60 s:** 2 prima (elfnote-0 95,9 s, elfnote_kewl_tune-1 103,5 s), 1 dopo (dracotail-0, 67,5-69,5 s). Rimisurate da sole, senza altri processi:

| Mano | Prima | Dopo | Ricerca prima → dopo | Dopo la ricerca | Turno di prova e resto prima → dopo |
|---|---|---|---|---|---|
| dracotail-0 | 44,3 s | 70,7 s | 16,5 → 16,8 s | 9,7 → 8,9 s | 18,1 → 45,0 s |
| elfnote-0 | 96,3 s | 60,1 s | 43,6 → 17,8 s | 20,8 → 10,5 s | 31,9 → 31,8 s |
| elfnote_kewl_tune-0 | 33,9 s | 50,8 s | 12,5 → 13,8 s | 7,5 → 8,5 s | 13,9 → 28,5 s |

**Cause.**
- **dracotail-0 (e in parte elfnote_kewl_tune-0): il turno di prova di `combos --probe`, non Lua né la ricerca.**
  - Dal commit 21c8885 («List each field once in combos») `--probe` misura le prime tre varianti di ogni campo e mostra la migliore.
  - Le 5 board di dracotail-0 sono quindi 15 turni di prova invece di 5, da circa 3 s l'uno: il resto passa da 18 a 45 s.
  - Lua è normale: la richiesta più grande fa 108 mila istruzioni e 2676 ingressi.
  - Riguarda `combos` e `combo_hunt.py`; l'app va guardata a parte.
- **elfnote-0: la ricerca** (43,6 s prima). Con e424904 scende a 17,8 s, per i cambi della ricerca entrati in `main`; ora sta sul filo dei 60 s per il turno di prova (31,8 s).
- **elfnote_kewl_tune-1** (103,5 s prima): con e424904 fa 33 s.

**Da decidere.** Per tenere ogni mano sotto 60 s con `--probe`, si possono misurare meno varianti per campo (una, o tre solo per le prime board) oppure misurarle tutte in un solo turno di prova. È una scelta di `crates/ygo-cli` e della corsia che ha fatto 21c8885, non del motore.

## Le varianti di `combos --probe` (22/10/2026, corsia 2)
**Cambio.** Da 21c8885 `combos --probe` misurava fino a tre varianti di ogni campo elencato. Ora ogni campo si misura una volta, e le altre due varianti solo per i tre campi stampati per primi. Un campo può solo guadagnare dalle sue varianti, quindi quei tre restano in testa e il #1 stampato resta il migliore misurato. Con `--top 5`: al massimo 11 turni di prova invece di 15.

**Ordine** (41 deck, una mano per deck con la regola "-lines", 3000 nodi, seme 1, prime 8 board, `rank_report.py`; prima `main` 18325a7):

| | Prima | Dopo |
|---|---|---|
| deck con il #1 stampato non il migliore misurato (`printed`) | 0 su 41 | 0 su 41 |
| coppie invertite di più di 0,1 | 173 su 1110 | 172 su 1110 |
| somma del migliore misurato | 59,86 | 59,42 |
| misura per board, mediana / p95 | 0,50 / 2,02 s | 0,50 / 1,87 s |
| `combo_hunt.py` | 41 ok, 0 problemi | uguale |

La somma scende in 3 deck, dove la variante migliore era di un campo oltre il terzo: elfnote_kewl_tune 1,336 → 1,032, snake_eye 2,767 → 2,723, vampire 1,238 → 1,147.

**Tempi** (mani di `combo_hunt --seed 7`, da sole):

| Mano | Prima | Dopo | Turno di prova e resto |
|---|---|---|---|
| dracotail-0 | 69,3 s | 64,2 s | 43,3 → 36,9 s |
| elfnote_kewl_tune-0 | 54,3 s | 47,4 s | 30,7 → 24,5 s |

dracotail-0 resta sopra 60 s: le sue 11 misure costano circa 3 s l'una. Per scendere ancora servirebbe misurare due varianti invece di tre, o rendere più veloce la misura stessa.

## Le fasi dopo la ricerca (22/10/2026, corsia 2)
**Domanda.** Sulla mano lenta di Kewl Tune (60 s, 1000 nodi, seme 7) la ricerca finisce presto, ma le fasi successive arrivano comunque verso la scadenza (limite + 10 s). Si possono fermare prima, senza perdere board?

**Prove** (`main` cc8a122, mano lenta da sola; prima: ricerca a 10,5 s, fine a 47,6 s, prime 3 board 8,6 · 8,4 · 8,3):

| Apprendimento | `confirm_best` e `zone_variants` | Fine | Prime 3 board |
|---|---|---|---|
| fermo 10 s dopo la ricerca | 10 s dopo la ricerca | 19,8 s | 8,3 · 8,3 · 8,1 |
| fermo 10 s dopo la ricerca | 15, 20, 25, 30, 40 s dopo la ricerca | 24,5-31,8 s | 8,3 · 8,3 · 8,1 |
| come prima | 20 s dopo la ricerca | 31,2 s | 8,3 · 8,3 · 8,1 |
| come prima | 25 s dopo la ricerca | 35,3 s | 8,6 · 8,4 · 8,3 |
| come prima | 25 s dopo la ricerca (secondo giro) | 36,9 s | 8,6 · 8,3 · 8,3 |
| come prima | 30 s dopo la ricerca | 41,0 s | 8,6 · 8,4 · 8,3 |

- Le board migliori vengono sia dall'ultimo giro di apprendimento intero, sia dalle varianti di zona giocate fino in fondo. Le varianti finiscono da sole circa 37 s dopo la ricerca.
- Un taglio a tempo prima di allora le perde o no a seconda della velocità della macchina: a 25 s, una volta sì e una volta no.
- Nessuna scadenza più corta di quella di prima tiene la qualità in modo affidabile. Quindi la scadenza resta quella di prima.

**Cambio rimasto** (`search.rs`). Un giro di apprendimento non comincia una board che non finirebbe prima della scadenza, a giudicare dalla misura più lunga del giro: tagliata a metà, non insegna niente. Sulle prove qui sopra non scatta mai, e la mano lenta finisce come prima.

Il tempo di queste mani lo accorcia l'utente, scegliendo il limite nell'app; la ricerca più efficiente è il lavoro del piano critico (`docs/plans/ricerca-piano-critico.md`, A2 e B1-B7).

## Dove va il lavoro della ricerca (A2 del piano critico, 22/10/2026, corsia 2)
**Strumento.** Con `DUELFORGE_SEARCH_STATS=1` la ricerca stampa alla fine una riga `search-stats` su stderr; senza, non registra niente (banco con e senza: uscite identiche). `scripts/search_stats.py <cartella>` somma le righe di tutte le ricerche. Per ogni discesa (un replay della linea dal principio, poi la prima risposta a ogni scelta fino alla fine del turno o a un taglio) conta:
- come finisce, con i suoi nodi, il tempo del replay e il tempo totale;
- la board migliore che legge, rispetto alla migliore della ricerca;
- le risposte dopo cui niente di visibile cambia;
- la posizione della risposta giusta, a ogni passo della linea migliore, tra quelle provate.

**Giri.** Banco: 16 mani, 4000 nodi, un thread. `combo_hunt --seed 7`: 82 mani, 3000 nodi, `--top 5 --probe`, 4 giri insieme. `main` cc8a122 più la PR #64.

| | Banco | `combo_hunt` |
|---|---|---|
| nodi | 60 211 | 222 071 |
| tempo della ricerca | 148,0 s | 619,8 s |
| di cui nelle discese | 87,6 s | 342,2 s |
| replay della linea dal principio | 69,9 s (80% delle discese) | 266,8 s (78%) |
| giri di apprendimento durante la ricerca | 28,5 s | 62,9 s |
| fasi dopo la ricerca | 33,1 s | 223,3 s |
| turno di prova e stampa | — | 859 s (58% del tempo totale di 1479 s) |

**Come finiscono le discese** (`combo_hunt`; il banco è simile):

| Fine | % nodi | % tempo delle discese |
|---|---|---|
| fine del turno, board letta | 73,5% | 66,6% |
| nodo già esplorato per altre scelte | 15,6% | 18,3% |
| nodo già esplorato con le stesse scelte in altro ordine | 9,3% | 10,2% |
| ogni risposta riporta dove un'altra linea va già | 1,4% | 4,5% |
| altro (negata, giro, handtrap inutile) | 0,2% | 0,4% |

- **Board lette:** il 19% del tempo delle discese va in discese che non leggono nessuna board (quasi tutte finiscono su un nodo già esplorato). Il 73% va in discese con board sotto il 90% della migliore, l'8% in discese vicine alla migliore.
- **Risposte che non cambiano niente di visibile:** 23,8% dei nodi. Sono i passi delle selezioni a più carte (`Choose`, 26 741) e la scelta di un'Evocazione Speciale prima dei suoi materiali (`SpecialSummon`, 21 521).
- **Ordinamento delle mosse:** sulla linea migliore la risposta giusta era la prima provata nel 77% dei passi, la seconda nell'8%, la terza nel 4%, oltre la decima nello 0,3%.

**I tre sprechi più grandi.**

| | % nodi | % tempo della ricerca |
|---|---|---|
| 1. Replay delle linee dal principio del duello: ogni discesa rigioca tutte le risposte già date per arrivare al suo nodo | — | 43% (78-80% del tempo delle discese) |
| 2. Discese che finiscono su un nodo già esplorato (altre scelte 15,6%, altro ordine 9,3%, risposte che riportano indietro 1,4%) | 26,3% | 18% (33% del tempo delle discese) |
| 3. Nodi senza cambi visibili (passi delle selezioni a più carte, scelta dell'Evocazione prima dei materiali) | 23,8% | non misurato a parte: ogni nodo in più allunga anche i replay successivi |

Fuori dalla ricerca, in `combo_hunt` pesa di più il turno di prova (58% del tempo totale) delle fasi dopo la ricerca (15%) e dei giri di apprendimento (4%). Il turno di prova lo guarda il coordinatore insieme a B5.

**Cosa ne segue per la fase B.**
- **B2 (ordini scambiabili)** toglie al più il 9,3% dei nodi e il 10% del tempo delle discese: è lo spreco misurato più chiaro.
- **B3 (chiave dei nodi)** tocca la parte "altre scelte" (15,6%) solo se quelle scelte arrivano davvero allo stesso stato; da misurare con la chiave più fine.
- **B1 (ordinamento)** ha poco margine: la risposta giusta è già la prima nel 77% dei passi.
- **Il replay** è lo spreco più grande e nessun punto B lo tocca. Due strade, entrambe nel motore e nella ricerca:
  - discese più lunghe per replay, cioè più risposte lasciate in coda per ogni replay;
  - un'istantanea del duello a metà turno, come quella che già riusa il duello all'inizio, per ripartire da un nodo senza rigiocare.
- **I nodi senza cambi visibili** si potrebbero contare come un solo passo: una selezione a più carte, o un'Evocazione con i suoi materiali. Nel piano è B2b, dopo B2, con il controllo che la chiave dei nodi non perda stati distinti.
- **Ordine deciso:** B0 (il replay, studio sotto), B2, B2b, B3, B1.

## B0: il replay, studio delle strade (22/10/2026, corsia 2)
Nessun codice nella ricerca: misure e stime, per decidere.

**Dove va il tempo di una discesa.** Profilo con callgrind di una mano del banco (inv-r1, 300 nodi), dentro `play`:

| Parte | Quota |
|---|---|
| rigiocare le risposte della linea (`advance` sul prefisso) | 45% |
| preparare il duello: azzeramento e carte aggiunte (`goldfish_against_from`) | 29% |
| esplorare i nodi nuovi | 17% |
| leggere lo stato, chiavi, resto del nostro codice | meno del 10% |

- Del rigiocare, il 98% è il motore (`OCG_DuelProcess`, quasi tutto Lua); leggere i messaggi è circa l'1%.
- Della preparazione, circa due terzi sono le carte aggiunte: 111 per duello, ognuna con il suo `initial_effect` in Lua. Un terzo è il duello nuovo o riusato.
- Sul banco ogni discesa rigioca in media 56 risposte per 4 nodi nuovi. Solo il 3,8% delle risposte rigiocate viene dopo l'ultimo momento libero (una finestra di scelta senza effetti a metà, Main Phase o catena aperta).

**Le strade.**

| Strada | Guadagno stimato | Costo e rischio |
|---|---|---|
| 1. Ordinare le discese perché condividano il prefisso | nessuno da sola: un duello non torna indietro, quindi due discese possono condividere un prefisso solo partendo da una copia | — |
| 2. Discese più lunghe per replay | quasi nessuno: arrivano già alla fine del turno; il replay sprecato è quello delle discese che finiscono su nodi già visti (33% del tempo delle discese), e lo tolgono B2 e B3 prima del replay | — |
| 3. Replay più economico dal nostro lato (saltare letture di messaggi e stato) | 1-2% del tempo delle discese | piccolo, ma inutile |
| 4. Copia del duello dopo le carte aggiunte, prima di `OCG_StartDuel`: tutti i replay di una ricerca partono dalla stessa situazione | fino al 29% del tempo delle discese, circa il 15% della ricerca; anche le misure dell'apprendimento e del turno di prova con lo stesso mazzo | medio: estende il riuso dei duelli (`shim/reuse.cpp`), riportando le carte e i loro effetti allo stato iniziale invece di ricrearli; controllo già pronto con `DUELFORGE_CHECK_REUSE` (ogni passo confrontato con un duello nuovo). Una o due settimane |
| 5. Copie del duello ai momenti liberi, tenute in una cache (le più recenti o i prefissi con più risposte in coda); ogni discesa riparte dalla copia più vicina | fino al 96% delle risposte rigiocate più la preparazione: circa il 70% del tempo delle discese, il 35-40% della ricerca, se la cache trova la copia spesso e se ripristinarla costa molto meno di 5,6 ms (il replay medio di oggi) | grande: copiare e ripristinare carte, effetti, gruppi, campo, catena e tabelle Lua a metà turno; ai momenti liberi non ci sono coroutine sospese, che Lua non sa copiare. Più settimane, rischio alto; si costruisce sopra la strada 4 |

**Proposta.**
1. Prima la strada 4: guadagno certo, sopra il meccanismo già provato del riuso e con lo stesso controllo.
2. Poi misurare quanto costa ripristinare quella copia rispetto a un replay. Se costa molto meno, la strada 5 (la vera istantanea a metà turno) vale il suo costo; altrimenti si ferma alla 4.
3. Le strade 1-3 non valgono il lavoro.
4. Nessuna richiede modifiche a `vendor/ygopro-core`: tutto passa dallo shim, come oggi.

## B0: le prove (22/10/2026, corsia 2)
### Strada 4: copia del duello dopo le carte aggiunte (fatta, non tenuta)
- **Com'era fatta** (`shim/reuse.cpp`): prima di `OCG_StartDuel` si copiano carte, effetti, gruppi, le parti del campo cambiate dalle carte aggiunte (giocatori, effetti del campo, contatori, `global_flag`, i contatori di attività) e le tabelle Lua. Due accorgimenti:
  - gli effetti della copia cancellati durante il duello vengono tenuti in memoria (patch di `duel::delete_effect` in `build.rs`) e rimessi al ripristino;
  - i gruppi della copia sono tenuti vivi dalla copia, perché il collector di Lua li libererebbe.
- **Verifiche:** `DUELFORGE_CHECK_REUSE` su tre mani del banco a 1000 nodi senza differenze, memcheck pulito, banco identico.
- **Costi:** il ripristino costa circa 1,5 milioni di istruzioni contro circa 5,3 milioni per preparare il duello. Fare la copia costa circa 10 milioni e lascia migliaia di tabelle al collector. Per questo si copia solo una preparazione che torna: il turno di prova e l'apprendimento ne fanno una per mano.
- **Banco alternato, prima → dopo:** ricerca 142,0 → 144,4 s e 136,1 → 148,0 s; tempo totale 368 → 527 s e 353 → 540 s.
- **Conclusione:** guadagno nullo sulla ricerca e perdita sul resto. Ripristinare oggetti costa troppo rispetto a quello che risparmia. Il codice resta su un branch locale come riferimento.

### B1a: l'ordine dei fratelli in coda (misurato, non tenuto)
- A pari priorità la coda prende l'arco più nuovo, e i fratelli di un nodo, che hanno quasi sempre la stessa priorità, sono spinti nell'ordine di `dive_order`: esce per prima l'alternativa meno preferita.
- Ribaltato l'ordine a ogni profondità, tenendo prima i più profondi, il banco a 4000 nodi peggiora:
  - somma dei migliori 25,85 → 24,39 (−5,6%);
  - peggiorano inv-r3 (3,09 → 1,60), dracotail (1,84 → 1,59), branded (1,16 → 1,05) e ignister (1,22 → 1,18); migliora mitsurugi (1,99 → 2,42).
- L'ordine di oggi rende di più a nodi fissi: provare per primi Passa e la fine del turno chiude in fretta i rami e porta prima alle board da confermare.

### Prototipo dell'arena (B13 = B0-5): il duello intero copiato come memoria
- **L'idea**, da ygo-combo-solver (stesso core, licenza AGPL): tutte le allocazioni di Lua e del C++ del motore vanno in un'arena per thread. Una copia della parte usata è un salvataggio, e ripristinarla allo stesso indirizzo lascia validi tutti i puntatori. Le coroutine Lua sospese sono comprese, perché si copia memoria e non oggetti.
- **Prototipo** (`shim/arena.cpp`, branch locale): regione riservata (`mmap` con `MAP_NORESERVE`; su Windows `VirtualAlloc`), allocatore a classi di grandezza con il suo stato in testa all'arena. `l_alloc` (patch di `build.rs`) e `operator new`/`delete` vanno nell'arena del thread mentre è attiva, altrimenti in mimalloc. `vendor/` non cambia.
- **Prova:** partite con risposte casuali fino alla fine del turno; si salva a metà, si gioca il resto, si ripristina e si rigioca il resto 5 volte, confrontando stato ed esiti a ogni passo. Il lato Rust del duello (turno, fase, attività, catena, decisione in attesa) va salvato e ripristinato insieme.

| Deck | Partite | Ripristini identici | Memoria del duello | Salvataggio | Ripristino | Replay fino allo stesso punto |
|---|---|---|---|---|---|---|
| Invoked | 100 | 500/500 | 2,0 MB | 0,22 ms | 0,18 ms | 4,12 ms |
| Dracotail | 100 | 500/500 | 1,9 MB | 0,19 ms | 0,15 ms | 3,43 ms |
| Elfnote Kewl Tune | 100 | 500/500 | 2,4 MB | 0,35 ms | 0,22 ms | 5,03 ms |

- **Esito:** ripristinare costa circa 22 volte meno di rigiocare. La copia è intera; con le pagine sporche costerebbe ancora meno.
- **Stima sulla ricerca:** il replay e la preparazione sono circa tre quarti del tempo delle discese, e le discese circa il 55% della ricerca. Con un ripristino da 0,2 ms al posto di un replay da 4-5 ms, la ricerca scende di circa il 35-40%, se si trova spesso una copia vicina al nodo.
- **Da risolvere per farne la strada vera:**
  - quante copie tenere: 2 MB l'una, quindi una cache limitata per thread, ai nodi con più fratelli in coda, ripartendo dalla copia più vicina e rigiocando solo il resto;
  - il riuso dei duelli di oggi, che l'arena rende inutile: ogni ricerca riparte da una copia del duello appena preparato;
  - Windows: `VirtualAlloc`, e `GetWriteWatch` per le pagine sporche;
  - nessuna allocazione fuori arena durante un duello: da contare e controllare, come fa ygo-combo-solver.

## B14: la valutazione delle board a gradini (22/10/2026, corsia 8)
**Domanda.** Oggi l'app misura per intero le prime 30 board della stima della ricerca. Come si spende meno tempo senza perdere la board migliore? La proposta viene dagli esperimenti della corsia 7 (`ricerca-esperimenti.md` §1, sul branch `claude/research-lab`).

**Cosa cambia.**
- **Misura economica** (`Prober::measure_cheap` in `probe.rs`). Ogni risposta si prova solo alle prime 2 finestre in cui è offerta, invece che a 16, e si usa una volta sola: niente ciclo degli usi, niente breaker, né la risposta dell'avversario alla prima interruzione. `Parts` ha due campi in più, finestre e giri degli usi. La misura completa resta a 16 e 3, identica.
- **Valore per il taglio:** campo + 0,9 · mano + 0,8 · floodgate, come nel composito ma senza resilienza né follow-up, che la misura economica non gioca.
- **App** (`combo_score`, mano precisa):
  1. le prime 40 board della ricerca, una per aspetto, hanno la misura economica;
  2. le migliori 15 per il valore economico hanno la misura completa, con l'esposizione e il confronto con il deck scelto;
  3. le altre restano visibili con il loro punteggio economico, segnato da `ScoreView.cheap` (`cheap?: boolean` in `api.ts`): niente breaker, follow-up, esposizione né confronto, e si mettono dopo quelle misurate per intero.

  Le misure economiche non dipendono dal deck avversario, quindi restano valide quando lo si cambia. In «carta + resto» non cambia niente: tutte le board tenute hanno la misura completa.
- **CLI:** `combos --staged` (e `combo_hunt.py --staged`) fa la stessa scala sui campi elencati: misura economica sui primi 40, misura completa sui migliori 15. Il JSON dice per ogni board in quale gradino si è fermata (`stage`) e dà il tempo di tutti i turni di prova (`probe_seconds`).

**Protocollo.** `combo_hunt.py --hands 1 --seed 7` sui 41 deck MDM del 22/10, 4 processi in parallelo su 4 core, build release:
- **A, oggi:** `--top 30`, misura completa delle prime 30;
- **C, a gradini:** `--top 40 --staged`;
- **B, riferimento:** `--top 40`, misura completa di tutte le 40, sul binario di `main`.

Il valore è il composito della CLI (campo, mano, floodgate, resilienza, follow-up); il tempo è quello di tutti i turni di prova, esposizione compresa.

| | A (oggi) | C (a gradini) |
|---|---:|---:|
| tempo dei turni di prova, 41 mani | 914 s | **653 s (71%)** |
| mediana per mano del rapporto C/A | | 73% |
| #1 identico ad A | | 37/41 |
| #1 uguale o migliore di A | | **41/41** |
| mani con un #1 peggiore di A oltre l'1% | | **0** |
| valore del #1 rispetto ad A (media / peggiore) | | +0,6% / 0,0% |
| top 3 con almeno 2 board su 3 in comune con A | | 35/41 |
| #1 uguale al migliore di B (tutte le 40 misurate) | 35/41 | **38/41** |

- In 3 mani la scala trova una board migliore di quella di oggi: Doomz +18,7%, White Forest Azamina +5,1%, Phantom Knights +1,3%. Sono board oltre la 30ª della stima (la 34ª, la 32ª, la 35ª), che oggi non si misurano. In Vampire il #1 è un'altra board di pari valore.
- Le 3 mani dove il #1 di C non è il migliore di B (Branded, Paleozoic, R-B) hanno lo stesso #1 di oggi.
- Le mani con la top 3 cambiata hanno il #1 uguale o migliore: cambiano il 2° e il 3° posto, perché le board misurate per intero sono 15 e non 30.
- **La misura completa non cambia.** Delle board con lo stesso indice e la stessa firma in A (binario nuovo) e in B (binario di `main`), 1173 su 1174 hanno lo stesso valore. L'unica diversa (Phantom Knights, indice 15, follow-up 1 o 0) cambia anche con il solo binario di `main` tra `--top 30` e `--top 40`, quindi la differenza c'era già prima.
- **Criterio di accettazione** (nessun #1 peggiore oltre l'1%, tempo almeno −25%): raggiunto, −29%. Il controllo del piano (−50%) richiederebbe la scala veloce, 20 → 10, che secondo la corsia 7 perde fino al 26% in una mano.
- **Altri controlli.** Il banco (`bench.py`, 4000 nodi) è identico prima e dopo (somma di `best` 11,56, di `first` 11,55). Le mani sono 12 su 16, perché le liste MDM del 22/10 non hanno Ignister, Labrynth, Mitsurugi e Yummy Mimighoul. `search_regression` è verde, e così i test, fmt, clippy (anche `src-tauri`), `pnpm typecheck` e `pnpm build`.
- **I numeri sono della CLI, a un thread per mano.** Nell'app le board si misurano su tutti i core, ma il rapporto tra i tempi resta lo stesso, perché le due fasi sono parallele allo stesso modo. Nell'app vera il tempo non è stato misurato.
