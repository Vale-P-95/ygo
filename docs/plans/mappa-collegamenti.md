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
  - **collegamenti solo dal testo:** D/D/D ne aveva 228 su 373, Exosister 58 su 96, Dracotail 47 su 219. Controllati, avevano due difetti della bozza dal testo, corretti in `clause_kind`:
    - la zona era presa da tutto il resto della frase dopo «from your»: «… from your hand and/or Deck…, also you cannot Special Summon monsters from the Extra Deck» diventava un'Evocazione dall'Extra Deck. Ora sono solo le zone elencate subito dopo («hand, Deck, or GY»);
    - un divieto («you cannot Special Summon monsters from your Extra Deck, except "D/D" monsters») diventava un collegamento verso ogni mostro nominato. Ora una frase con «cannot» o «can't» prima di «from your» non porta collegamenti.
    - Collegamenti solo dal testo dopo la correzione: Dracotail 47 → 17, D/D/D 228 → 218, Exosister 58 → 51, Artmage 47 → 41; Invoked invariato. Nessun collegamento confermato dal motore si perde.
    - Quelli di D/D/D che restano sono veri: effetti d'archetipo con condizioni che nelle mani provate non si verificano (Gilgamesh distrutto: «Special Summon 1 "D/D" monster from your Extra Deck or GY»);
  - **Maliss:** solo 13 collegamenti su 29 carte, ma è corretto. La lista ha quasi solo Phantom Knights e Ignister (di Maliss c'è solo Maliss in Underground), 13 handtrap e Magie generiche, e mostri Link con materiali generici che nessun testo nomina.


## Probabilità misurata sulle mani cercate (22/10/2026, corsia 6)
`chance` seguiva i collegamenti in un numero qualunque di passi. Così ignorava le risorse del turno: una sola Evocazione Normale, gli effetti «una volta per turno», le carte spese lungo il percorso. Era un limite superiore e si saturava. Ora è misurata sulle mani che la mappa cerca già.

### Come ho misurato la verità
- `duelforge-cli links --deck <d> --truth 200 --json <file>`: 200 mani iniziali di 5 a caso (seed fisso), cercate come la mappa (1000 nodi, 15 s, stesso kit avversario), con le linee migliori rigiocate.
- Una carta è «portata in gioco» se la linea migliore la tiene in mano dall'inizio, la aggiunge alla mano o la mette sul campo (Evocata, posizionata, coperta). La verità di una carta è la quota delle 200 mani in cui succede.
- Rumore della verità: confrontando metà delle mani con l'altra metà, circa 0,03 di errore medio. Sotto quella soglia le varianti non si distinguono.
- **Prime N linee invece della migliore:** con le prime 3 la quota sale in media di 0,01-0,02, con le prime 20 di 0,04-0,10. Ho tenuto la migliore: è quella che il giocatore gioca.
- Costo: 90-130 s per deck (318 s per elfnote_kewl_tune) su 4 core.
- Deck: invoked, snake_eye, elfnote_kewl_tune e psychics (i più saturi della lista di oggi; Vaylantz, Memento e Cyber Dragon non ci sono più), artmage, e branded che non satura. Poi blue_eyes, power_patron, live_twin e code_talker come controllo.
- `scripts/links_chance.py <cartella>` confronta le varianti con la verità: errore medio, carte sature, carte più sbagliate.

### Errore medio contro la verità
Carte del Main Deck; tra parentesi le carte con `chance` ≥ 0,99. Nessuna carta della verità arriva a 0,99 su questi 6 deck.

| variante | artmage | branded | elfnote_kewl_tune | invoked | psychics | snake_eye | media |
|---|---|---|---|---|---|---|---|
| prima: chiusura in ogni numero di passi | 0,350 (2) | 0,406 (0) | 0,377 (7) | 0,173 (6) | 0,341 (5) | 0,417 (8) | 0,344 |
| chiusura in 1 passo | 0,133 (0) | 0,125 (0) | 0,173 (0) | 0,048 (0) | 0,187 (0) | 0,156 (0) | 0,137 |
| chiusura in 2 passi | 0,273 (0) | 0,308 (0) | 0,338 (0) | 0,154 (1) | 0,285 (0) | 0,287 (0) | 0,274 |
| chiusura in 3 passi | 0,330 (0) | 0,387 (0) | 0,374 (1) | 0,172 (6) | 0,320 (2) | 0,376 (1) | 0,327 |
| chiusura sui soli collegamenti usati | 0,304 (1) | 0,372 (0) | 0,351 (2) | 0,151 (4) | 0,282 (2) | 0,417 (8) | 0,313 |
| quota delle mani cercate, pesata | 0,181 (0) | 0,108 (0) | 0,143 (0) | 0,127 (0) | 0,139 (0) | 0,082 (0) | 0,130 |
| la stessa, prime 3 linee | 0,162 (0) | 0,122 (0) | 0,139 (0) | 0,132 (0) | 0,135 (0) | 0,088 (0) | 0,130 |
| **tenuta esatta + resto misurato (scelta)** | 0,087 (0) | 0,049 (0) | 0,039 (0) | 0,058 (0) | 0,069 (0) | 0,052 (0) | **0,059** |
| la stessa, spinta verso la chiusura in 1 passo | 0,085 (0) | 0,048 (0) | 0,040 (0) | 0,055 (0) | 0,069 (0) | 0,052 (0) | 0,058 |
| la stessa, con sole mani nuove (vedi sotto) | 0,052 (0) | 0,040 (0) | 0,057 (0) | 0,059 (0) | 0,055 (0) | 0,052 (0) | 0,053 |

- Con l'Extra Deck l'errore medio passa da 0,374 a 0,057.
- **Deck di controllo:** blue_eyes da 0,171 a 0,102, power_patron da 0,172 a 0,077, live_twin da 0,119 a 0,048, code_talker da 0,509 a 0,049.
- **Le carte più sbagliate prima** erano quelle che il deck *può* portare in gioco ma una mano non porta quasi mai, perché il percorso richiede risorse già usate. Snake-Eye Ash 0,92 contro 0,24 vero; Branded Opening 0,94 contro 0,09; Virakam 1,00 contro 0,28; Gamma Summon 1,00 contro 0,30; Elfnote Fortuna 0,99 contro 0,40.
- **La chiusura in 1 passo** va bene solo su Invoked, dove i percorsi sono corti. Sugli altri deck sbaglia per eccesso: un collegamento possibile non è un collegamento giocato. In 2 o 3 passi torna verso la saturazione.
- **La quota misurata da sola** sbaglia sulle carte senza collegamenti, come le handtrap: le poche mani cercate che le tengono pesano troppo. Per questo la tenuta si conta esatta.

### La misura scelta
- `chance` = probabilità esatta di tenere la carta + (1 − quella) × quota delle mani cercate *senza* la carta la cui linea migliore la porta in gioco. Per un mostro dell'Extra Deck è tutta misurata.
- **Pesi delle mani.** La mappa cerca mani che contengono una carta data: quella carta più 4 a caso. Una mano così esce tanto più spesso quante copie della carta tiene, diviso le copie nel deck. Su tutte le carte, una mano conta quindi 1 / (somma sulle sue carte di 1 / copie). Con questi pesi le quote sono quelle di mani a caso: c'è un test unitario che lo verifica.
- **La bozza dal testo**, prima di ogni mano, usa la chiusura in 1 passo: è la migliore tra le chiusure.
- Ho provato anche a spingere la quota verso la bozza mentre le mani sono poche. Aiuta poco il Main Deck e peggiora l'Extra Deck, dove la bozza è debole. Non l'ho tenuta.
- Nessun caso per nome di carta: pesi e quote valgono per qualunque deck.

### Le mani della mappa erano quasi tutte uguali
- `sample_hands` usava lo stesso seed per ogni carta: le altre 4 carte si pescavano negli stessi punti del deck. Ogni mano era «la carta + le stesse 4 carte».
- Con un seed per carta l'errore scende a 0,053 e la mappa cerca meno mani (982 invece di 1561 sui 41 deck). Però cambiano i collegamenti trovati: 430 persi e 489 nuovi, alcuni molto usati (Junoldo → Medius, usato nel 35% delle linee). Con 4 mani per carta se ne perdono ancora 316.
- **Compromesso scelto:** le prime 2 mani di ogni carta restano quelle di prima (seed comune), e se ne aggiunge una terza con il seed della carta (`hands_per_card` passa da 2 a 3, anche nella CLI). I collegamenti di prima restano tutti; l'errore è 0,059 invece di 0,053.

### Giro sui 41 deck di Master Duel Meta
`scripts/links_report.py` prima e dopo, impostazioni di default.

| | prima | dopo |
|---|---|---|
| carte del Main Deck con `chance` ≥ 0,99 | 29 | 6 |
| mostri dell'Extra Deck con `chance` ≥ 0,99 | 20 | 2 |
| collegamenti | 5135 | 5345 (0 persi, 210 nuovi) |
| mani cercate | 1561 | 1905 |
| tempo per deck: media / mediana / massimo | 25,0 / 22,8 / 57,6 s | 30,7 / 27,4 / 63,0 s |
| deck oltre 60 s (tempo totale del processo) | 0 | 3 |

- Prima erano saturi snake_eye 8, elfnote_kewl_tune 7, invoked 6, psychics 5, artmage 2, doomz 1. Ora nessuno di questi.
- Restano a ≥ 0,99: power_patron 3 (Jupredo, Vidolium, Junoldo: 0,79-0,85 veri), blue_eyes 1 (Mausoleum of White), code_talker 1 (Salamangreat Sanctuary, satura anche nella verità), live_twin 1 (Fiendsmith Engraver, 0,90 vero), e due mostri dell'Extra Deck. Sono carte davvero molto raggiunte, sopravvalutate di 0,1-0,2 dalle mani a seed comune.
- **Tempi.** La terza mano costa 6 s in media. I 3 deck oltre i 60 s (earth_machine, ryu_ge, tearlaments) restano tagliati dal tetto della mappa, come prima: le mani in più si saltano per ultime, e i collegamenti delle prime due restano.
- **Ricerca delle combo invariata:** `combos --json --threads 1 --nodes 4000` sulle mani di `tests/fixtures/hands/bench.tsv` dà le stesse board, linee e misure prima e dopo. Le mani provate sono 12 su 16: le altre 4 usano deck che oggi non sono nella lista (ignister, labrynth, mitsurugi, yummy_mimighoul). In una mano (inv-r2) cambia solo l'ultima cifra decimale di `countered`, l'ordine di una somma. `search.rs` non è toccato.

### Cosa cambia per l'interfaccia
- Nessun campo nuovo. `chance` ha un significato nuovo, scritto nei commenti di `LinkCard` (`links.rs`, `src-tauri/src/lib.rs`, `api.ts`): «una mano iniziale porta in gioco la carta», non più «la può raggiungere».
- `measured` ora è vero per quasi tutte le carte appena c'è una mano cercata. L'interfaccia può mostrare tutte le probabilità come stime («circa 45%»), con un margine di ±0,05.
- La bozza dal testo (`done` 0) ha `measured` falso e la chiusura in 1 passo.
- Le probabilità si muovono un po' durante l'aggiornamento progressivo: è il rumore delle prime mani.

### Cosa resta aperto
- `key` (carte chiave) si calcola ancora sulla chiusura in ogni numero di passi. Con la `chance` misurata non si può negare una carta senza cercare di nuovo. Una via: `key` sulla chiusura in 1 passo, da validare come qui.
- Il confronto tra versioni (`compare_links`) ora vede anche il rumore delle mani: una carta tolta cambia le mani tenute. La soglia `CHANGE` di 0,02 è bassa per la misura nuova; forse 0,05.
- Mano di 5 e di 6 carte: non fatto. Servirebbe una seconda verità con 6 carte; la misura si estende con la tenuta esatta a 6 e le mani cercate di 6.
- La verità usa la ricerca a 1000 nodi, come la mappa. Con più nodi la linea migliore cambia un po'; non l'ho misurato.
- L'errore residuo, circa 0,06 contro 0,03 di rumore della verità, viene dalle poche mani della mappa (27-83 per deck). Più mani lo abbassano lentamente: con sole mani nuove, 0,056 a 2 mani per carta, 0,053 a 3 e 0,051 a 4.
