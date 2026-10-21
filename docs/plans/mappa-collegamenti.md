# Mappa dei collegamenti tra le carte del deck: piano

*Scritto il 21/10/2026 da Claude Code. Spunto: le grafiche di ygo-decklab ("Which card fetches what?"). Approvato dal proprietario il 21/10: scheda «Collegamenti» nel Laboratorio, si parte subito (Claude sul backend, Gemini sull'interfaccia dopo quella del turno di prova contro un deck). F0 fatto: tipi `DeckLinks` in `api.ts` e comando `deck_links` (per ora solo le carte).*

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

## Decisioni del proprietario (21/10/2026)
1. **Dove**: una scheda «Collegamenti» nel Laboratorio.
2. **Quando**: subito per Claude; Gemini dopo l'interfaccia del turno di prova contro un deck.
3. **Sessione sorella**: lavora sulla velocità della ricerca (branch `claude/search-speed`), non sulla mappa.

## F1: com'è fatto (21/10/2026)
- **Dove sta.**
  - Codice: `crates/ygo-solver/src/links.rs`.
  - CLI: `duelforge-cli links --deck <ydk> [--nodes 1000] [--hands 2] [--json <file>]`.
  - App: `api.deckLinks(deckId)`, con avanzamento `deck-links-progress` e una cache finché le carte del deck non cambiano.
- **Come legge i collegamenti.**
  - Per ogni carta del Main Deck si cercano 2 mani che la contengono (1000 nodi ciascuna) e si rigiocano le 20 linee migliori di ogni mano.
  - **Collegamenti possibili:** a ogni scelta di carte del motore, le carte offerte diventano collegamenti "causa → carta". Il tipo dipende dal suggerimento dello script (aggiungere, Evocare, mandare al Cimitero, materiali) e dalla zona di partenza. La causa è la carta il cui effetto si sta risolvendo, oppure, per i costi e le Evocazioni, la carta appena attivata.
  - **Collegamenti usati:** si contano dai movimenti delle carte e dalle Evocazioni. `used` è la frazione delle linee che li usano.
  - **Materiali:** contano solo quelli nominati dal mostro dell'Extra Deck nella riga dei materiali (il nome, o un archetipo tra virgolette contenuto nel nome). I materiali generici, come "2 mostri con effetto", non fanno collegamento.
  - **Zona (`zone`).** Ogni collegamento dice da dove arriva la carta: Deck, Extra Deck, Cimitero, bandite, mano. Rianimare o recuperare dal Cimitero non porta in gioco una carta nuova: Invoked Sorath, che rianima qualunque mostro di Livello 6 o inferiore, collega quasi tutto, ma solo dal Cimitero.
- **Raggiungibilità e probabilità.**
  - `reach` conta le carte a un passo, con qualunque collegamento.
  - `reachAll` aggiunge le carte portate in gioco da lì in più passi, solo con collegamenti dal Deck o dall'Extra Deck.
  - `chance` è la probabilità esatta che una mano di 5 tenga la carta o una carta che la porta in gioco.
- **Numeri sul deck Invoked** (fixture di test):
  - 31 carte e 89 collegamenti (78 visti usati) da 30 mani e 600 linee, in 27 secondi su tutti i core.
  - Aleister the Invoker: cerca Invocation (usato nel 30% delle linee) ed è materiale nominato di 7 Invoked.
  - Invocation: Evoca 9 Invoked dall'Extra Deck.
  - Aiwass: cerca i due Aleister ed Evoca gli Invoked.
  - Probabilità di arrivare ad Aleister the Invoker con la mano iniziale: 85%. Ash Blossom: 34%, solo tenendola.
- **Limiti.**
  - Si vedono gli effetti giocati nelle linee trovate: un effetto che la ricerca non usa mai in quelle mani non compare.
  - `chance` è ottimista quando un collegamento richiede più carte, ad esempio una Fusione che vuole anche il secondo materiale.
  - I mostri dell'Extra Deck che nessuna carta nomina (Link e Synchro con materiali generici) hanno `chance` 0: l'interfaccia mostra un trattino.

