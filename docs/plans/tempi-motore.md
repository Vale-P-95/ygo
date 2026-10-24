# Tempi del motore: dove si spreca, cosa si può togliere

*Thread «Abbattere i tempi di calcolo del motore», 24/10/2026, branch `claude/project-thread-jnnjz3` da `main` 96de01d. Richiesta di Valerio: trovare dove la ricerca delle linee spende tempo in azioni inutili o ripetute che non cambiano il destino della linea, cercare tecniche numeriche e statistiche per abbatterlo, e fare la prima se dà board identiche.*

## In breve
- **La ricerca vera è il 14% del tempo.** Il turno di prova (la misura delle board, compresi i giri di apprendimento) è l'82%. Le idee della ricerca (B1-B3, ordini scambiabili, chiave dei nodi) lavorano sul 14%: per questo davano poco.
- **Dentro il turno di prova** il 62% del tempo è nelle corse «usa questa risposta a questa finestra». Due sprechi grandi:
  1. **Giri di selezioni senza fine.** In Fur Hire ed Elfnote molte corse restano chiuse in una selezione che torna al punto di partenza (scegli il Mostro Fusione, scegli i materiali, si torna alla scelta del Fusione) per 1900 volte, fino al tetto di 4000 decisioni, e poi la corsa fallisce comunque. Fur Hire passava l'82% del suo tempo lì.
  2. **Il turno dell'avversario rigiocato fino alla finestra** a ogni corsa: nei deck normali è circa metà del costo di quelle corse.
- **Fatte tutte e due, con uscite identiche** a `main`: 19 mani del banco di generalità 466 → 337 s (−28%; Fur Hire 167 → 22 s, Elfnote 53 → 22 s); `paired_test.py` su 91 mani senza giri di selezioni 91 pari, tempo −13%.
- **Il resto** è in ordine di guadagno sul costo nella tabella finale; la prima da fare dopo è la risposta giusta alle selezioni che girano (cambia la misura, va giudicata con la prova equa).

## Come si è misurato
- **Mani:** 19 del banco di generalità (`tests/fixtures/generality/sets.json`): un deck di taratura ogni cinque (9) e un deck fresco ogni due (10), la mano 0 di ognuno estratta come in `generality.py`. Come `generality.py run`: 4000 nodi, un thread, `--top 10 --probe --seed 7`, quattro mani alla volta.
- **Strumenti:** `DUELFORGE_SEARCH_STATS` e `DUELFORGE_DEBUG_PHASES` (già nel codice); due contatori provvisori, non nella PR: tempo e decisioni di ogni corsa del turno di prova per tipo di politica (prima e dopo la prima risposta usata, corse ripetute uguali) e tempo di `Duel::advance` per tipo di decisione e giocatore; callgrind su Phantom Knights (1000 nodi) con i simboli.
- Il rumore fra due giri con quattro mani alla volta è grande sulle singole mani (fino al ±30% quando la macchina fa altro); le mani dubbie si sono rifatte da sole, una alla volta.

## Dove va il tempo (`main`, 19 mani, 528 s)
| Parte | Secondi | Quota |
|---|---:|---:|
| Ricerca: discese (replay compreso, 47 s) | 73,5 | 13,9% |
| Giri di apprendimento durante la ricerca | 60,6 | 11,5% |
| Giro di apprendimento finale | 68,6 | 13,0% |
| Fasi dopo la ricerca (`confirm_best`, varianti di zona) | 10,1 | 1,9% |
| Misura delle prime 10 board | 241,1 | 45,6% |
| Valori delle carte, avvio, stampa | 74,4 | 14,1% |

**Le corse del turno di prova** (19 mani, giro con il contatore: 466 s, di cui 382 s nelle corse):

| Corsa | Numero | Secondi | Note |
|---|---:|---:|---|
| Una risposta a una finestra (`UseAt`) | 18 887 | 290 | 62% del totale; 172 decisioni a corsa (110 nei deck normali, 550 in Fur Hire ed Elfnote) |
| Senza risposte (`Pass`) | 4 010 | 41 | la prima di ogni linea rigioca anche il tuo turno |
| Contro un altro avversario (breaker, risposta alla tua interruzione) | 5 156 | 31 | |
| Una risposta quando può (`Use`) | 2 292 | 14 | |
| Tutte insieme (`UseAll`) | 1 225 | 7 | |

- Nei deck normali la parte di una corsa `UseAt` prima della finestra è il 40-50% del suo tempo (48 decisioni su 110).
- Corse uguali ripetute (stessa linea, politica e contesto, anche fra giri di apprendimento e misura finale): 23 s, il 4%.

**Nel motore** (callgrind, Phantom Knights): l'83% è in `OCG_DuelProcess`, due terzi in Lua. Calcolare le azioni del giocatore di turno (`IdleCommand`) è il 41% di tutto, quasi tutto per controllare se ogni carta si può Evocare Specialmente (`is_special_summonable`, 35%); le finestre di catena (`QuickEffect`) il 23%. Il nostro codice (stato, azioni, osservatore) resta sotto il 10%. Per tipo di decisione (12,2 s di `advance` su 21 s):

| Decisione | Chiamate | Secondi |
|---|---:|---:|
| azioni dell'avversario nella sua Main Phase | 22 110 | 3,75 |
| tue finestre di catena nel turno avversario | 52 808 | 2,29 |
| finestre di catena dell'avversario | 51 395 | 1,50 |
| tue azioni nel tuo turno | 6 845 | 1,25 |

In Fur Hire invece 312 659 selezioni tue nel turno avversario costano 136,5 s, l'82% della mano; in Elfnote 625 815 selezioni costano 27,7 s.

## Gli sprechi, uno per uno
1. **Giri di selezioni.** GMX 55th Experiment Report, usato come risposta, chiede il Mostro Fusione e poi i materiali; la nostra scelta dei materiali non chiude la selezione e il motore torna alla scelta del Fusione. Le nostre risposte sono sempre le stesse, quindi il giro si ripete fino al tetto di 4000 decisioni (circa 0,8 s a corsa) e la corsa fallisce: la finestra non conta. Nessuna carta per nome: succede a ogni selezione che torna indietro con le stesse risposte.
2. **Il prefisso della finestra.** Tutte le corse che usano una risposta a una finestra giocano lo stesso turno dell'avversario fino a lì; con B13 ripartivano dall'inizio del turno avversario.
3. **Corse uguali** fra i giri di apprendimento e la misura finale: 4%.
4. **Le azioni dell'avversario nella Main Phase** costano 170 µs l'una perché il motore controlla l'Evocazione di ogni carta del suo Extra Deck (il kit ne ha 15, 7 Gaia Saber e 7 Flame Swordsman, messe per avere un Extra Deck «vero»). Da misurare quanto pesano; ridurle cambia la misura.
5. **I valori delle carte** si rifanno a ogni avvio della CLI (3-4 s a mano, uguali per tutte le mani dello stesso deck): nei banchi sono il 14%; nell'app sono già in memoria per la sessione.

## Le due correzioni fatte (`probe.rs`)
**Copie del duello alle finestre del turno avversario.** Una corsa che usa una risposta solo alla finestra W gioca, fino a W, come qualunque corsa che non ha usato niente prima. La prima corsa che arriva a W ne lascia una copia (arena di B13, con tutto lo stato della corsa); le altre ripartono da lì, o dalla copia più vicina prima di W sulla stessa strada. È esatto per costruzione, con due cautele:
- vale solo per le corse con tutti i passi del kit e senza ciò che cambia le tue risposte prima di W (valore delle risposte, risposta dell'avversario ai tuoi link, linea dell'avversario del livello 2);
- le tue risposte non di finestra preferiscono le carte che la corsa usa (`your_answer`): una copia non serve una corsa che usa una carta offerta in una di quelle risposte prima di W (`sensitive`).

Le copie si fanno solo dove servono: farle a ogni finestra durante la corsa senza risposte costava più di quanto rendeva (le corse `Pass` raddoppiavano). Tetto di 16 copie per board (`DUELFORGE_WINDOW_COPIES`): con 8 Phantom Knights va meglio di 1 s, Regenesis peggio di 3 s; con 32 nessun guadagno in più. Memoria: +65-100 MB al picco (Phantom Knights 233 → 336 MB). Sulle corse `UseAt` di Phantom Knights: 9,8 → 6,7 s (−32%). `DUELFORGE_COPY_CHECK=n` controlla anche queste copie.

**Taglio dei giri.** Nel turno avversario, se la stessa domanda torna 64 volte senza nessun evento in mezzo, la corsa finisce con lo stesso errore del tetto di 4000 decisioni (`DUELFORGE_ROUND_CUT`, 0 per spegnerlo).
- Prima si è provata la versione dimostrabile: fermare la corsa quando il duello torna identico byte per byte (arena, lato Rust e stato della corsa). Non scatta mai: a ogni giro il motore registra gruppi nuovi in Lua, quindi la memoria non torna uguale; e i confronti costavano (Fur Hire 167 → 292 s). Tolta.
- La regola delle 64 domande non è una prova: una sequenza vera di 64 domande uguali senza eventi la taglierebbe. Su tutte le mani provate le uscite sono identiche (sotto); una corsa tagliata per sbaglio darebbe una finestra in meno, mai un crash.

**Risultati.**
- **19 mani del banco di generalità** (quattro alla volta): 466 → 337 s (−28%). JSON identici in 19 mani su 19, a parte i tempi; con `DUELFORGE_COPY_CHECK=1` (ogni corsa da una copia rigiocata in un duello nuovo e confrontata) 0 differenze. Le due mani con i giri, rifatte da sole: Fur Hire 166,7 → 21,7 s, Elfnote 53,2 → 22,0 s. Le mani normali rifatte da sole: Phantom Knights 20,6 → 17,6 s, Regenesis 34,2 → 25,4 s, Blitzclique 13,4 → 12,0 s, Sacred Beasts 10,5 → 10,2 s; nel giro a quattro alcune sembravano più lente (Blitzclique 13,7 → 24,2 s), ma la macchina compilava in quel momento.
- **`paired_test.py`** (14 mani del banco e 77 di `combo_hunt --seed 7`, 4000 nodi, un thread, stesse condizioni per i due lati): 91 pari, 0 meglio, 0 peggio, ACCEPTED; tempo delle run 1472 → 1282 s (−13%), ricerca 496 → 447 s (i giri di apprendimento). JSON identici in 87 mani; nelle altre 4 cambia solo l'ultima cifra di `countered`, come fra due giri di `main` (somme fatte in un ordine diverso). Nessuna di queste mani ha i giri di selezioni: il −13% è delle copie alle finestre. La mano più lenta: radiant_typhoon_zoodiac-1 5,4 → 6,4 s; la più veloce heros-1 19,3 → 5,2 s.

## Tecniche, in ordine di guadagno sul costo
| # | Tecnica | Guadagno atteso | Costo e rischio | Stato |
|---|---|---|---|---|
| 1 | Taglio dei giri (rilevamento di cicli) | −87% Fur Hire, −59% Elfnote; 0 nei deck senza giri | 30 righe; non dimostrabile, uscite identiche sul campione | **Fatto** |
| 2 | Copie alle finestre (checkpoint del prefisso comune, come Forge e ygorl) | −30% delle corse `UseAt`, −5/−20% a mano normale | 250 righe sopra B13; esatto | **Fatto** |
| 3 | Rispondere bene alle selezioni che girano (chiudere la selezione o annullarla) | quanto il taglio, e in più la risposta conta davvero (oggi la finestra si perde) | piccolo in `your_answer`; cambia la misura di quei deck: prova equa e `paired_test.py` | Prossimo |
| 4 | Valori delle carte su disco, per deck, script e kit | −3/4 s a mano: −14% sui banchi (4 run per deck nel banco di generalità), 0 nell'app | invalidazione (versione degli script, del kit e del motore) | Da fare per i banchi |
| 5 | Memoria delle corse (tabella di trasposizione sulle corse: stessa linea, politica, contesto) | 4% | piccolo; chiave completa obbligatoria | Da fare, piccolo |
| 6 | Extra Deck del kit più leggero (meno copie di mostri con procedura di Evocazione) | fino a un terzo delle azioni dell'avversario, da misurare | cambia la misura: prova equa | Da misurare |
| 7 | Misura finale delle 10 board su più thread | tempo di attesa ÷3 su 4 core, lavoro uguale | piccolo; nei banchi i core sono già pieni | Per l'app |
| 8 | Statistiche: smettere di provare finestre quando il risultato di una risposta non può più cambiare; corse a eliminazione (racing, Hoeffding) per scegliere le board da misurare | da misurare; B14 fa già la misura economica | cambia la misura | Dopo la 3 |
| — | Ordini scambiabili (sleep set), selezioni come un passo, ordine dei fratelli, tabella a penalità minima, chiave dei nodi | ≤ 3% del tempo totale: la ricerca è il 14% | — | Già misurati e chiusi (`velocita-ricerca.md`) |

Le fonti per 2, 4 e 5 sono in `revisione-bibliografica-2.md` §4.4 (Jinnai & Fukunaga: cache delle simulazioni; Forge: snapshot con controllo che la copia valuti come l'originale). Il rilevamento di cicli è la tecnica standard dei model checker (stato già visto = ciclo); qui lo stato del motore non è confrontabile a basso costo, da cui la regola sulle domande.

## Protocollo
- `paired_test.py` sopra: 91 pari, ACCEPTED.
- **Prova equa sulle 406 board:** la misura di ogni board è la stessa (JSON identici su 19 + 91 mani, cioè circa 1100 board misurate, e il controllo di ogni copia), quindi la prova equa non cambia, come per B13. Le linee del giro A1b non sono in questo container per rifarla.
- `cargo fmt`, `clippy -D warnings`, `cargo test --release` e `search_regression`: tutti verdi.

## Prossimo passo
La 3: quando una selezione torna al punto di partenza, rispondere in un altro modo (un'altra combinazione di materiali, o annullare l'effetto) invece di ripetere la stessa scelta. Giudice: prova equa sulle 406 board e `paired_test.py`; il guadagno di tempo c'è già con il taglio, quindi la 3 si tiene solo se migliora o non cambia le coppie giuste.
