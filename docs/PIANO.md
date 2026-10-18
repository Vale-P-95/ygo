# DuelForge: piano di progetto (v0.3)

> Nome provvisorio. Il documento è la proposta iniziale: una volta approvate le decisioni aperte (§14) diventa la roadmap di sviluppo.
>
> **v0.2 (18/10/2026)**: repository `Vale-P-95/ygo`; Master Duel come primo formato; testi delle carte in inglese; deck di prova Invoked; obiettivo di monetizzazione nel lungo periodo, quindi motore con licenza MIT (§8.1) e nuova sezione su monetizzazione e aspetti legali (§15).
>
> **v0.3 (18/10/2026)**: il progetto resta **open source** (licenza AGPL-3.0). Si torna quindi al motore di EDOPro, tecnicamente più adatto al solver (§8.1); la §15 diventa "Licenza, sostenibilità e aspetti legali".

## 0. Visione

DuelForge è un'applicazione desktop per giocatori di Yu-Gi-Oh! competitivi che unisce quattro strumenti oggi separati (o inesistenti):

1. **Deck Builder** completo, con database carte e banlist sempre aggiornati.
2. **Scaffale dei deck**: la collezione personale, con deck box 3D personalizzabili.
3. **Laboratorio statistico**: calcolo esatto, simulazioni Monte Carlo, ottimizzazione del deck e analisi dei risultati delle partite.
4. **Combo Lab**: un motore di gioco con le regole reali che, data una mano o una carta, trova le combo possibili, le classifica con un punteggio di potenza e le spiega passo per passo.

Principi guida:

- **Offline-first**: tutto funziona senza connessione. La rete serve solo per gli aggiornamenti.
- **Ogni numero è spiegabile**: probabilità con intervalli di confidenza, punteggi con il dettaglio delle componenti, combo con la ragione di ogni passo.
- **Riproducibilità**: ogni simulazione ha un seed e una configurazione salvata e si può rieseguire.
- **Prestazioni**: calcolo pesante in Rust e su tutti i core, interfaccia fluida a 60 fps.

---

## 1. Scelte tecnologiche

### 1.1 Confronto delle opzioni

| Opzione | Pro | Contro | Verdetto |
|---|---|---|---|
| **Tauri 2 + React/TypeScript + Rust** | GUI web moderna (animazioni, 3D, audio), backend nativo velocissimo, installer leggero (~10–20 MB), integrazione diretta con codice C/C++ | Due linguaggi (TS + Rust) | ✅ **Consigliata** |
| Electron + React + Node | Ecosistema enorme, una sola lingua | Installer pesante (~150 MB), Node lento per Monte Carlo e ricerca combo, addon nativi scomodi | Seconda scelta |
| Python + PySide6/Qt | Rapido da prototipare, ottimo per la statistica | GUI "moderna" e animazioni più faticose, prestazioni del solver insufficienti, distribuzione su Windows scomoda | No |
| C# + Avalonia/.NET | Buone prestazioni, desktop nativo | Ecosistema UI/animazioni più povero, 3D più complesso | Possibile, non ottimale |
| Unity / Godot | Effetti visivi da videogioco | Pessimo per UI dense (tabelle, filtri, form), più difficile da integrare con database e statistica | No |

### 1.2 Stack consigliato

**Frontend (interfaccia)**
- React + TypeScript + Vite.
- Tailwind CSS + Radix UI (componenti accessibili) per un design system pulito, con tema chiaro e scuro.
- Framer Motion per animazioni e transizioni.
- react-three-fiber + drei (Three.js) per i deck box 3D e gli effetti olografici.
- Howler.js per l'audio (sprite sonori, volumi per categoria).
- TanStack Virtual per le griglie da oltre 13.000 carte senza rallentamenti; TanStack Query per i dati; Zustand per lo stato.
- dnd-kit per il drag & drop; ECharts (o visx) per i grafici statistici; i18next per l'interfaccia in italiano e inglese (testi delle carte in inglese nella prima versione, altre lingue in seguito).

**Backend (Rust, dentro l'app Tauri)**
- rusqlite (SQLite con FTS5 per la ricerca nel testo delle carte), reqwest + tokio (download), serde.
- rayon per il parallelismo (Monte Carlo e solver su tutti i core), rand con generatori deterministici (seed).
- image per generare miniature WebP.
- `cc`/cmake + bindgen per compilare e collegare il motore di gioco C++ (ygopro-core).

**Motore di gioco**: il core di EDOPro (C++ con Lua embedded) e gli script delle carte di Project Ignis, entrambi AGPL-3.0 come il nostro progetto (vedi §8 e §15).

### 1.3 Dove gira

- **Target principale: Windows 10/11** (installer `.msi`/`.exe`). Grazie a Tauri gira anche su macOS e Linux senza riscrivere nulla.
- Requisiti indicativi: CPU multi-core (il solver usa tutti i core), 8 GB di RAM consigliati, 2–3 GB di disco se si scaricano tutte le immagini in alta risoluzione.
- Su Windows usa WebView2, già presente su Windows 11 (l'installer lo installa se manca su Windows 10).

---

## 2. Architettura

```
┌──────────────────────────── App desktop (Tauri) ─────────────────────────────┐
│  Frontend (WebView) — React + TypeScript                                       │
│  Deck Builder │ Scaffale │ Lab Statistico │ Combo Lab │ Novità │ Impostazioni   │
│        ▲ comandi (invoke)        ▼ eventi (progresso, risultati in streaming)  │
├────────┼──────────────────────────────────────────────────────────────────────┤
│  Backend Rust                                                                   │
│  data-sync │ card-db │ deck │ stats (Monte Carlo) │ engine-host │ solver        │
│                                                       ▲            ▲            │
│                                                       │ FFI (API C)│            │
│                   core EDOPro (C++, AGPL) + Lua ── script carte Project Ignis (Lua) │
├────────────────────────────────────────────────────────────────────────────────┤
│  Storage locale: cards.db (SQLite+FTS5) │ user.db │ cache immagini │ script    │
└────────────────────────────────────────────────────────────────────────────────┘
   ▲ aggiornamenti: YGOPRODeck API · Project Ignis (BabelCDB, CardScripts, LFLists) · banlist Master Duel · Konami
```

- **Dati di gioco** (carte, banlist, script, immagini) e **dati utente** (deck, partite, simulazioni, combo salvate) stanno in due database separati: i primi si possono riscaricare in qualsiasi momento, i secondi si salvano ed esportano.
- Le operazioni lunghe (simulazioni, ricerca combo) girano in background, mostrano l'avanzamento in tempo reale e si possono annullare.
- Una **CLI headless** (`duelforge-cli`) espone statistica, motore e solver senza interfaccia: serve per i test automatici, i benchmark e le esecuzioni in batch.

### 2.1 Struttura del repository (monorepo `ygo`)

```
ygo/
├─ apps/desktop/
│  ├─ src/                 # React: pagine, componenti, design system, audio, 3D
│  └─ src-tauri/           # shell Tauri, comandi IPC, permessi
├─ crates/
│  ├─ ygo-model/           # tipi condivisi: Card, Deck, Format, Banlist, ...
│  ├─ ygo-data/            # sync fonti, SQLite, FTS, immagini, changelog
│  ├─ ygo-deck/            # validazione formati, import/export (ydk, ydke, testo)
│  ├─ ygo-stats/           # ipergeometrica, Monte Carlo, ottimizzatore, analisi partite
│  ├─ ygo-engine/          # binding sicuri al motore, parser messaggi, stato canonico
│  ├─ ygo-solver/          # ricerca combo, scoring, stress test, spiegazioni
│  └─ ygo-cli/             # CLI headless per test, benchmark, batch
├─ vendor/ygopro-core/     # submodule del motore EDOPro (AGPL-3.0)
├─ assets/sounds/          # suoni CC0 o originali
├─ tests/rulings/          # scenari di ruling in formato dichiarativo
└─ docs/
```

---

## 3. Dati: carte, immagini, banlist, aggiornamenti

### 3.1 Fonti

| Fonte | Cosa fornisce | Note |
|---|---|---|
| **YGOPRODeck API v7** (`cardinfo.php`) | Tutte le carte TCG/OCG, testi (anche in italiano), archetipi, set, rarità, prezzi, stato in banlist, immagini (piena, piccola, artwork ritagliato) | Limite di 20 richieste/s (oltre, blocco di 1 ora). Chiedono di salvare i dati in locale e di non fare hotlink delle immagini: scarichiamo una volta e serviamo dalla cache |
| **Project Ignis – BabelCDB** | `cards.cdb` (SQLite) con i dati nel formato del motore, incluse carte OCG e pre-release | Aggiornato molto spesso via GitHub |
| **Project Ignis – CardScripts** | Lo script Lua di ogni carta: è ciò che fa "capire" gli effetti al motore | Licenza AGPL-3.0; aggiornamenti pre-release rapidi. Verificato: aggiornato a ottobre 2026 |
| **Project Ignis – LFLists** | Banlist: TCG, OCG, Worlds, Traditional, GOAT, Speed Duel, Rush | Verificato: a oggi contiene TCG 2026.09 e OCG 2026.10 |
| **Konami** (yugioh-card.com) | Punti Genesys ufficiali (TCG e OCG), banlist ufficiali | Fonte di verità per verificare le altre |
| **Master Duel** (formato prioritario) | Banlist, carte disponibili nel gioco, rarità di creazione (N/R/SR/UR), data di uscita nel gioco | Verificato il 18/10/2026: YGOPRODeck indica disponibilità e rarità in Master Duel ma **non** la banlist; Master Duel Meta espone banlist, rarità e data di uscita tramite un'API pubblica ma non documentata: chiedere il permesso prima di usarla nell'app distribuita. Inserimento manuale come riserva |
| Formati retro (Edison, HAT, Tengu Plant…) | Banlist dedicate | Fonte da scegliere in Fase 3; previsto anche l'inserimento manuale |

Chiave comune tra le fonti: il **passcode** della carta. Una tabella di riconciliazione gestisce gli ID temporanei delle carte pre-release (che cambiano quando la carta esce ufficialmente), gli alias (artwork alternativi) e i conflitti tra fonti.

### 3.2 Aggiornamento "in tempo reale"

1. Controllo all'avvio, poi ogni N ore (configurabile), più il pulsante "Aggiorna ora".
2. Per ogni fonte si verifica se è cambiato qualcosa (versione del DB, ETag HTTP, ultimo commit Git) e si scarica solo la differenza.
3. I dati scaricati vengono validati e applicati in **una transazione atomica**: in caso di errore l'app resta sulla versione precedente.
4. Si ricostruiscono gli indici di ricerca e si genera il **changelog**.

**Pagina "Novità"**: nuove carte (con anteprima), banlist cambiate con il dettaglio (es. "X: Limitata → Proibita"), punti Genesys modificati e soprattutto **l'impatto sui tuoi deck**: "Il deck *Snake-Eye* non è più legale in TCG dal 19/10/2026: X ora è Limitata (ne hai 3)".

### 3.3 Formati supportati

| Formato | Regola di deck | Fase |
|---|---|---|
| **Master Duel** (priorità) | Banlist Master Duel, solo carte uscite in Master Duel, costo di creazione per rarità | 1 |
| TCG Advanced | Banlist TCG | 2 |
| OCG (Giappone/Corea), OCG asiatico-inglese | Banlist OCG / AE | 2 |
| **Genesys** (TCG e OCG, liste punti separate) | Punti per carta con tetto (100 standard, modificabile dall'organizzatore), niente mostri Link e Pendulum, campo senza Extra Monster Zone e Zone Pendulum | 2 |
| TCG Traditional, Worlds | Banlist dedicate | 2 |
| Retro: GOAT, Edison, HAT, Tengu Plant… | Banlist storiche + pool di carte limitato a una data | 3 |
| Duel Links / Speed Duel | Regole e mazzi diversi (20–30 carte, Skill) | Futuro |
| Rush Duel | È un gioco diverso | Fuori scope v1 |
| **Banlist personalizzate** | Create dall'utente | 2 |

Si conserva ogni versione di ogni banlist, così ogni carta ha la sua **timeline** ("Limitata da… a…, Proibita da…") e un deck si può validare "alla data X" (utile per i formati retro).

### 3.4 Immagini

- Download pigro: un'immagine si scarica la prima volta che serve e poi resta in cache su disco.
- Per ogni carta si generano tre tagli WebP: miniatura (griglia), media (anteprima) e piena (zoom), più l'artwork ritagliato per i deck box.
- Opzione "Scarica tutto" in background, rallentata per rispettare i limiti della fonte.
- Le immagini sono servite all'interfaccia tramite il protocollo locale di Tauri: nessuna richiesta di rete durante l'uso.

### 3.5 Schema dati (bozza)

**cards.db (dati di gioco)**
- `cards`: passcode, alias, tipo, frame, attributo, tipo mostro, livello/rango/valore Link, frecce Link, scala, ATK/DEF, archetipi, disponibilità (TCG/OCG), date di uscita, presenza dello script.
- `card_texts`: carta, lingua, nome, testo, testo Pendulum (+ indice FTS5 per la ricerca nel testo).
- `card_prints`: set, codice, rarità, prezzi.
- `card_images`: percorsi locali dei tagli.
- `formats`, `banlist_versions`, `banlist_entries` (copie consentite o punti).
- `card_tags`: tag semantici (handtrap, negazione, ricerca, board breaker…), con origine (motore / curato / utente) e affidabilità.

**user.db (dati utente)**
- `decks`, `deck_versions` (main/extra/side, nota, data), `deck_boxes` (configurazione estetica), `folders`, `tags`.
- `matches` (versione del deck, archetipo avversario, primo/secondo, esito, game 1/2/3, side usate, note).
- `sim_runs` (configurazione, seed, risultati), `combos` (mano, sequenza di azioni, board finale, punteggio).

---

## 4. Deck Builder

### 4.1 Ricerca carte
- Barra di ricerca istantanea con un **linguaggio di query** leggibile, ad esempio:
  `attr:dark type:dragon atk>=2500 "negate" format:tcg legal`
- Filtri a faccette: tipo, sottotipo, attributo, tipo mostro, livello/rango/Link, ATK/DEF, archetipo, set, rarità, legalità per formato, stato in banlist, punti Genesys, data di uscita, esclusive TCG/OCG.
- Ricerca nel testo degli effetti (FTS5) e **ricerca per ruolo** grazie ai tag semantici: "handtrap", "nega", "cerca dal deck", "board breaker", "floodgate".
- Ricerche salvate, ordinamenti multipli, vista griglia o lista, risultati virtualizzati (scorrimento fluido anche con tutte le carte).

### 4.2 Costruzione
- Drag & drop tra risultati, Main, Extra e Side; clic destro per aggiungere o rimuovere, Shift+clic per tre copie, scorciatoie da tastiera.
- **"Maybe board"**: area per le carte in prova, che non contano nel deck.
- Ordinamento automatico (tipo, livello, nome) o manuale.
- **Validazione live** del formato scelto: 40–60 / 0–15 / 0–15 carte, copie, banlist, tetto punti Genesys (barra "punti usati/100"), carte non ancora uscite in TCG, tipi vietati (es. Link e Pendulum in Genesys). Ogni errore spiega il motivo ed evidenzia la carta.
- **Statistiche rapide** nel builder: ripartizione per tipo, curva di livelli, numero di starter e handtrap, P(almeno 1 starter) andando primo e secondo, costo del deck (in Master Duel: punti di creazione per rarità UR/SR/R/N e carte mancanti).

### 4.3 Import/Export
- `.ydk` (EDOPro/YGOPro), `ydke://`, lista in testo semplice, appunti.
- Immagine del deck (PNG da condividere), **modulo decklist ufficiale Konami** compilato in automatico (PDF per i tornei).

### 4.4 Versioni e piani di side
- Ogni salvataggio crea una versione; confronto tra versioni ("+1 X, −2 Y"), ripristino, note.
- **Side plan**: tabella matchup × carte (fuori/dentro), stampabile.

### 4.5 Avanzato (fasi successive)
- Collezione personale: carte possedute, carte mancanti per un deck, "lista della spesa" con la stampa più economica.
- Suggerimenti: staple del formato e carte giocate spesso con l'archetipo (se le fonti dati lo consentono).

---

## 5. Visualizzazione carte, suoni ed effetti

### 5.1 Viewer
- **Hover**: popup con immagine media, testo (inglese nella prima versione; altre lingue in seguito), statistiche, stato in banlist per i formati principali, punti Genesys.
- **Clic**: vista a schermo intero con zoom (rotella o pinch), pan, lente d'ingrandimento sul testo, artwork alternativi, tutte le stampe e rarità, timeline della banlist, carte correlate (stesso archetipo, carte nominate nel testo), link ai ruling.
- Confronto di due carte affiancate.

### 5.2 Effetti visivi
- Inclinazione 3D della carta all'hover e **riflesso olografico per rarità** (Secret, Ultimate, Starlight, Quarter Century, Ghost…), con shader CSS/WebGL.
- Bagliore per tipo di carta (Fusion, Synchro, Xyz, Link, Ritual, Pendulum), particelle quando una carta entra nel deck, "scossa" e suono di errore per un'azione non consentita, transizioni fluide tra le sezioni.
- Animazioni solo su proprietà gestite dalla GPU (target 60 fps) e rispetto dell'impostazione di sistema "riduci movimento".

### 5.3 Audio
- Catalogo di eventi sonori: hover, presa, rilascio, aggiunta (variante per mostro, magia e trappola), rimozione, errore, salvataggio, apertura del deck box, evocazioni nel Combo Lab (Fusion, Synchro, Xyz, Link, Ritual, Pendulum), catena, negazione.
- Mixer con volume per categoria (interfaccia, effetti, musica), mute globale, "pacchetti suono" intercambiabili.
- **Asset solo CC0, royalty-free o creati da noi**: niente suoni estratti da Master Duel o Duel Links (sono protetti da copyright).

---

## 6. Scaffale dei deck

- Vista a scaffale o a griglia dei **deck box 3D**; all'hover il box si apre e **spuntano le 3 carte vetrina** scelte dall'utente.
- **Personalizzazione**:
  - nome, colore (tinta unita, gradiente, metallizzato), materiale (opaco, lucido, pelle, olografico);
  - **forma del contenitore**: deck box classico, deck box con coperchio a ribalta, tin metallica, scrigno, portale runico (le carte emergono dal portale);
  - animazione di uscita delle carte: ventaglio, ascesa, rotazione;
  - emblema o attributo sul box, etichetta, artwork scelto per ogni carta vetrina;
  - **sleeve personalizzata** (anche da immagine caricata), usata poi nel Combo Lab.
- Su ogni box: formato, **badge di legalità aggiornato** (verde o rosso, con il motivo), ultima modifica, win-rate.
- Organizzazione: cartelle, tag, preferiti, ricerca, ordinamento, archivio.
- Backup ed esportazione dell'intera collezione; sincronizzazione cloud come estensione futura.

---

## 7. Laboratorio statistico

Oggi esistono solo calcolatori ipergeometrici di base. Qui l'obiettivo è un vero laboratorio.

### 7.1 Calcolatore esatto
- L'utente definisce **categorie** (Starter, Estensori, Handtrap, Brick, Garnet…) e **condizioni composte** con AND/OR/NOT e soglie (≥, ≤, =): "almeno 1 starter E al massimo 1 brick E almeno 1 handtrap".
- Mano da 5 (primo) o 6 (secondo), anche ai turni successivi con le pescate.
- Calcolo **esatto** con l'ipergeometrica multivariata sulle categorie; se le condizioni sono troppo complesse si passa automaticamente al Monte Carlo, dichiarandolo.

### 7.2 Simulatore Monte Carlo
- Simula mano e **carte che modificano il mazzo**: ricerche (thinning), Pot of Prosperity, Pot of Extravagance, Pot of Desires (con il rischio di bandire le carte chiave), Pot of Duality, Upstart Goblin, Allure of Darkness, Card of Demise, Triple Tactics Talent e in generale gli effetti di "excavate".
- **Valutatore di mani a regole**: matrice "queste due carte insieme fanno la combo", carte da sole sufficienti, conflitti (due starter che vogliono entrambi la Normal Summon), once-per-turn (due copie dello stesso starter non valgono il doppio).
- Risultati con **intervallo di confidenza al 95%**, errore standard, numero di simulazioni, grafico di convergenza e seed per rieseguirli.
- In Rust con rayon: milioni di mani in pochi secondi.

### 7.3 Ottimizzazione del deck
- **"Quante copie?"**: un ottimizzatore (simulated annealing o algoritmo genetico) cerca le quantità migliori rispettando i vincoli (40 carte, massimo 3 copie, banlist, punti Genesys, carte fisse) e massimizzando un obiettivo configurabile (es. P(mano giocabile) − penalità per i brick + valore delle handtrap).
- **Analisi di sensibilità**: valore marginale di +1/−1 copia di ogni carta (grafico a "tornado").
- Confronti 40 contro 41–45 carte, rapporto engine/non-engine, heatmap "copie di A × copie di B → probabilità".

### 7.4 Monte Carlo guidato dal motore (collegamento con il Combo Lab)
- Si campionano N mani reali e su ognuna gira il **solver** con un budget di tempo: si ottiene la **distribuzione del punteggio della board**, il numero atteso di interruzioni, la % di mani che arrivano a una board "≥ X" e le cause dei fallimenti (perché quella mano non parte).
- **Classificazione automatica dei ruoli**: il motore stabilisce quali carte sono starter (da sole arrivano a una board accettabile), estensori (alzano il punteggio se aggiunte) e brick (lo abbassano), invece di affidarsi solo all'etichetta manuale.
- **Valore atteso di una carta**: si enumerano le mani che la contengono raggruppate per classi equivalenti, pesate con la loro probabilità esatta, e si valuta ognuna con il solver.

### 7.5 Interazione con l'avversario
- Probabilità che l'avversario abbia almeno k handtrap, dato un deck tipo o un rapporto medio del meta.
- Probabilità di giocarci attraverso, misurata con lo stress test del solver (§9.4).

### 7.6 Registro partite e analisi
- Registrazione rapida: deck/versione, archetipo avversario, primo o secondo, esito, game 1/2/3, side usate, note.
- Win-rate con **intervalli di Wilson o bayesiani** (beta-binomiale), matrice dei matchup, andamento nel tempo, effetto primo/secondo.
- **A/B test tra versioni del deck**: test esatto di Fisher o confronto bayesiano, e calcolo di "quante partite servono per capire se la modifica funziona".

### 7.7 Visualizzazioni e report
Istogrammi, heatmap, tornado, Sankey (mano → linea di gioco → board finale) ed esportazione in HTML, PDF e CSV.

---

## 8. Motore di gioco

### 8.1 Scelta: integrare un motore esistente

Scrivere da zero un motore con migliaia di carte e tutti i ruling richiederebbe anni-uomo: si integra un motore esistente. Ne esistono due famiglie, non compatibili tra loro (gli script dell'una non girano sull'altra):

| | **YGOPro originale** (Fluorohydride) | **EDOPro** (edo9300 / Project Ignis) |
|---|---|---|
| Motore | `ygopro-core`, licenza **MIT** | fork di ygopro-core, licenza **AGPL-3.0** |
| Script delle carte | `ygopro-scripts`, **GPL-2**, ~13.500 carte ufficiali | `CardScripts`, **AGPL-3.0**, ~22.800 script (incluse carte anime e non ufficiali) |
| Manutenzione | attiva (motore aggiornato a ottobre 2026, script a settembre 2026) | attiva (ottobre 2026) |
| API | C con callback globali (`create_duel`, `new_card`, `process`, `get_message`, `set_responsei/b`, `query_field_card`…) | C più recente, opzioni per partita (`OCG_CreateDuel`, `OCG_DuelProcess`…) |
| Programma a pagamento e a codice chiuso | **Possibile**: motore MIT, script tenuti separati (§15) | Non possibile: l'intero programma diventerebbe AGPL |

**Scelta: EDOPro**, perché il progetto è open source (§15) e, a parità di licenza possibile, è il più adatto al Combo Lab:
- **le callback sono per partita**, non globali: ogni thread del solver ha le sue partite indipendenti, il che semplifica il calcolo parallelo;
- API più recente, con le opzioni del duello (regole del campo, flag per formato) passate alla creazione della partita;
- dati delle carte (BabelCDB), script e banlist (LFLists) arrivano dallo stesso progetto, con gli script pre-release più rapidi;
- è il motore del simulatore desktop più usato, quindi i bug vengono trovati e corretti in fretta.

In entrambe le famiglie il funzionamento di base è lo stesso:
- ogni carta è descritta da uno **script Lua**: il motore "capisce" le carte eseguendo questi script;
- le regole complesse sono implementate: catene, SEGOC, "missing the timing", once-per-turn rigidi e soft, costi, priorità, Master Rule attuale;
- le nuove carte arrivano presto, spesso prima dell'uscita ufficiale.

Il motore resta dietro un'interfaccia nostra (`ygo-engine`): se un giorno servisse cambiarlo, per esempio con il motore originale di YGOPro (licenza MIT), il resto del programma non cambia.

**Cosa significa "perfetto" in pratica**: nessun software lo è al 100%. Gli script hanno occasionalmente bug e le carte appena annunciate possono non essere ancora scriptate. Mitigazioni:
- aggiornamento continuo degli script dal repository ufficiale;
- una nostra **suite di test dei ruling** (§8.4) eseguita a ogni aggiornamento;
- le carte senza script sono segnalate chiaramente nell'interfaccia e il solver non le usa in silenzio;
- i bug trovati si segnalano a Project Ignis, con la correzione quando possibile.

### 8.2 Integrazione (crate `ygo-engine`)
- Compilazione del core di EDOPro + Lua dentro la build Rust, con binding generati e un wrapper sicuro.
- Callback per partita per leggere i dati delle carte (dal nostro database, alimentato anche da BabelCDB) e gli script (dalla cartella aggiornata da CardScripts).
- **Parser dei messaggi binari** del motore e modello tipizzato delle **decisioni**: scegli un'azione (idle), rispondi in catena, scegli carte, zone, posizione, opzione, sì/no, somme di livelli, tributi, dichiarazioni (carta, tipo, attributo, numero)… Ogni decisione diventa una lista di azioni enumerabili, la base sia della sandbox sia del solver.
- **Stato canonico** del campo letto con le query: carte e zone, posizioni, contatori, LP, effetti once-per-turn già usati, Normal Summon usata, Extra Deck disponibile.
- **Configurazione delle regole per formato** (per esempio il campo di Genesys senza Extra Monster Zone e Zone Pendulum) tramite le opzioni del duello: da verificare nello spike.
- **Snapshot**: il motore non sa clonare una partita in corso. Strategia: partite **deterministiche** (seed fisso) + registro delle risposte; per "tornare indietro" o ramificare si ricrea la partita e si rigioca il prefisso, con cache dei prefissi più usati. In fase di R&D si può valutare uno snapshot nativo in un fork del core.
- **Pool di istanze**: una partita indipendente per ogni thread di lavoro. Le callback sono per partita e i dati letti sono in sola lettura; lo spike verificherà comunque la sicurezza tra thread, con un pool di processi come alternativa.

### 8.3 Sandbox "goldfish"
- Giochi la tua mano con il motore reale su un campo di gioco grafico.
- **Undo/redo illimitati e rami**: ogni scelta crea un nodo di un albero, si torna indietro e si prova un'alternativa.
- **Editor di scenario**: metti qualsiasi carta in qualsiasi zona (anche avversaria: campo, GY, bandite, mano) per verificare "cosa succede se…".
- Salvataggio di una linea come guida combo, con annotazioni.

### 8.4 Test dei ruling
Scenari dichiarativi (stato iniziale → azioni → risultato atteso), per esempio "Ash Blossom su una ricerca in catena 2", SEGOC, "missing the timing", once-per-turn. Girano a ogni aggiornamento di motore e script, così una regressione si vede subito.

---

## 9. Combo Lab: il ricercatore di combo

### 9.1 Input e modalità
- Si parte da un **deck salvato**.
- **Mano**: (a) mano precisa; (b) "carta X + resto": campioni casuali oppure enumerazione di tutte le mani con X, raggruppate per classi equivalenti e pesate per probabilità; (c) mano casuale.
- **Primo o secondo**. Andando secondo si inserisce la **board avversaria** e il solver cerca le linee per romperla e l'eventuale OTK.
- **Vincoli**: carte da non usare, carte da tenere in mano (es. una handtrap), "deve finire con Y sul campo", profondità massima, budget di tempo.

### 9.2 Ricerca
Il problema: ogni stato del motore offre decine di azioni, molte sequenze sono permutazioni della stessa combo e molte decisioni (posizione, zona) sono spesso irrilevanti. Tecniche:

- **Tabella di trasposizione** con hash dello stato canonico: la stessa board raggiunta in ordini diversi viene esplorata una volta sola.
- **Riduzione delle simmetrie**: copie identiche della stessa carta; zone equivalenti, tenendo conto che frecce Link ed Extra Monster Zone *contano*, quindi le zone si canonicalizzano rispetto alle frecce; posizione d'attacco o difesa di default salvo quando rilevante.
- **Riduzione degli ordini parziali**: azioni indipendenti e commutative si esplorano in un solo ordine.
- **Macro-azioni**: "attiva l'effetto di X e risolvi la catena" diventa un unico passo, e le decisioni banali ricevono una risposta automatica.
- **Algoritmo anytime**: best-first/beam search con un'euristica (punteggio parziale + potenziale residuo), approfondimento iterativo, MCTS per gli stati molto ramificati; tutto in parallelo su tutti i core; modalità esaustiva fino a una profondità N quando è fattibile.
- **Eventi casuali** (pescate, excavate, effetti con scelta casuale): nodi di probabilità gestiti con più campioni dell'ordine del deck; una combo che dipende dalla fortuna riporta la sua **probabilità di riuscita**.
- **Guida neurale** (R&D, fase finale): una rete di valore/policy addestrata con self-play guida la ricerca nei deck più complessi. Esiste già un precedente su ygopro-core (ygo-agent, reinforcement learning).
- **Output**: le migliori K combo più la **frontiera di Pareto** (non un solo numero), con deduplicazione per board finale: per ogni board si mostra la linea più corta o più robusta.

### 9.3 Punteggio della board: il cuore del sistema

Il punteggio è la somma pesata di componenti, ciascuna con il proprio dettaglio:

```
Punteggio = wI·Interruzioni + wF·Floodgate + wR·Resilienza + wH·Interazioni dalla mano
          + wG·Risorse (GY/bandite/ED) + wU·Follow-up + wP·Pressione − wV·Vulnerabilità
```

**A. Interruzioni.** Per ogni effetto attivabile durante il turno avversario (quick effect, trappole settate, effetti in risposta):
- tipo: nega e distrugge/bandisce, nega l'attivazione, nega l'effetto, bandisce, rimanda in mano o nel deck, distrugge, cambia posizione;
- bersaglio sì/no, cosa può colpire (solo mostri, magie, trappole, effetti nel GY), velocità, frequenza (una volta, due volte, soft), costo, condizioni.

Pesi iniziali indicativi, da calibrare:

| Interruzione | Peso |
|---|---|
| Nega l'attivazione di qualsiasi carta + distrugge/bandisce | 1.00 |
| Rimozione rapida senza bersaglio (bandisci) | 0.80 |
| Nega effetti dei mostri, quick, senza bersaglio | 0.80 |
| Nega con bersaglio / "soft" | 0.60 |
| Distruzione rapida con bersaglio | 0.50 |

Le interruzioni non sono solo stimate: si **certificano con il motore**. Si passa il turno, l'avversario esegue un'azione fittizia e si controlla quali effetti il motore offre davvero come attivabili.

**B. Floodgate e lock.** Effetti continui che limitano l'avversario (limiti alle evocazioni speciali, blocco degli effetti nel GY, blocco dell'Extra Deck…), pesati per matchup: un blocco dell'Extra Deck vale molto contro un deck che ne dipende e poco contro gli altri.

**C. Resilienza.** Si applicano alla board i **board breaker** più comuni con il motore reale (Evenly Matched, Dark Ruler No More, Lightning Storm, Raigeki, Harpie's Feather Duster, Forbidden Droplet, i Kaiju, Super Polymerization, Triple Tactics Thrust…, lista aggiornabile col meta) e si misura cosa resta. Contano protezioni (non può essere bersagliato o distrutto) ed effetti "fluttuanti" (quando lascia il campo…).

**D. Risorse.** Handtrap e interazioni rimaste in mano (Ash Blossom, Infinite Impermanence, Nibiru, Effect Veiler, Ghost Ogre…), che contano come interruzioni aggiuntive; effetti utilizzabili dal GY o dalle bandite; Extra Deck rimasto; deck thinning; LP pagati.

**E. Follow-up.** Capacità di ripartire al turno successivo se la board viene rotta: recuperi, effetti in Standby/End Phase, risorse per una nuova combo (stimato con una mini-ricerca dal GY e dalla mano).

**F. Pressione** (soprattutto andando secondo): ATK sul campo, danno potenziale, OTK.

**G. Vulnerabilità.** Risorse sprecate, Normal Summon usata male, **choke point**: i passi in cui una sola handtrap ferma tutto.

**Profili di punteggio** predefiniti ("Massime interruzioni", "Grind/risorse", "Anti-meta", "OTK andando secondo") e **pesi personalizzabili**. Ogni punteggio ha un grafico di dettaglio (radar o barre) che mostra da dove viene.

**Classificazione degli effetti**: automatica a partire dagli script (ogni effetto dichiara categorie come negazione, distruzione, bandire, ricerca, evocazione speciale; tipo quick o trigger; limiti di attivazione), più un **database di eccezioni curato** e, come opzione, un tagging del testo della carta assistito da un modello linguistico e verificato.

**Calibrazione dei pesi**: confronto con board note dei deck meta e feedback dell'utente ("questa board è migliore di quella"), da cui si possono imparare i pesi.

### 9.4 Turno avversario e interazioni

1. **Livello 1, valutazione statica**: il punteggio di §9.3.
2. **Livello 2, stress test delle handtrap durante la combo**: per ogni passo il motore prova a inserire Ash Blossom, Infinite Impermanence, Effect Veiler, Nibiru, Droll & Lock Bird, Ghost Belle, D.D. Crow, Bystial, Called by the Grave, Mulcharmy/Maxx "C" e simili (lista configurabile) **nel momento peggiore**, e il solver cerca la migliore continuazione. Risultato: "punteggio sotto interruzione" e consigli come *"Se ti fanno Ash su X, passa alla linea B (board da 2 interruzioni)"*. Le combo si possono ordinare per **valore atteso**, pesato con la probabilità che l'avversario abbia quella handtrap.
3. **Livello 3, turno avversario simulato**: contro la serie di board breaker e contro gli starter di un deck avversario caricabile, il solver decide **dove usare le tue interruzioni** (sui choke point dell'avversario) e quante risorse ti restano per il turno 3.
4. **Andando secondo**: dalla board avversaria alle linee di rottura e all'OTK.

### 9.5 Spiegazione della combo (interfaccia)
- **Timeline verticale** dei passi, per esempio: *Normal Summon di X → attivi l'effetto di X (costo: scarti Y) → aggiungi Z alla mano…*, con icone per tipo di azione.
- **Catene visualizzate**: anelli di catena impilati (CL1, CL2…) e risoluzione all'indietro.
- **Campo animato** che si aggiorna a ogni passo, con play/pausa/passo singolo/velocità ed effetti di evocazione per tipo.
- **Pannello risorse**: mano, GY, bandite, deck, Extra Deck, con le variazioni a ogni passo; tracker degli effetti "una volta per turno" già usati.
- **Avvisi sui choke point**: le icone delle handtrap che fermerebbero quel passo, con il ramo alternativo.
- **Board finale**: elenco delle interruzioni (cosa negano e quando), dettaglio del punteggio, confronto affiancato tra combo.
- **Perché**: annotazioni sui passi non ovvi ("prima evochi X perché Y richiede un Drago nel GY").
- Esportazione come guida combo testuale (stile community), immagine o PDF; libreria delle combo salvate per ogni deck.

### 9.6 Limiti dichiarati
Il numero di combo cresce in modo esponenziale con la lunghezza della linea. La ricerca è **esaustiva entro profondità e tempo stabiliti**; nei deck molto complessi il risultato è "la migliore trovata nel budget", con un **indicatore di copertura** della ricerca. È una garanzia onesta e misurabile, invece di una promessa impossibile.

---

## 10. Funzioni aggiuntive proposte

- **Draw simulator** visuale: mani di prova rapide da 5 o 6 carte, con la pesca del turno.
- **Modalità allenamento**: quiz "qual è la giocata migliore?" generati dal solver e quiz sui ruling generati dalla suite di test.
- **Meta dashboard**: archetipi più giocati, rapporti medi di handtrap e board breaker (se le fonti dati lo consentono), usata anche per pesare floodgate e stress test.
- **Strumenti per i tornei**: modulo decklist ufficiale, side plan stampabile, registro partite rapido durante l'evento.
- **Notifiche**: nuovi set, banlist, punti Genesys.
- Interfaccia **in italiano e inglese**, accessibilità (tastiera, contrasto, daltonismo), backup automatici.
- In futuro: sincronizzazione cloud e app companion per smartphone (registro partite e consultazione deck).

---

## 11. Roadmap

| Fase | Contenuto | Dimensione | Risultato verificabile |
|---|---|---|---|
| **0. Fondamenta + spike** | Monorepo, CI che produce l'installer Windows, scheletro Tauri, design system. **Spike sul motore**: compilare il core di EDOPro su Windows e Linux, caricare script, giocare una mano scriptata da CLI, misurare la velocità del replay. Spike sui dati: import carte e immagini | M | L'app si installa da GitHub Actions; la CLI gioca una combo nota |
| **1. Database carte & viewer** | Sync fonti, ricerca e filtri, viewer con zoom e popup, prime animazioni e suoni | M | Cerco e visualizzo qualsiasi carta offline |
| **2. Deck Builder** | Builder completo, formati e banlist (prima Master Duel, poi TCG/OCG e Genesys), validazione, import/export, versioni, pagina Novità | L | Costruisco un deck legale, lo esporto in .ydk e vedo l'impatto di una nuova banlist |
| **3. Scaffale deck** | Deck box 3D, personalizzazione, carte vetrina, audio ed effetti estesi | M | Scaffale con box personalizzati e animati |
| **4. Lab statistico v1** | Calcolo esatto, Monte Carlo, ottimizzatore, sensibilità, registro partite, report | L | Probabilità esatte verificate contro casi noti; MC con intervalli |
| **5. Motore + sandbox** | Wrapper completo, stato canonico, sandbox goldfish con undo e rami, editor di scenario, suite di ruling | XL | Gioco a mano una combo reale con undo; i test di ruling passano |
| **6. Combo Lab v1** | Solver (trasposizioni, simmetrie, macro-azioni, best-first parallelo), punteggio statico, interfaccia di spiegazione | XL | Data una mano, trova la combo nota del deck di prova e la spiega |
| **7. Combo Lab v2** | Stress test handtrap, board breaker, turno avversario, modalità secondo, MC guidato dal motore, ruoli automatici | XL | Classifica combo per valore atteso sotto interruzione |
| **8. Rifinitura & R&D** | Guida neurale, meta dashboard, collezione e prezzi, auto-aggiornamento dell'app, ottimizzazioni | L+ | — |

**Prima versione usabile**: al termine della Fase 2 (database + builder). Lo spike sul motore sta già nella Fase 0 perché è il rischio tecnico principale: meglio scoprire subito eventuali problemi.

**Deck di prova per il solver**: l'Invoked con cui Danilo Siracusa è arrivato Top 8 all'European WCQ 2026 (11 luglio 2026). Ha combo note e documentate, quindi i risultati si possono confrontare con le guide della community. È una lista TCG: per il Combo Lab non cambia nulla (le regole sono le stesse), mentre il validatore ne controllerà la legalità contro la banlist di Master Duel. Poi si passa a deck via via più complessi.

---

## 12. Ambiente di sviluppo e test

### 12.1 Strumenti (sul tuo PC Windows)
- **VS Code** con le estensioni rust-analyzer, Tauri, ESLint, Prettier.
- **Node.js LTS + pnpm**, **Rust** (rustup), **Visual Studio Build Tools** (compilatore C++, serve per il motore), **Git**.

### 12.2 Dove e come testare
1. **Sul tuo PC in sviluppo**: `pnpm tauri dev` avvia l'app con hot-reload (le modifiche all'interfaccia si vedono subito).
2. **Senza installare nulla**: a ogni push, **GitHub Actions** compila l'installer Windows (e macOS/Linux). Lo scarichi dalla pagina Actions o Releases e lo installi come un normale programma.
3. **Test automatici** in CI: `cargo test` (statistica, motore, solver), Vitest (componenti), Playwright (test end-to-end dell'interfaccia), suite dei ruling, benchmark del solver.
4. **Nelle sessioni cloud con me**: posso compilare e testare i moduli Rust e la CLI, avviare l'interfaccia in un browser headless e mandarti screenshot. Nell'ambiente cloud "Yu Gi Oh" sono abilitati (18/10/2026) `ygoprodeck.com`, `db.ygoprodeck.com`, `images.ygoprodeck.com`, `yugiohmeta.com` e `masterduelmeta.com`, oltre a GitHub e ai registri dei pacchetti.

### 12.3 Rilasci
Versionamento semantico, release su GitHub con changelog e **auto-aggiornamento dell'app** tramite l'updater di Tauri (aggiornamento di programma, separato da quello dei dati delle carte).

---

## 13. Rischi e mitigazioni

| Rischio | Impatto | Mitigazione |
|---|---|---|
| Esplosione combinatoria nella ricerca combo | Alto | Ricerca anytime, trasposizioni, simmetrie, macro-azioni, budget e indicatore di copertura, guida neurale in R&D |
| Bug negli script o carte non ancora scriptate | Medio | Aggiornamenti continui, suite di ruling, segnalazione in UI, contributi upstream |
| Licenze di motore e script | Basso | Progetto AGPL-3.0 come motore e script; file LICENSE nel repository; controllo automatico delle licenze delle dipendenze in CI (§15) |
| Proprietà intellettuale Konami (immagini, nomi, testi, marchio) | Medio (alto con servizi a pagamento) | Niente asset Konami nel programma, immagini scaricate dall'utente, nome e logo senza marchi Konami, avviso "non ufficiale" (§15) |
| Fonti dati che cambiano o limitano l'accesso | Medio | Cache locale completa, più fonti con adattatori intercambiabili, rispetto dei limiti di richieste |
| Prestazioni dell'interfaccia con oltre 13.000 carte | Basso | Virtualizzazione, miniature WebP, ricerca indicizzata |
| Ampiezza del progetto | Alto | Fasi con risultati verificabili, prima versione usabile alla Fase 2 |

---

## 14. Decisioni aperte

1. **Nome** definitivo (DuelForge è provvisorio).
2. ~~Repository dedicato~~: deciso, il progetto vive nel repository `Vale-P-95/ygo`.
3. ~~Licenza~~: deciso, progetto open source AGPL-3.0 con il motore di EDOPro (§8.1, §15).
4. ~~Formati prioritari~~: deciso, prima Master Duel.
5. ~~Lingua del testo delle carte~~: deciso, inglese per ora, altre lingue in seguito.
6. ~~Deck di prova~~: deciso, Invoked di Danilo Siracusa (European WCQ 2026), salvato in `tests/fixtures/decks/`. È una lista TCG: in Master Duel non è legale così com'è (3 carte Proibite, Super Polymerization Limitata a 2).
7. **Fonte della banlist di Master Duel**: proposta Master Duel Meta, previa autorizzazione; disponibilità e rarità da YGOPRODeck (§3.1).
8. **Sostenibilità**: se e come finanziare il progetto (§15.3), da decidere più avanti.

---

## 15. Licenza, sostenibilità e aspetti legali

> Questa sezione non è una consulenza legale: per qualsiasi uso commerciale serve il parere di un avvocato esperto di proprietà intellettuale e licenze software.

### 15.1 Licenza: AGPL-3.0-or-later
- È la licenza del motore e degli script di EDOPro, quindi anche del nostro progetto, che li usa.
- Chiunque può usare, studiare, modificare e ridistribuire il programma; chi distribuisce versioni modificate, o le offre come servizio online, deve pubblicarne il codice con la stessa licenza.
- Le dipendenze (React, Tauri, librerie Rust) hanno licenze permissive (MIT/Apache) compatibili; un controllo automatico in CI verifica le licenze delle nuove dipendenze.
- **Per lasciare aperte opzioni future**: come autore del codice puoi sempre rilasciarlo anche con un'altra licenza. Se arriveranno contributi esterni, un accordo di contribuzione (CLA) preserva questa possibilità anche per il loro codice.

### 15.2 Proprietà intellettuale Konami
Anche per un progetto open source e gratuito, nomi, testi e immagini delle carte e il marchio Yu-Gi-Oh! appartengono a Konami. I simulatori dei fan sono in genere tollerati, ma senza garanzie: nel 2016 Konami fece chiudere Dueling Network. Accorgimenti:
- nessuna immagine, suono o asset di Konami nel repository: le immagini le scarica l'app dell'utente in locale;
- nome e logo del programma senza "Yu-Gi-Oh!" né elementi grafici di Konami; avviso chiaro "prodotto non ufficiale, non affiliato a Konami";
- nessuna interazione con il client di Master Duel (niente lettura della memoria, overlay o automazioni): rispetta i termini di servizio ed evita rischi di ban per gli utenti.

### 15.3 Sostenibilità senza chiudere il codice
Open source non significa per forza gratis per sempre: se in futuro servisse coprire i costi, le strade compatibili con l'AGPL sono donazioni (GitHub Sponsors, Patreon), sponsor, contenuti (guide combo generate dal Combo Lab) e servizi cloud opzionali, come il solver su server potenti, il cui codice resterebbe comunque pubblico. Ognuna di queste aumenta l'esposizione verso Konami e va valutata quando sarà il momento.

### 15.4 Conseguenze sull'architettura
- Il motore sta dietro un'interfaccia (`ygo-engine`), quindi resta sostituibile.
- Il repository contiene il file `LICENSE` (AGPL-3.0) e un avviso di licenza nel README e nella schermata "Informazioni" dell'app.
