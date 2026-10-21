# Classificazione delle board: progetto di una misura più profonda e validata

> **Decisione del proprietario (20/10/2026):** niente ordinamenti fatti a mano. Dove il progetto dice "il proprietario ordina" (C3, C5.4, C5.5, D17), il giudizio lo dà un **pannello di agenti-giudici**: tre agenti indipendenti ordinano alla cieca le prime board distinte di ogni deck, con le carte e le linee ma senza punteggi. Una coppia diventa "sicura" quando tutti e tre sono d'accordo.

*Combo Lab, turno di prova. Documento per il proprietario e lo sviluppatore. Si basa su tre studi (audit del codice, criteri competitivi, evidenze su 45 deck) e sul codice attuale.*

---

## 0. In breve

Oggi il punteggio di una board somma otto voci: `interruptions`, `handTraps`, `floodgates`, `resilience`, `followup`, `resources`, `cost`, `exposure`. Ci sono tre tipi di problema.

1. **Errori di misura che spostano le classifiche.**
   - I responder con bersaglio colpiscono la prima carta della lista, non quella in catena. Per questo Impermanence, Veiler, Droplet e Fog Blade valgono circa 0,15 in tutte le 60 linee misurate.
   - Le fermate a cascata contano due o tre volte.
   - La copertura mette insieme unità diverse: le Evocazioni stanno nel numeratore ma non nel denominatore.
   - L'etichetta dell'effetto viene presa da trigger di altre carte.
   - Le copie di una carta si fondono in un solo responder.
   - La run congiunta azzera responder che semplicemente non servivano.
   - Se l'unica negazione ferma un board breaker, il risultato vale quanto la board distrutta.
2. **Voci che contano due volte la stessa cosa.**
   - La resilienza riconta le interruzioni del campo.
   - `cost` e `resources` misurano tutte e due le carte rimaste in mano.
   - I floodgate sono su una scala grezza, da 0 a circa 4, e nessun breaker li tocca.
3. **Cose che il punteggio non vede.**
   - Esca e ordine delle giocate.
   - Breaker con bersaglio, che non si possono concatenare o che bandiscono.
   - Floater, ricorsione dal Cimitero e responder che nascono nel turno avversario.
   - La finestra di Standby e la End Phase.
   - Quanto costa davvero ogni punto in cui una handtrap ferma la linea.
   - Il follow-up reale, cioè cosa si può fare al turno dopo.

Il progetto ha quattro parti:
- **A.** Correzioni alle misure attuali.
- **B.** Metriche nuove o ridefinite, ognuna con dove si misura e come entra nel punteggio.
- **C.** Protocollo di validazione sui deck di Master Duel Meta.
- **D.** Ordine di implementazione in passi piccoli e testabili.

Regola trasversale: **nessun caso speciale per nome di carta** nel motore, nel prober o nella ricerca. Le carte del kit sono l'avversario e hanno un nome per forza. Nei test si possono citare carte vere come casi di regressione.

---

## 1. Principi del nuovo punteggio

1. **Un'unità sola.** Tutto ciò che ferma l'avversario si misura in *interruzioni*: 1,0 vale una negazione senza bersaglio di qualsiasi attivazione, usata una volta. Vale anche per i floodgate e per la perdita ai breaker. Le carte in mano e le risorse restano in carte, con pesi bassi.
2. **Ogni fatto misurato entra in una voce sola.** Una carta in mano conta in `handTraps`, `followup` oppure `resources`, mai in due. La resilienza diventa una *perdita* (si sottrae) e non una seconda somma delle interruzioni.
3. **Si misura giocando.** Ogni metrica viene da una run del motore (turno di prova, replay della linea, stato), non da una lettura del testo delle carte.
4. **Invarianza.** Due linee che arrivano alla stessa board, salvo le zone principali, devono misurare uguale entro ε = 0,05.
5. **Costo controllato.** Obiettivo: mediana ≤ 1,2 s per board su un thread (oggi 0,1–0,4 s) e p95 ≤ 4 s. Il lavoro costoso (esposizione per punto critico, coppie di breaker) si fa solo sulle prime 8 board e si mette in cache per linea.

### Formula proposta (profilo Bilanciato, da calibrare in C)

```
score = 1.0·F + 0.9·H + 0.8·G − 0.5·L + 0.4·U + 0.1·R − 0.3·C − 0.5·E
```

| Simbolo | Chiave `ScoreView` | Significato | Unità |
|---|---|---|---|
| F | `interruptions` | interruzioni del campo, del Cimitero e bandite, nella run congiunta corretta (§B1) | interruzioni |
| H | `handTraps` | interruzioni dalla mano, nella stessa run congiunta | interruzioni |
| G | `floodgates` | floodgate normalizzati (§B4) | interruzioni |
| L | `breakerLoss` (nuova) | valore atteso perso ai board breaker (§B3) | interruzioni |
| — | `resilience` | rapporto di sopravvivenza S ∈ [0,1]: si mostra, non si somma | rapporto |
| U | `followup` | follow-up misurato (§B7) | carte equivalenti |
| R | `resources` | altre carte in mano | carte |
| C | `cost` | risorse spese fuori dalla mano (§B8) | 0–1 circa |
| E | `exposure` | valore atteso perso alle handtrap durante la linea (§B6) | interruzioni |

Gli altri profili cambiano solo i pesi. Per esempio "A prova di board breaker" usa L = −1,0 e "Linee sicure" usa E = −1,0. Sono pesi di partenza: si fissano con il protocollo in C, usando l'accuratezza sulle coppie di board.

**Coordinamento.** `score_view` e i tipi (`apps/desktop/src-tauri/src/lib.rs`) sono di Claude. `api.ts` va aggiornato nella stessa PR perché è un file caldo, quindi va preso il lock. `scoring.ts` (profili, testi "i") è di Gemini: la richiesta va in `docs/TASKS.md`. Nella transizione i campi nuovi si aggiungono come **opzionali** e i vecchi mantengono il loro significato finché Gemini non cambia `scoring.ts`.

---

## 2. Parte A: correzioni alle misure attuali

### A1. Bersagli e costi scelti con criterio (impatto alto)

**Problema.** `your_answer` (probe.rs, verso la fine) sceglie la prima azione che riguarda una carta avversaria, oppure la prima opzione non "annulla". Le conseguenze:
- Impermanence concatenata all'effetto di Link Spider colpisce Terrortop (dbg2.txt, imp.err).
- Spright Red tributa Carrot invece di Gigantic: x_spright #1 e #2 hanno le stesse carte e misurano F 0,44 contro 1,22.
- Rhapsodia paga il costo con Ash in mano (elf.out).

**Correzione** in `probe.rs`:
- Si aggiunge a `run()` un contesto di risposta `AnswerContext`:
  - il codice dell'anello avversario sotto il tuo, preso da `Observer.links` (controller 1, l'ultimo prima del tuo `Chaining`);
  - `Observer.summoning`;
  - la carta dell'`Act` del passo in corso;
  - l'insieme `protect: HashSet<u32>` delle carte che non vanno pagate come costo.
- **Bersagli.** Quando la decisione chiede carte avversarie mentre si attiva o risolve un tuo anello, l'ordine di preferenza è:
  1. la carta dell'anello avversario in catena;
  2. il mostro che si sta Evocando;
  3. la carta del passo;
  4. le altre.
- **Ripiego.** Se `UseAt` con il bersaglio preferito non ferma nulla, si riprova con al massimo altre 3 opzioni, con una nuova `Policy::UseAtTarget(id, window, k)` dove `k` è l'indice dell'opzione. Si tiene la run migliore.
- **Costi.** Quando la decisione chiede carte tue durante la tua attivazione, l'ordine di preferenza è:
  1. carte non responder: né in `hand_traps` né in `offered` del Pass;
  2. responder del campo già usati in questa run;
  3. la carta con il valore più basso in `CardValues`.
- **Mai** una handtrap misurata, se c'è un'alternativa.
- Lo stesso ordine vale per le scelte di tributo o materiale di un tuo effetto rapido: si provano al massimo 3 opzioni e si tiene la migliore.

**Test** (nuovo `crates/ygo-solver/tests/probe.rs`, saltato senza `data/` come `invoked.rs`):
- Impermanence piazzata da sola: `covers` contiene `FieldIgnition` e `DeckSummon`.
- Le due board di x_spright (stesse carte) misurano uguale entro 0,05.
- Rhapsodia più Ash in mano: Ash non scende a 0 se c'è un'altra carta da pagare.

**Rischio.** Più run per i responder con bersaglio (fino a 4× sulle finestre in cui si sceglie). Si limita il ripiego alle finestre dove il primo tentativo è fallito.

### A2. Una fermata conta solo per il passo che si è giocato (impatto alto)

**Problema.** `stopped_by` conta ogni probe che funzionava nella Pass e ora no, compresi i passi diventati indisponibili perché è fallito il loro `needs`:
- Impermanence su Terrortop risulta `[DeckSummon, FieldEffect]`;
- Foolish più effetto dal Cimitero vale 0,60;
- i floodgate fanno lo stesso con `floodgates.contains(&p)` nella condizione `prerequisite`.

**Regola nuova.** Il probe `p` del passo `s` è fermato dal responder se valgono tutte e tre le condizioni:
- `p` funzionava nella Pass;
- `p` non funziona ora;
- il `needs` di `s`, se c'è, **ha funzionato in questa run**.

Così:
- negare la ricerca di Terrortop conta `FieldEffect` e basta;
- distruggere Celtic dopo l'Evocazione Normale conta `ExtraSummon`, perché il suo `needs` (NormalSummon) ha funzionato e il passo è fallito per colpa tua: questo è giusto;
- per i floodgate si toglie `|| floodgates.contains(&p)`.

**Opzionale.** Si aggiunge `Interruption.also_prevents: Vec<Probe>`, con i passi persi a cascata. Si mostra nella scheda ("impedisce anche...") e **non** entra nel valore. Entrerà eventualmente con un peso esplicito dopo la validazione.

**File:** `probe.rs`, chiusura `stopped_by` e ciclo dei floodgate in `measure_from`.

**Test:**
- Impermanence piazzata: con il solo bersaglio Terrortop la copertura non include `DeckSummon`.
- Un floodgate sul Deck al Cimitero non conta anche `GraveEffect`.

### A3. Copertura in un'unità sola, Evocazione negata o mostro rimosso (impatto alto)

**Problema.**
- `coverage` somma tutti i pesi coperti, Evocazioni comprese, ma divide per i soli pesi delle attivazioni (`reference`). Torrential arriva a 2,0/2,7 = 74%.
- I goal si leggono a fine passo, quindi "rimosso dopo essere atterrato" vale come "Evocazione fermata".
- `Observer` etichetta `NegateSummon` ogni spostamento dalla MZONE del mostro appena Evocato.

**Correzione:**
- **Motore** (`ygo-engine/src/message.rs`): si aggiunge `Event::Summoned`, il messaggio di fine Evocazione Normale o Flip, che oggi arriva come `Other`. Si affianca a `SpecialSummoned`.
- **Observer:**
  - si aggiunge `landed: HashSet<u32>`: un mostro avversario entra in `landed` all'evento `Summoned` o `SpecialSummoned` successivo al suo `*Summoning`;
  - `NegateSummon` si assegna solo se la carta lascia la MZONE **prima** di essere in `landed`;
  - dopo, lo spostamento si classifica come rimozione: `Destroy`, `Banish`, ecc.
- **Goal di Evocazione** (`SpecialFromHand`, `NormalSummon`, `ExtraSummon`): valgono "atterrato" (`landed`), non "c'è a fine passo".
- **Denominatore:**
  - `coverage = Σ peso(probe di attivazione fermati) / reference`;
  - più `Σ peso(probe di Evocazione fermati) / reference` **solo** se l'effetto è `NegateSummon`;
  - limite 1,0, come oggi.
  - Così una negazione di Evocazione ha il suo valore, e una rimozione dopo l'atterraggio conta solo i passi successivi falliti (regola A2).

**Test:**
- Torrential piazzato: effetto `Destroy`, nessuna Evocazione in `covers`, copertura < 0,5.
- Una carta che nega davvero un'Evocazione (qualsiasi del deck di prova) resta `NegateSummon`.

### A4. Effetto e copertura presi dal proprio anello (impatto medio)

**Problema.**
- In `measure_from`, `effect` è il minimo su tutti gli `u in run.uses`.
- Il follow-up (`last_use` più `follow_up`) attribuisce alla risposta anche i trigger di altre tue carte. Per esempio Ash su x_kashtira risulta "bandisce", per via del floodgate di Kashtira.
- La stessa carta riceve etichette diverse da una board all'altra (Daruma: "posizione" su una board, "nega l'Evocazione" su un'altra).

**Correzione:**
1. Nel ciclo dei responder si filtra `run.uses` con `u.responder == *id`.
2. Il ramo `follow_up` di `run()` attribuisce un trigger alla tua ultima risposta solo se il trigger è della **stessa carta**, cioè con lo stesso `code` canonico. I trigger di altre carte vengono attivati lo stesso, ma senza `observer.waiting`.
3. L'etichetta deve venire dall'anello avversario o dalla carta del passo: `ChainNegated` o `ChainDisabled` dell'anello avversario, poi il movimento della carta del passo. Materiale Xyz o bandita coperta valgono `Banish`. Un mostro girato coperto vale `NegateEffect` e non `Position`. I movimenti di carte avversarie che non sono del passo non contano.
4. Se lo stesso responder mostra effetti diversi in finestre diverse, si tiene il più forte **visto in almeno 2 finestre**. Con una sola finestra si tiene quello.

**Test:**
- Ash in mano con un floodgate di banish sul campo: etichetta `NegateEffect` o `NegateActivation`, mai `Banish`.
- La stessa carta Trappola piazzata in due board diverse ha la stessa etichetta.

### A5. Copie e istanze (impatto medio)

**Problema.** `ResponderId` è `(code, description)`:
- due Fog Blade scoperte non si misurano;
- due Impermanence contano ×1.

**Correzione.** Si aggiunge l'istanza **solo per carte sul campo** (`Window.options` porta già `card.location`, si aggiunge `sequence`). `ResponderId` resta la chiave di visualizzazione e si introduce `ResponderSlot { id, location, sequence }` per le policy. `Policy::Use` usa qualunque slot con lo stesso `id`; il conteggio `uses` somma tra gli slot. Il limite "1 per turno" lo decide il motore: la seconda copia non viene offerta.

**Test:**
- Due copie scoperte di una Trappola senza limite per turno: `uses == 2`.
- Due copie di una carta con limite: `uses == 1`.

### A6. Finestre campionate per passo (impatto basso)

`windows.iter().take(12)` diventa:
- prima la prima finestra di ogni passo;
- poi le altre in ordine, fino a un budget di 16.

Ogni passo ha almeno un tentativo.

**Test:** un responder offerto a ogni finestra viene provato anche al passo 10 (Taketomborg).

### A7. Run congiunta: si conta ciò che si è fermato, non ciò che non serviva (impatto medio)

**Problema.** `UseAll` taglia ogni responder usato meno volte di `uses`, anche quando non è servito perché il kit non aveva più nulla da fare. Ci sono 35 casi "0.00 (alone X)" in 12 deck, e l'ordine greedy rende il risultato dipendente dalle zone (goblin_biker 0,52 contro 0,13).

**Correzione:**
1. Si aggiunge `Policy::UseUseful(&[(ResponderId, HashSet<WindowAt>)])`: ogni responder si usa **solo** nelle finestre in cui la sua run `UseAt` ha fermato qualcosa. Il dato si raccoglie già nel ciclo dei responder.
2. Un responder si riduce solo per **conflitto di risorse**: nella Pass era offerto in un passo `s`; nella congiunta il passo `s` è stato giocato e lui non è mai stato offerto in quel passo; non ha esaurito i suoi usi. Se invece era offerto ma il passo era già fermato o indisponibile, non si riduce.
3. **Assegnazione.** Se i conflitti riguardano 2 o 3 responder, si provano le permutazioni dell'ordine (al massimo 6 run) e si tiene quella con la somma più alta. Oltre 3, si usa l'ordine greedy di oggi.
4. **Ordine canonico.** Le opzioni e i responder si ordinano per `(code, description, location, sequence)` normalizzato, non per posizione nella lista del motore.
5. **Riprova del kit** (solo nelle run congiunte): un passo fermato si rigioca una volta con una seconda copia nel kit (`KitStep.retry: bool`, copie aggiunte in `Kit::standard`). Così le interruzioni ridondanti trovano un bersaglio, come contro un avversario vero con più starter.

**Test:**
- Board elfnote: Ash non è 0 se il campo non ne usa il costo.
- Board con un effetto che manda una carta dalla mano come costo (il caso di §26.2) **e** una handtrap come unica carta in mano: la riduzione resta.
- goblin_biker #1 e #2 (Caesar in zona principale o Extra) misurano uguale entro 0,05.

### A8. Board breaker negato con l'unica negazione (impatto alto)

**Problema.** `remaining = left − spent`. Con una sola negazione il risultato è 0, come per la board distrutta. Succede in 13 famiglie di board (Varudras, Regulus, Baronne da sola, Herald, Allied Code Talker...).

**Correzione.** Si integra nella resilienza nuova (§B3): un breaker negato vale come una **fermata** del turno avversario. Il valore della negazione spesa resta dentro F.

### A9. La condizione di Evocazione del kit scambiata per floodgate (impatto medio)

**Problema.** Le board che danno un mostro all'avversario (un Token) rendono indisponibile Terrortop ("se non controlli mostri"). Ne escono 3 floodgate finti, 0,9 in volcanic.

**Correzione.** Un passo **non disponibile** conta come floodgate solo se è disponibile in una run di controllo, `Prelude::Line` senza mostri avversari. È costosa, quindi è meglio una via più semplice e generale: per `SpecialFromHand` si usa nel kit una carta **senza condizioni sul campo** (da scegliere nei dati: un mostro che si Evoca Specialmente dalla mano senza condizioni sul campo). Terrortop resta come carta del passo `FieldEffect`, evocata dopo la Normale. In ogni caso, con A2 i passi dipendenti non contano più due volte.

**Test:** board volcanic con il Token: `floodgates` vuoto.

---

## 3. Parte B: metriche nuove o ridefinite

### B1. Interruzioni F e H: valore congiunto, fermate, esca

**Dove:** `probe.rs`, `measure_from`, dopo i responder.

**Come si misura:**
- `alone(r)` resta il valore singolo di oggi (effetto × 0,85 se ha bersaglio × copertura × usi), corretto con A1–A6. Si usa per le schede e per `value.rs`.
- **F e H** sono la somma dei valori dopo la run congiunta corretta (A7), divisa per posizione: `location != HAND` va in F, la mano in H.
- **`stops`** (nuovo): quanti passi distinti del kit la run congiunta ha fermato, contati con la regola A2. Più la riprova (A7.5), quindi un passo fermato due volte conta 2.
- **`first_stop`** (nuovo): l'indice del primo passo fermato. Fermare il primo starter vale di più che fermare il nono passo.
- **`bait_value`** (nuovo): F più H in una seconda run congiunta in cui:
  1. il kit apre con i passi meno pesanti (Evocazione Normale, Magia), in ordine di peso crescente;
  2. ogni responder si usa **alla prima finestra in cui è offerto**, non alla sua finestra migliore.

  Misura quanto vale la board contro chi la gioca attorno: la negazione con bersaglio, una per turno, finisce sull'esca.

**Come entra nel punteggio:**
- all'inizio solo F e H. `stops`, `first_stop` e `bait_value` si mostrano e basta;
- dopo la validazione (passo D11), se migliora l'accuratezza sulle coppie: `F_eff = 0,5·F + 0,5·F_bait`, e lo stesso per H.

**Costo:** 1 o 2 run in più, più al massimo 6 per le permutazioni.

### B2. Il kit: azioni più rappresentative e pesi calibrati

**Dove:** `probe.rs`, `Kit::standard` e `Probe::weight`.

1. **Meno azioni che toccano il Deck.** Oggi sono 7 su 8: Ash copre l'89% e vale 0,71 in più di 77 board. Si aggiungono:
   - un effetto di mostro dalla mano che **non** tocca il Deck, per esempio un mostro che si Evoca Specialmente scartando una carta;
   - un effetto dal Cimitero che richiede che la carta resti nel Cimitero, che si possa fermare bandendola;
   - una seconda Evocazione dall'Extra Deck di tipo diverso dal Link: Xyz o Synchro con due mostri semplici del kit;
   - una Magia Rapida attivata dalla mano.

   Ogni carta del kit si sceglie nei dati come "carta semplice che fa una cosa sola", nello stesso modo delle attuali.
2. **Pesi calibrati sui dati.** Un nuovo comando CLI, `duelforge-cli kit-mix --decks <cartella> --nodes 3000`:
   - per ogni deck cerca la mano fissa (stessa regola di §C1) e prende le linee delle prime 5 board;
   - con `explain`, conta i tipi delle prime giocate di ogni linea: Magia dalla mano, ricerca, effetto dalla mano, Deck al Cimitero, Evocazione Speciale dalla mano, trigger d'Evocazione, effetto dal Cimitero, Evocazione Normale, Evocazione dall'Extra, effetto di mostro sul campo, Evocazione dal Deck;
   - stampa la frequenza normalizzata.

   I pesi nuovi sono `0,5·peso attuale + 0,5·frequenza·Σpesi`. Si scrivono nel codice con un commento che cita la data e il numero di deck.
3. **Tetto per un responder solo** (salvaguardia): nessuna copertura singola supera il 60% senza un effetto "nega qualsiasi". Lo si **verifica** nella validazione (§C4), non lo si impone nel codice: se il kit è bilanciato non serve.

**Test.** Su board piazzate (con `card-values`) si guarda una tabella delle handtrap:
- Ash ≤ 0,60;
- Impermanence e Veiler ≥ 0,30;
- D.D. Crow > 0.

### B3. Resilienza come sopravvivenza; perdita ai breaker L

**Dove:** `probe.rs`, ciclo dei breaker, più nuovi campi in `BreakerResult` e `Measure`.

**Come si misura**, per ogni breaker `b`:
1. **Run "breaker poi kit"**: `steps = b.steps ++ kit.steps`. `run()` accetta già una lista di `&KitStep`; i goal del kit restano agli indici spostati. Si gioca con la policy congiunta corretta (A7) **più** l'uso libero nelle finestre del breaker:
   - variante (a): nella finestra del breaker si usa tutto ciò che è offerto e ha un effetto che nega;
   - variante (b): nella finestra del breaker si usano anche gli altri responder offerti, per esempio una Trappola attivata in risposta a Harpie's prima di essere distrutta;
   - si tiene la variante migliore.
2. **Valore dopo il breaker**, `after_b`: F, H e G misurati **in quella run**, cioè quanto ha fermato del kit dopo il breaker. Comprende automaticamente:
   - floater e ricorsione;
   - Set della End Phase;
   - floodgate sopravvissuti, con G ricalcolato come in §B4 confrontando i passi del kit con il vuoto.
3. **Breaker fermato.** Se una tua risposta ferma il breaker (nuovo goal "breaker fermato": il passo del breaker ha `negated == true` o il suo effetto non ha toccato le tue carte), la fermata vale come le altre: `credit_b = effect.value × fattore bersaglio`.
4. **Sopravvivenza:**
   ```
   kept_b = min(F + G, credit_b + after_b(F) + after_b(G))
   S_b    = kept_b / (F + G)          (1 se F + G = 0)
   ```
   Le handtrap (H) non entrano in S: nessun breaker le tocca, e ricontarle sarebbe un doppio conteggio.
5. **Media.** `S` è la media di `S_b` pesata con `b.weight`, **sui soli breaker applicabili**. Un breaker non applicabile esce dalla media e non vale più `total`.
6. **Coppia peggiore** (solo report all'inizio): per le coppie fra i 3 breaker più pesanti, si gioca `b1 ++ b2 ++ kit` e si tiene il minimo `S_pair`.

**Come entra nel punteggio:**
- `resilience = S`, da 0 a 1, solo da mostrare ("tiene il 72% dopo un breaker medio");
- `breakerLoss = L = (F + G)·(1 − S)`, con peso negativo (Bilanciato −0,5, "A prova di breaker" −1,0);
- quando la coppia peggiore è validata: `L = (F + G)·(1 − (0,7·S + 0,3·S_pair))`.

**Conseguenze attese:**
- Varudras che nega Dark Hole ottiene `S_b ≈ 1`, non 0;
- un floodgate che sopravvive a Dark Hole conta;
- un floodgate distrutto non conta più "resiliente al 100%";
- la resilienza non raddoppia più F (synchrons: 2,72 + 0,6×1,89 diventa 2,72 − 0,5·L).

**Costo:** 10 breaker × 2 varianti, più 3 coppie: circa 23 run. È la parte più cara, ma resta dentro il budget.

### B4. Floodgate in interruzioni

**Dove:** `probe.rs`, dove si calcola `floodgate_value`, e `value.rs` (`FLOODGATE_WEIGHT`).

**Come si misura:**
```
G = κ · Σ peso(probe bloccati, regola A2) / Σ peso(tutti i probe del kit)
```
con κ = 1,5 come valore di partenza: un blocco totale vale 1,5 negazioni. κ si calibra in C.

Si tolgono i doppi conteggi a cascata (A2) e i falsi floodgate (A9). Si aggiungono anche i **breaker bloccati**: un breaker che funziona sul campo vuoto e qui fallisce senza nessuna risposta, perché un floodgate impedisce Magie o Evocazioni. Vale `κ·b.weight/Σ b.weight` in G.

**Nel punteggio:** come oggi, con peso 0,8, ma alla scala nuova. In `value.rs` resta `FLOODGATE_WEIGHT`, applicato alla scala nuova; il valore di partenza va ricalibrato con la ricerca (bench).

### B5. Breaker nuovi, per assi

**Dove:** `Kit::standard`, più una struttura `BreakerAxes { targeted, chainable, kind, timing }` per descrivere ogni breaker.

I breaker si descrivono per assi generici. Se ne aggiunge uno per ogni asse oggi scoperto:

| Asse | Carta del kit (da verificare nei dati) | Mira a |
|---|---|---|
| bersaglio, rapido, nega o gira coperto | una Magia Rapida che gira coperto un mostro | il responder sul campo con il valore più alto (`picks = targets` come oggi) |
| bersaglio, bandisce | una carta che bandisce un mostro o una Magia/Trappola con bersaglio | idem |
| non si può concatenare, prende i materiali | una Fusione che non si può concatenare | i mostri tuoi |
| fine Battle Phase, bandisce coperte, ne lascia una | una Trappola o Magia che lascia una carta | tutto il campo |
| risposta alla tua negazione | una handtrap del kit usata sul tuo **primo** anello (in `opponent_answer`, `SelectChain` quando l'anello in cima è tuo) | il tuo primo responder |

Il peso di ogni breaker è la frequenza dell'asse nelle liste MDM: copie di carte di quell'asse nei 37 deck, normalizzate. La conta si fa con uno script su `mdm/*.ydk` e un **elenco di carte per asse tenuto nello script di calibrazione, non nel motore**. Per l'asse che non si può concatenare si salta il ciclo dei negatori.

**Nel punteggio:** entrano in S e L come gli altri breaker. La "risposta alla tua negazione" è un caso a parte:
- si misura `F_countered` = F nella run in cui l'avversario risponde alla tua prima interruzione;
- `B3.4` usa `min(kept_b, ...)`;
- all'inizio si **mostra** e basta ("con una negazione contro: 1,4 invece di 2,1").

**Protezione per pezzo** (solo report): per ogni interruzione sul campo, i breaker con bersaglio mirati a quella carta. Si registra quanta parte del suo valore resta. Si mostra nella scheda: "regge a: bandire / girare coperto / distruggere".

### B6. Esposizione come valore perso

**Dove:** `explain.rs` (`Exposure`, `exposure`) e `probe.rs` (nuovo `Prelude::Cut`).

**V1 (passo D8): linea tagliata.**
1. `exposure()` registra anche i **punti critici con la loro posizione**:
   ```
   pub struct ChokePoint { kind: ChokeKind, answer: usize }
   enum ChokeKind { DeckTouch, FieldEffect, FifthSummon, SecondSearch }
   ```
   `answer` è l'indice dell'ultima `choice_at` ≤ indice della risposta in cui avviene l'evento.
2. `Prelude::Cut(&Line, usize)`: in `run()` si rigiocano le risposte fino al taglio; alla prima `IdleCmd` del giocatore 0 si risponde con `end_turn`, poi `end_phase_answer`. È lo stesso schema del preludio `Line` provvisorio, con il passaggio esplicito alla End Phase.
3. `Prober::quick_value(prelude) -> f64` fa solo due run, Pass e congiunta (A7.1 con le finestre della Pass). Restituisce F + H + G.
4. Per ogni tipo di handtrap `h`:
   ```
   loss_h = V_full − min sui punti critici c di tipo h di quick_value(Cut(line, c))
   ```
   L'avversario sceglie il punto peggiore per te.
5. **Maxx "C" e Mulcharmy:** `loss_maxx = 0,1 × Evocazioni Speciali dopo la prima`. Sono carte date all'avversario, a 0,1 interruzioni per carta: un valore di partenza da calibrare.
6. **Probabilità:** `p_h` = probabilità che l'avversario, andando secondo, abbia almeno una copia in 6 carte. Le copie si contano con la media sulle liste MDM: con 3 copie è circa 0,39. Si calcola dalla cartella dei deck, non da un elenco nel motore.
7. `E = Σ_h p_h · loss_h`.

**V2 (passo D15): handtrap vera e ripresa.** L'avversario del kit ha la handtrap in mano e la usa al punto critico: `Prelude::LineWithInterrupt`, con l'avversario che risponde in `turn <= 1`. Poi una ricerca breve (300–500 nodi) dallo stato raggiunto. Serve un `SearchConfig.prefix` di risposte registrate (`RunTrace.answers`). `loss` = valore pieno meno la board migliore ritrovata.

**Nel punteggio:**
- `exposure = E` in interruzioni, peso −0,5 (Bilanciato) e −1,0 ("Linee sicure");
- il conteggio dei punti critici resta in `exposure_detail` per la scheda.

**Costo:** V1 fa 2 run per punto critico, di solito 3–16 punti. Si calcola solo per le prime 8 board e si mette in cache per `(board.key, kind)`.

### B7. Follow-up misurato

**Dove:** `probe.rs` (run del turno 3) e `lib.rs` (`score_view`).

**V1 (passo D12):** dalla run "breaker poi kit" del breaker che ha tolto di più (B3), si continua fino al **tuo turno 3**: l'avversario fa `EndTurn`, tu peschi. Alla prima `IdleCmd` del giocatore 0 si contano:
- `rebuild_effects`: le coppie distinte `(code, description)` attivabili da Cimitero, bandite, campo o zona Pendulum, più le Evocazioni offerte da carte non in mano;
- `ep_gain`: le carte aggiunte alla tua mano e i mostri Evocati durante la **End Phase avversaria** e la tua Standby. Servono le finestre di B9.

```
U = starters + 0,5·extenders                (mano, ruoli del Lab, come oggi)
  + 0,3·rebuild_effects + 0,5·ep_gain
```
Se i ruoli non ci sono, contano solo gli ultimi due termini.

**V2:** una ricerca di 300 nodi da quello stato. `U` diventa il `quick_score` della board migliore, normalizzato. Copre insieme Cimitero, bandite, motori sul campo e mano.

### B8. Costo e risorse senza doppio conteggio

**Dove:** `search.rs` (`cards_used` resta per l'ordine) e `lib.rs` (`score_view`).

- `cost` **non** è più `cards_used − 1`: le carte rimaste in mano sono già in H, U e R.
- Nuovo: `cost = 0,05·(carte bandite o mandate dal Deck come costo) + 0,1·(mostri dell'Extra Deck spesi e non sul campo) + LP pagati / 4000`. Si legge dagli eventi `Move` con `from.location == DECK | EXTRA` del replay di `exposure`, e da `PayLpCost`. Sono coefficienti di partenza.
- Opzionale (passo D13): si aggiunge il valore in mano delle carte spese (`CardValues::in_hand`). Una handtrap morta scartata costa circa 0.
- `resources`: il Cimitero non conta più 0,1 per carta. Lo sostituisce `rebuild_effects` (B7).

### B9. Finestre prima e dopo i passi del kit

**Dove:** `probe.rs`, `run()`.

- **Pre-passo.** Le finestre con `current == None` nel turno 2 (Draw e Standby avversarie) diventano il pseudo-passo `PRE`, alla posizione 0 nel vettore dei passi. Si registrano come `Window` e le policy le possono usare: in `UseAt` sono finestre come le altre. Servono per le Evocazioni rapide all'inizio del turno avversario e gli effetti in Standby.
- **Dopo l'ultimo passo:** invece di `complete = true; break`, l'avversario va in End Phase (`EndTurn`) e il loop continua fino al turno 3. Le tue finestre sono il pseudo-passo `END`. Si registra `ep_gain` (B7). I probe non cambiano: la End Phase non ha goal del kit.

**Rischio:** con le finestre nuove cambiano gli indici `WindowAt`. Va fatto in un commit a parte, con un test che le misure di oggi restino uguali per le board senza effetti in quelle finestre.

### B10. Responder che nascono nel turno avversario

**Dove:** `probe.rs`, ciclo dei responder.

Dopo ogni run `UseAt` o `Use`, si raccolgono i `ResponderId` presenti in `run.windows` ma **non** in `offered` della Pass: sono i nuovi responder, per esempio un mostro Fusione Evocato in modo rapido. Per ognuno, al massimo 4 per board e a profondità 1, si fa una run con `Policy::Chain([UseAt(r1, w1), UseAt(r_new, w)])` per ogni sua finestra (campionate come in A6). Si accredita al responder nuovo ciò che ferma in più. Nel punteggio entra in F come gli altri: la scheda mostra "nasce da *r1*".

### B11. Ordine della ricerca e stima rapida

**Dove:** `search.rs` (`quick_score`, `confirm_best`), `value.rs`, CLI.

1. **Riordino misurato nella CLI.** Dopo `confirm_best`, `combos --probe` ordina le board stampate per il punteggio composito misurato, non per `quick_score`. Stampa entrambi i numeri. L'app lo fa già: va verificato che lo faccia anche sulla top N mostrata.
2. **Valori condizionati in `value.rs`.** Ogni mostro e ogni carta scoperta si misurano due volte: con la mano del prober (3 carte semplici) e con **mano vuota**. Un nuovo `Spot` variante (`MainNoHand`, `ExtraNoHand`) e `of_field` usa la variante giusta secondo `state.players[0].hand.is_empty()`. Si corregge Dragoon con la mano vuota senza nominarlo.
3. **Seconda copia scoperta.** Si misura `Spot::Set` con **due** copie piazzate. Il valore marginale della seconda copia (spesso 0 per il limite di una attivazione per turno) sostituisce il +1,0 fisso di `quick_score` per i Set doppi.
4. **Bonus fissi.** +2,0 per mostro dell'Extra e +1,0 per mostro del Main si applicano **solo se il valore della carta non è stato misurato**. Quando è misurato, contano 0,3 come "corpo".
5. **Spot mancanti** (audit): `Spot::FaceUp` anche per le Trappole Continue e le Magie Equipaggiamento, `Spot::FaceDown` per i mostri coperti, `Spot::Pendulum` per le Scale.
6. **Look key.** `look_key` include i codici in Cimitero e bandite che sono comparsi in `offered` con `location` GRAVE o REMOVED in una misura della stessa ricerca. Le varianti che differiscono per carte usabili da lì si misurano separatamente. Esposizione e costo si calcolano sempre per linea.
7. **Varianti nella top N.** La top N si riempie con campi diversi. Le varianti di mano o di zona restano come "(+N varianti)", e la variante mostrata è quella con il punteggio misurato più alto.

---

## 4. Parte C: protocollo di validazione

### C1. Dati e regole fisse

- **Deck:** 37 liste MDM (`python3 scripts/mdm_decks.py --out <mdm>`) più gli 8 archetipi della cartella di studio. Non si committano.
- **Mano per deck:** `random.Random(f"{nome}-lines").sample(main, 5)`, la regola degli studi. I nomi `x_*` sono gli archetipi.
- **Ricerca:** `--nodes 3000 --threads 1 --seed 1 --top 8 --probe --lines`, quindi deterministica.
- **Misura del composito:** pesi del profilo Bilanciato. Lo script li legge da un solo posto, con la stessa lista in `scripts/board_pairs.py` e in `bench.py`. Più avanti si sposteranno in un `profiles.json` condiviso con `scoring.ts`, da chiedere a Gemini.

### C2. Strumenti da aggiungere prima di cambiare la misura (passo D0)

- `duelforge-cli combos ... --json <file>`: per ogni board stampa:
  - `look_key` e una **firma**: nomi del campo ordinati, con `[EMZ]` e il bersaglio dell'equipaggiamento, più la mano ordinata;
  - tutte le metriche, vecchie e nuove;
  - il composito Bilanciato;
  - le interruzioni con `alone`, `value`, `covers` ed `effect`;
  - i breaker;
  - l'esposizione;
  - il tempo della misura.
- `duelforge-cli combos ... --save-lines <file>`: salva `Line` (answers, choices, choice_at) delle board elencate.
- `duelforge-cli probe-line --deck D --hand H --lines <file> [--index k] --json`: misura una linea salvata **senza ricerca**. Le coppie si confrontano così anche quando la ricerca nuova non trova più quelle board.
- `scripts/board_pairs.py`:
  - `make <out>`: con la CLI di **prima**, salva le linee delle board citate in `tests/fixtures/boards/pairs.tsv`;
  - `eval <out> --cli <bin>`: misura tutte le coppie e stampa la tabella di §C5;
  - `sheet <out>`: genera per ogni deck le prime 5 board distinte, in ordine casuale e senza punteggi, da far ordinare al proprietario.
- `scripts/rank_report.py <json-dir>`: inversioni tra stima e misura, diversità della top 8, stabilità tra semi.
- `tests/fixtures/boards/pairs.tsv` (committabile: nomi di deck e di carte, nessuna lista). Colonne: `id`, `deck`, `firma_A`, `firma_B`, `atteso` (`A>B`, `B>A`, `A≈B`), `sicurezza` (`sicura` o `da confermare`), `perché`.

### C3. Insieme iniziale di coppie (dalle evidenze)

| id | deck | A | B | atteso | sicurezza | perché |
|---|---|---|---|---|---|---|
| P1 | azamina | #1 (Songs scoperta, mano vuota) | #6 (Songs in mano) | B>A | sicura | Dragoon senza carta da scartare non nega |
| P2 | exodia | #2 (Sequence come mostro, Desirae senza equip) | #6 (Sequence equipaggiato) | B>A | sicura | Desirae attiva solo con un Link equipaggiato |
| P3 | pumpking_zombies | #1 (due Vortex of Time) | #2 (Vortex più Undying Legion) | B>A | sicura | solo 1 Vortex per turno |
| P4 | x_labrynth | #1 (Little Knight più Torrential) | #8 (Arianna, Muckraker, Big Welcome, Torrential) | B>A | sicura | Torrential distrugge il proprio Little Knight; #8 ha più fermate |
| P5 | labrynth | #1 (Arianna, Big Welcome, Spark, Imperm, Daruma) | #2 (Daruma, Spark, Welcome, Imperm, Arias in mano) | A>B | da confermare | Big Welcome più Arianna contro una Trappola in più |
| P6 | doomz | #1 (Varudras) | #3 (Drastrius ×2, Vidolia) | A>B | sicura | negazione universale contro protezione dalla distruzione |
| P7 | phantom_knights | #1 (2 Fog Blade, 2 Impulse, Imperm) | #2 (1 Fog Blade, Launch, Soleret) | A>B | sicura | una Fog Blade in più |
| P8 | warrior_combo | #1 (Ha-Re più Fire Dance) | #5 (Mayowashidori equipaggiata su Herald) | da confermare | da confermare | equipaggiamento contro carta in più |
| P9 | volcanic | #1 (Caesar) | #2 (Lacrima, Token avversario) | A>B | da confermare | il floodgate di #2 è finto (A9) |
| I1 | x_spright | #1 | #2 | A≈B | sicura | stesse carte, solo la zona EMZ |
| I2 | goblin_biker | #1 | #2 | A≈B | sicura | Caesar in zona principale o Extra |
| I3 | synchrons | #1 | #2 | A≈B | sicura | stesse carte |
| I4 | elfnote | #1 | #4 | A≈B | sicura | solo la zona EMZ |
| I5 | argostars | #1 | #2 | A≈B | sicura | solo la zona EMZ |
| I6 | cyber_dragon | #1 | #2 | A≈B | sicura | differiscono nel terzo mostro dell'Extra senza effetto nel turno avversario |

**Zeri corretti** (regressioni): devono restare 0.

| Board | Perché vale 0 |
|---|---|
| Dragoon con mano vuota (azamina #1) | non ha carte da scartare |
| Drastrius rianimato senza materiali (doomz #5/#8) | non ha materiali |
| Exosister Mikailis non Evocata Xyz nel turno | l'effetto rapido vale solo nel turno in cui è Evocata Xyz |
| Fiendsmith's Desirae senza Link equipaggiato | le serve un Link equipaggiato |
| Secreterion Dragon | il suo floodgate tocca solo Draghi e Incantatori, che il kit non usa |

**Valori corretti da conservare:**
- Caesar: 33% ×2;
- Ash che nega Book of Eclipse.

L'insieme va allargato a **almeno 40 coppie, almeno una per deck**. Il proprietario ordina i fogli di `board_pairs.py sheet` (15 deck × 5 board bastano per partire). Ogni coppia nuova si segna `sicura` solo se il proprietario la dà per certa.

### C4. Test su board piazzate (unitari, `crates/ygo-solver/tests/probe.rs`)

| Test | Atteso |
|---|---|
| Impermanence scoperta da sola | `covers` ⊇ {FieldIgnition, DeckSummon}; valore ≥ 0,30 |
| Torrential scoperto da solo | `effect == Destroy`; nessuna Evocazione in `covers` |
| Floodgate più mostro senza effetti | nessun floodgate in più per i passi dipendenti |
| Token dato all'avversario | `floodgates` vuoto |
| Stesse carte, zone diverse | misure uguali entro 0,05 |
| Due copie scoperte senza limite per turno / con limite | `uses` 2 / 1 |
| Una negazione universale sola contro Dark Hole | `S_b ≥ 0,9` |
| Floodgate che sopravvive a un breaker | G dopo il breaker > 0 |
| Determinismo | stessa misura in due esecuzioni e con 1 o 4 thread di prober |

**Tabella di bilanciamento delle handtrap** (da `card-values` sui deck che le contengono). Si riporta il valore di Ash, Impermanence, Veiler, Droll, Nibiru, Ghost Belle, D.D. Crow e Maxx/Mulcharmy. Obiettivi:
- nessuna sopra 0,75;
- Ash ≤ 0,60;
- Impermanence e Veiler ≥ 0,30;
- D.D. Crow > 0.

### C5. Numeri da riportare (nella PR, prima e dopo)

1. **Accuratezza sulle coppie:** coppie `sicure` con il vincitore giusto rispetto al totale, e lo stesso per `da confermare`. Un vincitore è giusto se il segno di Δcomposito coincide con l'atteso e |Δ| ≥ 0,05. Obiettivo: ≥ 80% delle sicure, e nessuna coppia giusta prima e sbagliata dopo senza una spiegazione scritta.
2. **Invarianza:** numero di coppie `A≈B` con |Δcomposito| > 0,05, e il Δ massimo. Obiettivo: 0. Oggi 17 gruppi in 11 deck violano l'invarianza.
3. **Zeri corretti:** tutti a 0.
4. **Accordo con il proprietario:** τ di Kendall medio tra l'ordine del proprietario e il composito, sui deck ordinati, prima e dopo, più l'accordo sul #1. Obiettivo: τ più alto di almeno 0,15, e accordo sul #1 non più basso.
5. **Cambi del #1:** per ogni deck in cui cambia la board #1, una riga "prima → dopo, perché" e il giudizio del proprietario (meglio, peggio, uguale). Obiettivo: almeno il 70% "meglio" o "uguale".
6. **Stabilità tra semi:** ricerca con `--seed 1,2,3`. Per ogni deck si guardano:
   - l'indice di Jaccard delle firme delle prime 5;
   - il τ sulle board comuni;
   - la deviazione standard del composito per stessa `look_key`.

   Obiettivo: deviazione ≤ 0,05, e Jaccard e τ non peggiori di prima.
7. **Stima contro misura:** coppie della top 8 invertite di più di 0,1 tra `quick_score` e composito. Oggi sono 131 su 753 e 12 deck su 45 hanno un #1 dominato in tutte le metriche. Obiettivo dopo D14: ≤ 60, con il #1 stampato uguale al migliore misurato in tutti i deck.
8. **Diversità:** campi distinti nella top 8. Oggi 9 deck su 45 ne hanno al massimo 2.
9. **Tempo:** mediana e p95 della misura per board (dal JSON). Più le colonne `search`, `best` e `first` di `bench.py`, con il composito nuovo.
10. **Protocollo di AGENTS.md:** `bench.py` con `--nodes 4000`, `coverage` sui deck toccati e `combo_hunt.py`. Nessun crash nuovo e nessun "probe turn failed" in più.

### C6. Comandi

```bash
S=<cartella di lavoro>; CLI=target/release/duelforge-cli
python3 scripts/mdm_decks.py --out $S/mdm
# una volta, con la CLI di prima: linee delle coppie
python3 scripts/board_pairs.py make $S/pairs --decks $S/mdm --extra-decks $S/decks --cli $S/cli-before
# per ogni versione
python3 scripts/board_pairs.py eval $S/pairs --cli $CLI > $S/pairs-after.txt
for seed in 1 2 3; do
  python3 scripts/combo_hunt.py $S/mdm --hands 1 --nodes 3000 --top 8 --seed $seed --json $S/json-$seed
done
python3 scripts/rank_report.py $S/json-1 $S/json-2 $S/json-3 > $S/rank-after.txt
python3 scripts/bench.py $S/bench-after --decks $S/mdm --nodes 4000
$CLI --data data card-values --deck $S/mdm/<deck>.ydk        # tabella delle handtrap
```

`combo_hunt.py` ha bisogno di `--seed` e `--json`: sono modifiche piccole, da fare in D0.

---

### C7. Linea di base (21/10/2026)

Codice: il branch di Claude con mimalloc e meno replay (`c9ad88b`). Ricerca come in C1: 45 deck (37 MDM e 8 archetipi), mano della regola "-lines", 3000 nodi, un thread, prime 8 board misurate. Comandi: `combo_hunt.py --seed 1|2|3 --json`, `rank_report.py`, `board_pairs.py make|eval|sheet`. Nessun crash e nessun "probe turn failed".

| Numero (C5) | Oggi | Nel piano (20/10) | Obiettivo |
|---|---|---|---|
| 2. Invarianza: gruppi con le stesse carte che misurano diverso di più di 0,05 | 4 (Δ max 0,65) | 17 in 11 deck | 0 |
| 3. Zeri sbagliati (Dragoon a mano vuota, Desirae senza equip, Secreterion) | 0 | — | 0 |
| 7. Coppie della top 8 invertite di più di 0,1 tra stima e misura | 131 su 1260 | 131 su 753 | ≤ 60 |
| 7. Deck in cui il #1 elencato non è il migliore misurato | 12 su 45 | 12 su 45 | 0 |
| 8. Deck con al massimo 2 campi distinti nella top 8 | 5 | 9 | meno |
| 9. Tempo della misura per board | mediana 0,54 s, p95 1,56 s | — | — |
| 6. Stabilità tra semi: Jaccard medio delle prime 5 | 0,24 (90 confronti) | — | non peggiore |

Note:
- **Invarianza.** La correzione dei costi del turno di prova (§30.6 del PIANO) ha tolto quasi tutti i casi. Restano Synchrons, con F.A. Dawn Dragster: 4,01 in una zona principale e 3,35 nella Zona Mostri Extra (Δ 0,65, due gruppi), e White Forest Azamina, al limite (0,05).
- **Stabilità.** Il seme cambia anche l'ordine del deck: con la stessa mano le carte pescate o cercate cambiano (Azamina con il seme 2 trova board con un Infinite Impermanence pescato). Il Jaccard basso misura quindi sia questo sia il budget di nodi. Va confrontato prima e dopo, non con 1.
- **Coppie** (`board_pairs.py eval`). Sicure: 2 giuste su 3 misurate (P1 Dragoon a mano vuota, P2 Desirae senza equip). P4 risulta pari (1,38 contro 1,38) invece di B>A. P5, da confermare, esce al contrario (B>A). P3, P6, P7 e P9 descrivono board che la ricerca di oggi non trova più: vanno sostituite con coppie nuove.
- **Pannello di giudici.** Ancora da fare: tre agenti ordinano alla cieca i fogli di `board_pairs.py sheet` (prime 5 board distinte per deck) e le coppie su cui sono d'accordo entrano in `pairs.tsv` come sicure.

## 5. Parte D: ordine di implementazione

Tutto si fa su un branch `claude/board-score`, con lock in `docs/TASKS.md` su `probe.rs`, `value.rs`, `search.rs`, `lib.rs` e `api.ts` passo per passo. Ogni passo deve:
- compilare e passare `fmt`, `clippy` e i test;
- essere pushato;
- avere i numeri di C5 nella descrizione del commit o della PR.

I passi D1–D7 cambiano già molto i numeri. Si fa un giro completo di C5 dopo D7 e prima di toccare i pesi.

| Passo | Cosa | File | Test e numeri | Rischi |
|---|---|---|---|---|
| **D0** | Strumenti: `--json`, `--save-lines`, `probe-line`, `--seed` e `--json` in `combo_hunt.py`, `board_pairs.py`, `rank_report.py`, `pairs.tsv`. **Linea di base** di C5 con il codice di oggi | `ygo-cli/src/main.rs`, `scripts/`, `tests/fixtures/boards/` | i numeri di partenza | nessuno sul punteggio |
| **D1** | A1: bersagli e costi con contesto, ripiego su 3 opzioni | `probe.rs` | C4 Impermanence; I1 (spright); elfnote Ash | più run sui responder con bersaglio; una scelta sbagliata può peggiorare carte oggi giuste: controllare gli zeri corretti |
| **D2** | A2: credito al proprio passo; floodgate senza cascata; campo `also_prevents` | `probe.rs` | C4 Torrential e floodgate; Invoked e Kashtira con G più basso | i valori di `value.rs` scendono e la ricerca cambia: bench |
| **D3** | A3: `Event::Summoned`, `landed`, `NegateSummon` solo prima dell'atterraggio, denominatore unico | `ygo-engine/src/message.rs`, `probe.rs` | C4 Torrential; P4 | messaggio del motore da mappare bene; test del motore |
| **D4** | A4: effetto dal proprio anello, follow-up della stessa carta, etichetta stabile | `probe.rs` | Ash su x_kashtira; Daruma stabile; P5 | alcune catene legittime della stessa carta (trigger dopo la risoluzione) vanno conservate |
| **D5** | A5 più A6: istanze sul campo, finestre per passo, ordine canonico | `probe.rs` | C4 copie; I1–I6; P7 | cambiano gli indici delle finestre: commit separati |
| **D6** | A7: `UseUseful`, riduzione solo per conflitto, permutazioni ≤ 6, riprova del kit | `probe.rs` (`Kit`, `KitStep.retry`) | I2, I3; i casi "0.00 (alone X)" da 35 a meno di 5 | tempo (+6 run); la riprova cambia il kit: ricalibrare in D10 |
| **D7** | A9: carta del kit per `SpecialFromHand` senza condizioni; verifica Token | `probe.rs` | P9; volcanic G = 0 | carta del kit da scegliere nei dati: documentarla nel commento |
| — | **Giro completo di C5** (correzioni sole, pesi invariati) | — | tabella prima/dopo nella PR | — |
| **D8** | B3: resilienza come sopravvivenza; "breaker poi kit"; breaker fermati; non applicabili fuori dalla media; G dopo i breaker. `ScoreView`: `resilience` = S e nuovo `breakerLoss` (opzionale in `api.ts`). Richiesta in TASKS a Gemini per i profili | `probe.rs`, `lib.rs`, `api.ts`, `docs/TASKS.md` | C4 negazione contro Dark Hole; P6; tempo | file caldi (`lib.rs`, `api.ts`); tempo +20 run: verificare la mediana |
| **D9** | B4: G in interruzioni (κ = 1,5), breaker bloccati in G; `FLOODGATE_WEIGHT` ricalibrato con bench | `probe.rs`, `value.rs` | Invoked e Kashtira; bench | la scala nuova cambia la guida della ricerca |
| **D10** | B2: azioni nuove del kit (mano senza Deck, Cimitero che resta, Xyz o Synchro, Magia Rapida); `kit-mix` e pesi calibrati | `probe.rs`, `ygo-cli` | tabella delle handtrap di C4; P1–P9 | tutte le coperture cambiano; i vecchi numeri di PIANO §26 non sono più confrontabili: scriverlo |
| **D11** | B1: `stops`, `first_stop`, `bait_value` (solo report); poi, se C5.1 migliora, `F_eff` nel punteggio | `probe.rs`, `lib.rs`, `api.ts` | C5.1 e C5.4 con e senza `F_eff` | un'esca troppo aggressiva svaluta le negazioni con bersaglio: decidere con le coppie |
| **D12** | B6 V1: `ChokePoint`, `Prelude::Cut`, `quick_value`, `E` atteso con `p_h` dalle liste e il termine Maxx; cache | `explain.rs`, `probe.rs`, `lib.rs`, `api.ts` | linee lunghe (blue_eyes, tearlaments) non più penalizzate in modo lineare; tempo per board ≤ 2 s | V1 è ottimista (la carta non risulta spesa); il taglio deve cadere su una `IdleCmd` |
| **D13** | B8: costo nuovo e risorse senza Cimitero; B7 V1 (turno 3 dopo il breaker peggiore, `rebuild_effects`); B9 (finestre `PRE` ed `END`, `ep_gain`) | `lib.rs`, `probe.rs`, `api.ts` | branded #1 (Pot of Desires) paga il costo; tearlaments e argostars con follow-up > 0 | B9 tocca il ciclo di `run()`: commit separato con controllo che il resto non cambi |
| **D14** | B11: riordino misurato nella CLI, valori a mano vuota, seconda copia, bonus fissi solo se non misurati, spot nuovi, look key con Cimitero usabile, top N diversa | `search.rs`, `value.rs`, `ygo-cli` | C5.7 (inversioni ≤ 60), C5.8; bench sul composito nuovo | `quick_score` guida la ricerca: rischio di board peggiori; si accetta solo se bench (best) non scende più del 5% |
| **D15** | B5: breaker per assi, risposta alla tua negazione, protezione per pezzo (report), coppia peggiore in L | `probe.rs`, `lib.rs`, `api.ts` | P6; deck con protezioni (Baronne, Sifr) | il tempo cresce: limitare a 3 breaker nuovi più 3 coppie |
| **D16** | B10 (responder nuovi) più B6 V2 (handtrap vera e ripresa con ricerca breve) | `probe.rs`, `search.rs` (`SearchConfig.prefix`) | dracotail (Mululu), exosister; esposizione V2 contro V1 | la parte più costosa: in app solo a richiesta, per la board aperta |
| **D17** | Pesi dei profili calibrati: una ricerca a griglia sui pesi del Bilanciato che massimizza C5.1 (sicure più quelle del proprietario), con un vincolo di vicinanza ai pesi attuali. Il risultato va in TASKS per Gemini (`scoring.ts`) | script, `docs/TASKS.md` | C5.1 e C5.4 finali | sovradattamento a poche coppie: tenere fuori il 30% delle coppie come verifica |

**Documenti.** Alla fine di D7, D11 e D17 vanno aggiornati:
- `docs/PIANO.md`, nuova §31 "Classificazione delle board", con le tabelle prima/dopo;
- `docs/STATO.md`;
- `docs/TASKS.md`, task #47 e #43.

---

## 6. Rischi generali e come contenerli

- **Tempo di misura.** Ogni passo riporta mediana e p95. Se si supera il budget:
  - le parti B3.6 (coppie), B5 e B6 si limitano alle prime 8 board;
  - nell'app si calcolano in un secondo giro, dopo che la classifica è già visibile.
- **La ricerca cambia guida.** `value.rs` usa `measure_placed`, quindi ogni correzione al prober cambia i valori delle carte e l'ordine della ricerca. Dopo ogni passo che tocca la misura: bench con 4000 nodi, poi `combo_hunt` come regressione.
- **Kit "su misura" per gli studi.** Le carte e i pesi del kit si scelgono con regole scritte (azioni semplici, frequenze dalle liste) e un commento datato. Il kit per matchup (task #44) ne è l'estensione naturale, con la stessa struttura per assi.
- **Contratto con l'interfaccia.** I campi nuovi sono opzionali e quelli vecchi mantengono il significato finché `scoring.ts` non cambia. Ogni PR che tocca `ScoreView` aggiorna anche `api.ts`, con il lock.
- **Validazione povera.** Le coppie sicure sono poche (circa 9 più 6 di invarianza). Il τ con il proprietario (C5.4) e i cambi del #1 giudicati (C5.5) sono la prova principale. Senza almeno 15 deck ordinati dal proprietario, i pesi (D17) non si toccano.

---

## 7. Tracciabilità: segnalazioni → passi

| Segnalazione degli studi | Passo |
|---|---|
| Bersaglio sulla prima carta; costi naïf; spright | D1 |
| Fermate a cascata; floodgate a cascata | D2 |
| Unità della copertura; "nega l'Evocazione" falso; Torrential | D3 |
| Effetto preso da altre carte; etichette instabili; Ash "bandisce" | D4 |
| Copie fuse; finestre `take(12)`; dipendenza dalle zone | D5 |
| "0.00 (alone X)"; costi condivisi dipendenti dall'ordine | D6 |
| Token che crea floodgate finti | D7 |
| Breaker negato = board distrutta; resilienza che raddoppia F; non applicabili a pieno; floodgate non nella resilienza; floater e ricorsione | D8 |
| Floodgate fuori scala | D9 |
| Kit troppo sul Deck (Ash quasi universale); pesi a mano | D10 |
| Esca e ordine; numero di fermate; prima fermata | D11 |
| Esposizione come conteggio; Maxx "C" escluso | D12, D16 |
| Costo e risorse doppi; costi di Deck, Extra ed LP; follow-up solo per ruoli; Standby ed End Phase | D13 |
| Stima che ordina male; Set doppi; Extra +2; condizioni; varianti per Cimitero; top 8 duplicata | D14 |
| Breaker con bersaglio, non concatenabili, a fine Battle Phase; contro-negazione; protezione dei pezzi; due breaker | D15 |
| Responder che nascono nel turno avversario; esposizione con ripresa | D16 |
| Pesi dei profili | D17 |