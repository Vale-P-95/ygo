# Ricerca delle combo: piano critico per board più forti in meno tempo

*Scritto il 22/10/2026 dal coordinatore, dalle prove del proprietario nell'app vera. Il proprietario giudica la ricerca lontana dallo standard che vuole: troppo lenta, con tempo perso su linee inutili, ripetitive o deboli, e board finali non all'altezza delle linee note. Aggiornato lo stesso giorno con la misura A2 (PR #67) e con la ricerca di letteratura e l'audit delle carte (`ricerca-letteratura.md`).*

## Cosa ha visto il proprietario
- **Invoked, ricerca per obiettivo.** Obiettivo: Invoked Oceanus nella Zona Mostri Extra e S:P Little Knight sul campo.
  - La ricerca trova una linea che lo raggiunge, ma passa per Hyperinvoked Aeon Evocato con 2 materiali.
  - Aeon vale molto di più con 3 materiali: bandisce dal campo e dai Cimiteri, e con 3 o più materiali anche fino a 3 carte dell'Extra Deck avversario.
  - Le guide (video) arrivano alla stessa board con Aeon a 3 materiali, passando per i Link Magistus e cercando Crowley dal Deck.
- **I tempi** per trovare le linee sono troppo lunghi.
- **Ruoli («Riconosci»):**
  - Super Polymerization non risulta board breaker;
  - una carta non può essere sia starter sia estensore;
  - Invocation «Grail» risulta solo board breaker, ma è anche un estensore.
- **Obiettivi:** si vogliono chiedere anche carte coperte nella Zona Magie e Trappole, carte nel Cimitero o bandite.

**Regola:** il motore trova da solo le linee forti. Il proprietario non scrive combo, e nessun riferimento dipende da linee scritte a mano.

## Due difetti diversi
1. **La misura non vede il valore di molte carte.** Aeon a 3 materiali è un caso di una famiglia più grande: l'audit su 37 deck del meta ha trovato 17 categorie di effetti che la misura o la stima della ricerca non vedono, o vedono in parte (`ricerca-letteratura.md`, §1).
   - Per Aeon le cecità sono cinque in fila:
     1. la stima della ricerca non guarda l'avversario;
     2. i valori delle carte danno ad Aeon quasi zero;
     3. il kit dell'avversario ha un solo mostro nell'Extra Deck, quindi togliere 1 carta vale come toglierne 3;
     4. l'app misura una sola variante per «aspetto» della board;
     5. le scelte nell'Extra Deck avversario si rigiocano per posizione.
   - Anche una ricerca perfetta non ha motivo di preferire la linea a 3 materiali.
2. **La ricerca spende nodi e tempo dove non serve.** Ora è misurato (A2, sotto): il replay delle linee, gli stati già visti e i passi senza cambi visibili.

## Fase A: misurare
### A1. Riferimenti automatici: l'oracolo (corsia 6, dopo i ruoli)
- **L'oracolo.** Per un insieme fisso di mani si lancia la stessa ricerca con un budget enorme: 50-100 volte i nodi, più semi e più thread, anche per ore, di notte (le corse lunghe le lancia il proprietario). Si tengono le board migliori trovate e la loro misura.
  - Le mani: le 16 del banco, le 82 di `combo_hunt --seed 7`, e le ricerche per obiettivo con obiettivi presi dalle board forti di quei deck.
- **La forza vera** delle board dell'oracolo viene dal banco oggettivo (livello 2 contro il meta), non solo dalla stima rapida.
- **Formato:** `tests/fixtures/oracle/<deck>-<mano>.json`, con mano, board migliori (carte e zone, senza testi), misura e linea.
- **Misure per ogni mano**, con la ricerca normale:
  - dopo quanti nodi e secondi arriva al 90%, al 95% e al 100% del valore dell'oracolo;
  - se trova la board dell'oracolo e in che posizione la mette;
  - nella ricerca per obiettivo, quanto è forte la linea trovata rispetto alla migliore che raggiunge lo stesso obiettivo.
- **Quando l'oracolo stesso non arriva a una board**, si rigiocano i passi per capire perché: un'azione che il motore non offre, una risposta automatica sbagliata, una potatura. Sono difetti da correggere in modo generale.
- **Controllo esterno, facoltativo:** [ygo-combo-solver](https://github.com/Armytille/ygo-combo-solver) gira sullo stesso motore e accetta obiettivi con vincoli sui materiali. Sul caso Invoked separa «il motore non offre l'azione» da «la nostra ricerca non la trova». Serve la stessa versione del core e degli script.
- **Test appaiato per accettare una modifica della fase B.** Vale da subito, anche prima dell'oracolo.
  - Stesse mani e stesso seme, vecchio contro nuovo: le 16 mani del banco e le 82 di `combo_hunt --seed 7`.
  - Per ogni mano: meglio, peggio o pari, con una soglia dell'1% sul valore della board migliore.
  - Si accetta se le mani peggiori non superano quelle migliori (test del segno) e nessuna perde più del 5%.

### A2. Dove va il lavoro della ricerca: fatto (corsia 2, PR #67)
`DUELFORGE_SEARCH_STATS=1` e `scripts/search_stats.py`; numeri in `velocita-ricerca.md`.

| Spreco | % nodi | % tempo della ricerca |
|---|---|---|
| Replay delle linee dal principio | — | 43% |
| Discese che finiscono su un nodo già esplorato (altre scelte 15,6%, altro ordine 9,3%) | 26,3% | 18% |
| Nodi senza cambi visibili (selezioni a più carte, Evocazione prima dei materiali) | 23,8% | — |

- **Ordinamento:** la risposta giusta è già la prima provata nel 77% dei passi della linea migliore.
- **Fuori dalla ricerca:** in `combo_hunt` il turno di prova è il 58% del tempo totale, le fasi dopo la ricerca il 15%, l'apprendimento il 4%.
- **Contatori da aggiungere:**
  - in che posizione di `dive_order` escono dalla coda i fratelli (B1a);
  - gli stati a novità zero (B11);
  - un controllo a campione delle trasposizioni (B3).

## Fase B: correggere
Ogni correzione è generale: niente nomi di carte. Segue il protocollo del motore (`AGENTS.md`): banco, `combo_hunt --seed 7`, `search_regression`, più il test appaiato e, quando c'è, l'oracolo A1. Le idee e le fonti di ogni punto sono in `ricerca-letteratura.md`.

### Velocità della ricerca (corsia 2, `search.rs` e `ygo-engine`)
- **B0. Il replay.** Studio delle strade in `velocita-ricerca.md`, B0.
  - **Prima un prototipo di misura dell'arena di memoria (B13).** È l'approccio di ygo-combo-solver: le allocazioni del core e di Lua vanno in un'arena a indirizzo fisso, e un checkpoint copia le pagine scritte. Salva e ripristina il duello a metà turno, coroutine Lua comprese. Gli agganci ci sono già (`l_alloc` in `build.rs`, `shim/new.cpp`), e non serve toccare `vendor/ygopro-core`.
  - **Se ripristinare costa molto meno del replay medio (5,6 ms)**, si fa l'arena. Altrimenti la strada 4: una copia del duello dopo le carte aggiunte, sopra il riuso dei duelli dello shim.
  - **Controllo:** uscite identiche sul banco e controlli di `DUELFORGE_CHECK_REUSE` puliti. Nodi al secondo sul banco ×1,15 con la strada 4, ×1,5 con l'arena.
- **B1a. Ordine dei fratelli.** A parità di priorità la coda prende oggi la risposta più nuova, e i fratelli entrano nell'ordine di `dive_order`: escono per prime le alternative meno preferite (Passa, No, Annulla, Set).
  - Da verificare con il contatore A2, poi la correzione è piccola.
  - **Controllo:** nessuna inversione nel contatore; `best` del banco non scende.
- **B2. Ordini scambiabili.** Riduzione per ordine parziale, solo nella forma prudente (sleep set): DPOR presuppone una ricerca in profondità.
  - **Controllo:** i nodi doppi «altro ordine» scendono dal 9,3% a meno del 3%, con le stesse board migliori.
- **B2b. Selezioni a più carte come un solo passo**, e l'Evocazione con i suoi materiali.
  - **Controllo:** nodi del banco −15% senza perdere stati distinti (confronto delle chiavi).
- **B3. Chiave dei nodi con le scelte che contano** (bersagli e nomi dichiarati, §30.8). In più, in debug, un controllo a campione che due cammini verso lo stesso nodo offrano le stesse azioni.
- **B1b e B1c. Ordinamento imparato durante la ricerca.**
  - B1b, history relativa e di continuazione per offerta (tipo di azione, codice, luogo);
  - B1c, penalità di deviazione che cresce con il rango in `dive_order`, al posto dello 0,6 uguale per tutte.
  - **Controllo:** tempo per arrivare alla board migliore sul banco −20%, e `best` non scende più del 2%.
- **B10, B11, B16.** Tutti con controllo: tempo alla board migliore −10%, `best` non più basso.
  - B10, priorità ricalcolate dopo l'apprendimento («epoca dei valori»);
  - B11, più code alternate (forza, distanza dall'obiettivo, azioni utili, per tipo) e novità come chiave secondaria;
  - B16, tempo gestito dalla stabilità del migliore.
- **B6. Più thread che rendano.** Si rivede dopo B0.
- **B7. Partire da linee già trovate.** Prefissi promettenti dalla mappa dei collegamenti e dalle ricerche precedenti, una tabella con la linea più economica per ogni mostro dell'Extra Deck Evocato, e macro di 2-4 passi.

### Ricerca per obiettivo
- **B4. Ricerca per obiettivo guidata dalla mappa dei collegamenti.** È pianificazione su un grafo implicito: la «ricerca da labirinto» del proprietario.
  - Le tecniche:
    - una distanza rilassata (h_add) calcolata sulla mappa, in un modulo nuovo `relax.rs`;
    - le «azioni utili» del piano rilassato, provate per prime;
    - un credito all'obiettivo imparato durante la ricerca (AMAF);
    - dopo la prima linea che raggiunge l'obiettivo, si continua a migliorarla attorno.
  - La distanza serve solo a ordinare, mai a tagliare, così il verdetto «impossibile» resta corretto.
  - Tra le linee che raggiungono l'obiettivo vince quella con la board più forte.
  - **Controllo:** nodi alla prima soluzione −30% su un insieme fisso di obiettivi raggiungibili; i verdetti «impossibile» restano uguali.
- **B8. Obiettivi più ricchi.**
  - Ogni carta dell'obiettivo dice il posto: Mostro Principale o Extra, Magia/Trappola coperta o scoperta, Terreno, Cimitero, bandita, mano.
  - Ci sono anche atomi sull'avversario, come le carte tolte dal suo Extra Deck, e il segno duro o morbido.
  - Gli atomi fuori dal campo si controllano sulla board esatta, dopo la Fase Finale.
  - Il contratto `lib.rs` ↔ `api.ts` cambia in una PR di Claude; poi Gemini fa l'interfaccia. Gli obiettivi salvati oggi restano validi.
  - In un secondo passo, vincoli sui materiali (ad esempio Aeon con 3 o più materiali).
  - **Controllo:** un obiettivo con una Trappola coperta e una carta nel Cimitero si raggiunge su Labrynth e Branded, e un obiettivo impossibile riceve il verdetto giusto.

### La misura (coordinatore, `probe.rs`, `value.rs`, `lib.rs`)
- **B5a. Attrito avversario** (dopo D17).
  - Le parti:
    - un Extra Deck realistico per l'avversario, circa 15 mostri generici;
    - la metrica «risorse tolte all'avversario» nel tuo turno;
    - lo stesso termine nella stima della ricerca;
    - l'«aspetto» della board che distingue le carte tolte;
    - le scelte nell'Extra Deck avversario rigiocate per identità;
    - il sì opzionale degli script mai rifiutato in automatico;
    - le correzioni dell'Observer per controllo e rimozione.
  - **Controllo:** su Invoked la board con Aeon a 3 materiali sta sopra quella a 2, nella misura e nella stima.
  - Cambia tutti i punteggi: il banco oggettivo si rifà come nuova base, perché i numeri vecchi non sono più confrontabili.
- **B5b. Risposte ai board breaker diverse dalla negazione:** floater, Evocazioni rapide, protezioni sostitutive (30 e 28 deck su 37). Una run «react» accanto a quella in cui si passa.
  - **Controllo:** la resilienza dei deck con floater cambia; la correlazione con il livello 2 non scende; `combo_hunt` senza problemi nuovi.
- **B5c.** Protezione dei pezzi nel punteggio, turno 3 dal campo, Cimitero e bandite nella stima, danno e Life Point, tassa pagabile.
- **B5d.** Valori delle carte per «firma di costruzione»: materiali, Attributi, contatori, tipo di Evocazione.
- **B5e e B5f.** Kit più ampio ed esposizione letta dal motore: più avanti, con il valutatore imparato.
- **B9. Stima con l'avversario, tarata sui dati** (Texel, perdita a coppie), dopo B5a e B1.
  - **Controllo:** inversioni tra stima e misura nella top 8 (C5.7) da 174 a 60 o meno su 1110.
- **B14. Quali board misurare: dimezzamenti successivi.** Oggi l'app misura a piena fedeltà 30 board; il turno di prova è il 58% del tempo di `combo_hunt`. I gradini:
  1. stima rapida su 40-60 board;
  2. misura senza breaker su tutte, si tiene la metà;
  3. misura completa su 10-15;
  4. livello 2 su 3-4.
  - **Controllo:** tempo della valutazione nell'app −50%, con il #1 identico e almeno 2 board su 3 uguali nella top 3.

### Più avanti
- **B12.** Una politica imparata dall'oracolo (DAgger, perdita a coppie), mai addestrata sulle linee della ricerca stessa.
- **B15.** Taratura delle costanti della ricerca con SPSA o CEM, a budget di nodi; corse lunghe lanciate dal proprietario.
- **B17.** Probabilità di riuscita delle linee su più ordini del Deck (8-16 semi).

## Obiettivi misurabili
- **Oracolo A1:** la ricerca normale arriva al 95% del valore dell'oracolo entro 30 secondi in 9 mani su 10, e la board dell'oracolo è nella top 3 in 8 su 10.
- **Tempo** per arrivare alla board migliore sulle mani del banco: mediana dimezzata.
- **Forza:**
  - la verità del banco oggettivo delle board migliori non scende;
  - `best` del banco a nodi fissi non scende più del 2%.
- **Invoked:** l'obiettivo Oceanus nella Zona Mostri Extra + S:P si raggiunge con Aeon a 3 materiali, e la board viene classificata sopra quella con Aeon a 2, già nella stima della ricerca.
- **Stima contro misura:** 60 inversioni su 1110 o meno (C5.7).
- **App:** tempo della valutazione dimezzato, con la stessa top 3.
- **Obiettivi ricchi:** una Trappola coperta e una carta nel Cimitero si chiedono e si raggiungono.

## Ruoli (correzione piccola, a parte)
File: `crates/ygo-solver/src/roles.rs` e i suoi test.
- **Board breaker dal testo.** Oggi `breakers_by_text` cerca frasi come «destroy all» o «your opponent controls». Va riconosciuto anche chi usa le carte dell'avversario come materiale o come Tributo: «from either field», «monster(s) your opponent controls» insieme a «material» o «Tribute». Così Super Polymerization diventa board breaker, e con lei ogni carta con lo stesso schema, senza nomi.
- **Starter ed estensore insieme.** Oggi uno starter non si prova mai come estensore (`else if`). Uno starter va provato anche come estensore sulla linea di un altro starter: per lo starter migliore si usa la linea del secondo.
- **Estensore e board breaker insieme** è già possibile. Va capito perché Invocation «Grail» non risulta estensore. Le cause possibili:
  - la soglia `EXTENDER_GAIN`;
  - un materiale preso all'avversario, che giocando per primi non c'è;
  - il budget di 400 nodi.

  La correzione deve valere per ogni deck.
- **Controllo:** `duelforge-cli roles` prima e dopo su almeno 10 deck di Master Duel Meta, con l'elenco delle carte che cambiano ruolo, ognuna verificata sul testo.

## Chi fa cosa
I file caldi (`search.rs`, `probe.rs`, `value.rs`, `lib.rs`, `api.ts`) li tocca un agente alla volta. I moduli nuovi (`relax.rs`, la history, l'arena in `ygo-engine`) si scrivono in parallelo; si serializza solo l'aggancio.
- **Corsia 2 (velocità):** in quest'ordine, una correzione per PR.
  1. B0, con il prototipo dell'arena;
  2. B1a;
  3. B2 e B2b;
  4. B3;
  5. B1b e B1c;
  6. B8, motore e contratto;
  7. B10, B11 e B16;
  8. B4, dopo B8 e B9.
- **Corsia 6:** i ruoli, poi l'oracolo A1 con lo script del test appaiato.
- **Coordinatore:**
  1. D17, i pesi;
  2. B5a;
  3. B14 (`lib.rs`);
  4. B5b, poi B5c e B5d;
  5. B9, insieme alla corsia 2, dopo B5a e B1.
- **Gemini:** l'interfaccia di B8, dopo il contratto.
- **Sessione sorella di ricerca (proposta, da decidere con il proprietario).** Fa esperimenti offline sui dati del banco:
  - fit Texel sui JSON di `combo_hunt`;
  - correlazione tra i gradini di misura di B14;
  - il controllo con ygo-combo-solver sul caso Invoked.

  Non tocca né i file caldi né il motore.
