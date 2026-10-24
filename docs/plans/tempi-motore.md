# Tempi del motore: dove si spreca, cosa si può togliere

*Thread «Abbattere i tempi di calcolo del motore», 24/10/2026, branch `claude/project-thread-jnnjz3` da `main` 96de01d. Richiesta di Valerio: trovare dove la ricerca delle linee spende tempo in azioni inutili o ripetute che non cambiano il destino della linea, cercare tecniche numeriche e statistiche per abbatterlo, e fare la prima se dà board identiche.*

## In breve
- **La ricerca vera è il 14% del tempo.** Il turno di prova (la misura delle board, compresi i giri di apprendimento) è l'82%. Le idee della ricerca (B1-B3, ordini scambiabili, chiave dei nodi) lavorano sul 14%: per questo davano poco.
- **Dentro il turno di prova** il 62% del tempo è nelle corse «usa questa risposta a questa finestra». Due sprechi grandi:
  1. **Giri di selezioni senza fine.** In Fur Hire ed Elfnote molte corse restano chiuse in una selezione che torna al punto di partenza (scegli il Mostro Fusione, scegli i materiali, si torna alla scelta del Fusione) per 1900 volte, fino al tetto di 4000 decisioni, e poi la corsa fallisce comunque. Fur Hire passava l'82% del suo tempo lì.
  2. **Il turno dell'avversario rigiocato fino alla finestra** a ogni corsa: nei deck normali è circa metà del costo di quelle corse.
- **Fatte tutte e due, con uscite identiche** a `main`: 19 mani del banco di generalità 466 → 337 s (−28%; Fur Hire 167 → 22 s, Elfnote 53 → 22 s); `paired_test.py` su 91 mani senza giri di selezioni 91 pari, tempo −13%.
- **Il resto** è in ordine di guadagno sul costo nella tabella finale; la risposta giusta alle selezioni che girano è fatta (sotto): era una risposta «Annulla» scelta per sbaglio.

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

## La terza correzione: le selezioni non si annullano più (24/10)
*Thread «Accelerare ancora il motore», branch `claude/project-thread-gw025y` da `main` 1b25f3e.*
- **Il perché del giro.** Non era la scelta dei materiali a non chiudere la selezione: era la nostra risposta che sceglieva «Annulla». `forward_by` (le selezioni a una carta alla volta) prendeva come carte da scegliere le prime `selectable.len()` risposte della lista; ma la lista offre una sola volta le copie della stessa carta (due Fur Hire nel Deck sono una risposta), quindi è più corta, e «Annulla» (o una carta da togliere) finiva dentro. Senza carta, «Annulla» ha valore, Livello e ATK zero: per la regola «la più debole prima» vinceva lei. L'effetto tornava alla scelta del Mostro Fusione, e così via fino al taglio delle 64 domande.
- **La correzione.** Le carte da scegliere sono solo le risposte che prendono una carta di `selectable` (indice nella risposta). Vale per tutte le selezioni di questo tipo nel turno di prova: le tue risposte, quelle dell'avversario del kit e quelle del deck avversario del livello 2 (`matchup.rs` usa la stessa funzione). Un test (`picking_goes_forward_with_copies_offered_once`) riproduce il caso con due copie.
- **Giri rimasti** (63 mani `quick`, contatore provvisorio sul taglio, non nella PR): su `main` 4 mani con 127 tagli (Magistus Witchcrafter 67, Dracotail 28, Elfnote Kewl Tune 16, Shaddoll 16); dopo, 0.
- **Banco congelato, classe M** (`banco.py run … --set full --jobs 4 --measure-cli <branch>`, 34 minuti): 185 uscite su 193 identiche; delle 8 cambiate, rimisurate con la CLI del branch, 1 meglio (Tearlaments, mano 1: +6,2%) e 7 pari; nessuna peggiore, nessuna run rotta; media +0,03% (0,00% a +0,10%); freschi 60 pari; tempo 0,991 (0,986-0,996). `judge --measure-cli`: ACCEPTED. `coverage` su Fur Hire (24 mani): una mano 7,1 → 7,3, le altre uguali.
- **Tempo.** Il guadagno vero l'aveva già dato il taglio: un giro tagliato a 64 domande costava poco. Qui il −1% è il resto; il valore della correzione è che quelle risposte ora si giocano, invece di perdere la finestra.
- **Verità v1** non c'è ancora (`regole-motore.md` §4): per la regola, una PR della misura la decide il proprietario.

## Tecniche, in ordine di guadagno sul costo
| # | Tecnica | Guadagno atteso | Costo e rischio | Stato |
|---|---|---|---|---|
| 1 | Taglio dei giri (rilevamento di cicli) | −87% Fur Hire, −59% Elfnote; 0 nei deck senza giri | 30 righe; non dimostrabile, uscite identiche sul campione | **Fatto** |
| 2 | Copie alle finestre (checkpoint del prefisso comune, come Forge e ygorl) | −30% delle corse `UseAt`, −5/−20% a mano normale | 250 righe sopra B13; esatto | **Fatto** |
| 3 | Rispondere bene alle selezioni che girano (chiudere la selezione o annullarla) | il tempo l'aveva già preso il taglio (−1% sul banco); in più la finestra conta davvero | 10 righe in `forward_by`; cambia la misura di 8 mani su 193 | **Fatto** (sotto) |
| 4 | Valori delle carte su disco, per deck, script e kit | −3/4 s a mano: −14% sui banchi (4 run per deck nel banco di generalità), 0 nell'app | invalidazione (versione degli script, del kit e del motore) | **Respinta** da Valerio il 24/10 (sotto) |
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

## Decisione del 24/10: niente valori delle carte su disco
- **Respinta la tecnica 4.** Decisione di Valerio: il motore valuta sul momento, e in modo indipendente dal meta, le linee più forti da una mano o da una carta. Niente salvato su disco fra un avvio e l'altro, come per le linee precalcolate (`coordinamento.md`, `progetto-claude.md`).
- **Cosa resta possibile.** I valori in memoria nello stesso processo, come già nell'app; per i banchi, una CLI che giochi tutte le mani di un deck in un solo processo (`prossimi-passi-motore.md`).

## Valori delle carte una volta per deck (24/10, classe E)
- **I valori dipendono dal seme.** `card-values` con i semi 1, 2 e 3 sui 63 deck del banco: in 49 deck cambiano da 2 a 14 voci su 200-350 (Maxx "C" in Code Talker vale 0 col seme 1 e 0,09 col seme 2; alcune voci esistono solo con un seme, perché il turno di prova fallisce). Il calcolo è deterministico; i handtrap trovati sono gli stessi con i tre semi. Calcolarli una volta per deck con uscite identiche quindi non si può.
- **Provato il seme fisso** (valori col seme 1, come la Fucina e il Combo Lab dell'app): tempo del banco ×0,76 (0,737-0,782), ma 16 mani su 193 cambiano, tutte con seme 2 o 3, e `judge` dice REJECTED: 3 meglio, 7 peggio, 5 mani oltre −5% (Branded, Code Talker, Mitsurugi Yummy ×2, HERO). Non tenuto.
- **Tenuto: una CLI per deck.** `combos --cases <file>` gioca più mani dello stesso deck in un processo (riga: id, seme, mano, JSON, linee); il motore si carica una volta, i valori restano nella cache in memoria per seme (nessun file su disco). `banco.py run` usa un processo per deck quando la CLI lo sa fare (lo chiede a `combos --help`) e dà a ogni mano il suo tempo (la prima paga anche il caricamento); con la CLI di `main` gioca una mano per processo come prima. Con `--shard` i runner si dividono i deck, non le mani.
- **Banco:** `banco.py same`, set `full`: 193 mani, 0 uscite diverse, ACCEPTED; tempo ×0,969 (0,954-0,981).
- **Cosa resta.** Il −24% c'è solo se le mani di un deck condividono i valori, cioè con un banco `v2` che abbia lo stesso seme per le mani di un deck o i valori di un seme fisso, giudicato come R con la taratura del §6 di `regole-motore.md` (proposta 5 di `prossimi-passi-motore.md`).

## Prossimo passo
La 5 (memoria delle corse, −4%, classe E); per l'app la 7 (misura finale delle 10 board su più thread). La 4 non si fa (decisione sopra). Il taglio dei giri resta come rete di sicurezza: su 63 mani non scatta più.
