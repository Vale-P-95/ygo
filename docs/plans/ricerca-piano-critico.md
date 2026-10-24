# Ricerca delle combo: piano critico per board più forti in meno tempo

*Scritto il 22/10/2026 dal coordinatore, dalle prove del proprietario nell'app vera. Il proprietario giudica la ricerca lontana dallo standard che vuole: troppo lenta, con tempo perso su linee inutili, ripetitive o deboli, e board finali non all'altezza delle linee note. Aggiornato lo stesso giorno con la misura A2 (PR #67) e con la ricerca di letteratura e l'audit delle carte (`ricerca-letteratura.md`).*

## Cosa ha visto il proprietario
- **Invoked, ricerca per obiettivo.** Obiettivo: Invoked Oceanus nella Zona Mostri Extra e S:P Little Knight sul campo.
  - La ricerca trova una linea che lo raggiunge, ma passa per Hyperinvoked Aeon Evocato con 2 materiali.
  - Aeon vale molto di più con 3 materiali: bandisce dal campo e dai Cimiteri, e con 3 o più materiali anche fino a 3 carte dell'Extra Deck avversario.
  - Le guide (video) arrivano alla stessa board con Aeon a 3 materiali, passando per i Link Magistus e cercando Crowley dal Deck.
- **I tempi** per trovare le linee sono troppo lunghi.
- **Ruoli («Riconosci»):**
  - Super Polymerization non risulta board breaker;
  - una carta non può essere sia starter sia estensore;
  - Invocation «Grail» risulta solo board breaker, ma è anche un estensore.
- **Obiettivi:** si vogliono chiedere anche carte coperte nella Zona Magie e Trappole, carte nel Cimitero o bandite.

**Regola:** il motore trova da solo le linee forti. Il proprietario non scrive combo, e nessun riferimento dipende da linee scritte a mano.

## Due difetti diversi
1. **La misura non vede il valore di molte carte.** Aeon a 3 materiali è un caso di una famiglia più grande: l'audit su 37 deck del meta ha trovato 17 categorie di effetti che la misura o la stima della ricerca non vedono, o vedono in parte (`ricerca-letteratura.md`, §1).
   - Per Aeon le cecità sono cinque in fila:
     1. la stima della ricerca non guarda l'avversario;
     2. i valori delle carte danno ad Aeon quasi zero;
     3. il kit dell'avversario ha un solo mostro nell'Extra Deck, quindi togliere 1 carta vale come toglierne 3;
     4. l'app misura una sola variante per «aspetto» della board;
     5. le scelte nell'Extra Deck avversario si rigiocano per posizione.
   - Anche una ricerca perfetta non ha motivo di preferire la linea a 3 materiali.
2. **La ricerca spende nodi e tempo dove non serve.** Ora è misurato (A2, sotto): il replay delle linee, gli stati già visti e i passi senza cambi visibili.

## Fase A: misurare
### A1. Riferimenti automatici: l'oracolo (corsia 6, dopo i ruoli)
- **L'oracolo.** Per un insieme fisso di mani si lancia la stessa ricerca con un budget enorme: 50-100 volte i nodi, più semi e più thread, anche per ore, di notte (le corse lunghe le lancia il proprietario). Si tengono le board migliori trovate e la loro misura.
  - Le mani: le 16 del banco, le 82 di `combo_hunt --seed 7`, e le ricerche per obiettivo con obiettivi presi dalle board forti di quei deck.
- **La forza vera** delle board dell'oracolo viene dal banco oggettivo (livello 2 contro il meta), non solo dalla stima rapida.
- **Formato:** `tests/fixtures/oracle/<deck>-<mano>.json`, con mano, board migliori (carte e zone, senza testi), misura e linea.
- **Misure per ogni mano**, con la ricerca normale:
  - dopo quanti nodi e secondi arriva al 90%, al 95% e al 100% del valore dell'oracolo;
  - se trova la board dell'oracolo e in che posizione la mette;
  - nella ricerca per obiettivo, quanto è forte la linea trovata rispetto alla migliore che raggiunge lo stesso obiettivo.
- **Quando l'oracolo stesso non arriva a una board**, si rigiocano i passi per capire perché: un'azione che il motore non offre, una risposta automatica sbagliata, una potatura. Sono difetti da correggere in modo generale.
- **Controllo esterno, facoltativo:** [ygo-combo-solver](https://github.com/Armytille/ygo-combo-solver) gira sullo stesso motore e accetta obiettivi con vincoli sui materiali. Sul caso Invoked separa «il motore non offre l'azione» da «la nostra ricerca non la trova». Serve la stessa versione del core e degli script.
- **Test appaiato per accettare una modifica della fase B.** Vale da subito, anche prima dell'oracolo.
  - Stesse mani e stesso seme, vecchio contro nuovo: le 16 mani del banco e le 82 di `combo_hunt --seed 7`.
  - Per ogni mano: meglio, peggio o pari, con una soglia dell'1% sul valore della board migliore.
  - Si accetta se le mani peggiori non superano quelle migliori (test del segno) e nessuna perde più del 5%.

#### A1: strumenti fatti e primo pilota (corsia 6, 22/10, branch `claude/oracle`)
- **`scripts/oracle.py`.**
  - `hands`: l'insieme fisso di mani. Sono le mani del banco (seme 1), le 82 di `combo_hunt --seed 7` e gli obiettivi. Per ora l'obiettivo è uno solo: Invoked Oceanus nella Zona Mostri Extra + S:P Little Knight, sulla lista Invoked della fixture. Con `--auto-goals N` si aggiungono le ricerche per obiettivo sui mostri dell'Extra Deck delle board dell'oracolo.
  - `build`: la ricerca lunga, su un thread (stesse board su ogni macchina), con più semi. Le corse già scritte restano: una notte interrotta riparte da dove era.
  - `fixtures`: scrive `tests/fixtures/oracle/<id>.json`. Dentro ci sono mano, seme, impronta del deck, le 5 board migliori (codici e zone), la misura, le risposte della linea migliore e il valore trovato con gli altri semi. Nessun testo di carta.
  - `measure`: la ricerca normale a budget crescenti, contro l'oracolo.
- **`scripts/paired_test.py`**, il test appaiato: vecchio contro nuovo, stesse mani, stesso seme, stesso budget. Prima e dopo possono essere due eseguibili (lo script li lancia sulle mani del banco e di `combo_hunt --seed 7`), due cartelle di `combo_hunt --json`, oppure due `results.json` di `bench.py`. Ogni mano risulta meglio, peggio o pari, con una soglia dell'1%. Si accetta se le mani peggiori non superano le migliori (test del segno) e nessuna perde più del 5%.
- **Il valore** di una board è quello del banco: le parti del turno di prova pesate come il profilo «Bilanciato», più il follow-up.
- **Mani del banco che non si giocano più.** Con le liste di Master Duel Meta di oggi, 4 mani Invoked del banco chiedono carte che la lista non ha, o due copie dove ne ha una: inv-remi, inv-aleister (2 Droll & Lock Bird), inv-r3, inv-r4 (Mulcharmy Purulia, The Book of the Law, Invocation - "Grail"). La ricerca non parte: 0 nodi. Vale anche per `bench.py`, che lì misura board vuote. `oracle.py` le salta con un avviso. Mancano anche 4 deck del banco (ignister, labrynth, mitsurugi, yummy_mimighoul).
- **Pilota** (non è l'oracolo vero). Le 8 mani del banco giocabili, a 40 000 nodi (10 volte), solo con il seme della mano. Le fixture in `tests/fixtures/oracle` sono queste, finché il proprietario non lancia il giro lungo. Poi la ricerca normale a 1000-16 000 nodi:

  | mano | oracolo | 1000 | 2000 | 4000 | 8000 | 16 000 |
  |---|---|---|---|---|---|---|
  | branded | 1,38 | 89% | 89% | 91% | 91% | 89% |
  | dracotail | 2,18 | 67% | 67% | 70% | 79% | 79% |
  | inv-r1 | 3,10 | 59% | 62% | 66% | 66% | 96% |
  | inv-r2 | 2,53 | 72% | 76% | 76% | 82% | 100% |
  | inv-r5 | 2,45 | 63% | 68% | 61% | 61% | 61% |
  | inv-r6 | 2,69 | 55% | 55% | 55% | 55% | 55% |
  | maliss | 1,13 | 100% | 100% | 88% | 88% | 88% |
  | tearlaments | 2,73 | 56% | 56% | 56% | 69% | 69% |

  - Al budget del banco (4000 nodi) la ricerca normale arriva al 55-91% dell'oracolo in 7 mani su 8. A 16 000 nodi arriva al 95% in 3 su 8.
  - Una board con il 99% del valore dell'oracolo è fra le prime 3 in 1 mano su 8. La board stessa dell'oracolo in nessuna: spesso c'è una board diversa con lo stesso valore (mostri Link in più che la misura non conta). Per questo `measure` riporta tutte e due le posizioni.
  - **Il valore misurato non cresce sempre con il budget:** maliss da 100% a 88%, inv-r5 da 68% a 61%. Le board da misurare sono le 10 migliori per la stima rapida. Con più nodi la stima ne mette davanti altre, e la board che si misurava meglio esce dalla lista. È lo stesso problema delle inversioni tra stima e misura (C5.7).
- **Obiettivo Invoked** (Oceanus nella Zona Mostri Extra + S:P). Con la mano Aleister, "Grail", Magical Meltdown, Ash e Fuwalos, nemmeno 40 000 nodi trovano una board con tutte e due. A 4000 nodi escono board con S:P e Caliga o Mechaba, oppure con Oceanus senza S:P.
- **Aeon a 3 materiali: il motore la sa giocare.**
  - In una posizione costruita (Invoked Sorath, Magistus Chorozo e Invoked Oceanus nel Cimitero), l'effetto di Sorath dal Cimitero offre Hyperinvoked Aeon. Scelti Sorath e Oceanus, il motore offre sia «finisci» sia Chorozo come terzo materiale.
  - La linea migliore trovata su inv-r5 Evoca Aeon con 2 materiali (Sorath e Chorozo), ma lì un terzo valido non c'è: nel Cimitero Invoked Elysium è LIGHT come Sorath, e Aeon vuole attributi diversi. Elysium ha tutti gli attributi solo scoperta sul campo.
  - Quindi niente azione non offerta né risposta automatica sbagliata. Manca una linea che prepari tre Fusioni di attributo diverso, e la misura non dà alla ricerca un motivo per farlo: sono le cecità della misura descritte sopra (Aeon vale quasi zero, il kit ha un solo mostro nell'Extra Deck).
- **Il giro vero.** Lo fa la corsia 6 in una sessione cloud, per decisione del proprietario, sul branch `claude/oracle-run`:
  ```
  python3 scripts/oracle.py build <out> --decks <mdm> --mult 50 --own-seed-only --jobs 4
  python3 scripts/oracle.py fixtures <out> tests/fixtures/oracle --decks <mdm>
  python3 scripts/oracle.py measure tests/fixtures/oracle <out>/measure --decks <mdm> --budgets 1000,2000,4000,8000,16000
  ```
  - Si parte con il solo seme della mano, perché è l'unico che fa la fixture. Costa circa un terzo delle tre corse.
  - I semi 2 e 3 si aggiungono dopo (`--seeds 1,2,3`).
  - Un comando in background nel cloud si ferma dopo 2 ore. `build` riparte dalle corse già scritte, quindi si rilancia la stessa riga finché ha finito.
  - Dal pilota, una mano a 40 000 nodi prende da 5 s a 6,5 minuti; a 200 000 nodi va da qualche minuto a mezz'ora.

#### A1: il giro vero (corsia 6, 23/10, branch `claude/oracle-run`)
- **Oracolo:** 91 mani a 200 000 nodi (50 volte), solo con il seme della mano, in circa 4 ore e mezza nel cloud con 4 lavori. Una mano prende da pochi secondi a 15 minuti, in media 10. Le fixture di `tests/fixtures/oracle` sono queste e prendono il posto del pilota.
- **La ricerca normale contro l'oracolo** (`oracle.py measure`, a 1000-16 000 nodi), su 88 mani. Restano fuori le 2 mani kaiju, dove nessuna board vale qualcosa, e l'obiettivo Invoked:

  | nodi | quota mediana | quota media (fino al 100%) | mani al 95% o più | mani sopra l'oracolo | secondi (mediana) |
  |---|---|---|---|---|---|
  | 1000 | 69% | 71% | 27 | 8 | 2,9 |
  | 2000 | 77% | 75% | 29 | 7 | 5,0 |
  | 4000 | 87% | 80% | 31 | 9 | 8,4 |
  | 8000 | 90% | 83% | 35 | 9 | 17,1 |
  | 16 000 | 95% | 86% | 44 | 11 | 31,9 |

  - L'obiettivo del piano (95% dell'oracolo entro 30 secondi in 9 mani su 10) è lontano: a 16 000 nodi, circa 32 s, ci arriva una mano su due.
  - Con una board da almeno il 99% dell'oracolo fra le prime 3 sono 39 mani su 91; con la board stessa dell'oracolo 23 (obiettivo: 8 su 10).
  - **Il valore misurato non cresce sempre con il budget:** in 28 mani su 88 scende di oltre l'1% passando a un budget più grande.
  - **L'oracolo non è ancora un tetto.** In 14 mani la ricerca normale trova una board migliore di quella dell'oracolo (vanquish_soul_k9-0: 182%, phantom_knights-1: 141%). L'oracolo misura solo le 10 board migliori per la stima rapida, tra migliaia trovate. Così la board migliore può restare fuori dalla misura anche con 50 volte i nodi. Lo stesso limite spiega le discese. Per l'A1b: l'oracolo dovrebbe misurare molte più board (`--top` 50 o più), o misurarle tutte per ogni aspetto della board.
  - **Obiettivo Invoked** (Oceanus nella Zona Mostri Extra + S:P, mano Aleister, "Grail", Meltdown, Ash, Fuwalos). A 200 000 nodi l'oracolo lo raggiunge (valore 1,74). La ricerca normale no, nemmeno a 16 000 nodi.
- **Da fare:** i semi 2 e 3 (seconda passata) e l'A1b.

#### A1: l'oracolo con la misura a gradini (corsia 6, 23/10, branch `claude/oracle-top`)
- **`oracle.py build --staged 300,30`:** sulle prime 300 board per la stima rapida la misura economica (B14), sulle 30 migliori per quella la misura completa. Prima si misuravano per intero solo le prime 10. `--only` accetta più mani, separate da virgole.
- **Prova** a 200 000 nodi, solo con il seme della mano, su 29 mani: le 14 dove la ricerca normale batteva l'oracolo, più 15 prese a caso. Stesso eseguibile per tutto: l'oracolo con le prime 10 misurate dà gli stessi valori del giro vero.

  | | prime 10 misurate | misura a gradini 300/30 |
  |---|---|---|
  | mani dove la ricerca normale (1000-16 000 nodi) supera l'oracolo di oltre l'1% | 14 | 3 |
  | mani dove l'oracolo sale di oltre l'1% / scende | — | 15 / 1 |
  | tempo per mano (mediana) | — | +123 s (×1,39); in tutto +42% |

  - Gli aumenti più grandi: warrior_combo-0 da 0,38 a 1,78, vanquish_soul_k9-0 da 0,90 a 2,02, phantom_knights-1 da 1,16 a 2,24, artmage-0 da 2,79 a 4,02. Erano board che la stima rapida metteva fuori dalle prime 10.
- **Le tre eccezioni rimaste:**
  - **warrior_combo-1 (148%):** la board della ricerca normale è fuori dalle prime 300 per la stima. Con `--staged 2000,60` l'oracolo sale da 0,26 a 1,78 e la ricerca normale scende al 21%.
  - **elfnote-1 (102%):** la board c'è, al posto 44, ma per la misura economica vale 2,22, sotto la soglia delle 30 (2,28). Misurata per intero vale 3,27. La ricerca normale arriva a 3,92 misurando anche le varianti di zona della stessa board. Con 2000/60 l'oracolo fa 3,78 e la ricerca normale resta al 104%.
  - **sky_striker-1 (127%):** non è un limite della misura. Dopo 200 000 nodi l'oracolo ha 20 757 board ma solo 425 aspetti (campo e mano). La board migliore della ricerca a 1000 nodi non c'è: stesso campo (Sky Striker Ace = Zero nella Zona Mostri Extra, Widow Anchor coperta), ma con Maxx "C" ancora in mano. La ricerca lunga non passa per quello stato: la ricerca non è monotona nel budget. È un caso per la corsia 2.
  - code_talker-0 resta al 101%, entro la soglia.
- **Unica mano peggiorata:** sky_striker-1 scende da 1,09 a 0,93. La misura economica mette fuori dalle 30 la board che con le prime 10 era la migliore.
- **Proposta:** rifare le 91 fixture con `--staged 300,30`, circa 6 ore e mezza nel cloud invece di 4 e mezza. Con 2000/60 l'oracolo costa ancora di più: elfnote-1 passa da 638 a 1730 s.

#### A1: il giro con la misura a gradini (corsia 6, 23/10, branch `claude/oracle-staged`)
- **Oracolo:** le 91 mani a 200 000 nodi, solo con il seme della mano, con `--staged 300,30`. warrior_combo-1 è fatta con `--staged 2000,60`, perché la sua board migliore era oltre le prime 300. Il codice è quello di `main` dopo #93 e #94, che cambiano la misura. In media 8,6 minuti per mano, con 4 lavori. Le fixture di `tests/fixtures/oracle` sono queste e prendono il posto di quelle del giro vero.
- **Ricerche per obiettivo:** le board che raggiungono l'obiettivo vengono prima nella lista, quindi la misura completa va alle prime 30, non alle 30 migliori per la misura economica. Altrimenti restavano tutte con la sola misura economica, e la fixture veniva vuota.
- **La ricerca normale contro l'oracolo nuovo**, su 88 mani. Restano fuori le 2 mani kaiju, dove nessuna board vale qualcosa, e l'obiettivo Invoked:

  | nodi | quota mediana | quota media (fino al 100%) | mani al 95% o più | mani sopra l'oracolo (oltre l'1%) | secondi (mediana) |
  |---|---|---|---|---|---|
  | 1000 | 57% | 62% | 16 | 1 | 3,1 |
  | 2000 | 62% | 67% | 17 | 1 | 5,0 |
  | 4000 | 73% | 71% | 19 | 1 | 7,5 |
  | 8000 | 79% | 74% | 23 | 1 | 12,7 |
  | 16 000 | 83% | 76% | 25 | 1 | 21,6 |

  - **L'oracolo ora è quasi un tetto.** La ricerca normale lo supera di oltre l'1% solo in 2 mani a qualche budget: sky_striker-1 (127%, la ricerca a 200 000 nodi non arriva allo stato che a 1000 nodi trova: alla corsia 2) ed elfnote-1 (102%, varianti di zona della stessa board).
  - **Rispetto al giro vero** l'oracolo sale di oltre l'1% in 50 mani su 88 e scende in 2. Quindi la ricerca normale ne è più lontana di quanto sembrava: a 4000 nodi la quota mediana passa dall'87% al 73%, a 16 000 dal 95% all'83%.
  - Una board da almeno il 99% è fra le prime 3 in 24 mani su 91; la board stessa dell'oracolo in 18.
  - In 27 mani su 88 il valore misurato scende ancora di oltre l'1% passando a un budget più grande: la ricerca normale misura per intero solo le prime 10 board.
  - **Obiettivo Invoked:** l'oracolo lo raggiunge con valore 1,25 (Oceanus nella Zona Mostri Extra, S:P, Baybarron, Virakam, "Sword", Meltdown). La ricerca normale non lo raggiunge nemmeno a 16 000 nodi.

### A1b. Oltre i limiti dell'oracolo (corsia 6, dopo il giro vero; il livello 2 con la corsia 3)
L'oracolo è uno strumento di sviluppo, non una parte dell'app.
- Serve a controllare che il motore trovi da solo le linee più forti di **qualunque** deck, anche uno appena montato con carte appena uscite.
- Non usa nomi di carte: è la stessa ricerca con più budget, quindi vale per ogni deck.
- I deck del banco sono solo un campione vario di meccaniche. Quando le liste del meta cambiano, non si rifà niente.

Ha due limiti, più un rischio:
- è la board migliore *trovata* con molto budget, non per forza la migliore che esiste;
- le sue board valgono quanto la misura, quindi ne eredita le cecità;
- il rischio è adattare il motore ai deck del banco invece di renderlo generale.

Quattro passi, in quest'ordine.

1. **Livello 2 sulle board dell'oracolo e rapporto dei disaccordi.** È il passo con più valore, e gli strumenti ci sono già.
   - Ogni board delle fixture si gioca con `probe-line --vs X --moves` contro i deck del meta, pesati con la popolarità come nel banco oggettivo (`banco-oggettivo.md`).
   - Si scrivono le coppie di board dove la misura e il livello 2 non sono d'accordo, cioè l'ordine si inverte di almeno 0,05 di verità. Ognuna è una cecità della misura da correggere in B5 (esempi: Aeon, interruzioni dal Cimitero, trappole in mano, turno dopo).
   - Si aggiungono le ricerche per obiettivo automatiche (`--auto-goals N`): una board che raggiunge un mostro dell'Extra Deck, forte al livello 2 ma bassa per la misura, è una cecità trovata senza indovinarla.
   - Le fixture tengono anche board diverse, non solo le 5 migliori per punteggio. Per esempio la migliore per interruzioni, per risorse del turno dopo e per protezioni. Così un tipo di board che la misura sottovaluta resta fra i riferimenti.
   - **Controllo:** il rapporto elenca i disaccordi per deck, con la linea da rigiocare. Ogni correzione di B5 deve ridurli, senza peggiorare il test appaiato.
2. **Convergenza.** Su un campione di 12 mani (una per tipo di deck, con le 4 più lontane dal pilota) si lancia a 100× e 200× i nodi.
   - Una mano il cui valore non cresce più da 50× a 200× ha il tetto stabile.
   - Le altre si segnano «provvisorie» nella fixture, e `measure` le riporta a parte.
   - **Controllo:** la quota di mani stabili. Se è sotto l'80%, il giro vero passa a 100×.
3. **Generalità: deck mai visti.** Ogni modifica si giudica anche su deck che non hanno guidato il lavoro, come in D17 (70% per tarare, 30% per verificare).
   - Quando escono archetipi nuovi, i loro deck entrano come casi di prova nuovi. Le fixture dei deck vecchi restano come sono.
   - Se una lista cambia, le sue mani si saltano (`deck_hash`) e non si ricalcola niente.
   - Su un deck nuovo bastano poche mani con un oracolo breve, per vedere se la ricerca normale ci arriva vicino quanto sui deck noti.
   - **Controllo:** sui deck tenuti da parte, la quota del valore dell'oracolo raggiunta dalla ricerca normale non è più bassa di oltre 5 punti rispetto ai deck usati per lavorare. Se lo è, la modifica è adattata al banco e non generale.
4. **Più avanti, dopo l'arena nel turno di prova (B13).**
   - **Ricerca esaustiva sulle mani piccole:** senza potature e con le trasposizioni, fino a un tetto di nodi. Dove finisce, il migliore è certo e misura quanto ci arriva l'oracolo.
   - **Secondo cercatore indipendente:** ygo-combo-solver, con la stessa versione del core e degli script. Una board migliore della nostra abbassa il tetto e dà la linea da rigiocare, come per B4a.
   - **Controllo:** l'oracolo arriva ad almeno il 99% dell'esaustiva sulle mani dove questa finisce.

#### A1b: risultati (corsia 6, 23/10, branch `claude/oracle-a1b`)
Fermata il 23/10 per decisione dell'utente. I numeri sono quelli dei calcoli finiti fino a quel momento.

- **Strumenti nuovi in `scripts/oracle.py`:**
  - `fixtures` tiene anche le board di tipo diverso: la migliore per interruzioni sul campo, per resilienza e per risorse del turno dopo. Ogni board ha la sua linea, in esadecimale e senza testo di carta.
  - `lines` esporta le linee di una fixture per `probe-line`.
  - `as-hunt` scrive le fixture nel formato di `combo_hunt --json`, per `objective_bench.py`.
  - `disagree` scrive il rapporto dei disaccordi.
  - `converge` segna ogni fixture come stabile o provvisoria.
  - `measure` riporta la quota media per i deck di taratura e per quelli di verifica (70/30, seme 47, come il banco oggettivo), e per le mani stabili e provvisorie.
- **Passo 1, livello 2 sulle board dell'oracolo.**
  - `objective_bench.py` senza modifiche, sulle board di 82 mani: 388 board, contro i primi 6 avversari del meta del 22/10, che pesano il 55%.
  - 492 invocazioni, 12,8 ore in tutto con 3 lavori, mediana 77 s. 9 sono andate oltre il limite.
  - La verità di una board è 1 − `left`, pesata con la popolarità. Nessuna coppia dubbia.
  - La board migliore per la misura è anche la migliore per la verità, entro 0,05, in 74 mani su 82.
  - **48 coppie in disaccordo** di almeno 0,05: la misura mette prima una board, la verità l'altra. Sono in `tests/fixtures/oracle-disagreements.json`, con la board da rigiocare. Per deck: warrior_combo 8, dracotail 7, maliss 6, primite 4, yummy 4, orcust 4, blue_eyes 4, sky_striker 3, power_patron 2, snake_eye 2, elfnote_kewl_tune 2, railway 1, odion 1.
  - **I casi più grandi:**
    - dracotail-1: la #1 per la misura (2,58) ferma il 36% delle mosse del meta. La #5 (2,07: Albion, Ecclesia and the Dark Dragon, Fallen of the White Dragon, Predaplant Verte Anaconda nella Zona Mostri Extra, tre carte coperte) ne ferma l'80%.
    - primite-1: quattro board da 3,70 fermano il 57-59%. Una da 3,69, con Solemn Strike, Dominus Purge e Primite Drillbeam coperte e Baronne nella Zona Mostri Extra, ferma l'87%. Le trappole coperte valgono poco nella misura.
    - power_patron-0 e yummy-1: differenze di 0,13-0,16.
  - **Le board tenute per tipo servono.** In 6 mani una board tenuta per resilienza, interruzioni o turno dopo batte la migliore per la misura di almeno 0,05: dracotail-1 (+0,26), primite-1 (+0,28), warrior_combo-0 e -1, maliss-1.
- **Passo 2, convergenza.**
  - 12 mani: le 4 più lontane dall'oracolo, da deck diversi, e 8 di altri deck. A 400 000 nodi (100×) 10 su 12 hanno il tetto stabile; si alzano branded-1 (da 1,70 a 2,03) e mitsurugi_yummy-0 (da 3,15 a 3,27).
  - Il giro a 800 000 nodi (200×) è **non completato, fermato il 23/10 per costo**: 6 mani su 12. Due corse alla volta a 400 000 nodi bastano a esaurire la memoria (15 GB), per le 50-90 mila board tenute.
  - Con le 6 mani fatte, clown_crew-1 sale ancora dell'1,5%. Contando anche questo, le mani stabili sono 9 su 12 (75%), sotto l'80% del controllo. Le 3 provvisorie sono segnate nelle fixture (`convergence`).
- **Passo 3, deck tenuti da parte.** A 16 000 nodi la ricerca normale arriva in media al 75,8% dell'oracolo sui 59 deck di taratura e al 77,6% sui 29 di verifica. I deck di verifica non sono sotto di oltre 5 punti: con il codice di oggi niente è adattato al banco. Va rifatto a ogni modifica della ricerca o della misura.
- **Passo 4:** non cominciato. Viene dopo B13.

### A2. Dove va il lavoro della ricerca: fatto (corsia 2, PR #67)
`DUELFORGE_SEARCH_STATS=1` e `scripts/search_stats.py`; numeri in `velocita-ricerca.md`.

| Spreco | % nodi | % tempo della ricerca |
|---|---|---|
| Replay delle linee dal principio | — | 43% |
| Discese che finiscono su un nodo già esplorato (altre scelte 15,6%, altro ordine 9,3%) | 26,3% | 18% |
| Nodi senza cambi visibili (selezioni a più carte, Evocazione prima dei materiali) | 23,8% | — |

- **Ordinamento:** la risposta giusta è già la prima provata nel 77% dei passi della linea migliore.
- **Fuori dalla ricerca:** in `combo_hunt` il turno di prova è il 58% del tempo totale, le fasi dopo la ricerca il 15%, l'apprendimento il 4%.
- **Contatori da aggiungere:**
  - in che posizione di `dive_order` escono dalla coda i fratelli (B1a);
  - gli stati a novità zero (B11);
  - un controllo a campione delle trasposizioni (B3).

## Fase B: correggere
Ogni correzione è generale: niente nomi di carte. Segue il protocollo del motore (`AGENTS.md`): banco, `combo_hunt --seed 7`, `search_regression`, più il test appaiato e, quando c'è, l'oracolo A1. Le idee e le fonti di ogni punto sono in `ricerca-letteratura.md`.

### Velocità della ricerca (corsia 2, `search.rs` e `ygo-engine`)
- **B0. Il replay.** Studio delle strade in `velocita-ricerca.md`, B0.
  - **Prima un prototipo di misura dell'arena di memoria (B13).** È l'approccio di ygo-combo-solver: le allocazioni del core e di Lua vanno in un'arena a indirizzo fisso, e un checkpoint copia le pagine scritte. Salva e ripristina il duello a metà turno, coroutine Lua comprese. Gli agganci ci sono già (`l_alloc` in `build.rs`, `shim/new.cpp`), e non serve toccare `vendor/ygopro-core`.
  - **Se ripristinare costa molto meno del replay medio (5,6 ms)**, si fa l'arena. Altrimenti la strada 4: una copia del duello dopo le carte aggiunte, sopra il riuso dei duelli dello shim.
  - **Controllo:** uscite identiche sul banco e controlli di `DUELFORGE_CHECK_REUSE` puliti. Nodi al secondo sul banco ×1,15 con la strada 4, ×1,5 con l'arena.
- **B1a. Ordine dei fratelli.** A parità di priorità la coda prende oggi la risposta più nuova, e i fratelli entrano nell'ordine di `dive_order`: escono per prime le alternative meno preferite (Passa, No, Annulla, Set).
  - Da verificare con il contatore A2, poi la correzione è piccola.
  - **Controllo:** nessuna inversione nel contatore; `best` del banco non scende.
  - **Misurato il 24/10, non tenuto.** Il contatore c'è (`DUELFORGE_SEARCH_STATS=1`, voce `siblings`; `search_stats.py` lo somma). Su `main` il 46% dei fratelli esce dalla coda dopo uno che il tuffo aveva messo più avanti, alla stessa priorità; con la correzione (le risposte di un nodo messe in coda dall'ultima, così esce prima la preferita) 0%, e il primo fratello preso è il secondo del tuffo nel 98% dei nodi invece del 66%.
  - Il risultato non si muove in una direzione. `paired_test.py` (88 mani, 4000 nodi): 15 meglio, 9 peggio, p = 0,31, una mano −42% (Lunalight, con la stessa stima migliore: cambia quali board passano alla misura), quindi respinto. Un secondo campione di 80 mani nuove (le mani 2 e 3 di ogni deck, seme 7): 11 meglio, 13 peggio, peggiore −25%. Sulle prime 88 mani la stima migliore della ricerca sale in 22 e scende in 12; sulle 160 il tempo della ricerca cresce dell'1% (865 → 875 s).
  - Lettura: tra fratelli alla stessa priorità l'ordine conta poco, perché a 4000 nodi la ricerca li prova quasi tutti; spostare l'ordine sposta a caso quali board finiscono nella top 10. L'ordinamento che conta va fatto con priorità diverse, cioè B1b e B1c (history e penalità per rango), che rendono B1a inutile: lì il contatore serve da controllo.
- **B2. Ordini scambiabili.** Riduzione per ordine parziale, solo nella forma prudente (sleep set): DPOR presuppone una ricerca in profondità.
  - **Controllo:** i nodi doppi «altro ordine» scendono dal 9,3% a meno del 3%, con le stesse board migliori.
  - **Misurato il 24/10, non fatto.** Dopo B13 i doppi «altro ordine» sono l'8,3% dei nodi; quelli a un momento libero, il caso dei sleep set, il 6,3% dei nodi e il 3,4% del tempo della ricerca. È il tetto del guadagno, sotto il costo di una prova di commutatività e il rischio di perdere stati (`velocita-ricerca.md`, «B2 e B2b»).
- **B2b. Selezioni a più carte come un solo passo**, e l'Evocazione con i suoi materiali.
  - **Controllo:** nodi del banco −15% senza perdere stati distinti (confronto delle chiavi).
  - **Misurato il 24/10, non fatto.** Le scelte di una selezione si incontrano già come insieme: i doppi che restano sono lo 0,4% dei nodi. I nodi senza cambi visibili (22,3%) sono ramificazioni vere; contarli come un passo cambierebbe il conto del budget, non il lavoro.
- **B3. Chiave dei nodi con le scelte che contano** (bersagli e nomi dichiarati, §30.8). In più, in debug, un controllo a campione che due cammini verso lo stesso nodo offrano le stesse azioni.
- **B1b e B1c. Ordinamento imparato durante la ricerca.**
  - B1b, history relativa e di continuazione per offerta (tipo di azione, codice, luogo);
  - B1c, penalità di deviazione che cresce con il rango in `dive_order`, al posto dello 0,6 uguale per tutte.
  - **Controllo:** tempo per arrivare alla board migliore sul banco −20%, e `best` non scende più del 2%.
- **B10, B11, B16.** Tutti con controllo: tempo alla board migliore −10%, `best` non più basso.
  - B10, priorità ricalcolate dopo l'apprendimento («epoca dei valori»);
  - B11, più code alternate (forza, distanza dall'obiettivo, azioni utili, per tipo) e novità come chiave secondaria;
  - B16, tempo gestito dalla stabilità del migliore.
- **B6. Più thread che rendano.** Si rivede dopo B0.
- **B7. Partire da linee già trovate.** Prefissi promettenti dalla mappa dei collegamenti e dalle ricerche precedenti, una tabella con la linea più economica per ogni mostro dell'Extra Deck Evocato, e macro di 2-4 passi.

### Ricerca per obiettivo
- **B4. Ricerca per obiettivo guidata dalla mappa dei collegamenti.** È pianificazione su un grafo implicito: la «ricerca da labirinto» del proprietario.
  - Le tecniche:
    - una distanza rilassata (h_add) calcolata sulla mappa, in un modulo nuovo `relax.rs`;
    - le «azioni utili» del piano rilassato, provate per prime;
    - un credito all'obiettivo imparato durante la ricerca (AMAF);
    - dopo la prima linea che raggiunge l'obiettivo, si continua a migliorarla attorno.
  - La distanza serve solo a ordinare, mai a tagliare, così il verdetto «impossibile» resta corretto.
  - Tra le linee che raggiungono l'obiettivo vince quella con la board più forte.
  - **Controllo:** nodi alla prima soluzione −30% su un insieme fisso di obiettivi raggiungibili; i verdetti «impossibile» restano uguali.
  - **B4a, fatto (22/10, corsia 2):** dalla diagnosi in `ricerca-esperimenti.md` §3, una carta dell'obiettivo scelta in una selezione che la manda via (suggerimenti del motore 500-504, 507, 511-513, 533) va per ultima nella discesa, e il ramo perde priorità in coda se era l'ultima copia. Le 4 mani Invoked, 1 thread, 60 s: «Oceanus + S:P» da 0/4 a 4/4 (1,3-5,9 s), «Oceanus@extra + S:P» da 0/4 a 4/4 (7,0-28,8 s). Test `a_goal_card_is_not_sent_away_first` in `search_regression`.
- **B8. Obiettivi più ricchi.**
  - Ogni carta dell'obiettivo dice il posto: Mostro Principale o Extra, Magia/Trappola coperta o scoperta, Terreno, Cimitero, bandita, mano.
  - Ci sono anche atomi sull'avversario, come le carte tolte dal suo Extra Deck, e il segno duro o morbido.
  - Gli atomi fuori dal campo si controllano sulla board esatta, dopo la Fase Finale.
  - Il contratto `lib.rs` ↔ `api.ts` cambia in una PR di Claude; poi Gemini fa l'interfaccia. Gli obiettivi salvati oggi restano validi.
  - In un secondo passo, vincoli sui materiali (ad esempio Aeon con 3 o più materiali).
  - **Controllo:** un obiettivo con una Trappola coperta e una carta nel Cimitero si raggiunge su Labrynth e Branded, e un obiettivo impossibile riceve il verdetto giusto.

### La misura (coordinatore, `probe.rs`, `value.rs`, `lib.rs`)
- **B5a. Attrito avversario** (dopo D17).
  - Le parti:
    - un Extra Deck realistico per l'avversario, circa 15 mostri generici;
    - la metrica «risorse tolte all'avversario» nel tuo turno;
    - lo stesso termine nella stima della ricerca;
    - l'«aspetto» della board che distingue le carte tolte;
    - le scelte nell'Extra Deck avversario rigiocate per identità;
    - il sì opzionale degli script mai rifiutato in automatico;
    - le correzioni dell'Observer per controllo e rimozione.
  - **Controllo:** su Invoked la board con Aeon a 3 materiali sta sopra quella a 2, nella misura e nella stima.
  - Cambia tutti i punteggi: il banco oggettivo si rifà come nuova base, perché i numeri vecchi non sono più confrontabili.
- **B5b. Risposte ai board breaker diverse dalla negazione:** floater, Evocazioni rapide, protezioni sostitutive (30 e 28 deck su 37). Una run «react» accanto a quella in cui si passa.
  - **Controllo:** la resilienza dei deck con floater cambia; la correlazione con il livello 2 non scende; `combo_hunt` senza problemi nuovi.
- **B5c.** Protezione dei pezzi nel punteggio, turno 3 dal campo, Cimitero e bandite nella stima, danno e Life Point, tassa pagabile.
- **B5d.** Valori delle carte per «firma di costruzione»: materiali, Attributi, contatori, tipo di Evocazione.
- **B5e e B5f.** Kit più ampio ed esposizione letta dal motore: più avanti, con il valutatore imparato.
- **B9. Stima con l'avversario, tarata sui dati** (Texel, perdita a coppie), dopo B5a e B1.
  - **Controllo:** inversioni tra stima e misura nella top 8 (C5.7) da 174 a 60 o meno su 1110.
- **B14. Quali board misurare: dimezzamenti successivi.** Oggi l'app misura a piena fedeltà 30 board; il turno di prova è il 58% del tempo di `combo_hunt`. I gradini:
  1. stima rapida su 40-60 board;
  2. misura senza breaker su tutte, si tiene la metà;
  3. misura completa su 10-15;
  4. livello 2 su 3-4.
  - **Controllo:** tempo della valutazione nell'app −50%, con il #1 identico e almeno 2 board su 3 uguali nella top 3.

### Più avanti
- **B12.** Una politica imparata dall'oracolo (DAgger, perdita a coppie), mai addestrata sulle linee della ricerca stessa.
- **B15.** Taratura delle costanti della ricerca con SPSA o CEM, a budget di nodi; corse lunghe lanciate dal proprietario.
- **B17.** Probabilità di riuscita delle linee su più ordini del Deck (8-16 semi).

## Obiettivi misurabili
- **Oracolo A1:** la ricerca normale arriva al 95% del valore dell'oracolo entro 30 secondi in 9 mani su 10, e la board dell'oracolo è nella top 3 in 8 su 10.
- **Tempo** per arrivare alla board migliore sulle mani del banco: mediana dimezzata.
- **Forza:**
  - la verità del banco oggettivo delle board migliori non scende;
  - `best` del banco a nodi fissi non scende più del 2%.
- **Invoked:** l'obiettivo Oceanus nella Zona Mostri Extra + S:P si raggiunge con Aeon a 3 materiali, e la board viene classificata sopra quella con Aeon a 2, già nella stima della ricerca.
- **Stima contro misura:** 60 inversioni su 1110 o meno (C5.7).
- **App:** tempo della valutazione dimezzato, con la stessa top 3.
- **Obiettivi ricchi:** una Trappola coperta e una carta nel Cimitero si chiedono e si raggiungono.

## Ruoli (correzione piccola, a parte)
File: `crates/ygo-solver/src/roles.rs` e i suoi test.
- **Board breaker dal testo.** Oggi `breakers_by_text` cerca frasi come «destroy all» o «your opponent controls». Va riconosciuto anche chi usa le carte dell'avversario come materiale o come Tributo: «from either field», «monster(s) your opponent controls» insieme a «material» o «Tribute». Così Super Polymerization diventa board breaker, e con lei ogni carta con lo stesso schema, senza nomi.
- **Starter ed estensore insieme.** Oggi uno starter non si prova mai come estensore (`else if`). Uno starter va provato anche come estensore sulla linea di un altro starter: per lo starter migliore si usa la linea del secondo.
- **Estensore e board breaker insieme** è già possibile. Va capito perché Invocation «Grail» non risulta estensore. Le cause possibili:
  - la soglia `EXTENDER_GAIN`;
  - un materiale preso all'avversario, che giocando per primi non c'è;
  - il budget di 400 nodi.

  La correzione deve valere per ogni deck.
- **Controllo:** `duelforge-cli roles` prima e dopo su almeno 10 deck di Master Duel Meta, con l'elenco delle carte che cambiano ruolo, ognuna verificata sul testo.

### Ruoli: fatto (corsia 6, 22/10, branch `claude/roles-fix`)
- **Board breaker dal testo.** Una frase che usa i mostri dell'avversario come materiale, come Tributo o per una Fusione («from/on either field», «monster(s) your opponent controls») rende la carta board breaker. Le frasi si dividono anche ai due punti: «se è mandata al Cimitero come materiale: distruggi 1 mostro che controlla l'avversario» toglie un mostro con un effetto, non lo usa (Dracotail Mululu e Kewl Tune Mix restano fuori). Lo stacco di un materiale Xyz non conta.
- **Starter ed estensore insieme.** Ogni carta si prova come estensore sulla linea dello starter migliore; lo starter migliore sulla linea del secondo.
- **Invocation - "Chalice"** («Grail» in Master Duel). Il budget non c'entra: con 3000 nodi il risultato è lo stesso. Le altre due cause c'entrano:
  - la Fusione bandisce un tuo mostro e uno dal Deck. All'ultimo momento libero della linea di Magical Meltdown resta solo la board: "Chalice" bandisce S:P Little Knight per Evocare Oceanus, e la stima sale solo di 0,83;
  - nella lista Invoked della fixture, all'ultimo momento non si usa affatto. Un passo prima (quando Aleister ha già fatto il suo lavoro) vale +5,3.
- **La correzione è in due parti.**
  - La carta si prova negli ultimi 2 momenti liberi della linea, non solo nell'ultimo.
  - Basta un guadagno di 0,75 se la carta porta sul campo una carta del deck che la board senza di lei non ha, diversa da sé stessa.
  - La soglia di 1,5 resta per il resto: una carta solo coperta (+1,0) o usata per sé stessa (Crossout Designator +0,6) non porta niente.
- **Controllo:** `duelforge-cli roles` prima e dopo sui 41 deck di Master Duel Meta. Ruoli cambiati per 94 carte; ognuna è stata letta sul testo.

  | Cambio | Carte |
  |---|---|
  | + board breaker | 12: Super Polymerization (7 deck), Fallen of Albaz (2), Eternal Favorite, Megalith Anastasis e Megalith Notrah Plura («Tribute 1 monster your opponent controls»; era mattone) |
  | starter → starter ed estensore | 41 |
  | + estensore | 36, più 5 mattoni che diventano estensori (DoomZ XII Zero - Drastea, Super Express Bullet Train, Junoldo, Noh-P.U.N.K. Deer Note, Diabellstar Vengeance) |

  - Estensori in tutto: da 67 a 149; board breaker da 140 a 152. Nessun ruolo perso.
  - Su Invoked diventano estensori Invocation, "Sword", "Chalice", Aleister the Reminiscent; Aleister, Aiwass e Magical Meltdown diventano starter ed estensori; Super Polymerization diventa board breaker.
  - **Dubbi**, dal testo: Fydraulis Harmonia (handtrap), Rainbow Bridge of Salvation e Sinful Spoils of Betrayal - Silvera estendono poco o niente. Restano perché superano la soglia nella stima.
  - **Già prima:** le Trappole coperte risultano starter o estensori (Solemn Judgment, Solemn Accusation, Forbidden Crown), perché la stima rapida conta ogni interruzione coperta. Ora le Solemn di odion e primite sono «starter ed estensore»: è la stessa regola, non un difetto nuovo.
- **Tempo:** da 330 a 609 s sui 41 deck, cioè da 8,0 a 14,8 s a deck. La linea in più e il secondo momento costano; il secondo momento si prova solo se il primo non basta.
- Test: `crates/ygo-solver/tests/roles.rs` (Invoked: "Chalice" estensore e board breaker, Super Polymerization board breaker) e un test unitario sulle frasi.

## Chi fa cosa
I file caldi (`search.rs`, `probe.rs`, `value.rs`, `lib.rs`, `api.ts`) li tocca un agente alla volta. I moduli nuovi (`relax.rs`, la history, l'arena in `ygo-engine`) si scrivono in parallelo; si serializza solo l'aggancio.
- **Corsia 2 (velocità):** in quest'ordine, una correzione per PR.
  1. B0, con il prototipo dell'arena;
  2. B1a;
  3. B2 e B2b;
  4. B3;
  5. B1b e B1c;
  6. B8, motore e contratto;
  7. B10, B11 e B16;
  8. B4, dopo B8 e B9.
- **Corsia 6:** i ruoli, poi l'oracolo A1 con lo script del test appaiato.
- **Coordinatore:**
  1. D17, i pesi;
  2. B5a;
  3. B14 (`lib.rs`);
  4. B5b, poi B5c e B5d;
  5. B9, insieme alla corsia 2, dopo B5a e B1.
- **Gemini:** l'interfaccia di B8, dopo il contratto.
- **Sessione sorella di ricerca (proposta, da decidere con il proprietario).** Fa esperimenti offline sui dati del banco:
  - fit Texel sui JSON di `combo_hunt`;
  - correlazione tra i gradini di misura di B14;
  - il controllo con ygo-combo-solver sul caso Invoked.

  Non tocca né i file caldi né il motore.
