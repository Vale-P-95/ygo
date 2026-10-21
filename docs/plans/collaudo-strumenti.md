# Collaudo: turno di prova contro un deck e mappa dei collegamenti

*Scritto il 21/10/2026 da Claude Code (sessione sorella, branch `claude/tools-qa`). Bozza: si completa con le tabelle finali.*

## Metodo
- **Deck:** 40 liste di Master Duel Meta (`scripts/mdm_decks.py`, non committate). Dati del motore scaricati con `fetch-data` e `update-cards`.
- **Mappa dei collegamenti:** `scripts/links_report.py` lancia `duelforge-cli links` su tutti i deck (valori predefiniti: 2 mani per carta, 1000 nodi, 4 thread). Poi confronta la mappa con il testo delle carte (`data/cdb/cards.cdb`):
  - **mancanti:** frasi come "add ... from your Deck to your hand" senza il collegamento di quel tipo dalla carta;
  - **strani:** collegamenti la cui causa non ha nel testo le parole del tipo (un "summon" da una carta che non Evoca).
  - Il testo è solo un indizio: ogni caso segnalato l'ho controllato a mano sul testo della carta.
- **Turno di prova contro un deck:** `scripts/matchup_report.py`.
  - 12 deck "tuoi": branded, tearlaments, yubel, ryzeal_mitsurugi, k9, white_forest_azamina, dracotail, blue_eyes, synchrons, fur_hire, orcust, yummy.
  - 5 avversari: kaiju (breaker nel Main Deck, Maxx "C" e Mulcharmy), labrynth (trappole, Ash, Droll, Impermanence, Dominus), maliss (pieno di handtrap, Bystial), invoked (Nibiru, Veiler, Ghost Belle e Ogre), sky_striker (Evenly Matched, Ghost Belle).
  - Mano: 5 carte estratte con seme fisso come in `scripts/bench.py`; `combos --nodes 3000 --threads 1 --top 3 --save-lines`; poi `probe-line --vs` sulle prime 3 board con `DUELFORGE_DEBUG_MATCHUP=1`.
  - 180 misure (12 × 5 × 3), nessun errore "matchup failed".
- **Verifica delle ipotesi:** una copia del repository fuori dal branch (worktree nello scratchpad, mai committata) con tracce in più e le correzioni proposte dietro variabili d'ambiente (`QA_FIX_A`, `QA_FIX_B`, `QA_FIX_C`, `QA_FIX_D`). I numeri "dopo" vengono da lì. Il codice dei crate in questo branch non è toccato.
- Le misure girano in parallelo sulla stessa macchina (4 core): i tempi sono un po' gonfiati, i risultati no (budget a nodi, un thread).

## Problemi in ordine di gravità
Vedi le sezioni sotto; la sintesi è in "Conclusione".

### Alta 1. Maxx "C" e Mulcharmy fermano la linea in 6 deck su 12
- **Riprodurre:**
  ```
  python3 scripts/matchup_report.py <out> --decks <mdm> --own tearlaments,k9,fur_hire --vs kaiju
  duelforge-cli probe-line --deck tearlaments.ydk --lines <out>/tearlaments.lines.json --vs kaiju.ydk --index 0
  ```
- **Atteso:** 0 momenti (fanno solo pescare l'avversario).
- **Esce:** Maxx "C" ferma 550 momenti su 3955 (14%), Mulcharmy Fuwalos 388 su 3164 (12%), Purulia 178 su 1582. Su tearlaments 7 momenti su 32, fur_hire 12 su 19, k9 5 su 23, blue_eyes 2 su 27, branded 1 su 23, dracotail 1 su 28. Gli altri 6 deck: 0.
- **Quattro cause, tutte in `matchup.rs` (`replay`, `hand_trap_stops`):**
  - **A. Il deck dell'avversario del kit ha 12 carte.** Maxx "C" lo svuota in poche Evocazioni. Pilgrim Reaper ("each player sends the top 5 cards of their Deck") poi non si attiva più, e la linea Tearlaments si sfasa (traccia: risposta 72, la linea attivava Pilgrim Reaper, dopo Maxx "C" non c'è più nella lista).
  - **B. Domande nuove sì/no trattate come sfasamento.** Maxx "C" e Mulcharmy vanno al Cimitero dalla mano come costo: Necroquip Princess ("If a monster is sent from the hand to the GY to activate a card or effect") riceve un trigger nuovo (`EffectYesNo`). Anche la Fase Finale richiesta di nuovo dopo una catena (`IdleCmd` con solo "End Phase", linea Branded). `replay` risponde in modo semplice solo alle finestre `SelectChain` vuote: qui segna "diverged".
  - **C. Le risposte della linea sono indici.** Dopo la handtrap la lista delle azioni cambia e lo stesso indice sceglie un'altra carta. K9: dopo Maxx "C" K9-17 Izuna si può Evocare dalla mano ("if your opponent has activated a monster effect in the hand"), e la risposta `[5,0,0,0]` dell'`IdleCmd` sceglie Izuna invece di Chaotic Elements.
  - **D. La stessa scelta arriva con un'altra forma.** Fur Hire: la pescata di Maxx "C" dà a Rex, Ride Fur Hire un trigger ("If a card is added from the Deck to your opponent's hand"). Il trigger di Rex, Freight Fur Hire, che la linea aveva come `EffectYesNo`, ora è in una `SelectChain` con due trigger.
  - A parte la A, sono interazioni vere del gioco (K9 e Fur Hire puniscono Maxx "C"), ma la board finale non cambia: la linea si può ancora giocare uguale.
- **Correzioni proposte (valgono per qualunque deck):**
  - **A.** In `hand_trap_stops` (e in generale nel kit, vedi Alta 3) dare all'avversario un deck di dimensione vera: le carte del kit più carte semplici fino a 35 (40 meno la mano). Le carte da cercare restano.
  - **B.** In `replay`, dopo la handtrap: una domanda di tipo diverso da quella della linea che ha una risposta passiva (`EffectYesNo`, `YesNo`, `SelectChain`, `IdleCmd` con solo la fine del turno) riceve la risposta passiva, e la risposta della linea resta per la domanda dopo. Si segna "diverged" solo se non c'è risposta passiva.
  - **C+D.** In `own_answers` salvare per ogni risposta cosa fa: carta (codice originale), zona, descrizione dell'effetto e tipo di azione (`Action.kind`). In `replay`, dopo la handtrap, cercare nella lista nuova l'azione con la stessa identità e usare la sua risposta; un `EffectYesNo` sì vale come l'attivazione della stessa carta in una `SelectChain`. Se l'azione non c'è più, la linea è fermata. Attenzione al tipo di azione: senza, la prima prova ha scelto "Evoca" al posto di "Posiziona" e ha peggiorato tutto (k9 da 5 a 19 momenti).
- **Numeri prima → dopo** (stessi 180 casi):

| Correzione | Maxx "C" | Fuwalos | Purulia | Veiler | Deck con Maxx/Mulcharmy > 0 |
|---|---|---|---|---|---|
| nessuna | 550/3955 | 388/3164 | 178/1582 | 163/617 | 6 su 12 |
| A + B | 305/3955 | 136/3164 | 112/1582 | 130/617 | 4 su 12 |
| A + B + C/D | *(in misura)* | | | | |

### Alta 2. Mappa: la causa di un trigger facoltativo è la carta dell'ultima azione
- **Riprodurre:** `duelforge-cli links --deck branded.ydk`.
- **Atteso:** Branded in High Spirits non Evoca niente ("Reveal 1 monster ... send 1 Level 8 Fusion Monster from your Extra Deck to the GY").
- **Esce:** "Branded in High Spirits → summon grave" verso 9 mostri (Fallen of Albaz, Incredible Ecclesia...), e Alba-Lenatus verso 3.
- **Causa:** in `links.rs`, `play` aggiorna `reader.acting` solo per `IdleCmd` e `SelectChain`. Guiding Quem si attiva con un `EffectYesNo` ("If a card leaves the Extra Deck"), e il suo bersaglio dal Cimitero (`SelectCard`, suggerimento 509) viene dato alla carta dell'ultima azione, High Spirits.
- **Correzione:** quando rispondi sì a un `EffectYesNo` (o a una `SelectChain` con `triggers: true`), `acting` diventa quella carta.
- **Prova:** nella copia sperimentale (`QA_FIX_C`) sul deck Branded spariscono esattamente quei 12 collegamenti, nessun altro cambia (394 → 382).
- Altri casi con lo stesso schema, da ricontrollare dopo la correzione: Infinitrack Goliath e Superdreadnought Rail Cannon Juggernaut Liebe "recover" (earth_machine), Vernusylph of the Thawing Mountains "recover", Deception of the Sinful Spoils "summon grave" (chimera), Branded Banishment "search" di Starliege Seyfert.

### Alta 3. Il deck dell'avversario del kit ha 12 carte (ricerca e mappa)
- That Grass Looks Greener manda al Cimitero carte finché il Deck è grande come quello dell'avversario. In goldfish ne manda 43 su 55 (deck Branded da 60).
  ```
  duelforge-cli combos --deck branded.ydk --hand "That Grass Looks Greener;Droll & Lock Bird;Maxx \"C\";Pot of Sloth;Nadir Servant" --nodes 800 --threads 1 --top 1 --lines
  ```
  In partita, contro un deck da 35 carte, ne manderebbe 20.
- Sulla mappa: "That Grass Looks Greener → toGrave" verso 38 carte, ognuna "usata" nel 22-24% delle linee.
- Vale per ogni effetto che guarda il deck dell'avversario: Pilgrim Reaper (sopra), le pescate di Maxx "C", Pot of Sloth (carte controllate dall'avversario).
- **Correzione:** `Kit::standard` (`probe.rs`) riempie il deck dell'avversario di carte semplici fino a 35, in fondo, così le carte da cercare restano. Cambia la ricerca: va misurata con il protocollo (bench prima e dopo).

### Media
*(da completare)*

### Bassa
*(da completare)*

## Conclusione
*(da completare)*
