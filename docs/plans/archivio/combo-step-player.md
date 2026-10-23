# Piano: Riproduttore passo-passo della linea, frecce Link ed esportazione guida combo (`gemini/combo-step-player`)

## Obiettivo
Migliorare la lettura, lo studio e la condivisione delle linee nel **Circolo Arcano** (`Combo.tsx`) con tre aggiunte mirate, senza toccare `api.ts` né i crate Rust:

1. **Controlli del riproduttore passo-passo (`Steps.tsx` + `BoardDetail.tsx`)**:
   - Barra di controllo interattiva sopra la timeline visuale: `⏮ Inizio`, `◀ Prec.`, `▶ Riproduci / ⏸ Pausa`, `Succ. ▶`, `Fine ⏭`, velocità (`1×` / `2×`), contatore passo corrente `k / N` e scorciatoie da tastiera `[` / `]`.
   - Evidenziazione visiva del passo corrente nella timeline (con scorrimento automatico morbido) e riquadro di focus del passo attivo.
   - Le carte protagoniste del passo selezionato vengono evidenziate sul tappetino finale se presenti sulla board; lo stato intermedio del tappetino passo per passo verrà collegato non appena il backend esporrà lo stato del campo e della mano in `ComboStep` (richiesto in `docs/TASKS.md`).
2. **Frecce Link sul tappetino (`Field.tsx`)**:
   - Disegna le 8 direzioni della bitmask di EDOPro (`z.arrows`):
     - `0x001` = basso-sx, `0x002` = basso, `0x004` = basso-dx
     - `0x008` = sx, `0x020` = dx
     - `0x040` = alto-sx, `0x080` = alto, `0x100` = alto-dx
   - Supporta la rotazione di 180° per eventuali carte dell'avversario (`opponent?: boolean`).
3. **Esportazione guida combo (`BoardDetail.tsx`)**:
   - Pulsante `📋 Copia guida` che copia negli appunti una guida testuale pulita (Mano iniziale, Campo finale con soli nomi delle carte e posizioni, e passi numerati della linea), senza mai includere testi ufficiali degli effetti Konami.
