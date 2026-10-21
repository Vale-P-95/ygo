# Mappa dei collegamenti tra le carte del deck: piano

*Scritto il 21/10/2026 da Claude Code, da approvare con il proprietario. Spunto: le grafiche di ygo-decklab ("Which card fetches what?").*

## Cosa mostra ygo-decklab
Dagli screenshot del deck Invoked (#4 del meta TCG):
- **Le carte** diverse del deck (22, Extra Deck compreso) sono messe in cerchio, con frecce tra loro (94 collegamenti).
- **Cinque tipi di collegamento**, ognuno con un colore:
  - aggiunge alla mano dal Deck;
  - Evoca Specialmente;
  - manda dal Deck al Cimitero;
  - recupera dal Cimitero;
  - è materiale specifico di.
- **Toccando una carta** si accendono le sue frecce, con un riquadro "carta chiave": quante carte raggiunge e quante per tipo.
- **Dei filtri per tipo**, ognuno con il suo conteggio.

I collegamenti sembrano letti dal testo delle carte: quello che una carta *potrebbe* prendere, non quello che si fa davvero nelle combo.

## Cosa possiamo fare meglio
Abbiamo il motore EDOPro e la ricerca delle combo, quindi possiamo leggere i collegamenti dal motore e non dal testo. Si può aggiungere anche quello che il sito non sa:

1. **Collegamenti esatti.**
   - Quando un effetto chiede di scegliere carte (da aggiungere, Evocare, mandare al Cimitero, usare come materiali), il motore elenca tutte quelle che si possono scegliere, con le condizioni già applicate. La ricerca gioca linee da molte mani, e ogni scelta offerta diventa un collegamento "causa → carta", con il tipo letto dal suggerimento del motore (aggiungi, Evoca, manda) e dalla zona di partenza (Deck, Cimitero).
   - Vale per ogni deck e per le carte nuove, senza leggere il testo, come le handtrap e i breaker (§30.10).
2. **Possibili e usati.**
   - Ogni collegamento si dice "possibile" (il motore lo offre) e quanto è "usato" (in quante delle linee migliori del Combo Lab compare).
   - Sulla mappa le frecce usate sono spesse, quelle solo possibili sottili. Il sito questo non lo sa.
3. **Raggiungibilità in più passi.**
   - Esempio: "da Aleister: 11 carte in un passo, 17 in due".
   - Si vedono gli starter veri (le carte che raggiungono di più) e le carte isolate, che nessuna carta raggiunge e che non portano a niente (mattoni). Si collega ai ruoli della Fucina.
4. **Percorso tra due carte.**
   - Scelte due carte, la catena più corta che le collega, ad esempio "Aleister → cerca Invocation → Evoca per Fusione Mechaba".
   - Un pulsante lancia nel Combo Lab la ricerca con quella carta come obiettivo.
5. **Mano sulla mappa e probabilità.**
   - Con la Mano di prova si accende tutto quello che quella mano raggiunge.
   - Per ogni carta si vede la probabilità esatta che una mano iniziale ci arrivi: è la probabilità di pescare almeno una delle carte che hanno un percorso fino a lei, calcolata con il motore statistico del Laboratorio. Ad esempio "arrivi a Mechaba nell'87% delle mani".
6. **Carte chiave.**
   - Le carte che, se negate, tagliano più collegamenti. Ad esempio: "se Ash nega Aleister perdi 9 carte raggiungibili".
   - Si collega all'esposizione e alle handtrap del turno di prova (#44).
7. **Confronto tra versioni del deck.** Collegamenti guadagnati o persi cambiando le carte (si aggancia al confronto tra deck del Laboratorio).

## Come si divide il lavoro
- **Backend (Claude)**: modulo `crates/ygo-solver/src/links.rs`, comando `duelforge-cli links --deck` per le verifiche, comando Tauri `deck_links(deckId)` con eventi di avanzamento e una cache per versione del deck, tipi in `api.ts`.
- **Interfaccia (Gemini)**: la mappa in una scheda nuova del Laboratorio ("Collegamenti"), in SVG con le immagini delle carte.
  - Cerchio come il sito, più una vista a colonne: starter a sinistra, poi le carte intermedie, poi la board.
  - Evidenziazione al passaggio e al clic, filtri per tipo, riquadro della carta chiave, percorso, mano sovrapposta.
  - Parte subito con il backend finto (`mock.ts`) appena il contratto è in `api.ts`.
- **Sessione sorella**: per questo lavoro non serve.
  - L'interfaccia è l'area di Gemini, e il backend tocca `lib.rs` e `api.ts`, che sono file caldi: due sessioni Claude lì si bloccherebbero a vicenda.
  - Ha più senso darle un lavoro indipendente: la velocità della ricerca, o il livello 2 del turno di prova.

## Fasi
| Fase | Chi | Cosa | Stima |
|---|---|---|---|
| F0 | Claude | Contratto: tipi `DeckLinks` in `api.ts` (carte, collegamenti con tipo, possibile/usato, conteggi) e un esempio per il backend finto; richiesta a Gemini in `docs/TASKS.md` | ~30 min |
| F1 | Claude | Collegamenti possibili letti dal motore: ricerca breve da mani campionate di ogni carta, scelte offerte registrate con causa, tipo e zona; materiali dalle Evocazioni dall'Extra Deck; CLI e test sul deck Invoked | ~1 giorno |
| F1 | Gemini (in parallelo) | Mappa a cerchio sul backend finto: evidenziazione, filtri per tipo, riquadro della carta chiave | ~1 giorno |
| F2 | Claude | Collegamenti usati (frequenza nelle linee migliori), raggiungibilità in più passi, probabilità esatte di arrivare a ogni carta, percorso più corto | ~0,5-1 giorno |
| F2 | Gemini | Spessore per uso, anelli di raggiungibilità, mano sovrapposta, percorso con pulsante verso il Combo Lab, vista a colonne | ~1 giorno |
| F3 | Entrambi | Carte chiave (cosa si perde se una carta è negata), confronto tra versioni del deck | da decidere |

## Verifica
- Sul deck Invoked, i collegamenti che il sito mostra devono esserci, salvo quelli impossibili per il motore. Quelli in più vanno guardati uno per uno: sono la prova che il motore vede meglio, o un difetto da correggere.
- Su 5-6 deck di Master Duel Meta: il tempo per costruire la mappa (obiettivo sotto il minuto, poi in cache) e i collegamenti strani da controllare a mano.

## Decisioni per il proprietario
1. **Dove**: una scheda del Laboratorio (consigliato, vicino a ruoli e probabilità) o della Fucina.
2. **Quando**: dopo l'interfaccia del #44 per Gemini; per Claude subito dopo il #44 (consigliato) o dopo la classificazione delle board (#47).
3. **Sessione sorella**: per la velocità della ricerca o per il livello 2 del turno di prova, non per la mappa.
