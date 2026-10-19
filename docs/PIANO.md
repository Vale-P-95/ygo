# DuelForge: piano di progetto (v1.2)

> Nome provvisorio. Il documento è la proposta iniziale: una volta approvate le decisioni aperte (§14) diventa la roadmap di sviluppo.
>
> **v0.2 (18/10/2026)**: repository `Vale-P-95/ygo`; Master Duel come primo formato; testi delle carte in inglese; deck di prova Invoked; obiettivo di monetizzazione nel lungo periodo, quindi motore con licenza MIT (§8.1) e nuova sezione su monetizzazione e aspetti legali (§15).
>
> **v0.3 (18/10/2026)**: il progetto resta **open source** (licenza AGPL-3.0). Si torna quindi al motore di EDOPro, tecnicamente più adatto al solver (§8.1); la §15 diventa "Licenza, sostenibilità e aspetti legali".
>
> **v0.4 (18/10/2026)**: Fase 0 realizzata; risultati e misure nella §16.
>
> **v0.5 (18/10/2026)**: Fase 1 realizzata; risultati nella §17.
>
> **v0.6 (18/10/2026)**: Fase 2, prima parte: editor dei deck in stile Master Duel; risultati nella §18.
>
> **v1.2 (19/10/2026)**: la Fase 5 diventa direttamente il Combo Lab: ricerca automatica delle combo in background, senza partite da giocare a mano. La sandbox manuale (§8.3) esce dalla roadmap. Piano dettagliato, con il sistema di punteggio delle board, nella §22.
>
> **v1.1 (19/10/2026)**: prima proposta di Fase 5 con una sandbox da giocare a mano, sostituita dalla v1.2.
>
> **v1.0 (19/10/2026)**: Fase 4 realizzata: il Laboratorio statistico; risultati nella §21.
>
> **v0.9 (18/10/2026)**: piano dettagliato della Fase 4 (Laboratorio statistico) nella §20, da approvare prima di iniziare.
>
> **v0.8 (18/10/2026)**: Fase 3 realizzata: la Bottega con i deck box 3D; risultati nella §19.
>
> **v0.7 (18/10/2026)**: Fase 2 completata con le migliorie scelte (banlist di Master Duel, mano di prova, immagine del deck, filtri rapidi e preferiti, Maybe board); versioni dei deck, side plan e pagina Novità passano più avanti (§18.4).

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
> **Uscita dalla roadmap (v1.2)**: l'obiettivo è che il motore giochi da solo, non giocare a mano. Del Tavolo resta solo la riproduzione passo per passo delle combo trovate (§22.6), da guardare e non da giocare. Se in futuro servirà, la sandbox riusa lo stesso motore.

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

**C. Resilienza.** Si applicano alla board i **board breaker** più comuni con il motore reale (Evenly Matched, Dark Ruler No More, Lightning Storm, Raigeki, Harpie's Feather Duster, Forbidden Droplet, i Kaiju, Super Polymerization, Triple Tactics Talent…, lista aggiornabile col meta) e si misura cosa resta. Contano protezioni (non può essere bersagliato o distrutto) ed effetti "fluttuanti" (quando lascia il campo…).

**D. Risorse.** Handtrap e interazioni rimaste in mano (Ash Blossom, Infinite Impermanence, Nibiru, Effect Veiler, Ghost Ogre…), che contano come interruzioni aggiuntive; effetti utilizzabili dal GY o dalle bandite; Extra Deck rimasto; deck thinning; LP pagati.

**E. Follow-up.** Capacità di ripartire al turno successivo se la board viene rotta: recuperi, effetti in Standby/End Phase, risorse per una nuova combo (stimato con una mini-ricerca dal GY e dalla mano).

**F. Pressione** (soprattutto andando secondo): ATK sul campo, danno potenziale, OTK.

**G. Vulnerabilità.** Risorse sprecate, Normal Summon usata male, **choke point**: i passi in cui una sola handtrap ferma tutto.

**Profili di punteggio** predefiniti ("Massime interruzioni", "Grind/risorse", "Anti-meta", "OTK andando secondo") e **pesi personalizzabili**. Ogni punteggio ha un grafico di dettaglio (radar o barre) che mostra da dove viene.

**Classificazione degli effetti**: automatica a partire dagli script (ogni effetto dichiara categorie come negazione, distruzione, bandire, ricerca, evocazione speciale; tipo quick o trigger; limiti di attivazione), più un **database di eccezioni curato** e, come opzione, un tagging del testo della carta assistito da un modello linguistico e verificato.

**Calibrazione dei pesi**: confronto con board note dei deck meta e feedback dell'utente ("questa board è migliore di quella"), da cui si possono imparare i pesi.

> La versione dettagliata e misurata dal motore di questo punteggio è nella §22.4.

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
| **5. Combo Lab v1** | Motore pronto per la ricerca (stato canonico, tutte le scelte gestite, velocità), ricerca automatica in background da una mano o da una carta, punteggio misurato e personalizzabile delle board, spiegazione passo per passo (§22) | XL | Data una mano, trova da solo la combo nota del deck di prova e la classifica, spiegando ogni punto |
| **6. Combo Lab v2** | Stress test con le handtrap, andare secondo contro una board da rompere, turno avversario simulato con deck del meta, MC guidato dal motore, ruoli automatici | XL | Classifica combo per valore atteso sotto interruzione |
| **7. Rifinitura & R&D** | Guida neurale, meta dashboard, collezione e prezzi, auto-aggiornamento dell'app, ottimizzazioni | L+ | — |

**Prima versione usabile**: al termine della Fase 2 (database + builder). Lo spike sul motore sta già nella Fase 0 perché è il rischio tecnico principale: meglio scoprire subito eventuali problemi.

**Deck di prova per il solver**: l'Invoked con cui Danilo Siracusa è arrivato Top 8 all'European WCQ 2026 (11 luglio 2026), nella versione resa legale per Master Duel (`tests/fixtures/decks/invoked-siracusa-md.ydk`). Ha combo note e documentate, quindi i risultati si possono confrontare con le guide della community; la lista originale TCG resta come caso di prova per il validatore di formato. Poi si passa a deck via via più complessi.

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
6. ~~Deck di prova~~: deciso, Invoked di Danilo Siracusa (European WCQ 2026), salvato in `tests/fixtures/decks/`. Si usa la versione legale in Master Duel (`invoked-siracusa-md.ydk`): al posto di Called by the Grave, Dimension Shifter e Terraforming ci sono 3 Invocation - "Grail", e al posto di una Super Polymerization un Triple Tactics Talent.
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

---

## 16. Risultati della Fase 0 (18/10/2026)

### 16.1 Cosa è stato fatto
- **Motore**: il core di EDOPro (API 11.0) e Lua sono compilati dentro il crate Rust `ygo-engine` (`cc`, nessun premake). Il crate gestisce le partite, legge database e script, interpreta i messaggi di decisione del motore e trasforma ogni decisione nell'elenco delle azioni legali, con la risposta binaria esatta che il motore si aspetta.
- **Dati**: la CLI scarica CardScripts e BabelCDB con `git` (`duelforge-cli fetch-data`): 15.004 carte e 19.315 script caricati. Tutte le carte del deck Invoked hanno lo script.
- **CLI** `duelforge-cli`: `info`, `play` (interattivo o seguendo una linea di passi), `bench`.
- **Test end-to-end**: la linea Magical Meltdown → Aleister the Invoker → Invocation → Invocation - "Chalice" → Invoked Baybarron gira nel motore e la board finale viene verificata.
- **App desktop**: scheletro Tauri 2 + React con le sezioni del piano, tema chiaro/scuro e il pannello di stato del motore (il frontend interroga il motore reale).
- **CI**: GitHub Actions con formattazione, clippy e test (dati scaricati in CI), build del frontend e installer Windows (`.exe` NSIS) come artefatto scaricabile.

### 16.2 Il motore applica i ruling da solo
Nella linea di prova, Invoked Baybarron non offre la sua ricerca: cerca Virakam o Gamma Summon nel Deck, ma l'unico Virakam è stato bandito come materiale e Gamma Summon è nel Side Deck. Nessuna regola è stata scritta da noi: è lo script della carta eseguito dal motore. È esattamente il comportamento che serve al Combo Lab.

### 16.3 Prestazioni misurate (4 core, build release)
| Misura | Valore |
|---|---|
| Creazione di una partita vuota (solo script di base) | ~12 ms |
| Creazione della partita con il deck + prima decisione | ~17 ms |
| Elaborazione di una decisione | ~0,2 ms |
| Riprodurre da zero una linea di ~25 decisioni | ~22 ms |
| Decisioni al secondo riproducendo da zero, 4 thread in parallelo | ~4.400 |

**Conclusione**: la parte lenta non è il gioco ma la creazione della partita (caricamento di `constant.lua`, `utility.lua` e delle procedure). Ripristinare un nodo della ricerca rigiocando da zero costa ~20 ms, troppo per esplorare milioni di stati. In Fase 6 servirà una o più di queste soluzioni, in ordine di costo:
1. **ricerca in profondità che riusa la partita**: si ricrea la partita solo tornando indietro, e i rami fratelli si esplorano in blocco;
2. **pool di partite "calde"** già posizionate sui prefissi più usati;
3. **script di base precompilati** in bytecode Lua, per ridurre i ~12 ms di avvio;
4. **snapshot nativo** della partita in un fork del motore (la soluzione più efficace e più costosa).

La parallelizzazione su più thread funziona: ogni thread ha le sue partite.

### 16.4 Scoperte sui dati
- **I nomi delle carte non coincidono tra le fonti**: la stessa carta (passcode 76334960) è *Invocation - "Grail"* su YGOPRODeck e *Invocation - "Chalice"* nel database di EDOPro. La chiave unica tra le fonti deve essere il passcode, mai il nome.
- **Artwork alternativi**: i deck di YGOPRODeck usano a volte il passcode dell'artwork alternativo (Ash Blossom 14558128 invece di 14558127). Il motore li gestisce tramite `alias`; il nostro codice li tratta come la stessa carta.
- **Banlist Master Duel**: non è nei dati di YGOPRODeck né in LFLists di Project Ignis (vedi §3.1).

### 16.5 Prossimi passi
- Verificare in CI la compilazione del motore con MSVC e scaricare il primo installer Windows.
- Fase 1: database carte unificato (YGOPRODeck + BabelCDB, chiave passcode), immagini in cache, ricerca e visualizzatore.

---

## 17. Risultati della Fase 1 (18/10/2026)

### 17.1 Dati
- Nuovo crate `ygo-data`: l'elenco completo di YGOPRODeck (14.618 carte) diventa un database SQLite locale con indice full-text FTS5. Una ricerca tipica richiede pochi millisecondi.
- **Linguaggio di ricerca** con alias italiani (`tipo:magia`, `attributo:luce`, …) condiviso da app e CLI: i filtri del pannello scrivono nella barra di ricerca, quindi ciò che si vede è ciò che si cerca.
- **Immagini** scaricate alla prima visualizzazione e poi servite dal disco tramite il protocollo `cardimg://`, con un massimo di circa 14 richieste al secondo (limite YGOPRODeck: 20).
- **Download dentro l'app** con avanzamento: elenco carte (YGOPRODeck, con controllo di versione) e dati del motore (zip da GitHub, controllo del commit). Se fallisce solo la parte del motore, il catalogo funziona lo stesso e l'app mostra un avviso.
- Scoperta: YGOPRODeck a volte usa come id principale quello di un artwork alternativo (Aleister the Invoker: 86120752). In Fase 2 i deck importati andranno risolti tramite tutti gli id delle immagini.

### 17.2 Interfaccia "Fucina"
- **Temi**: Fucina (scuro, oro fuso e brace) e Pergamena (chiaro), più "come il sistema". Ogni sezione ha il suo colore: brace per la Fucina, ciano per il Laboratorio, viola per il Circolo Arcano, ottone per la Bottega.
- **Ispirazione dalle carte**: l'artwork di una carta a tema sfuma nell'intestazione di ogni sezione (Fiendsmith Engraver per la Fucina, Brain Research Lab per il Laboratorio, Ancient Gear Workshop per l'Officina, Witchcrafter Schmietta per la Bottega, Aleister the Invoker per il Circolo Arcano). Le immagini si scaricano a runtime e non sono nel repository.
- **Effetti visivi**: inclinazione 3D delle carte che segue il puntatore, riflesso e foil olografico per rarità di Master Duel (argento per SR, arcobaleno per UR), scintille quando si seleziona una carta, braci che salgono sullo sfondo, transizioni tra le sezioni. Tutto disattivabile, e rispettato "riduci movimento" del sistema.
- **Effetti sonori**: sintetizzati in tempo reale con Web Audio (nessun file audio da terzi): tic al passaggio, "clink" metallico alla selezione, folate per aprire e chiudere, martello sull'incudine a fine download, rintocchi ed errori. Mixer con volume generale, interfaccia ed effetti.
- **Catalogo** (Fucina): griglia virtualizzata di tutte le carte, pannello filtri a scomparsa, filtri come chip rimovibili, anteprima al passaggio del mouse, banco con i dettagli della carta (nomi tra virgolette e archetipi cliccabili per cercare).
- **Lente**: visualizzatore a tutto schermo con zoom intorno al puntatore (rotella, +/−, doppio clic), trascinamento, lente d'ingrandimento (L), artwork alternativi, navigazione con le frecce.
- **Primo avvio**: schermata "La fucina è spenta" con il download guidato e barre di avanzamento "metallo fuso".

### 17.3 Prossimo passo: Fase 2
Deck Builder vero e proprio (Main/Extra/Side con drag & drop e suoni "forgia"), banlist di Master Duel e validazione, import/export `.ydk`, pagina Novità.

## 18. Risultati della Fase 2 (18/10/2026)

### 18.1 Dati e regole
- `ygo-data` ora gestisce i deck: modello (Main, Extra, Side, formato), import ed export `.ydk` e `ydke://`, e un archivio locale `user.db` (SQLite) separato dal database delle carte, così un aggiornamento delle carte non tocca mai i deck dell'utente.
- **Validazione** per Master Duel, TCG e OCG: 40–60 carte nel Main, al massimo 15 in Extra e Side, carte sconosciute, carte da Extra Deck nella sezione sbagliata, skill e token, disponibilità nel formato, numero di copie con i limiti delle banlist TCG e OCG. Ogni problema elenca le carte coinvolte, che l'interfaccia evidenzia.
- **Artwork alternativi**: all'importazione ogni passcode viene ricondotto alla carta giusta anche quando YGOPRODeck usa un altro id come principale (§17.1).
- **Banlist di Master Duel**: YGOPRODeck non la pubblica, quindi DuelForge ne tiene una propria in `crates/ygo-data/banlists/md.json` (lista in vigore dal 6 ottobre 2026: 111 proibite, 72 limitate, 26 semi-limitate), inclusa nell'app. Quando aggiorna i dati, l'app cerca una versione più recente nel repository: per una nuova banlist basta aggiornare quel file, senza una nuova release. La lista guida validazione, badge, dettagli della carta e la ricerca `ban:` insieme a `format:md`.
- **Prova**: con la banlist vera l'Invoked di Siracusa resta legale in Master Duel (Droll & Lock Bird e Super Polymerization semi-limitate, 2 copie ciascuna).

### 18.2 Editor "Fucina"
- Disposizione in stile Master Duel, come scelto: deck al centro (Main 10 carte per riga con 40 posti tratteggiati, poi Extra e Side), catalogo in una colonna a destra. Una carta per ogni copia.
- **Drag & drop** con puntatore (non quello HTML5, che in WebView2 e WebKitGTK è poco fluido): dal catalogo al deck, tra le sezioni e dal deck al catalogo per togliere. La carta trascinata segue il puntatore, la sezione di arrivo si illumina e una carta da Extra Deck lasciata nel Main finisce comunque nell'Extra.
- **Clic destro**: aggiunge dal catalogo (Maiusc: nel Side) e toglie dal deck. Il catalogo mostra "×N" sulle carte già nel deck.
- **Catalogo**: carte grandi, medie o piccole (circa 3, 4 o 5 per riga) oppure elenco con miniatura, tipo, statistiche e stato in banlist; la colonna si allarga trascinandone il bordo, e con il catalogo largo il deck intero sta in una schermata.
- **Effetti**: "clink" e scintille sulla carta appena forgiata, animazione di arrivo, toast per gli errori (ad esempio "massimo 3 copie").
- **Barra del deck**: menu "I miei deck" con le copertine, nome modificabile, salvataggio automatico, annulla e ripeti (Ctrl+Z, Ctrl+Y), esporta, elimina con conferma; formato MD/TCG/OCG, badge di legalità con l'elenco dei problemi e **costo di creazione in Master Duel** per rarità (UR/SR/R/N).
- **Prova con l'Invoked di Siracusa**: in Master Duel il deck risulta legale (costo UR 33, SR 25, R 9, N 3); passando a TCG compare "Triple Tactics Talent: 2 copie, al massimo 1".

### 18.3 Migliorie scelte dopo la prima prova
- **Maybe board**: quarta area per le carte in prova, salvata con il deck, senza limiti di copie, mai validata né esportata.
- **Filtri rapidi e preferiti**: gruppi scelti a mano (`tag:handtrap`, `tag:staple`, `tag:breaker`, in `crates/ygo-data/src/tags.rs`), stella sulle carte (`tag:fav`) e ricerche salvate; preferiti e ricerche stanno in `user.db`.
- **Mano di prova**: 5 carte andando per primi, 6 per secondi, animazione di distribuzione, "pesca una carta"; ruoli Starter e Handtrap salvati con il deck (gli handtrap partono già segnati), con il conteggio delle mani pescate accanto alla probabilità esatta ipergeometrica. È il primo pezzo del Laboratorio statistico (§7.1).
- **Immagine del deck**: PNG 1552 px in stile Fucina (nome, formato, legalità, costo MD, una carta per copia con i badge di banlist), salvata in Download/DuelForge o copiata negli appunti. Le immagini delle carte passano come byte dal backend, così il canvas resta esportabile.

### 18.4 Rimandati
Versioni dei deck e piani di side (§4.4) e pagina Novità: non scelti per ora, restano in lista per le fasi successive.

## 19. Risultati della Fase 3: la Bottega (18/10/2026)

### 19.1 Dati
- Ogni deck ha un `box` (colore, materiale, forma, fino a 3 carte vetrina) salvato in `user.db` (nuova colonna, migrazione automatica). Cambiare l'aspetto non conta come modifica del deck, quindi non cambia l'ordine "Recenti".
- Il comando `shelf_decks` restituisce tutti i deck con la legalità ricalcolata sulle banlist in vigore: se una nuova banlist rende illegale un deck, il sigillo sul box diventa rosso con il numero di problemi.
- Senza carte vetrina scelte, il box mostra le prime tre carte diverse del Main Deck. I deck si possono duplicare.

### 19.2 Interfaccia
- **Scaffali di legno** con i deck box in 3D (trasformazioni CSS 3D: cinque pareti, coperchio incernierato appoggiato all'indietro, carte vetrina in piedi dentro il box). Ogni riga si riempie con quanti box ci stanno.
- **3 forme**: deck box verticale, latta arrotondata, cofanetto con fermaglio e cinghie. **5 materiali**: pelle con cuciture, metallo spazzolato con rivetti, legno, olografico animato, opaco. **12 colori** più un colore libero.
- **Effetti**: al passaggio del mouse il box si solleva e segue il puntatore, il coperchio si apre di più e le carte salgono a ventaglio; suoni sintetizzati nuovi: "knock" di legno quando si sceglie un box, "latch" metallico quando si cambia materiale.
- **Pannello Personalizza**: anteprima grande che oscilla lentamente, nome modificabile, colori, materiali, forme, scelta delle carte vetrina (numerate 1–3), "Apri nella Fucina", "Duplica", "Elimina" con conferma. Le modifiche si salvano da sole.
- Ricerca per nome, filtro per formato, ordine per recenti, nome o formato; "Nuovo deck" porta nella Fucina con un deck vuoto.

### 19.3 Verifica
WebKitGTK sotto Xvfb (l'ambiente di test su Linux) non compone il 3D, quindi i box lì appaiono piatti. La verifica del 3D è fatta con Chromium, lo stesso motore di WebView2 che l'app usa su Windows, rendendo l'interfaccia vera con un backend simulato.

## 20. Piano della Fase 4: il Laboratorio (proposta, 18/10/2026)

Versione illustrata con mockup e grafici: pagina "Il Laboratorio di DuelForge" pubblicata per Valerio. Qui la sintesi.

### 20.1 Domande a cui risponde
Quanto spesso apro bene (per primo e per secondo); quanto resiste la mano a un'interruzione; quante copie mi servono e se conviene andare oltre 40 carte; quanto valgono Pot e ricerche; come vanno le mie partite per matchup; se una modifica al deck ha funzionato e quante partite servono per saperlo.

### 20.2 Interfaccia
Stessa struttura delle altre sezioni: intestazione in Cinzel con l'artwork di Brain Research Lab, accento ciano, griglia millimetrata appena visibile. Barra con il deck (menu con la miniatura del deck box della Bottega), primo/secondo e cinque schede: Panoramica, Calcolatore, Simulatore, Ottimizzatore, Registro partite. Pannello laterale dei ruoli. Indicatori principali come ampolle che si riempiono; grafici SVG animati con tooltip e tabella dati; suoni sintetizzati (bollicine a fine calcolo, tintinnio di vetro).

### 20.3 Ruoli
Starter, Estensore, Handtrap (già segnati dal gruppo Handtrap), Board breaker, Brick/Garnet e ruoli personalizzati; una carta può averne più di uno; salvati con il deck. Colori dei ruoli verificati per il daltonismo in tema scuro e chiaro.

### 20.4 Schede
- **Panoramica**: probabilità esatte (ipergeometrica multivariata sui gruppi di ruoli) di starter, doppio starter, handtrap, mano ideale, mano che resiste a un'interruzione, mano solida, mano bloccata; valori attesi; istogramma delle handtrap in mano; curva "pescare una carta entro N carte viste" per 1/2/3 copie; probabilità per carta; probabilità che l'avversario abbia almeno una handtrap in funzione delle handtrap nel suo deck.
- **Calcolatore**: condizioni composte con ruoli o carte, soglie (almeno, al massimo, esattamente) ed E/O/NON; mani di 5/6 carte più le pescate successive; ricette salvate per deck; passaggio dichiarato al Monte Carlo per condizioni troppo complesse.
- **Simulatore**: Monte Carlo in Rust multi-thread con seed: Pot of Prosperity, Desires, Extravagance, Duality, Upstart Goblin, Allure of Darkness, Triple Tactics Talent; ricerche ("la carta vale come quella che può prendere"), combo a due carte, conflitti di Normal Summon e once-per-turn. Intervallo al 95% e grafico di convergenza in diretta. Obiettivo: un milione di mani in meno di un secondo.
- **Ottimizzatore**: valore di +1/−1 copia per ruolo o carta (tornado), 40 contro 41–45 carte, mappa starter × handtrap, ottimizzatore con vincoli (40–60 carte, 3 copie, banlist, carte bloccate) e obiettivo pesato; le proposte si possono mettere nella Maybe board.
- **Registro partite**: inserimento in pochi secondi (deck e versione, avversario, primo/secondo, game, note, tag); win-rate con intervalli di Wilson e stima bayesiana, forest plot per matchup, matrice matchup × primo/secondo, andamento, game 1 contro side, meta incontrato, CSV; calcolo di quante partite servono (55% contro 50%: circa 783 partite; due versioni a confronto: circa 1.565 per versione); fotografia del deck a ogni partita per confrontare le versioni.

### 20.5 Numeri d'esempio (Invoked di Siracusa, ruoli d'esempio)
Starter 13, Estensore 11, Handtrap 10, Board breaker 3, altro 3. Per primo / per secondo: almeno uno starter 87,7% / 92,3%; almeno una handtrap 78,3% / 84,5%; mano ideale 67,0% / 77,1%; resiste a un'interruzione 84,1% / 90,8%; mano bloccata 12,3% / 7,7%. Una copia di handtrap in più vale +3,3 punti di mano solida per primo; i board breaker non cambiano le mani per primo.

### 20.6 Costruzione e sotto-fasi
Nuovo crate `ygo-stats` (calcolo esatto, Monte Carlo, ottimizzatore, statistiche del registro) con test su valori noti; tabelle `user.db` per partite e ricette; grafici SVG propri.
- **4A** Ruoli, Panoramica e Calcolatore.
- **4B** Simulatore e Ottimizzatore.
- **4C** Registro partite.
Il Monte Carlo guidato dal motore resta nella Fase 7.

## 21. Risultati della Fase 4: il Laboratorio (19/10/2026)

### 21.1 Motore statistico (`ygo-stats`)
- **Calcolo esatto**: ipergeometrica multivariata sui gruppi di carte con gli stessi ruoli; condizioni composte (almeno / al massimo / esattamente, E / O / NON) su ruoli o singole carte. I test riproducono i numeri della §20.5 (starter 87,7% / 92,3%, mano ideale 67,0% / 77,1%, mano solida 63,4% per primo). Le condizioni troppo grandi per il calcolo esatto lo dicono e rimandano al simulatore.
- **Monte Carlo**: xoshiro256** con seed, blocchi da 4.096 mani su più thread; il risultato non cambia con il numero di thread. Effetti: pesca (Desires, Extravagance, Upstart, Allure, Pot of Greed), scava e aggiunge (Prosperity, Duality), ricerca di carte; una sola Normal Summon ed effetti una volta per turno. **Un milione di mani in 0,15–0,3 secondi**, intervallo di Wilson al 95%.
- **Ottimizzatore**: valore esatto di ±1 copia per ruolo e per carta, curva da 40 a 45 carte, mappa di due ruoli, ricerca per simulated annealing che rispetta banlist e numero di carte e può usare la Maybe board. Sul deck di prova propone +8,9 punti sull'obiettivo pesato.
- **Partite**: win-rate con intervallo di Wilson e intervallo bayesiano Beta(1+V, 1+S); test esatto di Fisher; probabilità che la versione nuova sia migliore (campionamento delle due Beta); numero di partite necessarie (55% contro 50%: 783; due versioni: 1.565 ciascuna).

### 21.2 Interfaccia
Cinque schede (Panoramica, Calcolatore, Simulatore, Ottimizzatore, Registro partite) con il pannello dei Ruoli a destra; nel Registro il pannello diventa l'inserimento della partita. Indicatori ad ampolla, grafici SVG propri (barre, linee con mirino, convergenza, tornado, mappa di calore, forest plot, matrice), ogni grafico con tooltip e tabella dei dati; la condizione scelta nel Calcolatore vale anche per Simulatore e Ottimizzatore; le proposte dell'ottimizzatore si applicano al deck spostando le copie tolte nella Maybe board.

### 21.3 Registro partite
- **Inserimento**: avversario (con i più frequenti a un clic), turno, partita singola (come il ranked di Master Duel) o al meglio di 3 con il game 3 che compare sull'1–1, tag, note, data (adesso o un'altra); Ctrl+Invio registra.
- **Versioni**: ogni partita salva la lista giocata (Main, Extra, Side) e la sua impronta; quando il deck cambia nasce la versione successiva (v1, v2...) e il registro mostra le carte cambiate e il confronto tra le ultime due versioni.
- **Statistiche**: win-rate, forma recente e serie, per primo e per secondo (con Fisher), forest plot per avversario, meta incontrato, andamento sulle ultime 20 partite, matrice avversario × turno, game 1 contro game dopo il side, versioni, tag, calcolatore delle partite necessarie, elenco con eliminazione. Vista per un deck o per tutti i deck (allora il confronto è tra deck).
- **Win-rate atteso contro il meta recente**: i matchup pesati su quanto spesso sono usciti nelle ultime 30 partite, con i matchup giocati poco avvicinati alla media. Pesarli su tutto il registro restituirebbe esattamente il win-rate complessivo, quindi non direbbe nulla.
- **CSV**: esportazione in `Download/DuelForge` (UTF-8 per i fogli di calcolo) e importazione da file di DuelForge o fatti a mano: intestazioni in italiano o inglese, separati da virgola, punto e virgola o tabulazione; risultati V/S/P o W/L/D.

### 21.4 Verifica
Test Rust su valori noti (ipergeometrica, Fisher, Beta, numero di partite, impronta delle versioni, CSV). Nell'app vera sotto Xvfb con 74 partite d'esempio su due versioni del deck Invoked: inserimento Bo1 e Bo3, vista di tutti i deck, esportazione e importazione CSV, stato vuoto.

### 21.5 Prossimo passo: Fase 5
Il Combo Lab: il motore cerca da solo le combo a partire da una mano o da una carta e classifica le board finali (§22).

## 22. Piano della Fase 5: il Combo Lab (proposta, 19/10/2026)

Obiettivo: **scegli un deck e una mano (o una carta), il motore gioca da solo in background tutte le linee possibili e ti restituisce le board finali classificate per forza, con ogni passaggio spiegato.** Nessuna partita da giocare a mano.

### 22.1 Cosa chiedi e cosa ottieni
- **Partenza**: un deck salvato e
  - una **mano precisa** (5 carte per primo), oppure
  - **"carta X + resto"**: il programma prova molte mani che contengono X (campionate o, quando sono poche, tutte), pesate per quanto sono probabili, e dice con quale frequenza raggiungi ciascuna board.
- **Vincoli facoltativi**: carte da non usare, carte da tenere in mano (per esempio Ash Blossom), board che deve contenere una certa carta, tempo massimo.
- **Risultato**: le board finali in classifica con punteggio e dettaglio di ogni punto, la linea più corta per raggiungerle, le carte usate, e una riproduzione animata passo per passo da guardare.
- **In background**: la ricerca gira sui thread del PC mentre usi il resto dell'app; vedi le board trovate man mano e ricevi un avviso alla fine. I risultati si salvano per deck e mano.

### 22.2 Il motore pronto per giocare da solo
Il motore di EDOPro applica regole e ruling (lo ha già dimostrato la linea Invoked della Fase 0). Il nostro lavoro è fargli giocare milioni di scelte senza intoppi e velocemente.

**Perché oggi una scelta può "bloccare" una linea.** Quando il motore chiede una scelta si ferma e aspetta una risposta. Per ogni tipo di scelta il programma costruisce l'elenco delle risposte possibili, e la ricerca le prova tutte. Oggi per alcune scelte rare l'elenco resta vuoto o incompleto:
- **dichiarare il nome di una carta** (il motore manda un piccolo programma di filtro che dice quali nomi sono validi: va interpretato e applicato a tutte le carte);
- **distribuire o togliere segnalini** da più carte;
- **dichiarare più Tipi o Attributi** insieme;
- **ordinare carte** (per esempio rimetterle in cima al Deck nell'ordine scelto): oggi si usa solo l'ordine predefinito;
- **sasso-carta-forbice** e altre scelte casuali.

Con un elenco vuoto la linea si interrompe lì in silenzio: non si rompe nulla, ma quella combo e tutte quelle che passavano da lì non vengono trovate.

**Come lo risolvo.**
1. **Gestire tutte le scelte**: interprete del filtro dei nomi (con i nomi sensati da provare: carte del tuo deck, del kit avversario e del meta), enumerazione delle distribuzioni di segnalini e delle combinazioni di Tipi/Attributi, permutazioni quando l'ordine conta (fino a 5 carte, altrimenti le più promettenti), nodi casuali per le scelte casuali.
2. **Rete di sicurezza**: una linea che incontra una scelta non gestita non sparisce, viene segnalata ("linea interrotta: scelta non gestita") così si vede e si corregge; una risposta rifiutata dal motore viene scartata e la ricerca continua; limite di passi e riconoscimento degli stati ripetuti contro i cicli infiniti.
3. **Collaudo a tappeto**: un test che gioca milioni di scelte casuali con centinaia di deck diversi (costruiti dagli archetipi del database) e fallisce se anche una sola scelta resta senza risposte o viene rifiutata. È il modo per trovare i casi rari prima che li trovi tu.

**Il resto del lavoro sul motore.**
- **Stato canonico** completo dei due campi (carte, posizioni, materiali, segnalini, ATK/DEF, effetti negati, LP, fase, Normal Summon usata) e un'**impronta** per riconoscere la stessa board raggiunta in ordini diversi.
- **Scelte irrilevanti risposte da sole** (posizione d'attacco, zona quando le zone sono equivalenti; le frecce Link e l'Extra Monster Zone invece contano) e **macro-azioni** ("attiva e risolvi la catena" come un solo passo).
- **Velocità**: oggi tornare a una scelta precedente ricrea la partita e rigioca le mosse (~20 ms, §16.3). La ricerca in profondità riusa la partita in avanti e la ricrea solo tornando indietro, con una cache delle partite sui punti più visitati e gli script di base precompilati. Le misure guidano il lavoro; uno snapshot nativo della partita resta l'ultima opzione.
- **Registro in italiano** di ogni passo, che diventa la spiegazione della combo.

### 22.3 La ricerca
- **Ricerca in profondità con priorità**: si esplorano prima i rami promettenti secondo una stima veloce del punteggio (letta dagli script: un effetto rapido che nega vale più di uno che pesca), su tutti i core, ciascuno con le sue partite.
- **Niente lavoro doppio**: tabella delle trasposizioni sull'impronta dello stato (la stessa board raggiunta in ordini diversi si esplora una volta), copie identiche della stessa carta trattate come una, azioni indipendenti provate in un solo ordine.
- **Eventi casuali** (pescate, scavi, Pot of Prosperity): più ordini del Deck; una combo che dipende dalla fortuna riporta la sua probabilità di riuscita.
- **Fine del turno**: ogni linea finisce passando il turno, così contano anche gli effetti della End Phase (ricerche, ritorni).
- **Board uguali, una voce sola**: per ogni board finale si tiene la linea migliore (meno carte dalla mano, meno passi).

### 22.4 La classificazione delle board
Il punteggio non è stimato leggendo il testo delle carte: **è misurato dal motore**, mettendo la board alla prova contro un avversario finto, il "turno di prova". Ogni punto ha una spiegazione.

**Il turno di prova.** Dalla board finale si passa il turno a un avversario che ha un kit di carte e azioni standard, e si prova ogni azione separatamente:
- **sonde di gioco**: attiva una Magia dalla mano, un effetto di mostro dalla mano, sul campo e nel Cimitero, una Trappola, una Magia Continua o Terreno; Evocazione Normale; Evocazione Speciale dalla mano, dal Cimitero e dall'Extra Deck; cerca una carta dal Deck; manda una carta dal Deck al Cimitero; attacca;
- **board breaker** veri, con i loro ruling: Dark Hole, Raigeki, Harpie's Feather Duster, Lightning Storm, Evenly Matched, Kaiju, Super Polymerization, Forbidden Droplet, Book of Eclipse, Dark Ruler No More, Triple Tactics Talent (lista aggiornabile con il meta).

Per ogni sonda il motore dice se l'azione è ancora possibile, se puoi rispondere, con quali carte e cosa succede se rispondi (attivazione negata, carta distrutta, bandita, rimandata). Così le condizioni, i costi e i limiti "una volta per turno" sono quelli veri.

**Le metriche** (unità: 1,0 = un'interruzione standard, cioè negare l'attivazione di qualsiasi carta e distruggerla, una volta, senza bersaglio):

| Metrica | Come si misura | Cosa la fa salire |
|---|---|---|
| **Interruzioni** | Quante azioni dell'avversario fermi una dopo l'altra, rispettando costi e limiti; ogni interruzione pesata per effetto, bersaglio e copertura | Più negazioni, senza bersaglio, che colpiscono tutto, usabili più volte |
| **Floodgate** | Sonde che diventano impossibili o limitate rispetto a un campo vuoto, pesate per quanto quell'azione conta nei deck | Blocchi dell'Extra Deck, del Cimitero, delle Evocazioni Speciali, delle ricerche |
| **Resilienza** | Interruzioni che restano dopo ogni board breaker (potendo rispondere), più ciò che torna in campo | Protezioni, risposte ai board breaker, effetti che "fluttuano" |
| **Interazioni dalla mano** | Le handtrap rimaste in mano messe alla prova con le sonde che le attivano | Ash, Impermanence, Nibiru, Droll, Veiler, Mulcharmy rimaste in mano |
| **Risorse** | Carte in mano e sul campo a fine turno, effetti utilizzabili nel Cimitero e tra le bandite, Extra Deck rimasto | Vantaggio di carte, Cimitero carico |
| **Follow-up** | Carte aggiunte in End Phase, starter rimasti (dai ruoli del Laboratorio), ricorsioni per ripartire se la board salta | Poter rifare una board al turno dopo |
| **Costo della linea** | Carte della mano usate, passi, LP pagati, dipendenza dalla Normal Summon | Linee che usano poche carte |
| **Vulnerabilità** (Fase 6) | Punti della linea in cui una sola handtrap ferma tutto | Linee con alternative; peso 0 finché non arriva lo stress test |

**Il valore di un'interruzione** = effetto × bersaglio × copertura × quante volte:
- **effetto**: nega l'attivazione e distrugge o bandisce 1,00; nega l'attivazione 0,90; bandisce 0,85; rimanda nel Deck 0,85; nega l'effetto 0,80; rimanda in mano 0,70; distrugge 0,70; manda al Cimitero 0,70; cambia posizione o controllo 0,40;
- **bersaglio**: senza bersaglio ×1,00, con bersaglio ×0,85;
- **copertura**: la quota di sonde che può fermare, pesata (qualsiasi carta ×1,00; solo effetti di mostri, solo Magie e Trappole, solo Cimitero, solo Evocazioni: la loro quota);
- **velocità**: una Counter Trap, che risponde a tutto, riceve un piccolo bonus;
- **quante volte**: ogni uso disponibile conta di nuovo. **Più interruzioni hai, più sale il punteggio**: niente rendimenti decrescenti, salvo che tu li attivi.

**Il punteggio** è la somma pesata delle metriche, mostrata come barra divisa per colore con il dettaglio di ogni voce ("Board A: nega l'attivazione di un effetto di mostro, senza bersaglio, 2 volte → 1,6"). Pesi predefiniti: Interruzioni 1,0; Floodgate 0,8; Interazioni dalla mano 0,9; Resilienza 0,6; Follow-up 0,4; Risorse 0,3; Costo −0,1 per carta della mano oltre la prima. A parità di punteggio: meno carte usate, linea più corta, più carte in mano.

**Oltre la classifica.** La vista "frontiera" mostra le board che nessun'altra supera in tutto: la board con più negazioni, quella più resistente, quella che lascia più risorse; si possono confrontare due board fianco a fianco.

**Personalizzabile.**
- **Profili**: Bilanciato (predefinito), Più negazioni possibili, Controllo e floodgate, A prova di board breaker, Risorse e follow-up, Contro il mio meta.
- **Pesi**: un cursore per ogni metrica e una tabella dei valori delle interruzioni; scelta delle sonde e dei board breaker considerati; rendimenti decrescenti attivabili.
- **Contro il mio meta**: i deck che incontri di più nel Registro partite decidono quali azioni avversarie contano di più (per esempio, contro deck che vivono di Extra Deck un blocco dell'Extra Deck vale di più).
- **Insegnagli i tuoi gusti**: quando secondo te una board è migliore di un'altra lo indichi; dopo una ventina di confronti i pesi si adattano (modello di Bradley-Terry) e vedi come sono cambiati.
- Le metriche restano salvate per ogni board, quindi **cambiare i pesi riordina subito** senza rifare la ricerca.

**Quanto è "perfetto".** Nessuna formula è perfetta in assoluto; questa punta a esserlo dove conta: le metriche sono misurate dal motore con i ruling veri, ogni punto è spiegato, i pesi predefiniti si tarano su un insieme di board note dei deck del meta ordinate da giocatori esperti (obiettivo: almeno 85% di accordo sulle coppie), e i tuoi confronti li adattano a te. Nella Fase 6 il turno avversario simulato con veri deck del meta darà una verifica sul campo: la probabilità che l'avversario non riesca a passare la board.

### 22.5 Interfaccia (il Circolo Arcano)
Tema ametista della sezione, intestazione con Aleister the Invoker. A sinistra: deck, mano (carte cliccabili dal deck) o "carta + resto", vincoli, profilo di punteggio, pulsante Cerca. Al centro: avanzamento della ricerca (stati esplorati, board trovate, la migliore finora) e la classifica con le barre del punteggio. A destra: la board selezionata sul campo, il dettaglio del punteggio e la linea passo per passo con riproduzione animata (carte che si spostano, catene numerate, registro in italiano).

### 22.6 Sotto-fasi
- **5A Motore per la ricerca**: tutte le scelte gestite, rete di sicurezza, collaudo a tappeto, stato canonico e impronta, risposte automatiche e macro-azioni, velocità misurata, registro in italiano. Pronto quando il collaudo gioca un milione di scelte con almeno 200 deck senza blocchi.
- **5B Ricerca in background**: mano precisa e "carta + resto", vincoli, trasposizioni e simmetrie, eventi casuali, risultati in diretta, salvataggio. Pronto quando, dalla mano della Fase 0, ritrova da sola la linea Invoked nota (e altre) entro un minuto.
- **5C Punteggio e spiegazione**: turno di prova, metriche, profili, pesi, confronti, frontiera, interfaccia dei risultati con la riproduzione passo per passo. Pronto quando ogni punto è spiegato e l'ordine delle board di riferimento rispetta l'obiettivo di accordo.

### 22.7 Scelte da confermare
1. Per ora solo andando **per primo**; per secondo (rompere una board avversaria) nella Fase 6 insieme alle handtrap.
2. I pesi predefiniti del profilo Bilanciato (§22.4).
3. Il kit del turno di prova: le sonde e i board breaker elencati, aggiornabili con il meta.

