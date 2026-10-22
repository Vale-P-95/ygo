# Revisione bibliografica: pianificazione del primo turno in un gioco di carte a regole, con generalizzazione a carte mai viste

*Corsia 10, 22-23/10/2026, branch `claude/literature-review`. Per il proprietario e il coordinatore. Estende e corregge `ricerca-letteratura.md` (22/10), che resta il riferimento per le idee già adottate; qui non si ripetono le fonti già verificate lì (§8.3), se non per correggerle o per collocarle rispetto a fonti nuove.*

**Bozza in lavorazione.** Le sezioni si riempiono una alla volta e si pushano a ogni passo.

## 0. Sommario esecutivo
*(da scrivere alla fine: 15 righe, le 5 direzioni più promettenti con i lavori chiave)*

## 1. Domande, metodo e registro delle interrogazioni

### 1.1 Le due domande
1. Che cosa esiste di applicabile al nostro problema: pianificazione del primo turno in un gioco di carte enorme, guidato da regole (motore EDOPro con script Lua), in parte casuale, con generalizzazione a carte mai viste?
2. Quali tecniche conviene trasferire, e quali ipotesi di quei lavori reggono o si rompono in Yu-Gi-Oh!?

### 1.2 Cosa rende Yu-Gi-Oh! diverso (i criteri di trasferibilità)
Ogni fonte è giudicata contro queste sette proprietà del nostro problema. «Regge» o «si rompe» si riferisce a queste.
- **P1. Catene e finestre di risposta.** Ogni attivazione apre una finestra in cui entrambi i giocatori possono rispondere; la risoluzione va al contrario. Il turno non è una sequenza piatta di azioni.
- **P2. Fattore di ramificazione enorme e profondo.** Una linea di combo ha decine di decisioni (Evocazioni, bersagli, materiali, zone, sì/no), e molte sono scelte dentro un'altra scelta.
- **P3. Caso nel proprio turno.** Pescate, carte mandate dal Deck, monete e dadi: il turno è stocastico anche senza l'avversario.
- **P4. Carte mai viste.** Escono carte nuove ogni mese; il motore le sa giocare dallo script, e il cercatore non deve conoscerle per nome.
- **P5. Primo turno senza azioni dell'avversario, ma valutato contro il suo turno.** La ricerca è a un giocatore; il valore della board è una misura contro un avversario (turno di prova, livello 2).
- **P6. Nodi che costano millisecondi.** Ogni stato si genera rigiocando il motore; non esiste un modello compatto e invertibile.
- **P7. Obiettivo: la board più forte, non il cammino più corto.** Le linee che raggiungono un obiettivo si confrontano per forza della board; la lunghezza è solo spareggio.

### 1.3 Criteri di inclusione ed esclusione
*(da compilare)*

### 1.4 Dove si è cercato e con quali limiti
*(da compilare: motori raggiungibili, livello di lettura)*

### 1.5 Registro delle interrogazioni
*(tabella: data, motore, interrogazione, risultati guardati, fonti tenute)*

## 2. Risultati per gruppo
### 2.1 Yu-Gi-Oh! (gruppo 1)

**Esito della ricerca.** Nessun articolo scientifico (peer-reviewed o arXiv) che tratti Yu-Gi-Oh! come problema di pianificazione o di apprendimento per rinforzo è emerso dalle 8 interrogazioni del gruppo né dalla query combinata A. I risultati accademici con «Yu-Gi-Oh!» nel testo sono riconoscimento di immagini (Konami 2018, un brevetto USPTO sulle contraffazioni, un articolo SciTePress 2025 sulla visualizzazione delle sinergie in Master Duel) e citazioni di passaggio in lavori su Hearthstone. Il lavoro vero sta in **quattro repository**, tre dei quali con documentazione tecnica abbastanza ricca da valere come fonte primaria. Li abbiamo letti per intero (README, documenti di progetto e, dove indicato, codice). Il quinto, WindBot, è la base dei bot a script.

Un fatto nuovo rispetto a `ricerca-letteratura.md`: nel 2026 c'è stata una **prima «YGO agent competition»** (annuncio su Bilibili, non raggiungibile dal container), vinta da un agente a self-play su un mirror Sky Striker, e Konami ha annunciato una gara interna fra due squadre con 30 giorni per costruire un duellante artificiale per Master Duel (notizia Automaton, letta solo dal riassunto del motore di ricerca). Nessuna delle due produce, per ora, articoli o codice sul primo turno.

#### Schede

**G1.1 — sbl1996/ygo-agent** (repository, 173 stelle; letto: README, `docs/feature_engineering.md`, `docs/network_design.md`, `docs/action.md`, `ygoai/rl/jax/agent.py`).
- *Problema.* Giocare partite intere di Yu-Gi-Oh! contro un avversario, con un agente addestrato.
- *Metodo.* Ambiente `ygoenv` su envpool e ygopro-core; PPO in self-play contro un checkpoint fisso. Ogni carta è una riga di 41 caratteristiche (luogo, sequenza, posizione, attributo, tipo, ATK/DEF a bin) più un **indice in `code_list.txt`**: l'identità è una tabella di embedding imparata (1000 righe × 1024, indice 0 = carta sconosciuta); in alternativa si può caricare un file `embed.pkl` di embedding del testo delle carte generati da un modello linguistico. Codificatore transformer a 2 strati da 128 canali sulle carte, LSTM a 512 sullo stato, azioni legali come token (24 × 12 caratteristiche) valutate con FiLM: un logit per azione, le illegali mascherate. Le scelte a più carte sono serializzate con un segnale di «fine».
- *Risultati (dal README).* Un modello addestrato per «oltre 100 milioni di partite, l'equivalente di 32 RTX 4090 per 5 giorni»; un modello corto (50 iterazioni, circa 205 000 passi) vince quasi il 100% contro la mossa casuale e il 3,1% su 128 partite contro quello lungo. Nessun confronto con umani o con bot a script, nessun articolo.
- *Ipotesi.* Insieme di carte chiuso (`code_list.txt`); una carta fuori lista non ha codifica. Ricompensa solo alla fine della partita.
- *Trasferibilità: bassa-media.* L'ingegneria (ambiente vettorizzato, azioni come token con maschera) è lo standard che anche gli altri due repository seguono e vale per noi (P2). Rompe P4 (carte per indice) e P5 (gioca partite intere; non c'è una misura della board). Il costo di calcolo è fuori dal nostro perimetro.

**G1.2 — tommy85/MirrorForce** (repository, 12 stelle; letto: README, `docs/model.md`, `docs/features.md`, `docs/training.md`, `docs/search.md`). Campione della prima «YGO agent competition» (mirror Sky Striker, formato retro).
- *Problema.* Partite intere nel mirror Sky Striker, senza dati umani.
- *Metodo.* Transformer da 28,1 milioni di parametri addestrato da zero: set transformer sulle carte (4 strati, larghezza 384), transformer causale sugli eventi del turno (fino a 512 righe), un token riassuntivo per ogni turno precedente; ogni azione legale è una query che attende alle carte, quindi menù di qualunque dimensione. Testa di valore categorica (vittoria/pareggio/sconfitta), testa ausiliaria di credenza sulla mano avversaria (peso 0,1, non letta dalla politica). Addestramento: PPO-clip (ε 0,2) con un termine «magnete» temperatura × KL(π ‖ uniforme sulle azioni legali) e temperatura decrescente, secondo la ricetta Ataraxos di Sokota et al. (*Scalable decision-making for games of imperfect information*, Nature 2026; non letto, DOI non raggiungibile); critico centrale separato da 9,4 milioni di parametri che vede le carte nascoste di entrambi; 5120 ambienti, circa 4,9 milioni di decisioni per iterazione, circa 1080 iterazioni; **3 giorni su 32 GPU H20** (dai numeri del documento, circa 5 miliardi di decisioni: aritmetica nostra). All'iterazione 1000 una sola passata supervisionata su 96 scene scritte a mano (letali, pulizie del campo, tempi delle difese), perché la politica non le trovava da sola.
- *Rappresentazione delle carte (la parte che ci interessa).* «Ogni carta è un token costruito dai campi del database e dalla struttura del suo script Lua»: 78 caratteristiche esatte da `cards.cdb` più **fino a 16 «unità di effetto» estratte dallo script, ognuna un sacchetto hashato a 64 dimensioni di token semantici** (categoria dell'effetto, codice, range, condizioni, operazioni), più un residuo imparato per carta. Il documento non dice come il parser delimita le unità né come tratta una carta assente dalla tabella.
- *Risultati.* Vittoria nella competizione; contro il campione umano del formato, 5-3 nel primo al meglio di sette (due sconfitte per disconnessione) e 3-4 nel secondo. Le curve di Elo interno e di vittorie contro WindBot sono in figura, senza numeri nel testo.
- *Ricerca.* Esiste come libreria: campiona disposizioni delle carte nascoste coerenti con il pubblico, rigioca ogni azione alla radice per 5 proprie decisioni con la politica, valuta con la testa di valore e mescola Q e logit. **Nessun client la usa e non c'è un guadagno misurato sulla politica nuda.**
- *Limiti dichiarati.* Un solo deck; linee forti mai trovate (Raye in Kagari in Battle Phase per schivare Veiler); setta le carte appena pescate e rivela informazione.
- *Ipotesi.* Partite intere a due giocatori, un solo mazzo, calcolo da laboratorio.
- *Trasferibilità: media.* Non per il metodo (P5, P6 e il costo lo escludono), ma per due cose: (a) è la prima prova concreta che **una rappresentazione della carta letta dalla struttura dello script Lua** regge un agente competitivo, cioè la strada di P4 che `roles.rs` e la mappa dei collegamenti già imboccano; (b) la lista delle linee che la politica non trova da sola (scene scritte a mano, Raye/Kagari) conferma che il self-play puro perde proprio le sequenze che a noi interessano.

**G1.3 — n0tevi1/ygorl** (repository; letto: README, `docs/design/02-challenges.md`, `03-play-policy.md`, `docs/benchmarks.md`, `docs/solver.md`, `docs/funnel.md`, `docs/nets.md`, `docs/spikes/combo-solver.md`). Progetto di ricerca sul core EDOPro con tre obiettivi: giocare, costruire deck, prevedere la mano avversaria. È la fonte con più numeri misurati e la più vicina ai nostri strumenti.
- *Metodo.* Stessa famiglia di ygo-agent, con nove modifiche dichiarate (I1-I9): pool di snapshot, critico privilegiato con vantaggi Q-boosted, **snapshot dell'arena** (preso da ygo-combo-solver), warm-up per clonazione di comportamento dalle dimostrazioni del **solver del primo turno**, embedding congelati del testo per carta e per effetto, transformer causale sugli eventi, partite che iniziano a metà, KL verso una politica lenta, critico asimmetrico. La sua lista di 12 sfide (C1-C12) è una buona formulazione di «cosa si rompe in Yu-Gi-Oh!»: regole come codice (C1), azioni strutturate e variabili (C2), decine di decisioni per turno con ricompensa solo alla fine (C3), carte nuove ogni pochi mesi (C6), core single-thread senza clonazione dello stato (C11), script della comunità con bug che insegnano combo false (C9).
- *Risultati con numeri (da `benchmarks.md`; container a 4 core, pochi semi, intervalli spesso a cavallo dello zero: lo dice il documento stesso).*
  - **Ripristino da snapshot 0,59 ms contro 47,5 ms di replay** (mediana, rapporto 0,012; 1000 ripristini, 0 differenze nel flusso dei messaggi). La prima versione copia il prefisso usato e costa «circa l'1% del replay» (C11).
  - Motore: 2900-10 100 decisioni al secondo da 1 a 4 thread.
  - PPO da zero, 1 ora su 4 core: contro Greedy da 0,11 a 0,15-0,19; 3 ore in più, nessun guadagno. Dalla issue #83: su 120 mani, l'esplorazione casuale raggiunge la board obiettivo in 41, il miglior checkpoint PPO in 7.
  - Clonazione dalle dimostrazioni del solver più Greedy: contro Random 0,795, contro Greedy 0,415; su 100 obiettivi mai visti 41 raggiunti (il solver 62). Solo solver: 0,305 contro Random, perché copre solo il primo turno.
  - **Ablazione delle viste di carta** (3 semi, 2000 aggiornamenti): su deck tenuti fuori, solo-ID 0,609 contro 0,585-0,590 delle varianti con testo; sui 10 deck di prova 0,713 contro 0,656-0,677. Cioè gli embedding del testo **non hanno migliorato** la generalizzazione a deck nuovi, nel loro regime. Il calo su deck nuovi resta grande: contro Greedy da 0,83 a 0,53.
  - Ricerca a un passo con informazione nascosta (oracolo diagnostico): +0,8-1,8 punti, non significativo. Involucro «letale»: entro il rumore.
  - Imbuto del primo turno con il solver (`funnel.md`): tasso di mani morte 42,5% contro 40,8% delle partite vere (120 mani, p = 0,63); con 5 s per mano l'errore è +6,5 punti e si concentra nei deck a combo lunga, con 10 s +2,9; costo 101 s per deck. Tra le mani con una linea, 25 su 69 reggono la handtrap provata.
  - Ricerca guidata da un piano su Lunalight: 7/16 contro 5/16 senza guida, sotto la soglia di 8/16 che si erano dati.
  - Il documento conclude che «nessuno di questi risultati mostra un guadagno confermato di forza di gioco alla scala attuale».
- *Ipotesi.* Partite intere; la misura di forza è la vittoria contro Greedy e Random, non la forza della board.
- *Trasferibilità: alta come base di evidenza, bassa come metodo.* Tre numeri contano direttamente per il nostro piano: il rapporto snapshot/replay (B0, B13), il costo per mano del solver come oracolo esterno (A1b.4) e l'ablazione del testo (B5d, B12: il testo da solo non basta per P4; servono la struttura dello script o la misura). La sua clonazione dal solver è esattamente la «politica imparata dall'oracolo» di B12, e il suo esito (41 contro 62) ne misura il tetto.

**G1.4 — Armytille/ygo-combo-solver** e il fork **96jonesa/ygo-combo-solver** (repository, 11 stelle, AGPL-3.0; letto: README con la tabella dei riferimenti, `search.cpp` per intero in tre parti, oltre a `arena.cpp` e `operators.cpp` già letti in `ricerca-letteratura.md`). È il lavoro più vicino al nostro: stesso motore, stesso problema «raggiungi una board», un solo turno.
- *Metodo (dal codice).* Tre fasi. (1) **Rollout con politica adattata** (NRPA/GNRPA, `NrpaTop`, `Nrpa`, `PolicyRollout`, `Adapt`): la politica è una tabella di pesi su chiavi a 64 bit delle mosse (`plan_key`), il campionamento è un softmax con temperatura, bonus per repertorio noto, suggerimenti e operatori, penalità di fase; ogni livello tiene una copia della politica e si ferma dopo 8 iterazioni senza miglioramento. Il punteggio di un rollout è il massimo lungo la linea: `materiale × 1000 + stati nuovi` prima dell'obiettivo; all'obiettivo `1e12 − bruciate × 1e9 − azioni × 1e5 − profondità`, cioè lessicografico per risorse spese, poi azioni, poi decisioni. (2) **Novità alla Iterated Width** (`NoveltyCut`): atomi = carte sul campo, codici, presenze nelle zone nascoste, numero di carte nel Deck; **la tabella di novità è partizionata per progresso** (carte obiettivo piazzate, risoluzioni, rango seriale, zone richieste), e un contatore di stallo contro `novelty_patience` decide il taglio. (3) **Archivio alla Go-Explore** (`ArchiveObserve`) con celle per progresso di sotto-obiettivo, non per hash della board, e un **finisher** best-first alla Levin Tree Search (`RunLevin`): costo `log(profondità + 1) − log π + levin_h · h`, con varianti PHS* e con re-rooting; il backtracking rigioca solo il suffisso dal più profondo antenato ancora in pila. La distanza h è il **numero di Evocazioni ancora necessarie** sulle «ricette» osservate (`RecipeDistance`; i prodotti ignoti costano almeno 1, così h non è mai infinita). In più: riconoscimento con il senno di poi delle Evocazioni dall'Extra Deck (`HindsightCommit`, con peso ridotto perché «altrimenti la politica converge sulle Fusioni economiche»), macro di 2-N mosse scelte con una perdita alla Levin (`MineOptionCatalog`), landmark come fatti (carta, zona, k-esima copia) con la loro prima comparsa, un bandit sulle prime decisioni (`--qhat`), trasposizioni con digest a 64 bit di campo, zone nascoste, ordine del Deck e stato del processore, `elide_forced` perché «nel 74,6% dei nodi non c'è scelta».
- *Risultati.* Il README non ha benchmark. Dal lavoro di ygorl sullo stesso binario (`spikes/combo-solver.md`, `solver.md`): obiettivo Flamberge su Snake-Eye, sonda 10,1 s più ricerca 34,9 s, 48 linee trovate; su 50 mani di 10 deck con 20 s di budget 28 risolte (56%, 7,1 s di media per quelle risolte); scenari «di blocco» 147/160 con 10 s. «Thread 1 divide la resa per 5-6». Le patch al core (5) si applicano pulite al nostro `vendor/ygopro-core`.
- *Ipotesi che valgono anche per noi.* Motore come scatola nera, nessun nome di carta, un turno, stato ripristinato dall'arena.
- *Ipotesi che non valgono.* **L'obiettivo è «raggiungi la board», non «la board più forte»** (P7): `--optimize` migliora solo i costi, e una board più forte dell'obiettivo non viene cercata; non esiste una misura contro l'avversario (P5). Il caso nel proprio turno (P3) è spento con `DUEL_PSEUDO_SHUFFLE`. Le linee possono selezionare e annullare la stessa carta centinaia di volte (ygorl ne cita una da 320 passi).
- *Trasferibilità: alta per i meccanismi, bassa per l'impianto.* Da prendere: novità partizionata per progresso (B11), distanza in Evocazioni (B4, alternativa alla distanza rilassata sui collegamenti), finisher alla Levin con re-rooting (B4, B12), `elide_forced` (B2b), riconoscimento con il senno di poi (B4, credito AMAF), macro selezionate con una perdita (B7). Da non prendere: NRPA come motore principale, perché ogni rollout è un replay (P6) e perché il punteggio non vede la forza della board.

**G1.5 — ProjectIgnis/windbot** (repository; letto: README, `Game/AI/Decks/` e `DefaultExecutor.cs` del progetto IceYGO di origine).
- *Metodo.* «Intelligenza artificiale semplice e deterministica»: un esecutore C# per deck, con regole scritte a mano. Il README elenca 47 deck; la cartella `Game/AI/Decks` ha 64 file `*Executor.cs` più 2 varianti fra parentesi, cioè 66 file: i due numeri di `ricerca-letteratura.md` §8.2 sono entrambi giusti, contano cose diverse. `DefaultExecutor.cs` (oltre 1000 righe) tiene la logica comune: bersagli d'attacco per confronto di ATK, soglie di LP per le Solemn, dizionari di protezioni per ID, nessuna lettura del testo o degli script.
- *Trasferibilità: bassa.* È il controesempio della regola dei nomi. Serve solo come avversario di riferimento (MirrorForce e ygorl lo usano così) e come catalogo di «regole generiche» che un kit avversario potrebbe imitare senza ID (B5e).

**Altri repository visti ed esclusi.** crispy-chiken/YugiohAi (52 stelle: WindBot più un server Python di pesi, un MCTS «vecchio», nessun risultato numerico, lavori in corso); sbl1996/yugioh-ai (deprecato, confluito in ygo-agent); JMaster-Duel-Bot, Duel-Forge-Bot, Yugioh-bot (automazione del client per le missioni, non IA); «bot deck» per YGO Omega (pagine SEO).

#### Cosa dice il gruppo 1 per DuelForge
1. **Non esiste letteratura scientifica su Yu-Gi-Oh! come problema di pianificazione.** Il nostro problema (primo turno, board più forte, carte mai viste) non è affrontato da nessuno: è la lacuna §5.1.
2. **Tutti e tre gli agenti imparati usano la stessa struttura**: carte come insieme di token, azioni legali come token mascherati, transformer; il motore resta la fonte di verità (P1). È la rappresentazione da usare anche per un ordinatore di mosse o un valutatore imparato (B9, B12).
3. **La rappresentazione dallo script Lua è stata fatta davvero** (MirrorForce: 16 unità di effetto hashate), ma nessuno ha misurato la generalizzazione a carte mai viste; l'unica ablazione (ygorl) dice che il testo da solo non aiuta. P4 resta aperto e va misurato da noi (A1b.3).
4. **Lo snapshot dell'arena vale 80-100× il replay** nella misura di ygorl (0,59 ms contro 47,5 ms). È la stima esterna più solida per decidere B0/B13.
5. **Il self-play puro perde le linee che ci interessano** (MirrorForce ha dovuto aggiungere 96 scene a mano; ygorl: PPO 7/120 contro 41/120 della casualità). Un cercatore esatto sul motore resta la base, e la parte imparata va addestrata sulle sue linee (B12), non viceversa.
6. **Il solver di Armytille è la cassetta degli attrezzi più vicina** (novità per progresso, distanza in Evocazioni, finisher alla Levin, elide dei nodi forzati), ma risponde a «raggiungi» e non a «più forte»: l'innesto è nei singoli meccanismi, non nell'impianto.
### 2.2 IA per giochi di carte collezionabili (gruppo 2)
### 2.3 Apprendimento per rinforzo e self-play (gruppo 3)
### 2.4 Ricerca e pianificazione (gruppo 4)
### 2.5 Informazione nascosta e teoria dei giochi (gruppo 5)
### 2.6 Generalizzazione (gruppo 6)
### 2.7 Architetture ibride e rappresentazione (gruppo 7)
### 2.8 Aggiunte: pianificazione a un giocatore su sequenze lunghe, novità e larghezza, limiti ammissibili, riduzione per ordine parziale, Hearthstone/LOCM/Forge, Prismata

## 3. Tabella di tutti i lavori
*(riferimento, tipo, livello di lettura, trasferibilità)*

## 4. Raccomandazioni sul piano (A1b, voci B di `ricerca-piano-critico.md`)
*(ogni voce con un controllo misurabile)*

## 5. Lacune: dove la letteratura non c'è
*(opportunità di ricerca)*
