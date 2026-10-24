# Caccia ai bug e alle inefficienze del motore delle combo

*Corsia 9, 23/10/2026, branch `claude/engine-bughunt` da `main` 9cc3005 (dopo le PR #80-#99: copie del duello B13, misura a gradini B14, rigioco del livello 2, «risposte insieme», oracolo `--staged`). Lettura del codice di `crates/ygo-solver`, `crates/ygo-engine` e della parte `combos` di `crates/ygo-cli`; riproduzioni con la CLI (build release, dati del 22/10, liste MDM del 22/10, deck Invoked della fixture `invoked-siracusa-md.ydk`, un thread). Nessuna modifica al codice. Ogni voce ha file e funzione, lo scenario o la riproduzione, la gravità, la correzione proposta e il controllo. Le ipotesi controllate e scartate sono in fondo: valgono quanto le voci, perché dicono cosa non serve cercare.*

**Gravità.** *Alta*: cambia quale board l'utente vede, o fa perdere board forti. *Media*: perde ordine o tempo in modo misurabile, o rende il risultato dipendente da budget e thread. *Bassa*: percentuali di tempo o memoria, senza effetto sulle board.

## In breve
1. **(Alta) La board che rappresenta un campo cambia con il budget.** `combos` raggruppa per `field_key` e misura il primo del gruppo nell'ordine della ricerca; l'app fa lo stesso per `look_key`. Con più nodi il primo cambia: su sky_striker-1, 6 campi su 22 comuni cambiano rappresentante tra 1000 e 20 000 nodi. È una causa misurata della non monotonia dell'oracolo (28 mani su 88 che scendono con il budget), insieme alle fasi finali a dimensione fissa (`confirm_top` 40, `zone_top` 20, i 300 campi di `oracle.py --staged`).
2. **(Media, già assegnata alla corsia 2) La tabella delle trasposizioni chiude un nodo alla prima visita** mentre le priorità dipendono dal cammino: il cammino migliore che arriva dopo è scartato e i figli restano alla penalità del peggiore.
3. **(Media) Tre potature imparate durante la ricerca valgono per tutto il resto della ricerca e per tutti i thread** (`idle_hand_traps` dopo 3 colpi, `own_negators`, i bersagli nascosti tolti dai fratelli): cosa si pota dipende da quali linee si sono viste prima, quindi dal budget e dal numero di thread.
4. **(Media-bassa) Lavoro che nessuno legge:** le etichette delle azioni si costruiscono a ogni decisione anche nel turno di prova, dove non si mostrano mai: `actions_with` è il 5,25% delle istruzioni del programma (profilo callgrind, §4). Nella ricerca etichette, cloni e impronte sono circa il 4%. Gli yield delle coroutine Lua passano per eccezioni C++: 2,56%.
5. **(Media) Nel rigioco del livello 1 contro un deck una risposta rifiutata dal motore viene sostituita in silenzio** fino a 20 volte, anche senza handtrap in mezzo: una linea che è cambiata può essere misurata come se avesse giocato.
6. **(Bassa) Memoria e lavoro che crescono con le board trovate:** ogni board tiene uno stato intero del duello e le etichette della linea; la classifica si riordina tutta quattro volte al secondo sotto il lock; le fasi finali e i giri di apprendimento rigiocano le linee dal principio anche con l'arena a disposizione.

Nessuna gara tra thread trovata nel codice letto; la dipendenza dal numero di thread viene dalle potature e dai valori imparati condivisi (punto 3 e B10), non da accessi non protetti. Le copie del duello (B13) sono fedeli al replay (`DUELFORGE_COPY_CHECK=1` su due deck, §7).

---

## 1. (Alta) Il rappresentante di un campo cambia con il budget
**Dove.** `crates/ygo-cli/src/main.rs`, `combos`: le board della ricerca si raggruppano per `ygo_solver::search::field_key` (solo le carte sul tuo campo, zone comprese nella stessa chiave; mano, Cimitero, bandite ed Extra Deck ignorati). Il rappresentante è `group[0]`, cioè la **prima board nell'ordine della ricerca** (`rank`: stima rapida, poi meno carte usate, poi linea più corta). La misura economica e quella intera si fanno sul rappresentante; le altre varianti si misurano solo per i primi 3 campi stampati, 2 per campo (`FIELDS_WITH_VARIANTS`, `MEASURED_VARIANTS`). Nell'app (`apps/desktop/src-tauri/src/lib.rs`, righe 1975-2000) la chiave è `look_key` (campo e mano), il rappresentante è di nuovo il primo nell'ordine della ricerca, e le varianti per Cimitero, bandite e zone principali non si misurano mai.

**Perché è un bug.** La stima rapida non vede Cimitero, bandite, materiali né l'avversario, quindi tra le varianti di un campo sceglie quasi a caso rispetto alla misura. Con più nodi un campo accumula varianti e il primo cambia. Il valore misurato di quel campo cambia con il budget senza che la ricerca abbia trovato niente di meglio o di peggio.

**Riproduzione** (lista Sky Striker MDM del 22/10, mano della fixture `hunt-sky_striker-1`, seme 7, un thread):
```
duelforge-cli combos --deck <mdm>/sky_striker.ydk --hand "14558127;26077387;63166095;14558127;26077387" --seed 7 --nodes 1000  --threads 1 --top 5000
duelforge-cli combos --deck <mdm>/sky_striker.ydk --hand "14558127;26077387;63166095;14558127;26077387" --seed 7 --nodes 20000 --threads 1 --top 5000
```
| | 1000 nodi | 20 000 nodi |
|---|---|---|
| campi distinti | 41 | 76 |
| campi comuni ai due budget | 22 | |
| **campi comuni con un rappresentante diverso** (altra mano) | **6** | |

Esempio: il campo «Sky Striker Ace = Zero [EMZ] + Widow Anchor coperta» ha a 1000 nodi la mano (Raye, Ash, Ash) e 0 varianti; a 20 000 nodi la mano (Raye, Ash, Raye, Ash, Engage) e 105 varianti. La lista di oggi non è quella della fixture (l'hash è diverso: Maxx "C" non c'è), quindi il caso esatto del piano non si rigioca qui; il meccanismo sì.

**Le stesse scelte a dimensione fissa, che cambiano con il budget nello stesso modo:**
- `search.rs`, `resume`: `confirm_best` gioca la End Phase alle prime `confirm_top` (40) board provvisorie per `rank`; `zone_variants` prova le altre zone per le prime `zone_top` (20) board esatte. A 1000 nodi (295 board) la board giusta è quasi sempre tra le 40; a 200 000 nodi (20 000 board) no. Una board provvisoria oltre la 40ª resta senza End Phase e senza varianti di zona.
- `scripts/oracle.py --staged 300,30`: i primi 300 campi per la stima, poi i 30 migliori per la misura economica. A1 registra già warrior_combo-1 fuori dai 300.

**Fatto in `combos --staged` (24/10):** il rappresentante è la migliore per misura economica di fino a 3 varianti (`piano-motore-impatto.md` §4, «Esito»); l'app resta da fare.

**Correzione.** Il rappresentante di un campo (o di un aspetto) si sceglie con la **misura economica tra le sue varianti**, non con la stima: si misura economica ogni variante distinta per `board_key` dei primi N campi (le varianti di uno stesso campo differiscono in mano, Cimitero, bandite e zone, che la misura vede), oppure si raggruppa *dopo* la misura economica. Le fasi finali si dimensionano in proporzione alle board trovate (per esempio il 2% delle provvisorie, con un minimo di 40), o meglio per stabilità: finché la top K misurata cambia. Nessun nome di carta.

**Controllo.** Sulle 91 mani dell'oracolo, mani con il valore in calo di oltre l'1% passando a un budget maggiore: da 28 a meno di 5. Sulle 82 mani di `combo_hunt --seed 7`, il #1 misurato con il nuovo criterio non è mai sotto quello di oggi (test appaiato).

## 2. (Media) La tabella delle trasposizioni chiude un nodo alla prima visita
*Già nella revisione (`revisione-motore.md` §1.3 e §3.2) e assegnata alla corsia 2; qui solo per completezza della classifica, con le righe.*

**Dove.** `search.rs`, `Shared::first_visit` (un `HashSet<u64>` a 64 shard) e il suo uso in `play`, subito dopo `node_key`: un nodo già visto termina la discesa con `End::Duplicate`. In `work`, le risposte lasciate da una discesa entrano in coda con `discrepancies = edge.discrepancies + 1` e `priority = value − 0,6 · discrepancies`.

**Scenario.** Il nodo N è raggiunto prima da una linea con 3 deviazioni: i suoi fratelli entrano in coda a −2,4. La linea principale (0 deviazioni) arriva a N dopo e viene scartata: nessuno riaccoda i fratelli a −0,6. A budget finito è una perdita di ordine, e l'ordine è ciò che decide quali board si trovano.

**Correzione.** La voce della tabella tiene la penalità minima vista per il nodo; un arrivo con penalità minore riaccoda i figli con la nuova penalità (le voci già in coda possono restare: escono dopo). **Controllo.** Prima, contare in `DUELFORGE_SEARCH_STATS` i nodi raggiunti una seconda volta con `discrepancies` minore della registrata (oggi non si conta). Dopo, `best` del banco non scende e il tempo alla board migliore cala; test appaiato su taratura e freschi.

## 3. (Media) Potature imparate durante la ricerca, globali e dipendenti dall'ordine
**Dove.** `search.rs`, `Shared`: `idle_hand_traps` (RwLock<HashSet<u32>>), `own_negators` (RwLock<HashSet<Mechanic>>), `idle_strikes`; in `play`:
- una handtrap attivata al momento libero che non dà niente di nuovo da fare prende un colpo (`shared.strike`); al terzo (`IDLE_STRIKES`) entra in `idle_hand_traps` e **non viene più attivata dalla mano in nessuna linea per il resto della ricerca**, anche in quelle già in coda (`actions.retain(...)` prima di `dive_order`, e il taglio `End::IdleHandTrap` per gli archi già accodati);
- una tua carta che nega un tuo anello finisce in `own_negators` e non si concatena più alla tua catena per il resto della ricerca;
- un bersaglio o un nome dichiarato che non cambia nulla di visibile fa togliere dai fratelli **tutte** le altre risposte dello stesso tipo a quella profondità (`hidden`, `dive.left.retain`): le alternative dello stesso effetto non si esplorano (è il limite noto del PIANO §30.8).

**Perché conta.** I tre insiemi si riempiono con le prime linee esplorate. Quali siano dipende dal budget (un giro corto non arriva ai 3 colpi), dal numero di thread (un altro worker li riempie prima) e dall'ordine della coda. Una handtrap inutile nelle prime 3 linee e utile in una dopo (Labrynth Cooclock prima e dopo un Set: il codice lo sa, `useful_hand_traps`, ma solo se la linea utile arriva prima del terzo colpo) resta in mano per tutta la ricerca. Lo stesso per una carta che in una linea nega la tua catena per errore e in un'altra è una risposta dalla mano legittima: dopo la prima è esclusa ovunque. Sono potature, non ordini: il verdetto «impossibile» della modalità obiettivo le eredita.

**Correzione.** I tre insiemi diventano **ordine, non taglio**: una risposta «colpita» va per ultima tra i fratelli (penalità in `dive_order` e in coda), e il taglio resta solo in `--exact` disattivabile (è la struttura `Pruning` proposta nella revisione, §3.2). Per i bersagli nascosti: le alternative tolte si rimettono in coda alla penalità massima (lo stesso esperimento del PIANO §30.8, che migliorava il banco e oscillava sulla caccia: va ripetuto con il gate su taratura e freschi).

**Controllo.** Contatore di quante risposte vengono tolte da ciascun insieme per ricerca (oggi ignoto); audit a budget fisso con e senza ogni insieme su 20 mani: valore perso per insieme sotto lo 0,5%; stesse board con 1 e con 4 thread (oggi non è garantito).

## 4. (Media-bassa) Lavoro che nessuno legge: etichette, cloni, impronte
**Dove**, in `play` (`search.rs`) per ogni nodo:
- `duel.actions(&decision)` costruisce `Vec<Action>` con un'etichetta `String` per ogni azione (`actions.rs`, `Action::label`), e viene chiamata **due o tre volte per nodo**: in `advance_auto_seen` (per ogni decisione, anche quelle risposte in automatico), poi in `hand_trap_trial` per la risposta dell'arco (riga ~1570) e per la prima risposta del nodo (riga ~1912);
- `italian::choice` (17 `String::replace` e un `format!`) per **ogni fratello** lasciato in coda (riga ~1904) e **due volte** per la prima risposta (righe ~1916 e ~1935), anche se nessuno leggerà mai quelle etichette: servono solo alla linea mostrata all'utente;
- `Arc::new(line.clone())` per nodo (riga ~1872): `Line` è `Vec<Vec<u8>>` più `Vec<String>` delle etichette, quindi O(profondità) allocazioni per nodo e O(profondità²) per discesa;
- `shown_state(&state)` clona l'intero `DuelState` e ricalcola l'impronta canonica (righe ~1719 e ~1922), oltre a `canonical_fingerprint(&state)` alla riga ~1733: due o tre impronte per nodo;
- `duel.state()` (14 interrogazioni FFI del motore) per nodo, più una in `distinct_zones` a ogni `SelectPlace`.

**Quanto pesa.** Profilo callgrind di `combos` su inv-r6, 400 nodi, un thread (35,8 miliardi di istruzioni; a questo budget il programma è per l'83% turno di prova: valori delle carte, giri di apprendimento, conferme).
- **Nella ricerca** (`play`, 1,42 miliardi): il replay nel motore è il 49%, `Duel::state` il 7%, le etichette (`italian::choice`, 915 chiamate) l'1%, `canonical_fingerprint` l'1%, `shown_state` lo 0,8%, i cloni di `Line` l'1%. In tutto **circa 4% della ricerca** per etichette, cloni e impronte: poco, ma gratis da togliere.
- **Nel programma intero**, `actions_with` è il **5,25%** con 80 509 chiamate, quasi tutte dal turno di prova (`run_with` chiede le azioni a ogni decisione dell'avversario e delle risposte): `describe_card` 226 768 volte (1,3%), `describe_activation` 132 491 (1,1%), `format!` 415 246 (0,9%), e `drop_copies` (`Vec::retain` sulle copie) l'1,05%. Nessuna di queste etichette viene letta nel turno di prova.
- **Un costo del motore da conoscere:** `luaD_throw` è il 2,56% delle istruzioni con 34 145 `__cxa_throw`. Non sono errori: sono gli *yield* delle coroutine degli script (`Detail::LuaFunction<N>::call`, le funzioni `Duel.Select*`), che nella build di EDOPro, con Lua compilato come C++ (`-xc++` in `build.rs`), passano per un'eccezione C++ a ogni domanda al giocatore. È una scelta del vendor (le eccezioni attraversano i frame C++ del core senza saltare i distruttori), quindi si cambia solo con cautela; vale un esperimento in `build.rs`, non nel core.

**Correzione.** `Action` senza etichetta (tipo, risposta, carta, descrizione) e l'etichetta calcolata a richiesta da `italian::choice` solo per la linea che si mostra; `Line.choices` sostituito dagli indici delle risposte (le etichette si rifanno rigiocando, come fa già `explain.rs`); `shown_state` come impronta con maschera, senza clonare; `hand_trap_trial` che riusa le `actions` già costruite. Uscite identiche per costruzione.

**Controllo.** `actions_with` sotto l'1% del profilo; banco con uscite identiche (`combos --json --save-lines`, come per la PR #33); misura per board −4% e nodi al secondo +2% o più. Per l'esperimento su Lua: istruzioni −2% sul banco, test del motore e `DUELFORGE_CHECK_REUSE` verdi, valgrind pulito.

## 5. (Media) Nel rigioco contro un deck una risposta rifiutata si sostituisce in silenzio
**Dove.** `matchup.rs`, `replay`: `Err(Error::Rejected(_)) if rejected < 20 => { ...; duel.respond(&passive_answer(...)); continue }`. Fino alla PR #91 valeva solo dopo una handtrap (`used`); ora sempre, perché «le carte dell'avversario sono di un altro deck e un effetto casuale sulla sua mano può cambiare le domande prima di qualunque handtrap» (commento alla riga ~775).

**Perché conta.** Un rifiuto del motore è il segno che la linea non calza più. La sostituzione con una risposta passiva fa proseguire il rigioco: la board finale e le `windows` (dove le handtrap si provano) possono essere di una linea diversa da quella misurata, senza che `diverged` lo dica. Venti rifiuti sono molti per una linea di 30-60 risposte. Al livello 2 il controllo delle «mosse fatte su campo vuoto» (`made`) intercetta le divergenze grosse; al livello 1 (`matchup`, handtrap per punto) non c'è un controllo analogo.

**Correzione.** Contare i rifiuti in `Replay` e segnare `diverged` oltre una soglia piccola (2-3), o quando il rifiuto cade dopo che la linea ha già dato la sua prima risposta reale; riportare il conteggio nel JSON di `probe-line` e nel banco oggettivo.

**Controllo.** Sul banco oggettivo (205 board × 19 avversari) la quota di rigiochi con almeno un rifiuto, e quante di quelle righe cambiano verdetto quando si segnano come divergenti; nessun cambiamento sulle righe senza rifiuti.

## 6. (Bassa) Memoria e lavoro che crescono con le board
- **Ogni `Board` tiene un `DuelState` intero** (`shared.board`, `search.rs`) più la `Line` con le etichette, e ogni arco in coda la sua etichetta. Picco di memoria (`VmHWM`) della CLI su sky_striker-1, un thread: **171 MB a 1000 nodi** (295 board), **378 MB a 40 000 nodi** (6382 board): circa 34 KB per board, o 5 KB per nodo, sopra una base di 170 MB (dati, script, 64 MB di copie). A 200 000 nodi, con 20 000-37 000 board, l'oracolo sta intorno al gigabyte per processo, e `oracle.py` ne lancia 4 insieme. In `resume`, alla fine, `boards.values().cloned().collect()` ne fa una seconda copia, e `confirm_best` clona tutte le provvisorie per ordinarle.
- **`Shared::progress`** (chiamata 4 volte al secondo dal thread che segue la ricerca) raccoglie **tutte** le board, le ordina con `rank` e clona le prime 10, tenendo il lock di `boards` che `record` vuole a ogni board nuova. A 37 000 board sono decine di migliaia di confronti ogni 250 ms.
- **Replay dal principio dove una copia basterebbe.** `confirm_best`, `zone_variants_of` e `learn_round` (tramite `Prober`) rigiocano ogni linea con `new_duel`, senza le copie dei worker (`work(shared, edge, None)`): 40 + 20 × varianti + 4 replay interi a fine ricerca, più i replay del turno di prova. Con l'arena già nel worker, il ripristino della copia più lunga costerebbe 0,2 ms invece di 4-5. *(24/10: `confirm_best` e `zone_variants` partono dalle copie dalla PR #104; il turno di prova e quindi `learn_round` dalle copie del prober, `velocita-ricerca.md`, B13 PR 4.)*
- **`Copies::reach`** clona il `Track` (due tabelle hash) a ogni arco e `Store::keep` clona risposte e `Track` per ogni copia; `prefix_hashes` alloca un `Vec` per arco. Piccolo.

**Correzione.** `Board` con lo stato in `Arc` o ridotto a ciò che serve (chiave, campo, mano, Cimitero per le chiavi) e lo stato intero rigiocato a richiesta; `progress` che tiene una top K incrementale; le fasi finali che passano per `Copies`. **Controllo.** VmHWM a 200 000 nodi −50%; tempo delle fasi dopo la ricerca −30% sul banco; uscite identiche.

## 7. Contesto verificato, senza difetto nuovo
- **Le copie del duello sono fedeli.** `DUELFORGE_COPY_CHECK=1` (ogni linea partita da una copia è rigiocata anche dal principio e confrontata) su inv-r6 della fixture e su `hunt-snake_eye-0`, 1500 nodi: nessuna differenza, 0 linee interrotte.
- **La misura economica costa ancora quasi tutta nel ciclo delle risposte.** `combos --staged --staged-cheap 40 --staged-full 0 --json` su inv-r6, 4000 nodi: 40 board in 18,8 s (0,47 s a board); `seconds_by_part`: risposte 17,4 s, «insieme» 1,4 s. La correzione «risposte insieme» (#93, #94) pesa il 7%: non è lei il costo. Resta vero che l'arena nel turno di prova è la leva.
- **L'oracolo non si rigioca con le liste di oggi.** Le fixture registrano `deck_hash`; sky_striker-1 oggi non contiene Maxx "C". Chi riproduce i casi di A1 deve usare le liste del 22/10 salvate dal proprietario, non `mdm_decks.py` di oggi.

## 8. Ipotesi controllate e scartate
- **Gara tra worker su `arena_duels`:** no, il flag è thread-local (`ARENAS.with`, `duel.rs`).
- **`max_choices` 60 taglia le linee lunghe:** non su Snake-Eye: mano della fixture `hunt-snake_eye-0`, 3000 nodi, 300 campi, linea più lunga 45 scelte.
- **`df_lua_alloc`, ramo «stessa classe di dimensione» (`arena.cpp`):** viene dopo il caso `nsize == 0`, quindi le liberazioni non si perdono; `class_at` si legge solo per blocchi dell'arena.
- **`Duel::save_in` con `set_len`:** `df_arena_save` scrive esattamente `df_arena_used` byte; corretto.
- **Ordine dei lock:** `learn_round` prende `boards` poi `measured`, e alla fine `values` (lettura) poi `boards`; `record` solo `boards`; `board()` solo `values`. Nessuna inversione, nessuno stallo possibile.
- **`Stop::Exhausted` mentre un giro di apprendimento è in corso:** `run_workers` aspetta tutti i thread (`thread::scope`), quindi il giro finisce prima delle fasi finali.
- **`measure_resumed` riusa la run senza risposte della misura economica** (che prosegue nel tuo turno 3): `windows` e `taken` si raccolgono prima, quindi la misura intera ripresa coincide con quella diretta (la corsia 7 lo ha verificato su `probe-line`; qui non rimisurato).
- **Le linee dei rivali nell'app sono in cache** (`rival_lines: Mutex<HashMap<..>>` in `lib.rs`); solo `probe-line --vs` le ricalcola a ogni invocazione.

## 9. Ordine consigliato
1. Punto 1 (rappresentante per misura economica, fasi finali proporzionali): è ciò che l'utente vede.
2. Punto 2 (tabella): piccolo, già assegnato.
3. Punto 3 (potature imparate come ordine): toglie una dipendenza da thread e budget.
4. Punto 4 (etichette e cloni): una PR meccanica con uscite identiche.
5. Punto 5 (rifiuti nel rigioco): pochi giorni, migliora la fiducia nel banco oggettivo.
6. Punto 6: quando l'arena entra nel turno di prova.

## 10. Punti 3, 5 e 6: codice scritto, prove da fare (passaggio di consegne, 23/10)
*Branch `claude/project-thread-nben5n`, commit 245a5ca su `main` f3c5c62. Le liste MDM non si scaricavano dal container (rete bloccata), quindi le prove sono solo sul deck Invoked della fixture. Lo riprende una sessione nuova con i domini permessi.*

**Cosa c'è nel codice.**
- **Punto 3** (`search.rs`): le handtrap imparate inutili e i tuoi negatori della tua catena non si tagliano più: vanno per ultimi tra i fratelli (ordinamento stabile dopo `dive_order`) e in coda con `STRUCK` (oggi `END_PHASE_LAST`, 100) tolto alla priorità; un arco già in coda con una handtrap diventata inutile torna in coda con la penalità invece di finire. `DUELFORGE_STRUCK=cut` rimette il taglio di prima (per il confronto), `DUELFORGE_STRUCK=<numero>` cambia la penalità, `DUELFORGE_STRUCK_HIDDEN=1` mette per ultimi anche i bersagli nascosti invece di toglierli (PIANO §30.8). `DUELFORGE_SEARCH_STATS` conta le risposte toccate da ciascun insieme (`struck`).
- **Punto 5** (`matchup.rs`): il rigioco del livello 1 conta i rifiuti del motore. Il rigioco senza handtrap li tollera come prima (20); quello con una handtrap è `diverged` (quindi «ferma») se ne ha più di `MAX_REJECTED` (2) oltre a quelli senza. `Matchup.rejected`/`diverged` e `HandTrapMoment.rejected` sono nel JSON di `probe-line --vs`. `draw_trade_off` è invariato.
- **Punto 6** (`search.rs`): `Board.state` è un `Arc<DuelState>` (le copie per progress, fasi finali e risultato non duplicano lo stato); `progress` ordina solo le prime `live_top` board (`select_nth_unstable_by`, stesso ordine perché `rank` è totale).

**Prove fatte** (deck Invoked della fixture, 4000 nodi, un thread): su 5 mani (bench inv-r1, r5, r6, hunt-invoked-0, -1) le prime 5 board sono identiche a `main` con la penalità 100, con `cut` e con `HIDDEN`; contatori `struck`: idle 0, negatori 1-68, nascosti 1-213. Memoria a 40 000 nodi su hunt-invoked-1 (11 602 board): 886 → 826 MB (−7%), stesse board, stesso tempo. Livello 1 di hunt-invoked-1 contro Invoked: identico, 0 rifiuti. Test della libreria verdi; `lib.rs` dell'app non compilato qui (mancano le librerie GTK): legge solo `&b.state`, che con `Arc` si deref.

**Cosa resta da provare (prossima sessione).**
1. `paired_test.py` sui 41 deck MDM, `--old` `main` contro il branch, con la penalità 100 e con una piccola (per esempio `DUELFORGE_STRUCK=3`), con e senza `HIDDEN`; 4 thread contro 1 su 20 mani. Si tiene la variante che passa; le altre si scrivono «misurate, non tenute» e si tolgono le variabili d'ambiente.
2. Punto 5: `probe-line --vs` (livello 1, senza `--moves`) sulle board del banco oggettivo contro i 3-6 avversari più giocati: quota di rigiochi con rifiuti (`rejected` > 0) e righe che cambiano `hand_trap_chance`; nessun cambio dove i rifiuti sono 0. La prova equa sulle 406 board non serve: misura senza `--vs`, che il punto 5 non tocca.
3. Punto 6: il −7% è poco. La memoria che resta è soprattutto la `Line` clonata per nodo (`Arc::new(line.clone())`, O(profondità)) e quella di ogni board; da misurare con un profilo dello heap prima di cambiare `Line` (tocca anche `lib.rs`).

## 11. Esito dei punti 3, 5 e 6 (24/10)
*Branch `claude/project-thread-az5qgt`. 40 liste MDM del 24/10 (Maliss e Mathmech incompleti: carte nuove non ancora nel database delle carte), 92 mani (banco e `combo_hunt.py --seed 7`), 4000 nodi, un thread, `--old` `main` f3c5c62.*

**Punto 3: misurato, non tenuto.** Le quattro varianti (penalità 100 o 3, con e senza `HIDDEN`) danno lo stesso risultato: 91 mani pari, 0 meglio, 1 peggio, `hunt-labrynth-0` −19,8% (1,59 → 1,28), quindi REJECTED. La causa: con il taglio la ricerca su quella mano è esaurita a 1804 nodi; senza, l'albero è più grande, e la board «Lovely Labrynth, Trap Trick, Welcome Labrynth, Dominus Spark» viene raggiunta prima da una linea di 17 scelte, che la misura valuta 0,93 (`floodgates` 0,13 contro 0,93), e la linea di 19 scelte non la sostituisce più. Anche a 8000 e 20 000 nodi resta 1,28: è il difetto del rappresentante (punto 1), che l'ordine diverso rende visibile. Il motivo del punto 3 (stesse board con 1 e 4 thread) non regge: con 4 thread contro 1, su 20 mani, ne cambiano 7 sia su `main` sia con il punto 3. Quindi il taglio resta com'era e le variabili `DUELFORGE_STRUCK*` sono tolte; resta in `DUELFORGE_SEARCH_STATS` il contatore `cut` delle risposte tolte da ciascun insieme (Labrynth: 29 handtrap, 21 bersagli nascosti).

**Punto 5: tenuto.** `probe-line --vs` (livello 1) su 32 board (16 mani, le prime 2 board) contro i 6 deck più giocati di MDM (Elfnote, Dracotail, Invoked, Branded, Elfnote Kewl Tune, DoomZ), `main` contro il branch: 192 righe, 28 con almeno un rifiuto, sempre in un rigioco con handtrap (66 momenti su 10 394, ciascuno con un solo rifiuto, tutti già «ferma»); il rigioco senza handtrap non ne ha mai. Nessuna `hand_trap_chance` cambia, né nelle righe con rifiuti né in quelle senza. La soglia di 2 non scatta mai: oggi il conteggio è informazione (nel JSON), non un cambio di verdetto. In 12 righe il rigioco senza handtrap è `diverged` (già su `main`, ora si vede).

**Punto 6: tenuto lo stato condiviso, chiarito dove sta la memoria.** VmHWM a 40 000 nodi, `main` → branch, stesse board e stessi nodi: hunt-invoked-1 941 → 909 MB (−3%), hunt-sky_striker-0 508 → 462 MB (−9%), hunt-snake_eye_rescue_ace-0 1045 → 847 MB (−19%); tempi uguali (68/70, 31/32, 53/52 s). Il profilo dello heap (massif, Sky Striker, 40 000 nodi, 6861 board) vede solo **24 MB** di memoria Rust al picco, metà il database delle carte: board, `Line` e coda non arrivano all'1%. L'ipotesi del §10 (la `Line` clonata per nodo) è quindi sbagliata: il resto del picco è memoria del motore, allocata con mimalloc fuori da `malloc` (oggetti Lua dei duelli, le arene delle copie dei worker, `take_copies`, e i duelli del turno di prova, `OCG_CreateDuel` da `Prober::run_with`). Massif con `--pages-as-heap` conta lo spazio riservato (4 GB per arena), non quello residente: il prossimo passo è contare i byte usati delle arene (`df_arena_used`) per copia, insieme all'arena nel turno di prova.

**Controlli.** Versione finale contro `main`: 92 mani pari, 0 meglio, 0 peggio, ACCEPTED. La misura non cambia: `probe-line` senza `--vs` sulle stesse linee dà uscite identiche a `main` (12 board, a parte i tempi), quindi la prova equa sulle 406 board non è toccata. Test della libreria e `search_regression` verdi, clippy pulito.

