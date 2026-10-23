# Piano: Scheda «Collegamenti» nel Laboratorio (Mappa delle carte del deck)

*Data: 21/10/2026 — Agente: Antigravity / Gemini — Branch: `gemini/links-map`*

Riferimenti:
- Specifiche generali e backend: [`docs/plans/mappa-collegamenti.md`](../mappa-collegamenti.md)
- Contratti API: [`apps/desktop/src/lib/api.ts`](../../../apps/desktop/src/lib/api.ts) (`DeckLinks`, `CardLink`, `LinkCard`, `CardLinkKind`, `CardLinkZone`, `api.deckLinks`, `api.onLinksProgress`)
- Bacheca: [`docs/TASKS.md`](../../TASKS.md)

---

## 1. Obiettivo
Creare la nuova scheda **«Collegamenti»** nel Laboratorio (`Lab.tsx`), che mostra visivamente in che modo le carte del deck portano l'una all'altra (ricerche dal Deck, Evocazioni, mandare al Cimitero, recuperi, materiali dell'Extra Deck), evidenziando starter, ponti, carte isolate e carte chiave del mazzo.

---

## 2. Architettura dei Componenti

```
apps/desktop/src/components/lab/
└── links/
    ├── LinksTab.tsx          # Scheda contenitore principale nel Laboratorio
    ├── LinksCircle.tsx        # Grafico SVG circolare con carte e frecce direzionali
    ├── LinksFilters.tsx       # Filtri per tipo di collegamento (search, summon, ecc.) e zona
    ├── LinksCardDetail.tsx    # Scheda laterale informativa per la carta selezionata/in hover
    └── LinksStats.tsx         # Metriche di sintesi: starter principali, carte chiave, mattoni
```

### 2.1 Scheda Principale (`LinksTab.tsx`)
- Richiede i collegamenti tramite `api.deckLinks(deck.id)`.
- Ascolta l'evento `api.onLinksProgress` per mostrare un indicatore di caricamento progressivo durante il primo calcolo (circa 20-30 secondi per la prima generazione, poi memorizzato nella cache del backend).
- Gestisce lo stato della carta attiva (selezionata o in hover) e i filtri attivi.
- Fornisce un pulsante di aggiornamento *"↻ Ricalcola collegamenti"*.

### 2.2 Grafico Circolare SVG (`LinksCircle.tsx`)
- Dispone le carte distinte del deck lungo un anello circolare (ordinabili per Main Deck / Extra Deck o per probabilità/ruolo).
- Ogni nodo è un badge con la miniatura della carta, nome compatto e indicatore copie.
- I nodi hanno dimensioni o anelli proporzionali alla rilevanza come carta chiave (`LinkCard.key`) o alla probabilità iniziale (`LinkCard.chance`).
- Traccia curve SVG (archi di Bézier) tra le carte:
  - **Colori per tipo di collegamento (`CardLinkKind`)**:
    - `search` (`var(--role-starter)`): Aggiunge alla mano dal Deck.
    - `summon` (`var(--role-extender)`): Evoca Specialmente.
    - `set` (`var(--role-custom-2)`): Posiziona dal Deck.
    - `toGrave` (`var(--role-brick)`): Manda al Cimitero dal Deck.
    - `recover` (`var(--role-breaker)`): Recupera dal Cimitero o carte bandite.
    - `material` (`var(--role-custom-1)`): Materiale specifico richiesto.
  - **Spessore freccia**:
    - Linee spesse e piene se `used > 0` (frequenza d'uso nelle combo migliori).
    - Linee sottili o tratteggiate se solo `possible` (`used === 0`).
  - **Trasparenza**: collegamenti dal Deck o dall'Extra Deck in risalto; collegamenti dalla mano o dal Cimitero più tenui.
- **Interattività**:
  - Quando una carta è selezionata o in hover, si illuminano unicamente i suoi collegamenti in uscita (carte che lei raggiunge) e in entrata (carte che portano a lei), attenuando le altre.

### 2.3 Scheda Informativa Carta (`LinksCardDetail.tsx`)
Quando una carta è selezionata:
- **Identità**: immagine grande, nome, copie nel mazzo, Main vs Extra Deck.
- **Raggiungibilità**:
  - *In 1 passo* (`reach`): quante carte raggiunge direttamente.
  - *In più passi* (`reachAll`): quante carte porta in gioco complessivamente nella combo.
  - *Probabilità in mano iniziale* (`chance`): probabilità esatta (es. `85%`, oppure `—` se mostro Extra non nominato).
  - *Impatto come carta chiave* (`key`): quante carte raggiungibili si perdono in media se la carta viene negata (es. `0,42 carte perse se negata`).
- **Elenco collegamenti**:
  - Carte che porta in gioco (con tipo, zona e % d'uso).
  - Carte che portano a lei.
- **Ponte rapido verso il Circolo Arcano (Combo Lab)**:
  - Tasto *"🎯 Cerca combo con questa carta come obiettivo"* (innesca `onGoCombo(card.code)` impostando l'obiettivo).

### 2.4 Esempio completo nel Backend Finto (`mock.ts`)
- Sostituire lo stub minimale con un grafo realistico basato sul deck Invoked della fixture (`tests/fixtures/decks/invoked-siracusa-md.ydk`), con:
  - Carte principali: Aleister the Invoker, Invocation, Magical Meltdown, Invoked Mechaba, Purgatrio, Augoeides, Artemis the Magistus Moon Maiden, Ash Blossom, ecc.
  - Collegamenti con tutti i 5 tipi e le varie zone.
  - Valori realistici di `reach`, `reachAll`, `chance` e `key`.

---

## 3. Piano di esecuzione a passi
1. **Mock e Dati**: arricchire `mockApi.deckLinks` in `apps/desktop/src/lib/mock.ts` con i dati realistici del deck Invoked.
2. **Scheda in `Lab.tsx`**: aggiungere `links` in `TABS` di `Lab.tsx` e il container `LinksTab`.
3. **Componenti visivi**: implementare `LinksFilters`, `LinksCardDetail` e `LinksCircle` in SVG con gestione completa degli stati hover/selezione.
4. **Verifiche CI e qualità**: `pnpm typecheck`, `pnpm build`, `cargo fmt --all -- --check`.
