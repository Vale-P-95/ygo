# Piano: Il Banditore v2 (ricerca OCG e vista «Cosa è cambiato con l'ultima lista»)

Completamento della pagina «Il Banditore» (`apps/desktop/src/pages/Banditore.tsx`), secondo quanto indicato in `docs/TASKS.md` e nella richiesta utente.

## Obiettivi

### 1. Lista OCG nella ricerca iniziale e nel catalogo mock
- In `Banditore.tsx`:
  - Aggiungere una terza ricerca nel `Promise.all` iniziale: `api.search("formato:ocg ban:forbidden ban:limited ban:semi")`.
  - Unire i risultati (`ids`) di TCG, Master Duel e OCG rimuovendo i duplicati con `Set`.
  - In questo modo le carte limitate o proibite solo in OCG compaiono nell'elenco iniziale della scheda «Confronto Formati».
- In `mock.ts`:
  - In `searchCatalog`, quando il token `formato:` o `format:` specifica `ocg`, la ricerca per `ban:` deve verificare `banOcg`.
  - Assicurarsi che quando non è specificato alcun formato, `ban:` legga `banTcg` (in coerenza con il comportamento di default del motore Rust in `ygo-data`).

### 2. Vista «Cosa è cambiato con l'ultima lista» (`api.banChanges`)
- Aggiungere un nuovo tab di navigazione principale in `Banditore.tsx`:
  - `activeTab`: `"confronto"` | `"variazioni"` | `"impatto"`.
  - Etichetta: `📜 Cosa è cambiato` (con contatore delle variazioni caricate).
- Funzionalità della vista variazioni:
  - Filtri per formato: `Tutti` | `Master Duel` | `TCG` | `OCG`.
  - Chiamata asincrona ad `api.banChanges(selectedFormat === "all" ? undefined : selectedFormat)`.
  - Gestione id di richiesta (`changesReqIdRef`) per evitare accavallamenti asincroni.
  - Caricamento in parallelo delle informazioni complete delle carte (`api.getCards(ids)`).
  - Apertura del Visore Carta (`CardViewerModal`) al clic sulla carta o sul nome.
- Raggruppamento ed evidenziazione:
  - Raggruppare le variazioni per data decrescente e formato.
  - Evidenziare per ciascun formato le voci con la data più recente (*«Ultima lista / Ultimo aggiornamento»*).
  - Mostrare chiaramente lo stato precedente e quello nuovo (es. `Illimitata → Limitata (1)`, `Limitata (1) → Semi-limitata (2)`, `Semi-limitata (2) → Illimitata (3)`).
  - Badge colorati per tipo di modifica (restrizione in rosso/arancio, allentamento/sban in verde/ok).
- Messaggio per storico iniziale vuoto:
  - Per TCG e OCG la data corrisponde all'aggiornamento dei dati di YGOPRODeck, e lo storico parte dal primo aggiornamento dopo questa versione.
  - Se la lista è vuota (`changes.length === 0`), mostrare un messaggio chiaro e rassicurante in italiano che spiega che lo storico parte dal primo aggiornamento successivo a questa versione.

## Protocollo e Verifiche
- Nessun `as any` o `@ts-ignore`.
- Id di richiesta su tutti i caricamenti asincroni, gestione errori con messaggi visibili in italiano.
- Colori e token da `src/index.css`.
- `pnpm lint`, `pnpm typecheck`, `pnpm build`.
- Collaudo con mock backend (`VITE_MOCK=1`) e documentazione nella PR.
