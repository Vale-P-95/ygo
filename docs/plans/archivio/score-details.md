# Piano: Scheda del punteggio (follow-up, risorse, costo, risposta prima interruzione e protezione pezzi)

Aggiornamento della scheda del punteggio nel Circolo Arcano (`Score.tsx`, `scoring.ts`, `BoardDetail.tsx`), secondo quanto indicato in `docs/TASKS.md` (passi D13 e D15) e nella richiesta utente.

## Obiettivi

### 1. Testi di follow-up, risorse e costo (`scoring.ts`)
- **`followup`**:
  - `hint`: include anche «effetti da Cimitero e bandite nel tuo turno dopo, dopo il board breaker peggiore (0,3 l'uno)».
  - `explain`: aggiornato per spiegare sia le carte starter/estensori in mano, sia gli effetti che ripartono da Cimitero/bandite dopo il board breaker peggiore.
- **`resources`**:
  - `hint` ed `explain`: rimossa la menzione del Cimitero («e un po' il Cimitero»), poiché il Cimitero non è più contato nelle risorse generiche.
- **`cost`**:
  - `hint` ed `explain`: aggiornato da «carte della mano usate oltre la prima» a «quello che la linea spende oltre alla mano: carte del Deck pagate come costo (0,05 l'una), mostri dell'Extra Deck finiti nel Cimitero o banditi (0,1 l'uno), LP pagati (1 ogni 4000)». Le carte della mano rimanenti contano già nelle altre voci.
- **Esposizione**: non toccare i testi di `exposure` (cambieranno in un task successivo).

### 2. Risposta alla prima interruzione (`ComboScore.countered`)
- Nella scheda del punteggio (`ScoreBreakdown` in `Score.tsx`), mostrare il valore `countered` (quota da 0 a 1):
  - Formato testo: *«con una risposta alla tua prima interruzione (Infinite Impermanence) fermi ancora il X%»* (o *«... del valore che fermi»*).
  - Se `countered` è `null`: non mostrare nulla (l'avversario non ha potuto rispondere, o la board non ferma nulla). Non entra nel calcolo del punteggio.

### 3. Protezione dei pezzi (`PieceProtection.tsx` / `api.boardProtection`)
- Aggiungere un pulsante «Protezione dei pezzi» per la board aperta nella scheda del punteggio.
- Alla pressione: chiamata asincrona a `api.boardProtection(board.hand, board.key)`.
- Stato di attesa durante la misura (qualche secondo).
- Gestione id di richiesta (`requestIdRef`) e azzeramento automatico al cambio di board (`board.hand`, `board.key`).
- Errori in italiano con pulsante di riprova.
- Per ogni pezzo restituito (`ComboPiece`):
  - Mostrare codice, nome carta e anteprima al passaggio del mouse (`onHover`).
  - Posizione (`place`: "campo", "Cimitero", "mano", "bandita").
  - Azioni a cui regge (`holds`: "distruggere", "bandire", "girare coperto", "negare").
  - Azioni a cui cade (`falls`).
  - Badge stilizzati con token colore di sistema (senza classi arbitrarie non supportate).

## Protocollo di verifica
- `pnpm lint`: zero errori e zero warning.
- `pnpm typecheck`: zero errori TypeScript, nessun `any`.
- `pnpm build`: compilazione Vite di produzione riuscita.
- Prova con l'app vera (`pnpm tauri dev` con `DUELFORGE_DATA`).
