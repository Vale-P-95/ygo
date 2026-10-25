# Anatomia degli effetti: leggere le carte per il deck building e per il motore

Richiesta di Valerio del 24/10/2026: un codice che legga le carte e ne separi costo, condizione o richiesta di attivazione, azione, bersagli, quantità e restrizioni; da mostrare nel deck building e da far usare al motore, che a volte non capisce la forza di un effetto (Hyperinvoked Aeon, che bandisce fino a 3 carte dall'Extra Deck avversario). Questo documento è solo analisi e progetto: nessun codice.

## Decisione proposta
- **Fonte principale: gli script Lua che il motore carica già**, letti mentre si registrano, non il testo inglese. Ogni effetto di EDOPro dichiara tipo, evento, zona, limite per turno, proprietà (bersaglio, ritardo) e categorie (bandisce, nega, cerca, distrugge…); quando si attiva dichiara anche quantità, giocatore e zona colpita (`Duel.SetOperationInfo`). Sono dati del motore stesso: nessuna regola per nome di carta.
- **Fonte secondaria: il testo PSCT di Konami**, diviso con le sue regole di punteggiatura (`:` condizione, `;` costo, il resto è l'azione). Serve alla vista per sottolineare le parti del testo e per le restrizioni scritte («you cannot Special Summon … except»), che lo script nasconde dentro una funzione.
- **Nel deck building**: una sezione «Anatomia» nell'ispettore della carta e filtri del catalogo per categoria, ricavati dai dati invece che dalle liste scritte a mano di `ygo-data/src/tags.rs`.
- **Nel motore**, in ordine di costo: (1) l'anatomia come priore per l'ordine delle mosse; (2) un valore di riserva per le carte che il valore misurato non vede; (3) il peso delle carte tolte all'avversario preso dai valori delle carte del suo deck, nel turno di prova contro un deck scelto. Ogni passo che tocca la misura si giudica con la prova equa sulle 406 board.

## 1. Che cosa c'è già nel repository
- **Il caso Aeon è già studiato** in `ricerca-letteratura.md` («Il caso Aeon, passo per passo») e in parte corretto da B5a (`attrito-avversario.md`): Extra Deck del kit a 15 carte, `Measure::attrition`, lo stesso termine in `quick_score`. Oggi la board con il bando sta in cima (2,61 contro 2,01, test `tests/attrition.rs`).
- **Che cosa resta debole, e qui serve leggere le carte:**
  - il peso per carta è piatto e provvisorio (Extra Deck 0,04 a carta: tre carte valgono 0,12, qualunque esse siano);
  - i valori delle carte (`value.rs`) piazzano Aeon con materiali generici e senza i trigger di Evocazione: vale 0,04, mentre un mostro dell'Extra Deck non misurato vale 2,0. Il valore misurato è affidabile per ciò che la carta fa *sul campo nel turno avversario*, cieco per ciò che fa *arrivando*;
  - la ricerca non sa, prima di attivare un effetto, che cosa farà: lo scopre solo giocandolo.
- **Il turno di prova classifica già le interruzioni** guardando gli esiti (`probe.rs`, `enum Effect`: nega, bandisce, rimanda nel Deck…, con pesi da 0,4 a 1,0). È una lettura dinamica e affidabile, ma vede solo ciò che il kit provoca.
- **Gli strumenti per leggere gli script ci sono:** `ScriptStore` carica dopo gli script di base delle funzioni Lua nostre (`Replacement`, `scripts.rs`), e il motore manda al nostro `log_handler` ogni `Debug.Message` (`duel.rs`). Quindi si può osservare la registrazione degli effetti senza toccare `vendor/ygopro-core`.
- **`carddb.rs` non legge la colonna `category` di `datas`**: in `cards.cdb` c'è una maschera di categorie per carta (quella dei filtri «effetto» di EDOPro). È grossolana (per carta, non per effetto) ma gratuita: va guardata sui dati veri prima di decidere se basta per i filtri.

## 2. Le due fonti
### 2.1 Gli script Lua, letti dal motore (fonte principale)
Un effetto di EDOPro nasce in `initial_effect` con `Effect.CreateEffect` e i suoi `Set*`, poi `c:RegisterEffect(e)`. Nella versione upstream di ygopro-core (controllata su `libeffect.cpp` di `master`; la versione del submodule va ricontrollata) esistono i getter `GetType`, `GetCode`, `GetCategory`, `GetProperty`, `GetRange`, `GetCountLimit`, `GetDescription`, `GetCost`, `GetCondition`, `GetTarget`, `GetOperation`.

**Lettura statica (una volta per carta).** Una funzione Lua caricata come `Replacement` avvolge `Card.RegisterEffect` e, in un «duello di lettura», scrive per ogni effetto una riga con `Debug.Message`:

| Campo | Da dove | Che cosa dice |
|---|---|---|
| tipo | `GetType` | Ignizione, Innescato (obbligatorio o facoltativo), Rapido, Continuo, Campo, Equip, Attivazione di Magia/Trappola |
| evento | `GetCode` | «quando è Evocato», «quando va al Cimitero», «quando l'avversario attiva»… |
| zona | `GetRange` | da dove si usa: mano, campo, Cimitero, bandite |
| limite | `GetCountLimit` | quante volte per turno, per nome o per effetto (HOPT, soft o hard) |
| proprietà | `GetProperty` | bersaglio (`CARD_TARGET`), ritardo (`DELAY`, quindi niente timing perso), danno |
| categorie | `GetCategory` | bandisce, nega attivazione/effetto/Evocazione, distrugge, cerca, aggiunge alla mano, Evoca, pesca, rimanda nel Deck… |
| costo, condizione, bersaglio | `GetCost`, `GetCondition`, `GetTarget` | solo *se ci sono* (sono funzioni: il corpo non si legge) |
| descrizione | `GetDescription` | l'indice della stringa `strN` della carta: è lo stesso numero che portano le attivazioni offerte dal motore (`actions.rs`, `described`), quindi ogni scelta della ricerca si collega al suo effetto |

**Lettura dinamica (quando l'effetto si attiva).** La funzione `target` degli script dichiara che cosa farà: `Duel.SetOperationInfo(0, CATEGORY_REMOVE, nil, 3, 1-tp, LOCATION_EXTRA)` dice «bandisce 3 carte dall'Extra Deck avversario». Avvolgendo `Duel.SetOperationInfo` e `Duel.SetPossibleOperationInfo` si ottengono quantità, giocatore e zona, che la lettura statica non può dare. Va accesa solo dove serve (vedi §4), perché ogni riga passa per il log.

**Limiti noti.** Le restrizioni («per il resto del turno non puoi Evocare Specialmente dall'Extra Deck, eccetto…») sono effetti registrati sul giocatore dentro `operation`, quindi si vedono solo dinamicamente (`Duel.RegisterEffect` avvolto allo stesso modo). Le procedure di Evocazione (`Fusion.AddProcMix`, `Xyz.AddProcedure`, `Link.AddProcedure`) sono effetti anche loro: vanno riconosciute e mostrate a parte come «materiali». Le categorie sono dichiarate da chi ha scritto lo script e qualche script ne omette una: per questo c'è la seconda fonte e il confronto tra le due.

### 2.2 Il testo PSCT (fonte secondaria)
Il Problem-Solving Card Text di Konami ha una grammatica regolare:
- frasi separate da `.`; ogni effetto è una frase o un gruppo di frasi;
- prima dei due punti `:` la condizione o il momento di attivazione («If this card is Fusion Summoned:»);
- prima del punto e virgola `;` il costo o l'azione fatta all'attivazione, compresa la scelta dei bersagli («You can discard 1 card; …», «target 1 card on the field;»);
- dopo, l'azione; «and if you do» lega azioni dipendenti, «then» le rende in sequenza, «also» indipendenti;
- clausole riconoscibili: «(Quick Effect)», «You can only use this effect of <nome> once per turn», «You can only use each effect of <nome> once per turn», «You cannot Special Summon … except», «must be <tipo> Summoned», la riga dei materiali dei mostri dell'Extra Deck;
- quantità e luoghi in forme fisse: «up to 3», «1 card», «from your opponent's Extra Deck», «face-down».

Un segmentatore a regole (Rust, in `ygo-data`) basta per la vista: divide il testo, marca costo, condizione, azione, restrizione, limite, quantità e luoghi, e non deve capire tutto. Dove il testo non segue lo schema (carte vecchie, testi lunghi), la vista mostra solo le parti riconosciute. I testi restano in `data/`, mai nel repository: le prove usano poche carte in fixture con testi scritti da noi nello stesso stile.

**Accordo tra le fonti.** Lo stesso effetto si riconosce nelle due fonti dall'ordine e dalla descrizione (`strN` è spesso un riassunto della frase). Le categorie Lua e i verbi PSCT si confrontano: un disaccordo («il testo dice banish, lo script non ha `CATEGORY_REMOVE`») si conta e si mostra come «da verificare», mai si corregge a mano per nome.

### 2.3 Letteratura e strumenti simili
- **Forge (Magic: The Gathering)** scrive le carte in un linguaggio strutturato (`A:AB$ Destroy | Cost$ … | ValidTgts$ …`) e la sua IA valuta le abilità leggendo quei campi, non il testo. È lo stesso schema qui proposto: gli script di EDOPro sono il nostro equivalente strutturato, ma vanno *osservati* perché sono codice, non dati.
- **Dal testo al codice**: Ling et al. 2016, «Latent Predictor Networks for Code Generation» (testi di Hearthstone e Magic tradotti nel codice dei loro simulatori). Mostra che il testo regolare di questi giochi si presta, ma con reti e errori; per noi il codice esiste già, quindi il problema inverso (codice → struttura) è più semplice e affidabile.
- **Limiti teorici**: Churchill, Biderman e Herrick 2019 («Magic: The Gathering is Turing Complete»). Nessuna lettura statica descrive tutti gli effetti: per questo la misura vera resta il turno di prova, e l'anatomia è un priore e una spiegazione, non un giudice.
- Fonti da verificare prima di citarle nei documenti di misura: nessuna; le tre sopra sono note e servono solo da contesto.

## 3. Nel deck building
**Sezione «Anatomia» nell'ispettore della carta** (`CardInspector.tsx`), sotto il testo:
- Una riga per effetto: tipo (chip: Ignizione, Innescato, Rapido, Continuo), zona da cui si usa, limite («1 volta per turno, per nome»).
- Quattro campi, ognuno solo se c'è: **Condizione** (quando), **Costo**, **Azione** (verbo con categoria, quantità e luogo: «bandisce fino a 3 · Extra Deck avversario · coperte»), **Restrizione**.
- Bersaglio sì/no come icona: conta per le carte che lo evitano (ad esempio chi non può essere bersaglio).
- **Tasto «Anatomia» nel dettaglio della carta** (richiesta di Valerio del 25/10): un'icona accanto al testo che, accesa, colora nel testo inglese le parti che parlano di condizione, costo, azione, restrizione e limite, ognuna col suo colore (token di `index.css`, temi chiaro e scuro), con una piccola legenda. Spenta, il testo resta com'è. Lo stato del tasto si ricorda per sessione. Le etichette sono in italiano, il testo della carta in inglese.
- Un «da verificare» discreto dove le due fonti non concordano.

**Filtri del catalogo per categoria**: «nega», «bandisce», «cerca», «Evoca dal Deck», «tocca l'Extra Deck avversario», «effetto rapido», «dalla mano», «dal Cimitero». Sostituiscono nel tempo le liste di nomi di `tags.rs` per gli handtrap (mostro + zona mano + tipo Rapido o Innescato nel turno avversario) e i breaker; le liste restano finché i filtri nuovi non le coprono.

**Riepilogo del deck** (barra `DeckInsightsBar`): quante carte negano, cercano, Evocano dal Deck; quante restrizioni di Evocazione ha il deck e quali si contraddicono (una carta che limita all'archetipo, un'altra che Evoca fuori archetipo). Questo è il secondo passo della vista, dopo l'ispettore.

Prima del codice serve un mockup di Claude Design (artboard 1280×800, due temi) per la sezione dell'ispettore.

## 4. Nel motore
L'anatomia non sostituisce la misura: il turno di prova resta il giudice di ciò che una board ferma. Serve dove la misura è cieca o arriva tardi. Tre usi, dal più economico.

### 4.1 Priore per l'ordine delle mosse (classe R)
Ogni attivazione offerta porta la sua descrizione, quindi il suo effetto e le sue categorie. Nell'ordine delle discese (`dive_order`) le attivazioni che dichiarano categorie verso l'avversario (bandisce, rimanda, distrugge, nega) o che portano risorse (cerca, Evoca dal Deck, pesca) passano prima di quelle senza categorie (cambi di Attributo, di posizione). Il caso Aeon: il trigger col bando e l'effetto rapido che cambia Attributo condividono il limite per turno; il priore fa provare prima il bando. Non cambia la misura, solo quali linee si trovano prima a parità di nodi. Si giudica con il banco congelato (`banco.py judge`).

### 4.2 Valore di riserva per ciò che la carta fa arrivando (classe M)
Il valore misurato di una carta dell'Extra Deck dice che cosa fa *dopo*; quello che fa *arrivando* (trigger di Evocazione) si vede solo nella linea. Proposta: nei valori delle carte, un mostro con un trigger di Evocazione che dichiara categorie verso l'avversario o di risorse riceve per quella parte un valore di riserva, calcolato in modo generale:

valore di riserva = somma, sugli effetti di arrivo, di peso della categoria × quantità dichiarata × peso della zona colpita × affidabilità

- peso della categoria: la scala di `Effect` di `probe.rs` che c'è già (bandisce 0,85, distrugge 0,7…);
- quantità e zona: da `SetOperationInfo`, letti la prima volta che l'effetto si attiva in una ricerca e tenuti con i valori;
- peso della zona: la tabella dell'attrito di B5a (Extra Deck 0,04, mano 0,15, campo 0,3, Deck 0,01);
- affidabilità: minore se l'effetto chiede un bersaglio, un costo, una condizione sui materiali, o se è facoltativo dopo un altro.

È un valore *di riserva*: si usa solo quando il valore misurato è sotto la riserva, e solo nella stima della ricerca (`CardValues::get` e `quick_score`), mai nella misura finale, che resta quella del turno di prova e dell'attrito vero. Si giudica con la prova equa sulle 406 board e `paired_test.py`.

### 4.3 Peso delle carte tolte, dal deck avversario (classe M, dopo 4.2)
Il dubbio di Valerio è giusto anche dopo B5a: tre carte dell'Extra Deck valgono 0,12 qualunque esse siano. Nel turno di prova contro un deck scelto (`turno-di-prova-matchup.md`) le carte bandite sono vere: il loro peso può essere il valore misurato *di quelle carte* nel deck avversario (gli stessi `CardValues`, calcolati per il suo deck), invece della media per pila. Con il kit generico resta la tabella di B5a. Nessun nome: il peso viene dalla misura delle carte avversarie.

### 4.4 Che cosa non fare
- Nessun punteggio scritto a mano per categoria oltre a quelli che esistono (scala di `Effect`, tabella dell'attrito). Se ne serve uno nuovo, si tara con D17, come gli altri.
- Nessuna lettura dell'anatomia nella misura finale: spiegare sì, giudicare no.
- Nessun parser che pretenda di capire tutte le carte: ciò che non si riconosce resta «sconosciuto» e il motore fa come oggi.

## 5. Ordine di lavoro e brief
Tre sessioni, una dopo l'altra (la seconda e la terza dipendono dalla prima; possono andare in parallelo tra loro).

### Brief A: lettore degli effetti dagli script (motore, classe E)
```
Compito: duelforge-cli anatomy --card <code> | --deck <deck> | --all scrive l'anatomia Lua di ogni effetto (tipo, evento, zona, limite, proprietà, categorie, costo/condizione/bersaglio presenti, descrizione) in JSON, più out/anatomia/summary.txt con la copertura.
Perché: docs/plans/anatomia-effetti.md §2.1 e §5.
Branch: claude/anatomia-lua, da main
File da toccare: crates/ygo-engine/src/scripts.rs (una Replacement che avvolge Card.RegisterEffect), crates/ygo-engine/src/anatomy.rs (nuovo), crates/ygo-engine/src/lib.rs, crates/ygo-cli; test con fixture.
Da leggere: anatomia-effetti.md §2.1; scripts.rs (Replacement), duel.rs (log_handler), actions.rs (described). Prima cosa: verificare i getter Effect.Get* nel submodule.
Verifica: cargo test -p ygo-engine -p ygo-cli; anatomy --all sui dati veri; search_regression; banco.py same (le uscite non cambiano: il lettore è spento nella ricerca).
Criterio di accettazione: oltre il 95% delle carte con script letto senza errori; Hyperinvoked Aeon con un trigger di Evocazione Fusion con CATEGORY_REMOVE e il limite condiviso con l'effetto rapido; ogni attivazione di una ricerca collegata al suo effetto dalla descrizione.
PR: template; summary.txt nella PR; classe E.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```

### Brief B: segmentatore PSCT e sezione «Anatomia» nell'ispettore (interfaccia e dati)
```
Compito: l'ispettore della carta mostra la sezione «Anatomia» (per effetto: tipo, zona, limite, condizione, costo, azione con quantità e luogo, restrizione, bersaglio) unendo l'anatomia Lua del Brief A e un segmentatore PSCT; nel dettaglio della carta un tasto con icona «anatomia» colora nel testo inglese condizione, costo, azione, restrizione e limite, con la legenda.
Perché: docs/plans/anatomia-effetti.md §2.2 e §3.
Branch: claude/anatomia-ispettore, da main dopo il Brief A
File da toccare: crates/ygo-data/src/psct.rs (nuovo), apps/desktop/src-tauri/src/lib.rs e src/lib/api.ts (comando card_anatomy: file caldi, chiederli in TASKS.md), src/lib/mock.ts, src/lib/fixtures/, src/components/CardInspector.tsx.
Da leggere: anatomia-effetti.md §2.2-§3; mockup di Claude Design del dettaglio della carta con il tasto acceso e spento e della sezione (da fare prima: artboard 1280×800, due temi).
Verifica: cargo test -p ygo-data psct (testi di prova scritti da noi, niente testi Konami nel repo); pnpm typecheck; pnpm build; VITE_MOCK=1 + node scripts/screens.mjs, pagina Deck builder nei due temi; app vera con DUELFORGE_DATA.
Criterio di accettazione: su 10 carte di prova (handtrap, Fusion, Link, Magia rapida, Trappola continua) i quattro campi giusti; disaccordi Lua/PSCT contati e mostrati come «da verificare».
PR: template; screenshot nei due temi, anteprima Artifact.
Modello: Sonnet 5.5, effort medio.
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```
I filtri del catalogo per categoria e il riepilogo del deck sono un brief successivo, dopo che Valerio ha visto la sezione.

### Brief C: priore delle mosse e valore di riserva (motore, classe R poi M)
```
Compito: (1) dive_order prova prima le attivazioni con categorie verso l'avversario o di risorse (classe R); (2) in una seconda PR, il valore di riserva dei trigger di arrivo in CardValues e quick_score, con quantità e zona da SetOperationInfo (classe M).
Perché: docs/plans/anatomia-effetti.md §4.1-§4.2.
Branch: claude/anatomia-motore, da main dopo il Brief A
File da toccare: crates/ygo-solver/src/search.rs, value.rs (file caldi: chiederli in TASKS.md), crates/ygo-engine/src/anatomy.rs.
Da leggere: anatomia-effetti.md §4; attrito-avversario.md; regole-motore.md.
Verifica: banco.py run … --set full in background, poi judge (R) e judge --measure-cli (M); prova equa sulle 406 board con paired_test.py; search_regression; tests/attrition.rs.
Criterio di accettazione: ACCEPTED sul banco e 406 board non peggiorate; nella mano Invoked di PIANO.md la linea con Aeon a 3 materiali e il bando trovata con meno nodi di main.
PR: template; summary.txt per intero, mani che perdono oltre il 5% con il perché.
Modello: Opus 5.5, effort alto.
Quando hai finito: TASKS.md e questo piano aggiornati, PR aperta, fermati.
```
Il §4.3 (peso delle carte tolte dal deck avversario) va dopo il livello 2 del turno di prova contro un deck scelto e dopo il Brief C.

## 6. Rischi
- **Versione del motore**: i getter vanno verificati nel submodule; se ne manca uno, il campo resta vuoto, non si cambia `vendor/`.
- **Costo**: il duello di lettura carica ogni script una volta (come i valori delle carte); la lettura dinamica passa per il log, quindi si accende solo per le carte non ancora lette, e si tiene in memoria con i valori.
- **Script che registrano effetti in modo indiretto** (procedure, effetti globali, `aux.*`): vanno riconosciuti come gruppo e contati nel `summary.txt`, non risolti uno per uno.
- **Aspettative**: per Aeon il guadagno vero è già in parte arrivato con B5a; l'anatomia lo rende più rapido da trovare (4.1), più stabile nella stima (4.2) e più giusto contro un deck vero (4.3). Se il banco non migliora, il priore e la riserva non si uniscono; la vista nel deck building resta utile da sola.

## 7. Brief A fatto (25/10/2026)
- **Cosa c'è**: `crates/ygo-engine/lua/anatomy.lua` avvolge `Card.RegisterEffect` solo nei duelli di lettura (mai avviati né riusati); `anatomy.rs` legge le righe del log (`read_anatomy`, `CardAnatomy`, `EffectAnatomy`, `EffectKind`), `effect_of` collega un'attivazione al suo effetto. `duelforge-cli anatomy --card <nome|passcode> | --deck <ydk> [--link <turni>] | --all [--json] [--out]`. La ricerca non lo usa: classe E.
- **Getter**: tutti quelli del §2.1 ci sono nel submodule. Le costanti lette sono controllate contro `constant.lua` dal test `tests/anatomy.rs`.
- **Copertura** (`anatomy --all`, 14 377 carte senza le illustrazioni alternative): tutte lette senza errori, 34 766 effetti, 21 379 attivati, l'82,6% di questi con almeno una categoria. Tempo: 2,5 s per tutto il database (0,17 ms a carta).
- **Collegamento attivazione → effetto** (63 deck del banco congelato, 20 turni a caso per deck): 11 154 attivazioni su 11 498, il 97,0%. Restano le domande «attivare l'effetto di…?» dei trigger facoltativi con la stringa di sistema 221, sulle carte con più trigger diversi: la domanda non dice quale. Lo dice l'evento `Chaining` che segue la risposta (porta la descrizione): è il modo per il Brief C. Alcuni effetti hanno come descrizione una stringa di sistema (1170, «Fusion Summon» di Invocation): si cercano prima per numero esatto.
- **Il caso Aeon, una scoperta**: lo script di Hyperinvoked Aeon registra tre effetti (la procedura, il trigger di Evocazione Speciale con `CATEGORY_REMOVE`, l'effetto rapido che cambia l'Attributo, con lo stesso limite per turno). **Il bando dall'Extra Deck non è un effetto a parte**: sta dentro l'operazione del trigger. La lettura statica vede «bandisce» e il limite condiviso, non quante carte né da dove. Per il §4.2 serve quindi la lettura dinamica di `SetOperationInfo`, nel Brief C, o il segmentatore PSCT del Brief B.
- **Prossimo passo**: Brief B (prima il mockup del tasto «Anatomia»), poi C.

## 8. Brief B fatto (25/10/2026)
- **Mockup**: https://claude.ai/artifact/P17uXCzpHkPyqCLKhzmdNd (dettaglio della carta con Hyperinvoked Aeon e ispettore con Ash Blossom, due temi). Il tasto «Anatomia» (richiesta di Valerio del 25/10) sta accanto al titolo del testo, nel visore della carta (`CardDetails`) e nell'ispettore della Fucina (`CardInspector`); lo stato vale per la sessione.
- **Lettore del testo**: `crates/ygo-data/src/psct.rs` (`read`, `Piece`, `Part`, `TextEffect`, `Frame`). Su tutte le 14 321 carte del database del motore classifica il 98,8% dei caratteri; nell'81,6% delle carte gli effetti attivati del testo sono tanti quanti quelli dello script (raggruppati per descrizione) e si collegano uno a uno; dove si collegano, l'83,9% dei verbi del testo (bandisce, nega, distrugge…) è dichiarato anche dallo script. Restano fuori soprattutto i testi vecchi, non scritti in PSCT.
- **Contratto**: comando `card_anatomy` (`lib.rs`: `AnatomyView`, `EffectView`, `LimitView`; la logica in `anatomy_view`, provata su Aeon con i dati veri) ↔ `CardAnatomy`, `EffectAnatomy`, `TextPiece` in `api.ts`; `mock.ts` divide i testi finti con le stesse regole di punteggiatura e prende gli effetti da `fixtures/anatomy.json` (letti con `duelforge-cli anatomy` sul catalogo finto, senza le stringhe delle carte).
- **Differenze dal mockup**: sotto ogni effetto si mostrano i pezzi del testo inglese (condizione, costo, azione), non una parafrasi italiana, che richiederebbe di capire il testo. «Solo dal testo» compare per i verbi che il testo nomina e lo script non dichiara; per Aeon non compare, perché lo script dichiara «bandisce» (manca la quantità e il luogo, che verranno dalla lettura dinamica del Brief C). Dove testo e script non combaciano uno a uno, le parti restano colorate nel testo e una riga lo dice.
- **Verifica**: test di `psct.rs` (testi di prova scritti da noi), test di `anatomy_view` su Aeon con i dati veri, test dei componenti (`CardAnatomy.test.tsx`), screenshot del backend finto nei due temi, anteprima Artifact. L'app vera non è stata provata cliccando (nessun desktop nella sessione cloud).
- **Prossimi passi**: i filtri del catalogo per categoria e il riepilogo del deck (§3), dopo che Valerio ha provato il tasto; poi il Brief C.

## 9. Filtri del catalogo per effetto (25/10/2026)
- **Ricerca**: `eff:` nella lingua della ricerca (`eff:negate`, `eff:gy`, alias italiani come `effetto:cimitero`), dieci filtri in `crates/ygo-data/src/effects.rs` (`EffectFilter`): nega, distrugge, bandisce, cerca nel Deck, pesca, Evoca dal Deck, Extra Deck avversario, effetto rapido, dalla mano, dal Cimitero. Nel pannello «Filtri» del catalogo c'è il gruppo «Che cosa fa»; «Dalla mano» con «Effetto rapido» trova gli handtrap senza liste di nomi.
- **Fonte**: per ogni carta una maschera da `effect_mask`, che unisce le categorie e le zone degli effetti attivati dello script (brief A) con il testo (brief B): il testo aggiunge ciò che lo script lascia dentro l'operazione, come il bando dall'Extra Deck avversario di Aeon e le Evocazioni dal Deck. Senza i dati del motore resta il solo testo.
- **Contratto invariato**: i filtri passano nel testo della ricerca; `search_cards` (`lib.rs`) scarta le carte la cui maschera non li ha tutti. Le maschere si leggono alla prima ricerca con `eff:` (`effect_index.rs`: 14 412 carte in 4,9 s in release) e si rileggono dopo un aggiornamento dei dati. `CardStore::search` ignora `eff:`: la CLI non li applica.
- **Numeri sui dati veri** (carte trovate, script e testo / solo testo): nega 1512/1452, distrugge 4009/3882, bandisce 1562/1442, cerca 1791/1379, pesca 848/738, Evoca dal Deck 1464/1464, Extra Deck avversario 29/29, rapido 2027/1150, dalla mano 1521/1242, dal Cimitero 1701/1147.
- **Resta**: il riepilogo del deck (quante carte negano, cercano, Evocano dal Deck; restrizioni che si contraddicono), che chiede un comando nuovo nel contratto; e la sostituzione delle liste di `tags.rs`, quando i filtri saranno provati nell'app vera.
