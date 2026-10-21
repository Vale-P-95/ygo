# Piano: Visore Carta interattivo (`CardViewer`)

## Obiettivo
Rendere il Visore Carta ([`CardViewer.tsx`](../../apps/desktop/src/components/CardViewer.tsx)) una postazione attiva di costruzione e classificazione del deck, consentendo all'utente di aggiungere/rimuovere copie e assegnare ruoli senza dover chiudere il visore e cercare nuovamente la carta nella griglia o nella lista.

## Regole e vincoli stabiliti
1. **Limiti di copie legati al formato del deck**:
   - I controlli di aggiunta copie verificano la banlist del formato effettivo del deck (`deck.format`: Master Duel `md`, `tcg` o `ocg`), calcolata con `copyLimit(card, deck.format)`.
   - Una carta proibita (limite 0) non può essere aggiunta al Main/Side/Extra.
   - Per carte limitate (max 1) o semi-limitate (max 2), il totale copie tra Main, Extra e Side non può superare il limite consentito dal formato.
   - Vengono rispettati anche i tetti massimi delle zone (`ZONE_LIMITS`: Main 60, Extra 15, Side 15, Maybe 30) e il tipo carta (carte Extra Deck non vanno nel Main, Skill/Token non vanno nel deck).
2. **Assegnazione ruoli sincronizzata in `deck.roles`**:
   - I ruoli assegnati o rimossi dal visore aggiornano `deck.roles` direttamente tramite la gestione centralizzata di `DeckBuilder` (`setRoles`), garantendo la persistenza su database/storage e la coerenza con il resto dell'applicazione (Laboratorio, Fucina, Bottega).
3. **Ponte rapido ✦ Circolo Arcano**:
   - Un'azione rapida per avviare la ricerca combo a 1-carta con la carta attualmente visualizzata nel Circolo Arcano (`mode: "card"`).
4. **Layout e design**:
   - Token CSS da `src/index.css` (tema chiaro "Pergamena" e scuro "Forge").
   - Testi in italiano, codice e commenti in inglese.
   - Ottimizzato per risoluzioni compatte (`1280×800`).
   - Nessuna modifica ai tipi di `api.ts`.

## File interessati
1. `apps/desktop/src/lib/deck.ts`
   - Aggiunta helper `removeCard(deck: Deck, id: number, zone: Zone): Deck`.
2. `apps/desktop/src/components/CardViewer.tsx`
   - Estensione delle `Props` per accogliere `deck?: Deck | null`, `onAddCard?: (zone: Zone) => void`, `onRemoveCard?: (zone: Zone) => void`, `onToggleRole?: (roleKey: string) => void`, `onExploreCombo?: (cardId: number) => void`.
   - Aggiunta della sezione interattiva nel pannello laterale destro:
     - Blocco copie con indicazione del formato, limite banlist e controlli `+` / `−` per Main (o Extra), Side e Maybe.
     - Blocco ruoli con chip interattivi (`Starter`, `Estensore`, `Handtrap`, `Board breaker`, `Brick` + eventuali custom).
     - Pulsante rapido `✦ Circolo Arcano`.
3. `apps/desktop/src/pages/DeckBuilder.tsx`
   - Cablaggio dei callback verso `CardViewer` passando il deck attivo, `tryAdd`, rimozione copie, toggle dei ruoli su `setRoles` e avvio combo.
4. `docs/TASKS.md`
   - Tracciamento e lock del task.

## Verifiche previste
- `pnpm --dir apps/desktop typecheck`: 0 errori
- `pnpm --dir apps/desktop build`: compilazione client riuscita
- `cargo fmt --all -- --check`: formattazione rust intatta
