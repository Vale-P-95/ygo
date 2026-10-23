# Piano: Il Banditore v1 (Pagina Novità)

Questo piano definisce l'architettura e l'implementazione della pagina **«Il Banditore»** (sezione «Novità», `active === "news"`), in conformità con `docs/plans/piano-prossimi-passi.md` (Corsia 5), `docs/TASKS.md` e `AGENTS.md`.

---

## Obiettivi di v1

1. **Confronto tra formati (Master Duel, TCG, OCG):**
   - Elenco completo e filtrabile delle carte soggette a restrizioni (Proibita, Limitata, Semi-Limitata) con evidenziazione delle **divergenze tra formati** (stati differenti tra MD, TCG e OCG).
   - Barra di ricerca per nome carta / passcode.
   - Filtri rapidi per formato (Tutti, MD, TCG, OCG) e per stato (Tutti, Solo divergenze, Proibite, Limitate, Semi-Limitate).
   - Mostrare in evidenza la **data della lista di Master Duel in uso** (da `api.dataStatus().banlistMd`).
   - Cliccando su una carta si apre il **Visore Carta** (`CardViewer`) a tutto schermo.

2. **Impatto sui deck della Bottega:**
   - Analisi di conformità per ciascun deck salvato dell'utente (`api.listDecks()` e contenuti).
   - Verifica di default nel formato proprio del deck (`deck.format`), con possibilità di selezionare al volo gli altri due formati per valutarne la compatibilità cross-format.
   - Riconoscimento immediato delle carte fuori legge con conteggio esatto delle copie in eccesso da rimuovere (`copiesInDeck - copyLimit`).
   - Pulsante «Modifica nella Fucina» per aprire direttamente il mazzo nell'editor e rettificare le copie.

3. **Nessun backend nuovo (v1):**
   - Si utilizzano i dati già esposti dal backend (`Card.banTcg`, `banOcg`, `banMd`, `api.dataStatus().banlistMd`, `copyLimit(card, format)`).
   - La v2 («cosa è cambiato dall'ultima lista») sarà realizzata in seguito quando Claude renderà disponibile lo storico delle banlist.

---

## Architettura e Componenti

### 1. `apps/desktop/src/pages/Banditore.tsx`
Componente principale strutturato a schede o a due pannelli integrati:
- **Intestazione:**
  - Titolo «Il Banditore», sottotitolo informativo, badge con la data di rilascio della banlist Master Duel attiva (`DataStatus.banlistMd`).
  - Selettore modalità di visualizzazione:
    - «⚖ Confronto Formati & Carte»
    - «📦 Impatto sui Tuoi Deck»
- **Pannello Confronto Formati:**
  - Controlli: input ricerca per nome/testo, toggle «Solo divergenze tra formati», filtro formato (MD / TCG / OCG), filtro restrizione (Tutte / Proibite / Limitate / Semi-Limitate).
  - Tabella / griglia di carte con badge colorati per ciascun formato:
    - 🚫 Proibita (0 copie) in `var(--df-danger)`
    - ⚠️ Limitata (1 copia) in `var(--sec)`
    - 🟡 Semi-Limitata (2 copie) in `var(--df-gold)`
    - 🟢 Illimitata / Legale (3 copie) in `var(--df-muted)`
  - Badge speciale `⇄ Divergenza` quando uno stato differisce dagli altri due.
  - Al click: apertura di `CardViewer` sul modello di `DeckBuilder`.
- **Pannello Impatto sui Deck:**
  - Card per ogni deck della Bottega con nome, formato originale e totale carte.
  - Selettore formato per testare la legalità (es. verificare un deck MD in formato TCG).
  - Riepilogo: badge verde «100% Legale» oppure avviso rosso «N carte fuori legge».
  - Elenco delle carte in violazione con:
    - Nome carta, immagine miniatura.
    - Dettaglio: `×3 in lista, limite: 1 → togli 2 copie`.
  - Pulsante «Apri nella Fucina» che memorizza l'id del deck e invoca `onOpenDeckInForge(deck.id)`.

### 2. `apps/desktop/src/App.tsx`
- Sostituzione del placeholder `Upcoming` per `active === "news"` con `<Banditore onOpenDeckInForge={...} />`.
- Collegamento della navigazione verso `deck-builder`.

### 3. `apps/desktop/src/lib/mock.ts`
- Arricchimento dei dati mock con carte con divergenze note (Maxx "C", Called by the Grave, Super Poly, Heavy Storm) e supporto a banlist nei filtri di ricerca mock per consentire una verifica completa in modalità mock.

---

## Rispetto delle Regole del Protocollo
- **Nessun `as any` o `@ts-ignore`**.
- **Gestione asincrona protetta**: `requestIdRef` su tutti i caricamenti; azzeramento su cambio o smontaggio.
- **Errori visibili**: gestione errori in italiano con messaggi visibili ed esaustivi.
- **Colori token**: `var(--df-ok)`, `var(--df-danger)`, `var(--df-gold)`, `var(--sec)`, `var(--df-surface-2)`, ecc.
- **Verifica**: `pnpm lint`, `pnpm typecheck`, `pnpm build`.
