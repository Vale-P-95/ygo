# Collaudo: turno di prova contro un deck e mappa dei collegamenti

*Scritto il 21/10/2026 da Claude Code (sessione sorella, branch `claude/tools-qa`). Codice dei crate non toccato: le correzioni sono proposte, provate solo in una copia fuori dal branch.*

## Metodo
- **Deck:** 40 liste di Master Duel Meta (`scripts/mdm_decks.py`, non committate). Dati del motore da `fetch-data` e `update-cards`.
- **Mappa dei collegamenti:** `scripts/links_report.py` lancia `duelforge-cli links` su tutti i deck, con i valori predefiniti (2 mani per carta, 1000 nodi, 4 thread). Poi confronta la mappa con il testo delle carte (`data/cdb/cards.cdb`):
  - **mancanti:** una frase come "add ... from your Deck to your hand" senza un collegamento di quel tipo dalla carta;
  - **strani:** collegamenti la cui causa non ha nel testo le parole del tipo (un "summon" da una carta che non Evoca).
  - Il testo è solo un indizio: i casi riportati sotto li ho controllati a mano sul testo delle carte.
- **Turno di prova contro un deck:** `scripts/matchup_report.py`.
  - 12 deck "tuoi": branded, tearlaments, yubel, ryzeal_mitsurugi, k9, white_forest_azamina, dracotail, blue_eyes, synchrons, fur_hire, orcust, yummy.
  - 5 avversari:
    - kaiju: breaker nel Main Deck, Maxx "C" e Mulcharmy;
    - labrynth: trappole, Ash, Droll, Impermanence, Dominus;
    - maliss: pieno di handtrap, Bystial;
    - invoked: Nibiru, Veiler, Ghost Ogre e Belle;
    - sky_striker: Evenly Matched, Ghost Belle.
  - Mano: 5 carte estratte con seme fisso, come in `scripts/bench.py`. Poi `combos --nodes 3000 --threads 1 --top 3 --save-lines` e `probe-line --vs` sulle 3 board, con `DUELFORGE_DEBUG_MATCHUP=1`.
  - 180 misure (12 × 5 × 3), nessun "matchup failed".
  - In più, l'intestazione di `probe-line --vs` (breaker e handtrap letti) su tutti i 40 deck.
- **Verifica delle ipotesi:** una copia del repository nello scratchpad (`git worktree`, mai committata), con tracce in più e le correzioni proposte accese da variabili d'ambiente:
  - `QA_FIX_A`: deck dell'avversario di 35 carte nel rigioco delle handtrap;
  - `QA_FIX_B`: risposta passiva alle domande nuove dopo la handtrap;
  - `QA_FIX_D`: risposte della linea rigiocate per identità (carta, zona, effetto, tipo di azione) invece che per indice;
  - `QA_FIX_C`: nella mappa, la causa di un trigger facoltativo è la carta del trigger.
- Le misure giravano in parallelo sulla stessa macchina (4 core): i tempi sono un po' gonfiati, i risultati no (budget a nodi, un thread per la ricerca).

## Tabelle

### Mappa dei collegamenti (40 deck)
Tempo totale per deck; "Isolate": carte senza collegamenti né in entrata né in uscita.

| Deck | Tempo (s) | Carte | Collegamenti | Usati | Cerca | Evoca | Al Cimitero | Recupera | Materiale | Isolate | Mani | Linee |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| argostars | 21.8 | 36 | 17 | 13 | 13 | 2 | 0 | 2 | 0 | 24 | 40 | 780 |
| artmage | 33.8 | 41 | 106 | 100 | 31 | 47 | 5 | 8 | 15 | 12 | 46 | 920 |
| banquet_ftk | 44.3 | 33 | 41 | 39 | 18 | 1 | 13 | 9 | 0 | 14 | 34 | 680 |
| blue_eyes | 40.5 | 35 | 61 | 55 | 17 | 33 | 4 | 7 | 0 | 18 | 36 | 681 |
| branded | 92.3 | 61 | 394 | 231 | 101 | 96 | 114 | 76 | 7 | 3 | 88 | 1696 |
| chimera | 79.8 | 49 | 165 | 121 | 56 | 60 | 11 | 17 | 21 | 9 | 65 | 1300 |
| clown_crew | 16.7 | 30 | 47 | 29 | 18 | 21 | 8 | 0 | 0 | 15 | 37 | 510 |
| cyber_dragon | 28.3 | 36 | 88 | 80 | 28 | 15 | 18 | 9 | 18 | 12 | 44 | 880 |
| doomz | 52.0 | 33 | 111 | 103 | 42 | 49 | 9 | 11 | 0 | 11 | 40 | 800 |
| dracotail | 109.6 | 37 | 140 | 104 | 23 | 58 | 1 | 38 | 20 | 2 | 42 | 840 |
| earth_machine | 190.2 | 50 | 458 | 243 | 94 | 111 | 83 | 164 | 6 | 3 | 71 | 1420 |
| elfnote | 35.9 | 37 | 76 | 69 | 25 | 38 | 6 | 7 | 0 | 16 | 40 | 800 |
| elfnote_kewl_tune | 76.2 | 37 | 120 | 106 | 37 | 51 | 17 | 8 | 7 | 5 | 41 | 820 |
| enneacraft | 89.1 | 40 | 156 | 66 | 56 | 90 | 10 | 0 | 0 | 17 | 51 | 1020 |
| exodia | 58.3 | 50 | 38 | 32 | 15 | 13 | 4 | 5 | 1 | 24 | 66 | 1261 |
| fur_hire | 47.8 | 40 | 132 | 102 | 26 | 65 | 1 | 39 | 1 | 16 | 51 | 1020 |
| gimmick_puppet | 40.1 | 28 | 101 | 56 | 17 | 46 | 32 | 2 | 4 | 2 | 34 | 640 |
| insect | 46.9 | 50 | 101 | 74 | 26 | 27 | 32 | 16 | 0 | 19 | 66 | 1320 |
| invoked | 36.0 | 36 | 62 | 52 | 9 | 37 | 0 | 3 | 13 | 11 | 38 | 760 |
| k9 | 33.8 | 36 | 58 | 47 | 25 | 21 | 9 | 1 | 2 | 15 | 40 | 794 |
| kaiju | 107.0 | 36 | 46 | 31 | 8 | 17 | 2 | 19 | 0 | 19 | 41 | 780 |
| labrynth | 75.0 | 39 | 29 | 26 | 9 | 18 | 0 | 2 | 0 | 20 | 45 | 900 |
| live_twin | 38.8 | 35 | 67 | 53 | 10 | 31 | 4 | 11 | 11 | 13 | 38 | 760 |
| lunalight | 641.6 | 38 | 154 | 122 | 36 | 50 | 32 | 19 | 17 | 13 | 46 | 881 |
| maliss | 31.3 | 32 | 18 | 18 | 11 | 1 | 6 | 0 | 0 | 17 | 35 | 682 |
| odion | 53.6 | 35 | 9 | 9 | 9 | 0 | 0 | 0 | 0 | 29 | 42 | 840 |
| orcust | 23.0 | 38 | 83 | 76 | 18 | 14 | 27 | 10 | 14 | 10 | 40 | 800 |
| phantom_knights_orcust | 40.8 | 39 | 131 | 103 | 27 | 34 | 46 | 12 | 12 | 7 | 43 | 824 |
| power_patron | 27.5 | 34 | 89 | 79 | 23 | 55 | 4 | 5 | 2 | 10 | 36 | 720 |
| r_b | 29.6 | 37 | 129 | 109 | 26 | 48 | 26 | 13 | 16 | 9 | 47 | 940 |
| radiant_typhoon_zoodiac | 31.3 | 36 | 74 | 66 | 32 | 21 | 0 | 15 | 6 | 14 | 40 | 800 |
| ryzeal_mitsurugi | 30.5 | 42 | 128 | 89 | 71 | 36 | 0 | 19 | 2 | 16 | 48 | 866 |
| sky_striker | 25.7 | 37 | 104 | 79 | 25 | 24 | 8 | 18 | 29 | 15 | 43 | 860 |
| synchrons | 58.2 | 41 | 107 | 95 | 35 | 51 | 14 | 2 | 5 | 7 | 47 | 910 |
| tearlaments | 27.3 | 36 | 145 | 120 | 19 | 17 | 61 | 41 | 7 | 5 | 38 | 760 |
| tenpai_dragon | 26.9 | 34 | 29 | 22 | 15 | 14 | 0 | 0 | 0 | 26 | 37 | 740 |
| vanquish_soul_k9 | 43.4 | 39 | 86 | 77 | 43 | 29 | 6 | 2 | 6 | 11 | 46 | 920 |
| white_forest_azamina | 27.8 | 41 | 76 | 68 | 41 | 18 | 8 | 8 | 1 | 16 | 48 | 960 |
| yubel | 19.6 | 34 | 66 | 49 | 21 | 27 | 10 | 2 | 6 | 11 | 37 | 738 |
| yummy | 26.6 | 36 | 68 | 53 | 15 | 29 | 7 | 15 | 2 | 10 | 39 | 780 |

- Tempo: mediana 40 s, 9 deck sopra il minuto (lunalight 642 s, earth_machine 190 s, dracotail 110 s, kaiju 107 s).
- 4110 collegamenti; 193 partono dalla mano (Evocare dalla mano), soprattutto enneacraft (90).
- Carte dell'Extra Deck con `chance` 0: 392 su 544 (72%).
- Carte del Main Deck con `chance` ≥ 95%: 158 su 990, concentrate in branded (34 su 46), cyber_dragon (15 su 24), doomz (14 su 21), lunalight (12 su 26).

### Turno di prova contro un deck
Prima board di ogni deck. "Campo" e "Resilienza generica" sono la misura standard. Per ogni avversario: resilienza contro di lui, e probabilità che abbia una handtrap che ferma la linea, prima → dopo le correzioni A+B+C/D.

| Deck tuo | Campo | Resilienza generica | kaiju: res. · fermata prima→dopo | labrynth: res. · fermata prima→dopo | maliss: res. · fermata prima→dopo | invoked: res. · fermata prima→dopo | sky_striker: res. · fermata prima→dopo |
|---|---|---|---|---|---|---|---|
| branded | 0.35 | 0.29 | 0.30 · 43→0% | 0.29 · 92→90% | 0.29 · 98→88% | 0.29 · 94→78% | 0.29 · 82→64% |
| tearlaments | 0.44 | 0.14 | 0.13 · 43→0% | 0.14 · 92→90% | 0.14 · 98→88% | 0.14 · 94→78% | 0.14 · 82→64% |
| yubel | 0.78 | 0.35 | 0.38 · 0→0% | 0.35 · 85→85% | 0.35 · 88→88% | 0.35 · 78→78% | 0.35 · 64→64% |
| ryzeal_mitsurugi | 1.50 | 1.08 | 1.14 · 0→0% | 1.08 · 90→90% | 1.08 · 88→80% | 1.08 · 78→78% | 1.08 · 64→64% |
| k9 | 0.60 | 0.18 | 0.26 · 43→0% | 0.18 · 85→82% | 0.18 · 98→88% | 0.18 · 94→74% | 0.18 · 82→64% |
| white_forest_azamina | 2.26 | 1.47 | 0.92 · 0→0% | 1.47 · 90→90% | 1.47 · 88→88% | 1.47 · 78→78% | 1.47 · 64→64% |
| dracotail | 0.40 | 0.11 | 0.12 · 43→12% | 0.11 · 92→88% | 0.11 · 98→91% | 0.11 · 94→74% | 0.11 · 82→58% |
| blue_eyes | 1.29 | 0.40 | 0.35 · 43→43% | 0.40 · 85→85% | 0.40 · 91→91% | 0.40 · 88→88% | 0.40 · 69→69% |
| synchrons | 1.64 | 0.88 | 1.16 · 0→0% | 0.88 · 90→90% | 0.88 · 88→80% | 0.88 · 78→78% | 0.88 · 64→64% |
| fur_hire | 0.23 | 0.19 | 0.19 · 43→0% | 0.19 · 92→90% | 0.19 · 96→80% | 0.19 · 94→78% | 0.19 · 82→64% |
| orcust | 0.49 | 0.40 | 0.42 · 0→0% | 0.40 · 0→0% | 0.40 · 0→0% | 0.40 · 0→0% | 0.40 · 0→0% |
| yummy | 0.23 | 0.19 | 0.19 · 0→0% | 0.19 · 78→78% | 0.19 · 88→88% | 0.19 · 69→69% | 0.19 · 51→51% |

- Tempo della misura contro un deck: mediana 0,6 s per board, 90% sotto 1,6 s, massimo 2,6 s. Tutto `probe-line --vs` con 3 board: da 1,2 a 10 s.
- orcust: la mano estratta non fa niente (la board è solo carte coperte), quindi nessun momento per le handtrap.
- blue_eyes: Ash Blossom ferma 0 momenti su 4, ed è giusto: la linea usa Crossout Designator dichiarando Ash Blossom, che nega le Ash dell'avversario per il turno.

**Fermate per handtrap, sulle 180 board** (momenti fermati / momenti in cui si può usare):

| Handtrap | Prima | A+B | A+B+C/D |
|---|---|---|---|
| Maxx "C" | 550/3955 | 305/3955 | 30/3955 |
| Mulcharmy Fuwalos | 388/3164 | 136/3164 | 12/3164 |
| Mulcharmy Purulia | 178/1582 | 112/1582 | 6/1582 |
| Ash Blossom & Joyous Spring | 592/640 | 592/640 | 592/640 |
| Ghost Ogre & Snow Rabbit | 414/450 | 414/450 | 414/450 |
| Nibiru, the Primal Being | 303/303 | 303/303 | 303/303 |
| Dominus Impulse | 298/298 | 298/298 | 298/298 |
| Dominus Spark | 442/456 | 442/456 | 436/456 |
| Droll & Lock Bird | 216/309 | 216/309 | 213/309 |
| Ghost Belle & Haunted Mansion | 142/146 | 142/146 | 142/146 |
| Songs of the Dominators | 139/142 | 139/142 | 139/142 |
| Ghost Mourner & Moonlit Chill | 112/193 | 108/193 | 108/193 |
| Bystial Druiswurm | 257/431 | 239/431 | 205/431 |
| Bystial Magnamhut | 254/431 | 236/431 | 202/431 |
| Effect Veiler | 163/617 | 130/617 | 118/617 |
| Infinite Impermanence | 133/681 | 124/681 | 121/681 |

- Le correzioni tolgono quasi solo fermate false: Ash, Ogre, Nibiru, Dominus, Belle restano uguali.
- Le fermate di Bystial, Veiler e Droll che spariscono le ho controllate a campione: erano sfasamenti di indice o di forma (vedi Alta 1). Esempio: Droll al momento 12 della linea dracotail "fermava" perché in Fase Finale il trigger di Blazing Cartesia arrivava come `EffectYesNo` invece che in una `SelectChain`.

## Problemi in ordine di gravità
I comandi usano `<mdm>` per la cartella dei deck e `<out>` per quella dei risultati.

### Alta: risultati sbagliati

#### A1. Turno di prova: Maxx "C" e Mulcharmy fermano la linea in 6 deck su 12
- **Riprodurre:**
  ```
  python3 scripts/matchup_report.py <out> --decks <mdm> --own tearlaments,k9,fur_hire --vs kaiju
  DUELFORGE_DEBUG_MATCHUP=1 duelforge-cli probe-line --deck <mdm>/tearlaments.ydk --lines <out>/tearlaments.lines.json --vs <mdm>/kaiju.ydk --index 0
  ```
- **Atteso:** 0 momenti; fanno solo pescare l'avversario.
- **Esce:** Maxx "C" ferma 550 momenti su 3955 (14%). Per deck: fur_hire 12 su 19, tearlaments 7 su 32, k9 5 su 23, blue_eyes 2 su 27, branded 1 su 23, dracotail 1 su 28. Contro kaiju (Maxx "C" e 3 Fuwalos) la probabilità di essere fermati è 43% invece di 0.
- **Cause, tutte in `matchup.rs`:**
  1. **Deck dell'avversario di 12 carte** (`hand_trap_stops` usa `kit.opponent().deck`). Maxx "C" lo svuota in poche Evocazioni. Tearlaments: Pilgrim Reaper ("each player sends the top 5 cards of their Deck") non si attiva più e la linea si sfasa alla risposta 72.
  2. **Domande nuove sì/no.** Maxx "C" e Mulcharmy vanno al Cimitero dalla mano come costo. Necroquip Princess ("If a monster(s) is sent from the hand to the GY to activate a card or effect") riceve un trigger nuovo (`EffectYesNo`). Branded: dopo una catena in Fase Finale il motore chiede di nuovo un `IdleCmd` con solo "End Phase". `replay` risponde da solo solo alle finestre `SelectChain` vuote; qui segna "diverged".
  3. **Risposte per indice.** Dopo la handtrap la lista delle azioni cambia, e lo stesso indice sceglie un'altra carta. K9: dopo Maxx "C" si può Evocare K9-17 Izuna ("if your opponent has activated a monster effect in the hand"), e la risposta `[5,0,0,0]` dell'`IdleCmd` sceglie Izuna invece di Chaotic Elements.
  4. **La stessa scelta in un'altra forma.** Fur Hire: la pescata di Maxx "C" dà un trigger a Rex, Ride Fur Hire ("If a card is added from the Deck to your opponent's hand"). Il trigger di Rex, Freight Fur Hire, che nella linea era un `EffectYesNo`, ora sta in una `SelectChain` con due trigger.
  - Le cause 2-4 sono interazioni vere del gioco (K9 e Fur Hire puniscono Maxx "C"), ma la linea si può ancora giocare uguale e la board finale non cambia.
- **Correzioni proposte:**
  - **A.** In `hand_trap_stops` l'avversario ha un deck di dimensione vera: le carte del kit più carte semplici fino a 35, messe in fondo.
  - **B.** In `replay`, dopo la handtrap: una domanda di tipo diverso da quella della linea, con una risposta passiva (`EffectYesNo`, `YesNo`, `SelectChain`, `IdleCmd` con solo la fine del turno), riceve la risposta passiva. La risposta della linea resta per la domanda dopo. "diverged" solo se non c'è una risposta passiva.
  - **C/D.** `own_answers` salva per ogni risposta cosa fa: carta (codice originale), zona, descrizione dell'effetto, tipo di azione (`Action.kind`). In `replay`, dopo la handtrap, si cerca nella lista nuova l'azione con la stessa identità e si usa la sua risposta. Un `EffectYesNo` con sì vale come l'attivazione della stessa carta in una `SelectChain` (lì la descrizione non coincide: si confronta solo carta e zona). Se l'azione non c'è più, la linea è fermata.
  - Il tipo di azione serve: la prima prova senza ha scelto "Evoca" al posto di "Posiziona" e ha peggiorato tutto (k9 da 5 a 19 momenti).
- **Numeri:** Maxx "C" 550 → 30, Fuwalos 388 → 12, Purulia 178 → 6 (tabella sopra). Restano 2 casi da capire:
  - blue_eyes, 1 momento su 27: dopo Maxx "C", attivando Junoldo the Shadespirit Power Patron il motore non apre la finestra in cui la linea concatenava Elfnote Regina;
  - dracotail, 1 su 28: Maxx "C" usato in Fase Finale, e il trigger di Fase Finale di Blazing Cartesia non viene più offerto.

#### A2. Mappa: la causa di un trigger facoltativo è la carta dell'ultima azione
- **Riprodurre:** `duelforge-cli links --deck <mdm>/branded.ydk`
- **Atteso:** Branded in High Spirits non Evoca niente ("Reveal 1 monster ... send 1 Level 8 Fusion Monster from your Extra Deck to the GY").
- **Esce:** "Branded in High Spirits → summon grave" verso 9 mostri (Fallen of Albaz, Incredible Ecclesia...), e "Alba-Lenatus the Abyss Dragon → summon" verso 3.
- **Causa:** in `links.rs`, `play` aggiorna `reader.acting` solo per `IdleCmd` e `SelectChain`. Guiding Quem si attiva con un `EffectYesNo` ("If a card(s) leaves the Extra Deck"). Il suo bersaglio nel Cimitero (`SelectCard`, suggerimento 509) va alla carta dell'ultima azione, High Spirits.
- **Correzione:** rispondendo sì a un `EffectYesNo`, `acting` diventa la carta di quel trigger.
- **Prova:** con `QA_FIX_C` sul deck Branded spariscono esattamente quei 12 collegamenti, nessun altro cambia (394 → 382).
- Stesso schema, da ricontrollare dopo la correzione:
  - Haggard Lizardose e S:P Little Knight "search" Bystial Druiswurm (maliss);
  - Infinitrack Goliath e Superdreadnought Rail Cannon Juggernaut Liebe "recover" (earth_machine);
  - Moon of the Closed Heaven e Fiendsmith's Agnumday "recover" (kaiju);
  - Deception of the Sinful Spoils "summon grave" (chimera);
  - Branded Banishment "search" Starliege Seyfert (branded).

#### A3. Mappa: i materiali di R.B. Shepherd's Crook
- **Riprodurre:** `duelforge-cli links --deck <mdm>/r_b.ydk`
- **Atteso:** materiali nominati solo dai mostri "R.B." ("2+ monsters, including an "R.B." monster").
- **Esce:** materiale anche da Clockwork Knight, Mulcharmy Fuwalos, Night Train Blue Traveler.
- **Causa:** `Names::new` taglia la riga dei materiali al primo punto (`split(['.', '\n'])`). Resta `including an "r`, quindi l'archetipo citato è "r", e quasi ogni nome lo contiene.
- **Correzione:** per i mostri dell'Extra Deck la riga dei materiali è la prima riga del testo: tagliare solo a `\n`. Tra i 40 deck è l'unico caso con un punto dentro le virgolette. Lo stesso problema avrebbero "D.D.", "U.A." e simili.

#### A4. Il deck dell'avversario del kit ha 12 carte (ricerca e mappa)
- That Grass Looks Greener manda carte al Cimitero finché il Deck è grande come quello dell'avversario. In goldfish ne manda 43 su 55 (deck Branded da 60); in partita, contro un deck da 35, ne manderebbe 20.
  ```
  duelforge-cli combos --deck <mdm>/branded.ydk --hand "That Grass Looks Greener;Droll & Lock Bird;Maxx \"C\";Pot of Sloth;Nadir Servant" --nodes 800 --threads 1 --top 1 --lines
  ```
- Sulla mappa: "That Grass Looks Greener → toGrave" verso 38 carte, ognuna "usata" nel 22-24% delle linee. È anche la causa principale della `chance` satura di Branded (vedi M4).
- Vale per ogni effetto che guarda il deck dell'avversario: Pilgrim Reaper (A1), le pescate di Maxx "C".
- **Correzione:** `Kit::standard` (`probe.rs`) riempie il deck dell'avversario di carte semplici fino a 35, in fondo, così le carte da cercare restano. Tocca la ricerca, quindi va misurata con il protocollo (bench prima e dopo).

#### A5. Turno di prova: breaker falsi tolgono quelli standard
- **Riprodurre:**
  ```
  duelforge-cli probe-line --deck <mdm>/branded.ydk --lines <out>/branded.lines.json --vs <mdm>/cyber_dragon.ydk --index 0
  ```
- **Atteso:** cyber_dragon non ha breaker nel Main Deck: si usano quelli standard, resilienza 0,29 come la generica.
- **Esce:** "breakers: Clockwork Night", che toglie 500 ATK e non distrugge niente. La resilienza contro cyber_dragon diventa 0,35 (tutto il campo), più alta di quella generica.
- Altri breaker falsi trovati sui 40 deck:
  - Argostars - Home Stadium;
  - Clown Crew Cappello;
  - Plundered Power Patron Plane - Vidolia (doomz, power_patron);
  - R.B. Ga10 Pile Bunker (distrugge solo in battaglia);
  - Yummyusment★Acroquey.
  - Il testo li prende ("negate its effects", "shuffle all", "lose ATK"...). `playable` li accetta perché le Magie Continue e di Terreno si attivano dalla mano.
- **Correzione:** in `playable`, giocare ogni candidato contro il campo finto (3 mostri, 2 carte coperte) e tenerlo solo se quel campo perde qualcosa. Più semplice: in `matchup`, se tutti i breaker dell'avversario lasciano la board intera, usare quelli standard (`standard_breakers = true`).

### Media: collegamenti, breaker o handtrap mancanti

#### M1. Turno di prova: breaker veri non riconosciuti (deck kaiju)
- Interrupted Kaiju Slumber ("Destroy as many monsters on the field as possible") e Ultimate Slayer ("target 1 monster your opponent controls ...; shuffle it into the Deck") non sono tra i breaker.
- **Cause:**
  - `breakers_by_text` (`roles.rs`) non conosce "as many ... as possible" né "shuffle ... into the Deck";
  - `playable` dà all'avversario il deck del kit e un Extra Deck vuoto: Ultimate Slayer ("Send 1 monster from your Extra Deck to the GY") non si può attivare. Lo stesso vale nella partita del breaker (`Kit::with_breakers` aggiunge solo la carta alla mano).
- **Correzione:** aggiungere le due frasi al testo dei breaker; in `playable` e nel kit con i breaker dell'avversario usare il suo Main Deck e il suo Extra Deck veri.
- Effetto sulla misura: contro kaiju la resilienza è ottimista. white_forest_azamina scende già da 1,47 a 0,92 con i Kaiju; con Slumber scenderebbe ancora.

#### M2. Mappa: "Posiziona dal Deck" non è un collegamento
- Il confronto con il testo segnala 34 carte che mettono una Magia o Trappola dal Deck sul campo (qualcuna per errore della regex, come Urgent Schedule). La mappa non vede questo passo. Esempi controllati:
  - Dracotail: Phryxul, Mululu, Lukias, Faimena, Urgula, Pan ("Set 1 "Dracotail" Spell/Trap from your Deck");
  - Virakam ("Set 1 "Invocation" ... from your Deck");
  - Albion the Branded Dragon, Keel (Fur Hire), Artmage Graflare, Diabellstar, Triple Tactics Thrust, "A Case for K9", Treasures of the Kings;
  - Elfnote Tinia ("Place 1 "Elfnote" Continuous Spell from your Deck face-up on your field"): la usa la prima linea blue_eyes, e sulla mappa non ha nessun collegamento.
- **Correzione:** in `Reader::see`, il movimento Deck → zona Magie e Trappole (o Terreno) diventa un tipo nuovo, `set` ("porta dal Deck sul campo"). In `offered`, il suggerimento 510 (HINTMSG_SET) dal Deck. Conta come "porta in gioco" per `reachAll` e `chance`.
- Tocca il contratto `DeckLinks` (`lib.rs` ↔ `api.ts`): un tipo in più.

#### M3. Mappa: i materiali di Fusione mandati dal Deck non sono "manda al Cimitero"
- **Riprodurre:** `duelforge-cli links --deck <mdm>/branded.ydk`. Branded Fusion ("using 2 monsters from your hand, Deck, or field as Fusion Material") ha solo collegamenti "summon".
- **Causa:** in `Reader::see` un movimento con `reason::MATERIAL` va solo in `used_materials` e non diventa mai `ToGrave`.
- **Correzione:** se il materiale parte dal Deck, aggiungere anche `(causa, carta, ToGrave, Deck)`.

#### M4. Mappa: la `chance` si satura e l'Extra Deck resta a 0
- **Saturazione:** Branded ha 34 carte del Main Deck su 46 con `chance` ≥ 95%. `brings` conta come "porta in gioco" anche `ToGrave` dal Deck. Così Foolish Burial e That Grass Looks Greener "portano" quasi tutto il deck, e tutto il deck porta a tutto.
  - **Correzione:** per `reachAll` e `chance` contare solo cerca, Evoca, posiziona (M2) e materiale. Oppure mostrare due probabilità: "in mano o in campo" e "nel Cimitero".
- **Extra Deck a 0:** 392 mostri dell'Extra Deck su 544 hanno `chance` 0 (Synchro e Link con materiali generici), anche quando sono in quasi tutte le linee (Ecclesia and the Dark Dragon nelle linee Branded).
  - È un limite scritto nel piano, ma riguarda il 72% delle carte dell'Extra Deck.
  - **Proposta:** per questi mostri, la frazione (pesata) delle mani cercate in cui una linea li Evoca, con una nota "misurata, non esatta".

#### M5. Mappa: rimescolare nel Deck conta come "recupera"
- The Dragon that Devours the Dogma ("shuffle up to 2 cards from any GY(s) and/or banishment into the Deck") e Ecclesia and the Dark Dragon ("shuffle both of those cards and this card into the Deck") risultano "recover" verso 16 e 3 carte.
- **Causa:** `Recover` comprende Cimitero → Deck.
- **Correzione:** `Recover` solo verso la mano. Cimitero → Deck non è un collegamento (o è un tipo a parte, non contato nella raggiungibilità).

### Bassa: tempi e cose da chiarire

#### B1. Tempo della mappa
- Mediana 40 s, ma 9 deck su 40 sopra il minuto: lunalight 642 s, earth_machine 190 s, dracotail 110 s, kaiju 107 s. L'obiettivo del piano è sotto il minuto.
- Lunalight da solo, senza altri processi: 502 s, quindi il tempo è reale e non dovuto al carico.
- La causa è la ricerca, non la mappa: con 1000 nodi su un thread la mano Lunalight Gold Leo, Forbidden Droplet, Lunalight Black Sheep, Nibiru, Heavy Polymerization va a 25 nodi al secondo (40 s); altre mani dello stesso deck vanno a 220-450 nodi al secondo. Da passare alla sessione sulla velocità della ricerca.
- **Proposta:** un tempo massimo per mano più basso (`time_limit` è 60 s per mano) e un tempo totale; oppure meno mani per le carte che compaiono già in molte mani cercate.

#### B2. `--nodes` e `--hands` più alti
Misurati da soli (nessun altro processo), 4 thread. "Nuovi" e "persi": collegamenti rispetto ai valori predefiniti (1000 nodi, 2 mani).

| Deck | Nodi · mani | Tempo (s) | Collegamenti | Usati | Nuovi | Persi |
|---|---|---|---|---|---|---|
| invoked | 1000 · 2 | 22,6 | 62 | 52 | 0 | 0 |
| invoked | 3000 · 2 | 55,0 | 71 | 56 | 11 | 2 |
| invoked | 1000 · 4 | 41,9 | 77 | 66 | 15 | 0 |
| yubel | 1000 · 2 | 21,0 | 66 | 49 | 0 | 0 |
| yubel | 3000 · 2 | 44,9 | 72 | 60 | 11 | 5 |
| yubel | 1000 · 4 | 43,6 | 82 | 69 | 16 | 0 |
| k9 | 1000 · 2 | 27,3 | 58 | 47 | 0 | 0 |
| k9 | 3000 · 2 | 62,0 | 57 | 45 | 0 | 1 |
| k9 | 1000 · 4 | 50,9 | 58 | 50 | 0 | 0 |

- Più mani rendono più di più nodi, e costano meno: +15 e +16 collegamenti con 4 mani, +11 con 3000 nodi, che però ne perde qualcuno (le 20 linee migliori cambiano).
- I collegamenti nuovi sono veri: su invoked compare la Fusione di Aiwass, Divine Spirit of the Law ("you can Fusion Summon 1 "Invoked" monster", Mechaba, Baybarron...), che con 2 mani mancava. Quasi tutti hanno `used` vicino a 0.
- Su k9 la mappa è già completa con 2 mani.
- **Proposta:** 3 mani per carta e 1000 nodi come valori predefiniti, se il tempo massimo (B1) tiene il caso peggiore sotto il minuto.

#### B3. Cose che sembrano errori ma non lo sono
- Ash Blossom ferma 0 momenti nelle linee blue_eyes: la linea gioca Crossout Designator su Ash Blossom.
- Predaplant Verte Anaconda "summon extra" e Transaction Rollback "summon"/"search": copiano l'effetto di un'altra carta, quindi la causa è giusta.
- Subspace Battle "toGrave" dal Deck: il mostro rivelato con meno ATK viene distrutto dal Deck.
- Molti "mancanti" dei deck con trappole (odion, labrynth) o con effetti nel turno dell'avversario: la ricerca gioca solo il primo turno.
- Super Polymerization e Fallen of Albaz senza "summon extra": servono mostri dell'avversario, e in goldfish non ce ne sono.

#### B4. Uscita della CLI
- `probe-line --vs` stampa "Lightning Storm" due volte tra i breaker (le due opzioni): basta il nome con l'opzione, come nel kit.
- Nel JSON di `probe-line` la misura contro il deck non ha il tempo: per le tabelle l'ho letto dall'output di testo.

## Dopo le correzioni della coordinatrice (commit `d5af0ba`, PR #23)
Stesse 180 board, rimisurate con il binario di `d5af0ba` (la A, deck dell'avversario di 35 carte, non c'è ancora). Fermate di Maxx "C", Fuwalos e Purulia sommate:

| Versione | Fermate | Deck con fermate (fermate) |
|---|---|---|
| prima (`7c0e640`) | 1116 | fur_hire 467, k9 396, tearlaments 106, blue_eyes 66, dracotail 48, branded 33 |
| copia sperimentale A+B+C/D | 48 | blue_eyes 33, dracotail 15 |
| `d5af0ba` | 385 | synchrons 99, k9 66, blue_eyes 66, tearlaments 62, branded 33, white_forest_azamina 33, dracotail 15, fur_hire 11 |
| `d5af0ba` + le due correzioni sotto | 121 | tearlaments 40, k9 33, synchrons 33, dracotail 15 |

- Fur_hire e k9 migliorano molto. Ma synchrons e white_forest_azamina, che prima avevano 0 fermate, ora ne hanno: sono due regressioni.
- Le fermate vere non cambiano: Ash 592, Ogre 414, Nibiru 303, Dominus 298 e 442.

### R1. Domande della linea rifiutate durante la risoluzione (regressione)
- **Riprodurre:**
  ```
  DUELFORGE_DEBUG_MATCHUP=1 duelforge-cli probe-line --deck <mdm>/synchrons.ydk --lines <out>/synchrons.lines.json --vs <mdm>/kaiju.ydk --index 0
  ```
  Maxx "C" al momento 8 risulta fermare la linea: "diverged at answer 23".
- **Causa:** in `replay` la condizione `declinable(&decision) && (theirs_on_top || !same_question)`. `theirs_on_top` resta vero da quando l'avversario concatena fino a `ChainEnd`, quindi per tutta la risoluzione. La `YesNo` dell'effetto della linea che si risolve sotto Maxx "C" ("then you can...", attesa proprio lì) viene rifiutata. Prima `theirs_on_top` valeva solo per le finestre `SelectChain`.
- **Correzione:** `theirs_on_top` solo per le `SelectChain`, come prima. Oppure azzerarlo a `ChainSolving`.

### R2. I passaggi della linea si saltano dopo il rifiuto
- In `replay` il ciclo "Passes of the line the engine no longer asks for are left out" viene dopo il rifiuto delle domande nuove. Se la linea aveva un passaggio in una finestra che non c'è più, la domanda dopo (la `YesNo` della linea) ha un tipo diverso da `own.kinds[next]` e viene rifiutata.
- **Correzione:** saltare i passaggi prima del rifiuto.
- **Numeri:** con R1 e R2 insieme (provate nella copia, dietro variabili d'ambiente) si passa da 385 a 121 fermate. white_forest_azamina, blue_eyes e branded tornano a 0, synchrons da 99 a 33.

### Cosa resta
- **tearlaments, 40:** è la A, il deck dell'avversario di 12 carte (Pilgrim Reaper). Con la A la copia sperimentale arrivava a 0.
- **k9 e synchrons, 33 ciascuno:** la linea aspetta una scelta di carte (`SelectCard`, risposta da 12 byte) che dopo Maxx "C" non arriva, e il motore chiede un `IdleCmd`. In `d5af0ba` l'identità copre solo `IdleCmd`, `SelectChain` ed `EffectYesNo`; le scelte di carte restano per indice. Se la lista cambia (Maxx "C" nel Cimitero dell'avversario per un effetto su "either GY", una carta in più in mano), l'indice sceglie un'altra carta e la linea si sfasa più avanti. La copia sperimentale copriva anche le scelte di una carta sola, e lì k9 e synchrons erano a 0. **Proposta:** estendere l'identità alle `SelectCard` di una carta (e alle multiple, carta per carta).
- **dracotail, 15:** Maxx "C" usato in Fase Finale, poi il trigger di Fase Finale di Blazing Cartesia non viene più offerto. Era già così nella copia sperimentale: da capire nel motore.

### La mappa con `d5af0ba`
Rimisurata su 6 deck (branded, r_b, maliss, earth_machine, kaiju, chimera):
- **Collegamenti strani** (segnalati dal testo): da 71 a 27.
  - Spariti: Branded in High Spirits, Infinitrack Goliath, Superdreadnought Rail Cannon Juggernaut Liebe, Vernusylph of the Thawing Mountains, Fiendsmith's Agnumday, Moon of the Closed Heaven, Deception of the Sinful Spoils.
  - Collegamenti in meno: branded 394 → 382, earth_machine 458 → 342, kaiju 46 → 39, chimera 165 → 156.
- **Restano:**
  - Transaction Rollback e Subspace Battle: sono giusti (B3).
  - Haggard Lizardose e S:P Little Knight "search" Bystial Druiswurm, Branded Banishment "search" Starliege Seyfert (`used` sotto l'1%). Accanto c'è il collegamento vero, Bystial Magnamhut "search", quindi è sempre la ricerca di Magnamhut nella Fase Finale, data alla carta dell'ultima azione. Forse quel trigger non passa né da un `EffectYesNo` né da una `SelectChain` (attivazione automatica). Da controllare con una traccia in `Reader::see`.
- **A3 (materiali di R.B. Shepherd's Crook):** c'è ancora; Clockwork Knight, Mulcharmy Fuwalos e Night Train Blue Traveler sono ancora materiali.

## Terzo giro: commit `eedda66` (PR #25)
Stesse 180 board e stessa mappa su 6 deck, con il binario di `eedda66`. Le prove delle correzioni in più sono in una copia nello scratchpad, dietro variabili d'ambiente.

### Turno di prova contro un deck
Fermate di Maxx "C", Fuwalos e Purulia sommate:

| Versione | Fermate false | Per deck |
|---|---|---|
| prima (`7c0e640`) | 1116 | fur_hire 467, k9 396, tearlaments 106, blue_eyes 66, dracotail 48, branded 33 |
| `d5af0ba` | 385 | (vedi sopra) |
| `eedda66` | 81 | k9 33, synchrons 33, dracotail 15 |
| `eedda66` + R3 + R4 (sotto) | **0** | — |

Fermate per handtrap (momenti fermati / momenti in cui si può usare):

| Handtrap | Prima | `eedda66` | `eedda66` + R3 + R4 |
|---|---|---|---|
| Maxx "C" | 550/3955 | 45/3955 | 0/3955 |
| Mulcharmy Fuwalos | 388/3164 | 24/3164 | 0/3164 |
| Mulcharmy Purulia | 178/1582 | 12/1582 | 0/1582 |
| Labrynth Chandraglier | — | 9/791 | 0/791 |
| Ash Blossom & Joyous Spring | 592/640 | 592/640 | 592/640 |
| Ghost Ogre & Snow Rabbit | 414/450 | 414/450 | 414/450 |
| Nibiru, the Primal Being | 303/303 | 303/303 | 303/303 |
| Dominus Impulse | 298/298 | 298/298 | 298/298 |
| Dominus Spark | 442/456 | 437/456 | 436/456 |
| Droll & Lock Bird | 216/309 | 213/309 | 213/309 |
| Ghost Belle & Haunted Mansion | 142/146 | 142/146 | 142/146 |
| Songs of the Dominators | 139/142 | 143/146 | 143/146 |
| Ghost Mourner & Moonlit Chill | 112/193 | 108/193 | 105/193 |
| Bystial Druiswurm | 257/431 | 130/431 | 127/431 |
| Bystial Magnamhut | 254/431 | 127/431 | 127/431 |
| Effect Veiler | 163/617 | 118/617 | 115/617 |
| Infinite Impermanence | 133/681 | 118/681 | 115/681 |

- Le fermate vere restano: Ash, Ogre, Nibiru, Dominus, Belle uguali; Bystial, Veiler e Impermanence quasi uguali a `eedda66`.
- Labrynth Chandraglier è nuova tra le handtrap (setta una Trappola "Labrynth"): non deve fermare niente, e con R3 e R4 non ferma niente.

#### R3. `find_again` ripiega su un altro effetto della stessa carta (k9, synchrons: 66)
- **Riprodurre:**
  ```
  DUELFORGE_DEBUG_MATCHUP=1 duelforge-cli probe-line --deck <mdm>/k9.ydk --lines <out>/k9.lines.json --vs <mdm>/kaiju.ydk --index 0
  ```
  Maxx "C" al momento 12: "lost K9-17 Izuna, Vallon...".
- **Causa:**
  - Maxx "C" si attiva quando la linea sta per attivare K9-17 "Ripper" (effetto di ricerca, descrizione `...048`). Nella `SelectChain` di risposta a Maxx "C" Ripper offre solo l'altro effetto, "When your opponent activates a monster effect in the hand or GY: negate that effect" (`...049`).
  - `find_again` non trova la descrizione e prende `offered.first()`, l'effetto di negazione: Ripper nega Maxx "C", spende il materiale, e la linea si sfasa.
- **Correzione provata:** ripiegare su un altro effetto della stessa carta solo quando l'azione attesa viene da un `EffectYesNo` (lì la descrizione della domanda non è quella dell'effetto). In tutti gli altri casi serve la stessa descrizione.
- Togliere del tutto il ripiego non basta: Fur Hire torna a 12 fermate (il trigger di Rex, Freight in una `SelectChain`), e Labrynth Chandraglier ne fa 24 su white_forest_azamina.

#### R4. I passaggi della linea finiscono su finestre in cui c'è la sua azione (dracotail: 15)
- **Riprodurre:** `probe-line --deck <mdm>/dracotail.ydk ... --vs <mdm>/kaiju.ydk --index 0`, Maxx "C" al momento 25: "lost Blazing Cartesia".
- **Causa:**
  - Maxx "C" si concatena al trigger di Fase Finale di Incredible Ecclesia, e la sua catena occupa una delle finestre della Fase Finale.
  - Le due risposte "passo" della linea (80, 81) cadono su finestre in cui il trigger di Blazing Cartesia è già offerto. Si passa, e la Fase Finale finisce senza Cartesia.
- **Correzione provata:** quando la risposta della linea è un passo, si guarda la prima risposta dopo i passi. Se quell'azione è già offerta nella domanda di adesso, si gioca lei e si saltano i passi.

#### `--stops` (dove fermare la linea contro Maxx "C")
Provato sulla prima board di tearlaments, branded, k9 e white_forest_azamina contro kaiju.
- **Il punto migliore è sempre la linea intera.** Mi sembra sensato: in queste linee la board si chiude solo alla fine.
  - branded: campo 0 fino alla risposta 61, poi 0,35;
  - white_forest_azamina: 0,82 alla 64, 2,26 alla fine;
  - tearlaments: 0,48 già alla 18, ma senza le carte che bloccano (0,30); la fine vale 0,44 + 0,30.
- **Tempo:** 3,6-7,3 s per handtrap e per board (due handtrap su kaiju: 7-15 s). Va bene a richiesta, non per ogni board del Combo Lab.
- **Parità:** a parità di valore atteso e di carte pescate (branded: 70, 73 e 82 valgono tutte 0,22 con 9 carte pescate) si sceglie l'ultimo punto. Proporrei il primo: meno azioni, stesso risultato.
- La riga "drawn by moment" (carte pescate se la handtrap arriva in quel momento) è chiara e utile per l'interfaccia.

### Mappa con `eedda66` (6 deck)

| Deck | Collegamenti (`7c0e640` → `d5af0ba` → `eedda66`) | Di cui "posiziona" | Tempo (s) |
|---|---|---|---|
| branded | 394 → 382 → 402 | 18 | 92 → 90 → 62 |
| r_b | 129 → 123 → 151 | 1 | 30 → 32 → 23 |
| maliss | 18 → 18 → 17 | 1 | 31 → 19 → 13 |
| earth_machine | 458 → 342 → 311 | 0 | 190 → 85 → 64 |
| kaiju | 46 → 39 → 41 | 0 | 107 → 81 → 49 |
| chimera | 165 → 156 → 196 | 12 | 80 → 60 → 50 |

- I tempi del primo giro erano gonfiati dalle altre misure; anche gli ultimi giravano accanto alla matrice (un thread).
- **Collegamenti strani** segnalati dal testo: 71 → 27 → 44. Sono tutti giusti, tranne 2:
  - Branded Fusion "toGrave" (19): è la M3, ora i materiali dal Deck vanno al Cimitero. Lo script li segnalava perché il testo non dice "GY" (corretto in `scripts/links_report.py`);
  - Subspace Battle (13) e Transaction Rollback (10): giusti (B3);
  - **restano 2:** Branded Banishment "search" Starliege Seyfert.
- **Haggard Lizardose e S:P Little Knight** "search" Bystial Druiswurm: spariti.
  - La traccia in `Reader::see` mostra il meccanismo. Bystial Magnamhut ("during the End Phase of this turn, add 1 Dragon monster") registra un effetto ritardato. La sua `SelectCard` arriva dopo `ChainSolved` e `ChainEnd`, fuori da ogni catena, quindi la causa è `acting`.
  - In questo giro `acting` era Magnamhut stessa; quando l'ultima azione era un'altra carta, la ricerca andava a quella. Branded Banishment → Seyfert è probabilmente lo stesso caso.
  - **Proposta:** azzerare `acting` a ogni cambio di fase (`NewPhase`), così un effetto ritardato non viene dato a nessuno invece che alla carta sbagliata. Meglio ancora: far passare dal motore il messaggio `HINT_CARD`, che gli script mandano proprio quando un effetto ritardato si applica, e usarlo come causa.
- **A3 (R.B.):** dal motore è corretta, Shepherd's Crook ha solo materiali "R.B.". Ma la **bozza dal testo** (`text: true`) aggiunge come materiali anche Magie e Trappole il cui nome contiene l'archetipo:
  - R.B. Stage Landing, R.B. Operation Test, R.B. Funk Dock, R.B. Next Phase;
  - in chimera, Artmage Vandalism -Assault- e Artmage Pact -Awakening- verso Artmage Diactorus, Non-Finito, Nerva.
  - 19 collegamenti in 2 deck su 6. **Correzione:** nella bozza dal testo, il materiale deve essere un mostro (`type & MONSTER`).
- **"Posiziona dal Deck"** (M2) c'è: branded 18, chimera 12. Restano senza: Triple Tactics Thrust (si posiziona solo se l'avversario ha attivato un effetto di mostro: in goldfish mai) e Urgent Schedule (falso positivo della regex).
- Collegamenti solo dalla bozza del testo: 112 su 1118.

## Corsia 3: coppie di board per il giudizio del proprietario (22/10/2026)
Per la #47 e i pesi (#43, D17 in `docs/plans/classificazione-board.md`): poche coppie chiare, su cui il proprietario dà il verdetto da una pagina.

### Come sono fatte
- **Script:** `scripts/owner_pairs.py`, dopo `board_pairs.py make` (le coppie di `pairs.tsv` con il codice di oggi) e `combo_hunt.py --json` (le board di oggi).
  ```
  python3 scripts/board_pairs.py make $S/made --decks $S/mdm
  python3 scripts/combo_hunt.py $S/mdm --lines-rule --top 8 --seed 7 --json $S/hunt7
  python3 scripts/combo_hunt.py $S/mdm --hands 2 --top 8 --seed 7 --json $S/hunt7b
  python3 scripts/owner_pairs.py --made $S/made --hunt $S/hunt7 $S/hunt7b --per-deck 3 --controls 8 --only <16 deck>
  ```
- **Uscita:**
  - `tests/fixtures/boards/owner-pairs.tsv`: condizioni come in `pairs.tsv` (`sig=`, leggibili da `board_pairs.py`), tipo, origine, punteggi di oggi, verdetto del pannello, perché;
  - `tests/fixtures/boards/owner-pairs.json`: per la pagina. Per ogni coppia: mano di partenza, e per A e B le carte sul campo (nome, codice, zona, posizione, bersaglio dell'equipaggiamento), la mano finale, Cimitero e bandite, lunghezza della linea, punteggio e metriche (campo, mano, floodgate, resilienza).
  - Solo nomi e codici: niente testi delle carte, niente decklist.
- **Deck:** 16 archetipi di Master Duel Meta, al più 3 coppie per deck. blue_eyes, branded, dracotail, elfnote, elfnote_kewl_tune, exodia, fur_hire, invoked, labrynth, maliss, mitsurugi, snake_eye, synchrons, tearlaments, white_forest_azamina, yubel.

### Le 46 coppie

| Tipo | Coppie | Come sono scelte |
|---|---|---|
| da confermare | 7 | Coppie «da confermare» del pannello del 21/10 (J7, J18, J20, J21, J26, J31, J35), una per deck |
| controllo | 8 | Una board e la stessa con una carta in più sul campo, stessa mano: il verdetto atteso è A>B |
| nuova | 31 | Le board di una mano con punteggi vicini (al più 0,15) e almeno 2 carte diverse, di cui almeno una sul campo; punteggio massimo almeno 0,5; le più diverse prima |

- **Le coppie del pannello quasi non si ritrovano.** Le liste di Master Duel Meta di oggi sono cambiate, quindi la regola "-lines" estrae altre mani: `board_pairs.py make` ritrova solo J21 (elfnote_kewl_tune).
  - Le altre 6 sono ricostruite dalla firma `sig=`, che contiene tutte le carte di campo e mano. Non hanno la mano di partenza, la linea né i punteggi di oggi (`rebuilt: true`, `score: null`).
  - Per misurarle di nuovo servono le liste del 21/10, se la coordinatrice le ha ancora.
- **I controlli che il codice sbaglia già:** 3 controlli su 8 hanno il punteggio di A più basso di quello di B, benché A abbia una carta in più:
  - labrynth: 0,85 contro 1,09;
  - synchrons: 1,98 contro 2,15;
  - fur_hire: 1,34 contro 1,96.

  La differenza può stare nel Cimitero o nelle risorse, che le firme non mostrano. Se il proprietario dice A>B, sono errori della misura.
- **21 coppie nuove su 31 hanno lo stesso punteggio** (pari). Sono proprio i casi in cui la classifica non sa scegliere.

### Limiti, da sistemare nella CLI (corsia 1, `crates/ygo-cli/src/main.rs`)
- **Cimitero e bandite:** `combos --json` dà solo quante carte ci sono (`grave`, `banished`), non quali. Nel JSON ora sono numeri.
  - Basta aggiungere in `state_json` `grave_names` e `banished_names` (nomi o codici). `owner_pairs.py` li usa da solo appena ci sono.
  - Per il giudizio servono: alcune coppie differiscono solo lì.
- **Posizione dei mostri:** si sa solo se sono coperti (`(set)`) e se sono nella Zona Mostri Extra (`[EMZ]`). Attacco o difesa scoperta no.
- **Zone Pendulum:** una carta Pendulum nelle Zone Magie e Trappole risulta come mostro.

## Conclusione
Dopo `eedda66` restano queste correzioni, in ordine:
1. **R3 e R4 in `replay`/`find_again` (`matchup.rs`):** ripiego su un altro effetto della stessa carta solo per le risposte venute da un `EffectYesNo`; un passo della linea lascia il posto alla sua prossima azione se è già offerta. Provate sulle 180 board: fermate false di Maxx "C" e Mulcharmy da 81 a 0, fermate vere invariate.
2. **Bozza dal testo nella mappa:** solo mostri come materiali (19 collegamenti falsi in r_b e chimera).
3. **Effetti ritardati nella mappa (`Reader::see`):** azzerare `acting` al cambio di fase, o usare `HINT_CARD` come causa.
4. **`--stops`:** a parità, il primo punto.
5. **Deck dell'avversario del kit a 35 carte (`probe.rs`):** resta per la ricerca, con il bench. Nel rigioco delle handtrap è già risolto: l'avversario pesca dal suo deck vero.
6. Il resto della prima lista (M4 `chance` senza Cimitero, tempo massimo per la mappa) l'ho visto citato come fatto in PR #24. Non l'ho ancora misurato: i 6 deck di questo giro non bastano per la `chance`, servirebbe un giro sui 40.

Gli script per ripetere tutto: `scripts/links_report.py` e `scripts/matchup_report.py` (con `--cli` per confrontare due build, `--skip-run` per rifare solo le tabelle).

## Appendice: la patch sperimentale
È la copia usata per i numeri "dopo", contro il commit `7c0e640`. Non va applicata così: le correzioni sono accese da variabili d'ambiente e ci sono tracce di debug (`QA_TRACE`). Serve per vedere dove stanno le modifiche.

```diff
diff --git a/crates/ygo-solver/src/links.rs b/crates/ygo-solver/src/links.rs
index 3aac0c9..d31dc12 100644
--- a/crates/ygo-solver/src/links.rs
+++ b/crates/ygo-solver/src/links.rs
@@ -444,6 +444,12 @@ fn play(
         if decision.player() == Some(0) {
             reader.offered(engine, &decision);
             // The card of an action you choose: its costs and its summon are its own.
+            // QA experiment: an optional trigger you say yes to is the card acting too.
+            if let (Decision::EffectYesNo { card, .. }, true) = (&decision, std::env::var_os("QA_FIX_C").is_some()) {
+                if duel.actions(&decision).iter().any(|a| a.kind == ygo_engine::ActionKind::Yes && &a.response == answer) {
+                    reader.acting = Some(engine.cards().canonical_code(card.code));
+                }
+            }
             if matches!(decision, Decision::IdleCmd { .. } | Decision::SelectChain { .. }) {
                 let actions = duel.actions(&decision);
                 if let Some(card) = actions.iter().find(|a| &a.response == answer).and_then(|a| a.card) {
diff --git a/crates/ygo-solver/src/matchup.rs b/crates/ygo-solver/src/matchup.rs
index f026f0b..f6120d1 100644
--- a/crates/ygo-solver/src/matchup.rs
+++ b/crates/ygo-solver/src/matchup.rs
@@ -213,16 +213,33 @@ pub(crate) fn none_drawn(size: usize, copies: u32, drawn: usize) -> f64 {
 struct Own {
     answers: Vec<Vec<u8>>,
     kinds: Vec<Discriminant<Decision>>,
+    /// QA experiment: what each answer does (card, location, description).
+    ids: Vec<Option<(u32, u8, u64, ActionKind)>>,
 }
 
 fn own_answers(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &Opponent, line: &Line) -> Result<Own, Error> {
     let mut duel = Duel::goldfish_against(engine, deck, hand, seed, Some(opponent))?;
-    let mut own = Own { answers: Vec::new(), kinds: Vec::new() };
+    let mut own = Own { answers: Vec::new(), kinds: Vec::new(), ids: Vec::new() };
+    let db = engine.cards();
     for answer in &line.answers {
         let Step::Decision { decision, .. } = duel.advance()? else { break };
         if decision.player() != Some(1) {
             own.answers.push(answer.clone());
             own.kinds.push(std::mem::discriminant(&decision));
+            let actions = duel.actions(&decision);
+            let id = match &decision {
+                Decision::EffectYesNo { card, description, .. }
+                    if actions.iter().any(|a| a.kind == ActionKind::Yes && &a.response == answer) =>
+                {
+                    Some((db.canonical_code(card.code), card.location, 0 * *description, ActionKind::Activate))
+                }
+                Decision::EffectYesNo { .. } => None,
+                _ => actions
+                    .iter()
+                    .find(|a| &a.response == answer)
+                    .and_then(|a| a.card.map(|c| (db.canonical_code(c.code), c.location, a.description, a.kind))),
+            };
+            own.ids.push(id);
         }
         duel.respond(answer);
     }
@@ -323,6 +340,36 @@ fn replay(
                 while next < own.answers.len() && own.kinds[next] != kind && own.answers[next] == PASS {
                     next += 1;
                 }
+                // QA experiment: the line's answer by what it does, not by its index.
+                if std::env::var_os("QA_FIX_D").is_some() {
+                    if let Some(Some((code, loc, desc, akind))) = own.ids.get(next) {
+                        let same = |c: &ygo_engine::CardRef, d: u64| db.canonical_code(c.code) == *code && c.location == *loc && (d == *desc || d == 0 || *desc == 0);
+                        let found = match &decision {
+                            Decision::EffectYesNo { card, description, .. } if same(card, *description) => {
+                                actions.iter().find(|a| a.kind == ActionKind::Yes).map(|a| a.response.clone())
+                            }
+                            _ => actions.iter().find(|a| (a.kind == *akind || (*akind == ActionKind::Activate && a.kind == ActionKind::Yes)) && a.card.is_some_and(|c| same(&c, a.description))).map(|a| a.response.clone()),
+                        };
+                        if let Some(r) = found {
+                            duel.respond(&r);
+                            asked = Some(decision);
+                            next += 1;
+                            continue;
+                        }
+                    }
+                }
+                // QA experiment: a new optional question (a trigger of yours the
+                // handtrap gave, the End Phase asked again) gets a passive answer.
+                let only_end = matches!(decision, Decision::IdleCmd { .. }) && actions.iter().all(|a| a.kind == ActionKind::EndTurn);
+                let optional = matches!(decision, Decision::EffectYesNo { .. } | Decision::YesNo { .. } | Decision::SelectChain { .. });
+                if std::env::var_os("QA_FIX_B").is_some() && own.kinds.get(next).is_some_and(|k| *k != kind) && (only_end || optional) && rejected < 40 {
+                    rejected += 1;
+                    if std::env::var_os("QA_TRACE").is_some() {
+                        eprintln!("QA T passive next={next} decision={decision:?}");
+                    }
+                    duel.respond(&passive_answer(&decision, &actions));
+                    continue;
+                }
                 // Asked something else: the handtrap changed the line.
                 if own.kinds.get(next).is_some_and(|k| *k != kind) {
                     if std::env::var_os("DUELFORGE_DEBUG_MATCHUP").is_some() {
@@ -333,6 +380,10 @@ fn replay(
                 }
             }
             let Some(answer) = own.answers.get(next) else { break };
+            if std::env::var_os("QA_TRACE").is_some() {
+                let picks: Vec<String> = actions.iter().filter(|a| &a.response == answer).filter_map(|a| a.card).map(|c| db.name(c.code)).collect();
+                eprintln!("QA {} use_at={:?} next={next} kind={:?} picks={picks:?} answer={answer:?}", if used {"T"} else {"B"}, use_at.map(|u| u.1), std::mem::discriminant(&decision));
+            }
             duel.respond(answer);
             asked = Some(decision);
             next += 1;
@@ -405,7 +456,16 @@ fn hand_trap_stops(
     own: &Own,
     hand_traps: &[u32],
 ) -> Result<Vec<RivalHandTrap>, Error> {
-    let opponent = Opponent { hand: hand_traps.to_vec(), deck: kit.opponent().deck.clone(), extra: Vec::new() };
+    let mut rdeck = kit.opponent().deck.clone();
+    // QA experiment: a deck as big as a real one (Maxx "C" draws from it).
+    if std::env::var_os("QA_FIX_A").is_some() {
+        if let Some(&filler) = rdeck.last() {
+            while rdeck.len() < 35 {
+                rdeck.insert(0, filler);
+            }
+        }
+    }
+    let opponent = Opponent { hand: hand_traps.to_vec(), deck: rdeck, extra: Vec::new() };
     let base = replay(engine, deck, hand, seed, &opponent, own, None)?;
     let mut out = Vec::new();
     for &code in hand_traps {
```

## Appendice 2: la prova di R3 e R4 su `eedda66`
Le correzioni sono accese da `QA_YN_ZERO` (R3) e `QA_LOOKAHEAD` (R4); `QA_NO_FALLBACK` è la variante scartata. Ci sono anche tracce di debug (`QA_TRACE`, `QA_EV`).

```diff
diff --git a/crates/ygo-solver/src/matchup.rs b/crates/ygo-solver/src/matchup.rs
index 7d387a7..d0a65df 100644
--- a/crates/ygo-solver/src/matchup.rs
+++ b/crates/ygo-solver/src/matchup.rs
@@ -594,7 +594,7 @@ fn own_answers(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, oppone
                 Decision::EffectYesNo { card, description, .. } if *answer == window_answer(&decision, Some(0)) => Some(ActionId {
                     code: db.canonical_code(card.code),
                     location: card.location,
-                    description: *description,
+                    description: if std::env::var_os("QA_YN_ZERO").is_some() { 0 } else { *description },
                     kind: ActionKind::Activate,
                 }),
                 Decision::IdleCmd { .. } | Decision::SelectChain { .. } => {
@@ -633,7 +633,7 @@ fn find_again(engine: &Engine, decision: &Decision, actions: &[Action], id: Acti
                 a.kind == id.kind && a.card.is_some_and(|c| db.canonical_code(c.code) == id.code && c.location == id.location)
             };
             let offered: Vec<&ygo_engine::Action> = actions.iter().filter(same).collect();
-            offered.iter().find(|a| a.description == id.description).or(offered.first()).map(|a| a.response.clone())
+            offered.iter().find(|a| a.description == id.description).or(offered.first().filter(|_| if std::env::var_os("QA_YN_ZERO").is_some() { id.description == 0 } else { std::env::var_os("QA_NO_FALLBACK").is_none() || matches!(decision, Decision::SelectChain { triggers: true, .. }) })).map(|a| a.response.clone())
         }
         _ => None,
     }
@@ -737,6 +737,11 @@ fn replay(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &
         if duel.turn_info().0 > 1 {
             break;
         }
+        if used && std::env::var_os("QA_EV").is_some() {
+            for e in &events {
+                if !matches!(e, Event::Move { .. }) || true { eprintln!("QA EV {e:?}"); }
+            }
+        }
         for e in &events {
             match e {
                 Event::Chaining { card, .. }
@@ -769,9 +774,25 @@ fn replay(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &
                 while next < own.answers.len() && own.kinds[next] != kind && own.answers[next] == PASS {
                     next += 1;
                 }
+                // QA: a pass of the line where its next action is already offered
+                // (a handtrap's chain took one of its windows): play the action.
+                if std::env::var_os("QA_LOOKAHEAD").is_some() && own.answers.get(next) == Some(&PASS.to_vec()) {
+                    let mut k = next;
+                    while k < own.answers.len() && own.answers[k] == PASS {
+                        k += 1;
+                    }
+                    if let Some(answer) = own.ids.get(k).copied().flatten().and_then(|id| find_again(engine, &decision, &actions, id)) {
+                        if std::env::var_os("QA_TRACE").is_some() { eprintln!("QA T lookahead {next}->{k}"); }
+                        duel.respond(&answer);
+                        asked = Some(decision);
+                        next = k + 1;
+                        continue;
+                    }
+                }
                 let expected = own.ids.get(next).copied().flatten();
                 // The line's next action, found again by what it is.
                 if let Some(answer) = expected.and_then(|id| find_again(engine, &decision, &actions, id)) {
+                    if std::env::var_os("QA_TRACE").is_some() { eprintln!("QA T next={next} found {expected:?}"); }
                     duel.respond(&answer);
                     asked = Some(decision);
                     next += 1;
@@ -785,6 +806,7 @@ fn replay(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &
                 let same_question = own.kinds.get(next) == Some(&kind) && expected.is_none();
                 let to_theirs = theirs_on_top && matches!(decision, Decision::SelectChain { .. });
                 if declinable(&decision) && (to_theirs || !same_question) {
+                    if std::env::var_os("QA_TRACE").is_some() { eprintln!("QA T declined next={next} {decision:?}"); }
                     duel.respond(&passive_answer(&decision, &actions));
                     continue;
                 }
@@ -802,6 +824,12 @@ fn replay(engine: &Engine, deck: &DeckList, hand: &[u32], seed: u64, opponent: &
             // After a handtrap, the cards the line picks are found again by
             // what they are, not by their place in the list.
             let again = own.picks.get(next).and_then(Option::as_ref).filter(|_| used).and_then(|p| pick_again(engine, &decision, p));
+            if std::env::var_os("QA_TRACE").is_some() {
+                let r = again.as_ref().unwrap_or(answer);
+                let picks: Vec<String> = actions.iter().filter(|a| &a.response == r).filter_map(|a| a.card).map(|c| db.name(c.code)).collect();
+                eprintln!("QA {} next={next} kind={:?} picks={picks:?} answer={r:?} cards={:?}", if used {"T"} else {"B"}, std::mem::discriminant(&decision),
+                    match &decision { Decision::SelectCard { cards, .. } => cards.iter().map(|c| db.name(c.code)).collect::<Vec<_>>(), _ => Vec::new() });
+            }
             duel.respond(again.as_ref().unwrap_or(answer));
             asked = Some(decision);
             next += 1;
```
