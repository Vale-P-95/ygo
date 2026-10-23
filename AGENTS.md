# Regole per gli agenti (sessioni Claude Code; Antigravity/Gemini fino alla fine del passo 5 dell'interfaccia)

DuelForge è una suite desktop per Yu-Gi-Oh!: deck builder, scaffale di deck box 3D, laboratorio statistico e Combo Lab, un ricercatore di combo che usa il motore di EDOPro.
Lo stack è un workspace Rust (motore, dati, statistiche, ricerca, CLI) più un'app Tauri 2 con React 19, TypeScript e Tailwind 4. La licenza è AGPL-3.0.
Il progetto completo e le decisioni sono in `docs/PIANO.md`; lo stato attuale e i prossimi passi in `docs/STATO.md`; i lavori in corso in `docs/TASKS.md`; come si lavora (strumenti, sessioni, token, come si vede l'app) in `docs/plans/piano-ambiente-e-risorse.md`.

## Mappa
```
crates/ygo-engine/   collegamento al motore EDOPro (C++, via FFI): partite, decisioni, azioni, stato
crates/ygo-data/     database delle carte, ricerca, immagini, banlist, dati utente
crates/ygo-stats/    probabilità esatte, Monte Carlo, ottimizzatore
crates/ygo-solver/   Combo Lab: ricerca (search.rs), turno di prova (probe.rs), valori delle carte (value.rs), ruoli, copertura
crates/ygo-cli/      duelforge-cli: dati, combos, coverage, card-values, roles, sweep
apps/desktop/src-tauri/  backend dell'app: comandi Tauri, tipi esposti al frontend (lib.rs)
apps/desktop/src/    interfaccia React: pages/, components/, lib/ (api.ts = contratto col backend)
vendor/ygopro-core/  motore EDOPro (submodule git): non si modifica
scripts/             bench.py, coverage_all.py, combo_hunt.py, mdm_decks.py
tests/fixtures/      deck e mani di prova
data/                carte, script e immagini scaricati: mai committati
```

## Comandi
Prima volta (Windows PowerShell; su Linux è uguale, con `export` al posto di `$env:`):
```powershell
git clone --recurse-submodules https://github.com/Vale-P-95/ygo.git
cd ygo
cargo run --release -p ygo-cli -- fetch-data     # script e database del motore in data/ (~1-2 GB)
cargo run --release -p ygo-cli -- update-cards   # elenco carte per la ricerca (data/cards.db)
cd apps/desktop; pnpm install
```
Su Windows servono Git, Rust, Node 22 + pnpm e Visual Studio Build Tools con "Sviluppo di applicazioni desktop con C++". La prima build release richiede diversi minuti.

App in sviluppo: `cd apps/desktop; $env:DUELFORGE_DATA = "<repo>\data"; pnpm tauri dev`.

Controlli prima di ogni push (gli stessi della CI):
```
cargo fmt --all -- --check
cargo clippy -p ygo-engine -p ygo-data -p ygo-stats -p ygo-solver -p ygo-cli --all-targets -- -D warnings
cargo test -p ygo-engine -p ygo-data -p ygo-stats -p ygo-solver -p ygo-cli    # più veloci con --release
cargo test --release -p ygo-solver --test search_regression                 # regressioni della ricerca (~2 min)
cd apps/desktop; pnpm typecheck; pnpm build
```
La CI gira una volta per ogni pull request, solo con i controlli delle parti toccate (Rust, interfaccia), e tutta su `main` dopo il merge; l'installer Windows si compila solo su `main`. I minuti di GitHub Actions del repository privato sono contati.

## Convenzioni
- **Lingue**: l'interfaccia è in italiano; i nomi e i testi delle carte restano in inglese; il codice, i commenti e i messaggi di commit sono in inglese; i documenti (`docs/`, README) sono in italiano.
- **Commenti**: frasi brevi che spiegano il perché, come nel codice che c'è. Il codice nuovo imita lo stile dei file in cui sta.
- **Rust**: `rustfmt.toml` (righe fino a 140), clippy senza warning.
- **Frontend**: componenti funzionali, Tailwind, colori dai token in `src/index.css` (tema chiaro e scuro).
- **Commit**: oggetto in inglese all'imperativo, corpo che spiega cosa e perché. Ogni agente firma i suoi commit.
- **Motore e ricerca**: nessun caso speciale per nome di carta. Ogni correzione deve valere per qualunque deck.

## Protocollo per chi tocca motore, ricerca o turno di prova
Vale per `crates/ygo-engine` e `crates/ygo-solver`. Prima e dopo la modifica:
1. `python3 scripts/mdm_decks.py --out <cartella>`: scarica le liste di Master Duel Meta (non si committano).
2. `python3 scripts/bench.py <out> --decks <cartella> --nodes 4000`: confronta le tabelle prima e dopo.
3. `duelforge-cli coverage --deck <deck>` sui deck toccati, e `python3 scripts/combo_hunt.py <cartella>` come regressione.
4. I numeri prima/dopo vanno nella PR.

## Protocollo per chi tocca l'interfaccia
Vale per `apps/desktop/src`. Nasce dagli errori ricorrenti delle revisioni: PR dichiarate pronte con difetti che il backend finto nascondeva, regressioni introdotte correggendo, caricamenti che si accavallano.
1. **Con gli occhi, non solo con i tipi.**
   - Prima di dichiarare la PR pronta: `VITE_MOCK=1 pnpm dev` e `node scripts/screens.mjs` (Chromium di Playwright), poi si guardano gli screenshot delle pagine toccate nei due temi e si confrontano con il mockup, se c'è. Gli screenshot delle pagine toccate si allegano alla PR (artifact della CI o file in chat).
   - L'anteprima del frontend con il backend finto si pubblica come Artifact, sempre allo stesso URL, così l'utente la vede dal telefono.
   - Le funzioni che chiamano il backend si provano nell'app vera con `pnpm tauri dev` (con `DUELFORGE_DATA`) quando la PR tocca una pagina intera, la Bottega 3D, i suoni o le prestazioni del motore. Se l'app vera non si può provare, la PR lo dice.
   - `mock.ts` non controlla i formati: ad esempio un oggetto salvato in `localStorage` e letto da un'altra pagina. Dove esistono, si usano le risposte registrate dal backend vero (`src/lib/fixtures/`), non dati inventati.
2. **Dati con i tipi del contratto.**
   - Ciò che si passa tra pagine o si salva usa i tipi di `api.ts` (ad esempio `GoalCard`), non oggetti scritti a mano.
   - Niente `as any`, `@ts-ignore` o `@ts-expect-error` per far tornare i tipi. Un tipo che non torna è un errore, oppure manca qualcosa nel contratto: allora il contratto si cambia nella stessa PR (`lib.rs`, `api.ts`, `mock.ts` e le fixture insieme).
3. **Caricamenti asincroni.**
   - Ogni caricamento ha un id di richiesta; si scartano le risposte vecchie e quelle che non sono del deck o dell'oggetto mostrato.
   - Cambiando deck, salvando o ricominciando, i dati vecchi si azzerano e il caricamento in corso si annulla.
   - Un effetto che carica da solo si ferma su errore: niente ricaricamenti in loop.
4. **Errori sempre visibili.** Ogni errore arriva all'utente in italiano, con un messaggio o un riquadro. Mai un `catch` vuoto, né un `try/finally` senza `catch` su una chiamata al backend.
5. **Effetti React.**
   - Le dipendenze sono complete, e i listener e i timer si tolgono nella pulizia.
   - Lo stato legato a un deck, a una ricerca o a una selezione si azzera quando questa cambia: filtri nascosti compresi, ad esempio con `key`.
6. **Correggendo una revisione si rilegge tutto il diff.** Non si tolgono controlli esistenti (una condizione, un `!error`, un id di richiesta) senza scriverne il motivo nella PR.
7. **"Pronto per il merge" solo quando è vero.**
   - Nella PR, per ogni punto chiesto: come è stato verificato (app vera, screenshot, backend finto, solo `typecheck`).
   - Un task si segna fatto in `docs/TASKS.md` solo quando sono fatti tutti i suoi punti; quelli rimasti restano aperti, ben visibili.
8. **Prima del push:** `pnpm typecheck` e `pnpm build`; quando ci saranno, anche i controlli di stile della CI (`any` vietato, dipendenze degli effetti).

## Cosa NON fare
- Modificare `vendor/ygopro-core`.
- Committare `data/`, immagini o testi Konami, o le decklist scaricate.
- Force push, rebase o commit sul branch di un'altra sessione o di un altro agente.
- Cambiare i tipi condivisi tra backend e interfaccia (`apps/desktop/src-tauri/src/lib.rs` ↔ `apps/desktop/src/lib/api.ts`) senza aggiornare entrambi nella stessa PR.
- Lanciare benchmark o build lunghe senza che l'utente lo chieda.
- Fare il merge in `main` senza la richiesta esplicita dell'utente.

## Coordinamento tra sessioni
Le sessioni Claude Code fanno tutto il codice: motore, backend dell'app e interfaccia. Gemini (Antigravity) finisce il passo 5 dell'interfaccia su `gemini/ui-shelf` e poi si ferma; i suoi branch si puliscono.
- **Una sessione, un compito.** Ogni sessione nasce da `main` con un brief di dieci righe (compito, file, comandi di verifica, criterio di accettazione, piano e mockup da leggere), fa la sua PR e si chiude quando è unita. Un secondo compito è una sessione nuova. Al massimo tre sessioni in parallelo.
- **Contesto corto.** Oltre i 200k token la sessione scrive il passaggio di consegne in `docs/plans/<tema>.md` (stato, numeri, prossimo passo), pusha e chiede di essere sostituita.
- **Branch**: `main` è protetto e si cambia solo con pull request e CI verde. Ogni sessione usa il suo branch `claude/<tema>`.
- **Inizio sessione**: `git pull`, poi leggere `docs/TASKS.md` e `git log -10`.
- **Prendere un task**: scriverlo in "In corso" con sessione, branch e **file che toccherai** (è il lock), poi committare e pushare quella riga prima di cominciare.
- **File caldi** (una sola sessione alla volta): `crates/ygo-solver/src/search.rs`, `probe.rs`, `value.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/lib/api.ts`.
- **Calcoli lunghi** (oltre dieci minuti): partono in background e la sessione chiude il turno; il risultato arriva come notifica. Niente controlli ripetuti, niente monitor sui log. Oltre le due ore si usa `bench.yml` su GitHub Actions.
- **Lavoro interrotto** (la quota può finire in qualsiasi momento): fare piccoli commit che compilano, pushare sul proprio branch dopo ogni passo funzionante e spuntare i sotto-passi in `docs/TASKS.md`.
- **Fine sessione**: aggiornare `docs/TASKS.md` (e `docs/STATO.md` se cambiano decisioni importanti), committare e pushare.
- **PR e merge**: la PR la apre la sessione stessa, con il template. Le PR piccole le rivede la sessione stessa con la lista del template; quelle grandi una sessione separata con brief corto. Il merge lo fa l'utente dalla pagina GitHub (squash), o chi lui delega.
