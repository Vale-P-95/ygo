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
| F3 | Entrambi | Carte chiave (cosa si perde se una carta è negata), confronto tra versioni del deck | backend fatto; interfaccia di Gemini |

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

## Carte chiave (21/10/2026)
- `key` per ogni carta: quante carte raggiunte in media dalla mano iniziale (la somma delle `chance`) si perdono se gli effetti della carta vengono negati, cioè se i suoi collegamenti non valgono più.
- Misura quanto il deck si appoggia su quella carta.
- Sul deck Invoked:
  - Aiwass 1,37;
  - Invoked Baybarron 0,88 (unica via a Virakam);
  - Sacred Spirit Sword Aiwass 0,77;
  - Aleister the Reminiscent 0,66;
  - Aleister the Invoker 0,42;
  - Invocation 0,05, perché molte carte portano gli stessi Invoked;
  - handtrap 0.
- I percorsi tra due carte si calcolano nell'interfaccia sui `links`.

## Correzioni dal collaudo (21/10/2026)
- **Causa di un trigger a cui si risponde sì.** Ora è la carta del trigger. Prima la causa era la carta dell'ultima azione: Branded in High Spirits risultava Evocare dal Cimitero 9 mostri che Evoca Guiding Quem.
- **Mappa vera per il backend finto.** In `tests/fixtures/links/invoked.json` c'è la mappa vera del deck Invoked della fixture (solo codici): 31 carte, 101 collegamenti (aggiornata con le correzioni sotto).

## Seconde correzioni dal collaudo (21/10/2026)
- **Materiali.** La riga dei materiali è la prima riga del testo del mostro, dopo l'eventuale Effetto Pendulum. Prima si tagliava al primo punto, e il materiale "R.B." diventava "r", contenuto in quasi ogni nome.
- **Materiali dal Deck.** Un Materiale da Fusione mandato dal Deck (Branded Fusion) è anche un collegamento "manda al Cimitero".
- **Recupera.** Vale solo verso la mano. Rimescolare nel Deck dal Cimitero non è più un collegamento.
- **Tipo nuovo `set`, "posiziona dal Deck".** Una carta messa dal Deck sul campo: coperta, oppure posizionata in una Zona Magie e Trappole o nella Zona Terreno.
  - Esempi: Elfnote Tinia → Elfnotes: Welcome Home (usato nel 64% delle linee), Virakam → Invocation, Albion the Branded Dragon → le Magie e Trappole "Branded".
  - Nel contratto è `CardLinkKind` `"set"`.
- **Probabilità senza saturazione.** `reachAll` e `chance` seguono solo i collegamenti che portano in gioco una carta nuova: cerca, Evoca, materiale, posiziona.
  - Mandare al Cimitero non conta più. Prima Foolish Burial e That Grass Looks Greener "portavano" quasi tutto il deck.
  - Branded: le carte del Main Deck con `chance` del 95% o più passano da 34 a 3.
- **Mostri dell'Extra Deck senza materiali nominati.** Prima avevano `chance` 0. Ora `chance` è la quota delle mani cercate in cui la linea migliore li Evoca, con `measured: true` (misurata, non esatta).
  - Branded: 2 mostri così, Invoked: 5.
- **Tempo.** Al massimo 15 secondi per mano e 60 per la mappa intera, poi le mani rimaste si saltano.
  - Le mani si cercano a turno: prima una per ogni carta, poi la seconda. Così una mappa tagliata ha comunque tutte le carte.
  - Sui deck provati: Branded 63 secondi (48 mani su 68), Elfnote 49, Invoked 23.
- **Per l'interfaccia.** `LINK_KINDS` deve avere la voce `set`. `LinkCard.measured` dice quando mostrare la probabilità come stima.


## Mappa quasi in tempo reale (21/10/2026)
Il proprietario ha chiesto una mappa quasi istantanea, come quella del sito. Le strade approvate sono tre, tutte fatte: bozza dal testo subito, aggiornamento progressivo, ricalcolo solo delle carte cambiate.
- **Bozza dal testo, subito.**
  - Prima di cercare qualunque mano, si leggono i testi delle carte del deck, con regole generali e senza nomi di carte:
    - "add ... from your Deck to your hand" è cerca, "... from your GY to your hand" è recupera;
    - "Special/Fusion/Ritual... Summon ... from your Deck/Extra Deck/GY/hand" è Evoca;
    - "send ... from your Deck to the GY" è manda al Cimitero;
    - "Set/place ... from your Deck" è posiziona;
    - i materiali sono quelli nominati dal mostro dell'Extra Deck.
  - Le carte tra virgolette valgono per nome esatto o per archetipo contenuto nel nome. Le filtra la parola che segue (monster, Spell, Trap) e la zona: un mostro dell'Extra Deck viene solo dall'Extra Deck.
  - Sul deck Invoked: 74 collegamenti in 0,2 secondi.
- **I collegamenti del testo restano.** Nella mappa finale un collegamento ha `text` quando il testo lo nomina.
  - Se il motore non l'ha mai offerto (`possible` falso), è un effetto che le linee trovate non usano: Invoked deck, 16 su 117. Esempi: Invocation - "Chalice" → Invoked Elysium, Sacred Spirit Sword Aiwass → manda Aleister al Cimitero, Artemis → cerca Crowley.
  - Contano anche nella raggiungibilità e nella probabilità.
- **Aggiornamento progressivo.**
  - L'evento `deck-links-progress` ora porta la mappa intera fin lì (`map`) e il deck (`deckId`).
  - Con `done` 0 c'è la bozza dal testo; poi arriva una mappa aggiornata dopo ogni mano cercata.
  - L'interfaccia può mostrare subito la bozza e aggiornarla mentre il motore gioca.
- **Ricalcolo delle sole carte cambiate** (`deck_links_again`, `LinksWork`).
  - Quando il deck cambia si tengono le mani cercate prima che hanno ancora tutte le loro carte e le cui linee non usano carte tolte. Si cercano solo le mani che servono perché ogni carta abbia di nuovo le sue.
  - Le linee tenute non vedono le carte aggiunte; quelle hanno mani proprie, e il testo dice dove portano.
  - L'app tiene il lavoro fatto per ogni deck nella cache e lo riusa alla modifica successiva. Nella CLI: `links --deck <nuovo> --after <vecchio>`.
  - Sul deck Invoked, con una Ash Blossom cambiata in Called by the Grave: 2 mani da cercare invece di 23, 3,2 secondi invece di 18,5, stessa mappa (116 collegamenti, 74 usati).

## Terzo giro del collaudo (21/10/2026)
- **Materiali della bozza dal testo.** Ora devono essere mostri. Magie e Trappole con l'archetipo nel nome ("R.B. Stage Landing", "Artmage Vandalism -Assault-") risultavano materiali: 19 collegamenti in 2 deck su 6.
- **Effetti ritardati.** Un effetto che si applica dopo, nella Fase Finale (Bystial Magnamhut che aggiunge un Drago), non ha più come causa l'ultima carta giocata: la causa si azzera a ogni cambio di fase. Prima Branded Banishment risultava "cercare" Starliege Seyfert.
- **Numeri del collaudo con la bozza dal testo** (6 deck): i tempi scendono (branded 62 s, earth_machine 64 s); 112 collegamenti su 1118 vengono solo dal testo.

## Confronto tra versioni del deck (21/10/2026)
Approvato dal proprietario come ultimo pezzo della mappa: cosa cambia sostituendo delle carte.
- **Cosa si confronta** (`compare_links` in `links.rs`):
  - collegamenti guadagnati e persi; un collegamento è lo stesso se ha la stessa carta di partenza, di arrivo e lo stesso tipo;
  - carte aggiunte e tolte;
  - per le carte che restano: probabilità di arrivarci, carte chiave e raggiungibilità prima e dopo. Contano solo le differenze di almeno 0,02 (sotto è rumore delle mani cercate) o una raggiungibilità diversa; le più cambiate vengono prima;
  - quante carte raggiunge in media una mano iniziale (somma delle probabilità), prima e dopo.
- **Velocità.** La seconda versione si legge riusando le mani cercate per la prima (`deck_links_again`). Se la prima è già nella cache dell'app non costa nulla; la seconda costa pochi secondi quando cambiano poche carte.
- **Nell'app.** Comando `deck_links_compare(deckIdA, deckIdB)` → `LinksComparison` (`api.deckLinksCompare`), con le due mappe intere (`before`, `after`) e le differenze. Gli eventi di avanzamento arrivano per tutti e due i deck, ognuno con il suo `deckId`.
- **Nella CLI.** `links --deck <nuovo> --after <vecchio> --diff`.
- **Prova sul deck Invoked** (lista EWCQ 2026 senza le 3 Magical Meltdown):
  - 1,8 secondi per la seconda versione, contro 20 della prima;
  - persi Meltdown → cerca Aleister e Terraforming → cerca Meltdown; Terraforming non porta più a nessuna carta (raggiungibilità da 14 a 0);
  - Aleister in mano iniziale dall'88% al 77%, una mano raggiunge in media 17,55 carte invece di 18,02;
  - le carte che non portano a nulla (Ash Blossom) salgono dal 34% al 36%, perché il deck ha 3 carte in meno.

## Giro su 37 deck dopo la PR #33 sulla velocità (22/10/2026, notte)
`duelforge-cli links` con le impostazioni di default (2 mani per carta, 1000 nodi), sui 37 deck di Master Duel Meta:
- **Tempi.** Nessun errore; in media 23 secondi a deck, al massimo 56 (yummy_mimighoul, warrior_combo). Branded scende da 62-63 a 48 secondi, Invoked 12.
- **Da guardare:**
  - **probabilità sature:** carte con `chance` ≥ 0,99, cioè quasi ogni mano iniziale ci arriva. Vaylantz ne ha 24, Memento 15, Invoked 15, Cyber Dragon 5. Lì la probabilità distingue poco: si potrebbe mostrare la differenza tra 5 e 6 carte, o i passi necessari;
  - **collegamenti solo dal testo:** D/D/D ne ha 228 su 373, Exosister 58 su 96. Le frasi d'archetipo ("D/D" monster) collegano quasi ogni carta a quasi ogni altra. Vanno controllati: se il testo è giusto ma le linee non li usano mai, l'interfaccia li mostra tenui, ma così sono tanti;
  - **Maliss:** solo 13 collegamenti su 29 carte, ma è corretto. La lista ha quasi solo Phantom Knights e Ignister (di Maliss c'è solo Maliss in Underground), 13 handtrap e Magie generiche, e mostri Link con materiali generici che nessun testo nomina.

