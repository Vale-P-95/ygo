# Piano: Ponte rapido tra Laboratorio, Bottega e Circolo Arcano (`gemini/workstation-bridge`)

## Obiettivo
Collegare fluidamente il flusso di lavoro tra le diverse sezioni dell'applicazione (Fucina, Laboratorio, Bottega, Circolo Arcano) senza modificare `api.ts` o i crate Rust:

1. **Dal Laboratorio al Circolo Arcano (`Lab.tsx` + `App.tsx`)**:
   - Nella tabella dei ruoli e carte nel Laboratorio Statistico, aggiungere un'azione rapida `✦ Cerca combo` su ogni carta del Main Deck (con particolare evidenza per le carte con ruolo *Starter* o *Estensore*).
   - Cliccando l'azione, configurare la modalità `card` in `duelforge.combo.${deckId}` con quella carta selezionata come capofila (`card: cardId, mode: "card"`), ricordando il mazzo e navigando direttamente al **Circolo Arcano**.

2. **Dalla Bottega a Laboratorio e Circolo Arcano (`BoxEditor.tsx` + `Shelf.tsx` + `App.tsx`)**:
   - Nel pannello laterale del Deck Box selezionato (`BoxEditor.tsx`), aggiungere i pulsanti di navigazione rapida:
     - `🧪 Laboratorio`: apre il deck selezionato nel Laboratorio statistico.
     - `🔮 Circolo Arcano`: apre il deck selezionato nel Combo Lab.
   - Mostrare un badge sintetico della salute/composizione del mazzo (ruoli assegnati o stato di legalità/carte).

3. **Inoltro navigazione (`App.tsx`)**:
   - `Shelf` e `Lab` ricevono i callback appropriati per cambiare sezione attiva mantenendo il deck attivo memorizzato con `rememberDeck(id)`.
