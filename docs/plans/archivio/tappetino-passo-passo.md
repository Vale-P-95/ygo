# Piano: Tappetino del Circolo Arcano collegato al riproduttore passo-passo

## Obiettivo
Collegare la visualizzazione del campo di gioco ([`Field.tsx`](../../../apps/desktop/src/components/combo/Field.tsx)) al riproduttore passo-passo della linea ([`Steps.tsx`](../../../apps/desktop/src/components/combo/Steps.tsx)) in [`BoardDetail.tsx`](../../../apps/desktop/src/components/combo/BoardDetail.tsx).
Quando l'utente avvia il player o si sposta tra i passi della combo (`[` / `]`, controlli multimediali `⏮ ◀ ▶ ⏭`, scrub bar), il tappetino deve mostrare lo stato del campo e della mano al passo selezionato tramite `ComboStep.field`, con indicatore contestuale, tornando alla board finale quando il passo viene deselezionato.

## Regole e vincoli stabiliti
1. **Dati dal backend**:
   - `ComboStep.field?: ComboField` è già disponibile in `main` (tipo `ComboField` in `apps/desktop/src/lib/api.ts`).
   - Nessuna modifica ai tipi condivisi né a `crates/` o `apps/desktop/src-tauri/`.
2. **Compatibilità `Field.tsx`**:
   - `FieldProps` accetta `ComboField` (di cui `ComboBoard` è un sottoinsieme arricchito di punteggi e metadati).
   - Sia la board finale sia i campi intermedi di passo possono essere passati a `Field` senza duplicare logica di rendering.
3. **Stato coerente in `BoardDetail.tsx`**:
   - Quando `activeStepField` è attivo, aggiornare anche:
     - Le carte in mano (`currentField.handCards`);
     - Le pile esterne (`currentField.grave`, `currentField.banished`, `currentField.extraFaceUp`, contatore deck);
     - Il banner descrittivo del passo (es. "Passo X di Y: [Nome]" con azione per tornare alla board finale).
   - In assenza di `field` (vecchie rigiocate), fallback trasparente a `board`.
4. **Token di stile e accessibilità**:
   - Token CSS da `src/index.css` (`warn`, `sec`, `muted`, `faint`, ecc.).
   - Supporto ad alta leggibilità sia in tema scuro "Fucina" sia in tema chiaro "Pergamena".
   - Scorciatoie da tastiera invariate (`[` e `]`).

## File interessati
1. `docs/plans/archivio/tappetino-passo-passo.md`
2. `docs/TASKS.md`
3. `apps/desktop/src/components/combo/Field.tsx`
4. `apps/desktop/src/components/combo/Steps.tsx`
5. `apps/desktop/src/components/combo/BoardDetail.tsx`

## Verifiche previste
- `pnpm --dir apps/desktop typecheck`: 0 errori
- `pnpm --dir apps/desktop build`: compilazione client riuscita
- `cargo fmt --all -- --check`: formattazione intatta
