# Banco oggettivo della forza delle board

*Scritto il 22/10/2026 da Claude Code (corsia 3, branch `claude/tools-qa`). Serve a tarare i pesi del punteggio (#47 D17, #43) sul risultato contro il meta, non sulle opinioni dei giudici.*

## Idea
La forza di una board si misura con quanto ferma il turno vero degli avversari del meta. Il turno di prova contro un deck, livello 2 (`probe-line --vs X --moves`), lo dà: l'avversario rigioca le sue linee migliori, e si conta quanta parte delle sue mosse fa ancora con tutte le tue risposte (`left`).

## Metodo
- **Script:** `scripts/objective_bench.py`.
  ```
  python3 scripts/mdm_decks.py --out $S/mdm
  python3 scripts/combo_hunt.py $S/mdm --hands 2 --top 8 --seed 1 --json $S/hunt
  python3 scripts/objective_bench.py $S/bench --decks $S/mdm --hunt $S/hunt --skip lunalight --jobs 3
  python3 scripts/objective_bench.py $S/bench --decks $S/mdm --hunt $S/hunt --skip-run    # solo le tabelle
  ```
  Il giro riparte da dove era: un'invocazione già fatta non si rifà.
- **Codice:** `main` con la #53 (`5df8595`, materiali dei Rituali per identità).
- **Deck:** le 41 liste di Master Duel Meta del 22/10 (`mdm_decks.py`, non committate).
- **Avversari e pesi del meta:**
  - Master Duel Meta dà la diffusione di ogni tipo di deck in `/api/v1/deck-types`, campo `popRank`: la quota dei top deck classificati, in percentuale. Il campo `tier` è vuoto per tutti.
  - I tipi con `popRank` sopra zero sono esattamente 20, e tutti hanno una lista tra le 41.
  - Il peso è `popRank` normalizzato. Lunalight è saltato: contro di lui una sola invocazione supera il limite di 300 s. Le medie si fanno sui pesi degli avversari misurati.

| Avversario | popRank | Peso |
|---|---|---|
| Elfnote | 14,1 | 20,0% |
| Dracotail | 6,16 | 8,8% |
| Invoked | 5,51 | 7,8% |
| DoomZ | 4,54 | 6,5% |
| Branded | 4,38 | 6,2% |
| Clown Crew | 3,73 | 5,3% |
| Elfnote Kewl Tune | 3,57 | 5,1% |
| Artmage | 3,08 | 4,4% |
| Phantom Knights | 2,76 | 3,9% |
| Sky Striker | 2,76 | 3,9% |
| Power Patron | 2,76 | 3,9% |
| Maliss | 2,27 | 3,2% |
| Resonators | 2,27 | 3,2% |
| Orcust | 2,11 | 3,0% |
| Lunalight (saltato) | 2,11 | 3,0% |
| Yummy | 1,94 | 2,8% |
| Blue-Eyes | 1,78 | 2,5% |
| Magistus Fairy Tail | 1,78 | 2,5% |
| Tearlaments | 1,46 | 2,1% |
| Vanquish Soul K9 | 1,30 | 1,8% |

- **Board:** per ognuno dei 41 deck, le prime 5 board con campi distinti (le zone non contano), nell'ordine stampato da `combo_hunt.py --hands 2 --top 8 --seed 1` (D14). Prima la mano 0, poi la 1. In tutto 205 board, in 43 file di linee.
- **Misure:** un'invocazione di `probe-line --vs <avversario> --moves --json` per ogni file di linee e ogni avversario, con tutte le sue board: le linee dell'avversario si leggono una volta. Limite 300 s per invocazione. Per ogni board e avversario:
  - **livello 2:** `left` (quota delle sue mosse fatte ancora con tutte le tue risposte), quota delle sue mosse fatte su campo vuoto (`made`), `floodgates stop`, `value`, linee lette;
  - **livello 1:** resilienza contro i suoi breaker, probabilità che una sua handtrap fermi la tua linea, breaker in mano;
  - **misure generiche della board**, dal JSON: campo, mano, floodgate, resilienza, `follow_up`, `countered`, esposizione. Il costo e le risorse non ci sono ancora nel JSON.
- **Coppie dubbie:** una coppia board-avversario con meno dell'80% delle mosse fatte su campo vuoto è dubbia (L4-6: le mosse della sua Fase Finale non si rigiocano). Resta nei risultati, ma esce dalla "verità".
- **Verità per board:** 1 − `left`, media pesata sugli avversari, senza le coppie dubbie. È la parte delle mosse del meta che la board ferma.
- **Divisione:** per deck (le board di un deck restano insieme), 70% taratura e 30% verifica, con il seme 47.

## Nuova base (23/10, main 1af365d)
Questa è la base da usare ora: `tests/fixtures/boards/objective-bench.json` e `owner-bench.json` contengono questi numeri. Quelli della prima base, più sotto, restano per il confronto.

### Cosa è cambiato nel codice
- B5a: il kit dell'avversario ha un Extra Deck.
- #86 e #91: nel livello 2, l'Extra Deck dell'avversario si rigioca e le opzioni si scelgono per descrizione.
- #93: le tue risposte valgono insieme.
- La verità (livello 2) viene dai giri su `main` 1af365d. Le misure generiche vengono da `main` baf1d82, cioè con la #94 (la regola "insieme" nella misura), rimisurate senza `--vs`. La #94 cambia il punteggio solo di 6 board su 205: Warrior Combo ×3 (+0,06), Tearlaments ×2 (+0,14), Yubel-0#3 (+0,42). Il ρ e l'ordine delle coppie non cambiano alla seconda cifra.

### Il giro
- Stesse board, avversari e pesi della prima base. Fa eccezione Elfnote Kewl Tune mano 0: le sue 5 linee vecchie non si rigiocano più neanche da sole dopo B5a (rc 1, "SelectChain { chains: [] } answered [0, 0, 0, 0]"). Le ho sostituite con le prime 5 board di una ricerca nuova sulla stessa mano (`combo_hunt.py --hands 2 --top 8 --seed 1` su `main`), con id `elfnote_kewl_tune-0new#k`.
- 817 invocazioni, 4 alla volta, mediana 82 s contro 41 s della prima base. Mentre girava, la macchina faceva anche altro (le misure senza `--vs` e la preparazione delle coppie), quindi non so quanta parte del raddoppio venga dalla #93.
- 8 invocazioni oltre il limite di 300 s: Elfnote mano 0 contro 5 avversari, Dracotail mano 0 contro 3. Le loro board hanno la verità su una parte del meta: 60% e 86% del peso.
- Righe senza misura: 40 contro 159 della prima base. Sono solo le 8 invocazioni oltre il limite: Tearlaments, Earth Machine e Railway ora si misurano tutti. Mosse dell'avversario fatte su campo vuoto: 96,3% (era 94,7%). Nessuna coppia dubbia.

### Cosa si è mosso
- La verità sale un poco: +0,018 in media sulle 200 board in comune, mediana di |Δ| 0,013. L'ordine resta quasi lo stesso: ρ tra vecchia e nuova 0,974. Mediana 0,643 (era 0,633).
- Rispetto al giro intermedio su 5b00370 (prima della #93): +0,013 in media, mediana di |Δ| 0,004. Quindi buona parte del cambio viene dalla #93 (le risposte insieme).
- Per avversario, la parte di mosse fermate (media sulle board) sale di più contro Phantom Knights (0,55 → 0,62), Vanquish Soul K9 (0,54 → 0,58), Sky Striker (0,57 → 0,60) ed Elfnote Kewl Tune (0,60 → 0,63). Scende solo contro Tearlaments (0,58 → 0,53), che prima aveva righe senza misura.
- Per deck: salgono di più Power Patron (+0,07), Odion (+0,06) e Branded (+0,05); scende solo Tearlaments (−0,015).
- Le board nuove di Elfnote Kewl Tune hanno verità 0,35–0,39 e punteggio 0,53–0,80: il punteggio le sopravvaluta.

### Come le misure generiche seguono la verità
| Gruppo | Board | ρ score | ρ campo | ρ resilienza | ρ countered | ρ floodgate | ρ l1 | Coppie dello stesso deck nell'ordine giusto |
|---|---|---|---|---|---|---|---|---|
| taratura | 145 | 0,87 | 0,72 | 0,74 | 0,40 | 0,00 | 0,74 | 113 su 135 |
| verifica | 60 | 0,63 | 0,52 | 0,63 | 0,28 | −0,33 | 0,59 | 48 su 50 |

- Sulle stesse 200 board della prima base: ρ score 0,86 in taratura (era 0,84) e 0,63 in verifica (era 0,64). Coppie nell'ordine giusto: 108 su 130 e 48 su 50.
- Il quadro non cambia: il punteggio composto segue la verità meglio di ogni misura singola, e il floodgate non aiuta.

### Le coppie del proprietario
- 77 board, 532 invocazioni, 30 oltre il limite:
  - O1a contro tutti i 19 avversari, che quindi resta senza verità;
  - Blue-Eyes mano 0 contro 7: O12–O14 hanno la verità sul 42% del peso;
  - Synchrons contro 3: O29 e O30 al 65%;
  - Dracotail mano 1 contro 1: O35 e O36 al 92%.
- O1b non si rigioca, come prima.
- Sulle 76 board con la verità, la misura generica di oggi la segue con ρ 0,85: taratura 0,85, verifica 0,76. Coppie dello stesso deck nell'ordine giusto: 51 su 60 e 26 su 27.
- Accordo col proprietario:
  - verità del banco: 9 su 38, stesso verso 11 su 32;
  - misura generica di oggi: 11 su 38, stesso verso 8 su 32.
- Le coppie dove la verità decide (|Δ| ≥ 0,02) sono le stesse 7 della prima base, con lo stesso segno: d'accordo col proprietario O10, O33 e O34; non d'accordo O16, O17, O18 e O23. La lettura della prima base resta valida.

## Prima base (22/10, main 5df8595)
- Il file di questa base è nella storia di `tests/fixtures/boards/objective-bench.json` (PR #76).
- Il giro è finito il 22/10 alle 16:51 UTC.
  - 817 invocazioni su 817, nessuna oltre il limite. Tempo totale 11,1 ore di invocazioni (3 alla volta), mediana 41 s. Il container è stato reclamato due volte quando la sessione era ferma; il giro è ripartito da dove era.
  - 3895 righe board-avversario. Mosse dell'avversario fatte su campo vuoto: 94,7% in media.
  - Una sola coppia dubbia (sotto l'80%): Railway contro Artmage, 79%.

### Righe senza misura (159)
- **Elfnote Kewl Tune, mano 0** (5 board × 19 avversari):
  - il livello 2 e il livello 1 falliscono con "the engine rejected the response: SelectChain { chains: [] } answered [1, 0, 0, 0]";
  - la stessa linea, senza `--vs`, si rigioca e si misura;
  - è lo stesso errore della linea J21 delle coppie del proprietario (O1b): nel turno di prova contro un deck la linea risponde "attiva" a una catena che lì è vuota.
- **Tearlaments, mano 0** (55 righe, soprattutto le board #0 e #1):
  - "0 lines · -0% of their moves made on an empty board · … · 100% of the rest left";
  - l'avversario non rigioca nessuna delle sue linee contro queste board, anche se le ha lette (12 da 12 mani);
  - il "-0%" e il "100% left" con 0 linee non sono una misura: la riga resta fuori.
- **Earth Machine, mano 1, board #0:** 9 righe falliscono, con "IdleCmd … answered [0, 0, 0, 0]" o "[5, 0, 0, 0]", e 7 hanno il livello 2 ma un'altra parte fallita. Anche Railway ha 33 righe con una parte fallita, ma il livello 2 c'è.

### La verità per board
- 200 board con la verità (le 5 di Elfnote Kewl Tune mano 0 non ce l'hanno). Mediana 0,63, da 0,00 a 0,83.
- Le board più forti per deck: Elfnote 0,83, White Forest Azamina 0,80, Snake-Eye 0,79, Megalith 0,79, Code Talker 0,78.
- Le più deboli: Ancient Gear 0,00, Kaiju 0,02, Invoked 0,30, Paleozoic 0,33.
- Avversari più facili da fermare (media di 1 − `left` sulle board): Resonators 0,69, Elfnote 0,63, DoomZ 0,61. I più difficili: Power Patron 0,35, Maliss 0,42, Artmage 0,45.

### Come le misure generiche seguono la verità
Correlazione di rango (Spearman) con la verità, per gruppo. `score` è il punteggio composto di oggi (campo, mano, floodgate, resilienza con pesi 1,0 · 0,9 · 0,8 · 0,6). `l1` è la resilienza contro i breaker veri del deck avversario (livello 1).

| Gruppo | Board | ρ score | ρ campo | ρ resilienza | ρ countered | ρ floodgate | ρ l1 | Coppie dello stesso deck nell'ordine giusto (score) |
|---|---|---|---|---|---|---|---|---|
| taratura | 140 | 0,84 | 0,74 | 0,73 | 0,34 | 0,04 | 0,73 | 103 su 137 (75%) |
| verifica | 60 | 0,64 | 0,50 | 0,62 | 0,24 | −0,28 | 0,60 | 44 su 50 (88%) |

- Su tutte le 200 board: ρ score 0,80, ρ l1 0,71, ρ mano 0,35, ρ della probabilità che una handtrap fermi la linea 0,32.
- Il punteggio composto segue la verità meglio di ogni misura da sola. Tra le misure singole, contano soprattutto campo e resilienza.
- Il floodgate, così come è misurato, non aiuta: ρ quasi zero, negativo sul gruppo di verifica.
- Nelle coppie di board dello stesso deck (differenza di verità ≥ 0,02), il punteggio mette prima quella giusta 3 volte su 4 sulla taratura e quasi 9 su 10 sulla verifica.
- Il livello 1 (resilienza contro i breaker del meta) non aggiunge ordine rispetto al punteggio: ρ 0,73 contro 0,84 sulla taratura.
- I dati per tarare i pesi sono in `tests/fixtures/boards/objective-bench.json`.

## Prima base: le coppie del proprietario nel banco
- **Script:** `scripts/owner_bench.py`. Ritrova le 90 board di `tests/fixtures/boards/owner-pairs.json` per firma senza zone, con lo stesso cimitero e le stesse bandite, nelle cartelle di `combo_hunt.py --json`. Le rimisura con `main` una alla volta e le mette nel banco con gli stessi avversari e pesi (Lunalight saltato).
  ```
  python3 scripts/owner_bench.py $S/owner --own $S/mdm2 --meta-decks $S/mdm6 --meta $S/bench/meta.json --hunt $S/h7m $S/h7bm $S/o1h $S/op_make
  python3 scripts/objective_bench.py $S/owner --decks $S/owner/decks --hunt - --skip lunalight
  python3 scripts/owner_bench.py $S/owner --table tests/fixtures/boards/owner-verdicts.tsv --json tests/fixtures/boards/owner-bench.json
  ```
- **Board non trovate (14 di 90):**
  - O2–O7 sono ricostruite dalla firma e non hanno una linea (12 board);
  - O1b: la linea J21, scritta con una build vecchia, su `main` non si rigioca;
  - O1a: l'ho ritrovata in una ricerca nuova di `main` sulla stessa mano, ma contro tutti gli avversari fallisce con lo stesso errore della mano 0 di Elfnote Kewl Tune ("SelectChain { chains: [] } answered [1, 0, 0, 0]"). Il difetto non dipende dalla build della linea.
- **Giro:** 77 board, 532 invocazioni, nessuna oltre il limite, nessuna coppia dubbia, 6,3 ore. 76 board hanno la verità.
- **Su queste board** la misura generica di oggi segue la verità con ρ 0,83 (taratura 0,84, verifica 0,74). Nelle coppie dello stesso deck l'ordine è giusto 53 volte su 62 e 24 su 25.

### Tabella per coppia
"Verità" è la quota delle mosse del meta che la board ferma. Le differenze sono B − A: positive se B è meglio. Due board sono pari sotto 0,02 di verità o sotto 0,05 di punteggio.

| Coppia | Deck | Tipo | Proprietario | Verità A | Verità B | B−A | Generica oggi B−A | Punteggio di prima B−A | Note |
|---|---|---|---|---|---|---|---|---|---|
| O1 | elfnote_kewl_tune | da confermare | B | — | — | — | — | +0.14 | la linea non rifà più la board: O1b |
| O2 | branded | da confermare | A | — | — | — | — | — | ricostruite dalla firma: nessuna linea |
| O3 | elfnote | da confermare | B | — | — | — | — | — | ricostruite dalla firma: nessuna linea |
| O4 | elfnote_kewl_tune | da confermare | B | — | — | — | — | — | ricostruite dalla firma: nessuna linea |
| O5 | invoked | da confermare | A | — | — | — | — | — | ricostruite dalla firma: nessuna linea |
| O6 | maliss | da confermare | A | — | — | — | — | — | ricostruite dalla firma: nessuna linea |
| O7 | mitsurugi | da confermare | A | — | — | — | — | — | ricostruite dalla firma: nessuna linea |
| O8 | elfnote | controllo | A | 0.83 | 0.83 | -0.01 | -0.16 | -0.10 |  |
| O9 | labrynth | controllo | A | 0.69 | 0.69 | -0.00 | +0.02 | +0.02 |  |
| O10 | snake_eye | controllo | A | 0.86 | 0.83 | -0.03 | -0.30 | -0.31 |  |
| O11 | yubel | controllo | A | 0.62 | 0.62 | +0.00 | +0.00 | +0.00 |  |
| O12 | blue_eyes | nuova | B | 0.71 | 0.70 | -0.01 | -0.08 | -0.05 |  |
| O13 | blue_eyes | nuova | pari | 0.71 | 0.71 | -0.00 | +0.00 | +0.00 |  |
| O14 | blue_eyes | nuova | B | 0.71 | 0.71 | +0.00 | +0.00 | +0.00 |  |
| O15 | branded | nuova | A | 0.60 | 0.61 | +0.00 | +0.00 | +0.00 |  |
| O16 | dracotail | nuova | B | 0.80 | 0.72 | -0.08 | -0.00 | -0.00 |  |
| O17 | elfnote | nuova | A | 0.77 | 0.80 | +0.03 | -0.06 | -0.07 |  |
| O18 | exodia | nuova | A | 0.42 | 0.50 | +0.08 | +0.11 | +0.11 |  |
| O19 | exodia | nuova | A | 0.74 | 0.74 | +0.00 | +0.00 | +0.00 |  |
| O20 | fur_hire | nuova | pari | 0.67 | 0.68 | +0.01 | +0.00 | +0.00 |  |
| O21 | fur_hire | nuova | B | 0.67 | 0.68 | +0.00 | +0.00 | +0.00 |  |
| O22 | invoked | nuova | pari | 0.36 | 0.38 | +0.01 | +0.10 | +0.10 |  |
| O23 | invoked | nuova | B | 0.68 | 0.62 | -0.05 | -0.08 | -0.08 |  |
| O24 | labrynth | nuova | B | 0.51 | 0.51 | -0.00 | +0.00 | +0.00 |  |
| O25 | maliss | nuova | pari | 0.69 | 0.68 | -0.00 | +0.00 | +0.00 |  |
| O26 | maliss | nuova | pari | 0.68 | 0.69 | +0.00 | +0.00 | +0.00 |  |
| O27 | snake_eye | nuova | A | 0.71 | 0.71 | +0.00 | +0.00 | +0.00 |  |
| O28 | snake_eye | nuova | B | 0.71 | 0.71 | -0.00 | +0.04 | +0.00 |  |
| O29 | synchrons | nuova | B | 0.75 | 0.77 | +0.02 | +0.06 | +0.02 |  |
| O30 | synchrons | nuova | B | 0.74 | 0.74 | +0.00 | -0.04 | -0.04 |  |
| O31 | tearlaments | nuova | A | 0.72 | 0.72 | +0.00 | +0.01 | +0.00 |  |
| O32 | tearlaments | nuova | A | 0.76 | 0.77 | +0.00 | +0.01 | +0.00 |  |
| O33 | yubel | nuova | B | 0.65 | 0.68 | +0.02 | +0.12 | +0.11 |  |
| O34 | branded | nuova | A | 0.73 | 0.62 | -0.11 | +0.00 | +0.00 |  |
| O35 | dracotail | nuova | B | 0.62 | 0.62 | -0.00 | -0.11 | -0.11 |  |
| O36 | dracotail | nuova | B | 0.64 | 0.62 | -0.01 | +0.02 | +0.02 |  |
| O37 | exodia | nuova | A | 0.71 | 0.71 | +0.00 | +0.00 | +0.00 |  |
| O38 | fur_hire | nuova | A | 0.67 | 0.67 | +0.01 | +0.11 | +0.11 |  |
| O39 | labrynth | nuova | A | 0.50 | 0.50 | -0.01 | +0.14 | +0.14 |  |
| O40 | mitsurugi | nuova | A | 0.74 | 0.73 | -0.01 | -0.11 | -0.11 |  |
| O41 | synchrons | nuova | B | 0.85 | 0.85 | -0.00 | +0.00 | +0.00 |  |
| O42 | tearlaments | nuova | A | 0.67 | 0.67 | +0.00 | +0.00 | +0.00 |  |
| O43 | white_forest_azamina | nuova | B | 0.69 | 0.69 | +0.00 | +0.00 | +0.00 |  |
| O44 | white_forest_azamina | nuova | B | 0.35 | 0.34 | -0.00 | +0.00 | +0.00 |  |
| O45 | yubel | nuova | pari | 0.62 | 0.63 | +0.00 | +0.00 | +0.00 |  |

| Misura | D'accordo col proprietario (pari compresi) | Stesso verso sulle coppie non pari |
|---|---|---|
| Verità del banco | 9 su 38 | 10 su 32 |
| Misura generica di oggi | 11 su 38 | 8 su 32 |
| Punteggio di prima | 11 su 39 | 8 su 33 |

### Lettura
- **Il banco quasi non separa queste coppie.** In 31 coppie su 38 le due board differiscono di meno di 0,02 di verità.
  - Le coppie erano state scelte proprio perché il punteggio le dava vicine o pari.
  - Le differenze stanno quasi sempre in una carta in mano, nel cimitero, o in un mostro senza effetti nel turno avversario. Il turno vero dell'avversario ne risente poco.
- **I 9 accordi della verità** sono per due terzi i 6 "pari" del proprietario, che il banco dà pari anche lui. Restano O10, O33 e O34.
- **Sulle 7 coppie dove la verità decide**, è d'accordo col proprietario 3 volte (O10, O33, O34) e no 4 volte (O16, O17, O18, O23). Su questi numeri il banco non conferma né smentisce il proprietario.
- **Misura generica:** come prima, perché i pesi non cambiano. Dà pari 25 coppie su 38, e il proprietario quasi mai.

### Cosa dicono le note del proprietario
I criteri che usa e come li tratta oggi il turno di prova:
- **Un'interruzione nel turno avversario vale.** Una carta sul campo che non interagisce nel suo turno (Track Maker in O1, Regina in O28) vale poco anche se è un mostro in più. Il turno di prova conta solo le risposte che fermano mosse, quindi va nella stessa direzione. Il campo (`field`) invece dà valore anche ai corpi senza effetto.
- **Un mostro senza effetti nel turno avversario vale poco:** O12, un Blue-Eyes White Dragon in più contro una trappola in più. Il banco lo conferma: verità −0,01, quasi pari, mentre il punteggio dà −0,08.
- **Una carta in più in mano decide solo a board uguali** (O2, O5: "le board sono identiche ma ho una carta in più in mano"). È un criterio di spareggio: va dopo il resto, non sommato con peso fisso come `hand`.
- **Una trappola che si attiva dalla mano vale più in mano che piazzata** (O9, le Dominus). Oggi una trappola piazzata conta come risposta sul campo e una in mano no, a meno che sia una handtrap.
- **Un mostro nel cimitero può essere un'interruzione** (O14: Neo Blue-Eyes Ultimate Dragon che va al cimitero nel turno avversario). Le risposte dal cimitero contano solo se l'effetto si attiva da lì nella linea dell'avversario.
- **Lo spostamento di zona per schivare un bersaglio conta** (O3 Tinia, O10). Il turno di prova non lo misura.
