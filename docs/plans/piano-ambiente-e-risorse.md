# Piano per l'ambiente di lavoro e le risorse (v2, 24/10/2026)

Come sviluppare DuelForge nella prossima fase con Claude (Progetti, Design, Code) e GitHub, **frontend compreso**, senza sprecare token e minuti. La v1 teneva l'interfaccia su Gemini; questa la riporta su Claude, perché il coordinamento tra due agenti costava turni (prompt da preparare, revisioni incrociate, contratti in due PR, prove «non fatte nell'app vera») senza dare velocità.

Il piano del prodotto resta `docs/PIANO.md`; i lavori in corso `docs/TASKS.md`; la memoria del coordinamento `docs/plans/coordinamento.md`. Questo documento dice **come** si lavora, non **cosa** si costruisce.

## 1. Diagnosi: dove sono finiti i token

Dati letti dalle sessioni il 23/10 (`list_sessions`, campo `usage`):

| Sessione | Contesto finale | Spesa | Note |
|---|---|---|---|
| Vecchia coordinatrice (5 giorni) | 263k dopo la compattazione; 3,6 miliardi di token letti dalla cache | 1 244 $ | Monitor di git, trigger orario, `list_events` sulle sorelle |
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

1. **Contesti enormi riletti a ogni turno.** Una sessione con 600k token di contesto paga quei 600k (dalla cache, ma li paga) a ogni messaggio, notifica o controllo di un calcolo. Il costo di un turno cresce con la lunghezza della storia, non con la difficoltà del compito. È la causa principale.
2. **Sessioni longeve con più compiti in fila.** Il contesto del compito 1 resta nella storia mentre si fa il compito 3.
3. **Aspettare un calcolo lungo dentro la sessione.** Un banco di 3 ore con un agente che «controlla come va» ogni dieci minuti costa un turno pieno a controllo.
4. **Svegliarsi per niente.** Monitor di git, trigger orario, `list_events` sui transcript altrui (decine di migliaia di token a lettura), `list_sessions` (15-20k token a chiamata).
5. **Modello sbagliato per il compito.** Fable 5.1 su cento articoli a testo intero; Opus 5.5 per una riga in un documento.
6. **Lavoro doppio e coordinamento a due.** Agenti subordinati per verifiche poi rifatte; CI rilanciata dopo ogni push; prompt per Gemini preparati a mano, revisioni delle sue PR, contratti `lib.rs` ↔ `api.ts` spezzati in due PR con uno stub in mezzo.
7. **Ragionamento alto per compiti di segreteria.**

## 2. Regole per i token

Valgono per ogni sessione Claude del progetto. Vanno in `CLAUDE.md` (§9) perché si applichino da sole.

### 2.1 Una sessione, un compito, un contesto corto
- **Una sessione per compito**, creata da `main` con un brief di dieci righe, chiusa e archiviata quando la PR è unita. Mai un secondo compito nella stessa sessione.
- **Soglia di contesto: 200k token.** Oltre, la sessione scrive il passaggio di consegne (`docs/plans/<tema>.md`: stato, numeri, prossimo passo), pusha e chiede di essere sostituita. Non si aspetta la compattazione automatica.
- **Il brief è il contesto.** Compito, file da toccare, comandi di verifica, criterio di accettazione, documento di piano da leggere, modello consigliato. La storia sta nei documenti, non nel prompt.
- **Letture mirate.** `Read` con `offset`/`limit`; `grep` prima di `cat`; mai `docs/PIANO.md` per intero (1 400 righe): si legge `docs/STATO.md` e il piano del tema. L'output dei comandi lunghi va in un file e si legge con `tail` (`pnpm build 2>&1 | tail -5`).
- **Le letture pesanti stanno in un agente subordinato.** Esplorare dieci file per trovare una funzione, leggere un log, confrontare due versioni: lo fa un `Explore` con `model: "haiku"`, che restituisce la conclusione. Il contesto della sessione principale resta corto. È la leva più efficace dopo la durata della sessione.
- **Niente transcript altrui.** `list_events` vietato salvo per diagnosticare una sessione bloccata. `list_sessions` al massimo una volta per turno utente, e solo a richiesta.
- **Immagini con parsimonia.** Uno screenshot costa più di mille token: si guarda quello che serve (una pagina, un componente), non la galleria intera a ogni modifica.

### 2.2 I calcoli lunghi non tengono sveglio nessuno
- Un calcolo oltre i dieci minuti (banco, sweep, oracolo, `paired_test`) parte con `run_in_background` e un timeout generoso, e **la sessione finisce il turno**. Il risultato arriva come notifica: un solo risveglio.
- Niente `Monitor` con `while true` sui log; niente `sleep`; niente «controllo a che punto è».
- Oltre le due ore si va su GitHub Actions (`bench.yml`, a mano) o sul computer dell'utente; la sessione si chiude e la riapre il risultato.
- Ogni script lungo scrive un **riassunto di dieci righe** (`out/<nome>/summary.txt`). La sessione legge il riassunto, non il log.

### 2.3 Svegliarsi solo per eventi
- Nessun trigger orario. Risvegli ammessi: messaggio dell'utente, fine di un calcolo, evento GitHub sulla PR a cui si è iscritti, messaggio di un'altra sessione per una PR pronta o un blocco.
- Una iscrizione agli eventi per PR, tolta al merge (automatico).
- Prima di scrivere a una sessione con contesto grande, chiedersi se serve: ogni messaggio le costa un turno intero.

### 2.4 Il modello giusto per il compito
| Compito | Modello | Ragionamento |
|---|---|---|
| Architettura, bug difficili del motore, algoritmi di ricerca, decisioni di misura, contratto `lib.rs` ↔ `api.ts`, la macchina a stati di `Combo.tsx` e dei caricamenti asincroni | **Opus 5.5** | alto |
| Componenti e pagine dell'interfaccia, Tailwind, test Vitest e Playwright, script Python, documenti, revisione di una PR, CLI | **Sonnet 5.5** | medio |
| Coordinamento (PR, merge, `coordinamento.md`), correzioni di lint, stringhe in italiano, rinomine | **Sonnet 5.5** o Haiku 5.5 | basso |
| Agenti subordinati di ricerca nel codice, riassunti di log, confronto di screenshot | **Haiku 5.5** | basso |
| Primo sistema di design e schermate chiave in Claude Design | Opus 5.5 | medio |
| Varianti di un mockup esistente | Sonnet 5.5 | basso |
| Ragionamento profondo su un problema delimitato, con contesto sotto 100k | Fable 5.1 | alto |

- Il modello si imposta alla creazione (`create_session`, parametro `model`) o con `/model`; il livello con `/effort`. Fasi di un compito: pianificare ad alto, implementare a medio, rifinire a basso.
- Fable 5.1 **mai** per letture lunghe, cicli prova-correggi o sessioni di ore.
- I subagenti ricevono sempre `model: "haiku"` o `"sonnet"`: altrimenti ereditano Opus.
- `/fast` per scrivere documenti già decisi o codice meccanico.

### 2.5 Quota del piano Max
Due finestre, 5 ore e 7 giorni. Il 23/10 quella settimanale era in «avviso» alle 14.
- Al massimo **tre sessioni in parallelo**. Dieci corsie in un giorno hanno consumato in otto ore quello che tre consumano in una settimana, soprattutto aspettando.
- Il lavoro che consuma CPU (banchi, sweep) va in coda su una sessione o su Actions.
- **Il merge dalla pagina GitHub costa zero token.** Quando la CI è verde e la revisione è fatta, l'utente preme «Squash and merge» dal telefono: nessuna sessione deve svegliarsi. La coordinatrice fa il merge solo quando l'utente lo delega esplicitamente.
- A fine giornata, un `list_sessions` e tre righe in `coordinamento.md`: spesa per sessione e contesto. Una sessione sopra i 250k si chiude il giorno dopo con un passaggio di consegne.

## 3. Il frontend torna su Claude: la filiera

Oggi l'interfaccia è in `apps/desktop/src/` (React 19, TypeScript, Tailwind 4, token in `src/index.css`, 1 600 righe, due temi; componenti base del passo 0: `Toolbar`, `FilterChip`, `IconButton`, `SidePanel`, `HelpPopover`; `mock.ts` da 3 200 righe; Vitest; `scripts/screens.mjs` con Playwright e axe). Con Claude si lavora in quattro passi, ognuno nel suo strumento.

### 3.1 Il sistema di design di DuelForge (una volta)
- Si costruisce **dal codice**: una sessione Code (Opus, poi Sonnet) crea una libreria di anteprime in `apps/desktop/design/` (un HTML per componente o gruppo: colori e tipografia dai token, pulsanti, chip, barre, pannelli, schede, la scatola 3D, nei due temi), ognuna con il marcatore `<!-- @dsCard group="…" -->`, e con `/design-sync` la carica in un progetto «Sistema di design DuelForge» su claude.ai. Da lì ogni mockup in Claude Design usa i colori, i font (Inter, Cinzel) e i componenti veri, non un'invenzione.
- Si aggiorna **per differenza**, componente per componente, quando i token o i componenti base cambiano: la skill lo fa in modo incrementale, mai con una sostituzione in blocco.
- L'anteprima di ogni componente serve anche agli screenshot (§6) e come «galleria» di sviluppo (§3.3): un lavoro, tre usi.

### 3.2 Il mockup (per ogni schermata nuova o rifatta)
- Nel **Progetto Claude** (§5) si chiede il mockup: un artifact «Design» con artboard a 1280×800 per i due temi, costruito sul sistema di design. Si rivede sul canvas, anche dal telefono; i commenti sull'artifact restano attaccati al punto commentato.
- Si fa per: i passi 5 e 6 di `ui-revisione-2.md`, la scheda del punteggio con il meta e le board `cheap`, gli obiettivi ricchi (B8), il simulatore (#36). Non per i ritocchi (una scritta, un'icona): quelli vanno diretti al codice.
- Un mockup costa una chat nel Progetto, senza repository né strumenti: una frazione di una sessione Code. Cambiare idea sul canvas costa meno che cambiarla nel codice.

### 3.3 Il codice (una sessione Code per schermata)
- Brief: il link all'artifact del mockup, i file da toccare, il contratto `api.ts` (se cambia, la stessa PR tocca `lib.rs` e `mock.ts`), i criteri. La sessione legge il mockup con `Artifact read` e lo traduce nei componenti esistenti; i componenti nuovi entrano nella libreria e nel sistema di design.
- **Si verifica da sola, con gli occhi**: `VITE_MOCK=1 pnpm dev` nel container, `scripts/screens.mjs` con il Chromium preinstallato, e la sessione guarda gli screenshot (uno per pagina e tema) confrontandoli con il mockup, prima di dichiarare la PR pronta. È il punto che con Gemini mancava: nessuna PR «verificata solo con typecheck».
- Una pagina `/galleria`, solo in modo mock, mostra ogni componente in tutti gli stati (vuoto, caricamento, errore, dati): è lo Storybook di DuelForge senza Storybook, e la sorgente delle anteprime del sistema di design.
- Protocollo dell'interfaccia di `AGENTS.md` (id di richiesta, azzeramento al cambio deck, errori in italiano, dipendenze degli effetti, niente `any`) invariato: ora lo rispetta chi scrive e chi rivede, nella stessa sessione, e la revisione finale la fa una sessione Sonnet diversa solo per le PR grandi.
- Modelli: Sonnet per componenti e pagine; Opus per lo stato di `Combo.tsx`, i caricamenti asincroni, il contratto; Haiku per lint e stringhe.

### 3.4 La prova nell'app vera e il merge
- Anteprima Artifact per l'utente (§6.1), screenshot nella PR tramite gli artifact della CI (§6.2), prova nell'app Tauri sul computer per le PR che toccano pagine intere, 3D o suoni (§6.4).
- Merge dalla pagina GitHub.

### 3.5 Perché costa meno di Gemini
Nessun prompt da preparare e incollare; nessuna revisione incrociata obbligatoria; contratto in una PR sola, senza stub provvisori; nessuna PR unita «non provata nell'app vera»; i mockup si rivedono prima di scrivere codice, quindi meno giri di correzione; un solo insieme di regole (`AGENTS.md` e `CLAUDE.md` tornano a coincidere).

### 3.6 Uscita da Gemini
- Il passo 5 (Bottega) è in corso su `gemini/ui-shelf`: si lascia finire, lo rivede una sessione Sonnet con screenshot, si unisce. Poi Gemini si ferma: il passo 6 va a Claude con la filiera di §3.
- I 26 branch `gemini/*` si puliscono: quelli uniti si cancellano; `gemini/workstation-bridge` (non unito) si rivede una volta e si decide se recuperarlo o chiuderlo.
- `AGENTS.md` perde la sezione sulle aree di Gemini e le due sezioni fisse dei prompt; resta il protocollo dell'interfaccia.

## 4. Gli strumenti e il loro ruolo

### 4.1 Claude Code sul web (claude.ai/code): motore, backend, frontend
- Container Linux con Rust, Node, pnpm, Chromium per Playwright, Xvfb, repository, CI via GitHub, messaggi tra sessioni.
- Vi si fa tutto il codice: `crates/`, `src-tauri/`, `apps/desktop/src/`, `scripts/`. Il frontend si prova nel browser headless con il backend finto; il backend dell'app con `cargo check -p duelforge-desktop` e la CLI.
- **App Tauri sotto Xvfb: provato il 23/10, funziona.** WebKitGTK si installa con apt (`scripts/cloud-setup.sh`); `scripts/cloud-app.sh <png>` lancia `pnpm tauri dev` senza finestra e scatta lo screenshot (prima build debug ~8 minuti, poi pochi secondi; in background). Provato con `VITE_MOCK=1`; con il motore vero serve `data/` (`cloud-setup.sh --data`). Non sostituisce la prova sul computer per suoni, prestazioni e 3D con la GPU (qui rendering software).
- L'ambiente va sistemato una volta: script di avvio con `pnpm install` e `cargo fetch`; `fetch-data` solo quando serve (le sessioni di frontend non ne hanno bisogno); variabili `DUELFORGE_DATA`, `VITE_MOCK`; rete verso Master Duel Meta, YGOPRODeck, crates.io, npm, ambientCG/Poly Haven per le texture.

### 4.2 Claude Code sul computer (app desktop su Windows): l'app vera
- L'unico posto con `pnpm tauri dev`, la finestra, le texture, i suoni, il 3D. Una sessione locale può lanciare l'app, guardarla (uso del computer: screenshot della finestra) e correggere con ricarica a caldo. Costa gli stessi token di una sessione cloud, ma vede l'app vera.
- Con **Remote Control** una sessione cloud usa il computer: `git checkout` del branch, `pnpm tauri dev`, screenshot. Da attivare una volta.
- Il repository locale su `main` aggiornato, con `data/` scaricato: provare una PR è `git fetch && git checkout <branch> && pnpm tauri dev`.

### 4.3 Progetto Claude (claude.ai): lo studio
Vedi §5.

### 4.4 Claude Design (dentro Claude): mockup e sistema di design
Vedi §3.1 e §3.2. Il sistema di design è un progetto su claude.ai alimentato da `/design-sync`; i mockup sono artifact «Design» del Progetto. Non si fanno mockup fuori dal sistema di design: altrimenti il codice non torna.

### 4.5 Artifact: anteprime per l'utente
Vedi §6.1.

### 4.6 GitHub
- **Branch protection** su `main` (c'è): PR con CI verde. Nessuna approvazione obbligatoria: il merge lo fa l'utente dalla pagina.
- **Template di PR** (`.github/pull_request_template.md`): «Cosa cambia», «Numeri prima/dopo» (motore), «Verifica» con la lista del protocollo dell'interfaccia e il link agli screenshot (interfaccia), «Attività in pausa toccate».
- **Actions**: budget contato. Restano le regole di `ci.yml`. Aggiunte al job frontend: `pnpm screens` con gli screenshot e l'audit axe come artifact del run (30 secondi), e una prova Playwright «fumo» sul mock (apre ogni pagina nei due temi, nessun errore in console). I banchi lunghi su `bench.yml`.
- **Issues** per richieste e bug (un'issue per tema, chiusa dalla PR con `Closes #n`). `TASKS.md` resta per lock e stato operativo.
- **Niente Projects di GitHub, niente Dependabot di routine** (solo vulnerabilità): ogni PR automatica costa una CI e un turno di revisione.
- **Release**: tag `v0.x` quando `main` passa l'installer; l'installer si scarica dall'Actions run.
- **Pulizia dei branch**: dopo il merge, cancellare il branch (impostazione «automatically delete head branches»).

## 5. Il ruolo del Progetto Claude

Con il frontend su Claude il Progetto diventa lo **studio** del prodotto: il posto dove si decide, si disegna e si guarda, senza aprire sessioni Code.

- **Conoscenza**: `docs/STATO.md`, `AGENTS.md`, questo piano, `coordinamento.md`, un glossario (board, kit, turno di prova, oracolo, A1b, banco di generalità, misura a gradini), gli screenshot correnti delle sei pagine nei due temi (rinnovati a ogni release). Si aggiorna quando `main` cambia davvero, non a ogni PR.
- **Istruzioni**: lingua e convenzioni di `AGENTS.md`, i principi non negoziabili (motore generale, niente linee precalcolate, prova equa sulle 406 board), «ogni decisione finisce in `docs/plans/`», «i mockup si fanno sul sistema di design DuelForge».
- **Cosa ci si fa**:
  1. le **decisioni** (opzioni della misura, priorità del giro, cosa tenere e cosa no) con i numeri delle PR davanti;
  2. i **mockup** in Claude Design e la loro revisione, dal telefono, con i commenti;
  3. i **brief** delle sessioni Code, scritti con il modello di §9.4: dieci righe, poi la sessione nasce dal Progetto e ne eredita istruzioni e memoria (è la funzione proposta dal popup «Continua il lavoro sulle pull request tra le sessioni»);
  4. gli **avvisi**: il Progetto segnala quando una PR finisce o serve l'utente, al posto della coordinatrice sempre sveglia;
  5. i **riepiloghi** di fine giro, letti dai documenti su `main`.
- **Cosa non ci si fa**: codice. Una chat del Progetto non ha repository né strumenti; se serve toccare un file, nasce una sessione Code con il brief.
- Costo: una chat del Progetto ha un contesto piccolo e stabile (le istruzioni e la conoscenza sono in cache); una decisione discussa lì costa una frazione della stessa discussione in una sessione Code con 200k di storia.

Con questo, la coordinatrice permanente non serve più: le sessioni sono poche (al massimo tre), nascono dal Progetto, aprono la propria PR e si iscrivono ai suoi eventi; l'utente unisce dalla pagina GitHub o lo delega. Una sessione Sonnet di coordinamento si apre solo nei giorni con molte PR in coda, e si chiude la sera.

## 6. Vedere l'app in sviluppo

Dal più immediato al più completo. Le opzioni 1-3 non richiedono niente sul computer.

1. **Anteprima Artifact del frontend** (per l'utente, dal telefono). La sessione fa `VITE_MOCK=1 pnpm build` e pubblica `dist/` come artifact a più file, **sempre allo stesso URL** (si aggiorna in posto), privato. Mostra layout, temi, testi, flussi, animazioni con i dati di `mock.ts`. Per renderla realistica, il backend finto si alimenta con **risposte registrate** dal backend vero: un comando della CLI (`duelforge-cli record-fixtures --deck <deck>`) salva in `apps/desktop/src/lib/fixtures/` le risposte JSON di un deck di esempio (ricerca, punteggi, meta, collegamenti, banlist), e `mock.ts` le serve invece di inventarle. Un minuto dopo ogni passo utile. Limiti: niente motore dal vivo, niente 3D pesante oltre i 16 MB (font e texture fuori dall'anteprima o sotto i limiti).
2. **Screenshot della sessione** (per la sessione e per l'utente, subito). `scripts/screens.mjs` nel container: la sessione li guarda per verificarsi, e manda all'utente quelli che contano con un file in chat. In CI gli stessi screenshot diventano artifact del run, visibili dalla PR.
3. **Browser con il motore vero, nel cloud** (fase 2, se il mock registrato non basta). Un «ponte di sviluppo»: un piccolo binario Rust che espone gli stessi comandi di `lib.rs` su HTTP locale, e `api.ts` che con `VITE_BRIDGE=1` chiama il ponte invece di `invoke`. Permette a Playwright di provare l'interfaccia con il motore vero e i dati veri, nel container e in CI. Richiede di spostare i comandi di `lib.rs` in un crate condiviso. L'utente non vede le porte del container: serve alla sessione, non ai suoi occhi.
4. **App Tauri vera**: sul computer con `pnpm tauri dev` e ricarica a caldo, da una sessione Claude Code desktop (che la guarda) o da una sessione cloud via Remote Control. Obbligatoria per le PR che toccano pagine intere, la Bottega 3D, i suoni, le prestazioni del motore dall'app.
5. **App Tauri sotto Xvfb nel cloud** (provata il 23/10, funziona): `scripts/cloud-app.sh` lancia `pnpm tauri dev` headless con screenshot dell'app vera senza il computer. Sostituisce il punto 4 per tutto tranne suoni e prestazioni.
6. **Installer da `main`**, una volta a settimana: la prova completa.

Regola: 1 e 2 per ogni PR dell'interfaccia; 4 per i passi grandi; 6 settimanale. Il 3 si fa quando il primo bug «solo con il motore vero» costa più del ponte.

## 7. Backend, frontend e ambiente: dove si fa cosa

| Parte | Dove | Come si prova | Modello |
|---|---|---|---|
| Motore, dati, statistiche, CLI | Code web | `cargo test --release`, CLI, `paired_test.py`, prova equa, banco di generalità; banchi lunghi su Actions | Opus (algoritmi), Sonnet (script, documenti) |
| Backend dell'app (`lib.rs`) | Code web | `cargo check -p duelforge-desktop`, clippy; la funzione dalla CLI che la copre | Opus per il contratto, Sonnet per il resto |
| Contratto `lib.rs` ↔ `api.ts` ↔ `mock.ts` | Una PR sola | `cargo check` più `pnpm typecheck`; fixture registrate aggiornate | Opus |
| Interfaccia | Code web, mockup prima in Claude Design | Screenshot propri, Vitest, Playwright fumo, anteprima Artifact; app vera sul computer per i passi grandi | Sonnet; Opus per stato e asincronia |
| Sistema di design | Code web con `/design-sync` | La galleria e le anteprime | Opus la prima volta, poi Sonnet |
| Decisioni, mockup, brief | Progetto Claude | Documento in `docs/plans/` | Opus per le decisioni, Sonnet per le varianti |
| Prova finale, installer | Computer Windows, Actions | Installer da `main` | utente |

Regole del contratto: un campo nuovo nasce in una PR che tocca `lib.rs`, `api.ts`, `mock.ts` e le fixture insieme; il backend non manda dati che l'interfaccia non mostra (l'interruttore `SEND_CHEAP_SCORES` si accende nella stessa PR che fa la scheda).

## 8. Il ciclo di un compito

1. **Decisione** nel Progetto o in `docs/plans/<tema>.md`: cosa, perché, criterio, numeri attesi.
2. **Mockup** in Claude Design, se la schermata è nuova o rifatta; revisione con commenti.
3. **Issue** su GitHub con il riassunto e i link (piano, mockup).
4. **Brief** di dieci righe nel Progetto; la sessione Code nasce da lì con il modello giusto.
5. **Lavoro** in una sessione sola: commit piccoli che compilano, push dopo ogni passo, calcoli lunghi in background con il turno chiuso, screenshot propri prima di dire «pronto».
6. **PR** dal template, aperta dalla sessione; CI all'apertura; un solo rilancio a mano dopo l'ultimo push, se ce n'è stato uno.
7. **Revisione**: per le PR piccole basta la sessione stessa con la lista del template; per quelle grandi una sessione Sonnet separata con brief corto (diff, lista, screenshot).
8. **Merge** dalla pagina GitHub da parte dell'utente (o delegato), squash, branch cancellato.
9. **Chiusura**: `TASKS.md` aggiornato, `STATO.md` se cambiano decisioni, sessione archiviata.

## 9. Le regole stabili, nei file giusti

### 9.1 `AGENTS.md` (c'è)
Resta la carta del progetto: mappa, comandi, convenzioni, protocolli, cosa non fare. Cambia:
- via le aree di Gemini, i branch `gemini/*`, le due sezioni fisse dei prompt, «la PR di un agente la rivede l'altro»;
- «Coordinamento tra agenti» diventa «Coordinamento tra sessioni»: una sessione per compito, lock in `TASKS.md`, file caldi, soglia 200k, passaggio di consegne;
- nel protocollo dell'interfaccia: la verifica con screenshot propri e l'anteprima Artifact come passi obbligatori; «se l'app vera non si può provare, la PR lo dice» resta.

### 9.2 `CLAUDE.md` (c'è, cinque righe)
Solo per Claude Code: tabella dei modelli e livelli (§2.4); risvegli (§2.3); subagenti Haiku per le letture pesanti; `list_events` vietato; brief standard; passaggio di consegne; file caldi; «il merge lo fa l'utente dalla pagina, salvo delega».

### 9.3 `.claude/settings.json` (fatto)
Permessi senza conferma: `cargo fmt/clippy/test/check`, `pnpm lint/typecheck/test/build/dev/screens`, `git add/commit/push` sul proprio branch, `python3 scripts/*.py`, `node scripts/screens.mjs`. Il 23/10 quattro blocchi su commit e push di documenti sono costati quattro turni e quattro messaggi. Vietati: `git push --force`, `git rebase` su branch altrui, scritture in `vendor/`.

### 9.4 `.claude/skills/` (fatto)
- **`brief`**: il modello del brief (compito, file, comandi, criterio, piano, mockup, modello).
- **`pr-motore`**: il protocollo del motore con i comandi esatti (`mdm_decks.py`, `bench.py`, `paired_test.py`, `a1b_fair_measure.py`, `generality.py`) e la tabella numeri prima/dopo.
- **`pr-interfaccia`**: la lista del protocollo dell'interfaccia, `VITE_MOCK=1 pnpm dev` più `screens.mjs`, come confrontare con il mockup, come pubblicare l'anteprima Artifact allo stesso URL, cosa scrivere nel template.
- **`design-to-code`**: come leggere un artifact Design, mappare i componenti del sistema di design a quelli di `src/components/`, aggiungere un componente nuovo alla galleria e a `/design-sync`.
- **`steward`** (letta da chi guida una PR): convenzioni di merge e CI, quando rilanciare la CI (una volta, dopo l'ultimo push), niente commit vuoti.
- **`coordinamento`**: per i giorni con una coordinatrice: PR, iscrizione, merge con squash e `expectedHeadSha` completo, `coordinamento.md`, passaggio di consegne.

### 9.5 `.github/`
`pull_request_template.md`; in `ci.yml` gli screenshot e la prova fumo Playwright sul job frontend; cancellazione automatica dei branch uniti.

### 9.6 Il Progetto Claude
Istruzioni e conoscenza come in §5.

### 9.7 `docs/plans/coordinamento.md`
Resta la memoria operativa nei giorni con coordinatrice; altrimenti si aggiorna a fine giro con le sessioni, i costi e le decisioni.

## 10. Cosa fare adesso, in ordine
1. **Chiudere il giro**: PR dell'A1b (corsia 6), merge, archiviazione; `coordinamento.md` e questo piano su `main`.
2. **Regole** (§9.1-9.5) in una PR di documenti e configurazione: mezza giornata di una sessione Sonnet.
3. ~~**Ambiente cloud**~~ fatto il 23/10: `scripts/cloud-setup.sh`, variabili in `.cloud-env`, prova di WebKitGTK sotto Xvfb riuscita (§6.5). Da impostare nell'ambiente su claude.ai: come setup script `scripts/cloud-setup.sh`.
4. **Progetto Claude**: conoscenza, istruzioni, collegamento con Code; Remote Control sul computer.
5. **Fixture registrate e anteprima**: ~~il comando `record-fixtures`, `mock.ts` che le serve, la prima anteprima Artifact~~ fatto il 23/10 nella struttura: `record-fixtures` scrive `fixtures/recorded.json` (ricerche, faccette, banlist, deck; niente testi), `mock.ts` lo serve se presente, anteprima pubblicata con `vite build --base ./` (percorsi relativi, serve per l'Artifact). Resta da registrare i dati veri dove `update-cards` funziona (nel cloud ygoprodeck è bloccata), e da aggiungere punteggi e probabilità.
6. **Sistema di design**: la galleria `/galleria`, le anteprime in `apps/desktop/design/`, `/design-sync` (Opus la prima volta). *24/10: galleria, anteprime e script `pnpm design` nella PR del thread «Costruire il sistema di design»; il primo `/design-sync` lo lancia l'utente (`docs/plans/sistema-di-design.md`). Il §9 è fatto, salvo gli screenshot in CI del §9.5, in coda in `TASKS.md`.*
7. **Passo 5 di Gemini**: revisione con screenshot e merge; poi Gemini si ferma e i branch si puliscono.
8. **Primo compito con la filiera completa**: il passo 6 (effetti e suoni, ognuno spegnibile): mockup delle Impostazioni in Claude Design, sessione Sonnet, screenshot, anteprima, prova sul computer. Serve a collaudare la filiera su un compito piccolo prima di quelli grandi.
9. **Poi, uno per sessione, al massimo tre in parallelo**:
   - i due pesi del punteggio proposti dalla §7 (resilienza 0,6 → 0, floodgate 0,8 → 0,4): conferma sul banco di generalità, poi una PR che cambia `bench.py`, CLI, `lib.rs`, `scoring.ts` insieme;
   - [fatto] le board `cheap: true`: scheda e interruttore `SEND_CHEAP_SCORES` nella stessa PR;
   - B8, obiettivi ricchi: mockup, poi contratto e interfaccia (Opus);
   - i punti 3, 5 e 6 della caccia ai bug, l'arena nel turno di prova, il caso Vendread (Opus);
   - il simulatore (#36): mockup prima.

## 11. Come si misura se funziona
A fine settimana, in `coordinamento.md`:
- spesa per PR unita (23/10: circa 85 $ contando tutte le sessioni); obiettivo sotto 30 $;
- contesto massimo delle sessioni chiuse; obiettivo: nessuna sopra 250k;
- risvegli a vuoto; obiettivo zero;
- PR dell'interfaccia con screenshot propri nella PR; obiettivo: tutte;
- PR dell'interfaccia provate nell'app vera prima del merge; obiettivo: tutte quelle che toccano una pagina intera;
- minuti di Actions al mese, sotto il budget del repository.
