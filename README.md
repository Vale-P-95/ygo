# DuelForge (nome provvisorio)

Suite desktop per Yu-Gi-Oh!: deck builder con carte e banlist sempre aggiornate, scaffale di deck box 3D personalizzabili, laboratorio statistico (calcolo esatto e Monte Carlo) e Combo Lab, un ricercatore di combo basato sul motore open source di EDOPro.

Il piano di progetto completo è in [`docs/PIANO.md`](docs/PIANO.md).

**Stato**: Fasi 0–4 completate. L'app scarica da sola carte e dati del motore. Nella Fucina si costruiscono i deck in stile Master Duel (Main, Extra, Side e Maybe board al centro, catalogo a destra), con banlist di Master Duel, TCG e OCG, costo di creazione in Master Duel, filtri rapidi e preferiti, mano di prova con probabilità esatte, salvataggio automatico, import/export `.ydk`/`ydke://` e immagine PNG del deck. Nella Bottega ogni deck è un deck box 3D personalizzabile (colore, materiale, forma, tre carte vetrina) con la legalità sempre aggiornata. Nel Laboratorio si calcolano le probabilità esatte delle mani in base ai ruoli delle carte, si simulano milioni di mani con Pot e ricerche, si ottimizza il numero di copie e si registrano le partite per vedere win-rate, matchup e l'effetto di ogni modifica del deck. Nel Circolo Arcano (Fase 5, in corso) il motore gioca da solo in background tutte le linee del primo turno da una mano o da una carta e mostra le board finali con la linea raccontata in italiano; con un obiettivo (carte da avere sul campo, anche in una zona precisa) dice se si raggiunge, con la linea, o se è impossibile. Le board migliori passano un turno di prova contro un avversario che gioca carte vere e usa board breaker: la classifica conta interruzioni, handtrap, floodgate e resilienza misurate, con profili e pesi personalizzabili.

## Struttura

```
apps/desktop/          app desktop: React + TypeScript (src/) e Tauri 2 (src-tauri/)
crates/ygo-engine/     collegamento Rust al motore EDOPro: partite, carte, script, deck .ydk
crates/ygo-data/       database delle carte (SQLite + ricerca full-text), linguaggio di ricerca, immagini, download, dati utente
crates/ygo-stats/      statistiche: calcolo esatto, Monte Carlo, ottimizzatore, statistiche delle partite
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
cargo test -p ygo-engine -p ygo-data -p ygo-cli
cargo clippy -p ygo-engine -p ygo-data -p ygo-cli -p duelforge-desktop --all-targets
cd apps\desktop; pnpm typecheck
```

## Costruire un deck

| Azione | Come |
|---|---|
| Aggiungere una carta | trascinala dal catalogo nel deck, oppure clic destro sulla carta del catalogo (Maiusc+clic destro: Side) |
| Togliere una carta | clic destro sulla carta nel deck, oppure trascinala sul catalogo |
| Spostare tra Main, Extra e Side | trascina la carta nell'altra sezione |
| Annullare, ripetere | Ctrl+Z, Ctrl+Y (o i pulsanti ↶ ↷) |
| Vedere i problemi | clic sul badge di legalità: ogni problema evidenzia le carte coinvolte |
| Importare, esportare | "I miei deck" → Importa; Esporta → testo (`.ydk` e `ydke://`) o immagine PNG (salvata in Download/DuelForge) |
| Provare le carte senza metterle nel deck | trascinale nella Maybe board, sotto il Side |
| Pescare mani di prova | pulsante Mano di prova: N nuova mano, D pesca una carta; segna Starter (S) e Handtrap (H) per contare le mani buone e vedere la probabilità esatta |
| Filtri rapidi, preferiti | pulsanti Handtrap, Staple, Board breaker, ★ Preferite sotto la ricerca; stella su ogni carta; segnalibro per salvare le ricerche |
| Cambiare la vista del catalogo | i quattro pulsanti accanto al numero di carte: carte grandi, medie (predefinita), piccole, elenco; trascina il bordo sinistro del catalogo per allargarlo |

I deck si salvano da soli in `user.db` nella cartella dati dell'app. Le carte Fusion, Synchro, Xyz e Link vanno sempre nell'Extra Deck, e il deck si riordina come in Master Duel (mostri, magie, trappole).

## La Bottega

Ogni deck salvato sta su uno scaffale, nel suo deck box. Clic su un box per personalizzarlo: colore, materiale (pelle, metallo, legno, olografico, opaco), forma (deck box, latta, cofanetto) e fino a tre carte vetrina. Il sigillo verde o rosso sul box dice se il deck è legale con le banlist in vigore. Dal pannello si apre il deck nella Fucina, lo si duplica o lo si elimina.

## Il Laboratorio

Il laboratorio lavora sui deck salvati. Nel pannello **Ruoli** si segna cosa fa ogni carta (Starter, Estensore, Handtrap, Board breaker, Brick e fino a tre ruoli personali; «Suggerisci» segna da solo handtrap e board breaker). Poi, per primo (5 carte) o per secondo (6):

| Scheda | Cosa fa |
|---|---|
| Panoramica | Probabilità esatte delle mani principali (starter, handtrap, mano ideale, mano che resiste a un'interruzione, mano bloccata), distribuzioni, curva delle carte viste, probabilità che l'avversario abbia una handtrap |
| Calcolatore | Condizioni composte su ruoli e carte (almeno, al massimo, esattamente; E, O, NON) con calcolo esatto; ricette salvate |
| Simulatore | Monte Carlo multi-thread con seed: Pot of Desires, Prosperity, Extravagance, Duality, Upstart Goblin, Allure, ricerche, una sola Normal Summon e effetti una volta per turno; intervallo al 95% e convergenza in diretta |
| Ottimizzatore | Valore di una copia in più o in meno per ruolo e per carta, 40 carte o più, mappa di due ruoli, ricerca automatica del deck migliore (anche con le carte della Maybe board) |
| Registro partite | Inserimento rapido (avversario, turno, Bo1 o Bo3, tag, note); win-rate con intervalli, matchup, primo e secondo, game 1 e side, meta incontrato, andamento, versioni del deck a confronto, quante partite servono; CSV |

Ogni partita ricorda la lista con cui è stata giocata: quando il deck cambia nasce una nuova versione, e il registro confronta le versioni dicendo quali carte sono cambiate.

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

# Il motore gioca da solo tutte le linee da una mano e elenca le board finali
cargo run --release -p ygo-cli -- combos --deck tests/fixtures/decks/invoked-siracusa-md.ydk \
  --hand "Aleister the Invoker;76334960;Magical Meltdown;Ash Blossom & Joyous Spring;Mulcharmy Fuwalos" --seconds 60 --lines

# Obiettivo: il motore cerca una linea che porti queste carte sul campo,
# o dimostra che non esiste
cargo run --release -p ygo-cli -- combos --deck tests/fixtures/decks/invoked-siracusa-md.ydk \
  --hand "Aleister the Invoker;76334960;Magical Meltdown;Ash Blossom & Joyous Spring;Mulcharmy Fuwalos" \
  --goal "Invoked Mechaba" --first --lines

# Obiettivo con la zona (@extra o @main) e turno di prova sulle board trovate:
# interruzioni, floodgate e cosa resta dopo ogni board breaker
cargo run --release -p ygo-cli -- combos --deck tests/fixtures/decks/invoked-siracusa-md.ydk \
  --hand "Aleister the Invoker;76334960;Magical Meltdown;Ash Blossom & Joyous Spring;Mulcharmy Fuwalos" \
  --goal "Invoked Oceanus@extra;Invoked Mechaba" --first --probe

# "Carta + resto": tante mani con una carta, pesate per quanto sono probabili
cargo run --release -p ygo-cli -- combos --deck tests/fixtures/decks/invoked-siracusa-md.ydk \
  --card "Aleister the Invoker" --hands 12 --seconds 120

# Collaudo del Combo Lab sulle liste più recenti di Master Duel Meta:
# scarica le liste (non vengono salvate nel repository) e gioca ricerca e
# turno di prova su ognuna, a budget di nodi (risultati riproducibili)
python3 scripts/mdm_decks.py --out /tmp/mdm
python3 scripts/combo_hunt.py /tmp/mdm --hands 2 --nodes 3000

# Quanto vale ogni carta da sola sul campo (o in mano, per le handtrap),
# misurato dal turno di prova: è la stima che guida la ricerca
cargo run --release -p ygo-cli -- card-values --deck tests/fixtures/decks/invoked-siracusa-md.ydk

# Handtrap del deck, riconosciute dal motore
cargo run --release -p ygo-cli -- hand-traps --deck tests/fixtures/decks/invoked-siracusa-md.ydk

# Ruoli delle carte (starter, estensore, handtrap, board breaker, brick),
# trovati giocandole: da sole, sulla board dello starter migliore
cargo run --release -p ygo-cli -- roles --deck tests/fixtures/decks/invoked-siracusa-md.ydk

# Collaudo a tappeto: duelli tra deck casuali con scelte casuali; segnala ogni
# scelta senza risposte o risposta rifiutata dal motore
cargo run --release -p ygo-cli -- sweep --duels 6000 --max-decisions 600 --max-turns 10
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
| `format:md`, `rarity:ur`, `ban:limited` | Master Duel, rarità MD, banlist (quella di Master Duel insieme a `format:md`, altrimenti TCG) |
| `tag:handtrap`, `tag:staple`, `tag:breaker`, `tag:fav` | gruppi rapidi: handtrap, staple, board breaker, carte preferite (★) |
| `sort:atk`, `sort:popular`, `sort:new` | ordinamento |

Ogni filtro scelto nel pannello dell'app diventa testo nella barra di ricerca, e viceversa.

## Licenza

Progetto open source con licenza [GNU AGPL-3.0-or-later](LICENSE), la stessa del motore e degli script di EDOPro.

Progetto non ufficiale, non affiliato né approvato da Konami. Yu-Gi-Oh! e i relativi nomi, testi e immagini delle carte sono di proprietà dei rispettivi titolari.
