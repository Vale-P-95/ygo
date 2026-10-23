# Piano per l'ambiente di lavoro e le risorse (24/10/2026)

Come sviluppare DuelForge nella prossima fase sfruttando Claude (Progetti, Design, Code) e GitHub, senza sprecare token e minuti. Nasce dai numeri del 23/10, il giorno con dieci sessioni in parallelo: ha prodotto undici PR unite, ma anche sprechi evidenti, misurati qui sotto. Le regole stabili vanno nei file elencati al §8, così non si ripetono a ogni prompt.

Il piano del prodotto resta `docs/PIANO.md`; i lavori in corso `docs/TASKS.md`; la memoria del coordinamento `docs/plans/coordinamento.md`. Questo documento dice **come** si lavora, non **cosa** si costruisce.

## 1. Diagnosi: dove sono finiti i token

Dati letti dalle sessioni il 23/10 (`list_sessions`, campo `usage`):

| Sessione | Contesto finale | Spesa | Note |
|---|---|---|---|
| Vecchia coordinatrice (5 giorni) | 263k dopo la compattazione, 3,6 miliardi di token letti dalla cache | 1 244 $ | Monitor di git, trigger orario, `list_events` sulle sorelle |
| Corsia 2, velocità | 466k | 145 $ | Controlli frequenti sui calcoli lunghi: ogni risveglio rilegge mezzo milione di token |
| Corsia 10, bibliografia (Fable) | 769k | 156 $ | 5 milioni di token in ingresso senza cache, 1,4 milioni in uscita |
| Corsia 4, misura | 717k | 131 $ | Tre compiti in fila nella stessa sessione |
| Corsia 3, banco | 380k | 85 $ | |
| Corsia 6, oracolo | 700k | 89 $ | Calcoli di ore, con l'agente che aspetta |
| Corsia 7, ricerca | 146k | 77 $ | Compattata a metà: costo per turno molto più basso dopo |
| Corsia 9, revisione (Fable) | 652k | 66 $ | |
| Corsia 8, generalità | 594k | 51 $ | |
| Nuova coordinatrice (un pomeriggio) | 250k | 7 $ | Nessun monitor, nessun trigger, solo eventi |

Le cause, in ordine di peso:

1. **Contesti enormi riletti a ogni turno.** Una sessione con 600k token di contesto paga quei 600k (dalla cache, ma li paga) a ogni messaggio, a ogni notifica, a ogni controllo di un calcolo. Il costo di un turno cresce con la lunghezza della storia, non con la difficoltà del compito. È la causa principale: le sessioni più care sono quelle con il contesto più lungo, non quelle con il lavoro più difficile.
2. **Sessioni longeve con più compiti in fila.** Il contesto del compito 1 resta nella storia mentre si fa il compito 3. La corsia 4 ha fatto tre PR nella stessa sessione a 717k.
3. **Aspettare un calcolo lungo dentro la sessione.** Un banco di 3 ore con un agente che «controlla come va» ogni dieci minuti costa un turno pieno a controllo. Il calcolo non ha bisogno dell'agente finché non finisce.
4. **Svegliarsi per niente.** Monitor di git a ogni push, trigger orario, `list_events` sui transcript delle sorelle (decine di migliaia di token a lettura), `list_sessions` (il JSON completo di dieci sessioni vale 15-20k token).
5. **Modello sbagliato per il compito.** Fable 5.1 su una lettura di cento articoli a testo intero: utile per il ragionamento, costoso per il volume. Opus 5.5 per scrivere una riga in un documento.
6. **Lavoro doppio.** Agenti subordinati lanciati per verifiche che il coordinatore poi rifaceva; la stessa CI rilanciata a mano dopo ogni push; revisioni in Chromium per PR piccole.
7. **Ragionamento alto per compiti di segreteria.** Il coordinamento (aprire PR, fare merge, smistare messaggi) non richiede il livello di ragionamento del lavoro sul motore.

## 2. Regole per i token

Le regole che seguono valgono per ogni sessione Claude del progetto. Vanno in `CLAUDE.md` (§8) perché si applichino da sole.

### 2.1 Una sessione, un compito, un contesto corto
- **Una sessione per compito**, creata da `main` con un brief di poche righe, chiusa e archiviata quando la PR è unita. Mai un secondo compito nella stessa sessione: si apre una sessione nuova, che parte da zero e legge il brief.
- **Soglia di contesto: 200k token.** Oltre, la sessione fa un passaggio di consegne (`docs/plans/<tema>.md` con stato, numeri e prossimo passo), pusha, e chiede di essere sostituita. Non si aspetta la compattazione automatica, che arriva tardi e perde dettagli.
- **Il brief è il contesto.** Nel prompt iniziale: il compito, i file da toccare, i comandi di verifica, il criterio di accettazione, il nome del documento di piano da leggere. Niente storia: la storia sta nei documenti.
- **Leggere solo ciò che serve.** `Read` con `offset`/`limit`; `grep` prima di `cat`; mai leggere `docs/PIANO.md` (1 400 righe) per intero: si legge `docs/STATO.md` e il piano del tema.
- **Niente transcript altrui.** `list_events` è vietato salvo per diagnosticare una sessione bloccata. Ci si fida dei messaggi.
- **`list_sessions` al massimo una volta per turno utente**, e solo quando l'utente chiede lo stato o i costi.

### 2.2 I calcoli lunghi non tengono sveglio nessuno
- Un calcolo che dura più di dieci minuti (banco, sweep, oracolo, `paired_test`) si lancia con `run_in_background` e un timeout generoso, e **la sessione finisce il turno**. Il risultato arriva come notifica di completamento: un solo risveglio.
- Niente `Monitor` con `while true` su file di log; niente `sleep` in foreground; niente «controllo a che punto è».
- Se il calcolo supera le due ore, si sposta su GitHub Actions (`bench.yml` già esiste, su richiesta) o sulla macchina dell'utente, e la sessione si chiude: la riapre il risultato.
- Ogni script lungo scrive un **riassunto di dieci righe** in un file (`out/<nome>/summary.txt`). La sessione legge il riassunto, non il log.

### 2.3 Svegliarsi solo per eventi
- Nessun trigger orario per «vedere se c'è qualcosa». Le sessioni si svegliano per: un messaggio dell'utente, una notifica di fine calcolo, un evento GitHub sulla PR a cui sono iscritte, un messaggio di un'altra sessione.
- Una sola iscrizione agli eventi per PR, tolta al merge (è automatico).
- Le sessioni sorelle scrivono alla coordinatrice **solo** per: PR pronta, blocco, richiesta di un file caldo. Non per «ho finito il passo 2 di 5».
- Un messaggio a una sessione con contesto grande costa un turno di quella sessione: prima di scrivere, chiedersi se serve davvero.

### 2.4 Il modello giusto per il compito
| Compito | Modello | Livello di ragionamento |
|---|---|---|
| Architettura, bug difficili del motore, algoritmi di ricerca, decisioni di misura | **Opus 5.5** | alto |
| Revisione di una PR di Gemini, documenti, script Python, test, correzioni piccole, CLI | **Sonnet 5.5** | medio |
| Coordinamento: aprire PR, merge, smistare messaggi, aggiornare `coordinamento.md` | **Sonnet 5.5** | basso |
| Agenti subordinati di ricerca nel codice (`Explore`), riassunti di log, verifiche meccaniche | **Haiku 5.5** | basso |
| Ragionamento profondo su un problema delimitato (una dimostrazione, un piano di esperimenti), con contesto corto | Fable 5.1 | alto, con contesto sotto 100k |

- Il modello si imposta alla creazione della sessione e con `/model`; il livello con `/effort`. Chi crea una sessione sorella (`create_session`) passa il modello nel parametro `model`.
- Fable 5.1 **mai** per letture lunghe, cicli di prova e correzione, o sessioni che durano ore: costa di più per token e il vantaggio sparisce quando il lavoro è volume, non ragionamento.
- Gli agenti subordinati (`Agent`) ricevono `model: "haiku"` o `"sonnet"` a meno che il compito non richieda Opus. Un subagente eredita il modello del padre se non si specifica: con Opus in sessione, ogni `Explore` è un Opus.
- `/fast` per i turni di segreteria con molto output (scrivere un documento già deciso).

### 2.5 Quota del piano Max
Il piano ha due finestre: 5 ore e 7 giorni. Il 23/10 la finestra settimanale era in «avviso» alle 14. Regole:
- Le sessioni in parallelo si limitano a **tre** contemporanee, più la coordinatrice. Dieci corsie in un giorno hanno consumato in otto ore quello che tre corsie consumano in una settimana, con molto lavoro di attesa.
- Il lavoro che consuma solo CPU (banchi, sweep) va in coda su una sola sessione o su Actions, non in parallelo su sessioni Claude.
- Un controllo dei costi a fine giornata, con `list_sessions` (una volta), si annota in `coordinamento.md`: spesa per sessione e contesto. Se una sessione supera i 300k, si chiude il giorno dopo con un passaggio di consegne.

## 3. Gli strumenti e il loro ruolo

### 3.1 Claude Code sul web (claude.ai/code): motore, dati, backend dell'app
- È l'ambiente di oggi: container Linux, Rust, Node, Chromium, accesso al repository, CI via GitHub MCP, messaggi tra sessioni, task programmati.
- Vi si fa tutto il lavoro su `crates/`, `apps/desktop/src-tauri/` e `scripts/`: compila, testa, lancia la CLI (`combos`, `coverage`, `card-values`, `roles`), fa i banchi corti.
- **Limite:** non può aprire l'app Tauri con la sua finestra. Il backend dell'app (`lib.rs`) si compila con `cargo check -p duelforge-desktop` e si prova attraverso la CLI e i test; l'app vera si prova altrove (§6).
- L'ambiente (`env_01V1q8vVFjcriRQeHs7ZLwV8`) va sistemato una volta: script di avvio che scarica `data/` in cache (oggi ogni sessione nuova rifà `fetch-data`), variabili `DUELFORGE_DATA` e `VITE_MOCK`, rete aperta verso Master Duel Meta, YGOPRODeck, crates.io, npm.

### 3.2 Claude Code sul computer (app desktop o CLI su Windows): l'app vera
- È l'unico posto dove gira `pnpm tauri dev` con la finestra, le texture, i suoni, il 3D della Bottega. Ogni PR di interfaccia si prova qui prima del merge, o lo fa l'utente.
- Con **Remote Control** una sessione cloud può usare il computer dell'utente: lanciare l'app, fare screenshot, provare un pulsante. Da attivare una volta nelle impostazioni; poi la sessione di revisione dell'interfaccia lo usa invece di dichiarare «non provato nell'app vera» (come nelle PR #113 e #116).
- Il repository locale va tenuto su `main` aggiornato, con `data/` già scaricato: così la prova di una PR è `git fetch && git checkout gemini/<branch> && pnpm tauri dev`, due minuti.

### 3.3 Antigravity/Gemini: interfaccia
- Resta sull'interfaccia (`apps/desktop/src/`), con i suoi branch `gemini/*`. Non consuma la quota Claude. Legge `AGENTS.md`.
- Ogni suo prompt ha le due sezioni fisse (controlli automatici con i comandi; attività in pausa da ricontrollare in `TASKS.md`). Il prompt lo prepara la coordinatrice, l'utente lo incolla.
- Le sue PR le rivede una sessione Claude **Sonnet** con brief corto: area giusta, protocollo dell'interfaccia di `AGENTS.md`, CI verde, e la prova nell'app vera via Remote Control quando la PR tocca pagine intere.

### 3.4 Progetti Claude (claude.ai): la memoria e le decisioni
- Un **Progetto «DuelForge»** con, come conoscenza: `docs/STATO.md`, `AGENTS.md`, questo piano, `docs/plans/coordinamento.md`, il glossario dei termini (board, kit, turno di prova, oracolo, A1b, banco di generalità). Si aggiornano quando cambiano su `main`, non a ogni PR.
- Istruzioni del Progetto: lingua, convenzioni, i principi non negoziabili (motore generale, niente linee precalcolate, prova equa sulle 406 board), e la regola «le decisioni si scrivono in `docs/plans/<tema>.md`, mai solo in chat».
- Serve per: **discutere le decisioni** (quale opzione della misura, cosa fare della (d), le priorità del prossimo giro) senza aprire una sessione Code con il repository; **preparare i brief** delle sessioni; **leggere i riepiloghi**. Una chat nel Progetto costa una frazione di una sessione Code, perché non ha strumenti, repository né notifiche.
- Il popup «Continua il lavoro sulle pull request tra le sessioni» propone proprio un Progetto collegato a Code: quando è attivo, le sessioni Code nate dal Progetto ne ereditano istruzioni e memoria, e il Progetto avvisa quando una PR finisce o serve l'utente. Da attivare al prossimo giro, con la sessione coordinatrice chiusa.

### 3.5 Claude Design: prima di Gemini, non dopo
- I passi dell'interfaccia (`docs/plans/ui-revisione-2.md`) oggi partono da una descrizione testuale e si giudicano sul risultato. Con Claude Design si fa prima il **mockup** della schermata (Bottega con mensole e scatole, scheda del punteggio con il meta) sul canvas, con il sistema di design di DuelForge (token di `src/index.css`, Inter e Cinzel, i due temi), e lo si rivede con l'utente. Gemini riceve il mockup esportato più le specifiche, invece di interpretare un testo.
- Un **sistema di design** di DuelForge in Claude Design, costruito dai token e dai componenti base del passo 0 (`Toolbar`, `FilterChip`, `IconButton`, `SidePanel`, `HelpPopover`): si fa una volta, poi ogni mockup lo riusa.
- Vale per le pagine nuove e i rifacimenti grandi (passi 5 e 6, l'interfaccia del simulatore #36). Non per i ritocchi.

### 3.6 Artifact: anteprima del frontend senza installare niente
- Il frontend con il backend finto (`VITE_MOCK=1 pnpm build`) è un sito statico. Una sessione cloud lo può **pubblicare come Artifact a più file** (pagina più `dist/assets/*`), privato, apribile dal telefono. Non ha il motore, ma mostra layout, temi, testi, animazioni, con i dati finti di `mock.ts`.
- Si usa per rivedere le PR di Gemini prima del merge e per far vedere all'utente una schermata senza che debba compilare. Costa una build (un minuto) e una pubblicazione; il limite di 16 MB per pagina va rispettato tenendo i font e le texture fuori dall'anteprima o sotto i limiti.

### 3.7 GitHub: cosa usare e cosa no
- **Branch protection** su `main` (c'è): PR con CI verde. Aggiungere la regola «una approvazione» no: il merge lo fa la coordinatrice su mandato dell'utente, e un'approvazione formale costerebbe un turno in più per PR.
- **Template di PR** (`.github/pull_request_template.md`): sezioni «Cosa cambia», «Numeri prima/dopo» (per il motore), «Verifica punto per punto» (per l'interfaccia), «Attività in pausa toccate». Oggi ogni PR lo riscrive a mano.
- **CODEOWNERS**: `crates/`, `src-tauri/` a Claude; `apps/desktop/src/` a Gemini. Serve come documentazione delle aree, non per bloccare.
- **Actions**: il budget è contato. Restano le regole di `ci.yml` (PR una volta, solo le aree toccate; tutto su `main`; installer solo su `main`). Il bench e i banchi lunghi su `workflow_dispatch` quando una sessione cloud non deve restare sveglia ore. I risultati tornano come artifact di Actions, non in chat.
- **Issues** per le richieste dell'utente e i bug trovati (oggi stanno in `TASKS.md` e nel PIANO): un'issue per tema, con l'etichetta dell'area; la PR la chiude con `Closes #n`. `TASKS.md` resta per i lock e lo stato operativo, non per la lista dei desideri.
- **Projects di GitHub**: no. Un'altra bacheca da tenere aggiornata costa turni; `TASKS.md` più le issue bastano.
- **Dependabot**: solo per le vulnerabilità, non per gli aggiornamenti di routine (ogni PR di aggiornamento costa una CI).
- **Release**: tag `v0.x` quando `main` passa l'installer; l'utente scarica l'installer dall'Actions run o dalla release.

## 4. Backend, frontend e ambiente: dove si fa cosa

| Parte | Dove si sviluppa | Come si prova | Chi |
|---|---|---|---|
| Motore (`ygo-engine`, `ygo-solver`), dati, statistiche, CLI | Claude Code web | `cargo test --release`, CLI, `paired_test.py`, banchi corti; banchi lunghi su Actions | Claude (Opus per gli algoritmi, Sonnet per script e documenti) |
| Backend dell'app (`src-tauri/src/lib.rs`) | Claude Code web | `cargo check -p duelforge-desktop`, clippy; la funzione si prova dalla CLI che la copre (`combos --staged`, ecc.) | Claude |
| Contratto `lib.rs` ↔ `api.ts` | Una sola PR che tocca entrambi | `pnpm typecheck` più `cargo check`; `mock.ts` aggiornato nella stessa PR | Claude propone, Gemini consuma |
| Interfaccia (`apps/desktop/src/`) | Antigravity sul computer dell'utente, con `pnpm tauri dev` e `DUELFORGE_DATA` | App vera, Vitest, Playwright (`screens.mjs`), anteprima Artifact | Gemini; revisione Claude Sonnet |
| Mockup e sistema di design | Claude Design | Revisione con l'utente sul canvas | Claude + utente |
| Decisioni e priorità | Progetto Claude | Documento in `docs/plans/` | Utente + Claude |
| Prova finale, installer | Computer Windows dell'utente, Actions per l'installer | Installer da `main` | Utente |

Regole del contratto:
- Un campo nuovo in `api.ts` nasce in una PR di Claude che tocca `lib.rs`, `api.ts` e `mock.ts` insieme, con lo stub finto che produce dati plausibili. Gemini non aspetta: lavora sul finto, come per `comboMeta`.
- Il backend non manda mai all'interfaccia dati che l'interfaccia non sa mostrare (è la regola di `SEND_CHEAP_SCORES`): l'interruttore sta nel backend e si accende quando Gemini ha fatto la sua parte.

## 5. Il ciclo di lavoro (un compito dall'idea al merge)

1. **Decisione** nel Progetto Claude o in `docs/plans/<tema>.md`: cosa, perché, criterio di accettazione, numeri attesi. Costa poco, non apre sessioni Code.
2. **Issue** su GitHub con il riassunto e il collegamento al piano.
3. **Brief** di dieci righe (modello da §8.4). La coordinatrice lo scrive; l'utente, o la coordinatrice con `create_session`, apre la sessione con il modello giusto.
4. **Lavoro** in una sessione sola: piccoli commit che compilano, push dopo ogni passo, calcoli lunghi in background con la sessione che finisce il turno.
5. **PR** aperta dalla sessione stessa con il template; CI parte da sola all'apertura. Se serve un push dopo, la CI si rilancia una volta a mano, non a ogni commit.
6. **Revisione** da una sessione Sonnet diversa (o dalla coordinatrice se il diff è piccolo). Per Gemini: sempre.
7. **Merge** squash dalla coordinatrice, con CI verde e mandato dell'utente.
8. **Chiusura**: la sessione aggiorna `TASKS.md` e, se cambiano decisioni, `STATO.md`; l'utente la archivia. Una sessione archiviata non consuma più niente.

La coordinatrice non è un servizio sempre acceso: lavora a eventi (PR, messaggi) e sta sotto i 250k di contesto. Quando li supera, passa le consegne con `coordinamento.md`, come è successo il 23/10.

## 6. Provare l'app in tempo reale (o quasi)

Dal più immediato al più completo:

1. **Anteprima Artifact del frontend finto** (§3.6): zero installazioni, si apre dal telefono, mostra layout e flussi con i dati di `mock.ts`. Non mostra il motore né il 3D pesante. Tempo: un minuto dopo ogni push di Gemini, pubblicata dalla sessione di revisione.
2. **Screenshot automatici**: `pnpm screens` (Playwright, con l'audit di accessibilità) in CI sul job frontend, con gli screenshot come artifact di Actions. Si vedono nella PR senza aprire niente. Costa pochi secondi di Actions.
3. **`pnpm tauri dev` sul computer** con ricarica a caldo: l'interfaccia si aggiorna mentre Gemini salva. È l'ambiente di Antigravity, quindi è già «in tempo reale» per chi guarda quello schermo. Con **Remote Control**, anche una sessione cloud lo vede e lo usa.
4. **Build release locale** (`pnpm tauri build` con `--features tauri/custom-protocol`, come dice `CLAUDE.md`) per le prove di velocità vera del motore: il modo dev è più lento.
5. **Installer da `main`**: la prova completa, dopo il merge. Si scarica dall'Actions run.

Consiglio: 1 e 2 per ogni PR di interfaccia; 3 per i passi grandi (5 e 6) e per ogni cosa che tocca il motore dall'app; 5 una volta a settimana.

## 7. Qualità senza spreco
- **Niente prove ridondanti.** La CI della PR è la prova; non si rifanno `cargo test` completi in sessione se la CI li fa. In sessione si fa il test del pezzo toccato (`cargo test -p ygo-solver --test <nome>`).
- **`search_regression` e i banchi** restano l'arbitro per il motore, con `paired_test.py` e la prova equa sulle 406 board: nessuna PR del motore senza numeri, come oggi. Il banco di generalità (tuning/fresh, PR #111) entra nel protocollo per ogni costante nuova.
- **Le prove lunghe si fanno una volta**, prima della PR, e i numeri vanno nella PR. La revisione non le ripete.
- **Agenti subordinati**: solo per cercare nel codice (Haiku) o per una revisione indipendente di una PR grande (Sonnet). Mai per «verificare» ciò che la sessione ha appena fatto.
- **Workflow multi-agente**: solo per revisioni grandi o analisi su molti file, su richiesta esplicita (regola già in `CLAUDE.md`).

## 8. Le regole stabili, nei file giusti

Ogni regola sta in un posto solo, letto da chi deve rispettarla.

### 8.1 `AGENTS.md` (c'è): regole per tutti gli agenti
Resta com'è: mappa, comandi, convenzioni, protocolli del motore e dell'interfaccia, cosa non fare, coordinamento. Aggiunte:
- un rimando a questo piano nella sezione «Coordinamento»;
- la regola «una sessione, un compito» e la soglia di contesto (§2.1);
- la regola dei calcoli lunghi in background (§2.2).

### 8.2 `CLAUDE.md` (c'è): solo per Claude Code
Oggi ha cinque righe. Diventa il posto delle regole di §2 che Gemini non deve leggere:
- tabella dei modelli (§2.4) e livello di ragionamento per tipo di compito;
- regole sui risvegli (§2.3): niente monitor, niente trigger, iscrizione per PR;
- `list_events` vietato, `list_sessions` solo su richiesta;
- il brief standard (§8.4) e il passaggio di consegne a 200k;
- i file caldi e come si chiede il lock.

### 8.3 `.claude/settings.json` (da creare): permessi e comportamento
- Elenco dei comandi permessi senza conferma: `cargo fmt/clippy/test/check`, `pnpm lint/typecheck/test/build`, `git commit/push` sul proprio branch, `python3 scripts/*.py`. Il 23/10 il classificatore ha bloccato quattro volte commit e push di documenti: ogni blocco è un turno perso e un messaggio all'utente.
- Comandi vietati: `git push --force`, `git rebase` su branch altrui, qualunque scrittura in `vendor/`.
- Hook di fine sessione già in uso (controllo dei cambi non committati) resta.

### 8.4 `.claude/skills/` (da creare): le procedure ripetute
Ogni skill è un file `SKILL.md` che una sessione carica quando serve, invece di ricevere le istruzioni nel prompt:
- **`brief`**: il modello del brief di una sessione (compito, file, comandi, criterio, piano da leggere, modello consigliato).
- **`pr-motore`**: il protocollo di `AGENTS.md` per il motore con i comandi esatti (`mdm_decks.py`, `bench.py`, `paired_test.py`, `a1b_fair_measure.py`, `generality.py`), e la tabella numeri prima/dopo da incollare nella PR.
- **`revisione-gemini`**: la lista di controllo del protocollo dell'interfaccia, le due sezioni fisse del commento, il piè di pagina, il comando per l'anteprima Artifact e per Remote Control.
- **`coordinamento`**: aprire la PR, iscriversi agli eventi, merge con squash e `expectedHeadSha` completo, aggiornare `coordinamento.md`, quando passare le consegne.
- **`steward`** (letta automaticamente da chi guida una PR): convenzioni di merge e CI del repository, quando rilanciare la CI (una volta dopo l'ultimo push, mai a ogni commit).

### 8.5 `.github/` (da creare o completare)
- `pull_request_template.md` (§3.7).
- `CODEOWNERS` (§3.7).
- In `ci.yml`: gli screenshot Playwright come artifact del job frontend (§6.2).

### 8.6 Il Progetto Claude
Istruzioni del Progetto: le righe di `AGENTS.md` «Convenzioni» e «Cosa NON fare», i principi non negoziabili, e «ogni decisione finisce in `docs/plans/`». Conoscenza: `STATO.md`, questo piano, `coordinamento.md`, il glossario.

### 8.7 `docs/plans/coordinamento.md` (c'è): la memoria operativa
Resta la memoria della coordinatrice: corsie, PR, decisioni del giorno, costi. Si unisce a `main` con una PR di soli documenti a fine giornata.

## 9. Cosa fare adesso, in ordine
1. **Chiudere il giro**: aspettare la PR dell'A1b (corsia 6), unirla, archiviare la sessione. Unire `coordinamento.md` e questo piano a `main`.
2. **Scrivere le regole** (§8.1-8.5) in una PR di soli documenti e configurazione: `CLAUDE.md`, `AGENTS.md` (tre righe), `.claude/settings.json`, le cinque skill, il template di PR, `CODEOWNERS`. Mezza giornata di una sessione Sonnet.
3. **Sistemare l'ambiente cloud**: script di avvio con `data/` in cache, variabili, rete. Una sessione, una volta.
4. **Creare il Progetto Claude** con conoscenza e istruzioni; attivare il collegamento con Code dal popup; attivare Remote Control sul computer.
5. **Il sistema di design** in Claude Design, dai token e dai componenti del passo 0; il mockup della Bottega (passo 5, in corso con Gemini) e del passo 6.
6. **Riaprire i compiti** del prossimo giro, uno per sessione, al massimo tre in parallelo, secondo `docs/STATO.md` e ciò che è rimasto in coda il 23/10:
   - i due pesi del punteggio proposti dalla §7 (resilienza 0,6 → 0, floodgate 0,8 → 0,4), con la conferma sul banco di generalità e il cambio coordinato in `bench.py`, CLI, `lib.rs`, `scoring.ts` (una PR, Sonnet per il cambio, Opus se il banco dice no);
   - i punti 3, 5 e 6 della caccia ai bug e l'arena nel turno di prova (Opus, uno per sessione);
   - il caso Vendread e il tetto Lua (Opus);
   - le board `cheap: true` nell'interfaccia e `SEND_CHEAP_SCORES` (Gemini, poi Claude Sonnet per l'interruttore);
   - B8, gli obiettivi ricchi: il contratto `lib.rs` ↔ `api.ts` (Opus per il contratto, Gemini dopo).

## 10. Come si misura se funziona
Da leggere a fine di ogni settimana, in `coordinamento.md`:
- spesa per PR unita (il 23/10: circa 85 $ per PR, contando tutte le sessioni); obiettivo: sotto 30 $;
- contesto massimo delle sessioni chiuse; obiettivo: nessuna sopra 250k;
- turni «a vuoto» (risvegli senza azione); obiettivo: zero monitor e zero trigger;
- minuti di Actions al mese, con la CI che resta sotto il budget del repository;
- PR dell'interfaccia provate nell'app vera prima del merge; obiettivo: tutte quelle che toccano una pagina intera.
