# Combo Lab: cosa non vede la misura, come guidare la ricerca, cosa prendere dalla letteratura

*22/10/2026. Rapporto di una ricerca multi-agente (letteratura e audit su 37 deck di Master Duel Meta), per il proprietario e il coordinatore. Gli errori trovati dalla critica di completezza sono già corretti nel testo; le lacune che ha segnalato sono in fondo (§7). Il piano che ne segue è in `ricerca-piano-critico.md`.*

Il rapporto si basa su tre fonti:
- quattro ricerche bibliografiche, ognuna controllata da un verificatore (le idee bocciate non compaiono);
- un audit della misura delle board su 37 deck del meta;
- il piano `docs/plans/ricerca-piano-critico.md`.

Molte fonti sono state lette solo dagli abstract, perché la rete era bloccata. Il codice di DuelForge è stato letto direttamente.

## In breve
- **Aeon.** Il problema non è solo la ricerca: dietro c'è una catena di cinque cecità, e la prima sta nella misura.
  - Oggi togliere 1 carta dall'Extra Deck avversario vale quanto toglierne 3.
  - La stima della ricerca non vede né l'una né l'altra.
- **Ricerca per obiettivo.** È pianificazione su un grafo implicito.
  - Gli algoritmi «da labirinto» (A*, IDA*, ricerca bidirezionale) si adattano male.
  - Si adattano quelli dei pianificatori che lavorano su un simulatore chiuso: distanza rilassata sulla mappa dei collegamenti, azioni utili, novità, più code alternate.
- **Stockfish.** Alcune cose le facciamo già: tabella delle trasposizioni, linea principale per prima, penalità per le deviazioni. Mancano quattro cose:
  - l'ordinamento delle mosse imparato durante la ricerca (history);
  - la stima tarata sui dati (Texel);
  - la correzione della stima mentre la ricerca impara;
  - i test appaiati per accettare una modifica.
- **Velocità.** La leva più grande è nel motore, non nella ricerca: salvare e ripristinare lo stato del duello invece di rigiocare la linea dall'inizio. Un solver open source sullo stesso motore lo fa già. La misura A2 (PR #67) lo conferma: il replay vale il 43% del tempo della ricerca.
- **Obiettivi più ricchi** (Magie e Trappole coperte, Cimitero, bandite, carte tolte all'avversario): poco lavoro nel motore, più un cambio di contratto `lib.rs`/`api.ts`.
- **Correzione rapida da misurare.** A parità di priorità, la coda gioca per prima l'alternativa *meno* preferita del nodo più profondo: Passa, No, Annulla, poi i Set.
  - È nel codice: `Edge::cmp` a pari priorità prende il più nuovo, e `dive.left` è riempito nell'ordine di `dive_order`.
  - Quanto pesa va misurato con A2.

---

## 1. Effetti che il motore non vede

### Le categorie confermate
Ordinate per numero di deck del meta toccati (su 37). «Oggi» dice cosa conta oggi la misura finale o la stima della ricerca.

| # | Categoria | Deck | Oggi | Impatto | Esempi | Come misurarlo, in generale | Sforzo |
|---|---|---|---|---|---|---|---|
| 1 | Risposte a un board breaker diverse dalla negazione: floater, punitori, protezioni sostitutive | 30 | non misurato | alto | Golden Cloud Beast - Malong, Diabellstar the Black Witch, Muckraker From the Underworld | Per ogni breaker, una run «react» accanto a quella con Passa: si risponde sì ai propri trigger opzionali e alle sostituzioni (`EffectYesNo`, `SelectChain` con trigger). Resilienza = max(Passa, react, negazione). | medio |
| 2 | Evocazioni rapide, estensori e schivate nel turno avversario, soprattutto per ricostruire dopo un breaker | 28 | in parte | alto | Lacrima the Crimson Tears, I:P Masquerena, The Book of the Law | La stessa run «react», con ogni effetto offerto dopo l'attivazione del breaker. Un piccolo valore a un corpo che resta fino al turno 3 anche se non ferma niente. | medio |
| 3 | Assicurazioni contro le interazioni nel tuo turno: protezione della combo, carte che pagano solo se l'avversario interrompe | 27 | non misurato | medio | Crossout Designator, Triple Tactics Talent, Magical Meltdown | Esposizione letta dal motore: a ogni punto di strozzatura si provano davvero le handtrap standard. Poi una breve ricerca ripresa dal punto interrotto (`resume()` con pochi nodi) trova le linee di riserva. | grande |
| 4 | Interruzioni che hanno bisogno di altre tue carte: partner, numero di Tipi o Attributi, costi pagati rivelando carte | 24 | in parte (la misura finale sì, la stima no) | medio-alto | Lovely Labrynth of the Silver Castle, Artmage Finmel, Tri-Brigade Mercourier | Tre interventi: (1) valori condizionati a coppie, misurati togliendo una carta alla volta; (2) handtrap cercate anche su un campo piazzato (`Spot::HandWithField`); (3) mano di misura con un Normale per Attributo. | medio |
| 5 | Effetti che dipendono da come la carta è stata costruita: materiali, Attributi, contatori, equipaggiamenti, tipo di Evocazione | 22 | in parte (la misura finale sì, la stima no) | alto per la ricerca | Hyperinvoked Aeon, Fiendsmith's Desirae, Borreload Savage Dragon | Valori delle carte per «firma di costruzione» letta dal motore: numero di materiali, Attributi distinti, contatori, Link Rating equipaggiato, tipo di Evocazione. Se la firma non è nota, si usa il valore generico. | medio |
| 6 | Protezione dei pezzi che non interrompono, e non bersagliabilità | 22 | in parte | medio | Mudragon of the Swamp, Cyberse Wicckid, Stardust Sifr Divine Dragon | `protection()` entra nel punteggio, oggi è solo un rapporto. Una rimozione che bersaglia tra i breaker (oggi solo Impermanence, peso 1 su 10,2). | piccolo |
| 7 | Blocchi su giocate che il kit non fa mai: Evocazioni Fusion, Synchro, Xyz, Rituali e Pendulum, 2+ Evocazioni insieme, Magia che Evoca, Battle Phase | 19 | in parte | medio | Dimensional Barrier, Clown Crew Diabolo, Blue-Eyes Spirit Dragon | Azioni generiche nuove nel kit, ognuna con la sua prova; pesi ricalibrati sul banco. | grande |
| 8 | Prendere il controllo dei mostri avversari, equipaggiarli o spostarli nella loro Zona Magie e Trappole | 16 | valutato male (0,4 o 0,7) | medio | Invoked Sorath, Snake-Eyes Vengeance Dragon, Mementotlan Mace | Nell'Observer: carta avversaria portata sul tuo campo = almeno Distruzione; portata nella sua Zona Magie e Trappole = Bandita. Poche righe in `probe.rs:980-990`. | piccolo |
| 9 | Risorse per il turno dopo: preparazione nella Fase Finale, carte sul campo che agiscono al tuo turno, carte guadagnate | 15 | in parte | medio | Albion the Branded Dragon, Sengenjin Wakes from a Millennium, Blaze Accelerator | `turn_three` conta anche gli effetti dal campo e le Evocazioni dalla Zona Magie e Trappole. `gained` pesato per ruolo. Fase Finale provvisoria giocata anche nei giri di apprendimento. | piccolo |
| 10 | Il tuo Cimitero e le tue bandite nella stima della ricerca | 9 | in parte | medio | Promethean Princess, Bestower of Flames, Cyberse Desavewurm, Sinful Spoils of Betrayal - Silvera | `Spot::Grave` e `Spot::Banished` in `CardValues`; `learn_round` non li salta più (`search.rs:1774`). | piccolo |
| 11 | Togliere carte dal Cimitero, dal Deck o dall'Extra Deck avversario nel suo turno | 9 | in parte (Lock 0,7) | basso-medio | White Knight of Dogmatika, Dracotail Sting, Keldo the Sacred Protector | L'Observer gradua per destinazione (Bandita o nel Deck); il resto va nella metrica di attrito della riga 12. | piccolo |
| 12 | Togliere risorse all'avversario nel tuo turno: Extra Deck, Deck, mano, mostri dati al suo campo | 8 | in parte, con un artefatto | **alto** | Hyperinvoked Aeon, Dogmatika Maximus, Kewl Tune Clip | Metrica «risorse tolte all'avversario», Extra Deck avversario realistico e termine in `quick_score` (dettagli sotto). | medio |
| 13 | Effetti di zona, colonna e zona centrale | 7 | in parte | basso-medio | Elfnote Tinia, Arktos XII - Chronochasm Vaylantz, The Phantom Knights of Rusty Bardiche | `zone_variants` prova ogni Zona Mostri Principale per le board migliori; zone del kit variate tra le run. | medio |
| 14 | Rimozione in risposta che non ferma l'effetto già attivato | 6 | in parte | basso | Invoked Augoeides, S:P Little Knight, Fydraulis Harmonia | Contare i corpi avversari tolti anche quando nessuna prova fallisce, a valore parziale (circa il valore dell'effetto × 0,5). | piccolo |
| 15 | Rimuovere Magie e Trappole avversarie sul campo | 5 | non misurato | basso-medio | Silhouhatte Rabbit, Artmage Graflare, Elfnote Fortuna | Nel kit una Magia Terreno o Continua e un Set, con la loro prova. | medio |
| 16 | Danno e Life Point | 4 | quasi non misurato (il costo conta `lp_paid / 4000` con peso −0,1; il turno di prova non legge gli LP) | basso | Volcanic Emperor, Deception of the Sinful Spoils, Spiritual Fire Art - Kurenai | Metrica «danno» = LP tolti / 8000, con un segnale di danno letale. | piccolo |
| 17 | Interruzioni con una tassa che l'avversario può pagare | 3 | in parte (il kit paga sempre) | basso | Ame no Murakumo no Mitsurugi, Fairy Tail - Luna | Giocare entrambe le risposte dell'avversario e tenere la peggiore per te; nelle run, mano del kit realistica di 5-6 carte. | piccolo |

Due casi non sono problemi della misura:
- i trigger sulle Evocazioni dall'Extra Deck avversario (Guiding Quem, Despian Luluwalilith) sono già misurati;
- Mercurium the Living Quicksilver (101303084) non ha script nel motore. Serve un avviso nei controlli del deck.

### Quattro meccanismi coprono quasi tutto
1. **Run «react» contro i breaker**: righe 1 e 2, i due gruppi più grandi (30 e 28 deck).
2. **Attrito avversario con Extra Deck realistico e correzioni dell'Observer**: righe 8, 11, 12 e 14.
3. **Stima della ricerca più ricca**: righe 4, 5, 9 e 10 (firma di costruzione, Cimitero e bandite, coppie, Fase Finale).
4. **Kit più ampio ed esposizione letta dal motore**: righe 3, 7 e 15. Sono grandi: vanno dopo.

### Il caso Aeon, passo per passo
1. **La stima non vede l'avversario.** `quick_score` legge solo `players[0]` (`search.rs:893-951`), quindi per la frontiera il bando non esiste.
2. **Aeon vale quasi zero nei valori delle carte.** `CardValues` piazza Aeon con 2 materiali generici di riempimento e non gioca i trigger di Evocazione: Aeon vale 0,04 in ogni Spot. Le regole vogliono invece 2 o più mostri Fusion con Attributi tutti diversi, e il bando dall'Extra Deck scatta con 3 o più materiali.
   - Un mostro misurato debole vale 0,3 + 5 × 0,04 = 0,5.
   - Un mostro dell'Extra Deck non misurato vale 2,0.
3. **Il kit dell'avversario ha un solo mostro nell'Extra Deck** (Link Spider, `probe.rs:474-481`). Toglierlo fa fallire la prova ExtraSummon e conta come floodgate da 0,6.
   - Una carta tolta vale quanto un blocco totale dell'Extra Deck; tre carte valgono lo stesso.
   - Visto con la CLI su Dogmatika (board #7 e #8). Va rifatto scrivendo comando, seme e uscita: così com'è non si riproduce.
   - Lo script di Aeon offre il bando dall'Extra Deck solo se il primo bando ha tolto almeno una carta, e dopo un sì/no. Giocando per primi, il primo bando colpisce le tue carte.
   - L'effetto rapido di Aeon (cambio di Attributo) e il trigger di Evocazione condividono il limite di un effetto di Aeon per turno: usare il primo toglie il bando.
4. **Nell'app le due varianti diventano una.** L'app misura una board per (mano, `look_key`), e `look_key` ignora Cimitero, bandite ed Extra Deck (`search.rs:833-855`, `lib.rs:1857-1871).
   - A stima pari, `rank()` preferisce meno carte usate e la linea più corta: vince Aeon a 2.
   - Anche costo ed esposizione penalizzano un po' la linea a 3: +0,1 per ogni mostro dell'Extra Deck speso, +1 per ogni Evocazione in più.
5. **Plausibile, da verificare.** Nel livello 2 e nella run di Super Polymerization l'Extra Deck avversario cambia, ma le risposte della linea si rigiocano per posizione nella lista. Due conseguenze possibili:
   - il bando colpisce carte a caso;
   - la run si desincronizza, il breaker risulta «non applicabile» e la resilienza viene piena.

**Correzione generale (B5a, sotto).** È l'idea del valutatore di Forge per Magic: il punteggio conta le risorse di entrambi i giocatori ([GameStateEvaluator.java](https://github.com/Card-Forge/forge/blob/master/forge-ai/src/main/java/forge/ai/simulation/GameStateEvaluator.java)).
- **Extra Deck realistico per l'avversario.** Circa 15 mostri generici di ogni tipo di Evocazione, Link Spider compreso, nella ricerca e nel turno di prova. Così perdere Link Spider diventa attrito piccolo, non un floodgate.
- **Metrica «risorse tolte all'avversario».** Si legge dagli eventi del tuo turno, che `explain.rs` rigioca già. Ogni carta pesa secondo la pila da cui viene: una carta dell'Extra Deck vale circa 1/15 di un blocco dell'Extra Deck, una carta della mano circa 1 carta su 5-6. Il peso finale si tara in D17.
- **Lo stesso termine in `quick_score`.** È economico: dimensione dell'Extra Deck, bandite e Cimitero di `players[1]` contro l'inizio del turno.
- **`look_key`, o la scelta della board da misurare, distingue l'attrito avversario.**
- **Le scelte nell'Extra Deck avversario si rigiocano per identità della carta**, come fa `follow.rs` (`Picked::of`).
- **Il sì opzionale dello script di Aeon non va mai rifiutato in automatico.** Da verificare quale regola lo rifiuta oggi (`dive_order` o una risposta automatica), con la riga dello script.

---

## 2. La ricerca per obiettivo è un problema di pianificazione

**La domanda del proprietario:** «la ricerca obiettivo ricorda gli algoritmi per i labirinti o la classificazione, sbaglio?». Non sbagli. Il modo giusto di chiamarla è **pianificazione in uno spazio di stati implicito**:
- i nodi sono i momenti di decisione del motore, gli archi sono le risposte;
- il grafo non esiste in memoria: lo genera EDOPro;
- l'obiettivo è un insieme di fatti (carta X in un certo posto).

Rispetto a un labirinto ci sono quattro differenze che contano:
1. **Non si cerca il percorso più corto.** Si cerca la board più forte tra quelle che soddisfano l'obiettivo. La lunghezza della linea in `rank()` serve solo a spareggiare.
2. **Non si torna indietro.** Il motore non gioca gli effetti al contrario, quindi niente ricerca dall'obiettivo verso l'inizio (bidirezionale o per regressione).
3. **Ogni nodo costa millisecondi, non nanosecondi.** Sul banco una discesa rigioca in media 56 risposte per 4 nodi nuovi, e il replay medio costa 5,6 ms (A2, `velocita-ricerca.md`). Gli algoritmi che rigenerano i nodi, come IDA*, moltiplicano proprio il costo che pesa.
4. **L'obiettivo descrive una board parziale**, che molti stati completi soddisfano.

La classificazione c'entra in due punti:
- **classificare le mosse**: «questa risposta sta nelle linee forti?». È una politica imparata (learning to rank);
- **classificare le board**: decidere quale di due board è migliore. È Bradley-Terry, già in `crates/ygo-stats/src/prefs.rs`.

### Quali algoritmi si adattano

| Famiglia | Si adatta? | Perché |
|---|---|---|
| A*, A* pesato, IDA*, bidirezionale MM, pattern database, DeepCubeA ([Korf 1985](https://cse.sc.edu/~mgv/csce580f11/gradPres/korf_IDAStar_1985.pdf), [MM](https://ojs.aaai.org/index.php/AAAI/article/view/10436), [DeepCubeA](https://github.com/forestagostinelli/DeepCubeA)) | no | Ottimizzano il costo del cammino e richiedono predecessori o rigenerazione dei nodi. Si tiene solo l'idea «anytime»: dopo la prima soluzione si continua a migliorarla ([Hansen & Zhou](https://jair.org/index.php/jair/article/view/10489)). |
| Ricerca a discrepanze limitate ([Harvey & Ginsberg](https://ai.dmi.unibas.ch/research/reading_group/harvey-ginsberg-ijcai1995.pdf), [ILDS](https://ai.dmi.unibas.ch/research/reading_group/korf-aaai1996.pdf), [DDS](https://mlanthology.org/ijcai/1997/walsh1997ijcai-depth)) | c'è già | `search.rs` è un best-first con 0,6 di penalità per ogni deviazione dal tuffo. La letteratura conferma la scelta. |
| Distanza dal rilassamento (h_add, h_FF) con azioni utili come operatori preferiti ([FF](https://jair.org/index.php/jair/article/view/10276), [Bonet & Geffner](https://www.cs.uky.edu/~sgware/reading/papers/bonet2001planning.pdf), [Richter & Helmert](https://ojs.aaai.org/index.php/ICAPS/article/view/13345), [Fast Downward lazy search](https://github.com/aibasel/downward/blob/main/src/search/search_algorithms/lazy_search.cc)) | **sì** | È la forma concreta di B4. |
| Novità: Iterated Width e Best-First Width Search ([BFWS](https://ojs.aaai.org/index.php/AAAI/article/view/11027), [codice](https://github.com/nirlipo/BFWS-public), [Atari](https://ijcai.org/Abstract/15/230)) | **sì, come chiave secondaria** | Pensata per simulatori chiusi come il nostro. |
| Più code alternate e coda per tipo ([Röger & Helmert](https://ojs.aaai.org/index.php/ICAPS/article/view/13435), [Type-GBFS](https://ojs.aaai.org/index.php/AAAI/article/view/9036)) | **sì** | Toglie le costanti fisse +3 e +10. |
| Obiettivi parziali con utilità, ricerca anytime ([Benton et al.](https://rakaposhi.eas.asu.edu/sapaps_aij.pdf)) | **sì** | Base per gli obiettivi ricchi (§5). |
| Landmark e agenda degli obiettivi ([Hoffmann et al.](https://strathprints.strath.ac.uk/1873/), [LAMA](https://gki.informatik.uni-freiburg.de/papers/richter-westphal-jair10.pdf)) | più avanti | Dopo la distanza rilassata, e solo come chiave morbida. |
| Ricerca guidata da una politica, LTS e PHS ([Orseau et al.](https://arxiv.org/abs/1811.10928), [PHS](https://ojs.aaai.org/index.php/AAAI/article/view/17469), [LTS+CM](https://www.ijcai.org/proceedings/2023/0624)) | più avanti | Serve prima una politica imparata (§4). |
| Macro-azioni imparate ([Macro-FF](https://arxiv.org/abs/1109.2154), [focused macros](https://arxiv.org/abs/2004.13242)) | più avanti | Estende B7. |
| Riduzione per ordine parziale ([DPOR](https://www.cs.columbia.edu/~junfeng/17sp-e6121/papers/dpor.pdf), [stubborn sets](https://ojs.aaai.org/index.php/ICAPS/article/view/13638)) | solo i sleep set, prudenti | DPOR presuppone una ricerca in profondità; la nostra è best-first a più thread. |

### Cosa adottare, in ordine

**1. Distanza rilassata sulla mappa dei collegamenti (B4).** Un modulo nuovo, `crates/ygo-solver/src/relax.rs`, costruito da `DeckLinks`.
- Fatti: (carta, posto), dove il posto è mano, campo, Magia/Trappola coperta, Cimitero o bandita.
- Azioni rilassate: l'Evocazione Normale più i collegamenti `Search`, `Summon`, `ToGrave`, `Recover`, `Set`, `Material`.
- Costo di un collegamento: 1 − ln(max(used, ε)). Il minimo ε serve perché i collegamenti presi dal testo possono avere `used` = 0.
- In modalità obiettivo la stima diventa `quick_score − λ·h_goal`, al posto del 3·goal_hits fisso.
- **Azioni utili:** le risposte la cui carta è l'origine di un collegamento del piano rilassato vanno prima nel tuffo e prendono il bonus. Oggi `Goal::involves` scatta solo sulla carta obiettivo stessa, quindi i 3-6 passi di preparazione sono al buio.
- Solo per ordinare, mai per potare: la mappa ha buchi. Così il verdetto «impossibile» (`Stop::Exhausted` con `interrupted == 0`) resta valido.
- Limite: il rilassamento non conta i materiali. Aeon a 2 e a 3 materiali sono uguali per lui, quindi serve anche B5a.

**2. Credito all'obiettivo stile AMAF** ([RAVE](https://research.google/pubs/monte-carlo-tree-search-and-rapid-action-value-estimation-in-computer-go/), [GRAVE](https://www.lamsade.dauphine.fr/~cazenave/papers/grave.pdf)).
- Per ogni codice di mossa si stima quanto la sua presenza nella linea alza la probabilità di arrivare a h carte dell'obiettivo: P(colpi ≥ h | mossa nella linea) − P(colpi ≥ h).
- Si usano i colpi parziali, così c'è un segnale prima che una linea raggiunga l'obiettivo intero.
- La mappa fa da stima iniziale, AMAF la corregge durante la ricerca.

**3. Più code** al posto dell'unico `BinaryHeap`:
- Q1, forza della board;
- Q2, −h_goal;
- Q3, azioni utili o volute;
- Q4, per tipo: (goal_hits, carte usate, profondità), estratta a caso con il seme della ricerca.

Si estrae a turno da ciascuna, e Q3 riceve estrazioni in più quando la ricerca progredisce. Lo sforzo è più che piccolo: cancellazione pigra tra le code, mutex, `Frontier` di `resume()`. Con un thread resta riproducibile, con più thread no.

**4. Novità come chiave secondaria.**
- Atomi:
  - per te, (codice canonico, tipo di zona);
  - le attivazioni del turno e l'Evocazione Normale usata;
  - per l'avversario, quante carte ha nel Cimitero e bandite.
- La tabella è divisa per numero di colpi all'obiettivo, come fa BFWS.
- Nota del verificatore: `canonical_fingerprint` unisce già i Cimiteri in un altro ordine e le copie. La novità serve per gli stati diversi che non portano niente di nuovo: zone diverse, Set di riempimento, giri inutili.
- Solo penalità, mai tagli.
- Rischi: la tabella dipende dall'ordine dei thread, e va salvata in `resume()`.

**5. Dopo il primo obiettivo, migliorare attorno alla linea.** È l'equivalente di «matto trovato, continua» negli scacchi, o di [NMCS](https://ijcai.org/Abstract/09/083).
- Si rimettono in coda, a discrepanza zero, le alternative di ogni nodo della linea che ha raggiunto l'obiettivo. Un altro gruppo di materiali alla scelta della Fusione è a una sola deviazione.
- `first_only` diventa «fermati dopo il vicinato della prima linea».
- Serve B5a, altrimenti le linee a 2 e a 3 materiali valgono uguale.

---

## 3. Stockfish

### Cosa facciamo già come un motore di scacchi

| Stockfish | DuelForge |
|---|---|
| Tabella delle trasposizioni | `seen`: 64 shard di `HashSet<u64>` sulla chiave canonica. Non serve una politica di sostituzione: 8 byte a nodo, collisioni trascurabili ([tt.cpp](https://raw.githubusercontent.com/official-stockfish/Stockfish/master/src/tt.cpp)). |
| Linea principale per prima | Il tuffo segue la prima risposta di `dive_order` fino alla fine del turno. |
| Riduzione delle mosse tardive | Penalità di 0,6 per ogni deviazione, uguale per tutte. |
| «Stand pat» della quiescenza | Board provvisoria a ogni Fine Turno; la Fase Finale si gioca dopo. |
| Tabelle pezzo-casa | `CardValues` per (carta, Spot), misurate dal turno di prova. |
| Valutazione lineare scritta a mano | `quick_score`. |

### Cosa adottare
1. **History heuristic**, relativa e di continuazione ([history.h](https://raw.githubusercontent.com/official-stockfish/Stockfish/master/src/history.h), [search.cpp](https://github.com/official-stockfish/Stockfish/blob/master/src/search.cpp), [relative history](https://cris.maastrichtuniversity.nl/en/publications/the-relative-history-heuristic/), [MAST/CadiaPlayer](https://staff.ru.is/yngvi/pdf/FinnssonB11a.pdf)).
   - Una tabella per `Offer` (ActionKind, codice canonico, luogo, descrizione) più il numero di attivazioni della carta nel turno.
   - Credito: quando una board entra nella top K, le risposte della sua linea ricevono un bonus e le altre risposte date negli stessi nodi una piccola penalità. Il valore è la media (successi divisi per tentativi).
   - Tabella di continuazione per coppie (offerta precedente, offerta), per sequenze come «Evoca X, poi l'effetto di X».
   - Usi: spareggio dentro le classi di `dive_order` (oggi conta l'ordine del motore) e termine di priorità per le risposte lasciate in coda.
   - Le sotto-scelte (materiali, zone) vogliono una chiave propria.
   - Decadimento una volta per ricerca o per giro.
   - Rischi: rinforza ciò che è stato esplorato per primo, ed eredita le cecità della stima.
2. **Penalità di deviazione graduata**, sul modello della riduzione delle mosse tardive e di DDS.
   - Penalità = c·ln(1 + rango in `dive_order`), più bassa ai momenti liberi che dentro una risoluzione, meno la history.
   - Va insieme alla history: con un ordinamento piatto il rango è in parte arbitrario.
3. **Taratura Texel della stima** ([Ethereal Tuning.pdf](https://github.com/AndyGrant/Ethereal/blob/master/Tuning.pdf), [CPW](https://www.chessprogramming.org/Texel%27s_Tuning_Method)).
   - Si scrive `quick_score` come w·φ(stato), con termini nuovi generali: carte avversarie tolte per luogo, tuoi pezzi con effetti nel Cimitero e nelle bandite, effetti una volta per turno non ancora usati, firma di costruzione.
   - Il fit è offline, con perdita a coppie (Bradley-Terry) tra board della stessa mano. Si regolarizza verso i pesi di oggi, il 30% dei deck resta fuori dal fit e le board quasi uguali si deduplicano.
   - Bersaglio: il livello 2 o la verità di `tune_weights.py`, non il livello 1. Un fit non dà valore a ciò che la verità non vede, quindi prima B5a.
   - Obiettivo C5.7: da 174 inversioni su 1110 a 60 o meno.
4. **Correzione della stima mentre la ricerca impara** ([correction history](https://raw.githubusercontent.com/official-stockfish/Stockfish/master/src/history.h), [KataGo](https://github.com/lightvector/KataGo/blob/master/docs/KataGoMethods.md)). Due parti:
   - **Priorità vecchie.** Oggi `learn_round` alza i valori e ristima le board registrate, ma le Edge già in coda tengono la priorità calcolata quando sono entrate. Soluzione:
     - un contatore di «epoca dei valori»;
     - la Edge porta la chiave della board da cui viene il suo valore (va aggiunta: `Dive.boards` ha solo profondità e stima);
     - quando la Edge esce dalla coda si ricalcola la priorità, e se è scesa la Edge torna in coda.
   - **Residui con segno** (misura − stima) per chiave, con passi limitati. Solo dopo la taratura Texel, perché misura e stima sono su scale diverse.
5. **NNUE: la ricetta sì, l'architettura no** ([nnue.md](https://github.com/official-stockfish/nnue-pytorch/blob/master/docs/nnue.md), [dataset](https://github.com/official-stockfish/nnue-pytorch/wiki/Training-datasets)).
   - Accumulatore e quantizzazione non servono: la valutazione costa microsecondi, il motore millisecondi.
   - Le etichette invece si prendono: alle board finali la misura, agli stati intermedi la misura della board migliore raggiunta sotto di loro. Questo insegna il potenziale a metà combo, che `quick_score` non vede.
   - Scala: un milione di board sono circa 9 giorni di CPU di turni di prova.
   - Ordine: prima il fit lineare, poi alberi di decisione, una rete solo se i primi due si fermano. È già il piano `valutatore-imparato.md`, dopo #47 e #43.
6. **Il metodo di fishtest** ([Fishtest mathematics](https://github.com/official-stockfish/fishtest/wiki/Fishtest-mathematics), [SPSA](https://jhuapl.edu/SPSA/PDF-SPSA/Spall_An_Overview.PDF)).
   - SPSA per tarare insieme le costanti: DISCREPANCY 0,6, +10, END_PHASE_LAST 100, 3,0 per colpo, LEARN_EVERY 1500, LEARN_BOARDS 2, LEARN_AMONG 200, VALUE_WEIGHT 5, MEASURED_BODY 0,3.
   - Per accettare una modifica: test appaiato per mano (stesso seme, vecchio contro nuovo: meglio, peggio o pari) con arresto sequenziale.
   - Esempio dal PIANO §30.8: la chiave dei nodi con le scelte aveva migliorato il banco (best da 26,32 a 27,23), ma su tre semi di `combo_hunt` ha dato risultati misti ed è stata tolta.
   - Le corse lunghe le lancia il proprietario.
7. **Tempo gestito dalla stabilità del migliore** (`iterative_deepening` in [search.cpp](https://github.com/official-stockfish/Stockfish/blob/master/src/search.cpp)).
   - Se la top 1 o la top K cambiano ancora, la ricerca prende più tempo fino a un tetto, con un indicatore «sta ancora migliorando».
   - Se sono ferme da molti nodi, la ricerca si ferma e passa il tempo a `confirm_best`.
   - La regola va misurata in nodi, non in secondi, per restare riproducibile.
8. **Controllo della storia nelle trasposizioni**, solo in modalità di debug ([GHI](https://webdocs.cs.ualberta.ca/~mmueller/ghi.html)).
   - Quando un nodo risulta già visto, si confrontano a campione le azioni offerte nei due cammini.
   - I flag Lua e i blocchi non legati a un'attivazione non sono nella chiave, e possono nascondere linee.

**Non si adattano:** null move, finestre di aspirazione, PVS, killer move per profondità. Nel nostro turno c'è un solo giocatore, quindi non c'è una finestra alfa-beta su cui agire. Un alfa-beta sulle risposte dell'avversario costerebbe un rigioco per nodo su due livelli: troppo per un'app desktop.

---

## 4. Altre idee dalla letteratura

Ogni idea ha impatto, sforzo e rischi. Le prime tre sono le più utili.

1. **Salvare e ripristinare il motore con un'arena di memoria** ([ygo-combo-solver](https://github.com/Armytille/ygo-combo-solver), [arena.cpp](https://raw.githubusercontent.com/Armytille/ygo-combo-solver/master/arena.cpp), [fork macOS](https://github.com/96jonesa/ygo-combo-solver)). Impatto alto, sforzo grande, confidenza alta (codice letto).
   - Come funziona: tutte le allocazioni del core e di Lua vanno in un'arena a indirizzo fisso. Un checkpoint copia solo le pagine scritte. Il README parla di costi «di ordini di grandezza» inferiori al rigioco.
   - DuelForge ha già i due agganci: la patch di `l_alloc` in `build.rs` e l'`operator new` in `shim/new.cpp`. Non serve toccare `vendor/ygopro-core`.
   - Rischi e lavoro:
     - su Linux servono mprotect/SIGSEGV, userfaultfd o una copia intera;
     - il garbage collector di Lua va fermato e il seme di Lua fissato, con patch al build;
     - i checkpoint sono una pila: per saltare a un nodo qualsiasi della frontiera bisogna tornare all'antenato comune e rigiocare il resto;
     - va conciliato con il riuso dei duelli che c'è già (`shim/reuse.cpp`).
   - Il guadagno va misurato sul banco. Se arriva, rende possibili NRPA e Go-Explore e cambia il discorso sui thread (B6).
   - Su Windows le pagine scritte si leggono con `VirtualAlloc(MEM_WRITE_WATCH)` e `GetWriteWatch`. La corsia 2 ne fa un prototipo di misura prima della strada 4 di B0 (`velocita-ricerca.md`).
2. **Scegliere quali board misurare per dimezzamenti successivi** ([Karnin et al.](https://proceedings.mlr.press/v28/karnin13.html), [Jamieson & Talwalkar](https://proceedings.mlr.press/v51/jamieson16.html), [Hyperband](https://homes.cs.washington.edu/~jamieson/hyperband.html)). Impatto alto sul tempo dell'app, sforzo piccolo.
   - Oggi `combo_score` misura tutte le 30 board di SCORED_BOARDS a piena fedeltà: 0,78 s di mediana e 2,64 s al 95° percentile per board, minuti al livello 2.
   - Proposta a gradini:
     - `quick_score` su 40-60 board;
     - misura senza breaker su tutte, si tiene la metà;
     - misura completa su 10-15;
     - livello 2 su 3-4.
   - I tagli devono essere dolci, o basati su limiti superiori. La correlazione tra un gradino e l'altro si verifica prima con `rank_report`.
   - Vale per l'app e per `combos --probe`, non per `learn_round`, che serve a imparare valori.
3. **Statistiche d'azione durante la ricerca** ([NST](https://dke.maastrichtuniversity.nl/m.winands/documents/article_vMay2012.pdf), [progressive strategies](https://cris.maastrichtuniversity.nl/en/publications/progressive-strategies-for-monte-carlo-tree-search), [decay](https://dke.maastrichtuniversity.nl/m.winands/documents/TCIAIG_decay.pdf)). Impatto alto, sforzo piccolo.
   - È la stessa famiglia della history (§3.1), con il bias che decresce con le visite. Coppie di mosse consecutive per «cerca X, poi usa X».
   - Prima va corretto l'ordine inverso dei fratelli (vedi «In breve»).
4. **Politica imparata: un classificatore di mosse addestrato sull'oracolo con DAgger** ([He, Daumé & Eisner](https://papers.nips.cc/paper_files/paper/2014/hash/757f843a169cc678064d9530d12a1881-Abstract.html), [SaIL](https://proceedings.mlr.press/v78/bhardwaj17a.html), [Chrestien et al.](https://proceedings.neurips.cc/paper_files/paper/2023/hash/50ea4dbd1cff6bd3daef939eff10c092-Abstract.html), [learning to rank](https://arxiv.org/abs/1608.01302)). Impatto alto, sforzo grande.
   - Caratteristiche senza nomi di carta: tipo di decisione, tipo di carta, luogo di partenza e di arrivo, ruolo, quota `used` nei collegamenti, azione utile o no, profondità, carte in mano.
   - Perdita a coppie: la risposta scelta nella linea migliore contro le altre offerte nello stesso punto.
   - Uso: −λ·ln p nella priorità e nell'ordine del tuffo, sempre con un minimo, mai come taglio. In modalità obiettivo si può ordinare alla LTS (profondità divisa per la probabilità del cammino); la garanzia sui nodi vale solo lì.
   - Rischio principale: addestrarlo sulle linee della ricerca stessa gli insegna i suoi difetti, per esempio Aeon a 2. Quindi si addestra sull'oracolo A1 e sulle linee guida.
   - Per l'inferenza in Rust c'è [gbdt-rs](https://github.com/mesalock-linux/gbdt-rs), ma la manutenzione è incerta. Utile anche normalizzare i punteggi in ranghi dentro ogni mano ([Ranked Reward](https://arxiv.org/abs/1807.01672v2)).
5. **Scelta delle board nei giri di apprendimento** (active learning, [Settles](https://khoury.northeastern.edu/home/vip/teach/MLcourse/4_boosting/materials/settles.activelearning.pdf)). Impatto medio, sforzo medio.
   - Si misurano le board con stima + κ·incertezza, invece che solo le più nuove.
   - Il termine di esplorazione resta: `learn_round` serve a imparare valori.
6. **NRPA e GNRPA: adattare per ogni mano la politica del tuffo** ([Rosin](https://mlanthology.org/ijcai/2011/rosin2011ijcai-nested), [GNRPA](https://arxiv.org/abs/2003.10024)). Impatto medio, sforzo grande, confidenza bassa.
   - Prima degli snapshot si può fare solo il passo di adattamento verso la linea migliore. I rollout veri costano troppo: ognuno è un replay completo (5,6 ms in media sul banco, molto di più sui deck pesanti di Lua).
7. **Probabilità di riuscita su più ordini del Deck** ([Klondike](https://ojs.aaai.org/index.php/ICAPS/article/view/13363), [ensemble per MTG](https://eprints.whiterose.ac.uk/75050/1/EnsDetMagic.pdf)). Impatto medio, sforzo medio.
   - Ogni ricerca usa un solo seme, quindi rischia di premiare una linea che ha pescato o scavato bene.
   - Le linee top con eventi casuali si rigiocano su 8-16 semi, con risposte riconosciute per identità come in `follow.rs`, e si mostra la quota di semi in cui riescono. È già nel PIANO.
8. **ygo-combo-solver come oracolo esterno** ([README](https://raw.githubusercontent.com/Armytille/ygo-combo-solver/master/README.md), [ygorl](https://raw.githubusercontent.com/n0tevi1/ygorl/main/docs/solver.md)). Impatto medio, sforzo medio (il verificatore lo giudica sottostimato).
   - Per Invoked: obiettivo Oceanus nella Zona Mostri Extra + S:P Little Knight, con `--material` ripetuto per Attributo e `--resolve` su Aeon. Così si separa «il motore non offre l'azione» da «la nostra ricerca non la trova».
   - Lavoro e rischi:
     - servono un template `.yrpX` e una build per Windows o macOS;
     - la versione del core e degli script deve essere identica a `data/`;
     - un fallimento non prova che l'obiettivo è impossibile.
   - Licenza AGPL compatibile; serve l'attribuzione se si porta codice.
9. **Riusare le linee già trovate.** Impatto medio, sforzo medio.
   - Una tabella con, per ogni mostro dell'Extra Deck Evocato, la linea più economica: vale solo per la stessa mano e lo stesso seme.
   - Macro di 2-4 passi come bias, ordinate per quanti stati nuovi raggiungono.
   - Landmark imparati da molte linee, mai da una sola.
   - Estende B7 ([landmark appresi](https://arxiv.org/abs/2508.21564)).
10. **Ricette lette dagli script Lua** ([operators.cpp](https://raw.githubusercontent.com/Armytille/ygo-combo-solver/master/operators.cpp), [Retro*](https://proceedings.mlr.press/v119/chen20k.html)). Impatto alto in teoria, sforzo grande, fragile.
    - Per Aeon lo script dice `AddProcMixRep(…, 2, 99)` con un filtro proprio: 2 o più mostri Fusion, tutti con Attributi diversi. Il bando dall'Extra Deck chiede 3 o più materiali.
    - Il bando dall'Extra Deck è dichiarato in `SetPossibleOperationInfo`.
    - Va fatto dopo il rilassamento sui collegamenti.
11. **Taratura offline con CEM, SMAC o irace** ([SMAC3](https://github.com/automl/SMAC3), [irace](https://iridia.ulb.ac.be/supp/IridiaSupp2016-003/), [SameGame CEM](https://dke.maastrichtuniversity.nl/m.winands/documents/KNOSYS_SameGame.pdf)). Impatto medio, sforzo medio.
    - Alternativa a SPSA, con racing per scartare presto le configurazioni peggiori.
    - Va eseguita con budget di nodi, non di tempo, e su deck tenuti fuori.
12. **Coppie suggerite per le preferenze del giocatore** ([Christiano et al.](https://arxiv.org/pdf/1706.03741)). Impatto basso.
    - Si propone il confronto dove i fit di Bradley-Terry sono più in disaccordo.
    - Richiede lavoro sull'interfaccia (Gemini) e un cambio di contratto.
13. **Parole chiave dal testo delle carte come valore iniziale in `CardValues`** ([ygo-agent](https://github.com/sbl1996/ygo-agent)). Impatto basso-medio.
    - Prima un sacchetto di parole chiave (`roles.rs` legge già i testi). Embedding di un modello linguistico solo più avanti: dipendenza pesante, calcolati in locale e mai committati.
    - Non risolve la misura.

**Da non adottare:**
- UCT con rollout casuali;
- ISMCTS e insiemi determinizzati nella ricerca del tuo turno (al massimo in `matchup.rs` e `follow.rs`);
- Online Evolution ([Justesen et al.](https://pure.itu.dk/en/publications/online-evolution-for-multi-action-adversarial-games/));
- RL per self-play: ygo-agent ha richiesto circa 100 milioni di partite, l'equivalente di 32 RTX 4090 per 5 giorni, e in ygorl la politica clonata raggiunge 41 obiettivi su 100 contro i 62 del solver;
- i bot di [WindBot](https://github.com/ProjectIgnis/windbot): 66 esecutori scritti a mano per deck, contro la regola dei nomi;
- AlphaZero.

---

## 5. Obiettivi più ricchi

### Oggi
- `Goal { on_field: Vec<GoalCard { code, zone: Any | Extra | Main }>, all, first_only }`, più `keep_in_hand` come filtro duro separato.
- `hits()` conta i mostri scoperti nelle Zone Mostri Principali ed Extra, e qualunque carta nella fila Magie e Trappole come `Any`, coperta o no.
- Non si possono esprimere: una carta coperta come tale, il Cimitero, le bandite, la mano (solo con `keep_in_hand`), i materiali, lo stato dell'avversario.

### Cosa cambia nel motore (`search.rs`)
- **`GoalCard` diventa un atomo** `{ code, place, min_count, hard, weight }`.
  - `place` può essere: Zona Mostri Principale, Zona Mostri Extra, una Zona Mostri qualsiasi, Magia/Trappola scoperta, Magia/Trappola coperta, Zona Terreno, Cimitero, bandita, mano.
  - Ci sono anche atomi sull'avversario senza codice di carta: «carte tolte dal suo Extra Deck ≥ n», «sue carte nel Cimitero o bandite ≥ n».
- **`hits()`** legge `me.spells` con `is_face_up()`, `me.grave`, `me.banished` e `me.hand`, e confronta Extra Deck, bandite e Cimitero di `players[1]` con l'inizio del turno.
- **Gli atomi fuori dal campo si controllano solo sulle board esatte**, cioè dopo la Fase Finale: le board provvisorie si leggono prima. Non sono monotoni (una carta nel Cimitero ora può servire sul campo dopo), quindi la distanza non deve premiarli presto.
- **`involves()`** distingue «porta la carta dove serve» da «la consuma» (materiale, Tributo, costo).
- `SelectPlace` e il bonus del tuffo si estendono ai nuovi posti.
- **Atomi duri e morbidi.** I duri restano la prima chiave di `rank()`. I morbidi pesano in `estimate()` al posto di 3·goal_hits.
- **Il verdetto «impossibile» resta corretto**: `goal_reached` dà `Some(false)` solo dopo una ricerca esaustiva.
- **Seconda fase: vincoli sui materiali e sulle risoluzioni.** Esempi: Aeon Evocato con 3 o più materiali, o 3 Attributi distinti; un effetto risolto n volte.
  - Servono i materiali di ogni Evocazione, dai messaggi del motore.
  - Serve unire `ChainSolved` e `Chaining`, perché `ChainSolved` porta solo il numero dell'anello.
  - «Attributi distinti ≥ n» va progettato da zero.
- **Limite attuale:** con il kit di oggi l'Extra Deck avversario ha una sola carta. «≥ 3 carte tolte dal suo Extra Deck» è irraggiungibile, «≥ 1» funziona. Con B5a l'atomo diventa pieno.

### Contratto e interfaccia
- `GoalCardRequest` in `apps/desktop/src-tauri/src/lib.rs` e `GoalZone`/`GoalCard` in `apps/desktop/src/lib/api.ts` cambiano nella stessa PR, fatta da Claude.
- Gli obiettivi salvati con `any`, `extra` o `main` continuano a valere: diventano i posti nuovi corrispondenti.
- Interfaccia (Gemini, richiesta in `docs/TASKS.md`):
  - per ogni carta, un selettore del posto: Mostro Principale, Mostro Extra, Magia/Trappola coperta, scoperta, Cimitero, bandita, mano;
  - una riga «avversario»;
  - il segno duro o morbido;
  - quando l'obiettivo è impossibile, il messaggio dice perché.
- Opzione: un pulsante «obiettivo suggerito», come fa ygorl: le Magie e Trappole coperte, le handtrap in mano e i pezzi che bloccano della board migliore.

### Come lo usano le euristiche di pianificazione
- Gli atomi sono i fatti del rilassamento. Cimitero, bandite e Set ci sono già nella mappa (`ToGrave`, `Recover`, `Set`), quindi h_goal si estende senza lavoro nuovo.
- La novità si divide per numero di atomi soddisfatti, come in BFWS.
- AMAF usa i colpi parziali sugli atomi.
- Q2 misura la distanza dagli atomi duri. Dopo il primo raggiungimento, il miglioramento attorno alla linea ottimizza gli atomi morbidi e la forza della board.

---

## 6. Piano aggiornato

### Modifiche alle voci esistenti
- **A1 (oracolo):**
  - **niente linee scritte a mano** (decisione del proprietario): il riferimento resta l'oracolo automatico, più il controllo esterno opzionale con ygo-combo-solver;
  - l'oracolo diventa la fonte delle etichette per B9, B12 e B15;
  - si aggiunge il test appaiato per mano con arresto sequenziale, come regola per accettare ogni modifica della fase B.
- **A2 (dove va il lavoro), contatori nuovi:**
  - posizione in cui escono dalla coda i fratelli (ordine inverso);
  - stati a novità zero;
  - doppi nati da due azioni libere scambiate, che decidono se fare B2;
  - controllo a campione della storia nelle trasposizioni.
- **B1 (ordinamento delle mosse)** si divide in tre:
  - B1a, spareggio dei fratelli nell'ordine di `dive_order`: correzione rapida;
  - B1b, history relativa e di continuazione;
  - B1c, penalità di deviazione graduata.
  - La politica imparata diventa B12.
- **B2:** solo sleep set con una dipendenza prudente, e solo se A2 mostra una quota grande. DPOR non si adatta.
- **B3:** si aggiunge il controllo della storia nelle trasposizioni.
- **B4** diventa concreto: rilassamento h_add sulla mappa, azioni utili, credito AMAF all'obiettivo, miglioramento attorno alla linea obiettivo.
- **B5** si divide:
  - **B5a:** attrito avversario, Extra Deck avversario realistico, correzioni dell'Observer (righe 8, 11 e 14), `look_key`, rigioco per identità, sì di Aeon mai rifiutato;
  - **B5b:** run «react» contro i breaker;
  - **B5c:** protezione nel punteggio, turno 3 dal campo, Spot di Cimitero e bandite, danno, tassa;
  - **B5d:** firma di costruzione nei valori;
  - **B5e:** kit più ampio;
  - **B5f:** esposizione letta dal motore.
- **B6:** si rivede dopo B13.
- **B7:** si aggiungono la tabella delle linee per mostro Evocato e le macro.

### Voci nuove
- **B8.** Obiettivi più ricchi (§5).
- **B9.** Stima con l'avversario e taratura Texel a coppie, con i termini nuovi.
- **B10.** Priorità aggiornate dopo l'apprendimento (epoca dei valori), poi la correzione con segno.
- **B11.** Più code alternate e novità come chiave secondaria.
- **B12.** Politica imparata dall'oracolo con DAgger, e ordinamento alla LTS in modalità obiettivo.
- **B13.** Salvare e ripristinare il motore con un'arena di memoria.
- **B14.** Quali board misurare: dimezzamenti successivi nell'app e nella CLI, scelta per incertezza nei giri di apprendimento.
- **B15.** Taratura delle costanti con SPSA o CEM.
- **B16.** Tempo gestito dalla stabilità del migliore.
- **B17.** Probabilità di riuscita su più semi.

### Ordine di lavoro
File caldi: `search.rs`, `probe.rs`, `value.rs`, `lib.rs`, `api.ts`, un agente alla volta per file. I moduli nuovi (`relax.rs`, la tabella della history, l'arena in `ygo-engine`) si scrivono e si provano in parallelo; si serializza solo l'aggancio in `search.rs`.

| Fase | Passo | Impatto atteso | Sforzo | File | Chi, in parallelo con |
|---|---|---|---|---|---|
| 1 | A2 con i contatori nuovi (A2 di base fatto, PR #67) | permette tutto il resto | piccolo | `search.rs` (dietro variabile d'ambiente) | corsia 2 |
| 1 | A1 con guide e test appaiato | etichette e controllo fisso | medio | script, fixture | corsia 6, in parallelo |
| 1 | B5a | alto: caso Aeon, toglie l'artefatto da 0,6 | medio | `probe.rs`, `lib.rs` (`score_view`); peso in `scoring.ts` (Gemini, D17) | coordinatore, in parallelo con la corsia 2 |
| 1 | Prototipo di B13 | decide la leva più grande | medio | `ygo-engine` (`duel.rs`, `build.rs`, `shim/`): nessun file caldo | corsia 2, dentro B0 |
| 1 | Esperimenti offline: fit Texel sui JSON di `combo_hunt`, correlazione tra gradini di misura | prepara B9 e B14 | piccolo | script, `docs/plans/` | sessione sorella di ricerca |
| 2 | B1a | medio, quasi gratis | piccolo | `search.rs` | corsia 2 |
| 2 | B8 (motore e contratto) | alto per il proprietario | piccolo-medio | `search.rs`, `lib.rs`, `api.ts`; poi interfaccia di Gemini | corsia 2 dopo B1a |
| 2 | B1b e B1c | alto | piccolo-medio | `search.rs` più un modulo nuovo | corsia 2, una PR ciascuno |
| 2 | B5b | alto (30 e 28 deck) | medio | `probe.rs` | coordinatore |
| 2 | B14 nell'app e nella CLI | alto sul tempo dell'app | piccolo | `lib.rs`, `ygo-cli` | dopo B8 (stesso `lib.rs`) |
| 3 | B9 con B5d | alto; obiettivo C5.7 ≤ 60 | medio | `search.rs`, `value.rs` | dopo B5a (la verità deve vedere il bando) e dopo B1 |
| 3 | B4 | alto in modalità obiettivo | medio | `relax.rs` (in parallelo), aggancio in `search.rs` | dopo B8 e B9 |
| 3 | B10, B11, B16 | medio ciascuno | piccolo-medio | `search.rs` | in sequenza sulla corsia 2 |
| 3 | B5c | medio | piccolo ciascuno | `probe.rs`, `value.rs`, `lib.rs` | coordinatore dopo B5b |
| 4 | Integrazione di B13, poi B6 | alto, se il prototipo convince | grande | `ygo-engine`, `search.rs`, `probe.rs` | dopo la fase 3 su `search.rs` |
| 4 | B12 | alto | grande | modulo nuovo, `search.rs`, script | dopo A1 e B1 |
| 4 | B15 | medio | medio | script; corse notturne lanciate dal proprietario | dopo B1 e B9 |
| 4 | B7 esteso, B17 | medio | medio | `search.rs`, CLI | dopo B12 |
| 4 | B2 | basso-medio | grande | `search.rs` | solo se A2 lo giustifica |
| 4 | B5e e B5f, poi il valutatore imparato | medio | grande | `probe.rs`, `explain.rs` | dopo #47 e #43, con ricalibrazione del banco |

### Obiettivi misurabili in più
Quelli di `ricerca-piano-critico.md` restano. Si aggiungono:
- **Aeon:** la board con Aeon a 3 materiali sta sopra quella a 2 già nella stima della ricerca, non solo nella misura.
- **Ordine dei fratelli:** nessuna inversione dopo B1a.
- **C5.7:** 60 inversioni su 1110 o meno, dopo B9.
- **Obiettivi ricchi:** un obiettivo con una Trappola coperta e una carta nel Cimitero si raggiunge su Labrynth e Branded; un obiettivo impossibile riceve il verdetto giusto.
- **App:** con la misura a gradini, tempo dimezzato a parità di board nella top 3.

### La sessione sorella di ricerca
Sì, ha senso, con limiti precisi. Questo rapporto è già il risultato di quel tipo di lavoro.
- **Una sola sessione**, non un flusso multi-agente continuo: consuma molta quota (`CLAUDE.md`).
- **Compiti:**
  - esperimenti offline in Python sui dati che il banco produce già: fit Texel, correlazione tra gradini di misura, simulazioni di SPSA;
  - il controllo con ygo-combo-solver sul caso Invoked;
  - aggiornare la bibliografia in un documento di `docs/plans/`.
- **Non tocca** i file caldi né il motore. Consegna numeri e note di piano, e le corsie di Claude le trasformano in codice.
- **Le corse lunghe** (oracolo, SPSA, CEM) partono solo quando il proprietario le chiede.

---

## 7. Critica di completezza: cosa manca
Un agente separato ha riletto il rapporto contro il database delle carte e il codice. Gli errori di fatto che ha trovato sono già corretti sopra (testo di Aeon, unità dei costi di velocità, Life Point, verifica su Dogmatika). Qui restano le lacune, da usare come lista di controllo quando si scrivono i passi del piano.

### 7.1 Meccaniche delle carte che mancano o sono coperte solo in parte
| Meccanica | Cosa sfugge oggi (verificato) | Manca nel rapporto |
|---|---|---|
| **Materiali Xyz** | `value.rs:278` misura ogni Xyz con 2 materiali di riempimento fissi. Un Xyz con 0 o 1 materiale è sopravvalutato, con 3 o più è sottovalutato. `quick_score` non conta i materiali. | I materiali come numero di usi rimasti. Gli effetti che dipendono dal materiale (bonus se c'è un certo tipo o Attributo). |
| **Pendulum** | I Pendulum scoperti nell'Extra Deck valgono 0 in `quick_score`. Le scale valgono 0,5 come una Magia scoperta. | Evocazione Pendulum al turno 3 come risorsa per il turno dopo, accanto a `turn_three`. Il kit dell'avversario non fa mai Evocazioni Pendulum, solo «blocchi» (riga 7). |
| **Freccette Link** | — | Effetti dei mostri «co-linked» o «a cui punta». Zone libere per i Link del turno 3. L'occupazione di entrambe le Zone Mostri Extra, che oggi fa fallire per caso la prova ExtraSummon e vale 0,6. |
| **Token** | Contano 1,0 come corpo in `quick_score`. | Spesso non possono essere materiale Xyz o Link, non lasciano risorse nel Cimitero e non si rievocano. |
| **Una volta per turno, limiti duri** | — | Limiti condivisi tra più effetti della stessa carta (Aeon). «Una volta per Duello», «una volta finché è scoperto», «fino alla fine del prossimo turno»: `CardValues` misura in uno stato vergine e non li vede. |
| **Blocchi di Evocazione delle tue carte** | — | Il rilassamento B4 li ignora, quindi può guidare la ricerca verso linee senza uscita (una carta che blocca attivata presto). Mancano anche i blocchi o i floodgate che durano fino al tuo turno 3 e colpiscono anche te, come Skill Drain o «non puoi Evocare per il resto di questo turno e il prossimo». |
| **Catene** | — | Ordine dei trigger (SEGOC), «perdere il tempismo» (*when* contro *if*), costruire catene per schivare una handtrap, effetti a cui non si può rispondere, effetti «una volta per catena». La riga 3 parla solo di protezione generica. |
| **Moneta, dado, casuale** | — | B17 cita solo «pescato o scavato». Mancano lanci di moneta, dadi, scarti casuali e i deck che mandano carte dal Deck (Tearlaments, Lightsworn): una ricerca a un solo seme sfrutta l'esito fortunato. |
| **LP** | Vedi §1. | LP pagati contro il letale nel turno avversario (l'attacco da 4200 non guarda gli LP). Effetti condizionati agli LP. |
| **Contatori** | Citati alla riga 5. | Contatori messi sulle carte avversarie (Kaiju). Contatori come munizioni per il turno dopo. |
| **Effetti ritardati** | — | Ritorni «durante la prossima Standby Phase», bandite temporanee, Spirit, limite di 6 carte in mano alla fine del turno. Conta anche per gli atomi «mano» del §5. |
| **Bandite coperte e scoperte** | — | Gli atomi «bandita» del §5 e lo `Spot::Banished` non le distinguono: Pot of Desires bandisce coperto, e quella carta non si recupera. |
| **Rituali, Gemini, Union, Trappole-mostro** | — | Assenti, sia come Evocazioni proprie sia come prove del kit. Nel kit (`probe.rs` `Probe::ALL`) mancano anche l'attivazione di Trappole, l'Evocazione per Tributo e la Magia Terreno attivata. |
| **Master Duel e TCG/OCG** | — | I deck vengono da Master Duel Meta, gli script EDOPro sono TCG/OCG. Servono un controllo sistematico delle carte senza script, solo prerelease o diverse tra i formati (non solo Mercurium) e un avviso generale. |

### 7.2 Aree di ricerca non coperte
- **IA di altri giochi di carte con turni a molte azioni.**
  - Hierarchical Portfolio Search (Churchill & Buro, *Prismata*).
  - Ricerca del letale e pianificazione del turno in Hearthstone, con MCTS più apprendimento supervisionato (Świechowski et al. 2018), e Legends of Code and Magic.
  - Sono i domini più vicini a «un turno da solo con tante azioni». Il rapporto respinge solo Online Evolution.
- **Quality-Diversity (MAP-Elites).** Serve a tenere board diverse per descrittori, ad esempio numero di interruzioni contro risorse per il turno dopo. Risponde direttamente a «linee ripetitive», che la novità copre solo in parte.
- **Best-first parallelo** (HDA*, PBNF) e riproducibilità a più thread. Il rapporto rinvia B6 senza bibliografia.
- **Simmetrie nei pianificatori** (Pochter, Domshlak e Hoffmann) per copie e zone equivalenti, oltre a `canonical_fingerprint`.
- **Più obiettivi e scelta di Pareto.** Il punteggio somma pesi fissi. Esistono la frontiera di Pareto (task #40) e il piano `pesi-imparati-pareto-ui.md`, ma il rapporto non discute ricerca lessicografica o multi-obiettivo.
- **Metriche di ordinamento** (NDCG@k, tau di Kendall) per valutare la stima contro la misura. Oggi si contano solo le «inversioni».
- **Validazione esterna della verità.** Texel e DAgger tarano su livello 2 o `tune_weights`, cioè su un altro modello: il ragionamento è circolare. Non c'è un insieme di N board di riferimento da guide o esperti, su M deck e con le fonti. Oggi c'è il solo Invoked.
- **Obiettivi più espressivi.** Mancano disgiunzioni («uno tra questi tre»), «almeno N tra», obiettivi per proprietà (un Link-3 qualsiasi, un mostro LIGHT), obiettivi negativi (non usare l'Evocazione Normale, X resta nel Deck), materiali attaccati, posizione coperta in Difesa.
- **Cache delle misure tra ricerche** (stessa `look_key` in mani diverse o ricerche ripetute): manca come leva di velocità accanto a B13 e B14.

### 7.3 Affermazioni senza fonte o con fonte debole
- I conteggi per categoria (30, 28, 27… su 37) non dicono metodo, data dell'elenco MDM, deck ed elenco delle carte per riga, né come è stata attribuita una carta a una categoria. Il criterio «deck toccati» non pesa quanto la carta è centrale nel deck.
- Le colonne Impatto (alto, medio, basso) e la frase «le prime tre sono le più utili» non hanno un criterio dichiarato.
- «ygo-agent: circa 100 milioni di partite, 32 RTX 4090 per 5 giorni»: nessun link al dato.
- «ygorl: 41 contro 62 su 100»: non dice su quali obiettivi né con quale budget.
- «Costi di ordini di grandezza inferiori» per l'arena: riportato dal README, non misurato. Non si conferma che il core di ygo-combo-solver sia della stessa versione del submodule.
- «0,78 s di mediana e 2,64 s al 95° percentile per board»: mancano deck, macchina, data e numero di thread.
- «Collisioni trascurabili» per la chiave a 64 bit: manca il conto (circa n²/2⁶⁵ per n nodi).
- «La letteratura conferma la scelta» (discrepanze): affermazione generica.
- «Molte fonti lette solo dagli abstract»: non dice quali. Serve un segno di confidenza per ogni fonte, soprattutto per le garanzie di LTS e PHS e per Type-GBFS.
- «Il primo bando colpisce le tue carte» e «il sì opzionale è rifiutato in automatico»: manca il riferimento alla riga dello script e alla regola di `dive_order` o della risposta automatica che lo rifiuta.
- Riga 6, «peso 1 su 10,2»: non dice da dove viene il totale.

### 7.4 Passi del piano senza un controllo misurabile
- **B5b, run «react»:** non c'è un bersaglio. Esempi possibili: resilienza dei deck con floater che cambia di almeno x, correlazione con il livello 2, nessun peggioramento su `combo_hunt`.
- **B5c, B5d, B5e, B5f:** nessun numero. «Ricalibrazione del banco» non è un criterio.
- **B4:** manca una metrica, ad esempio nodi alla prima soluzione su un insieme fisso di obiettivi raggiungibili, con riduzione attesa almeno del x%.
- **B1b e B1c:** c'è solo «nessuna inversione» per B1a. Manca l'effetto su nodi e tempo al migliore in `bench.py --nodes 4000`.
- **B10, B11, B16:** «medio ciascuno», nessuna soglia.
- **B12:** mancano accuratezza top-k sull'oracolo e nodi risparmiati.
- **B13:** non c'è una regola per procedere o fermarsi (ad esempio almeno 3× nodi al secondo sul banco, con risultati identici).
- **B2:** «quota grande» non è definita.
- **B14:** «parità di board nella top 3» va definita (stessa insieme? sovrapposizione almeno 2 su 3?).
- **B15 e B17:** nessun obiettivo.
- **B7 esteso:** nessuna metrica.
- **Test appaiato con arresto sequenziale:** mancano numero di mani e semi, metrica (miglior punteggio L1 o L2), soglie del test sequenziale, trattamento dei pareggi e del rumore della misura.
- **A2:** i contatori nuovi non hanno soglie che decidano i passi successivi.
- **B5a:** cambia tutti i punteggi (Extra Deck avversario da 15 carte). Manca un piano per rifare la base del banco, perché i numeri storici non sono più confrontabili. Il peso sta in `scoring.ts`, cioè nell'area di Gemini (D17): la misura viene divisa tra due agenti senza un controllo di coerenza.
- **Protocollo di `AGENTS.md`:** non è richiamato per ogni passo che tocca `ygo-solver` o `ygo-engine` (`mdm_decks.py`, `bench.py --nodes 4000`, `coverage`, `combo_hunt`, numeri prima e dopo nella PR).
- **Lentezza, la lamentela principale:** il rapporto non dava una scomposizione attuale del tempo. Ora c'è (A2, PR #67): in `combo_hunt` il turno di prova è il 58% del tempo totale, le fasi dopo la ricerca il 15%, l'apprendimento il 4%; dentro la ricerca il replay è il 43%. Quindi B0/B13 (replay) e B14 (quali board misurare) sono davvero le due leve più grandi.
