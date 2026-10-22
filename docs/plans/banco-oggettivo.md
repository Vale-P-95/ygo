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

## Risultati
- I risultati parziali sono in `tests/fixtures/boards/objective-bench.json`: solo nomi, codici e numeri. Lo stato del giro è in `objective-bench.status`. Si aggiornano ogni 45 minuti durante il giro.
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
