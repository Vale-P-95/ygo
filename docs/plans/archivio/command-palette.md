# Piano: Navigazione da tastiera e Palette dei Comandi (`gemini/command-palette`)

## Obiettivo
Aggiungere a DuelForge la navigazione rapida da tastiera tra le 7 sezioni dell'officina e una **Palette dei Comandi (`Ctrl+K`)** in italiano nello stile della forgia, che permette di cambiare sezione, selezionare un deck salvato (aprendolo nella Fucina, nel Laboratorio o nel Circolo Arcano), cambiare tema/audio e consultare la Guida rapida.

## File coinvolti
1. `apps/desktop/src/components/CommandPalette.tsx` (nuovo):
   - Finestra modale compatta aperta con `Ctrl+K` (o `Cmd+K`) o dal pulsante nella barra laterale.
   - Ricerca istantanea su:
     - **Sezioni dell'officina**: Panoramica (`Ctrl+1`), La Fucina (`Ctrl+2`), La Bottega (`Ctrl+3`), Il Laboratorio (`Ctrl+4`), Il Circolo Arcano (`Ctrl+5`), Il Banditore (`Ctrl+6`), Impostazioni (`Ctrl+7`).
     - **Deck in bottega**: per ogni deck salvato, azioni rapide per aprirlo nella Fucina, nel Laboratorio o nel Circolo Arcano, più `"Nuovo deck nella Fucina"`.
     - **Azioni rapide**: passa a tema Fucina (scuro) / Pergamena (chiaro), attiva/disattiva effetti sonori, apri la Guida rapida (`?`).
   - Navigazione completa da tastiera (`↑`/`↓`, `Invio`, `Esc`).
2. `apps/desktop/src/App.tsx`:
   - Registra le scorciatoie globali (`Ctrl+K` per la palette, `Ctrl+1..7` / `Alt+1..7` per le sezioni, `?` fuori dai campi di testo per la Guida rapida).
   - Passa l'elenco dei deck (`decks`) a `CommandPalette` e `deckCount` a `Overview` / `WelcomeModal`.
3. `apps/desktop/src/components/Sidebar.tsx`:
   - Aggiunge il pulsante rapido **Comandi (`Ctrl+K`)** sopra **Guida rapida (`?`)**, funzionante sia in modalità estesa sia nella barra compatta a icone.
4. `apps/desktop/src/components/WelcomeModal.tsx`:
   - Aggiunge `Ctrl+K` e `Ctrl+1..7` / `?` tra le scorciatoie utili del Passo 1.

## Vincoli
- Nessuna modifica a `crates/`, `apps/desktop/src-tauri/` o ai tipi in `apps/desktop/src/lib/api.ts`.
- Testi dell'interfaccia in italiano; codice, commenti e messaggi di commit in inglese.
