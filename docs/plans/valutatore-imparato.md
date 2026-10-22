# Valutatore imparato (idea per più avanti)

Idea discussa con il proprietario il 22/10/2026, da tenere in cantiere: **non è una priorità** e va fatta dopo la classificazione delle board (#47) e i pesi (#43).

## L'idea
Il motore resta come è: le regole le applica EDOPro, la ricerca prova le linee e il turno di prova misura le board. Una rete neurale piccola impara solo a **stimare la valutazione**, come ha fatto Stockfish con NNUE: il motore di scacchi resta lo stesso, la rete sostituisce la formula di valutazione scritta a mano.
- **Oggi:**
  - Il turno di prova costa da decimi di secondo a qualche secondo per board, quindi durante la ricerca si usa una stima scritta a mano (`quick_score`, con i valori delle carte di `value.rs`).
  - La ricerca segue quella stima: dove sbaglia, esplora prima le strade sbagliate.
- **Con il valutatore imparato:**
  - Una rete piccola (poche migliaia di parametri, un MLP) stima in microsecondi il risultato del turno di prova di una board: interruzioni, resilienza, punteggio.
  - La ricerca la usa al posto di `quick_score` o accanto a esso. Il turno di prova vero resta il giudice finale delle board mostrate.

## Perché solo più avanti
- **La valutazione deve essere giusta prima di impararla.** Una rete addestrata sul turno di prova di oggi imparerebbe anche i suoi difetti: 18 coppie sicure giuste su 29 (`docs/plans/classificazione-board.md`). Prima la #47 con i giudizi del proprietario, poi la rete.
- **I dati si producono da soli,** con gli strumenti che ci sono: `combo_hunt.py --json` e `combos --json` salvano board e misure su 45 deck. Mancano solo il volume (milioni di board) e una rappresentazione delle board come numeri.

## Come si farebbe
1. **Rappresentazione:**
   - le carte sul campo, in mano e nel Cimitero come vettori di caratteristiche generali: tipo, livello o rango, ATK e DEF, categorie di effetto lette dal motore (nega, distrugge, bandisce…), zone;
   - **mai l'identità della carta**, così la rete resta generale e vale per le carte nuove.
2. **Dati:** board e misure del turno di prova su tutti i deck di Master Duel Meta, con più semi e più mani.
3. **Addestramento:**
   - in Python, fuori dall'app;
   - verifica su deck tenuti fuori dall'addestramento (circa il 30%): la rete deve funzionare su deck che non ha mai visto.
4. **Uso:**
   - i pesi della rete in un file;
   - l'inferenza in Rust nella ricerca, senza GPU e senza dipendenze pesanti;
   - si confronta con `quick_score` sul banco: migliori board trovate con lo stesso numero di nodi.

## Regole che restano
- Nessun caso speciale per nome di carta: la rete vede caratteristiche, non nomi.
- Il motore EDOPro decide sempre se una mossa è legale; la rete stima soltanto.
- Il risultato mostrato all'utente resta spiegabile: la linea e la misura del turno di prova vero.
