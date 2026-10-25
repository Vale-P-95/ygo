# Linee per l'interazione a fine turno, senza turno avversario

Richiesta di Valerio del 25/10/2026: un'opzione del Combo Lab che, partendo da una carta o da una mano, trovi le linee che lasciano a fine turno più interazione su campo, mano e Cimitero, senza giocare il turno dell'avversario. Per interazione si intende tutto ciò che si può attivare nel turno avversario o in risposta ai suoi effetti, più gli effetti permanenti: negazioni da ogni zona, effetti rapidi, scarti, distruzioni, bandi, rimbalzi in mano e nel Deck.
Questo documento è solo analisi e progetto: nessun codice. Dipende dal lettore degli effetti (`docs/plans/anatomia-effetti.md`, brief A, PR #181).

**Revisione del 25/10, dopo la prima versione.** Valerio ha escluso ogni avversario, anche passivo o fatto di stimoli singoli: l'analisi si basa solo sugli effetti delle sue carte. Le ragioni sono due. Se un effetto è un'interruzione lo dice la carta, non serve provarlo contro qualcuno. E il conteggio non deve dipendere da che cosa gioca l'avversario: un avversario che non fa scattare un effetto lo farebbe sparire dal conteggio. La misura è quindi solo **lettura** e **verifica a secco** sulla propria board (§3).

## Decisione proposta
- **Nessun avversario, in nessuna forma.** Un effetto conta se è attivabile o applicabile nel turno avversario. Le attivazioni del tipo «quando l'avversario attiva» o «quando l'avversario Evoca» contano, *assumendo che il loro innesco sia possibile*.
- **Si verifica solo ciò che dipende dalla tua board**: costi pagabili, materiali da staccare, limiti per nome e per duello, carte negate o coperte, condizioni sul tuo campo. Lo si fa chiamando dagli script Lua le funzioni di condizione, costo e bersaglio in modalità di controllo (`chk == 0`), che non cambiano nulla. Ogni domanda sul lato avversario o sull'evento che innesca riceve la risposta «sì» (§3.2).
- **Il numero che si massimizza è la capacità d'interruzione**: quante interruzioni separate puoi usare nello stesso turno avversario, con i limiti condivisi e le carte che i costi si contendono. Accanto ci sono il profilo per tipo e per zona e gli effetti permanenti.
- **La ricerca cerca quel numero, non lo riordina dopo.** Nella modalità «interazione» la stima delle board di fine turno è la capacità stessa, perché la verifica a secco costa pochi millisecondi. Le finaliste si mostrano diverse tra loro (fronte di Pareto).
- **Da una carta, la risposta è una distribuzione**: su mani campionate con la carta, la capacità media e la probabilità di avere almeno 1, 2, 3… interruzioni, con intervalli di confidenza, e il contributo della carta stimato con mani gemelle.
- **La modalità normale non cambia** (classe E sul banco congelato). La modalità nuova si giudica sul suo numero con le regole del banco e con una verità etichettata da Valerio.

## 1. Che cosa c'è oggi
- **Il turno di prova** (`crates/ygo-solver/src/probe.rs`) gioca un avversario generico, il kit (`Kit::standard`, `probe.rs:367`), con 13 tipi di mossa (`enum Probe`, `probe.rs:41`), e riconosce le interruzioni dagli esiti (`enum Effect`, `probe.rs:139`). Resta la misura della modalità normale; **questa modalità non lo usa**, per le due ragioni di Valerio. In più non vede la battaglia, gli scarti dalla mano avversaria e le rimozioni che chiedono un bersaglio quando il kit non ne ha.
- **Il profilo «Interazione a fine turno»** (PR #171, `docs/plans/interazione-fine-turno.md`, `interaction.rs`) ordina le board per `campo + mano + floodgate` del turno di prova, *dopo* una ricerca che ha seguito la stima normale. Una linea che tiene un handtrap in mano invece di usarlo come materiale può non arrivare tra le finaliste. Con `--card` non c'è misura.
- **La ricerca** (`search.rs`) è un best-first con tuffi fino a fine turno e 0,6 di penalità per deviazione. La stima `quick_score` (`search.rs:951`) non sa che cosa farà un effetto finché non lo gioca.
- **Il motore sa già eseguire Lua a metà duello**: `Duel::run_lua_source` (`crates/ygo-engine/src/duel.rs:927`) chiama `OCG_LoadScript` sullo stato del duello, ed è usato per piazzare le carte. `Replacement` (`scripts.rs:25`) carica funzioni nostre dopo gli script di base, senza toccare `vendor/`. Il brief A usa lo stesso meccanismo per leggere gli effetti.
- **Il lettore degli effetti** (brief A, in corso) darà per ogni effetto tipo, evento, zona, limite, proprietà, categorie, presenza di costo, condizione e bersaglio, e il collegamento attivazione → effetto dalla descrizione.

**Che cosa manca, quindi:** una misura dell'interazione fatta solo sulle tue carte, che copra tutte le categorie di Valerio e conti ciò che si può usare *insieme*; una ricerca che la insegua; una risposta statistica per la carta singola.

## 2. Che cosa si conta
### 2.1 Definizione operativa
Un'**interruzione** è un effetto di una tua carta, nella sua zona a fine turno, che:
1. si può usare nel turno avversario: effetto rapido, Trappola o Magia Rapida coperta, effetto innescato da un'azione dell'avversario; oppure è permanente: effetto continuo o di campo che limita l'avversario;
2. agisce sull'avversario o sulle sue azioni, in una delle classi della tabella;
3. supera la verifica a secco della tua board (§3.2).

| Classe di Valerio | Categorie Lua (lettore A) |
|---|---|
| Negare un'attivazione o un effetto, da campo, mano o Cimitero | `NEGATE`, `DISABLE` |
| Negare un'Evocazione | `DISABLE_SUMMON` |
| Bandire | `REMOVE` |
| Distruggere («spaccare») | `DESTROY` |
| Rimbalzare in mano | `TOHAND`, se il bersaglio è sul lato avversario |
| Rimbalzare nel Deck | `TODECK` |
| Far scartare | `HANDES` |
| Mandare al Cimitero | `TOGRAVE` |
| Prendere il controllo, cambiare posizione | `CONTROL`, `POSITION` |
| Effetti permanenti (floodgate) | effetto continuo o di campo con bersagli sul lato avversario e un codice di divieto (non Evocare, non attivare, effetti negati…) |
| Effetti rapidi senza categoria verso l'avversario (Evocare in risposta, proteggere) | `SPECIAL_SUMMON` rapido, protezioni: contano a parte, come «risposte» |

Se il bersaglio di un `TOHAND` o `TODECK` sia l'avversario lo dice la dichiarazione dell'effetto (`SetOperationInfo` con il giocatore, §3.2), o la ricerca dei bersagli sul lato avversario.

Le zone sono quattro: campo (mostri e Magie/Trappole, coperte comprese), mano, Cimitero, bandite. La mano conta solo per le carte che si attivano da lì; le altre carte tenute in mano sono risorse, e si mostrano a parte.

### 2.2 La capacità: contare ciò che si usa insieme
Sommare gli effetti esagera. Due copie di un handtrap con «once per turn» per nome danno un'interruzione sola per turno; due effetti che scartano come costo la stessa ultima carta della mano ne danno una; un effetto della stessa carta con «you can only use 1 effect of … per turn» esclude gli altri.

Sia E l'insieme delle interruzioni che superano la verifica a secco. Per ogni e:
- q(e), la qualità: 1 per il conteggio semplice, oppure la scala della categoria (le stesse costanti di `Effect` in `probe.rs`, da 0,4 a 1,0: si riusano i numeri, non il turno di prova);
- u(e), gli usi per turno, dal limite dello script (1 se c'è un limite per turno, 2 per «twice per turn», di più senza limite ma con un costo che li esaurisce);
- g(e), il gruppo di limite con capienza κ(g): il codice del limite letto dallo script unisce le copie e gli effetti con lo stesso nome;
- il fabbisogno di risorse: quante carte k(e) dall'insieme S(e) consuma il costo (carte da scartare, materiali da staccare, carte da bandire dal Cimitero, la carta stessa se lascia la zona), letto dalla verifica a secco (§3.2).

La **capacità** è

C = max Σ q(e)·x(e), con x(e) intero, 0 ≤ x(e) ≤ u(e), Σ_{e∈g} x(e) ≤ κ(g) per ogni gruppo, e un'assegnazione delle risorse in cui ogni carta paga al più un costo e ogni uso di e riceve k(e) carte distinte da S(e).

- È un piccolo programma intero: un *b-matching* tra usi e carte con i vincoli di gruppo. Le dimensioni sono minuscole (di solito meno di 15 effetti e 10 carte di costo), quindi si risolve esattamente per enumerazione o branch-and-bound in microsecondi; con un solo uso per effetto è un flusso massimo.
- Due numeri da mostrare: **C** con q = 1 (quante interruzioni separate, la domanda di Valerio) e **C_q** con la scala di qualità (una negazione vale più di un cambio di posizione).
- Restano fuori dal programma, e si dichiarano come limiti: costi che si ricaricano nel turno avversario (un effetto che rimette materiali), e catene in cui un tuo effetto rende possibile un altro.

### 2.3 Il profilo
Per ogni board, oltre a C e C_q: le interruzioni per classe (§2.1) e per zona; i floodgate e che cosa vietano; le risposte senza categoria; le **assunzioni** di ogni effetto («se l'avversario attiva un effetto di mostro», «se l'avversario controlla un mostro», §3.2); quante interruzioni restano se spariscono tutti i tuoi mostri, se sparisce il tuo Cimitero, se la prima viene negata (sottrazioni sui posti, calcolate sull'inventario: nessun avversario); le carte rimaste in mano e nel Cimitero; la lunghezza della linea.

## 3. La misura: lettura e verifica a secco
### 3.1 S0, lettura (microsecondi)
Dall'anatomia del brief A: ogni effetto di una tua carta la cui zona d'uso (`range`) contiene la zona della carta a fine turno, di tipo utilizzabile nel turno avversario (rapido, innescato da un evento dell'avversario, attivazione di una Trappola o Magia Rapida coperta, continuo o di campo), con almeno una categoria o un codice di divieto del §2.1.
- **I limiti per turno ripartono al cambio di turno**: un effetto usato nella combo resta disponibile nel turno avversario. Restano consumati solo i limiti per duello (`EFFECT_COUNT_CODE_DUEL`) e i limiti scritti come flag con una durata che arriva al turno dopo, che si leggono nella verifica a secco.
- Le Trappole coperte in questo turno si possono attivare nel turno avversario; le Magie Rapide coperte anche.
- S0 è il candidato. Basta da solo come stima ottimistica nei nodi interni della ricerca (§4.2), mai come numero finale.

### 3.2 S1, verifica a secco sulla tua board (millisecondi)
Gli script di EDOPro scrivono condizione, costo e bersaglio come funzioni Lua. Costo e bersaglio hanno un parametro `chk`: chiamate con `chk == 0` rispondono soltanto «si può fare?» e per convenzione degli script non cambiano lo stato del duello. È lo stesso controllo che il motore fa prima di offrire un'attivazione. La verifica a secco chiama queste tre funzioni per ogni effetto candidato, a fine turno, con le domande divise in due:

| Domanda | Esempio negli script | Risposta |
|---|---|---|
| Sulla tua board | carte da scartare (`IsExistingMatchingCard(Card.IsDiscardable, tp, LOCATION_HAND, 0, 1, …)`), materiali (`CheckRemoveOverlayCard`), carte da bandire dal Cimitero, Life Point (`CheckLPCost`), «se controlli un mostro Fusione», flag dei limiti (`GetFlagEffect(tp, id)`), carta negata (`IsDisabled`) o coperta | **vera**, letta dallo stato del duello |
| Sul lato avversario | «bersaglia 1 carta che l'avversario controlla» (`IsExistingTarget(…, tp, 0, LOCATION_ONFIELD, 1, …)`), «se l'avversario ha carte in mano» | **sì per ipotesi**, registrata come assunzione |
| Sull'evento che innesca | `rp == 1-tp`, `re:IsActiveType(TYPE_MONSTER)`, `Duel.IsChainNegatable(ev)`, il gruppo `eg` delle carte evocate | **sì per ipotesi**, registrata come assunzione |
| Sul turno | `Duel.GetTurnPlayer() ~= tp`, la fase | il turno è quello avversario: fa parte dello scenario, non di una mossa |

**Come si fa senza toccare `vendor/`.** Con una `Replacement` caricata dopo gli script di base e con `run_lua_source` a fine turno:
1. Il lettore del brief A, avvolgendo `Card.RegisterEffect`, tiene in una tabella Lua il riferimento a ogni effetto registrato con la sua carta. A fine turno lo script di verifica scorre le tue carte e i loro effetti candidati (S0).
2. Durante la verifica, le funzioni di `Duel` che contano o cercano carte (`IsExistingMatchingCard`, `IsExistingTarget`, `GetMatchingGroup`, `GetMatchingGroupCount`, `GetFieldGroupCount`, `GetLocationCount`…) sono avvolte. Ogni chiamata ha un raggio per te e uno per l'avversario. La parte tua si calcola davvero; se la domanda tocca il lato avversario, la risposta è «sì», cioè abbastanza carte, e l'effetto prende l'assunzione corrispondente. Con raggio misto (tuo e avversario) basta la parte avversaria per dire sì, e si registra.
3. I parametri dell'evento (`eg`, `ep`, `ev`, `re`, `r`, `rp`) sono oggetti segnaposto: `rp` vale l'avversario, `re` è una tabella che risponde «sì» a ogni domanda sul tipo dell'effetto attivato, `eg` è un gruppo segnaposto. Le funzioni della catena (`IsChainNegatable`, `GetChainInfo`, `GetCurrentChain`) sono avvolte allo stesso modo. Ogni accesso si registra: dice a che cosa risponde l'effetto.
4. **I costi rivelano le risorse.** Quando il controllo del costo chiama `IsExistingMatchingCard(f, tp, LOCATION_HAND, 0, k, ex)`, il segnaposto calcola anche l'insieme S(e) delle tue carte che soddisfano f e la quantità k. Sono il fabbisogno del §2.2, letto dallo script e non da una lista.
5. Le funzioni Lua dei controlli girano su una copia del duello (`Copies`, come la ricerca), così un costo scritto male, che cambia qualcosa anche con `chk == 0`, non tocca la board vera. Il risultato è per effetto: **attivabile**, **attivabile con assunzioni** (con l'elenco), **non attivabile** con il motivo (costo non pagabile, materiali assenti, limite per duello usato, carta negata, condizione falsa sulla tua board).
6. **I controlli del motore in C++** (zona giusta, carta scoperta, divieti sul giocatore come `EFFECT_CANNOT_ACTIVATE` lasciati dalle tue carte con durata fino al turno dopo) si leggono con le funzioni che li espongono a Lua: `Duel.IsPlayerAffectedByEffect` per i divieti, e, se il submodule ce l'ha, `Effect.IsActivatable` con le opzioni per ignorare la catena e i bersagli. Va controllato nel brief I1; se manca, si controllano a parte zona, stato della carta e divieti.

**Che cosa non si può sapere senza avversario, e va bene così:** se il bersaglio ci sarà, se l'avversario farà l'azione che innesca, se un'altra carta avversaria impedirà l'attivazione. Sono proprio le cose che Valerio vuole lasciare fuori. Diventano assunzioni scritte accanto all'effetto, non sconti sul numero.

**Limiti noti della verifica a secco.**
- Alcune condizioni miste («se l'avversario non controlla mostri») diventano «sì» per ipotesi anche quando sono rare: si mostrano come assunzioni, e una colonna conta le interruzioni senza assunzioni sul lato avversario.
- Script che con `chk == 0` fanno qualcosa di diverso dalla convenzione: si contano nel `summary.txt` del brief I1 (risultato diverso tra copia e originale, errori Lua) e si mostrano come «da verificare».
- Le restrizioni registrate dentro l'operazione di un effetto si leggono solo se sono già sul giocatore a fine turno, cioè se la linea le ha davvero create: è il caso giusto.

### 3.3 Dove si usa che cosa
| Dove | Livello | Perché |
|---|---|---|
| Nodi interni della ricerca (ordine dei tuffi) | S0 | microsecondi; ottimistico, solo per ordinare |
| Fine di ogni tuffo (board di fine turno) | S0 + S1 + capacità | pochi millisecondi, sotto il costo di un replay medio (5,6 ms, `velocita-ricerca.md`) |
| Finaliste mostrate | lo stesso, con il profilo completo | il numero che vede Valerio è quello che ha guidato la ricerca |

Il numero non dipende da un seme né da un avversario: la stessa board dà sempre la stessa capacità.

## 4. La ricerca
### 4.1 Che problema è
Senza un obiettivo fisso e con un numero da massimizzare, è **pianificazione con obiettivi in eccesso** (*oversubscription* o *partial satisfaction planning*): ogni interruzione è un obiettivo morbido con un'utilità, non si possono avere tutti (le risorse della mano sono poche), e si cerca il piano con la somma massima (Smith 2004; Benton, Do e Kambhampati 2009). Gli obiettivi morbidi si possono trasformare in obiettivi rigidi con azioni di rinuncia (Keyder e Geffner 2009), ma qui non serve: il nostro best-first è già *anytime* e confronta board finite.
Due cose lo distinguono dalla ricerca di oggi:
- il valore di una board non cresce con la linea. Usare un handtrap come materiale toglie un'interruzione dalla mano per metterne forse una sul campo: la ricerca deve poter non giocare;
- l'utilità è un vettore (quante, di che tipo, da dove), e Valerio può volere board diverse con numeri simili.

### 4.2 Che cosa cambia nella modalità «interazione»
1. **Stima delle board.** Alla fine di ogni tuffo, al posto di `quick_score`: C_q (S0 + S1) + un termine piccolo per le risorse (carte in mano e nel Cimitero che non interrompono, peso 0,05 da tarare) + i floodgate. Il resto della ricerca (coda, tuffi, penalità, duplicati) resta com'è.
2. **Ordine delle mosse.** «Tieni l'handtrap» c'è già nell'ordine dei tuffi (`dive_order`); con S0 la regola diventa generale: una carta in mano con un'interruzione da mano si gioca per ultima nel tuffo. Il priore del brief C dell'anatomia (provare prima le attivazioni con categorie verso l'avversario) aiuta anche qui.
3. **Nessuna taratura durante la ricerca.** La prima versione tarava una stima rapida contro il turno di prova; ora la stima *è* la misura, quindi `learn` non serve in questa modalità.
4. **Finaliste diverse.** Le board candidate vanno in un archivio a celle (stile MAP-Elites, Mouret e Clune 2015): una cella per (negazioni, rimozioni, interruzioni fuori dal campo), e si tiene la migliore per cella. Si mostrano le migliori K celle, non le K board più alte, che sono spesso varianti della stessa.
5. **Classifica finale.** Ordine lessicografico: C, poi C_q, poi le interruzioni senza assunzioni sul lato avversario, poi quelle fuori dal campo (resistono a una pulizia del campo), poi i floodgate, poi le risorse, poi la linea più corta. Accanto, il **fronte di Pareto** su (C, negazioni, interruzioni fuori dal campo): le board non dominate, che sono le scelte vere (Mandow e Pérez de la Cruz 2010 per la ricerca multi-obiettivo; qui il fronte si calcola solo sulle finaliste, quindi basta un filtro).

### 4.3 Che cosa non fare
- Nessuna potatura con S0: ordina e stima, non scarta.
- Nessun bonus per nome, nessuna linea precalcolata.
- La misura normale (`Measure`, `quick_score` della modalità solita, turno di prova) non si tocca.

## 5. Da una carta: la statistica
### 5.1 Che cosa si stima
La carta c e il deck D (40 carte o più). Andando per primi la mano è H = {c} ∪ R, con R di 4 carte estratte senza reimmissione da D meno una copia di c (ipergeometrica multivariata). Per ogni mano, J(H) = la capacità C della board migliore trovata con budget N. Si vuole la distribuzione di J:
- la media E[J] con il suo intervallo;
- la curva p_k = P(J ≥ k) per k = 1…5, la più leggibile («con questa carta, 7 volte su 10 chiudi con almeno 2 interruzioni»);
- il contributo della carta Δ(c) = E[J(c + R)] − E[J(x + R)], dove x è un'altra carta del deck estratta a caso (o una carta inerte).

J dipende dal budget e dal seme della ricerca: si dichiarano nel risultato. Con un thread e un budget di nodi la ricerca è deterministica (`regole-motore.md` §3), e la misura non ha caso: la sola variabilità è quella delle mani.

### 5.2 Stimatori e intervalli
- **Media:** media campionaria con intervallo bootstrap percentile (10 000 ricampionamenti, seme fisso; Efron e Tibshirani 1993). J è intero e limitato: la t di Student va bene con n ≥ 30, il bootstrap regge anche le code.
- **p_k:** intervallo di Wilson (Wilson 1927), migliore del normale vicino a 0 e 1 e meno conservativo di Clopper-Pearson (Clopper e Pearson 1934; il confronto è in Brown, Cai e DasGupta 2001).
- **Quante mani.** Con p ≈ 0,5 il semi-intervallo di Wilson al 95% è circa 0,98/√n: ±0,10 con 96 mani, ±0,07 con 196. Per la media, con deviazione standard di J intorno a 1,2 (da misurare), ±0,25 servono (1,96·1,2/0,25)² ≈ 89 mani.
- **Arresto sequenziale.** Si gioca a blocchi di 16 mani e ci si ferma quando il semi-intervallo della media scende sotto ε (0,25 di default) o finisce il tempo. Per fermarsi senza perdere la copertura dell'intervallo si usa il limite di Bernstein empirico (Maurer e Pontil 2009; Mnih, Szepesvári e Audibert 2008), valido per variabili limitate qualunque sia la distribuzione.

### 5.3 Ridurre la varianza
- **Mani gemelle per il contributo.** Per ogni R estratto si giocano c + R e x + R con lo stesso seme: è il metodo dei numeri casuali comuni (Glasserman 2004, cap. 4; Owen 2013). Se la correlazione tra le due mani è ρ, la varianza della differenza è 2σ²(1 − ρ) invece di 2σ²: con ρ = 0,6 servono 2,5 volte meno mani. Il contributo si riporta con il suo intervallo bootstrap sulle coppie.
- **Mani uguali si giocano una volta.** Le mani si riducono al multinsieme canonico delle carte (copie e artwork unite, `canonical_code`); i doppioni prendono il risultato in memoria con il loro peso.
- **Stratificazione.** Le mani si dividono per quante carte del deck «avviano» una linea (dalla mappa dei collegamenti, `links.rs`, o dal numero di collegamenti in uscita): la frequenza di ogni strato si calcola esattamente con l'ipergeometrica (`ygo-stats`), e i campioni si allocano con la regola di Neyman (proporzionali a frequenza × deviazione standard dello strato). Riduce la varianza quando gli strati differiscono molto, il caso tipico (mani con o senza un avviatore).
- **Due fedeltà.** Tutte le mani con un budget piccolo (ad esempio 1000 nodi), una parte con il budget pieno; la media si corregge con la variabile di controllo: Ĵ = media_alta + β·(media_bassa_tutte − media_bassa_sottoinsieme), β stimato sulle coppie. È il Monte Carlo a più livelli nella sua forma più semplice (Giles 2008). Conviene solo se la correlazione tra i due budget è alta: va misurata prima (brief I3).
- **Una verifica esatta.** La parte che viene dalla sola mano ha una forma chiusa: P(almeno un'interruzione da mano in R) dall'ipergeometrica, con le carte che S0 + S1 riconoscono come interruzioni dalla mano. È un limite inferiore di p_1 per le linee che non giocano quelle carte: se la stima è sotto, la ricerca sta usando male gli handtrap.

### 5.4 Costo
Sul contenitore cloud una mano costa in mediana 16 s a 4000 nodi su un thread (`regole-motore.md` §5); la verifica a secco alla fine dei tuffi aggiunge pochi millisecondi per tuffo, da misurare nel brief I2. 96 mani sono circa 26 minuti su un thread, 7 su 4 core; con le mani gemelle il doppio. È un calcolo da lanciare in background, con `out/<nome>/summary.txt`. Nell'app serve una barra di avanzamento e il risultato parziale a ogni blocco.

### 5.5 Più carte (estensione)
Per «quale carta del deck aggiunge più interazione» si ripete §5.3 su ogni carta: è il valore di Shapley della carta nel gioco «mano → capacità» ristretto alle coppie con una sola carta scambiata (Shapley 1953; stima per campionamento di Castro, Gómez e Tejada 2009). Costa una corsa per carta: solo su richiesta.

## 6. Validazione
### 6.1 La verità di Valerio
Nessuna lettura automatica sa da sola che cosa conta un giocatore come interruzione. Serve un piccolo insieme etichettato:
- 30 board di fine turno (6 deck × 5, scelti tra quelli del banco congelato con interazione diversa), stampate con la linea e le carte per zona;
- per ognuna Valerio scrive quante interruzioni separate conta e quali (carta e classe del §2.1). Circa 20 minuti; un file TSV come `owner-verdicts.tsv`.

Con 30 board e circa 4 interruzioni l'una ci sono circa 120 effetti etichettati: il richiamo di una misura vicino a 0,95 si stima con ±0,04 (Wilson). Metriche, fissate prima:
- per effetto, **richiamo e precisione** di S0 e di S0 + S1 rispetto alle etichette;
- per board, l'errore assoluto medio di C rispetto al numero di Valerio, e l'accordo sulla classe (kappa di Cohen, Cohen 1960);
- per coppie di board dello stesso deck, l'ordine giusto, con il test di McNemar esatto quando si confrontano due versioni della misura (McNemar 1947).

Obiettivo per l'uscita della modalità: richiamo di S0 + S1 ≥ 0,95, precisione ≥ 0,9, errore medio di C ≤ 0,5 interruzioni.

### 6.2 Controlli automatici, senza etichette
Su tutte le board di fine turno del banco congelato (decine di migliaia di effetti):
- **copertura del lettore**: quota di effetti candidati che la verifica a secco riesce a valutare senza errori Lua (obiettivo ≥ 95%, come il criterio del brief A);
- **motivi di esclusione**: quanti effetti cadono per costo, materiali, limite, negazione, condizione. Un motivo che cresce di colpo dopo un cambio del lettore è un errore del lettore;
- **coerenza con il motore**: la verifica a secco deve dire «non attivabile» quando il motore stesso, nel tuo turno, non offre l'effetto per un motivo che dipende solo da te (costo, limite). Si confronta sulle attivazioni che la ricerca vede offerte o non offerte, senza nessun avversario;
- **assunzioni**: la distribuzione delle assunzioni per classe, per vedere quali condizioni il lettore lascia aperte più spesso.

### 6.3 Come si accetta ogni PR
- **La modalità normale.** Ogni PR del motore di questo piano lascia uguale la modalità solita: classe E, `banco.py same` sul set `full`, tutte le 193 uscite uguali (`regole-motore.md` §2). Nota: dal 24/10 la prova equa sulle 406 board è sostituita dal banco congelato, perché non si può rifare; `paired_test.py` resta per gli studi su mani scelte.
- **La modalità «interazione».** Il suo numero è C. Il confronto è *ricerca guidata* (questo piano) contro *riordino dopo la ricerca* (PR #171), stesse 193 mani, stesso seme e budget, con le board migliori dei due lati rimisurate con S0 + S1: sono le regole R del banco sulla scala di C (non inferiorità con bootstrap per deck, test del segno, grandi perdite, deck freschi). Serve un'opzione `--metric interaction` in `banco.py` e `paired_test.py`. Si dice «migliora» solo con il limite basso dell'intervallo sopra 0.
- **La misura S0 + S1** si giudica sulla verità del §6.1. Il turno di prova non cambia.

## 7. Letteratura
Citate a memoria e non ricontrollate online in questa sessione: le referenze sono classiche, ma anno e sede vanno verificati prima di citarle in un documento di misura (come in `ricerca-letteratura.md` §8).

| Tema | Fonte | Che cosa si prende |
|---|---|---|
| Obiettivi in eccesso | D. E. Smith, «Choosing objectives in over-subscription planning», ICAPS 2004; J. Benton, M. B. Do, S. Kambhampati, «Anytime heuristic search for partial satisfaction planning», *Artificial Intelligence* 173, 2009 | il problema (§4.1): massimizzare l'utilità degli obiettivi raggiunti, ricerca anytime |
| Obiettivi morbidi | E. Keyder, H. Geffner, «Soft goals can be compiled away», *JAIR* 36, 2009 | perché non serve trasformarli (§4.1) |
| Euristiche rilassate | J. Hoffmann, B. Nebel, «The FF planning system», *JAIR* 14, 2001; B. Bonet, H. Geffner, «Planning as heuristic search», *AIJ* 129, 2001 | già in `ricerca-letteratura.md` §2: B4 aiuta anche questa modalità |
| Valutazione parziale | N. D. Jones, C. K. Gomard, P. Sestoft, *Partial Evaluation and Automatic Program Generation*, 1993; P. Cousot, R. Cousot, «Abstract interpretation», POPL 1977 | la verifica a secco: le domande sulla tua board si calcolano, quelle sull'avversario e sull'evento restano aperte e si rispondono «sì» (§3.2) |
| Diversità delle finaliste | J.-B. Mouret, J. Clune, «Illuminating search spaces by mapping elites», arXiv:1504.04909, 2015 | l'archivio a celle (§4.2) |
| Multi-obiettivo | L. Mandow, J. L. Pérez de la Cruz, «Multiobjective A* search with consistent heuristics», *JACM* 57(5), 2010 | il fronte di Pareto (§4.2) |
| Capacità | A. Schrijver, *Combinatorial Optimization*, Springer 2003 (b-matching, flussi) | la capacità come programma intero piccolo (§2.2) |
| Intervalli per proporzioni | E. B. Wilson, *JASA* 22, 1927; C. J. Clopper, E. S. Pearson, *Biometrika* 26, 1934; L. D. Brown, T. T. Cai, A. DasGupta, «Interval estimation for a binomial proportion», *Statistical Science* 16, 2001 | p_k (§5.2), richiamo (§6.1) |
| Bootstrap | B. Efron, R. Tibshirani, *An Introduction to the Bootstrap*, 1993 | media e contributo (§5.2), come nel banco |
| Arresto sequenziale | A. Maurer, M. Pontil, «Empirical Bernstein bounds and sample variance penalization», COLT 2009; V. Mnih, C. Szepesvári, J.-Y. Audibert, «Empirical Bernstein stopping», ICML 2008 | quando smettere di campionare (§5.2) |
| Riduzione della varianza | P. Glasserman, *Monte Carlo Methods in Financial Engineering*, 2004, cap. 4; A. B. Owen, *Monte Carlo theory, methods and examples*, 2013; M. B. Giles, «Multilevel Monte Carlo path simulation», *Operations Research* 56(3), 2008 | mani gemelle, stratificazione, due fedeltà (§5.3) |
| Contributo di una carta | L. S. Shapley, 1953; J. Castro, D. Gómez, J. Tejada, «Polynomial calculation of the Shapley value based on sampling», *Computers & OR* 36(5), 2009 | §5.5 |
| Accordo e coppie | J. Cohen, *Educ. Psych. Meas.* 20, 1960; Q. McNemar, *Psychometrika* 12, 1947 | §6.1 |
| Giochi di carte | C. D. Ward, P. I. Cowling, «Monte Carlo search applied to card selection in Magic: The Gathering», IEEE CIG 2009 | precedente di Monte Carlo sulle mani per scegliere carte |

## 8. Coordinamento con il lettore degli effetti
S0 e S1 reggono sul brief A di `anatomia-effetti.md`. Campi e appigli che servono, oltre a quelli già nel piano:
1. **il codice del limite** (`GetCountLimit` con il suo codice), per unire copie ed effetti con lo stesso nome nel gruppo g(e) e distinguere «per turno» da «per duello»;
2. **il raggio dei bersagli degli effetti di campo** (il lato avversario o il tuo), per riconoscere i floodgate: va verificato se il submodule ha un getter (`GetTargetRange` o equivalente); se manca, lo dice la verifica a secco dalle funzioni di bersaglio;
3. **le categorie `HANDES`, `TODECK`, `TOHAND`, `CONTROL`, `POSITION`** nella stessa maschera;
4. **l'evento dei trigger** (codice `EVENT_*`), per sapere a che cosa risponde l'effetto;
5. **il riferimento all'effetto registrato**, tenuto in una tabella Lua dal wrapper di `Card.RegisterEffect`, perché la verifica a secco possa chiamarne condizione, costo e bersaglio a fine turno;
6. **il collegamento attivazione → effetto** già previsto.

Il 25/10 il thread del lettore ha confermato che i punti 1-4 arrivano dal brief A (nel JSON: `countLimit` con count, code, flag e index; `targetRange` [tuo lato, avversario]; tutte le categorie per nome; code ed evento dei trigger). Il punto 5 no: il wrapper del brief A si carica solo nei duelli di lettura, quindi la tabella dei riferimenti, caricata anche nei duelli giocati, la fa il brief I1 (tocca `duel.rs`).

## 9. Brief
Ordine: dopo il brief A dell'anatomia, I1 → I2; I3 può andare in parallelo a I2 (file diversi); I4 dopo il mockup e dopo I2.

### Brief I1: inventario, verifica a secco e capacità (motore, classe E)
```
Compito: duelforge-cli combos --interaction e probe-line stampano per ogni board la capacità C e C_q (§2.2), il profilo con le assunzioni (§2.3) e per ogni effetto candidato l'esito della verifica a secco (attivabile, con assunzioni, non attivabile e perché, §3.2), senza nessun avversario; la modalità normale non cambia.
Perché: docs/plans/linee-interazione.md §2-§3.
Branch: claude/interazione-inventario, da main dopo il brief A dell'anatomia
File da toccare: crates/ygo-engine/src/scripts.rs (Replacement con la tabella degli effetti registrati e i segnaposto di Duel e dell'evento), crates/ygo-engine/src/duel.rs (caricarla nei duelli giocati, solo nella modalità), crates/ygo-engine/src/anatomy.rs, crates/ygo-solver/src/inventory.rs (nuovo: S0, S1 e capacità), interaction.rs, crates/ygo-cli; test con fixture.
Da leggere: linee-interazione.md §2-§3, §6.2; anatomia-effetti.md §2.1. Prima cosa: verificare nel submodule Effect.IsActivatable, Duel.IsPlayerAffectedByEffect e un getter del raggio dei bersagli.
Verifica: cargo test -p ygo-engine -p ygo-solver -p ygo-cli; banco.py same --set full (193 uscite uguali); out/interazione/summary.txt con copertura, motivi di esclusione e coerenza con il motore (§6.2) sulle board del banco; il foglio delle 30 board per Valerio (§6.1).
Criterio di accettazione: banco ACCEPTED (E); copertura ≥ 95%; due copie di un effetto con limite per nome contano 1; un Xyz senza materiali non conta l'effetto che li stacca; un handtrap che si scarta da solo conta 1; nessuna differenza di stato tra la copia e il duello dopo la verifica.
PR: template; summary.txt per intero; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief I2: ricerca guidata dall'interazione (motore, classe E per la modalità normale)
```
Compito: combos --objective interaction cerca con C_q (S0 + S1) alla fine di ogni tuffo e S0 nell'ordine delle mosse (§4.2), tiene le finaliste in un archivio a celle e stampa la classifica lessicografica e il fronte di Pareto; banco.py e paired_test.py hanno --metric interaction.
Perché: docs/plans/linee-interazione.md §4 e §6.3.
Branch: claude/interazione-ricerca, da main dopo I1
File da toccare: crates/ygo-solver/src/search.rs (file caldo, chiederlo in TASKS.md), inventory.rs, interaction.rs, crates/ygo-cli, scripts/banco.py, scripts/paired_test.py.
Da leggere: linee-interazione.md §4, §6.3; regole-motore.md §2-§3.
Verifica: banco.py same --set full (modalità normale); banco.py run/judge --metric interaction, ricerca guidata contro riordino della PR #171, 193 mani, in background; search_regression; costo della verifica a secco per tuffo.
Criterio di accettazione: modalità normale uguale (E); sulla metrica C le regole R del banco ACCEPTED contro il riordino; «migliora» solo con il limite basso sopra 0; la verifica a secco sotto il 20% del tempo della ricerca.
PR: template; i due summary.txt per intero; mani che perdono oltre il 5% con il perché.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief I3: da una carta, la distribuzione (CLI e statistica)
```
Compito: combos --card <carta> --interaction stima su mani campionate E[J], p_k = P(J ≥ k) per k = 1..5 e il contributo Δ(c) con mani gemelle (§5), con intervalli bootstrap e di Wilson, arresto sequenziale e mani uguali giocate una volta; JSON e out/<nome>/summary.txt.
Perché: docs/plans/linee-interazione.md §5.
Branch: claude/interazione-carta, da main dopo I1 (in parallelo a I2: fino ad allora la board di ogni mano si sceglie con il riordino per C)
File da toccare: crates/ygo-cli/src/main.rs, crates/ygo-stats (intervalli, stratificazione), test.
Da leggere: linee-interazione.md §5; interazione-fine-turno.md («--card»).
Verifica: cargo test -p ygo-stats -p ygo-cli; copertura degli intervalli su 200 ripetizioni con dati simulati (il 95% deve contenere il valore vero entro ±2 punti); su 3 deck del banco con una carta ciascuno, in background; correlazione tra 1000 e 4000 nodi misurata e scritta nel piano, per decidere le due fedeltà.
Criterio di accettazione: intervalli che coprono come dichiarato; riduzione della varianza delle mani gemelle misurata e riportata; nessuna modifica del motore (banco.py same se tocca ygo-solver).
PR: template; summary.txt dei tre deck.
Modello: Sonnet 5.5, effort medio (Opus se servono cambi nella ricerca).
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief I4: la modalità nel Combo Lab (interfaccia)
```
Compito: nel Combo Lab l'opzione «Cerca linee per l'interazione» con partenza da una mano o da una carta; per la mano, le board con C, profilo per tipo e zona, assunzioni e fronte di Pareto; per la carta, la curva P(almeno k interruzioni) con gli intervalli, il contributo della carta e le board migliori.
Perché: docs/plans/linee-interazione.md §2.3, §4.2, §5.
Branch: claude/interazione-lab, da main dopo I2 e I3
File da toccare: apps/desktop/src-tauri/src/lib.rs e src/lib/api.ts (file caldi), src/lib/mock.ts, src/lib/fixtures/ (risposte registrate dalla CLI), src/pages/Combo.tsx, componenti nuovi.
Da leggere: linee-interazione.md §2.3, §5; mockup di Claude Design (da fare prima: artboard 1280×800, due temi, mano e carta).
Verifica: pnpm typecheck; pnpm build; VITE_MOCK=1 + node scripts/screens.mjs sul Combo Lab nei due temi; app vera con DUELFORGE_DATA (caricamento lungo con avanzamento e annullamento).
Criterio di accettazione: protocollo dell'interfaccia di AGENTS.md; il calcolo da una carta mostra il risultato parziale a ogni blocco e si annulla cambiando deck.
PR: template; screenshot nei due temi, anteprima Artifact.
Modello: Sonnet 5.5, effort medio.
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## 10. Rischi
- **Gli script non sempre seguono la convenzione di `chk == 0`.** La verifica gira su una copia e controlla che lo stato non cambi; gli script anomali si contano e si mostrano come «da verificare».
- **Le assunzioni possono essere larghe.** «Sì per ipotesi» a ogni domanda sul lato avversario fa contare anche effetti che chiedono situazioni rare. È la scelta di Valerio; per questo il profilo mostra le assunzioni e una colonna senza assunzioni sul lato avversario.
- **Contare non è valutare.** Tre cambi di posizione non valgono una negazione: per questo ci sono C_q accanto a C e il fronte di Pareto. La scelta finale resta di chi guarda.
- **Il costo da una carta** (decine di minuti) va gestito come calcolo lungo: in background nella CLI, con avanzamento nell'app.
- **Dipendenze:** I1 aspetta il brief A dell'anatomia e tocca il suo `anatomy.rs`; I2 tocca un file caldo (`search.rs`): uno per volta, chiesto in `TASKS.md`.
