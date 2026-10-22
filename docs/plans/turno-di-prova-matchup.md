# Turno di prova contro un deck scelto (#44): proposta

*Scritta il 21/10/2026 da Claude Code. Scelte del proprietario: livello 1 adesso, deck avversario dalla Bottega, punteggio "contro X" accanto a quello generico. Il livello 1 è fatto nel backend (sotto, "Livello 1: com'è fatto").*

## Obiettivo
Oggi una board si misura contro un avversario generico: un kit di carte semplici che fanno una cosa sola (11 azioni tipiche) e 9 board breaker comuni. L'esposizione della linea considera le handtrap più giocate (Ash Blossom, Veiler e Impermanence, Nibiru, Droll, Maxx "C").

L'obiettivo è misurare la board contro il deck che si incontrerà davvero, scelto dall'utente.

## Tre livelli possibili

| Livello | Cosa cambia | Costo | Fedeltà |
|---|---|---|---|
| **1. Breaker e handtrap dell'avversario** | Resilienza ed esposizione si misurano con le carte del deck scelto | medio: backend 3-4 ore, un selettore nell'interfaccia | buona per "la mia board regge ai suoi breaker?" e "le sue handtrap fermano la mia linea?" |
| **2. Azioni dell'avversario** | Anche le interruzioni: il kit usa le giocate tipiche del deck scelto | alto | buona anche per "cosa ferma la mia board di quel deck" |
| **3. Partita simulata** | Il motore cerca la linea migliore dell'avversario contro la board | molto alto (minuti per board) | la più fedele |

### Livello 1 nel dettaglio (consigliato come primo passo)
- **Handtrap.** Si leggono dal motore sul deck avversario: è la funzione `hand_traps` che il Combo Lab usa già, applicata al suo deck. L'esposizione conta solo i punti in cui le sue handtrap fermano la linea. Ogni punto pesa quanto è probabile che l'avversario ne abbia almeno una in mano (copie nel deck, 5 o 6 carte pescate).
- **Board breaker.**
  - Sono le carte del deck avversario con il ruolo "board breaker", lo stesso riconoscimento della Fucina, e si giocano come i breaker del kit.
  - Ognuno pesa quanto è probabile averlo in mano.
  - La resilienza diventa "quanto resta della board contro i suoi breaker".
  - Se il deck non ne ha, si usano quelli standard.
- **Interruzioni e floodgate** restano misurati con il kit generico (livello 2 per cambiarli).
- **Interfaccia** (per Gemini):
  - nel Circolo Arcano, un selettore "Contro: generico / deck salvato";
  - nella scheda del punteggio, resilienza ed esposizione "contro X" accanto a quelle generiche.

### Livello 2 (dopo)
- Le azioni del kit vengono dal deck avversario. Si prendono le sue prime giocate tipiche dalle linee migliori che la ricerca trova per lui, contate come fa `duelforge-cli kit-mix`.
- Le carte del kit diventano le sue.
- Serve far giocare al motore le linee dell'avversario nel turno 2, con la board in campo e risposte a metà linea: è il lavoro più grosso.

## Decisioni per il proprietario
1. **Livello:** consiglio il 1 adesso e il 2 più avanti.
2. **Scelta del deck avversario:** tra i deck salvati in Bottega (consigliato, già importabili da `.ydk`) o da una lista di Master Duel Meta scaricata al momento.
3. **Punteggio:** il punteggio "contro X" si affianca a quello generico (consigliato) o lo sostituisce mentre l'avversario è scelto.

## Livello 1: com'è fatto (21/10/2026)
Il codice è in `crates/ygo-solver/src/matchup.rs`. Si prova con `duelforge-cli probe-line --vs <deck.ydk>`; nell'app con `api.comboScore(vsDeckId)`, che riempie `ComboScore.matchup`.

### Il deck avversario (`Rival`), letto una volta
- **Breaker.**
  - Sono le carte del Main Deck che tolgono o negano carte dell'avversario (lo stesso testo dei ruoli della Fucina) e che il motore lascia giocare dalla mano.
  - Il motore le prova contro un campo di tre mostri e due carte coperte: si attivano dalla mano (Dark Hole) o si Evocano con la loro procedura (i Kaiju, Lava Golem). Dopo l'Evocazione si usa anche il loro effetto sul campo, se ce l'hanno.
  - Le carte che non si giocano così restano fuori: le Trappole come Evenly Matched, i mostri dell'Extra Deck.
  - Le carte del kit standard (Lightning Storm, Forbidden Droplet...) si giocano come nel kit.
  - Se il deck non ne ha, si usano i breaker standard e la resilienza è quella generica.
- **Handtrap.** Sono quelle che il motore gli lascia usare dalla mano nel turno dell'avversario (`hand_traps`, la stessa funzione del Combo Lab).

### La resilienza contro X
- Ogni breaker si gioca contro la board in una partita a parte, come quelli del kit.
- L'avversario usa il breaker più dannoso che ha in mano. La resilienza è il valore che resta in media: per ogni breaker, la probabilità che sia il più dannoso in mano (lui sì, quelli più dannosi no) per quello che lascia, più la probabilità di non averne nessuno per tutto il campo.
- Le probabilità sono esatte (ipergeometrica) su 6 carte: le 5 iniziali più la pescata del suo turno.

### Le handtrap contro la linea
- La linea si rigioca con le handtrap dell'avversario in mano. Il motore registra ogni momento in cui ne può usare una e quale tua carta si sta giocando.
- Per ogni handtrap e ogni momento la linea si rigioca usandola lì. Una handtrap **ferma la linea** in quel momento se il campo o la mano a fine linea cambiano, o se la linea non si può più giocare com'era.
  - Maxx "C" e le Mulcharmy fanno pescare l'avversario ma la board resta uguale: fermano 0 momenti.
  - Ash Blossom su una ricerca o Bystial che bandisce un materiale dal Cimitero la cambiano.
- Le risposte della linea restano quelle del giocatore: le domande nuove che arrivano dopo una handtrap (rispondere alla handtrap, una finestra vuota dopo la sua catena, una scelta che la linea non aveva) ricevono una risposta semplice e la linea continua.
- La probabilità di essere fermati è quella di avere almeno una delle handtrap che fermano qualcosa tra le sue 5 carte.

### Numeri su alcune board
Linea Invoked (mano `inv-r2` del banco, prima board) e linea Branded (mano casuale del banco), deck di Master Duel Meta. Il tempo è quello in più per board, oltre al turno di prova generico (circa 1 secondo).

| Board | Contro | Breaker che sa giocare | Resilienza generica → contro X | Handtrap che fermano la linea | Tempo |
|---|---|---|---|---|---|
| Invoked | Branded | Forbidden Droplet (19%) | 0,22 → 0,54 | 48%: Bystial Saronir (9 momenti su 17), Ash (2 su 2), Droll (1 su 2); Maxx "C", Fuwalos e Purulia 0 su 21 | 1,1 s |
| Invoked | Labrynth | nessuno (standard) | 0,22 → 0,22 | 85%: Dominus Spark (16 su 16), Dominus Impulse (4 su 4), Impermanence (2 su 19), Ash (2 su 2); Arias 0 su 18 | 0,9 s |
| Invoked | Tearlaments | nessuno (standard) | 0,22 → 0,22 | 74%: Ghost Ogre (3 su 3), Impermanence, Ash, Droll; Kashtira 0 su 18, Havnis 0 su 3 | 0,8 s |
| Branded | Invoked | nessuno (standard) | 0,30 → 0,30 | 85% | 0,6 s |
| Branded | Labrynth | nessuno (standard) | 0,30 → 0,30 | 90% | 0,7 s |

### Limiti del livello 1
- I breaker dell'Extra Deck e quelli che arrivano con una combo (Apollousa, Baronne de Fleur) non si giocano: per i deck che rompono le board così la resilienza contro X è ottimista. Serve il livello 2.
- Una handtrap si prova una volta per momento, con la risposta più semplice dell'avversario (i suoi bersagli sono le tue carte, la prima della lista). Non si cerca la sua scelta migliore né la tua risposta migliore (una linea alternativa dopo l'interruzione): è il "punteggio sotto interruzione" del PIANO §9 livello 2.
- Un caso su 16 momenti (Maxx "C" su una linea Branded) risulta una fermata per un effetto "quando" che perde il tempismo: da controllare se diventa frequente.

## Livello 2: piano (21/10/2026)
Obiettivo: nel turno di prova contro un deck X, l'avversario gioca **le sue mosse tipiche** e non le 11 azioni del kit generico. Le interruzioni si misurano contro quello che X fa davvero nel suo turno.

### Come
1. **Le linee di X.**
   - Si cercano le linee migliori di X da 8-12 mani campionate del suo deck, con la ricerca del Combo Lab (circa 1000 nodi per mano).
   - Una volta per deck, poi in cache: costa circa 10-20 secondi, come la mappa dei collegamenti.
2. **I passi.**
   - Ogni linea diventa una sequenza di passi come quelli del kit: carta e azione (attivare dalla mano o dal campo, Evocare Normalmente o Specialmente), le carte scelte e gli obiettivi osservati nella linea (carte aggiunte, mostri atterrati, carte mandate al Cimitero).
   - Non si rigiocano le risposte grezze della linea: con la tua board sul campo le scelte cambiano. Si rigiocano i passi, come oggi fa il kit, quindi le stesse funzioni del turno di prova.
   - Ogni passo dipende dal precedente (`needs`): fermare il primo passo non conta anche come fermare quelli dopo (D2).
3. **I pesi.**
   - Ogni passo pesa quanto spesso compare nelle linee di X, al posto dei pesi fissi del kit.
   - Le prime mosse di una linea pesano di più, perché fermarle ferma tutto il resto: è il punto dove un'interruzione vale di più contro X.
4. **La misura.**
   - Per ogni tua risposta: quali passi di X ferma e la parte della sua linea media che resta.
   - Per la board: le "interruzioni contro X", accanto a quelle generiche e alla resilienza contro X del livello 1.
   - Con lo stesso giro si vedono anche i floodgate contro X: i passi di X che con la tua board non si possono fare.

### Cosa serve nel codice
- **In `probe.rs`:**
  - costruire un `Kit` con passi dati da fuori;
  - pesare le probe per passo invece che per tipo.
  - È un file caldo: si fa quando la sessione sulla velocità lo libera.
- **In `matchup.rs`:**
  - leggere le linee di X e trasformarle in passi, riusando il conteggio delle mosse di `duelforge-cli kit-mix`;
  - la misura contro X.
- **Interfaccia:** le interruzioni "contro X" nella scheda del punteggio.

### Rischi
- **Passi non riproducibili.** Un passo di X può non essere ripetibile nel turno 2: ad esempio una carta già usata, o una condizione "se non controlli carte". In quel caso il passo è "non disponibile" e non conta. Se succede spesso, si giocano le prime 3-4 mosse e non la linea intera.
- **Tempo.** Più passi del kit generico significano più partite per board. Il guadagno della sessione sulla velocità (partite create più in fretta) arriva al momento giusto.

### Stima
1-1,5 giorni di backend, più l'interfaccia (Gemini).

### Livello 2, primo passo fatto: le mosse tipiche di un deck (21/10/2026)
- `rival_plays` in `matchup.rs`, comando `duelforge-cli rival-plays --deck <ydk> [--hands 12] [--nodes 1500]`.
- Cerca le linee migliori del deck da mani casuali e ne legge le mosse: carta, come la gioca (attivata, Evocata Normalmente o Specialmente) e da dove. Per ogni mossa dà quante linee la giocano, quante la giocano per prima e in che posizione arriva in media.
- **Branded** (12 mani, 36 s): Ecclesia e Cartesia attivate sul campo nel 50% delle linee; Branded Opening, Branded in High Spirits, Branded Fusion e The Fallen & The Virtuous nel 33%; per prime l'Evocazione Normale di Ecclesia (25%) o di Aluber (17%).
- **Labrynth** (12 mani, 13 s): Evocazione Normale di Arianna per prima nel 33% delle linee, poi Lady Labrynth dalla mano; Arias dalla mano per primo nel 17%.
- Il passo che manca è trasformare queste mosse nei passi del turno di prova, in `probe.rs`, quando la sessione sulla velocità lo libera.

### Correzioni dal collaudo (21/10/2026)
Il collaudo su molti deck (`docs/plans/collaudo-strumenti.md`, branch `claude/tools-qa`) ha trovato Maxx "C" e le Mulcharmy che "fermavano" la linea in 6 deck su 12. Dopo una handtrap, la linea ora si rigioca così:
- **Azioni per identità.** Le azioni della linea si ritrovano per quello che sono (carta, zona, effetto, tipo di azione) e non per la loro posizione nella lista. Dopo Maxx "C" la lista può cambiare: K9-17 Izuna diventa Evocabile. Un trigger che la linea aveva come sì/no può anche arrivare in una finestra di trigger (Fur Hire).
- **Domande nuove rifiutate.** Una domanda che la linea non aveva e che si può rifiutare riceve la risposta passiva, e la linea va avanti: una finestra di catena, un sì/no, una Main Phase dove resta solo da finire il turno.
- **Fermata solo se manca qualcosa.** La linea è fermata solo se a fine linea manca qualcosa del campo o della mano. Finire con una carta in più, come K9 che punisce Maxx "C", non è una fermata.
- **Risultato.** Su 5 deck contro Branded e Labrynth, Maxx "C" e le Mulcharmy sono a 0 su Tearlaments e a 1-3 momenti su circa 20 negli altri. Il resto dipendeva dal deck da 12 carte dell'avversario del kit: Maxx "C" lo svuotava e le carte che guardano il deck dell'avversario cambiavano.
- **Il Deck vero dell'avversario nel rigioco.** Nel rigioco delle handtrap l'avversario ora pesca dal suo Main Deck vero, non dalle 12 carte del kit. Per la ricerca e la mappa, che usano ancora il kit, resta da fare in `Kit::standard` con il banco, quando `probe.rs` si libera.

### Maxx "C" e Mulcharmy, passo 1: le carte fatte pescare (21/10/2026)
Il proprietario ha chiesto di trovare il compromesso tra le carte che l'avversario pesca con Maxx "C" o le Mulcharmy e le interazioni che la board tiene nel suo turno. Il passo 1 misura le pescate; il passo 2 cercherà il punto migliore dove fermare la linea.
- **Cosa si misura.**
  - Per ogni handtrap e ogni momento in cui l'avversario la può usare, si rigioca la linea usandola lì e si contano le carte che pesca fino alla fine del tuo turno.
  - Si tolgono le carte che rimescola dalla mano nel Deck: è il limite di mano delle Mulcharmy nella End Phase.
  - Vale per qualunque carta che fa pescare, senza nomi.
- **Dove si legge.**
  - `RivalHandTrap.drawn`: il massimo di carte pescate.
  - `RivalHandTrap.moments`: per ogni momento la tua carta in gioco, se ferma la linea e quante carte pesca.
  - Nell'app `ComboRivalHandTrap.drawn` e `moments`; nella CLI `probe-line --vs` scrive "draws up to N" e le carte pescate momento per momento.
- **Scelte rigiocate per identità.** Dopo una handtrap, le scelte di carte della linea si rigiocano per quello che sono (codice, zona, giocatore) e non per la posizione nell'elenco.
  - Il caso trovato: con Fuwalos l'elenco dell'Extra Deck cambiava ordine, e Invocation Evocava Invoked Caliga invece di Baybarron.
  - Fuwalos sulla linea Invoked passa da 23 momenti su 25 "fermati" a 1.
- **Numeri** (prima board di una mano per deck, contro Branded di Master Duel Meta: Maxx "C", Fuwalos, Purulia). Il numero è il massimo di carte che l'avversario pesca, cioè usandola al primo momento.

| Linea | Maxx "C" | Fuwalos | Purulia | Quando smette di pagare |
|---|---|---|---|---|
| Invoked (Aleister, Chalice, Meltdown) | 9 | 7 | 2 | Maxx "C" a metà linea vale ancora 5 carte (S:P Little Knight), dopo Virakam 1 |
| Tearlaments (Reinoheart, Havnis, Engraver) | 12 | 6 | 1 | Maxx "C" vale ancora 8 carte a Lacrima, 5 a Requiem |
| Vanquish Soul K9 (Mad Love, Razen, Izuna) | 9 | 5 | 5 | Maxx "C" vale ancora 6 carte a Heavy Borger, 3 a Mad Love |
| Branded (Aluber, Branded Fusion, Ecclesia) | 5 | 3 | 1 | da Ecclesia in poi 0 |

- **Cosa dice.** Le linee lunghe (Tearlaments, Invoked) regalano molte carte se Maxx "C" arriva presto. Il passo 2 deve dire fino a dove conviene continuare: il valore della board a ogni punto di arresto contro le carte regalate.
- **Due regressioni corrette** (trovate dal collaudo dopo la PR #23):
  - **R1.** Il segnale "l'ultimo anello della catena è dell'avversario" restava acceso per tutta la risoluzione, quindi il sì/no di un tuo effetto che si risolveva sotto Maxx "C" veniva rifiutato. Ora vale solo per le finestre di catena.
  - **R2.** I passaggi della linea che il motore non chiede più si saltano prima di decidere se rifiutare una domanda.
  - Sulle quattro linee del passo 1, Maxx "C", Fuwalos e Purulia passano da un momento "fermato" per linea a 0, tranne K9 (1 su 21, a Razen). Le fermate vere di Ash, Saronir e Droll restano.
- **Terzo giro del collaudo** (180 board, `docs/plans/collaudo-strumenti.md`): con le correzioni fin qui le fermate false di Maxx "C" e Mulcharmy passano da 1116 a 81. Le ultime due cause sono corrette:
  - **R3.** Quando l'effetto atteso di una carta non è offerto, la linea non ripiega più su un altro effetto della stessa carta. Lo fa solo per un sì a una domanda, la cui descrizione non è quella dell'effetto. Prima K9-17 "Ripper" negava Maxx "C" invece di cercare.
  - **R4.** Se la linea passa in una finestra dove la sua azione successiva è già offerta, gioca l'azione e salta i passaggi. La catena della handtrap aveva preso una finestra della Fase Finale e Blazing Cartesia restava fuori.
  - Nella copia del collaudo, con R3 e R4 le fermate false vanno a 0 e quelle vere restano uguali.

### Maxx "C" e Mulcharmy, passo 2: dove fermare la linea (21/10/2026)
- **La domanda.** L'avversario usa Maxx "C" (o una Mulcharmy) al primo momento utile. Conviene finire la linea, o fermarsi prima e regalargli meno carte?
- **Come si misura** (`draw_trade_off` in `matchup.rs`).
  - I punti di arresto sono i momenti liberi della tua Main Phase lungo la linea, al massimo 12 distribuiti, più la linea intera. A ogni punto la linea finisce il turno lì.
  - La board di ogni punto si misura con il turno di prova standard: interruzioni e floodgate.
  - Le carte pescate dall'avversario si contano rigiocando la linea fermata lì, con la handtrap usata al primo momento. Contano anche la End Phase e il limite di mano delle Mulcharmy.
- **Il valore atteso nel suo turno.**
  - L'avversario ha 6 carte più quelle pescate, quindi più probabilità di avere un breaker. Gioca il più dannoso che ha, con la stessa formula esatta della resilienza contro X.
  - I floodgate contano solo se non ha breaker: un Dark Hole toglie anche quelli.
  - Se il deck non ha breaker che il turno di prova sa giocare, si usano quelli standard e si suppone che il deck ne giochi circa 5 su 40.
  - Il punto migliore è quello col valore atteso più alto. A parità vince quello che regala meno carte, poi il primo: stesso risultato con meno azioni (proposta del collaudo).
- **Dove si usa.** Nella CLI con `probe-line --vs <deck> --stops`. Nell'app con `api.comboStops(hand, key, vsDeckId)`: pochi secondi per board, si chiama a richiesta su una board.
- **Numeri** (le stesse quattro linee del passo 1, contro Branded, con Maxx "C" al primo momento):

| Linea | Linea intera | Punto migliore | Cosa dice |
|---|---|---|---|
| Invoked | 0,78 con 9 carte pescate | dopo Virakam: 0,80 con 8 | gli ultimi passi (Invocation) non aggiungono niente e regalano una carta |
| Tearlaments | 1,27 con 12 | la linea intera | The Undying Legion arriva alla fine e vale da sola più di tutto il resto (da 0,28 a 1,27) |
| Vanquish Soul K9 | 0,09 con 9 | dopo Snow Devil: 0,10 con 7 | gli ultimi tre passi (Rock of the Vanquisher) regalano 2 carte senza aggiungere interruzioni |
| Branded | 0,49 con 5 | la linea intera | Albion the Branded Dragon arriva alla fine |

- **Limiti.**
  - Le carte pescate contano solo per la probabilità di un breaker nel suo turno. Non contano le risorse per la sua combo, né le handtrap per il tuo turno dopo: il costo vero di una carta pescata è più alto. Un peso in più per le carte regalate si può aggiungere come preferenza del giocatore.
  - Si prova solo l'uso al primo momento, che è il più comune; l'avversario potrebbe aspettare.
  - La linea fermata a metà finisce il turno così com'è: non cerca una chiusura diversa con le carte rimaste.

### Seconde correzioni dal collaudo: i breaker (21/10/2026)
- **Breaker falsi.** Una carta presa dal testo resta un breaker solo se, giocata contro il campo finto, gli toglie una carta o ne nega una.
  - Il campo finto ha tre mostri (un mostro con effetto, un Fusion e un Link) e due carte coperte.
  - L'avversario gioca la carta come farebbe davvero: mira alle tue carte, risponde sì, attiva gli effetti della carta, e dopo un'Evocazione usa una volta il suo effetto sul campo.
  - Un Kaiju che arriva sul tuo campo al posto del mostro che tributa conta: si contano solo le carte del campo finto.
  - Esempi: Clockwork Night toglie solo ATK e non conta più. Contro Cyber Dragon si usano quindi i breaker standard (resilienza 0,38 come la generica; prima era il campo intero).
- **Il Deck vero dell'avversario.** Nella prova l'avversario ha il suo Main Deck e il suo Extra Deck, perché alcune carte pagano un costo da lì (Ultimate Slayer manda un mostro dall'Extra Deck) o prendono carte dal Deck (Interrupted Kaiju Slumber vuole due Kaiju).
  - In cima al suo Deck c'è una carta neutra: la pescata del suo turno non gli toglie una carta che serve.
- **Testo dei breaker.** Ora si riconoscono anche "as many ... as possible" (Slumber), "shuffle" (Ultimate Slayer) e "your opponent currently controls" (Dark Ruler No More).
- **Prova** (linea Branded, prima board, contro un deck Invoked con 6 breaker aggiunti): riconosciuti Dark Hole, Dark Ruler No More, Gameciel, Dogoran, Ultimate Slayer e Interrupted Kaiju Slumber; scartata Clockwork Night.
  - Ultimate Slayer e Slumber risultano "non applicabili" nella partita del breaker, perché lì l'avversario ha ancora il deck del kit, senza Extra Deck e senza Kaiju.
  - Si sistema con `Kit::with_breakers` (`probe.rs`), insieme al deck del kit da 35 carte.

### Il Deck dell'avversario: 35 carte, e quello vero nei breaker (21/10/2026)
Le ultime due correzioni del collaudo (A4 e M1), in `Kit::standard` e `Kit::with_breakers` (`probe.rs`).
- **35 carte.** Il Deck dell'avversario del kit aveva 12 carte, e gli effetti che lo guardano vedevano molto meno che in partita.
  - Ora ne ha 35, quante ne ha l'avversario durante il tuo primo turno (40 meno le 5 in mano). Le carte semplici in più stanno in fondo, così le carte che il kit cerca restano.
  - La ricerca gioca contro lo stesso avversario, quindi le linee trovate lo vedono già.
  - Esempio del collaudo (Branded, That Grass Looks Greener in mano): mandava al Cimitero 38 carte, ora 14, quante in una partita vera.
- **Il Deck vero dell'avversario nei breaker.** Contro un deck scelto, i suoi breaker si giocavano con il Deck e l'Extra Deck del kit: Ultimate Slayer (costo dall'Extra Deck) e Interrupted Kaiju Slumber (Kaiju dal Deck) risultavano «non applicabili».
  - Ora carte a caso del suo Main Deck, mescolate con il seme della partita, prendono il posto di quelle semplici in fondo. Il Deck resta da 35 carte, come quello contro cui sono state trovate le linee, e il suo Extra Deck si aggiunge a quello del kit.
  - `Rival::kit` mostra il kit risultante.
- **Banco** (protocollo di `AGENTS.md`, sulla `main` di stasera):
  - 16 mani a 4000 nodi: board, nodi, replay e punteggi identici, cambiano solo i tempi (rumore);
  - `combo_hunt.py` su 37 deck, 2 mani, seme 7: 74 righe uguali, salvo una mano zoodiac con 584 board invece di 585 e la stessa board migliore. Nessun problema.
  - Le mani del banco non usano effetti che guardano il Deck dell'avversario: il cambio si vede su quelle che li usano, come That Grass Looks Greener.
- **Test:** in `tests/matchup.rs` il Deck del kit ha 35 carte, quello contro un deck ha le sue carte e il suo Extra Deck, e Ultimate Slayer contro la board di Invoked Mechaba è giocabile.

### Livello 2, seconda parte: le mosse tipiche nel turno di prova (22/10/2026, notte)
Prima versione, solo aggiuntiva: la misura generica (interruzioni, floodgate, breaker, resilienza) resta identica. Per ora c'è solo nella CLI: `probe-line --vs <deck> --moves`.
- **Le mosse.** `rival_plays` legge le mosse delle linee migliori del deck avversario (12 mani, 1000 nodi; da 4 a 13 secondi, una volta per deck). `typical_moves` tiene:
  - le mosse di almeno un sesto delle linee, nell'ordine in cui arrivano di solito, al massimo 10;
  - una sola Evocazione Normale, la più giocata: le altre sono aperture di altre linee.
- **Il turno di prova.** `Kit::with_moves` fa giocare all'avversario quelle mosse al posto delle 11 del kit:
  - le carte giocate dalla mano sono in mano; le altre vengono dalle mosse prima;
  - Deck vero (35 carte) ed Extra Deck vero;
  - una mossa che non si può fare si salta.
  - Una mossa riesce quando non è negata (un'Evocazione, quando il mostro atterra). Le sue risposte automatiche contano con lei: Ash che nega la ricerca di Arianna ferma la mossa «Evocazione Normale di Arianna».
- **Il peso di una mossa:** quota delle linee che la giocano × (1 + quota in cui è la prima).
- **La misura** (`Prober::against_moves`, `matchup::against_moves`):
  - **ogni risposta** vale la parte del peso delle mosse che toglie nel suo momento migliore, o usata ogni volta che è offerta. Contano le mosse che fa fallire e quelle perse con loro (un mostro evocato, poi il suo effetto sul campo). Come nella misura generica: valore dell'effetto × 0,85 se bersaglia × quota tolta;
  - **«mosse fermate»:** si mostrano solo quelle che la risposta ferma da sé (D2);
  - **floodgate:** la parte del peso che la tua board toglie senza rispondere;
  - **«resta»:** la parte delle mosse ancora fatte con tutte le risposte usate insieme, la più forte prima. Due risposte che fermano la stessa mossa contano una volta.
- **Prova** (board Invoked Mechaba, Baybarron, Virakam, Caliga, Meltdown con Ash in mano):

  | Contro | Mosse | Floodgate | Mechaba | Ash | Resta con tutte |
  |---|---|---|---|---|---|
  | Labrynth | NS Arianna, Arias, Lady, Welcome Labrynth, Muckraker | 66% (Caliga blocca Arias) | 100% (nega Arianna: il resto si perde) | 100% | 0% |
  | Tearlaments | NS Reinoheart, Scream, Instant Fusion, Engraver, Scheiren… | 10% | 40% | 40% | 60% |
  | Branded | NS Aluber, Branded Opening, Albion, High Spirits, Branded Fusion… | 22% | 19% | 24% | 81% |

- **Limiti di questa prima versione:**
  - le mosse sono una sola sequenza: quelle che vengono da linee diverse si perdono (Branded: l'effetto di Ecclesia sul campo non si fa, perché la sua Evocazione è un'altra apertura);
  - un'attivazione conta come fermata solo se negata: distruggere il mostro in catena, se l'effetto si risolve lo stesso, non la ferma, e Droll & Lock Bird non è visto;
  - due risposte che fermano la stessa mossa contano tutte e due nella somma dei valori (il numero «resta» ne tiene conto).
- **Da decidere con il proprietario prima dell'interfaccia:**
  - se mostrarla accanto alla misura contro il deck: costa 4-13 secondi in più alla prima scelta del deck, per leggere le sue linee;
  - i pesi;
  - se giocare più sequenze, una per apertura, per non perdere le mosse delle altre linee.


### Livello 2: collaudo su 37 deck e prova con le linee vere (22/10/2026, notte)
Prova: 6 board contro i 37 deck di Master Duel Meta, con `probe-line --vs X --moves` (222 prove). Le board sono la prima della caccia `combo_hunt.py --seed 7` di Invoked, Branded, Tearlaments, Labrynth, K9 e Dracotail.
- **Errori: nessuno**, dopo una correzione di `probe-line`.
  - Prima rigiocava le linee salvate con il seme di `--seed` (1), non con quello salvato con le linee (7). Con un altro seme cambia l'ordine del Deck, e una linea che manda carte dal Deck al Cimitero si rompe: le board Branded e Tearlaments fallivano contro tutti i deck.
  - Ora usa il seme salvato nel file.
- **Tempi:** le mosse tipiche 31 secondi di mediana e 109 al massimo (Yummy, Elfnote); la misura contro le mosse 0,9 secondi di mediana e 4 al massimo. Erano 4 prove insieme su 4 core: da sola la lettura delle mosse prende 4-13 secondi.
- **Le mosse tipiche cambiano molto con il seme.** Con 12 mani, lo stesso deck con il seme 1 e con il seme 7 ha in comune in media il 43% delle mosse (37 deck): dal 18% di Ignister al 67% di Azamina, D/D/D, Exosister e Maliss.
  - Con 24 mani si sale solo al 50% (dal 24% di Warrior Combo all'83% di Dogmatika), e la lettura costa il doppio.
  - Elfnote Kewl Tune con 24 mani ha impiegato 463 secondi: vedi `docs/plans/velocita-ricerca.md`.
  - Più mani non bastano: per mosse stabili servono le linee vere (sotto).
- **Il 38% delle mosse tipiche non si fa nemmeno contro un campo vuoto** (816 su 2166). Il kit non le trova offerte; nessuna è offerta e poi fallisce. Seguendo una linea di Tearlaments, le cause sono quattro:
  1. **Effetti in catena.** Una mossa che nella linea vera si attiva in catena (un effetto nel Cimitero, un mostro evocato durante la catena) non può cominciare: il kit comincia un passo solo nella sua Main Phase.
  2. **Effetti della stessa carta.** Due effetti della stessa carta nello stesso posto sono la stessa mossa. Il kit ha usato l'effetto di Kitkallos all'Evocazione al posto di quello sul campo, e le mosse dopo si sono perse.
  3. **Carte scelte.** Le carte cercate o mandate al Cimitero sono le prime offerte, non quelle scelte dalla linea.
  4. **Mano e Deck.** Il kit ha in mano solo le carte giocate dalla mano: un costo come "scarta 1 carta" non si paga. Anche l'ordine del Deck non è quello della linea.
- **Floodgate:** sono veri. Ad esempio Radiant Typhoon Swen ed Eldam si Evocano dalla mano solo se l'avversario non controlla Magie o Trappole. Ogni board con una Magia sul campo le ferma (34-41% del peso contro Goblin Biker e Radiant Typhoon).
- **Prova (non committata, patch nello scratchpad della sessione).** Al posto della sequenza tipica, le 12 linee migliori del deck avversario, ognuna con le sue prime 10 mosse. Board Branded contro 10 deck (Tearlaments, Labrynth, Branded, Yummy, K9, Dracotail, Blue-Eyes, Maliss, Ignister, Memento). Mosse fatte contro un campo vuoto:

  | Kit | Mosse fatte |
  |---|---|
  | sequenza tipica (oggi) | 53% |
  | linee vere | 62% |
  | + effetti in catena (cause 1) | 63% |
  | + effetto giusto della carta (2) | 63% |
  | + carte scelte dalla linea (3) | 70% |

  - Gli effetti in catena aiutano K9 (39 → 53%) e Tearlaments (43 → 48%), ma peggiorano Labrynth (100 → 90%): a volte prendono in anticipo l'effetto di una mossa che viene dopo.
  - Le carte scelte aiutano molto Dracotail (64 → 92%) e Blue-Eyes (53 → 63%).
  - Tempo: da 0,1-0,5 a 0,8-4 secondi per board.
- **Proposta per il proprietario.** Migliorare il kit porta al 70% delle mosse. Per andare oltre serve la causa 4, cioè che il turno dell'avversario rigiochi le sue linee vere come si rigioca la tua linea contro le handtrap:
  - la sua mano vera e l'ordine del Deck della sua ricerca;
  - le risposte ritrovate per identità (`find_again`) dove la tua board cambia le domande.

  Insieme alle linee vere, rende stabili anche le mosse (punto sopra). Lavoro stimato: circa un giorno. Da decidere insieme agli altri punti aperti del livello 2.

### Livello 2 con le linee vere dell'avversario (22/10/2026)
Fatto il piano P1 di `docs/plans/piano-prossimi-passi.md`: al posto della sequenza di mosse tipiche giocata dal kit, l'avversario rigioca le sue linee migliori.
- **Le linee** (`rival_lines`, `rival_line`): 12 mani a caso del deck avversario, cercate a 1000 nodi contro l'avversario del kit come fa il Combo Lab. Per ogni mano restano la mano e le risposte della linea migliore, registrate per quello che fanno (`own_answers`), e le sue mosse.
- **Il suo turno** (`Kit::with_line`): l'avversario ha la mano della linea e il suo Deck nello stesso ordine della ricerca (`ygo_engine::shuffle`, ora pubblica), più la carta del kit in cima per la pescata del turno 2.
  - Un `Follower` (`follow.rs`) rigioca le risposte e ritrova le azioni per identità (carta, posto, effetto) e le carte scelte per quello che sono.
  - Declina le domande che la linea non aveva, e non risponde mai ai tuoi effetti.
  - Quando un'azione non c'è più (la tua board ha fermato ciò che le serviva), la linea riparte dalla prima azione successiva offerta.
  - Ogni mossa della linea è un passo del turno di prova: `Prober::against_moves` resta com'era.
- **La misura** (`against_moves`): linea per linea; ogni linea pesa uguale, e ogni mossa di una linea pure. Una linea senza mosse non si misura, perché la sua mano ha solo Settato carte e il pericolo è nel tuo turno: è il caso di molte mani Labrynth.
- **Collaudo veloce:** board Branded e Tearlaments (la prima della caccia `--seed 7`) contro i 37 deck di Master Duel Meta, 74 prove:

  | | Prima (sequenza tipica) | Ora (linee vere) |
  |---|---|---|
  | Errori | 0 | 0 |
  | Mosse fatte su un campo vuoto | 62% | 90,7% (Branded), 90,9% (Tearlaments); minimo 74% (Branded contro Branded) |
  | Misura di una board | 0,1-0,5 s | 4 s di mediana, 9 al massimo (2 prove insieme) |
  | Lettura delle linee di un deck | 4-13 s | 10 s di mediana, 25 al massimo |

- **Nell'app:** `api.comboMoves(hand, key, vsDeckId)`, a richiesta per la board aperta. Le linee di un deck si leggono una volta e restano in memoria finché il deck e il seme sono gli stessi. L'interfaccia è chiesta a Gemini in `docs/TASKS.md`.
- **Resta da capire:**
  - il 9-10% di mosse che il turno di prova non rigioca;
  - la stabilità dei risultati tra semi diversi.

  Lo misura il collaudo completo della corsia 3 (L4, 6 board).
- **Banco** (16 mani, 4000 nodi): identico, perché la misura generica e la ricerca non cambiano.
