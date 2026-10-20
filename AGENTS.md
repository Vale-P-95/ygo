# Regole per gli agenti (Claude Code, Antigravity/Gemini, altri)

DuelForge è una suite desktop per Yu-Gi-Oh!: deck builder, scaffale di deck box 3D, laboratorio statistico e Combo Lab, un ricercatore di combo che usa il motore di EDOPro.
Lo stack è un workspace Rust (motore, dati, statistiche, ricerca, CLI) più un'app Tauri 2 con React 19, TypeScript e Tailwind 4. La licenza è AGPL-3.0.
Il progetto completo e le decisioni sono in `docs/PIANO.md`; lo stato attuale e i prossimi passi in `docs/STATO.md`; i lavori in corso in `docs/TASKS.md`.

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

## Cosa NON fare
- Modificare `vendor/ygopro-core`.
- Committare `data/`, immagini o testi Konami, o le decklist scaricate.
- Force push, rebase o commit sul branch di un altro agente.
- Cambiare i tipi condivisi tra backend e interfaccia (`apps/desktop/src-tauri/src/lib.rs` ↔ `apps/desktop/src/lib/api.ts`) senza aggiornare entrambi nella stessa PR.
- Lanciare benchmark o build lunghe senza che l'utente lo chieda.
- Fare il merge in `main` senza la richiesta esplicita dell'utente.

## Coordinamento tra agenti
- **Aree**: Claude Code lavora su `crates/` e `apps/desktop/src-tauri/`; Antigravity/Gemini su `apps/desktop/src/`. Se all'interfaccia serve un dato nuovo dal backend, la richiesta va in `docs/TASKS.md` per Claude.
- **Branch**: `main` è protetto e si cambia solo con pull request e CI verde. Ogni agente usa i suoi branch: `claude/<tema>`, `gemini/<tema>`.
- **Inizio sessione**: `git pull`, poi leggere `docs/TASKS.md` e `git log -10`.
- **Prendere un task**: scriverlo in "In corso" con agente, branch e **file che toccherai** (è il lock), poi committare e pushare quella riga prima di cominciare.
- **File caldi** (un solo agente alla volta): `crates/ygo-solver/src/search.rs`, `probe.rs`, `value.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/lib/api.ts`.
- **Lavoro interrotto** (la quota può finire in qualsiasi momento): fare piccoli commit che compilano, pushare sul proprio branch dopo ogni passo funzionante e spuntare i sotto-passi in `docs/TASKS.md`.
- **Fine sessione**: aggiornare `docs/TASKS.md` (e `docs/STATO.md` se cambiano decisioni importanti), committare e pushare.
- **Revisione**: la PR di un agente la rivede l'altro. Il merge lo decide l'utente.
