# DuelForge (nome provvisorio)

Suite desktop per Yu-Gi-Oh!: deck builder con carte e banlist sempre aggiornate, scaffale di deck box 3D personalizzabili, laboratorio statistico (calcolo esatto e Monte Carlo) e Combo Lab, un ricercatore di combo basato sul motore open source di EDOPro.

Il piano di progetto completo è in [`docs/PIANO.md`](docs/PIANO.md).

**Stato**: Fase 1 completata. L'app scarica da sola carte e dati del motore e offre la Fucina: catalogo di tutte le carte con ricerca avanzata, filtri, visualizzatore con zoom e lente, effetti visivi e sonori.

## Struttura

```
apps/desktop/          app desktop: React + TypeScript (src/) e Tauri 2 (src-tauri/)
crates/ygo-engine/     collegamento Rust al motore EDOPro: partite, carte, script, deck .ydk
crates/ygo-data/       database delle carte (SQLite + ricerca full-text), linguaggio di ricerca, immagini, download
crates/ygo-cli/        duelforge-cli: scarica i dati, gioca mani, benchmark
vendor/ygopro-core/    motore EDOPro (submodule git, AGPL-3.0)
tests/fixtures/decks/  deck di prova (Invoked di Siracusa, versione Master Duel)
docs/                  piano di progetto
data/                  dati scaricati (carte, script, database del motore, immagini), non versionati
```

## Provare l'app senza installare nulla

A ogni push, GitHub Actions compila l'installer Windows:

1. apri la scheda **Actions** del repository e scegli l'ultima esecuzione di **CI** riuscita;
2. in fondo alla pagina, sotto **Artifacts**, scarica `DuelForge-windows-…`;
3. estrai lo zip ed esegui il file `.exe`.

L'installer non è firmato, quindi Windows mostrerà l'avviso SmartScreen: scegli "Ulteriori informazioni" → "Esegui comunque".

## Sviluppo su Windows

Strumenti da installare una volta:

- [Git](https://git-scm.com/download/win)
- [Rust](https://rustup.rs) (toolchain stable)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) con il carico di lavoro "Sviluppo di applicazioni desktop con C++" (serve per compilare il motore)
- [Node.js 22 LTS](https://nodejs.org) e pnpm (`npm install -g pnpm`)
- [VS Code](https://code.visualstudio.com) con le estensioni rust-analyzer, Tauri, ESLint

Prima volta:

```powershell
git clone --recurse-submodules https://github.com/Vale-P-95/ygo.git
cd ygo
cargo run --release -p ygo-cli -- fetch-data      # scarica script e database delle carte in data\
cd apps\desktop
pnpm install
```

Avviare l'app in sviluppo (le modifiche all'interfaccia si vedono subito):

```powershell
$env:DUELFORGE_DATA = "$PWD\..\..\data"   # usa i dati scaricati sopra
pnpm tauri dev
```

Senza `DUELFORGE_DATA`, l'app cerca i dati nella sua cartella dati (`%APPDATA%\app.duelforge.desktop\data`).

Test e controlli:

```powershell
cargo test -p ygo-engine -p ygo-cli
cargo clippy -p ygo-engine -p ygo-cli --all-targets
cd apps\desktop; pnpm typecheck
```

## CLI

```bash
# Gioca una mano passo per passo (interattivo)
cargo run --release -p ygo-cli -- play --deck tests/fixtures/decks/invoked-siracusa-md.ydk \
  --hand "Aleister the Invoker;76334960;Magical Meltdown;Ash Blossom & Joyous Spring;Mulcharmy Fuwalos"

# Segui una linea: ogni passo viene cercato nel testo delle azioni disponibili
cargo run --release -p ygo-cli -- play --deck tests/fixtures/decks/invoked-siracusa-md.ydk \
  --hand "Aleister the Invoker;76334960;Magical Meltdown;Ash Blossom & Joyous Spring;Mulcharmy Fuwalos" \
  --steps 'activate magical meltdown;yes;aleister the invoker (deck);normal summon aleister the invoker;yes;select invocation (deck)'

# Misura le prestazioni del motore
cargo run --release -p ygo-cli -- bench --deck tests/fixtures/decks/invoked-siracusa-md.ydk --hand "..."
```

Le carte della mano si indicano per nome inglese (come nel database di EDOPro) o per passcode.

```bash
# Scarica l'elenco delle carte da YGOPRODeck in data/cards.db
cargo run --release -p ygo-cli -- update-cards

# Cerca con lo stesso linguaggio dell'app
cargo run --release -p ygo-cli -- search 'arch:invoked type:fusion sort:atk'
```

### Linguaggio di ricerca

| Esempio | Significato |
|---|---|
| `aleister` | parole nel nome o nel testo (anche iniziali) |
| `"negate the activation"` | frase esatta |
| `name:invoked` | solo nel nome |
| `type:fusion`, `tipo:magia`, `type:quick-play` | tipo o sottotipo |
| `attr:dark`, `race:spellcaster`, `arch:invoked` | attributo, tipo di mostro, archetipo |
| `atk>=2500`, `def<1000`, `lv:4-6`, `link>=2` | statistiche |
| `format:md`, `rarity:ur`, `ban:limited` | Master Duel, rarità MD, banlist TCG |
| `sort:atk`, `sort:popular`, `sort:new` | ordinamento |

Ogni filtro scelto nel pannello dell'app diventa testo nella barra di ricerca, e viceversa.

## Licenza

Progetto open source con licenza [GNU AGPL-3.0-or-later](LICENSE), la stessa del motore e degli script di EDOPro.

Progetto non ufficiale, non affiliato né approvato da Konami. Yu-Gi-Oh! e i relativi nomi, testi e immagini delle carte sono di proprietà dei rispettivi titolari.
