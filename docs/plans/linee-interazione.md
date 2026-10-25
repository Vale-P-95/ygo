# Linee per l'interazione a fine turno, senza turno avversario

Richiesta di Valerio del 25/10/2026: un'opzione del Combo Lab che, partendo da una carta o da una mano, trovi le linee che lasciano a fine turno più interazione su campo, mano e Cimitero, senza giocare il turno dell'avversario. Per interazione si intende tutto ciò che si può attivare nel turno avversario o in risposta ai suoi effetti, più gli effetti permanenti: negazioni da ogni zona, effetti rapidi, scarti, distruzioni, bandi, rimbalzi in mano e nel Deck.
Questo documento è solo analisi e progetto: nessun codice. Dipende dal lettore degli effetti (`docs/plans/anatomia-effetti.md`, brief A, PR #181).

## Decisione proposta
- **La misura non gioca un turno avversario.** Niente deck avversario, niente strategia, niente mosse in sequenza. Si chiede al motore, dalla board di fine turno, *che cosa potresti attivare*: prima leggendo gli effetti (inventario), poi aprendo il turno avversario a vuoto davanti a un manichino con dei bersagli, poi con stimoli singoli, uno per volta, ognuno dalla stessa copia della board.
- **Il numero che si massimizza è la capacità d'interruzione**: quante interruzioni separate puoi usare nello stesso turno avversario, tenendo conto dei limiti per turno condivisi, dei costi in comune e degli usi. Accanto, il profilo per tipo e per zona e gli effetti permanenti.
- **La ricerca cerca quel numero, non lo riordina dopo.** Nella modalità «interazione» la stima della ricerca è l'inventario pesato per l'affidabilità di ogni classe di effetto, tarata sui dati del turno di prova; le finaliste si verificano con il motore e si mostrano diverse tra loro (fronte di Pareto).
- **Da una carta, la risposta è una distribuzione**: su mani campionate con la carta, la capacità media, la probabilità di avere almeno 1, 2, 3… interruzioni, con intervalli di confidenza, e il contributo della carta stimato con mani gemelle.
- **La modalità normale non cambia** (classe E sul banco congelato). La modalità nuova si giudica sul suo numero con le regole del banco e con una verità etichettata da Valerio.

## 1. Che cosa c'è oggi
- **Il turno di prova** (`crates/ygo-solver/src/probe.rs`) gioca un avversario generico, il kit (`Kit::standard`, `probe.rs:367`), con 13 tipi di mossa (`enum Probe`, `probe.rs:41`): attivare una Magia, cercare, effetti da mano, campo e Cimitero, Evocazioni Normali, Speciali, dall'Extra Deck e dal Deck. Le interruzioni si riconoscono dagli esiti (`enum Effect`, `probe.rs:139`: nega, bandisce, rimanda nel Deck, rimbalza, distrugge, manda al Cimitero, blocca, cambia posizione), con valori da 0,4 a 1,0.
  - Ogni risposta offerta si prova da sola (`alone`) e insieme alle altre, fino a 3 usi e 16 finestre (`Parts::ALL`). `Measure.offered` elenca le carte che il motore ti ha lasciato usare nel turno avversario.
  - È una lettura affidabile ma legata al kit: vede solo ciò che le 13 mosse provocano. Mancano la battaglia (dichiarazione d'attacco), gli scarti dalla mano avversaria e le rimozioni che chiedono un bersaglio, quando il kit non ha carte sul campo o in mano da colpire.
- **Il profilo «Interazione a fine turno»** (PR #171, `docs/plans/interazione-fine-turno.md`, `interaction.rs`): le board si ordinano per `campo + mano + floodgate` *dopo* la ricerca, che ha seguito la stima normale. Una linea che tiene un handtrap in mano invece di usarlo come materiale può non arrivare mai tra le finaliste. Con `--card` non c'è misura, quindi niente classifica.
- **La ricerca** (`search.rs`) è un best-first con tuffi fino a fine turno e 0,6 di penalità per deviazione. La stima `quick_score` (`search.rs:951`) pesa mostri, Magie e Trappole coperte, mano e valori delle carte; non sa che cosa farà un effetto finché non lo gioca. Ogni 1500 nodi (`LEARN_EVERY`, `search.rs:2082`) misura alcune board con il turno di prova.
- **Gli handtrap** si riconoscono già dal comportamento, senza nomi (`hand_traps`, `probe.rs:1225`): le carte che il motore offre dalla mano mentre il kit gioca contro una board vuota.
- **Il lettore degli effetti** (brief A, in corso) darà per ogni effetto tipo, evento, zona, limite, proprietà, categorie, presenza di costo, condizione e bersaglio, e il collegamento attivazione → effetto dalla descrizione.

**Che cosa manca, quindi:** una misura dell'interazione che non dipenda dalle mosse di un avversario, che copra tutte le categorie di Valerio, che conti ciò che si può usare *insieme*; una ricerca che la insegua; una risposta statistica per la carta singola.

## 2. Che cosa si conta
### 2.1 Definizione operativa
Un'**interruzione** è un effetto di una tua carta, in una zona a fine turno, che nel turno avversario:
1. si può attivare (effetto rapido, Trappola o Magia Rapida coperta, innescato da un evento dell'avversario), oppure è permanente (effetto continuo o di campo che limita l'avversario);
2. agisce sull'avversario o sulle sue azioni, in una di queste classi.

| Classe di Valerio | Categorie Lua (lettore A) | Esito osservato (`Effect`) |
|---|---|---|
| Negare un'attivazione o un effetto, da campo, mano o Cimitero | `NEGATE`, `DISABLE` | `NegateActivation`, `NegateEffect`, `NegateAndRemove` |
| Negare un'Evocazione | `DISABLE_SUMMON` | `NegateSummon` |
| Bandire | `REMOVE` | `Banish` |
| Distruggere («spaccare») | `DESTROY` | `Destroy` |
| Rimbalzare in mano | `TOHAND` sulle carte avversarie | `Bounce` |
| Rimbalzare nel Deck | `TODECK` | `ToDeck` |
| Far scartare | `HANDES` | manca: va aggiunto (`Discard`) |
| Mandare al Cimitero | `TOGRAVE` | `ToGrave` |
| Prendere il controllo, cambiare posizione | `CONTROL`, `POSITION` | `Position`; il controllo vale come distruzione (B5a) |
| Effetti permanenti (floodgate) | effetti di campo con bersaglio sul lato avversario | floodgate del turno di prova |
| Effetti rapidi senza categoria verso l'avversario (Evocare in risposta, proteggere) | `SPECIAL_SUMMON` rapido, protezioni | `offered` senza esito: contano a parte, come «risposte» |

Le zone sono quattro: campo (mostri e Magie/Trappole, coperte comprese), mano, Cimitero, bandite. La mano conta solo per le carte che si attivano da lì; le altre carte tenute in mano sono risorse, e si mostrano a parte.

### 2.2 La capacità: contare ciò che si usa insieme
Sommare gli effetti esagera. Due copie di un handtrap con «once per turn» per nome danno un'interruzione sola per turno; due effetti che scartano come costo la stessa ultima carta della mano ne danno una; un effetto della stessa carta con «you can only use 1 effect of … per turn» esclude gli altri.

Sia E l'insieme delle interruzioni. Per ogni e: la qualità q(e) (la scala di `Effect`, oppure 1 per il conteggio semplice), gli usi u(e), il gruppo di limite g(e) con capienza κ(g) (dal codice del limite letto dallo script, che unisce le copie e gli effetti con lo stesso nome), e le risorse che consuma a(e, r) (carte della mano da scartare, materiali da staccare, la carta stessa se lascia la zona). La **capacità** è

C = max Σ q(e)·x(e), con 0 ≤ x(e) ≤ u(e), Σ_{e∈g} x(e) ≤ κ(g) per ogni gruppo, Σ_e a(e, r)·x(e) ≤ 1 per ogni risorsa r, x intero.

- È un piccolo programma lineare intero (un *b-matching* con gruppi). Le dimensioni sono minuscole (di solito meno di 15 effetti), quindi si risolve esattamente per enumerazione o branch-and-bound in microsecondi; con risorse unitarie e q = 1 è un flusso massimo.
- Il turno di prova sa già scoprire i conflitti di risorse dai fatti (`alone` contro «insieme», il caso Mechaba): nel livello 2 i vincoli si leggono dagli esiti, nell'inventario dalla struttura (gruppo di limite, «ha un costo», zona d'origine).
- Due numeri da mostrare: **C** con q = 1 (quante interruzioni separate, la domanda di Valerio) e **C_q** con la scala di qualità (una negazione vale più di un cambio di posizione).

### 2.3 Il profilo
Per ogni board, oltre a C e C_q: le interruzioni per classe (§2.1) e per zona; i floodgate (quanti tipi di mossa avversaria bloccano); le risposte senza esito; quante interruzioni restano se l'avversario toglie tutti i tuoi mostri, se bandisce il tuo Cimitero, se nega la prima (calcolate sull'inventario, senza giocare nulla: sono sottrazioni sui posti, non un avversario); le carte rimaste in mano e nel Cimitero; la lunghezza della linea.

## 3. La misura in tre livelli, senza avversario
Perché non basta leggere gli script: la condizione e il costo di un effetto sono funzioni Lua opache (il lettore A dice se ci sono, non che cosa chiedono). «Quando l'avversario attiva un effetto» o «se un mostro avversario è sul campo» si sanno solo chiedendo al motore in quella situazione. Per questo la misura sale per livelli, dal più economico al più fedele, e ognuno risponde alla stessa domanda: *che cosa potresti attivare*.

### L0. Inventario (lettura, microsecondi)
Dallo stato di fine turno e dall'anatomia: ogni effetto di una tua carta la cui zona d'uso (`range`) contiene la zona della carta, il cui tipo si usa nel turno avversario (rapido, innescato da un evento dell'avversario, attivazione di una Trappola o Magia Rapida coperta, continuo o di campo con bersaglio sull'avversario), con almeno una categoria del §2.1.
- Si tolgono gli effetti con un limite «per duello» già usato. I limiti per turno ripartono al cambio di turno, quindi un effetto usato nella combo resta disponibile nel turno avversario: è un punto che il conteggio deve rispettare.
- Ogni effetto riceve la sua **classe di anatomia**: (tipo, famiglia di evento, zona, bersaglio sì/no, costo sì/no, condizione sì/no). L'affidabilità ρ(classe) è la probabilità che un effetto di quella classe sia davvero attivabile al livello 2 (§6.2).
- Il valore stimato è Ĉ = C calcolata con q(e)·ρ(classe(e)). È il numero che la ricerca usa a ogni nodo.

### L1. Turno avversario a vuoto, davanti a un manichino (millisecondi)
Dalla copia della board di fine turno il motore passa al turno avversario. L'avversario è un **manichino**: non fa nulla, passa la priorità a ogni finestra, ma ha i bersagli che servono agli effetti per essere attivabili: un mostro scoperto, una Magia/Trappola coperta, tre carte in mano, due nel Cimitero, l'Extra Deck del kit di B5a.
- Si registrano tutte le attivazioni che il motore ti offre, nelle finestre di Draw, Standby, Main, Battle ed End. Sono gli effetti «liberi»: rimozioni rapide con bersaglio, Trappole che si attivano senza un evento, Evocazioni rapide.
- Non è un turno giocato: l'avversario non prende decisioni. È lo stesso principio di `hand_traps`, con dei bersagli in più.

### L2. Stimoli singoli (decimi di secondo)
Dalla stessa copia, uno stimolo per volta, ognuno da solo: il manichino attiva una Magia, un effetto dalla mano, dal campo, dal Cimitero; evoca normalmente, specialmente, dall'Extra Deck; manda una carta al Cimitero; aggiunge una carta alla mano; dichiara un attacco. Si registra a quali stimoli ogni tuo effetto risponde e con quale esito.
- Sono le mosse del kit di oggi (13), più tre: **dichiarazione d'attacco**, **carta aggiunta alla mano** (per gli effetti che rispondono alle ricerche), **bersagli presenti** (il manichino del L1, che oggi il kit non ha).
- La differenza con il turno di prova: niente sequenze, niente strategia, nessun peso di frequenza nel numero principale. Ogni stimolo è un «se l'avversario facesse questo, potresti rispondere?». I pesi del kit restano solo per la copertura (C_q, profilo).
- Da L2 si ottengono gli usi u(e), gli esiti per il profilo e i conflitti di risorse (provando insieme le coppie che condividono un costo, come fa oggi `alone`).

### Quale livello dove
| Dove | Livello | Perché |
|---|---|---|
| Ogni nodo della ricerca | L0 | microsecondi; l'affidabilità corregge le classi incerte |
| Ogni 1500 nodi (`learn`), le migliori board non misurate | L1 + L2 ridotto (stimoli più pesati, un uso) | taratura di ρ durante la ricerca |
| Le finaliste mostrate | L1 + L2 completo | il numero che vede Valerio |
| Lettura statica pura (opzione `--static`) | solo L0 | se si vuole zero simulazione: è un limite superiore, dichiarato come tale |

## 4. La ricerca
### 4.1 Che problema è
Senza un obiettivo fisso e con un numero da massimizzare, è **pianificazione con obiettivi in eccesso** (*oversubscription* o *partial satisfaction planning*): ogni interruzione è un obiettivo morbido con un'utilità, non si possono avere tutti (le risorse della mano sono poche), e si cerca il piano con la somma massima (Smith 2004; Benton, Do e Kambhampati 2009). Gli obiettivi morbidi si possono anche trasformare in obiettivi rigidi con azioni di rinuncia (Keyder e Geffner 2009), ma qui non serve: il nostro best-first è già *anytime* e confronta board finite.
Due cose lo distinguono dalla ricerca di oggi:
- il valore di una board non cresce con la linea. Usare un handtrap come materiale toglie un'interruzione dalla mano per metterne forse una sul campo: la ricerca deve poter non giocare;
- l'utilità è un vettore (quante, di che tipo, da dove), e Valerio può volere board diverse con numeri simili.

### 4.2 Che cosa cambia nella modalità «interazione»
1. **Stima delle board.** Alla fine di ogni tuffo, al posto di `quick_score`: Ĉ (L0) + un termine piccolo per le risorse (carte in mano e nel Cimitero che non interrompono, peso 0,05 da tarare) + un termine per i floodgate dell'inventario. Il resto della ricerca (coda, tuffi, penalità, duplicati) resta com'è.
2. **Ordine delle mosse.** «Tieni l'handtrap» c'è già nell'ordine dei tuffi (`dive_order`); con l'inventario la regola diventa generale: una carta in mano con un'interruzione da mano si gioca per ultima nel tuffo. Il priore del brief C dell'anatomia (provare prima le attivazioni con categorie verso l'avversario) aiuta anche qui.
3. **Taratura durante la ricerca.** A ogni `learn` le board misurate con L1+L2 aggiornano ρ per classe (§6.2), così una classe che il L0 sopravvaluta in questo deck scende subito.
4. **Finaliste diverse.** Le board candidate vanno in un archivio a celle (stile MAP-Elites, Mouret e Clune 2015): una cella per (negazioni, rimozioni, interruzioni fuori dal campo), e si tiene la migliore per cella. Si misurano con L1+L2 le migliori K celle, non le K board più alte, che sono spesso varianti della stessa.
5. **Classifica finale.** Ordine lessicografico: C, poi C_q, poi le interruzioni fuori dal campo (resistono a una pulizia del campo), poi i floodgate, poi le risorse, poi la linea più corta. Accanto, il **fronte di Pareto** su (C, negazioni, interruzioni fuori dal campo): le board non dominate, che sono le scelte vere (Mandow e Pérez de la Cruz 2010 per la ricerca multi-obiettivo; qui il fronte si calcola solo sulle finaliste, quindi basta un filtro, non NAMOA*).

### 4.3 Che cosa non fare
- Nessuna potatura con L0: l'inventario ordina e stima, non scarta. Un effetto che il L0 non riconosce resta trovabile.
- Nessun bonus per nome, nessuna linea precalcolata.
- La misura normale (`Measure`, `quick_score` della modalità solita) non si tocca.

## 5. Da una carta: la statistica
### 5.1 Che cosa si stima
La carta c e il deck D (40 carte o più). Andando per primi la mano è H = {c} ∪ R, con R di 4 carte estratte senza reimmissione da D meno una copia di c (ipergeometrica multivariata). Per ogni mano, J(H) = la capacità C della board migliore trovata con budget N. Si vuole la distribuzione di J:
- la media E[J] con il suo intervallo;
- la curva p_k = P(J ≥ k) per k = 1…5, la più leggibile («con questa carta, 7 volte su 10 chiudi con almeno 2 interruzioni»);
- il contributo della carta Δ(c) = E[J(c + R)] − E[J(x + R)], dove x è un'altra carta del deck estratta a caso (o una carta inerte).

J dipende dal budget e dal seme: si dichiarano nel risultato. Con un thread e un budget di nodi la ricerca è deterministica (`regole-motore.md` §3), quindi la sola variabilità è quella delle mani.

### 5.2 Stimatori e intervalli
- **Media:** media campionaria con intervallo bootstrap percentile (10 000 ricampionamenti, seme fisso; Efron e Tibshirani 1993). J è intero e limitato, la t di Student va bene con n ≥ 30, il bootstrap regge anche le code.
- **p_k:** intervallo di Wilson (Wilson 1927), migliore del normale vicino a 0 e 1 e meno conservativo di Clopper-Pearson (Clopper e Pearson 1934; il confronto è in Brown, Cai e DasGupta 2001).
- **Quante mani.** Con p ≈ 0,5 il semi-intervallo di Wilson al 95% è circa 0,98/√n: ±0,10 con 96 mani, ±0,07 con 196. Per la media, con deviazione standard di J intorno a 1,2 (da misurare sul banco), ±0,25 servono (1,96·1,2/0,25)² ≈ 89 mani.
- **Arresto sequenziale.** Si gioca a blocchi di 16 mani e ci si ferma quando il semi-intervallo della media scende sotto ε (0,25 di default) o finisce il tempo. Per fermarsi senza perdere la copertura dell'intervallo si usa il limite di Bernstein empirico (Maurer e Pontil 2009; Mnih, Szepesvári e Audibert 2008), valido per variabili limitate qualunque sia la distribuzione.

### 5.3 Ridurre la varianza
- **Mani gemelle per il contributo.** Per ogni R estratto si giocano c + R e x + R con lo stesso seme: è il metodo dei numeri casuali comuni (Glasserman 2004, cap. 4; Owen 2013). Se la correlazione tra le due mani è ρ, la varianza della differenza è 2σ²(1 − ρ) invece di 2σ²: con ρ = 0,6 servono 2,5 volte meno mani. Il contributo si riporta con il suo intervallo bootstrap sulle coppie.
- **Mani uguali si giocano una volta.** Le mani si riducono al multinsieme canonico delle carte (copie e artwork unite, `canonical_code`); i doppioni prendono il risultato in memoria con il loro peso.
- **Stratificazione.** Le mani si dividono per quante carte del deck «avviano» una linea (dalla mappa dei collegamenti, `links.rs`, o dal numero di collegamenti in uscita): la frequenza di ogni strato si calcola esattamente con l'ipergeometrica (`ygo-stats`), e i campioni si allocano con la regola di Neyman (proporzionali a frequenza × deviazione standard dello strato). Riduce la varianza quando gli strati differiscono molto, il caso tipico (mani con o senza un avviatore).
- **Due fedeltà.** Tutte le mani con un budget piccolo (ad esempio 1000 nodi), una parte con il budget pieno; la media si corregge con la variabile di controllo: Ĵ = media_alta + β·(media_bassa_tutte − media_bassa_sottoinsieme), β stimato sulle coppie. È il Monte Carlo a più livelli nella sua forma più semplice (Giles 2008). Conviene solo se la correlazione tra i due budget è alta: va misurata prima (§9, brief I3).
- **Una verifica esatta.** La parte che viene dalla sola mano (handtrap non usati nella linea) ha una forma chiusa: P(almeno un'interruzione da mano in R) dall'ipergeometrica con le carte che il L0 classifica come interruzioni da mano. È un limite inferiore di p_1 da confrontare con la stima: se la stima è sotto, la ricerca sta usando male gli handtrap.

### 5.4 Costo
Sul contenitore cloud una mano costa in mediana 16 s a 4000 nodi su un thread (`regole-motore.md` §5). 96 mani sono circa 26 minuti su un thread, 7 su 4 core; con le mani gemelle il doppio. È un calcolo da lanciare in background, con `out/<nome>/summary.txt`. Nell'app serve una barra di avanzamento e il risultato parziale a ogni blocco.

### 5.5 Più carte (estensione)
Per «quale carta del deck aggiunge più interazione» si ripete §5.3 su ogni carta: è il valore di Shapley della carta nel gioco «mano → capacità» ristretto alle coppie con una sola carta scambiata (Shapley 1953; stima per campionamento di Castro, Gómez e Tejada 2009). Costa una corsa per carta: solo su richiesta.

## 6. Validazione
### 6.1 La verità di Valerio
Nessuna misura automatica sa da sola che cosa conta un giocatore come interruzione. Serve un piccolo insieme etichettato:
- 30 board di fine turno (5 deck × 6, scelti tra quelli del banco congelato con interazione diversa), stampate con la linea e le carte per zona;
- per ognuna Valerio scrive quante interruzioni separate conta e quali (carta e classe del §2.1). Circa 20 minuti; un file TSV come `owner-verdicts.tsv`.

Con 30 board e circa 4 interruzioni l'una ci sono circa 120 effetti etichettati: il richiamo di una misura vicino a 0,95 si stima con ±0,04 (Wilson). Metriche, fissate prima:
- per effetto, **richiamo e precisione** di L0, L1+L2 e del turno di prova di oggi rispetto alle etichette;
- per board, l'errore assoluto medio di C rispetto al numero di Valerio, e l'accordo sulla classe (kappa di Cohen, Cohen 1960);
- per coppie di board dello stesso deck, l'ordine giusto (come la verità v1 del banco, `regole-motore.md` §4), con il test di McNemar esatto quando si confrontano due misure (McNemar 1947).

Obiettivo per l'uscita della modalità: richiamo di L1+L2 ≥ 0,9 e errore medio di C ≤ 0,5 interruzioni. Se il turno di prova di oggi fa meglio di L1+L2 su queste etichette, il piano si ferma lì e si riusa il turno di prova.

### 6.2 L'affidabilità delle classi (automatica)
Su tutte le board misurate dal banco congelato (decine di migliaia di effetti, nessuna etichetta umana) si conta, per ogni classe di anatomia, quante volte un effetto inventariato da L0 è attivabile al L2: k successi su n. Le classi rare si stabilizzano con uno stimatore bayesiano empirico beta-binomiale: ρ = (a + k)/(a + b + n), con a e b stimati sulle classi grandi per momenti (Efron e Morris 1975; Gelman et al., *Bayesian Data Analysis*, cap. 5). Durante una ricerca la stessa formula parte da questi a e b del deck e si aggiorna con le board misurate da `learn`.
- Si riporta il richiamo di L0 rispetto a L2 (un effetto attivabile al L2 che L0 non vede è un buco del lettore o della definizione): obiettivo ≥ 0,95, come il criterio del brief A.
- Una classe con ρ sotto 0,2 si guarda a mano: di solito è una condizione che il L0 non sa leggere (ad esempio «se controlli un mostro …»), da segnalare al lettore, non da correggere per nome.

### 6.3 Come si accetta ogni PR
- **La modalità normale.** Ogni PR del motore di questo piano lascia uguale la modalità solita: classe E, `banco.py same` sul set `full`, tutte le 193 uscite uguali (`regole-motore.md` §2). Nota: dal 24/10 la prova equa sulle 406 board è sostituita dal banco congelato, perché non si può rifare; `paired_test.py` resta per gli studi su mani scelte.
- **La modalità «interazione».** Il suo numero è C misurato con L1+L2. Il confronto è *ricerca guidata* (questo piano) contro *riordino dopo la ricerca* (PR #171), stesse 193 mani, stesso seme e budget, rimisurate entrambe con L1+L2: sono le regole R del banco sulla scala di C (non inferiorità con bootstrap per deck, test del segno, grandi perdite, deck freschi). Serve un'opzione `--metric interaction` in `banco.py` e `paired_test.py`. Si dice «migliora» solo con il limite basso dell'intervallo sopra 0.
- **La misura L1+L2.** È una misura nuova (classe M per la modalità): si giudica sulla verità del §6.1. Il turno di prova di oggi non cambia; se un giorno L2 sostituisse parti del turno di prova normale, quella sarebbe una PR M a parte, con la verità v1 del banco.

## 7. Letteratura
Citate a memoria e non ricontrollate online in questa sessione: le referenze sono classiche, ma anno e sede vanno verificati prima di citarle in un documento di misura (come in `ricerca-letteratura.md` §8).

| Tema | Fonte | Che cosa si prende |
|---|---|---|
| Obiettivi in eccesso | D. E. Smith, «Choosing objectives in over-subscription planning», ICAPS 2004; J. Benton, M. B. Do, S. Kambhampati, «Anytime heuristic search for partial satisfaction planning», *Artificial Intelligence* 173, 2009 | il problema (§4.1): massimizzare l'utilità degli obiettivi raggiunti, ricerca anytime |
| Obiettivi morbidi | E. Keyder, H. Geffner, «Soft goals can be compiled away», *JAIR* 36, 2009 | perché non serve trasformarli (§4.1) |
| Euristiche rilassate | J. Hoffmann, B. Nebel, «The FF planning system», *JAIR* 14, 2001; B. Bonet, H. Geffner, «Planning as heuristic search», *AIJ* 129, 2001 | già in `ricerca-letteratura.md` §2: B4 aiuta anche questa modalità |
| Diversità delle finaliste | J.-B. Mouret, J. Clune, «Illuminating search spaces by mapping elites», arXiv:1504.04909, 2015 | l'archivio a celle (§4.2) |
| Multi-obiettivo | L. Mandow, J. L. Pérez de la Cruz, «Multiobjective A* search with consistent heuristics», *JACM* 57(5), 2010 | il fronte di Pareto (§4.2) |
| Capacità | A. Schrijver, *Combinatorial Optimization*, Springer 2003 (b-matching, flussi) | la capacità come programma intero piccolo (§2.2) |
| Intervalli per proporzioni | E. B. Wilson, *JASA* 22, 1927; C. J. Clopper, E. S. Pearson, *Biometrika* 26, 1934; L. D. Brown, T. T. Cai, A. DasGupta, «Interval estimation for a binomial proportion», *Statistical Science* 16, 2001 | p_k (§5.2), richiamo (§6.1) |
| Bootstrap | B. Efron, R. Tibshirani, *An Introduction to the Bootstrap*, 1993 | media e contributo (§5.2), come nel banco |
| Arresto sequenziale | A. Maurer, M. Pontil, «Empirical Bernstein bounds and sample variance penalization», COLT 2009; V. Mnih, C. Szepesvári, J.-Y. Audibert, «Empirical Bernstein stopping», ICML 2008 | quando smettere di campionare (§5.2) |
| Riduzione della varianza | P. Glasserman, *Monte Carlo Methods in Financial Engineering*, 2004, cap. 4; A. B. Owen, *Monte Carlo theory, methods and examples*, 2013; M. B. Giles, «Multilevel Monte Carlo path simulation», *Operations Research* 56(3), 2008 | mani gemelle, stratificazione, due fedeltà (§5.3) |
| Bayes empirico | B. Efron, C. Morris, «Data analysis using Stein's estimator and its generalizations», *JASA* 70, 1975; A. Gelman et al., *Bayesian Data Analysis* | affidabilità delle classi (§6.2) |
| Contributo di una carta | L. S. Shapley, 1953; J. Castro, D. Gómez, J. Tejada, «Polynomial calculation of the Shapley value based on sampling», *Computers & OR* 36(5), 2009 | §5.5 |
| Accordo e coppie | J. Cohen, *Educ. Psych. Meas.* 20, 1960; Q. McNemar, *Psychometrika* 12, 1947 | §6.1 |
| Giochi di carte | C. D. Ward, P. I. Cowling, «Monte Carlo search applied to card selection in Magic: The Gathering», IEEE CIG 2009 | precedente di Monte Carlo sulle mani per scegliere carte |

## 8. Coordinamento con il lettore degli effetti
Il L0 regge sul brief A di `anatomia-effetti.md`. Campi che servono, oltre a quelli già nel piano:
1. **il codice del limite** (`GetCountLimit` con il suo codice), per unire copie ed effetti con lo stesso nome nel gruppo g(e) e distinguere «per turno» da «per duello»;
2. **il raggio dei bersagli degli effetti di campo** (il lato avversario o il tuo), per riconoscere i floodgate dalla lettura: va verificato se il submodule ha un getter (`GetTargetRange` o equivalente); se manca, i floodgate restano al L2;
3. **le categorie `HANDES`, `TODECK`, `TOHAND`, `CONTROL`, `POSITION`** nella stessa maschera, che il piano dell'anatomia nomina solo in parte;
4. **la famiglia dell'evento** dei trigger (attivazione, Evocazione, attacco, carta al Cimitero, ricerca), per la classe di anatomia e per scegliere gli stimoli del L2;
5. **il collegamento attivazione → effetto** già previsto, per legare le risposte offerte al L1 e al L2 alla loro classe.

Il messaggio con questi cinque punti è stato mandato al thread del lettore il 25/10.

## 9. Brief
Ordine: dopo il brief A dell'anatomia, I1 → I2; I3 può andare in parallelo a I2 (file diversi); I4 dopo il mockup e dopo I2.

### Brief I1: inventario, manichino e capacità (motore, classe E)
```
Compito: duelforge-cli combos --interaction e probe-line stampano per ogni board la capacità C e C_q (§2.2), il profilo (§2.3) e i tre livelli L0, L1 e L2 (§3), con il manichino e gli stimoli nuovi (attacco, carta aggiunta alla mano, bersagli); la modalità normale non cambia.
Perché: docs/plans/linee-interazione.md §2-§3.
Branch: claude/interazione-inventario, da main dopo il brief A dell'anatomia
File da toccare: crates/ygo-solver/src/inventory.rs (nuovo: L0 e capacità), interaction.rs, probe.rs (manichino e stimoli, solo nella modalità: file caldo, chiederlo in TASKS.md), crates/ygo-cli; test con fixture.
Da leggere: linee-interazione.md §2-§3, §6.2; anatomia-effetti.md §2.1; interazione-fine-turno.md.
Verifica: cargo test -p ygo-solver -p ygo-cli; banco.py same --set full (193 uscite uguali); out/interazione/summary.txt con richiamo e precisione di L0 rispetto a L2 sulle board del banco e la tabella di ρ per classe; il foglio delle 30 board per Valerio (§6.1).
Criterio di accettazione: banco ACCEPTED (E); richiamo di L0 rispetto a L2 ≥ 0,95; C ≤ somma degli effetti su ogni board; due copie di un effetto con limite per nome contano 1.
PR: template; summary.txt per intero; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief I2: ricerca guidata dall'interazione (motore, classe E per la modalità normale)
```
Compito: combos --objective interaction cerca con la stima Ĉ del L0 (§4.2), tara ρ durante learn, tiene le finaliste in un archivio a celle, le misura con L1+L2 e stampa la classifica lessicografica e il fronte di Pareto; banco.py e paired_test.py hanno --metric interaction.
Perché: docs/plans/linee-interazione.md §4 e §6.3.
Branch: claude/interazione-ricerca, da main dopo I1
File da toccare: crates/ygo-solver/src/search.rs (file caldo, chiederlo in TASKS.md), inventory.rs, interaction.rs, crates/ygo-cli, scripts/banco.py, scripts/paired_test.py.
Da leggere: linee-interazione.md §4, §6.3; regole-motore.md §2-§3.
Verifica: banco.py same --set full (modalità normale); banco.py run/judge --metric interaction, ricerca guidata contro riordino della PR #171, 193 mani, in background; search_regression.
Criterio di accettazione: modalità normale uguale (E); sulla metrica C le regole R del banco ACCEPTED contro il riordino; «migliora» solo con il limite basso sopra 0.
PR: template; i due summary.txt per intero; mani che perdono oltre il 5% con il perché.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief I3: da una carta, la distribuzione (CLI e statistica)
```
Compito: combos --card <carta> --interaction stima su mani campionate E[J], p_k = P(J ≥ k) per k = 1..5 e il contributo Δ(c) con mani gemelle (§5), con intervalli bootstrap e di Wilson, arresto sequenziale e mani uguali giocate una volta; JSON e out/<nome>/summary.txt.
Perché: docs/plans/linee-interazione.md §5.
Branch: claude/interazione-carta, da main dopo I1 (in parallelo a I2: fino ad allora usa il riordino della PR #171)
File da toccare: crates/ygo-cli/src/main.rs, crates/ygo-stats (intervalli, stratificazione), test.
Da leggere: linee-interazione.md §5; interazione-fine-turno.md («--card»).
Verifica: cargo test -p ygo-stats -p ygo-cli; su 3 deck del banco con una carta ciascuno, in background: copertura degli intervalli su 200 ripetizioni con dati simulati (il 95% deve contenere il valore vero entro ±2 punti); correlazione tra 1000 e 4000 nodi misurata e scritta nel piano, per decidere le due fedeltà.
Criterio di accettazione: intervalli che coprono come dichiarato; riduzione della varianza delle mani gemelle misurata e riportata; nessuna modifica del motore (banco.py same se tocca ygo-solver).
PR: template; summary.txt dei tre deck.
Modello: Sonnet 5.5, effort medio (Opus se servono cambi nella ricerca).
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief I4: la modalità nel Combo Lab (interfaccia)
```
Compito: nel Combo Lab l'opzione «Cerca linee per l'interazione» con partenza da una mano o da una carta; per la mano, le board con C, profilo per tipo e zona e fronte di Pareto; per la carta, la curva P(almeno k interruzioni) con gli intervalli, il contributo della carta e le board migliori.
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
- **Il L0 vede poco** se il lettore manca categorie o condizioni: si vede subito dal richiamo rispetto a L2 (§6.2). Il L2 resta il giudice; il L0 serve solo a guidare.
- **Il manichino sbaglia i bersagli.** Alcuni effetti chiedono bersagli precisi (un mostro di Livello alto, una carta coperta da un certo turno). Il manichino ha bersagli generici: ciò che non trova si conta come «condizionato» e si mostra a parte, non come zero.
- **Contare non è valutare.** Tre cambi di posizione non valgono una negazione: per questo c'è C_q accanto a C e il fronte di Pareto. La scelta finale resta di chi guarda.
- **Il costo da una carta** (decine di minuti) va gestito come calcolo lungo: in background nella CLI, con avanzamento nell'app.
- **Dipendenze:** I1 aspetta il brief A dell'anatomia; I2 e I1 toccano file caldi (`probe.rs`, `search.rs`): uno per volta, chiesti in `TASKS.md`.
