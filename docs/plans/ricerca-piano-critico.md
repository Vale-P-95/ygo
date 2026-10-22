# Ricerca delle combo: piano critico per board più forti in meno tempo

*Scritto il 22/10/2026 dal coordinatore, dalle prove del proprietario nell'app vera. Il proprietario giudica la ricerca lontana dallo standard che vuole: troppo lenta, con tempo perso su linee inutili, ripetitive o deboli, e board finali non all'altezza delle linee note.*

## Cosa ha visto il proprietario
- **Invoked, ricerca per obiettivo.** Obiettivo: Invoked Oceanus nella Zona Mostri Extra e S:P Little Knight sul campo.
  - La ricerca trova una linea che lo raggiunge, ma passa per Hyperinvoked Aeon Evocato con 2 materiali.
  - Aeon vale molto di più con 3 materiali di Attributi diversi: bandisce dal campo e dai Cimiteri tante carte quanti sono i materiali, e con 3 o più materiali anche fino a 3 carte dell'Extra Deck avversario.
  - Le guide (video) arrivano alla stessa board con Aeon a 3 materiali, passando per i Link Magistus e cercando Crowley dal Deck.
- **I tempi** per trovare le linee sono troppo lunghi.
- **Ruoli («Riconosci»):**
  - Super Polymerization non risulta board breaker;
  - una carta non può essere sia starter sia estensore;
  - Invocation «Grail» risulta solo board breaker, ma è anche un estensore.

## Due difetti diversi
1. **La misura non vede il valore di Aeon a 3 materiali.** Il turno di prova conta quello che la board ferma nel turno avversario. Le carte bandite dall'Extra Deck avversario, e in generale i bonus che dipendono dal numero di materiali, non entrano nel punteggio. Quindi anche una ricerca perfetta non ha motivo di preferire la linea a 3 materiali. È lo stesso tipo di cecità dei giudizi del proprietario (`tests/fixtures/boards/owner-verdicts.tsv`): carte che cambiano la partita e valgono 0 nella misura.
2. **La ricerca spende nodi e tempo dove non serve.** Non sappiamo ancora quanto né dove: va misurato prima di correggere. Le correzioni passate (#46, §29.4; velocità, `docs/plans/velocita-ricerca.md`) hanno tolto mosse inutili e reso il motore più veloce, ma nessuno ha misurato quanti nodi di una ricerca portano davvero alla board migliore, né quanto la board trovata è lontana dalla migliore possibile.

## Fase A: misurare (prima di toccare la ricerca)
### A1. Riferimenti automatici: l'oracolo
Il motore deve trovare da solo le linee forti: il proprietario non scrive combo, e i riferimenti non possono dipendere da lui. Il metro si costruisce in automatico.
- **L'oracolo.** Per un insieme fisso di mani si lancia la stessa ricerca con un budget enorme, per esempio 50-100 volte i nodi, più semi e più thread, anche per ore, di notte. Si tengono le board migliori trovate e la loro misura.
  - Le mani: le 16 del banco, le 82 di `combo_hunt --seed 7`, e le ricerche per obiettivo con obiettivi presi dalle board forti di quei deck (ad esempio una board Invoked con Oceanus nella Zona Mostri Extra e S:P).
- **La forza vera delle board dell'oracolo** viene dal banco oggettivo (livello 2 contro il meta), non solo dalla stima rapida.
- **Formato:** `tests/fixtures/oracle/<deck>-<mano>.json`, con mano, board migliori (carte e zone, senza testi), misura e linea.
- **Misure per ogni mano**, con la ricerca normale:
  - dopo quanti nodi e secondi arriva al 90%, al 95% e al 100% del valore dell'oracolo;
  - se trova la board dell'oracolo e in che posizione la mette;
  - nella ricerca per obiettivo, quanto è forte la linea trovata rispetto alla migliore che raggiunge lo stesso obiettivo.
- **Quando l'oracolo stesso non arriva a una board** (ad esempio Aeon a 3 materiali), si capisce perché rigiocando i passi con lo strumento che rigioca una linea:
  - un'azione che il motore non offre;
  - una risposta automatica sbagliata;
  - una potatura che taglia il ramo.
  Sono difetti del motore o della ricerca, da correggere in modo generale.

### A2. Dove va il lavoro della ricerca
Strumentazione dietro una variabile d'ambiente: senza la variabile la ricerca non cambia. Per ogni ricerca del banco e delle linee A1 conta:
- **nodi doppi:** stati già visti per un'altra strada, presi o non presi dalla chiave dei nodi;
- **azioni che non cambiano niente:** lo stato canonico resta uguale, o cambia solo in modi che la board finale ignora;
- **ordini scambiabili:** due azioni indipendenti esplorate in entrambi gli ordini;
- **rami morti:** sottoalberi senza nessuna board vicina alla migliore, quanto pesano;
- **ordinamento delle mosse:** a ogni passo della linea migliore trovata, in che posizione era l'azione giusta tra quelle offerte. Se arriva sempre dopo molte altre, la stima rapida guida male;
- **tempo per fase:** ricerca, apprendimento, conferma, varianti di zona, turno di prova. Questa parte c'è già, va solo raccolta su tutto il banco.

Risultato atteso: una tabella che dice quali sono i tre sprechi più grandi, in percentuale dei nodi e del tempo.

## Fase B: correggere (in ordine di quanto spreco toglie)
Ogni correzione è generale: niente nomi di carte. Segue il protocollo del motore (`AGENTS.md`):
- banco, `combo_hunt --seed 7` e `search_regression`;
- in più l'oracolo A1, che diventa un controllo fisso.

- **B1. Ordinamento delle mosse.** Una stima rapida che ordini meglio le azioni, dalla misura A2: prima quelle che aggiungono carte o Evocano dal Deck, poi il resto; il Set e la fine del turno dopo. Pesi imparati sulle linee migliori del banco.
- **B2. Ordini scambiabili.** Riduzione per ordine parziale: due azioni che non si influenzano si esplorano in un solo ordine.
- **B3. Chiave dei nodi con le scelte che contano.** Bersagli e nomi dichiarati durante un effetto (§30.8). Provata e tolta perché peggiorava di poco la caccia; con A1 e A2 si può decidere su numeri migliori.
- **B4. Ricerca per obiettivo guidata dalla mappa dei collegamenti.** La mappa sa quali carte portano a quali e con che probabilità (PR #61). Nella ricerca per obiettivo può fare da distanza: un'euristica in stile A* che preferisce le azioni che avvicinano le carte dell'obiettivo. Tra le linee che raggiungono l'obiettivo vince quella con la board più forte, non la prima trovata.
- **B5. Il valore che la misura non vede.** Va nella classificazione delle board (#47, dopo D17):
  - carte bandite o tolte all'avversario nel tuo turno, come l'Extra Deck bandito da Aeon;
  - bonus che dipendono dal numero di materiali;
  - i casi del primo tipo delle note del proprietario (Elfnote Fortuna, Neo Blue-Eyes nel Cimitero).
  Senza questo la ricerca non ha motivo di scegliere Aeon a 3 materiali.
- **B6. Più thread che rendano.** Oggi 4 thread danno +16% contro +29% di uno (§velocità): la cache si satura. Da rivedere dopo B1-B3.
- **B7. Partire da linee già trovate.** Per lo stesso deck, le linee della mappa dei collegamenti e delle ricerche precedenti possono seminare la ricerca (prefissi promettenti), invece di ripartire da zero.

## Obiettivi misurabili
- **Oracolo A1:** la ricerca normale arriva al 95% del valore dell'oracolo entro 30 secondi in 9 mani su 10, e la board dell'oracolo è nella top 3 in 8 su 10.
- **Tempo** per arrivare alla board migliore sulle mani del banco: mediana dimezzata.
- **Forza:**
  - la verità del banco oggettivo delle board migliori non scende;
  - `best` del banco a nodi fissi non scende più del 2%.
- **Invoked:** l'obiettivo Oceanus nella Zona Mostri Extra + S:P si raggiunge con Aeon a 3 materiali, e la board viene classificata sopra quella con Aeon a 2.

## Ruoli (correzione piccola, a parte)
File: `crates/ygo-solver/src/roles.rs` e i suoi test.
- **Board breaker dal testo.** Oggi `breakers_by_text` cerca frasi come «destroy all» o «your opponent controls». Va riconosciuto anche chi usa le carte dell'avversario come materiale o come Tributo: «from either field», «monster(s) your opponent controls» insieme a «material» o «Tribute». Così Super Polymerization diventa board breaker, e con lei ogni carta con lo stesso schema, senza nomi.
- **Starter ed estensore insieme.** Oggi uno starter non si prova mai come estensore (`else if`). Uno starter va provato anche come estensore sulla linea di un altro starter: per lo starter migliore si usa la linea del secondo.
- **Estensore e board breaker insieme** è già possibile. Va capito perché Invocation «Grail» non risulta estensore:
  - la soglia `EXTENDER_GAIN`;
  - un materiale preso all'avversario, che giocando per primi non c'è;
  - il budget di 400 nodi.
  La correzione deve valere per ogni deck.
- **Controllo:** `duelforge-cli roles` prima e dopo su almeno 10 deck di Master Duel Meta, con l'elenco delle carte che cambiano ruolo, ognuna verificata sul testo.

## Chi fa cosa
- **Ruoli:** sessione sorella (corsia 6, che ha finito la mappa). File: `roles.rs` e test.
- **A1 (oracolo):** corsia 6 dopo i ruoli. File nuovi: script e fixture, nessun file caldo. Tutto automatico: nessuna linea scritta a mano.
- **A2 e poi B1, B2, B3, B6, B7:** corsia 2 (velocità), una correzione per PR. Lavora in `search.rs`, quindi una sola corsia alla volta su quel file.
- **B4:** dopo B1-B3, perché tocca la stessa parte di `search.rs`.
- **B5:** coordinatore, dopo D17 (stasera) e D12 (corsia 4), perché tocca `probe.rs`.
