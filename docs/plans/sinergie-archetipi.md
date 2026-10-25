# Il Crogiolo: sinergie tra archetipi (piano, 25/10/2026)

Richiesta di Valerio del 25/10: una sezione che esplori come mescolare un archetipo con uno o più altri per fare un deck nuovo con linee di combo nuove, leggendo gli effetti delle carte e le loro caratteristiche (archetipo, Attributo, Tipo, Livello). Valerio la considera una sezione importante. Questo documento è solo progetto: nessun codice da unire. **Approvato da Valerio il 25/10, con le quattro decisioni consigliate del §11**; si parte dalla fase F0.

Mockup (Claude Design, 1280×800, temi Fucina e Pergamena): https://claude.ai/artifact/WDzepFuEQTNKsXL5CDMhoJ. Carte e numeri del mockup sono esempi per il disegno, non risultati.

## In breve
- **Che cosa fa.** Parte da un deck o da un archetipo (A) e risponde a tre domande: con che cosa sta bene A (i partner), come si collega a un partner B (i ponti), che cosa fa il mix in più di A da solo (la prova col motore, con le linee nuove che la ricerca trova).
- **Due tempi, come nel resto dell'app.** Una stima veloce, dai dati delle carte e dagli effetti letti dal motore, ordina i partner e propone un pacchetto di carte. Poi il motore gioca davvero le mani del deck misto e di quello di partenza e giudica. La stima sceglie che cosa provare; solo il motore dice se funziona.
- **I ponti sono dati, mai nomi.** Una carta di A che cerca o Evoca per Attributo, Tipo o Livello e accetta carte di B; un mostro dell'Extra Deck con materiali che A e B soddisfano insieme; Small World; un evento che A produce e B ascolta (mandare al Cimitero, bandire, scartare); un motore che gioca da solo. Ognuno si legge dagli script, dalle carte offerte dal motore o dai dati del database.
- **Il dato nuovo più importante** è «che cosa accetta ogni effetto che sceglie carte», letto dal motore con un deck di lettura che contiene tutto il pool: il motore stesso dice quali carte un effetto può prendere, con le condizioni già applicate, come fa oggi la mappa dei Collegamenti dentro un deck.
- **Linee nuove**: nessuna linea precalcolata. La ricerca del Combo Lab gioca le mani del deck misto; una linea è «nuova» se usa effetti dei due archetipi e passa da almeno un ponte. Si raggruppano per i ponti che usano e si aprono nel riproduttore del Circolo Arcano.
- **Sinergia misurata**: per le mani con carte dei due archetipi, il valore della board meno quello che ciascun archetipo fa da solo con la stessa mano (le carte dell'altro diventano carte che non fanno niente, come nei ruoli). È l'indice d'interazione della teoria dei giochi cooperativi.
- **Costo.** Otto PR in sei fasi, circa 170-215 $ in tutto (sotto 30 $ per PR). La CLI è utile già dalla fase 2; l'interfaccia arriva dalla fase 4.

## 1. Che cosa fa e per chi

### 1.1 Per chi
- **Chi inventa deck** (il «brewer»): ama un archetipo e cerca con che cosa unirlo per un deck che non c'è nelle liste del meta.
- **Chi ha già un deck** e cerca un motore da aggiungere: «che cosa metto in Invoked per avere un secondo Starter?».
- **Chi studia il meta**: vuole capire perché un mix che vince funziona, quali carte fanno da ponte e quanto vale ciascun archetipo da solo.

### 1.2 Tre viste
1. **Partner.** Da A, la lista degli archetipi che si uniscono meglio, con i ponti contati per tipo, i vincoli che li ostacolano (blocchi di Evocazione), la stima, se il motore li ha già provati e con quale esito, e se il mix è già nelle liste del meta o è nuovo.
2. **Ponti e prova.** Scelto B: la mappa dei ponti tra le carte di A e di B, il pacchetto proposto (carte che entrano e che escono), e il bottone «Prova col motore»: P(board ≥ soglia) di A da solo e del mix sulle stesse mani, la differenza con l'intervallo, la stessa quota per numero di carte di B in mano, la sinergia, i costi (mattoni, mani bloccate) e le linee nuove.
3. **Analizza un deck.** Il contrario: un deck della Bottega o del meta si scompone nei suoi archetipi e si vede quali ponti usa davvero, quanto vale ogni pezzo da solo e quanto insieme.

### 1.3 Che cosa non fa
- Non tiene un archivio di combo o di mix «buoni»: ogni risposta si ricalcola dai dati e dal motore.
- Non giudica senza il motore: la stima ordina, non promuove. Un partner «da provare» resta tale finché il motore non l'ha giocato.
- Non decide per nome: nessuna carta, archetipo o mix ha un trattamento speciale nel codice. Un archetipo nuovo funziona appena gli script e il database lo conoscono.
- Non cambia la misura del Combo Lab: la usa com'è (profilo «Bilanciato», turno di prova, `engine-mc`).

### 1.4 Dove sta nell'app
Una sezione nuova nella barra, **«Sinergie»** con titolo **«Il Crogiolo»** (il recipiente dove si fondono i metalli, in tema con Fucina e Bottega), con un suo accento (blu, `--sec` `#6fa8ff` nel tema scuro e `#2f63b8` nel chiaro, diverso dal ciano del Laboratorio e dal viola del Circolo Arcano). Si collega a:
- **Bottega**: il deck di partenza; «Crea il deck nella Bottega» salva il mix come deck nuovo, senza toccare l'originale.
- **Laboratorio**: «Collegamenti» del mix con i collegamenti nuovi accesi (`compare_links`); «Carte speciali» per i ponti di Small World; `engine-mc` come misura.
- **Circolo Arcano**: «Apri nel Combo Lab» apre la mano e la linea nuova nel riproduttore, e può partire con un obiettivo (il mostro ponte sul campo).
- **Fucina**: dall'ispettore di una carta, «Con chi si unisce questo archetipo».

## 2. I numeri del problema (censimento del 25/10)
Un censimento grezzo sui dati veri (BabelCDB `cards.cdb` e CardScripts di Project Ignis, scaricati il 25/10; espressioni regolari sugli script, quindi solo per dare la scala). Script: `/mnt/project-files/analisi/sinergie/census.py`, fuori dal repository.

| Che cosa | Quante |
|---|---|
| Carte ufficiali (OCG/TCG, senza alias e token) | 14 121 |
| Carte con almeno un archetipo (setcode) | 7 906 |
| Carte in due o più archetipi di base | 651 |
| Archetipi di base con almeno 5 / 10 / 20 carte | 406 / 326 / 140 |
| Coppie di archetipi che condividono almeno una carta | 421 |
| Script che cercano, Evocano o mandano al Cimitero | 7 368 |
| ... con un filtro per Attributo, Tipo, Livello o tipo di carta, senza nome né archetipo | 2 589 |
| ... con un filtro per archetipo o per nome | 4 568 |
| Mostri dell'Extra Deck con una procedura di Evocazione | 2 128 |
| ... con materiali generici (Attributo, Tipo, Livello, «2 mostri») | 1 618 |
| Script con un blocco di Evocazione Speciale | 905 (131 solo archetipo, 128 per Attributo o Tipo, 47 per tipo di Evocazione, il resto da leggere) |

Che cosa dicono:
- **I ponti generici sono tanti.** Più di un terzo delle carte che muovono carte ha un filtro per proprietà: sono le porte d'ingresso per un altro archetipo. Tre quarti dell'Extra Deck accetta materiali generici.
- **Lo spazio dei mix è grande.** Con i 140 archetipi da almeno 20 carte le coppie sono 9 730 e le terne 447 580; il motore costa circa 10 minuti per provare un mix con 200 mani su 4 core (`monte-carlo-motore.md`: 5-9 s a mano). Quindi serve la stima per scegliere i pochi da provare: è la struttura di tutta la sezione.
- **Le liste vere sono già mix.** Nel banco congelato ci sono Azamina Mitsurugi Dark Magician, Fiendsmith Unchained, Atlantean Mermail, White Forest Azamina, Radiant Typhoon Zoodiac, Vanquish Soul K9, Magistus Witchcrafter, Onomat Ryzeal: sono la prova che i giocatori mescolano, e una verità da riscoprire per misurare la stima (§9).
- **Gli archetipi «di famiglia»** (Number, Xyz, Chaos, Hero/Neos) dominano le carte condivise: vanno trattati con la gerarchia dei setcode (i 4 bit alti sono il sottoarchetipo) e non contati come ponti veri.

Lo stesso censimento dai dati del motore (F0, 25/10, `synergy-census`), con il pool del deck di lettura: anche le carte dei database `release-*` e `prerelease-*` e i Mostri Normali senza script, quindi un po' più grande.

| Che cosa | Quante |
|---|---|
| Carte del pool (Main Deck + Extra Deck) | 14 403 (12 186 + 2 217) |
| Con almeno un archetipo / in due o più archetipi di base | 8 105 / 681 |
| Archetipi di base con almeno 5 / 10 / 20 carte | 419 / 337 / 143 (494 in tutto) |
| Coppie di archetipi che condividono almeno una carta | 443 |
| Carte con un effetto attivato che cerca, aggiunge alla mano, Evoca Specialmente o manda al Cimitero (categorie dell'anatomia) | 7 452 |
| Mostri dell'Extra Deck con una procedura di Evocazione (materiali o altro modo) | 2 195 su 2 217 |

Delle 32 scelte lette sulle 30 carte di prova, 27 offrono carte di almeno 3 archetipi o senza archetipo: sono ponti generici, letti dal motore e non più da espressioni regolari.

## 3. I dati che servono

### 3.1 Che cosa c'è già
| Fonte | Che cosa dà | Dove |
|---|---|---|
| Database del motore (`cards.cdb`) | setcode (fino a 4 per carta, con il sottoarchetipo), tipo, Attributo, Tipo, Livello/Rango/Link, ATK/DEF, scale, frecce Link | `ygo-engine/src/carddb.rs` (`Card::setcodes`) |
| Database delle carte (YGOPRODeck) | il nome dell'archetipo (uno solo), testi, legalità e banlist Master Duel | `ygo-data` (`cards.db`, `store.rs`) |
| Mappa dei collegamenti | per un deck, chi cerca, Evoca, manda al Cimitero, recupera o fa da materiale a chi; possibile e usato; confronto tra versioni | `ygo-solver/src/links.rs` |
| Ruoli | Starter, Extender, Handtrap, Breaker, Mattone, giocando le carte | `ygo-solver/src/roles.rs` |
| Valori delle carte | quanto vale una carta sul campo nel turno avversario | `ygo-solver/src/value.rs` |
| Monte Carlo del motore | P(board ≥ soglia) con mani comuni tra due varianti, scambi, `climb` | `duelforge-cli engine-mc` |
| Obiettivi | ricerca verso un mostro o una carta in un posto | `ygo-solver/src/goal.rs` |
| Carte speciali | ponti di Small World verso un bersaglio | `ygo-stats/src/special.rs` |
| Liste | top deck di Master Duel Meta (scaricate, mai committate); 63 liste del banco congelato; i deck della Bottega | `scripts/mdm_decks.py`, `tests/fixtures/banco-motore/v1` |

### 3.2 Che cosa arriva dall'anatomia degli effetti (PR #181)
Il lettore del Brief A di `anatomia-effetti.md` dà, per ogni effetto: tipo, evento, zona, limite per turno, proprietà, categorie, descrizione; e, quando l'effetto si attiva, quantità e zona colpita (`SetOperationInfo`) e le restrizioni registrate sul giocatore. Qui servono:
- **le categorie** (cerca, Evoca, manda al Cimitero, bandisce) per sapere quali effetti muovono carte;
- **gli eventi** degli effetti innescati (quando va al Cimitero, quando è bandito, quando è Evocato) per i ponti di eventi;
- **le restrizioni** («non puoi Evocare Specialmente, eccetto…») per la compatibilità;
- **i limiti per turno**, per sapere se due carte si contendono lo stesso uso.

La sezione può cominciare senza l'anatomia (fasi 0-1 leggono i predicati da sole); i ponti di eventi e i blocchi letti bene arrivano quando il Brief A è unito.

### 3.3 Che cosa manca, e come si ottiene
**(a) Che cosa accetta ogni effetto che sceglie carte.** È il dato centrale. Oggi `links.rs` registra le carte che il motore offre quando un effetto chiede di scegliere, ma solo tra le carte del deck. Per sapere se A può prendere una carta di B bisogna chiederlo sull'intero pool.
- **Metodo: il deck di lettura.** Un duello in cui il Deck contiene tutte le carte del pool (o un campione stratificato: almeno una carta per ogni combinazione di tipo, Attributo, Tipo e Livello, più tutti i membri degli archetipi in gruppi). Si gioca una ricerca breve da una mano con la carta da leggere, come fanno `links.rs` e `roles.rs`; a ogni scelta il motore elenca le carte che l'effetto accetta, con le condizioni dello script già applicate. Quell'elenco, per effetto, è il dato.
- **Perché così.** I filtri sono funzioni Lua con chiusure e parametri di contesto: chiamarli da fuori è fragile. Il motore li valuta nel loro contesto, e la regola resta «leggiamo ciò che il motore offre», la stessa della mappa dei Collegamenti.
- **Verificato nella fase F0 (25/10, `duelforge-cli synergy-census`).** Il nucleo accetta il pool intero senza toccare `vendor/`: un duello con 12 186 carte nel Deck e 2 217 nell'Extra Deck si crea in 0,5 s (riusato sul thread) e gioca senza errori e senza toccare il tetto di Lua; memoria di picco 734 MB con 4 duelli in parallelo. Una carta si legge in 3,1 s in media (mediana 3,0 s, massimo 6,5 s per una carta con due turni), quindi tutto il Main Deck del pool in circa 2 ore e 40 minuti su 4 core: il campione stratificato non serve. Le 30 carte di prova offrono tutte qualcosa, e per 29 l'elenco letto nel pool, ristretto alle carte di un deck vero che le contiene (21 liste del banco congelato, 9 deck costruiti dal pool), è uguale a quello che il motore offre in quel deck. L'unica differenza, Charge of the Light Brigade, viene dal costo, che manda al Cimitero le prime 3 carte del Deck: l'ordine del Deck cambia ciò che resta da cercare, e con il seme 2 le due letture coincidono. Uscite complete in `/mnt/project-files/analisi/sinergie/f0-summary.txt` e `f0-readings.tsv`.
- **Come si legge** (`crates/ygo-solver/src/synergy/read.rs`). La carta sta in mano con tre Black Luster Ritual e un Mystical Elf: carte che non fanno niente, ma pagano scarti, Tributi e la rivelazione di Small World. Nel primo momento libero si usa, un turno per volta, ciascuna delle sue Evocazioni ed effetti (al massimo 4 turni); poi le sue azioni non ancora usate e i suoi effetti innescati; le altre carte non si attivano mai. A ogni scelta di carte si registrano le carte offerte fuori dal campo, per suggerimento della scelta (`HINTMSG_*`) e per zona. Si prendono prima le carte preferite (nel confronto, quelle del deck vero) e poi i codici più bassi, così le due letture fanno le stesse scelte.
- **Il pool** sono tutte le carte con uno script, più i Mostri Normali (quasi tutti senza script, come Elemental HERO Neos); fuori le illustrazioni alternative e i token. Una carta «sempre trattata come» un'altra (Fallen of the White Dragon) ha un alias lontano dal codice ed è una carta a sé. Le due regole sono nate dalla prova: senza, due confronti fallivano.
- **Limiti, per F1.** Si legge solo ciò che la carta fa dalla mano in un turno vuoto: il Cimitero è vuoto, quindi i recuperi non offrono niente, e gli effetti che vogliono un'altra carta sul campo non partono. Una sola scelta per decisione: i rami di Small World dipendono dal mostro rivelato. I mostri dell'Extra Deck non si leggono dalla mano: servono carte già sul campo (`Placement`). Il tempo va quasi tutto nel gioco, non nella creazione del duello: prima della lettura completa conviene guardare dove (ad esempio un duello salvato e ripreso per carta).
- **Riassunto leggibile.** Dall'elenco delle carte accettate si ricava una regola sulle proprietà, ad esempio «LIGHT e Livello 4 o meno», con un albero di decisione piccolo sulle colonne del database. Serve a mostrare il ponte («cerca un mostro LIGHT di Livello 4 o meno») e a dire perché una carta è accettata. La verità resta l'elenco.
- **Rete di sicurezza statica.** Dove la ricerca breve non arriva ad attivare l'effetto (costi, condizioni sul campo), si guarda lo script: le funzioni di filtro senza `IsSetCard` né `IsCode` che chiamano `IsAttribute`, `IsRace` o `IsLevel` (il censimento ne trova 2 589). Queste righe si marcano «da script, non provate».

**(b) Che cosa chiedono i materiali dell'Extra Deck.** Le procedure (`Fusion.AddProcMix`, `Synchro.AddProcedure`, `Xyz.AddProcedure`, `Link.AddProcedure`) ricevono filtri come `aux.FilterBoolFunctionEx(Card.IsAttribute, ATTRIBUTE_DARK)`. Una funzione Lua nostra caricata come `Replacement` (come in `scripts.rs`) avvolge i costruttori dei filtri di `aux` e le procedure, e scrive con `Debug.Message` i parametri: «materiale 1: setcode X; materiale 2: Attributo DARK», «Synchro: 1 Tuner + 1 o più non Tuner», «Xyz: 2 mostri di Livello 4». Nessuna modifica a `vendor/`, come il lettore dell'anatomia. I filtri scritti a mano negli script si leggono con la lettura (a), mettendo i mostri sul campo nel duello di lettura.

**(c) Il profilo di un archetipo.** Per ogni archetipo: i membri con i loro ruoli (dalla lista di riferimento, sotto), le proprietà dei suoi mostri (Attributi, Tipi, Livelli, ATK/DEF), che cosa accettano i suoi effetti (unione dei predicati), che cosa offre agli altri (corpi, eventi), i blocchi di Evocazione, se il suo Starter usa l'Evocazione Normale. Si calcola quando serve e si tiene in memoria per versione degli script.

**(d) Le liste di riferimento.** Per provare un mix serve un deck di A e uno di B. In ordine: il deck della Bottega scelto dall'utente; la lista più recente di Master Duel Meta con quell'archetipo; altrimenti uno **scheletro** costruito dai dati (i membri legali nel formato, 3 copie dei mostri che cercano o Evocano, 1-2 delle altre, il resto carte che non fanno niente), segnato come meno affidabile. Le liste si scaricano e non si committano mai.

**(e) Dove stanno.** Tutto il derivato (predicati letti, profili, indice delle proprietà) sta in `data/synergy/`, con la versione degli script e del database nella chiave: si ricalcola dopo `fetch-data`, mai nel repository. L'indice delle proprietà è istantaneo; i predicati si leggono una volta per carta, in background (misurato in F0: 3,1 s a carta, circa 2 ore e 40 minuti per tutto il Main Deck del pool su 4 core), oppure solo per gli archetipi richiesti, in pochi secondi.

## 4. Come si trovano i ponti

### 4.1 I tipi di ponte
| Tipo | Definizione, solo con dati | Da dove si legge |
|---|---|---|
| **Ricerca ed Evocazione** | un effetto di A accetta carte di B (o il contrario): cerca, Evoca, manda al Cimitero, recupera | predicati letti (§3.3 a) |
| **Carta terza** | una carta fuori da A e da B (spesso una carta generica) accetta carte di entrambi: entra nel pacchetto come colla | predicati letti, su tutto il pool |
| **Materiali** | un mostro dell'Extra Deck ha materiali che A e B soddisfano insieme: un setcode di A e un Attributo di B, un Tuner di B con i Livelli di A, due mostri di pari Livello per uno Xyz, due corpi per un Link | procedure lette (§3.3 b), proprietà del database |
| **Small World** | mostro in mano → mostro del Deck con esattamente una proprietà in comune → mostro da aggiungere, con A all'inizio e B alla fine | regola già in `special.rs`, sui due insiemi di mostri |
| **Eventi** | A produce un evento (manda al Cimitero da un effetto, bandisce, scarta, Evoca Specialmente dal Deck), B ha un effetto innescato da quell'evento; vale anche il costo pagato con le carte dell'altro | anatomia (categorie e costi di A, eventi di B) |
| **Motore condiviso** | un gruppo piccolo di carte che arriva a una board da solo, senza blocchi o con blocchi compatibili, e che entra in molti deck | liste del meta (insiemi frequenti) e prova del motore su un mini deck |
| **Anti-ponte** | ciò che rompe il mix: un blocco «solo A» mentre B Evoca dall'Extra Deck, due Starter che vogliono la stessa Evocazione Normale, carte che si contendono un limite per turno | anatomia (restrizioni, limiti), ruoli |

### 4.2 Il grafo
Un grafo con nodi di più tipi: carte, valori delle proprietà (DARK, Spellcaster, Livello 4), eventi, procedure dell'Extra Deck. Gli archi hanno un tipo: «è membro di», «ha la proprietà», «accetta» (un effetto e la carta che può prendere), «produce l'evento», «si innesca con», «è materiale di», «Small World verso». Un ponte tra A e B è un cammino corto con archi di tipi consentiti, ad esempio carta di A → accetta → carta di B, oppure carta di A → è materiale di → mostro dell'Extra Deck ← è materiale di ← carta di B. È lo schema dei meta-path dei grafi eterogenei (§6.1).

### 4.3 La stima di un partner
Per A fissato, ogni archetipo B riceve una stima fatta di cinque parti, ognuna in [0, 1] e mostrata da sola nell'interfaccia:
1. **Forza dei ponti**: i cammini tra A e B per tipo, normalizzati sulla taglia dei due archetipi (come PathSim, perché un archetipo enorme non vinca solo per numero), pesati per il ruolo delle carte toccate (un ponte verso uno Starter di B vale più di uno verso un Mattone).
2. **Raggiungibilità**: la probabilità esatta (Laboratorio) che una mano del mix abbia almeno una carta che apre un ponte, con il pacchetto proposto.
3. **Complementarità**: B dà ad A ciò che gli manca (un secondo Starter, corpi dell'Attributo che servono ai suoi materiali, risorse per il turno dopo), non ciò che ha già. È la differenza tra prodotti complementari e sostituti (§6.4).
4. **Compatibilità**: 1 meno la penalità degli anti-ponti.
5. **Novità**: quanto il mix è raro nelle liste (lift basso). Non entra nella stima: è un filtro e un'etichetta («nuovo», «noto nel meta»).

Il peso delle parti nasce uguale e si tara sui risultati del motore (§5.8), mai a mano per nome.

### 4.4 Il pacchetto
Il pacchetto è l'insieme di carte di B (e di carte terze) che entra in A, da 3 a 15, con le carte di A che escono.
- **Dentro**: le carte di B che sono ponti; gli Starter di B (ruoli sulla lista di riferimento di B); le carte che le linee migliori di B usano partendo da quegli Starter (collegamenti «usati» di B); le carte terze; i mostri dell'Extra Deck dei ponti di materiali.
- **Il pacchetto minimo** di ponti è una copertura: le poche carte che coprono la maggior parte dei collegamenti tra A e B, scelte con l'algoritmo goloso della copertura d'insiemi (§6.10).
- **Fuori**: le carte di A che più di rado fanno una board quando sono in mano e la linea le usa (lo stesso ordine di `engine-mc --climb`); le handtrap restano, salvo scelta dell'utente; nell'Extra Deck escono i mostri meno usati dai collegamenti di A.
- **Regole del formato**: banlist di Master Duel, 3 copie, Main da 40 a 60, Extra fino a 15.

## 5. Dalla stima alla prova: il motore giudica

### 5.1 La prova di un mix
`engine-mc` gioca le stesse mani (numeri casuali comuni: le mani sono posizioni del Main Deck, e una carta che resta tiene il suo posto) nel deck di partenza e nel mix, con la ricerca del Combo Lab e la misura a gradini. Il risultato è P(board ≥ soglia) dei due, con la soglia del deck di partenza, e la differenza con l'intervallo al 95%. Tutto c'è già; la sezione aggiunge il deck misto e la lettura per strati.

### 5.2 Strati con pesi esatti
Le mani si dividono per numero di carte del pacchetto (nessuna, una, due o più). Il peso di ogni strato è esatto (ipergeometrica, `ygo-stats`), la quota dentro lo strato la misura il motore. La stima pesata (post-stratificazione) ha varianza minore della media semplice e si legge meglio: «con due carte Shaddoll in mano, +19 punti; senza, come prima». Lo strato «nessuna» dice anche quanto costa togliere le carte di A.

### 5.3 La sinergia
Per una mano H con carte di A e di B, v(S) è il valore della board migliore tenendo solo le carte di S e sostituendo le altre con carte che non fanno niente (come i ruoli). La sinergia è

I(H) = v(A∪B) − v(A) − v(B) + v(∅)

cioè l'indice d'interazione tra due giocatori (§6.8): positiva se insieme fanno più della somma. Costa tre ricerche in più per mano, quindi si calcola su un sottoinsieme (40 mani dello strato «due o più» e «una»), con il suo intervallo.

### 5.4 Le linee nuove
Una linea della ricerca è nuova se attiva effetti di carte di A e di B e usa almeno un ponte della mappa (un arco possibile tra i due archetipi). Le linee nuove si raggruppano per la firma dei ponti che usano (ad esempio «materiale DARK + innesco dal Cimitero») e per ogni gruppo si mostrano quante mani lo fanno, la board migliore e il valore, con «Apri nel Combo Lab» sul riproduttore che c'è (`Steps.tsx`). Nulla si salva come combo: è il risultato della ricerca su quelle mani.

### 5.5 Provare un ponte preciso
Per un ponte di materiali o un mostro ponte, la ricerca con obiettivo (`goal.rs`): «El Shaddoll Winda sul campo» dalle mani del mix. Dice quanto spesso il ponte si raggiunge e da quali mani, senza confondere il ponte con il valore della board.

### 5.6 I collegamenti nuovi
`deck_links` sul mix e sul deck di partenza, poi `compare_links`: i collegamenti guadagnati tra carte di archetipi diversi. Nella scheda «Collegamenti» si accendono.

### 5.7 Tanti candidati, poco tempo
Provare un mix con 200 mani costa circa 10 minuti su 4 core. Per scegliere tra molti partner o pacchetti si usano i dimezzamenti successivi (§6.6):

| Giro | Candidati | Mani | Nodi | Mani cercate |
|---|---|---|---|---|
| 1 | 8 | 24 | 1000 | 192 |
| 2 | 4 | 60 | 2000 | 240 |
| 3 | 2 | 200 | 2000 | 400 |

Con le 200 mani del deck di partenza, in comune, sono circa 1 030 mani cercate: 25-30 minuti su 4 core (si lancia e si torna). La versione rapida è solo il primo giro, circa 5 minuti. Il vincitore si riconferma su mani nuove (un altro seme), perché tra 8 candidati uno può vincere per caso (`monte-carlo-motore.md`, confronto multiplo); gli intervalli usano la correzione di Holm.

### 5.8 La stima impara dal motore
Ogni prova lascia una riga: le cinque parti della stima e la differenza misurata. Con una ventina di prove si tarano i pesi della stima con una regressione piccola (poche caratteristiche, nessun nome di carta), e si controlla che l'ordine della stima e quello del motore vadano d'accordo (correlazione di Spearman). È lo stesso principio delle revisioni bibliografiche: le caratteristiche vengono dagli script e dal motore, non dall'identità delle carte (`revisione-bibliografica.md`, gruppo 6).

### 5.9 Più di due archetipi
Golosamente: si trova il miglior B per A, poi il mix A+B diventa il nuovo punto di partenza e si cerca C; ci si ferma quando la differenza non è più sopra zero. Con un fascio di 2 (si tengono i due migliori a ogni passo) i costi restano lineari. Più avanti, un archivio alla MAP-Elites (§6.9) tiene mix diversi e buoni per descrittori: quota di carte fuori da A, numero di archetipi, tipo di board (interazione a fine turno o valore).

## 6. Metodi e letteratura
Le fonti sotto sono note e si citano per il metodo; dove una cosa è stata controllata il 25/10 lo dice.

### 6.1 Grafi eterogenei e meta-path
- Sun, Han, Yan, Yu, Wu, *PathSim: Meta Path-Based Top-K Similarity Search in Heterogeneous Information Networks*, VLDB 2011. Similarità tra nodi dello stesso tipo contando i cammini di un tipo dato e normalizzando per i cammini di ciascuno con sé: evita che i nodi grandi vincano solo per numero. Per noi: la forza dei ponti per tipo (§4.3, parte 1).
- Yu, Ren, Sun, Gu, Sturt, Khandelwal, Norick, Han, *Personalized Entity Recommendation: A Heterogeneous Information Network Approach*, WSDM 2014. Raccomandazione che combina più meta-path con pesi imparati. Per noi: i pesi dei tipi di ponte imparati dal motore (§5.8).

### 6.2 Passeggiate con ritorno
- Page, Brin, Motwani, Winograd, *The PageRank Citation Ranking*, 1999; Haveliwala, *Topic-Sensitive PageRank*, WWW 2002; Tong, Faloutsos, Pan, *Fast Random Walk with Restart and Its Applications*, ICDM 2006. La vicinanza di ogni nodo a un insieme di partenza, sommando tutti i cammini con un peso che cala con la lunghezza. Per noi: dalle carte di A, quanto è vicino ogni archetipo, anche per molti ponti deboli; è il controllo della stima a cammini corti.

### 6.3 Previsione dei collegamenti
- Liben-Nowell, Kleinberg, *The Link-Prediction Problem for Social Networks*, JASIST 2007; Adamic, Adar, *Friends and Neighbors on the Web*, Social Networks 2003. Le misure di base (vicini comuni, Adamic-Adar, Katz) e il protocollo: nascondere archi veri e vedere se si ritrovano. Per noi: il protocollo della riscoperta dei mix del meta (§9).

### 6.4 Complementari, non simili
- McAuley, Pandey, Leskovec, *Inferring Networks of Substitutable and Complementary Products*, KDD 2015. Distingue i prodotti che si sostituiscono da quelli che si comprano insieme. Un partner che fa la stessa cosa di A è un sostituto; serve quello che riempie i buchi di A.
- Pathak, Gupta, McAuley, *Generating and Personalizing Bundle Recommendations on Steam*, SIGIR 2017 (controllato il 25/10). Che cosa rende buono un pacchetto e come generarne di nuovi da componenti: è il nostro «pacchetto» di B.

### 6.5 Co-occorrenza e lift
- Agrawal, Srikant, *Fast Algorithms for Mining Association Rules*, VLDB 1994. Insiemi frequenti e regole, con supporto, confidenza e lift. Per noi: i motori condivisi (insiemi di carte frequenti in deck di archetipi diversi) e la novità di un mix.
- EDHREC (Magic, formato Commander): calcolava la «synergy» come quota dei deck di un comandante che giocano una carta meno la quota nei deck dei suoi colori, e l'ha sostituita con il lift, più solido (articolo «From Synergy to Lift», controllato il 25/10: https://edhrec.com/articles/from-synergy-to-lift-the-math-behind-edhrecs-new-era). È lo strumento più vicino nello spirito, ma vede solo l'uso: dice che cosa si gioca insieme, non perché né se funziona. Noi usiamo il lift solo come etichetta di novità.

### 6.6 Scegliere con un budget
- Maron, Moore, *Hoeffding Races*, NIPS 1994; Karnin, Koren, Somekh, *Almost Optimal Exploration in Multi-Armed Bandits*, ICML 2013 (dimezzamenti successivi); Jamieson, Talwalkar, *Non-stochastic Best Arm Identification and Hyperparameter Optimization*, AISTATS 2016; Li, Jamieson, DeSalvo, Rostamizadeh, Talwalkar, *Hyperband*, JMLR 2018. Per noi: §5.7. È lo stesso principio della «misura a dimezzamenti» proposta per le finaliste del motore (`prossimi-passi-motore.md`).

### 6.7 Varianza del Monte Carlo
- Numeri casuali comuni e stratificazione con pesi esatti: Owen, *Monte Carlo Theory, Methods and Examples*, 2013, capitoli sulla riduzione della varianza. I numeri casuali comuni ci sono già in `engine-mc`; la post-stratificazione per numero di carte del pacchetto è nuova (§5.2).

### 6.8 La sinergia come interazione
- Shapley, *A Value for n-Person Games*, 1953; Grabisch, Roubens, *An Axiomatic Approach to the Concept of Interaction among Players in Cooperative Games*, International Journal of Game Theory 1999. Con due giocatori l'indice d'interazione è v(AB) − v(A) − v(B) + v(∅): è la nostra sinergia (§5.3), con le carte che non fanno niente al posto dei giocatori assenti.

### 6.9 Costruire deck nei giochi di carte
- García-Sánchez, Tonda, Squillero, Mora, Merelo, *Evolutionary Deckbuilding in Hearthstone*, IEEE CIG 2016; Bhatt, Lee, de Mesentier Silva, Watson, Togelius, Hoover, *Exploring the Hearthstone Deck Space*, FDG 2018; Fontaine, Lee, Soros, de Mesentier Silva, Togelius, Hoover, *Mapping Hearthstone Deck Spaces through MAP-Elites with Sliding Boundaries*, GECCO 2019; Mouret, Clune, *Illuminating Search Spaces by Mapping Elites*, 2015. Il deck si valuta giocandolo, e ogni valutazione costa: per questo archivi per descrittori e pochi candidati scelti bene. Da noi la valutazione è `engine-mc` e la scelta dei candidati è la stima dei ponti.
- Ward, Brooks, Troha, Mills, Khakhalin, *AI Solutions for Drafting in Magic: the Gathering*, IEEE CoG 2021 (arXiv 2009.00655, controllato il 25/10). Agenti di draft imparati dalle scelte umane: imparano che cosa i giocatori mettono insieme, non che cosa funziona nelle regole. Il contrasto con noi è utile: i nostri ponti vengono dalle regole e dal motore.

### 6.10 Il pacchetto minimo
- Chvátal, *A Greedy Heuristic for the Set-Covering Problem*, Mathematics of Operations Research 1979. Il goloso è vicino all'ottimo (fattore logaritmico) ed è istantaneo sulle nostre taglie.

### 6.11 Spiegare un predicato
- Quinlan, *Induction of Decision Trees*, Machine Learning 1986. Un albero piccolo sulle proprietà del database riassume l'elenco delle carte accettate in una regola da leggere (§3.3 a).

### 6.12 Strumenti esistenti (ricerca del 25/10)
- `ygo-small-world` (GitHub, KennethJAllen): i ponti di Small World dentro un `.ydk`, con la matrice e il grafo. Solo Small World, solo dentro un deck, senza motore.
- Nessuno strumento trovato che unisca archetipi leggendo gli effetti e verificando col motore. I calcolatori di probabilità contano le carte, EDHREC conta l'uso.

### 6.13 Scartati
- **Rappresentazioni imparate delle carte** (reti sui grafi, card2vec dalle liste): servono dati di addestramento, generalizzano male alle carte nuove senza caratteristiche lette dalle regole (`revisione-bibliografica.md`, gruppo 6) e non spiegano il ponte. Forse più avanti, come una delle parti della stima, mai come giudice.
- **Un modello linguistico che legge i testi di ogni coppia**: costa per coppia, non si verifica e contraddice «leggiamo il motore».
- **Un archivio di combo e mix noti**: contro il principio di nessuna linea precalcolata.

## 7. L'interfaccia
Il mockup ha due schermate nei due temi (Partner; Ponti e prova). Si rifinisce in Claude Design prima del codice (fase 5a).

### 7.1 Partner
- A sinistra «Parti da»: un deck della Bottega o un archetipo; sotto, il profilo (Attributi, Tipi e Livelli più frequenti, Starter, Extender e Mattoni dai ruoli), «Che cosa cerca nei partner» (i buchi di A: corpi per i materiali, un secondo Starter) e i filtri (taglia del pacchetto, solo carte legali, nascondi i mix già nel meta, cerca un terzo archetipo).
- Al centro la lista dei partner: nome, etichetta «nuovo» o «noto nel meta», i ponti contati per tipo con il colore del tipo, il blocco che li ostacola, la barra della stima, l'esito del motore se c'è («+9 punti», «−2 punti», «da provare»).
- A destra «Perché»: le frasi dei ponti principali (ognuna dai dati: «Aleister è DARK: fa da materiale DARK di El Shaddoll Winda»), il pacchetto proposto e «Prova col motore» con il costo atteso.

### 7.2 Ponti e prova
- A sinistra la mappa dei ponti in tre colonne (carte di A, ponti, carte di B), archi colorati per tipo con gli stessi colori dei Collegamenti dove il tipo coincide, spessi se usati dalle linee migliori; sotto, i ponti minimi.
- A destra la prova: A da solo, il mix, la differenza con l'intervallo; le quote per numero di carte di B in mano; sinergia, mani bloccate, mattoni; poi le linee nuove raggruppate, ognuna con «Apri nel Combo Lab», e il link ai collegamenti nuovi.
- In alto «Modifica il pacchetto» (si tolgono e aggiungono carte con le regole del formato) e «Crea il deck nella Bottega».

### 7.3 Analizza un deck
Da disegnare nella fase 5a: il deck scomposto in archetipi (barre per quota di carte), i ponti usati dalle linee migliori, il valore di ogni archetipo da solo e la sinergia.

### 7.4 Regole dell'interfaccia (da `AGENTS.md`)
- Ogni caricamento con id di richiesta, legato al deck di partenza e al partner: cambiando l'uno o l'altro si azzera tutto e la prova in corso si annulla.
- La prova mostra l'avanzamento (mani cercate, giro dei dimezzamenti) e si può fermare; un errore arriva in italiano in un riquadro.
- Dati con i tipi del contratto; fixture registrate dal backend vero per l'anteprima.
- Componenti che ci sono: `CardTile`, `AttributeIcon`, i colori dei Collegamenti, `Steps.tsx`, `InfoButton` con l'Arcimago; nuovi: la mappa a tre colonne, la barra degli strati, l'emblema del crogiolo in `Emblems.tsx` e l'accento `data-section="synergy"` in `index.css`.

### 7.5 Il contratto (bozza per la fase 4)
Tipi in `lib.rs` e `api.ts` insieme: `ArchetypeProfile`, `PartnerCandidate` (parti della stima, ponti per tipo, anti-ponti, novità, esito), `Bridge` (tipo, carte di A, carte di B, carta terza o mostro ponte, regola leggibile, «provato» o «da script»), `MixPackage` (dentro, fuori, Extra Deck), `MixTrial` (quote, differenza, intervallo, strati, sinergia, mani bloccate, gruppi di linee nuove con una mano e una linea ciascuno, collegamenti nuovi). Comandi: `synergy_profile`, `synergy_partners`, `synergy_bridges`, `synergy_trial` (con eventi di avanzamento e annullamento), `synergy_apply` (salva il mix nella Bottega).

## 8. Fasi, brief e costi
Costi per PR stimati dalla tabella del piano ambiente (una sessione, contesto sotto 200k): Opus alto 25-30 $, Sonnet medio 20-25 $. La CPU dei calcoli lunghi è a parte e gira in background.

| Fase | Che cosa | Modello | Dipende da | Costo |
|---|---|---|---|---|
| F0 | Prova del deck di lettura e censimento dalla CLI (fatta il 25/10) | Opus 5.5 alto | nulla | 15-20 $ |
| F1 | Predicati letti, materiali, profili d'archetipo, cache in `data/synergy/` | Opus 5.5 alto | F0; l'anatomia (Brief A della #181) per eventi e blocchi | 25-30 $ |
| F2 | Grafo dei ponti, stima dei partner, pacchetto; CLI `partners` e `bridges`; prova di riscoperta | Opus 5.5 alto | F1 | 25-30 $ |
| F3 | Prova col motore: `mix-trial` con strati, sinergia, linee nuove, collegamenti nuovi, dimezzamenti | Opus 5.5 alto | F2 | 25-30 $ |
| F4 | Contratto e backend Tauri, fixture registrate | Opus 5.5 alto | F3 | 15-20 $ |
| F5a | Mockup rifinito, terza schermata | chat del progetto, Claude Design | questo piano | 3-5 $ |
| F5b | Interfaccia: Partner, Ponti | Sonnet 5.5 medio | F4, F5a | 20-25 $ |
| F5c | Interfaccia: Prova, linee nuove, Analizza un deck | Sonnet 5.5 medio | F5b | 20-25 $ |
| F6 | Più archetipi, archivio dei mix, taratura della stima | Opus 5.5 alto | F3 e 20 prove | 25-30 $ |

Totale circa 170-215 $. Le fasi F0-F3 sono in fila; F5a può andare in parallelo a tutto; F4 tocca i file caldi `lib.rs` e `api.ts` e va chiesta in `TASKS.md`. Nessuna fase tocca `search.rs`, `probe.rs` o `value.rs`: il codice nuovo sta in `crates/ygo-solver/src/synergy/` (nuovo) e in `ygo-engine` solo per le funzioni Lua di lettura in `scripts.rs`. Le PR del motore sono di classe E (le uscite delle linee non cambiano) e lo dicono con `banco.py same`.

### Brief F0: deck di lettura e censimento
```
Compito: duelforge-cli synergy-census scrive out/synergy-census/summary.txt (il censimento del §2 dai dati del motore, non da regex) e prova il deck di lettura: per 30 carte di prova (cerca per Attributo, per Tipo, per Livello, per archetipo, Small World, carte che Evocano dal Deck) l'elenco delle carte del pool che il motore offre a ogni scelta, con tempo per carta.
Perché: docs/plans/sinergie-archetipi.md §2 e §3.3 (a).
Branch: claude/synergy-census, da main
File da toccare: crates/ygo-solver/src/synergy/mod.rs e census.rs (nuovi), crates/ygo-solver/src/lib.rs, crates/ygo-cli/src/main.rs; test con fixture piccole.
Da leggere: sinergie-archetipi.md §2-§3; links.rs (scelte offerte); roles.rs (carte che non fanno niente); carddb.rs (setcodes).
Verifica: cargo test -p ygo-solver -p ygo-cli; synergy-census sui dati veri; search_regression; banco.py same (classe E).
Criterio di accettazione: il deck di lettura con tutto il pool regge (o il campione stratificato, se no) e il tempo per carta è misurato; per le 30 carte l'elenco letto coincide con quello che il motore offre in un deck vero che le contiene.
PR: template; summary.txt per intero; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati (§3.3 con i numeri), PR aperta, fermati.
```

### Brief F1: predicati, materiali e profili
```
Compito: data/synergy/ con, per ogni carta, ciò che accetta ogni effetto che sceglie (letto col deck di lettura, con la regola leggibile dall'albero di decisione) e i requisiti dei materiali dell'Extra Deck (funzioni Lua di lettura su aux e sulle procedure); duelforge-cli archetype --profile <nome|setcode> e accepts --card <code>.
Perché: docs/plans/sinergie-archetipi.md §3.3.
Branch: claude/synergy-data, da main dopo F0
File da toccare: crates/ygo-engine/src/scripts.rs (Replacement per aux.FilterBoolFunction* e le procedure), crates/ygo-solver/src/synergy/{read.rs,profile.rs,cache.rs}, crates/ygo-cli; test con fixture.
Da leggere: sinergie-archetipi.md §3; anatomia-effetti.md §2.1 (stesso meccanismo di lettura); i numeri di F0 nel §3.3.
Verifica: cargo test -p ygo-engine -p ygo-solver -p ygo-cli; lettura completa in background con out/synergy-data/summary.txt (copertura per tipo di effetto, carte non lette e perché); banco.py same.
Criterio di accettazione: oltre il 90% degli effetti che scelgono carte letti dal motore, il resto marcato «da script»; profili di 10 archetipi del banco controllati a mano su 3 carte ciascuno.
PR: template; summary.txt; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief F2: ponti e partner
```
Compito: duelforge-cli partners --deck <ydk>|--archetype <nome> dà i partner ordinati con le cinque parti della stima, i ponti per tipo e il pacchetto proposto; bridges --a <A> --b <B> dà la mappa dei ponti e i ponti minimi. Prova di riscoperta sulle liste miste del meta.
Perché: docs/plans/sinergie-archetipi.md §4 e §9.
Branch: claude/synergy-bridges, da main dopo F1
File da toccare: crates/ygo-solver/src/synergy/{graph.rs,bridges.rs,partners.rs,package.rs}, crates/ygo-cli; scripts/synergy_rediscover.py (nuovo).
Da leggere: sinergie-archetipi.md §4, §6.1-§6.5, §9; carte-speciali.md (Small World); monte-carlo-motore.md (ordine di climb per le carte che escono).
Verifica: cargo test -p ygo-solver -p ygo-cli; synergy_rediscover.py sulle liste di mdm_decks.py e del banco con più archetipi, out/synergy-rediscover/summary.txt; banco.py same.
Criterio di accettazione: scritto prima di misurare, nel piano: l'archetipo nascosto è tra i primi 10 partner in almeno il 60% delle liste miste; ogni ponte mostrato ha la sua fonte (provato o da script).
PR: template; summary.txt; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief F3: la prova col motore
```
Compito: duelforge-cli mix-trial --deck <A.ydk> --with <B> [--package N] gioca il deck e il mix sulle stesse mani (engine-mc), con strati a pesi esatti, sinergia su un sottoinsieme, linee nuove raggruppate per ponti, collegamenti nuovi; --candidates K sceglie tra K partner o pacchetti con i dimezzamenti del §5.7.
Perché: docs/plans/sinergie-archetipi.md §5.
Branch: claude/synergy-trial, da main dopo F2
File da toccare: crates/ygo-solver/src/synergy/{trial.rs,lines.rs}, crates/ygo-solver/src/engine_mc.rs (solo per il deck misto e gli strati, senza cambiare le uscite di oggi), crates/ygo-cli.
Da leggere: sinergie-archetipi.md §5, §6.6-§6.8; monte-carlo-motore.md; links.rs (compare_links); goal.rs.
Verifica: cargo test -p ygo-solver -p ygo-cli; engine-mc di oggi con uscite identiche; mix-trial su 5 coppie (3 note nel meta, 2 nuove) in background, out/synergy-trial/summary.txt; banco.py same.
Criterio di accettazione: in almeno 3 coppie su 5 il mix proposto ha la differenza sopra zero con l'intervallo; il vincitore dei dimezzamenti si riconferma su un altro seme; ogni linea nuova si apre nel riproduttore.
PR: template; summary.txt per intero; tempi per coppia; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief F4: contratto e backend
```
Compito: comandi Tauri synergy_profile, synergy_partners, synergy_bridges, synergy_trial (avanzamento e annullamento), synergy_apply, con i tipi in lib.rs e api.ts, mock.ts e le fixture registrate dal backend vero.
Perché: docs/plans/sinergie-archetipi.md §7.5.
Branch: claude/synergy-contract, da main dopo F3
File da toccare: apps/desktop/src-tauri/src/lib.rs e apps/desktop/src/lib/api.ts (file caldi: chiederli in TASKS.md), src/lib/mock.ts, src/lib/fixtures/, record-fixtures.
Da leggere: sinergie-archetipi.md §7; AGENTS.md (protocollo dell'interfaccia, punto 2).
Verifica: cargo test; pnpm typecheck; pnpm build; record-fixtures con i dati veri; una chiamata di prova per comando dall'app vera con DUELFORGE_DATA.
Criterio di accettazione: lib.rs, api.ts, mock.ts e fixture nella stessa PR; synergy_trial si annulla cambiando deck; nessun errore muto.
PR: template; «Non tocca il motore».
Modello: Opus 5.5, effort alto (contratto).
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

### Brief F5b: interfaccia, Partner e Ponti
```
Compito: sezione «Sinergie» (Il Crogiolo) con le viste Partner e Ponti e prova come nel mockup rifinito, accento data-section="synergy", emblema del crogiolo.
Perché: docs/plans/sinergie-archetipi.md §7.
Branch: claude/synergy-ui, da main dopo F4 e F5a
File da toccare: apps/desktop/src/pages/Synergy.tsx (nuovo), src/components/synergy/ (nuovi), src/sections.ts, src/components/Emblems.tsx, src/index.css, src/pages/Gallery.tsx (le DsCard nuove) e apps/desktop/design/.
Da leggere: sinergie-archetipi.md §7; mockup https://claude.ai/artifact/WDzepFuEQTNKsXL5CDMhoJ (versione rifinita della F5a); docs/plans/sistema-di-design.md.
Verifica: pnpm typecheck; pnpm build; VITE_MOCK=1 pnpm dev e node scripts/screens.mjs per la pagina nuova nei due temi; anteprima Artifact allo stesso URL; app vera con DUELFORGE_DATA per una prova di un mix.
Criterio di accettazione: le due viste uguali al mockup nei due temi a 1280×800; cambiando deck o partner i dati vecchi spariscono e la prova si annulla; errori in italiano.
PR: template; screenshot nei due temi; come è stato verificato ogni punto.
Modello: Sonnet 5.5, effort medio.
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

F5c (Prova, linee nuove, Analizza un deck) e F6 (più archetipi, archivio alla MAP-Elites, taratura della stima con le prove raccolte) hanno il loro brief quando F5b e F3 sono unite: dipendono da che cosa i numeri di F3 dicono.

## 9. Come si misura se funziona
Ogni criterio si scrive nella PR della sua fase prima di misurare.
- **Riscoperta (F2).** Sulle liste del meta e del banco con due o più archetipi, si nasconde un archetipo e si guarda a che posto la stima lo rimette tra i partner dell'altro. Obiettivo: tra i primi 10 in almeno il 60% dei casi, contro il caso (circa 10 su 140) e contro una base che usa solo i setcode condivisi.
- **Il motore conferma (F3).** Su 5 coppie, il mix proposto migliora P(board ≥ soglia) in almeno 3, con l'estremo basso dell'intervallo sopra zero; su 20 prove (F6), l'ordine della stima e quello del motore hanno una correlazione di Spearman sopra 0,3.
- **Linee nuove vere (F3).** Ogni gruppo di linee nuove ha almeno una mano con sinergia positiva e si riapre nel riproduttore con la stessa board.
- **Giudizio di Valerio (F5b).** Su tre archetipi che conosce, i primi cinque partner e i loro ponti hanno senso; ciò che non lo ha diventa un caso da capire, mai una regola per nome.
- **Nessuna regressione.** Le PR del motore sono di classe E con `banco.py same`; la misura del Combo Lab non cambia.

## 10. Rischi e limiti
- **Il deck di lettura può non reggere** (memoria, script che contano il Deck, condizioni mai vere). Misurato in F0: regge (§3.3); restano le condizioni mai vere in un turno vuoto, e per quelle la rete statica. Ciò che non si legge resta «sconosciuto», mai indovinato.
- **Le etichette degli archetipi**: setcode e nome di YGOPRODeck non coincidono sempre, e le famiglie (Number, Xyz) gonfiano i ponti. Si usa la gerarchia dei setcode e le famiglie si mostrano a parte.
- **Liste di riferimento mancanti**: lo scheletro è meno affidabile e lo dice.
- **La misura ha i suoi gusti**: un'handtrap tenuta in mano vale 0,9 nel profilo «Bilanciato» (`monte-carlo-motore.md`), quindi un pacchetto che toglie handtrap perde. Le handtrap restano fuori dagli scambi di base.
- **Tempo**: una prova completa costa 25-30 minuti su 4 core. Si lancia e si torna, con avanzamento e annullamento; la versione rapida è di circa 5 minuti; i risultati restano in cache per versione del deck.
- **Evocazione Normale e limiti per turno**: due Starter che vogliono la stessa Evocazione Normale non si sommano. La stima lo segna come anti-ponte; il motore lo vede da solo.
- **Nuovo non vuol dire buono**: le etichette «nuovo» e «noto nel meta» stanno accanto all'esito, non al posto.

## 11. Decisioni (approvate da Valerio il 25/10: tutte e quattro come consigliato)
1. **Nome e posto**: sezione propria «Sinergie» / «Il Crogiolo» nella barra (consigliato), oppure una scheda del Laboratorio. La sezione propria regge tre viste e i collegamenti verso tutte le altre.
2. **Da dove si parte**: dal deck della Bottega, con l'archetipo nudo come seconda scelta (consigliato), oppure solo dall'archetipo.
3. **Liste del meta**: solo Master Duel Meta, come oggi (consigliato), oppure anche le liste di YGOPRODeck, dopo averne controllato i termini d'uso.
4. **Più di due archetipi**: dopo la fase 3, quando i numeri della prova sono noti (consigliato).
