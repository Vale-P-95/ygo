# Revisione del motore delle combo: cosa deve essere, dove sbaglia, che strada prendere

*Scritto il 22/10/2026 dalla corsia 9 (sessione sorella di Claude, branch `claude/engine-review`), su richiesta del proprietario tramite il coordinatore. È una revisione indipendente, da principi primi, della ricerca delle combo (`crates/ygo-solver`: `search.rs`, `probe.rs`, `value.rs`, `matchup.rs`, `follow.rs`) e del suo aggancio al motore (`crates/ygo-engine`). Ho letto i piani (`ricerca-piano-critico.md`, `ricerca-letteratura.md`, `revisione-bibliografica.md` della corsia 10, `ricerca-esperimenti.md` della corsia 7, `velocita-ricerca.md`, `classificazione-board.md`, `banco-oggettivo.md`, `taratura-pesi.md`, `valutatore-imparato.md`, `esposizione.md`) e il codice. Non ho modificato né `crates/` né `apps/`. Le prove fatte qui sono in §9; il confronto con la revisione bibliografica della corsia 10 in §10; le fonti che ho letto io in §11.*

*Lo stile è quello del progetto: frasi corte, e il perché di ogni affermazione. Dove dico «questo è sbagliato» porto la riga di codice o il numero che lo mostra. Dove non ho potuto verificare, lo dico.*

## 0. In breve: la strada consigliata

1. **Il collo di bottiglia non è l'algoritmo di ricerca. È la funzione che la guida.** La stima rapida ordina le board con una correlazione di Spearman 0,57 con la misura, e in 10 mani su 82 la board migliore vera è oltre la 30ª posizione (corsia 7, §1). Una ricerca perfetta guidata da questa stima trova board sbagliate. L'oracolo a 10 volte il budget che resta al 55% su inv-r6 ne è la prova: più nodi non aiutano se la bussola è storta.
2. **La misura è anche l'obiettivo, e l'obiettivo non è definito.** «Board più forte» oggi vuol dire «ciò che il turno di prova contro il kit premia». Il kit è un avversario scritto a mano, e il suo accordo con il livello 2 (linee vere del meta) è debole: Spearman 0,52 (corsia 7, §1). La definizione va data: valore atteso dell'esito contro gli avversari del meta, pesati per diffusione. Il kit resta uno strumento per *misurare* componenti, non la verità.
3. **L'architettura (best-first con tuffi, tabella delle trasposizioni, stima rapida, misura cara sulle finaliste) è giusta per questo problema.** Non va sostituita con MCTS, beam o altro. Vanno cambiate tre cose dentro di essa: la stima (da formula additiva a piccola simulazione), il modo in cui la misura corregge la ricerca (da «alza i valori delle carte» a «riordina le board intere»), e il replay (arena). Più una correzione precisa: la tabella delle trasposizioni chiude un nodo alla prima visita, anche quando il primo cammino era il più penalizzato (§3.2, punto 10).
4. **L'arena di memoria è la sola leva da ordine di grandezza sul tempo, ed è già dimostrata (ripristino 22 volte più economico del replay).** Va portata sia nella ricerca sia nel turno di prova, dove il 69% del tempo è il ciclo delle risposte che rigioca il tuo turno a ogni finestra. Il resto delle ottimizzazioni della ricerca (ordine dei fratelli, ordini scambiabili, chiavi dei nodi) vale percentuali.
5. **La ricerca per obiettivo è un problema diverso (raggiungibilità) e merita un risolutore suo**, con una distanza dall'obiettivo e un ordinamento guidato da una politica. Mescolare «+3 per carta dell'obiettivo» nella stessa coda della ricerca aperta è una formulazione sbagliata, e i numeri lo dicono: 0 su 4 mani Invoked contro 4 su 4 di un solver esterno sullo stesso motore (corsia 7, §3), prima della correzione B4a.
6. **Sicurezza: l'unica garanzia forte oggi è la correttezza (ogni linea si rigioca nel motore, ogni board si misura nel motore).** La completezza vale solo «al netto delle potature», che sono una dozzina e non sono elencate. Non esistono limiti superiori ammissibili sulla misura, quindi nessun certificato «entro ε dall'ottimo». Le garanzie raggiungibili sono empiriche: convergenza misurata contro un oracolo *indipendente*, stabilità su più semi, e un audit periodico di ciò che ogni potatura perde.
7. **Generalità: il banco di 16 mani, 8 delle quali Invoked, è troppo piccolo e troppo concentrato per tarare costanti.** Serve un protocollo con deck di taratura congelati e deck freschi tenuti fuori, e il divario tra i due come metrica di sovradattamento. La corsia 7 ha già mostrato che 27 caratteristiche sovradattano su 41 deck.
8. **Ordine dei prossimi 3-6 mesi:** (a) verità e banco di generalità; (b) arena nella ricerca e nella misura; (c) sonda economica dentro la ricerca e misura a gradini come prefisso; (d) obiettivo della misura: attrito, reazioni ai breaker, kit derivato dalle linee del meta; (e) risolutore per obiettivo; (f) economia dei nodi; (g) politica imparata dall'oracolo. Dettaglio e controlli in §7.
9. **Da fermare:** tarare sul banco di 16 mani; l'oracolo come «stessa ricerca con più budget» da solo; il valutatore imparato dalle sole caratteristiche statiche della board; i giri di apprendimento che alzano i valori delle carte come unico ritorno dalla misura; le aggiunte al kit una per volta quando salta fuori una cecità; i giudizi a un solo seme. Dettaglio in §8.

---

## 1. Che problema è, formalmente

### 1.1 La formulazione
Fissato il seme, EDOPro è un simulatore **deterministico**. Nel tuo primo turno l'avversario non risponde mai (`SearchConfig::opponent` serve solo a dare alle tue carte le stesse carte avversarie del turno di prova, `search.rs:70-73`). Quindi la ricerca è:

- un **problema di pianificazione a un solo agente** su un grafo implicito, generato dal motore;
- **deterministico per seme**: l'ordine del Deck, le monete e gli scarti casuali sono fissati; la ricerca esplora *una determinizzazione*;
- con **ricompensa solo terminale**: il valore sta nella board di fine turno, letta dal turno di prova;
- con due funzioni di valore: una **stima rapida** g (microsecondi, `quick_score`) e una **misura** f (circa 0,5 s in mediana, `Prober::measure`), e un rapporto di costo tra misura e nodo di **100-300 a 1** (3-5 ms un nodo con il suo replay, 0,5-1 s una misura; §9.3).

Questa è la stessa famiglia dei giochi a un giocatore con ricompensa finale (SameGame, Morpion Solitaire, costruzione di cruciverba: il terreno di NMCS e NRPA) e della pianificazione con simulatore chiuso (Iterated Width, Best-First Width Search). Non è la famiglia degli scacchi: non c'è un avversario nel tuo turno, quindi niente alfa-beta, niente finestre, niente null move. Il rapporto di letteratura lo dice già (§3 di `ricerca-letteratura.md`) e ha ragione.

Due cose della formulazione sono particolari e decidono l'architettura:

1. **La misura costa cento volte un nodo.** Negli scacchi la valutazione costa meno di una mossa. Qui è il contrario. Per ogni board misurata la ricerca può permettersi da cento a trecento nodi. Quindi la ricerca *deve* lavorare su una stima, e la qualità della stima decide tutto. È il punto centrale di questa revisione (§2).
2. **L'incertezza non è nell'avversario ma in te.** L'informazione nascosta (ordine del Deck) è tua. Una linea trovata con un seme può dipendere da una pescata fortunata. La «sicurezza» per il proprietario include «non mostrarmi una linea che riesce una volta su cinque». Oggi ogni ricerca usa un solo seme (§4.4).

### 1.2 Cosa ottimizza davvero la ricerca di oggi
In `search.rs`:
- la priorità di una risposta in coda è la stima della miglior board raggiunta dopo di lei, meno 0,6 per ogni deviazione dal tuffo (`work`, riga 1271): è una ricerca a discrepanze limitate su un best-first, con la linea principale per prima;
- la stima è `quick_score + 3·goal_hits` (riga 711);
- `quick_score` (righe 946-1004) è **additiva**: corpi sul campo per tipo, carte coperte per tipo, carte in mano, più 5 volte la somma dei valori misurati delle singole carte;
- i valori delle carte sono misurati **una carta alla volta** su un campo vuoto (`value.rs`), poi alzati durante la ricerca dai giri di apprendimento quando una carta ferma qualcosa su una board vera (`learn_round`, righe 1847-1931);
- alla fine le 10-30 board migliori *per la stima* vengono misurate; la #1 mostrata è la migliore *misurata* tra quelle.

Quindi la ricerca ottimizza una somma di valori di carte singole. Il valore di una board di Yu-Gi-Oh! è però **interazione**: Allied Code Talker vale per il Link che punta, Aeon per i materiali, una Trappola per il mostro che Evoca, una negazione per le altre che la proteggono. L'additività è un prior accettabile per ordinare dentro la ricerca; non è accettabile come criterio di quali board misurare, perché decide cosa l'utente vede.

Tre difetti precisi della stima, dal codice:
- **Le board misurate sono scelte dalla stima.** Si misurano le 10-30 migliori per `quick_score`; con più nodi la stima mette davanti altre board e quella che misurava meglio esce dalla lista. È la causa, già scritta in A1, dei valori che *scendono* con il budget (maliss da 100% a 88%, inv-r5 da 68% a 61%). Non è un difetto di budget: è la selezione sbagliata di cosa misurare.
- **Un'anomalia piccola, da contare.** Un mostro dell'Extra Deck scoperto *senza valore misurato* vale 2,0; uno misurato debole vale 0,3 + 5 × 0,04 = 0,5 (`quick_score`, righe 956-961). `card_values` misura in anticipo tutte le carte del deck, quindi il caso tocca solo le carte la cui misura è fallita («a turn that could not be played tells nothing», `value.rs:365`) e i token. Quante siano per deck va contato con `card-values`; se sono più di qualche unità, l'anomalia pesa.
- **Inflazione monotona.** `CardValues::raise` alza e non abbassa mai (`value.rs:207`). Una carta vista forte in un contesto resta forte in ogni contesto per tutta la ricerca. Il segnale giusto sarebbe «quanto la board intera misura rispetto alla stima», con segno.
- **La stima non vede l'avversario, il Cimitero, le bandite, i materiali, i limiti già usati.** Noto e nel piano (B5a fatto in parte: `taken_from`, riga 1010).

### 1.3 L'architettura è giusta? Confronto con le alternative
Giudizio per ciascuna, con il motivo. La regola che uso: un'alternativa vale solo se tocca uno dei due colli di bottiglia veri (stima e replay) o il problema specifico dell'obiettivo.

| Alternativa | Giudizio | Perché |
|---|---|---|
| **Best-first con tuffi e discrepanze** (oggi) | **tenere** | Si adatta a profondità 20-30, ramificazione modesta, molte trasposizioni, nodo caro. Il tuffo ammortizza il replay; la discrepanza concentra il lavoro vicino alle linee buone. La corsia 2 ha misurato che l'ordine dei fratelli di oggi rende meglio dell'alternativa ovvia (B1a). Il difetto è ciò che la guida, non la forma. |
| **UCT con rollout casuali** | **no** | I rollout casuali in Yu-Gi-Oh! finiscono il turno subito o fanno giocate inutili: il segnale è nullo. D'accordo con `ricerca-letteratura.md`. |
| **NMCS / NRPA (rollout annidati con politica adattata)** | **sì, ma solo per l'obiettivo e solo con l'arena** | È ciò che usa ygo-combo-solver (README letto per intero): NRPA con livelli annidati, snapshot in arena, tabella delle trasposizioni, più Go-Explore e Levin Tree Search come «finitore». Sulle 4 mani Invoked trova l'obiettivo in 2 s dove la nostra ricerca per obiettivo non lo trovava in 60 s (corsia 7, §3; dopo B4a la nostra arriva in 1,3-29 s). NRPA impara una politica *per mano* sui codici delle mosse: è esattamente ciò che manca a una ricerca per obiettivo. Senza arena ogni rollout è un replay da 5 ms: troppo. Per la ricerca aperta (massimizzare la misura) NRPA converge a ottimi locali e non dà copertura: no. |
| **Beam search / allargamento iterativo** | **no** | Il beam pota per livello con la stima, cioè con la cosa meno affidabile che abbiamo. La profondità in «scelte» non è semantica (una selezione a 5 carte sono 5 livelli). La ricerca di oggi è già un beam morbido: le discrepanze fanno da larghezza. |
| **Macro-azioni / livello degli effetti** | **sì, come riformulazione del grafo** | Un nodo dovrebbe essere un *momento libero* (`is_free`, riga 751) e un arco una *giocata intera* (attivazione più tutte le sue sotto-scelte fino al momento libero seguente). Oggi `Pending` e `node_key` lo approssimano, e il 24% dei nodi non cambia nulla di visibile (A2). Il 96% delle risposte rigiocate viene prima dell'ultimo momento libero (B0): le istantanee ai momenti liberi coprono quasi tutto. B2b è il primo passo; la forma piena è «grafo dei momenti liberi con archi enumerati pigramente». |
| **Riduzione per ordine parziale, simmetrie** | **solo ciò che c'è, più i sleep set se gratis** | Copie, Set in un ordine, zone canoniche, selezioni come insiemi ci sono già. Resta il 9,3% dei nodi «altro ordine»: percentuali, non ordini di grandezza. DPOR vuole la profondità; qui no. |
| **Trasposizioni** | **tenere; correggere la voce; aggiungere l'audit** | La tabella c'è (`seen`, 64 shard sulla chiave canonica più la scelta e il `Pending`). Due difetti. (1) I bersagli e i nomi dichiarati non sono nella chiave: una scelta alternativa dello stesso effetto non si esplora (§30.8). (2) **La voce è «chiuso alla prima visita»** (`first_visit`, riga 1562): se un nodo è raggiunto prima da un cammino con 3 deviazioni, i suoi figli entrano in coda con penalità 4 × 0,6, e il cammino a 0 deviazioni che arriva dopo finisce come duplicato (riga 1563) senza rimettere i figli a penalità più bassa. È l'interazione tra potatura per ordine e tabella chiusa di cui avvertono Burch e Holte (corsia 10, G4.13 e G8.8), verificata nel codice. Correzione: la voce tiene la penalità minima vista, e un arrivo con penalità minore riaccoda i figli. |
| **Dominanza** (A domina B se ha più risorse) | **no come potatura, forse come ordine** | In Yu-Gi-Oh! non è monotona: una volta per turno, blocchi («non puoi Evocare Speciale per il resto del turno»), limite della mano, carte che vogliono il Cimitero vuoto. Una potatura per dominanza perde linee giuste. |
| **Valore imparato dalle caratteristiche statiche della board** | **no** | La corsia 7 l'ha provato: 27 caratteristiche con perdita a coppie peggiorano fuori campione (inversioni dal 14,7% al 15,6-16,5%). Ciò che la stima sbaglia non si legge dalla board, si legge *giocando*. Due run economiche del turno di prova (0,06 s) spostano le coppie giuste dal 72% all'80%. Il valutatore imparato va riformulato (§8). |
| **Politica imparata (ordinamento delle mosse)** | **sì, a medio termine, addestrata sull'oracolo** | È la leva lunga: se la risposta giusta è già la prima nel 77% dei passi, portarla al 90-95% rende i tuffi quasi sempre buoni e riduce i nodi alla board migliore di molto. Caratteristiche senza nomi (tipo di decisione, tipo di carta, da dove a dove, ruolo, profondità). Rischio di imparare i difetti della ricerca: per questo serve un oracolo indipendente (§3.3). |
| **Limiti superiori ammissibili e potatura** | **no** | Non esiste un limite stretto sulla misura: non è additiva (la run congiunta la riduce), e il massimo teorico («ogni carta ancora giocabile vale il massimo dei valori») è così largo da non potare mai. Serve solo per la modalità esaustiva su mani piccole, dove non serve potare. |
| **Go-Explore** (archivio di celle, ritorna e esplora) | **forse, dopo l'arena** | Richiede ritornare a uno stato a costo basso: con l'arena sì. È un modo naturale per garantire *copertura* (una board per «cella» di descrittori) e rispondere alla lamentela «linee ripetitive». Vale come esperimento, non come piano. |
| **Più obiettivi / archivio per descrittori (MAP-Elites)** | **sì, piccolo** | La top N per stima mostra board quasi uguali. Un archivio «una board per (numero di interruzioni, risorse per il turno dopo, carte usate)» costa poco e serve all'utente e ai giri di misura. |

Conclusione: **la struttura resta**. Cambiano la stima, il ritorno dalla misura, il replay, e la modalità obiettivo diventa un risolutore a parte.

---

## 2. La stima è il problema principale, e cosa farne

### 2.1 I numeri
- Spearman stima/misura 0,57; Kendall 0,42; NDCG@3 0,82 (corsia 7, 82 mani, 4490 board).
- La board migliore vera è oltre la 30ª per la stima in 10 mani su 82, oltre la 15ª in 20. L'app misura le prime 30: perde il #1 vero in 10 mani su 82, in media l'1,7% del valore, fino al 50%.
- A nodi crescenti il valore misurato della #1 **scende** in 2 mani su 8 dell'oracolo pilota. Non è un difetto della ricerca: con più nodi la stima mette davanti altre board, e quella buona esce dalle 10 misurate.
- Il fit Texel su caratteristiche statiche non migliora fuori campione. Il fit su due run economiche (B6 e w1u0, 0,06 s) sì: coppie giuste dal 72,2% al 79,6%, #1 nelle prime 15 da 368 a 399 su 480.

### 2.2 Cosa ne segue
1. **La valutazione va fatta giocando, a tre livelli di costo, e i livelli devono essere prefissi l'uno dell'altro.**
   - T0: `quick_score`, microsecondi, per ordinare la coda.
   - T1: la **sonda economica** (quick_value a due run, o 6-8 finestre per risposta, 0,02-0,2 s) per *tutte* le board candidate a fine turno, non solo per 30.
   - T2: la misura completa contro il kit (0,5 s) sulle 10-15 migliori per T1.
   - T3: il livello 2 contro i deck veri (7 s), a richiesta e per la taratura.
   La corsia 7 ha mostrato che il risparmio c'è solo se T2 riprende da dove T1 si è fermata (finestre 9-16, giri degli usi): va costruito così in `probe.rs`.
2. **T1 dentro la ricerca, al posto dei giri di apprendimento come sono.** Oggi ogni 1500 nodi si misurano 2 board e si alzano i valori delle carte. Proposta: ogni giro misura con T1 le K board nuove migliori per T0 (K = 10-20, costo 0,2-2 s) e usa due cose:
   - il **residuo con segno** (T1 − T0) per correggere la priorità delle risposte lasciate in coda da quelle linee (è B10, «epoca dei valori», ma con una misura intera, non con i valori delle carte);
   - la classifica T1 come **criterio di quali board confermare e misurare**, al posto della stima.
   I valori delle carte restano come prior di T0; il `raise` monotono va sostituito da una media con segno.
3. **Taratura di T0 su T2, non su T1, con deck tenuti fuori.** Perdita a coppie tra board della stessa mano, regolarizzata verso i pesi di oggi, 30% dei deck fuori (è B9 con il protocollo della corsia 7). Caratteristiche da aggiungere perché l'avversario, i materiali, gli effetti già usati e il Cimitero entrino nella stima: le 6 che la corsia 7 chiede al motore.
4. **Le board da misurare si scelgono con T1, non con T0.** È questo che rende il valore monotono nel budget: una board che T1 giudica buona non esce più dalla lista perché T0 ne preferisce altre. L'anomalia del 2,0 per le carte senza misura si corregge insieme (media dei misurati del suo tipo), dopo averne contato il peso.

### 2.3 Controllo
- Spearman T0/T2 sulle board finali, deck tenuti fuori: da 0,57 a 0,75 o più.
- #1 vero (misurato su tutte le board) nelle prime 15 di T1: 95% delle mani.
- Valore della #1 **mai** in calo a budget crescente sulle mani dell'oracolo (oggi 2 su 8).
- Tempo della valutazione nell'app dimezzato a parità di #1 (è l'obiettivo di B14).

---

## 3. Sicurezza: cosa si può garantire

### 3.1 Quattro significati, e quali sono raggiungibili
| Garanzia | Oggi | Raggiungibile? |
|---|---|---|
| **Correttezza** (ciò che mostro è vero: la linea si gioca, la board fa quello che dico) | sì: ogni linea è replay nel motore, ogni misura è una partita vera | sì. È il punto forte del progetto. Da aggiungere solo il controllo su più semi (§3.4). |
| **Completezza** (con budget infinito trovo ogni board distinta) | «al netto delle potature», non elencate | sì, se le potature sono elencate, spegnibili e misurate (§3.2). Il verdetto «impossibile» della modalità obiettivo vale solo così. |
| **Ottimalità entro ε** (certificato sul valore) | no | **no**, in generale: nessun limite superiore ammissibile e stretto sulla misura. Non va inseguito. |
| **Anytime e convergenza** (più budget, meglio; so quanto manca) | no: il valore può scendere con il budget | sì, empiricamente: con T1 su tutte le candidate (§2.2) il valore misurato diventa monotono per costruzione; «quanto manca» si stima con la curva contro l'oracolo e con la stabilità della top K. |
| **Robustezza al caso** (la linea riesce su più ordini del Deck) | no: un seme | sì, a costo lineare nei semi (§3.4). |

### 3.2 Le potature: elencarle, spegnerle, misurarle
Dal codice, tutto ciò che riduce lo spazio al di là della tabella delle trasposizioni esatta:
1. `AutoPolicy`: posizioni (solo Attacco), niente Battle Phase, zone canoniche, solo Trappole e Magie Rapide si Settano (`auto.rs`).
2. `max_choices` 60 per linea (`search.rs:108`).
3. Handtrap che non fa nulla: dopo 3 volte resta in mano per tutta la ricerca (`IDLE_STRIKES`, riga 1121).
4. Linee in cui una tua carta nega la tua catena: scartate, e la carta non si concatena più (`own_negators`, righe 1446-1451 e 1575-1583).
5. Bersagli, nomi dichiarati e opzioni non nella chiave del nodo (`node_key`, §30.8).
6. Set in un solo ordine (`set_key`), selezioni come insiemi (`without_returns`), niente «annulla» subito dopo aver iniziato (`undoable`).
7. Risposte rapide fuori dalla Main Phase: «passa» per prima (ordine, ma con il budget diventa potatura).
8. Copie identiche offerte una volta.
9. Nella misura: 16 finestre per risposta al massimo, 3 giri degli usi, il kit stesso.
10. La tabella delle trasposizioni chiusa alla prima visita, con priorità che dipendono dal cammino (§1.3, riga «Trasposizioni»): non perde stati, perde *priorità*, cioè ordine, che a budget finito è la stessa cosa.

Nessuna è sbagliata a priori. Tutte insieme sono la differenza tra «esaustivo» e «esaustivo nel senso nostro». Proposta:
- una struttura `Pruning` con un flag per voce, tutte vere di default, tutte false con `--exact`;
- un **audit periodico a budget fisso** (l'esaurimento non arriva nemmeno con due carte, §9.4): su 20 mani di 1-2 carte, stesso seme e stesso budget, si confrontano default e «senza la regola R», una regola alla volta: valore della migliore misurata e board distinte. Soglia: nessuna regola perde più dello 0,5% del valore su quelle mani; chi supera la soglia diventa ordine invece che potatura.
- il verdetto «impossibile» dell'obiettivo si dichiara solo in `--exact`, oppure si scrive «non trovato con le potature standard».

### 3.3 L'oracolo come è definito è circolare
L'oracolo A1 è «la stessa ricerca con 10-50 volte i nodi». Eredita la stessa stima, quindi gli stessi punti ciechi: inv-r6 ferma al 55% a ogni budget vuol dire che la ricerca normale e l'oracolo *concordano su una board sbagliata*, oppure che il 55% è il meglio raggiungibile. Non si distingue. E l'oracolo misura solo le 10 migliori per stima: la sua «board migliore» è la migliore misurata tra quelle che la stima ha scelto.

Correzioni, tutte automatiche e senza linee scritte a mano:
1. **L'oracolo misura molto di più**: T1 su tutte le board trovate, T2 sulle prime 100-200 per T1. Il riferimento diventa «la board con la misura più alta tra tutte le trovate», non «tra le 10 stimate meglio».
2. **Un secondo oracolo con un bias diverso.** Due strade, entrambe sullo stesso motore:
   - ygo-combo-solver con obiettivi *presi dalle board del nostro oracolo* (i suoi mostri dell'Extra Deck e le carte coperte), così si vede se raggiunge board che noi non raggiungiamo, e con che costo;
   - una nostra ricerca con **ripartenze casuali** (ordine dei fratelli perturbato con un seme, stima perturbata del 20%) o NRPA sopra l'arena, sulle stesse mani. Dove i due oracoli trovano board diverse con valore simile, il nostro ha un buco di copertura; dove uno trova di più, l'altro ha un bias.
3. **Il banco dell'oracolo con mani giocabili e un deck fisso committato nelle fixture** (oggi 4 mani Invoked su 8 non si giocano con la lista MDM corrente).

### 3.4 Più semi
Una linea con pescate, scavi, monete o scarti casuali va rigiocata su 8-16 ordini del Deck con `follow.rs` (le risposte riconosciute per identità, non per posizione), e si mostra la quota di semi in cui riesce. È B17: andrebbe anticipato perché tocca la sicurezza percepita, non solo la precisione. Costo: 8-16 replay per board mostrata, decimi di secondo. La probabilità si moltiplica al valore nel punteggio «Linee sicure».

---

## 4. Generalità: dove il progetto rischia di adattarsi al banco

### 4.1 I rischi concreti
1. **Il banco**: 16 mani, 8 Invoked. `VALUE_WEIGHT` 5, `DISCREPANCY` 0,6, i +10 e −10 dell'obiettivo, `LEARN_EVERY` 1500, `MEASURED_BODY` 0,3 e le regole d'ordine del tuffo sono stati scelti lì. I commenti di `search.rs` citano Invoked a ogni riga (Mechaba, Aeon, Artemis, Oceanus). I test di regressione sono 5, tutti Invoked. Non è un difetto del codice: è il segno che ogni costante è una regressione su un deck.
2. **Il kit** è una lista di carte per nome, per forza. Ma la sua *composizione* è stata corretta a posteriori ogni volta che una cecità saltava fuori (Link Spider unico mostro dell'Extra → 15 mostri generici; Impermanence e Super Polymerization aggiunte in D15). È un ciclo che non finisce e che adatta il kit ai deck che abbiamo guardato.
3. **La stima** vede handtrap per lista (`hand_traps`), Trappole e Magie Rapide per tipo, Pendulum a 0: regole per tipo di carta, generali, ma tarate sul meta di oggi.
4. **Le regole d'ordine** («non rispondere alla tua catena», «rapide in End Phase per ultime», «handtrap dopo i Set») sono regole di buon senso per i deck di oggi; con il budget diventano potature, e un deck futuro che vive di risposte alla propria catena (ce ne sono: Dragon Link con i Bystial, deck Counter Trap) le paga.
5. **La taratura su 41 deck** sovradatta già con 27 caratteristiche (corsia 7). Qualunque cosa imparata va verificata su deck mai visti.

### 4.2 Come provarla
- **Due insiemi di deck, congelati e committati per impronta** (`deck_hash` come fa l'oracolo): *taratura* (circa 30 liste MDM di oggi) e *freschi* (le 10-15 liste più nuove, più 5-10 liste TCG/OCG di archetipi usciti negli ultimi 6 mesi, che hanno già script in EDOPro prima di arrivare in Master Duel). Ogni numero del protocollo (`AGENTS.md`) si riporta su tutti e due; il **divario** è la metrica di generalità. Soglia: il divario sul valore rispetto all'oracolo non supera 5 punti percentuali.
- **Rotazione**: ogni 2-3 mesi i freschi entrano nella taratura e si prendono freschi nuovi. Così il banco segue il meta senza mani scritte a mano.
- **Sweep casuale** (`sweep.rs` c'è): 200 deck casuali dal pool, 1000 nodi, per contare linee interrotte, risposte rifiutate, azioni mai offerte. È il controllo «non si rompe su un deck che non abbiamo mai visto».
- **Nessuna costante nuova senza il test appaiato su taratura e freschi insieme**, con la regola del segno di `paired_test.py`.

### 4.3 La regola «nessun caso per nome di carta» va estesa al kit
Il kit avversario è l'unico posto dove i nomi sono inevitabili. Si può ridurre: le *azioni* del kit (sonde) restano generiche; le *carte* del turno avversario possono venire dalle linee vere dei deck del meta (§5.3). I breaker restano una lista, ma pesata dalle liste MDM (D15 lo fa già per Impermanence e Super Polymerization) invece che fissa.

---

## 5. La misura: definizione, difetti, cosa farne

### 5.1 Cosa misura oggi
Il turno di prova (`probe.rs`, `measure_from`): si rigioca il tuo turno, poi l'avversario con il kit gioca 13 sonde (Pot of Greed, Reinforcement of the Army, Thunder Dragon, Foolish Burial, Terrortop, Bystial, Glow-Up Bulb, Celtic Guardian, Link Spider, Taketomborg, Red-Eyed Dice, Dian Keto). Per ogni tua risposta offerta, una run per finestra (fino a 16) in cui la si usa, e si legge quali sonde falliscono. Poi i floodgate (sonde che falliscono senza risposte), la run congiunta, i breaker (10-12, ognuno in una run), il turno dopo. Valore = effetto × bersaglio × copertura × usi.

È un **banco di prova a unità**: ogni componente è letta dal motore, spiegabile, generale per costruzione (una sonda è un *tipo* di azione). Questa parte è giusta e va tenuta come strumento.

### 5.2 Perché non è la verità
1. **I pesi sono scritti a mano** (effetto, bersaglio, copertura, i pesi delle sonde, i pesi dei breaker, 1/0,9/0,8/0,6 tra le voci). La taratura su 200 board del banco oggettivo non li sposta fuori campione (`taratura-pesi.md`): un segno che le *componenti* non catturano ciò che la verità vede, non solo i pesi.
2. **Il kit è un avversario che non esiste.** Il livello 2 (linee vere) ordina le prime 8 in modo diverso: Spearman 0,52 in mediana (corsia 7; 39 mani, quasi sempre contro il solo Snake-Eye, quindi un dato da rifare con più avversari). Con le coppie del proprietario l'accordo è 6 su 32 (`taratura-pesi.md`). Con le coppie sicure dei tre giudici è 22 su 29, ma quasi tutte le sbagliate sono *pari*: le board differiscono in carte che contro il kit valgono zero.
3. **È deterministico**: una mano, un ordine. Il valore di una board è un'aspettativa su mani e deck avversari.
4. **Misura solo il turno 2 dell'avversario**, mentre il proprietario giudica anche il turno 3 (follow-up, Cimitero, attrito dell'Extra Deck avversario). Le cecità note (Aeon a 3 materiali, interruzioni dal Cimitero, Trappole in mano, valore del turno dopo) sono tutte di questo tipo.

### 5.3 Definizione proposta e come avvicinarsi a costo accettabile
**Definizione.** V(board) = valore atteso, sugli avversari del meta pesati per diffusione e sulle loro mani, di [quota delle loro giocate del turno 2 che fermi] + [ciò che ti resta per il turno 3 dopo il loro turno]. Il banco oggettivo (`banco-oggettivo.md`) ne calcola già la prima metà (1 − `left`).

**Tre passi, in ordine di rapporto tra resa e costo:**
1. **Le correzioni già nel piano che cambiano l'obiettivo** (B5a attrito con Extra Deck realistico, fatto in parte; B5b run «react» contro i breaker; B5c protezione e turno 3 dal Cimitero; B5d firma di costruzione). Giuste. Il controllo deve essere sulla verità (coppie del banco oggettivo e dei giudici), non solo sul caso Aeon.
2. **Un kit derivato dalle linee del meta invece che scritto a mano.** `rival_lines` esiste già per il livello 2. Proposta: una volta per aggiornamento del meta (non per ricerca) si calcolano e si salvano su disco le prime 6-8 mosse delle linee migliori dei 20 deck più diffusi; la misura T2 usa, al posto delle 13 sonde fisse, **un campione di 3-4 di queste aperture** (pesate per diffusione) più i breaker. Costo per board: simile a oggi (le aperture sono corte); il livello 2 completo resta a richiesta. Vantaggi: il kit segue il meta da solo, nessuna carta aggiunta a mano, il valore diventa un'aspettativa (campione diverso per seme), e le sonde «generiche» restano come T1 economica. Rischio: le linee avversarie vengono dalla nostra ricerca, quindi una ricerca debole dà un avversario debole. Mitigazione: le linee del meta si calcolano con budget alto, di notte, e si validano con il banco oggettivo.
3. **Il turno 3 nel valore**: dopo il turno avversario (con la sua apertura campionata), una ricerca corta (500 nodi) del tuo turno 3 dal campo e dalla mano che restano, letta con T1. È la metà mancante della definizione e la risposta a «risorse per il turno dopo». Costo: circa come una misura T2 in più; solo per le prime 3-5 board.

### 5.4 Cosa non fare nella misura
- Non aggiungere un'altra carta al kit alla prossima cecità. Chiedersi prima se la cecità è di tipo «l'avversario non fa mai questa azione» (allora è il §5.3.2) o «il motore non legge questo effetto» (allora è l'Observer).
- Non mettere il livello 2 nella cascata automatica (la corsia 7 lo mostra: 7 s a board, ordine diverso, e serve a 6 board per vedere la sua #1).
- Non tarare pesi con 200 board: la verità stessa ha rumore (coppie pari per 0,02). Prima più board e più avversari, poi la taratura.

---

## 6. Velocità: i tetti veri e le leve

### 6.1 Dove sta il tetto
- **Il motore.** Giocare le scelte (`OCG_DuelProcess`, quasi tutto Lua) è i due terzi del tempo di un replay; `vendor/` non si tocca. Una richiesta Lua va da microsecondi a secondi secondo il deck (Kewl Tune: 100 milioni di istruzioni per richiesta prima del tetto). Qui non c'è leva se non il tetto per richiesta, già fatto.
- **Il replay**: 43% della ricerca. L'arena lo toglie quasi del tutto (ripristino 0,2 ms contro 4-5 ms). Stima della corsia 2: ricerca −35/40%. Dico di più: con l'arena cambia la *forma* della ricerca (§6.2), e il guadagno è anche nei nodi.
- **La misura**: 58% del tempo totale in `combo_hunt`, e dentro la misura il 69% è il ciclo delle risposte, dove ogni run rigioca il tuo turno intero (44% di una run) più il turno avversario fino alla finestra. **È qui che l'arena vale di più**: un'istantanea dopo il tuo turno e una a ogni finestra del turno avversario trasformano «usa la risposta R alla finestra W» da «rigioca tutto» a «ripristina W, attiva R, gioca il resto». Stima: misura −50/60%.
- **I thread**: 4 core; il best-first parallelo con una coda sola scala male (+16% su 4 thread contro +29% su 1 con il riuso). Con un'arena per thread e la tabella condivisa può andare meglio, ma il tetto è 3× su 4 core.

### 6.2 Le leve, in ordine di grandezza
| Leva | Guadagno atteso | Tipo |
|---|---|---|
| Arena nella misura (istantanea dopo il tuo turno, e per finestra) | misura ÷2-2,5 → tempo totale −30/35% | ordine di grandezza, insieme alla riga sotto |
| Misura a gradini come prefisso (T1 su tutte, T2 su 10-15) | tempo della valutazione ÷2 a parità di #1 | ordine di grandezza, insieme alla riga sopra |
| Arena nella ricerca (istantanee ai momenti liberi, cache LRU per thread, ripartenza dalla più vicina) | ricerca −35/40%, e meno nodi sprecati perché i tuffi non devono più essere lunghi | grande |
| Nodi senza cambi visibili come un passo (B2b), più il grafo dei momenti liberi | nodi −15/25% | medio |
| Ordinamento imparato (history, poi politica) | nodi alla board migliore −20% subito, −50% con la politica | medio ora, grande dopo |
| Ordini scambiabili, chiave dei nodi, fratelli | ≤10% ciascuno | percentuali |
| Più thread che rendano | fino a 3× su 4 core, dopo l'arena | grande, ma è hardware |

Messe insieme, in modo realistico: tempo a parità di board migliore **÷4-6 entro sei mesi**; nulla dà 100×. L'unica cosa che può dare un altro fattore 5-10 è una politica che renda i tuffi quasi sempre giusti: allora la ricerca diventa «poche decine di ripartenze guidate», che è ciò che fa NRPA quando funziona.

### 6.3 L'arena cambia l'algoritmo, non solo il tempo
Con il ripristino a 0,2 ms:
- una coda di *stati* (istantanee) al posto di una coda di *prefissi di linea*; i tuffi lunghi, nati per ammortizzare il replay, non servono più, e si può espandere il nodo migliore senza costo di ritorno;
- i giri di misura T1 possono partire dall'istantanea della board invece che rigiocare la linea;
- le varianti di zona e le conferme costano il resto del turno, non il turno intero;
- NRPA e Go-Explore diventano possibili per l'obiettivo.
Costo: 2 MB per istantanea; una cache di 100-200 per thread sta in memoria su un PC. Da decidere la politica di sostituzione (più fratelli in coda prima, più recente poi). È la PR 2 di B13: è la cosa più importante da fare nel motore.

---

## 7. Priorità per i prossimi 3-6 mesi
In ordine. Ogni voce ha un controllo misurabile; senza il controllo la voce non si chiude. I numeri si riportano su deck di taratura **e** freschi (§4.2).

1. **Verità e banco di generalità** (2-3 settimane, script e fixture, nessun file caldo).
   - Insiemi di deck congelati per impronta; oracolo che misura T1 su tutto e T2 sulle prime 200; secondo oracolo (ygo-combo-solver con obiettivi presi dalle nostre board, o ripartenze casuali); audit delle potature a budget fisso su 20 mani piccole, una regola alla volta.
   - **Controllo:** curva valore/budget per mano su entrambi gli insiemi; valore mai in calo con il budget; elenco delle potature con il valore perso da ciascuna; divario taratura/freschi sotto 5 punti.
2. **Arena nella ricerca e nella misura** (4-6 settimane, `ygo-engine`, `search.rs`, `probe.rs`: file caldi, un agente).
   - Prima la misura (meno rischio, guadagno più grande sul tempo totale), poi la ricerca.
   - **Controllo:** board e linee identiche sul banco (`DUELFORGE_CHECK_REUSE`-style per l'arena); misura per board ÷2 in mediana; nodi al secondo ×1,5 su un thread; `combo_hunt --seed 7` 82 su 82 identiche salvo i tempi.
3. **Sonda economica dentro la ricerca, misura a gradini come prefisso, stima tarata** (3-4 settimane, `search.rs`, `probe.rs`, script).
   - T1 su tutte le candidate; residuo con segno in coda; via l'incentivo rovesciato; `raise` sostituito da una media con segno; fit di T0 su T2 con deck fuori.
   - **Controllo:** Spearman T0/T2 ≥ 0,75 sui freschi; #1 vero nelle prime 15 di T1 nel 95% delle mani; valutazione nell'app ÷2 a parità di #1; test appaiato verde su taratura e freschi.
4. **L'obiettivo della misura** (4-6 settimane, `probe.rs`, coordinatore).
   - B5a completo, B5b, B5d, poi il kit dalle aperture del meta (§5.3.2) e il turno 3 (§5.3.3) sulle prime 5.
   - **Controllo:** coppie sicure dei giudici ≥ 80% giuste; accordo con le coppie del proprietario in salita misurabile (oggi 6 su 32); Spearman tra T2 e il banco oggettivo da 0,52 a 0,7; Aeon a 3 materiali sopra Aeon a 2, in misura e in stima.
5. **Risolutore per obiettivo** (3-4 settimane, `relax.rs` nuovo, aggancio in `search.rs`).
   - Distanza rilassata sulla mappa dei collegamenti, azioni utili prima, credito AMAF, miglioramento attorno alla prima linea (B4). Dopo l'arena, NRPA come secondo motore per l'obiettivo, da confrontare.
   - **Controllo:** `invoked-goals.tsv` 4 su 4 sotto 5 s su un thread; 20 obiettivi generati dalle board dell'oracolo raggiunti al 95% del valore dell'oracolo; verdetti «impossibile» invariati in `--exact`.
6. **Economia dei nodi e correttezza della tabella** (2-3 settimane, `search.rs`): la voce della tabella con la penalità minima vista e il riaccodamento dei figli quando scende (è piccolo e va per primo); B2b; poi la chiave dei nodi con le scelte nascoste *in fondo alla coda* (riprovata con il gate su taratura e freschi); i sleep set solo se gratis e solo con la prova di commutatività a campione che propone la corsia 10.
   - **Controllo:** quota di nodi raggiunti una seconda volta con penalità minore (da contare prima: oggi ignota); nodi a parità di board migliore −25%; nessuna mano peggiore del 2% nel test appaiato.
7. **Politica imparata dall'oracolo** (6-8 settimane, modulo nuovo e script). Caratteristiche senza nomi; perdita a coppie tra la risposta della linea dell'oracolo e le altre offerte; uso come ordine del tuffo e come termine di priorità, mai come taglio.
   - **Controllo:** accuratezza top-1 ≥ 85% sui deck freschi; nodi alla board migliore −50% sul banco; test appaiato verde.
8. **Più semi per le linee con eventi casuali** (1-2 settimane, `follow.rs`, CLI, poi contratto): quota di semi in cui la linea riesce, nel punteggio e nell'app.
   - **Controllo:** su 82 mani, quante top 3 hanno una linea sotto il 70% di riuscita; dopo, nessuna mostrata senza l'avviso.

Fuori dai sei mesi: Go-Explore e archivio per descrittori; valutatore imparato riformulato (§8.3); livello 3.

---

## 8. Cosa fermare

1. **Tarare costanti sul banco di 16 mani.** Ogni costante nuova passa dal test appaiato su taratura e freschi. Il banco resta come test di regressione (uscite identiche), non come misura di qualità.
2. **L'oracolo come «stessa ricerca con più budget» da solo**, e che misura solo le 10 migliori per stima. Vedi §3.3.
3. **I giri di apprendimento che alzano i valori delle carte come unico ritorno dalla misura.** Sostituirli con T1 sulle board intere e il residuo con segno. I valori delle carte restano come prior.
4. **Il valutatore imparato come è scritto** (`valutatore-imparato.md`: rete dalle caratteristiche della board). La corsia 7 mostra che non c'è segnale sufficiente nelle caratteristiche statiche. Riformulare: la «valutazione imparata» è *T1 più una correzione imparata* (predire T2 da T1 e poche caratteristiche), e la *politica* imparata per l'ordine delle mosse. Nessuna rete sui soli stati.
5. **Aggiungere carte al kit a ogni cecità nuova.** Prima il kit derivato dalle aperture del meta (§5.3.2).
6. **Giudizi a un solo seme** per linee con pescate, scavi, monete.
7. **Il livello 2 nella cascata automatica** (resta a richiesta e per la taratura).
8. **Lavoro su B2 oltre i sleep set, su B1a e sugli ordini dei fratelli senza gate multi-seme**: percentuali, e la corsia 2 ha già visto che i cambi d'ordine oscillano con il budget.
9. **Costruire limiti superiori per potare.** Non esistono stretti.
10. **Dichiarare «impossibile» senza `--exact`.** Finché le potature non sono spegnibili, il verdetto dice «non trovato con le potature standard».

---

## 9. Prove fatte in questa sessione
Container Linux a 4 core, `main` 93d2edd, build release, dati scaricati il 22/10 (`fetch-data`, `update-cards`: database 147.53, 14 636 carte), liste MDM del 22/10. Deck Invoked: la fixture `tests/fixtures/decks/invoked-siracusa-md.ydk` (committata, quindi riproducibile). Un thread ovunque. Le uscite intere sono nello scratchpad della sessione; qui i numeri che contano.

### 9.1 Valori delle carte: il prior additivo è debole
```
duelforge-cli card-values --deck tests/fixtures/decks/invoked-siracusa-md.ydk     # 2,9 s, 104 coppie (carta, posto)
```
| Carta | Valore da sola |
|---|---|
| Invoked Caliga | 1,15 |
| Invoked Mechaba | 1,00 |
| Ash Blossom (mano) | 0,53 |
| Invoked Oceanus (Zona Extra / Principale) | 0,26 / 0,00 |
| **S:P Little Knight** | **0,09** |
| **Hyperinvoked Aeon** | **0,04** |
| Endymion, Artemis, Crowley, Virakam | 0,00 |

S:P Little Knight, una delle interruzioni più giocate del formato, vale 0,09: contro le sonde del kit (Magie dalla mano, ricerche, Foolish Burial) bandire un mostro ferma poco (copertura 13%). Nel turno di prova di una board intera (§9.3) resta 0,09, e nella run congiunta scende a 0,00. Non è un difetto di misura: è la *definizione* del kit che decide. È l'esempio più netto del §5.2.

**Carte del deck senza valore misurato**: 3 su 39 carte distinte hanno un posto misurabile e nessun valore (Kashtira Fenrir e Santa Claws in Zona Principale, Gamma Summon coperta); le altre «mancanti» sono Magie Normali che non restano sul campo, handtrap fuori dall'elenco passato e artwork alternativi. Nessun mostro dell'Extra Deck: l'anomalia del 2,0 (§1.2) qui non scatta.

### 9.2 Ricerca per obiettivo (B4a) su inv-r1
```
duelforge-cli combos --deck <fixture> --hand 'Aleister the Reminiscent;Aleister the Invoker;Invocation - "Sword";Ash Blossom & Joyous Spring;Crowley, the Gifted Magistus' \
    --goal "Invoked Oceanus@extra;S:P Little Knight" --first --seconds 90 --threads 1
```
`GoalReached after 7.8 s: 2762 nodes (352/s)`, board Oceanus [EMZ] + S:P + "Sword" coperta, 5 carte usate, 42 scelte. Conferma B4a (prima: non trovata in 60 s). Il solver esterno impiega 2 s sulla stessa mano con il solo obiettivo di base: il divario che resta è quello che B4 (distanza dall'obiettivo) e NRPA sopra l'arena devono chiudere.

### 9.3 Una mano del banco a 4000 nodi, con le statistiche della ricerca e la misura
```
DUELFORGE_SEARCH_STATS=1 duelforge-cli combos --deck <fixture> --hand '<inv-r6>' --nodes 4000 --threads 1 --top 10 --probe --quick-value
```
| Misura | Valore |
|---|---|
| nodi al secondo | 324 (4003 nodi in 12,3 s) |
| risposte rigiocate per nodo nuovo | 13,3 (53 117 su 4003) |
| nodi in discese che finiscono su una board sotto il 90% della migliore | 3654 su 4003 (**91%**); vicino alla migliore 97 (2,4%); nessuna board 252 |
| discese finite su un nodo già visto | 231 su 819 (28%) |
| nodi senza cambi visibili | 803 (20%: `Choose` 444, `SpecialSummon` 295, `Yes` 64) |
| risposta giusta prima provata, sulla linea migliore | 17 passi su 23 (74%); rango massimo 4 |
| turno di prova per board | 0,9-1,1 s; sonda economica (`quick value`) 0,0 s |
| **costo di una misura in nodi** | **circa 300** |

Tre cose da leggere qui.
- **Il 91% dei nodi va in discese che non si avvicinano alla migliore.** È la firma di una stima che non distingue: la ricerca è efficiente nel *fare* nodi, non nello *scegliere* dove farli.
- **Una misura costa 300 nodi**, non 100: il rapporto del §1.1 è anche più sfavorevole di quanto scritto nei piani. Di qui la necessità di T1 (0,02-0,06 s, cioè 6-20 nodi).
- **Le prime due board misurano uguale (2,04) e differiscono solo per l'esposizione** (0,14 contro 0,97: la #2 lascia Aleister the Reminiscent sul campo invece che in mano). Il valore «di turno 2» non le separa; un termine di turno 3 o di esposizione sì. È il §5.3.

### 9.4 Esaustività su mani piccole: non c'è
```
duelforge-cli combos --deck <fixture> --hand "Aleister the Invoker;Invocation" --nodes 300000 --seconds 900 --threads 1
```
| Mano | Dopo 900 s | Coda alla fine | Board distinte | Migliore (stima) |
|---|---|---|---|---|
| Aleister the Invoker + Invocation | 216 131 nodi (239/s), 102 453 replay, 51 403 duplicati | **99 103**, in crescita | 24 114 | 9,1 (ferma dal secondo 25) |
| + Magical Meltdown | 285 292 nodi (315/s), 96 253 replay, 46 497 duplicati | **224 101**, in crescita | 37 064 | 13,9 |

Una mano di due carte, in un deck di 40, non si esaurisce in 15 minuti: la coda cresce più in fretta di quanto si svuoti. La board migliore per la stima è trovata dopo 25 s e non cambia più nei 14 minuti seguenti: un indicatore di stabilità (§3.1) avrebbe fermato la ricerca lì. Un quarto dei nodi sono duplicati anche qui. Conseguenza per il §3.2: l'audit delle potature non può appoggiarsi all'esaurimento, nemmeno su mani piccole; va fatto **a budget fisso**, confrontando il valore della migliore e le board distinte con e senza ogni regola, su mani di 1-2 carte e con `max_choices` basso. E il verdetto «impossibile» a budget finito resta «non trovato».

### 9.5 Una mano morta esaurisce in un secondo
```
duelforge-cli combos --deck <mdm>/branded.ydk --hand 68468459;48130397;19271881;14558127;19096726 --seed 1 --nodes 3000 --threads 1 --probe
```
Cinque carte a caso (seme 1) della lista Branded: 28 nodi, esaurita in 1,1 s, migliore misurata 0,85 (Mercourier, un Branded King e Super Polymerization coperta). Lo spazio di ricerca va da «esaurito in un secondo» a «inesauribile in un quarto d'ora» secondo la mano, non secondo il numero di carte: i limiti a nodi fissi del banco (3000-4000) misurano cose diverse su mani diverse. Un budget in *tempo alla stabilità del migliore* (B16) è più sensato di un budget in nodi.

## 10. Rispetto alla revisione bibliografica della corsia 10
`revisione-bibliografica.md` (circa 150 lavori, 45 letti per intero su GitHub, nessun articolo letto per intero: lo dice in §1.4) è il riferimento per le fonti; qui non la rifaccio. Dove concordiamo e dove no.

**Concordo, e lo uso.**
- Il self-play RL è escluso; ogni parte imparata ordina, non decide. Le etichette vengono da un oracolo, con validazione su carte posteriori all'istantanea (le sue A1b.3 e §4.4 B12 sono il mio §4.2 e il mio punto 7 di §7).
- Il livello 2 come media su K mani avversarie campionate dal meta, con le stesse mani per tutte le board e una correzione sul quarto peggiore (CVaR): è la forma precisa della mia definizione del §5.3, e la prendo così com'è. Il kit derivato dalle aperture del meta (§5.3.2) è il modo di pagarla.
- L'avvertimento di Burch e Holte sulla tabella chiusa più la potatura per ordine: l'ho verificato nel codice ed è nel §3.2 e nel §7.
- I numeri di ygorl sull'arena (0,59 ms di ripristino contro 47,5 ms di replay sullo stesso core) confermano il nostro prototipo (0,2 contro 4-5 ms).
- La firma dallo script Lua come caratteristica delle carte (MirrorForce) è il modo giusto di dare alla stima le «funzioni del core che la carta chiama» senza nomi: va in B5d e B9.

**Ordino diversamente.**
- La corsia 10 mette per prime la struttura della ricerca (novità, bandit per prompt, dimezzamenti alla radice, portafoglio di politiche). Io metto per prima la **stima** (T1 dentro la ricerca) e la **selezione di cosa misurare**: i numeri del §9.3 dicono che il 91% dei nodi va dove la stima manda, e nessuna disciplina di coda cambia dove si guarda se la bussola è la stessa. Novità e bandit sono percentuali sopra una stima giusta; sopra una sbagliata ordinano meglio le linee sbagliate.
- Il portafoglio di politiche di risposta (Prismata, PGS) è utile come **semina** dei tuffi e come secondo oracolo (§3.3), non come sostituto della ricerca: le combinazioni di politiche generiche raggiungono le board «ovvie», e le linee che ci mancano sono quelle non ovvie.
- Il nodo afterstate per il caso (B17) è giusto in teoria; a 300 nodi al secondo lo terrei fuori dalla ricerca e dentro la *verifica* delle linee finali su più semi (§3.4). Dentro la ricerca costa un fattore n su ogni evento casuale.

**Non farei ora.**
- Gumbel e i dimezzamenti alla radice: vogliono un priore per azione, che senza nomi è debole; la nostra prima scelta è già quasi sempre giusta (74-77%). Rinviare a dopo la politica imparata.
- La ricerca a fascio di SabberStone come modalità: è il best-first con un'altra coda; il problema resta cosa c'è nella coda.

## 11. Fonti, e quanto le ho lette
Per la bibliografia completa vale `revisione-bibliografica.md` (corsia 10). Qui solo ciò che ho letto io in questa sessione, con il segno: **intero** = codice o README letto per intero; **abstract** = solo abstract o pagina di riepilogo; **memoria** = conoscenza pregressa, non riverificata in questa sessione. I siti degli articoli sono bloccati dal proxy del container.

- Codice di DuelForge: `search.rs`, `value.rs`, `probe.rs` (prime 2100 righe), `matchup.rs` e `follow.rs` (intestazioni), `auto.rs`, `duel.rs` (`advance`): **intero** per le parti citate.
- [ygo-combo-solver, README](https://github.com/Armytille/ygo-combo-solver) (Armytille, AGPL): **intero**. NRPA/GNRPA a livelli annidati, snapshot in arena, tabella delle trasposizioni condivisa, zone canoniche, «74,6% dei nodi senza scelta» giocati in linea, Go-Explore e Levin Tree Search come finitore, obiettivi con `--resolve` e `--summon-min`; «le board difficili non si convertono a ogni corsa».
- [ygorl, docs/solver.md](https://github.com/n0tevi1/ygorl) (**intero**): usa ygo-combo-solver come pianificatore per mano e fonte di dimostrazioni; ogni linea rigiocata da zero nel core come verifica; 28 mani su 50 risolte a 20 s; obiettivi «bloccanti» generati dai fatti degli script.
- Orseau, Lelis et al., *Single-Agent Policy Tree Search With Guarantees* (NeurIPS 2018) e *Policy-Guided Heuristic Search with Guarantees* (AAAI 2021): **abstract**. Il limite sul numero di espansioni vale per il costo minimo e dipende dalla qualità della politica; vale per l'obiettivo, non per la ricerca aperta.
- Rosin, *Nested Rollout Policy Adaptation* (IJCAI 2011): **abstract**; Cazenave, NMCS (IJCAI 2009) e *Beam NRPA*: **memoria**. NRPA adatta una politica per mossa ai migliori rollout annidati; converge a ottimi locali.
- Harvey & Ginsberg, *Limited Discrepancy Search* (1995); Korf, ILDS: **memoria**. È la forma della nostra priorità.
- Lipovetzky & Geffner, Iterated Width e Best-First Width Search: **memoria**. Novità come chiave secondaria, per simulatori chiusi.
- Karnin et al., *Almost optimal exploration in multi-armed bandits* (successive halving, 2013); Jamieson & Talwalkar; Hyperband: **memoria**. Base della misura a gradini.
- Ecoffet et al., *Go-Explore* (2019/2021): **memoria**. Archivio di celle e ritorno a basso costo: richiede l'arena.
- Churchill & Buro, *Hierarchical Portfolio Search* (Prismata): **memoria**. Portafoglio di politiche per turni a molte azioni.
- Stockfish: history heuristic, correction history, SPRT di fishtest; Texel tuning: **memoria**.
- `ricerca-letteratura.md`, `ricerca-esperimenti.md` e `revisione-bibliografica.md` del repository: **intero** le prime due; della terza il sommario, i gruppi 2.4 e 2.8, le raccomandazioni (§4) e le lacune (§5). Le conclusioni che ho ribaltato o ristretto sono dette nel testo (§1.3: valore statico imparato; §3.3: oracolo; §5.4: livello 2 e kit; §10: ordine delle priorità).

## 12. Cose che non ho potuto verificare in questa sessione
- Non ho rifatto i numeri della corsia 7 né della corsia 2: li uso come sono, con la loro data e versione.
- Il costo di un'istantanea per finestra nel turno di prova è una stima dal prototipo dell'arena (0,2 ms per 2 MB), non una misura nella misura.
- L'accordo tra il kit derivato dalle aperture del meta e il banco oggettivo è una previsione, non un dato: va misurato prima di sostituire le sonde.
