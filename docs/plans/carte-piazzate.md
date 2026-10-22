# Carte piazzate e ruoli (corsia 4, PIANO §30.4)

Branch `claude/placed-cards`. Tre approssimazioni delle «carte piazzate», cioè le carte messe sul campo senza giocare il turno che le porta lì (`Placement` in `crates/ygo-engine/src/duel.rs`).

## Cosa cambia

### 1. L'estensore si prova nello stesso turno dello starter
Prima la board dello starter veniva piazzata in un turno nuovo: l'Evocazione Normale e gli effetti «una volta per turno» già usati dalla linea tornavano disponibili, e un estensore poteva sembrare migliore di quanto è.

Ora (`roles.rs`):
- si rigioca la linea migliore dello starter fino alla sua ultima scelta libera, quella che chiude la Main Phase (stessa mano, stesso seme: le risposte si rigiocano identiche);
- l'estensore entra in mano dal Deck senza eventi: niente pescata, niente «aggiunta alla mano», niente mescolata (`Prelude::joining`, `shim/hand.cpp`);
- la ricerca continua da lì (`SearchConfig.prelude`, `Duel::goldfish_against_from`) e si confronta con la stessa continuazione senza l'estensore. Quello che la linea ha usato resta usato.

La modifica a `search.rs` è solo additiva (il campo, `new_duel`, e nessun apprendimento con un prelude, come con le carte piazzate), d'accordo con il coordinatore.

### 2. Equipaggiamenti piazzati con il mostro equipaggiato
`Placement.equip_to` dice a quale zona mostri va la carta. Senza, una carta Equipaggiamento scoperta va al mostro piazzato a cui si può equipaggiare (`CheckEquipTarget`, cioè il limite di equipaggiamento della carta) con più ATK. Se non ce n'è nessuno resta com'era. Si usa `Debug.PreEquip` della libreria Debug del motore.

`card-values` ora misura anche le Magie Equipaggiamento, scoperte su un mostro che non fa nulla (Alexandrite Dragon), quando si possono equipaggiare a quel mostro.

### 3. Mostri dell'Extra Deck piazzati con i materiali
`Placement.materials` sono i materiali dell'Evocazione: per ognuno una carta piazzata con quel codice nel Cimitero, bandita o attaccata al mostro (ognuna usata una volta), altrimenti una nuova nel Cimitero. Uno Xyz senza `materials` ha come materiali quelli attaccati.

Avere i materiali non basta: Apollousa fissa il suo ATK quando viene Evocata. Così, dopo i materiali, si gioca quello che l'Evocazione fa per regola:
- i controlli dei materiali (`EFFECT_MATERIAL_CHECK`);
- gli effetti continui «quando viene Evocato Specialmente».

Gli effetti a innesco (opzionali o obbligatori) no: li ha già usati la linea che ha fatto la board.

Quando i materiali veri non si sanno, `generic_materials` dà materiali generici validi: tanti quanto il Link Rating per un Mostro Link, due per gli altri, Mostri Normali con nomi diversi. `card-values` li usa: Apollousa piazzata ha 3200 ATK invece di 0.

Limite rimasto: gli effetti «se usato come materiale» delle carte materiale non si giocano (vogliono la carta Evocata come `GetReasonCard`, che dal Lua non si imposta).

## Verifiche
- Test nuovi in `crates/ygo-engine/tests/placed.rs`: equipaggiamento al mostro valido (anche con un limite, «solo a un Guerriero»), al bersaglio dato, nessun equipaggiamento senza un mostro valido; Apollousa con 0, 4 e 2 materiali (presi dal Cimitero quando ci sono); un turno che continua con una carta in più e l'Evocazione Normale già usata.
- Gli stessi test con `DUELFORGE_CHECK_REUSE=1` (il duello riusato gioca come uno nuovo) e con `DUELFORGE_FRESH_DUELS=1`.

## Numeri prima e dopo
Prima: `main` (8b367cd). Dopo: questo branch. Le liste di Master Duel Meta sono quelle del 22/10: 41 deck per la caccia, `--limit 300` per il banco e per `card-values` su tutti i deck. I tempi sono presi con altro che girava sulla macchina.

### Banco (`bench.py`, 16 mani, 4000 nodi)
Identico: somma delle board migliori 16,15 prima e dopo, delle prime 14,70 prima e dopo, stessi nodi, replay e board per ogni mano. Quattro mani `inv-*` non partono né prima né dopo: la lista Invoked di oggi non ha alcune carte di quelle mani.

Il banco non cambia perché i valori delle carte delle sue mani non cambiano (vedi sotto), e la ricerca usa le carte piazzate solo tramite quei valori.

### Valori delle carte (`card-values`, 87 deck)
Un solo valore diverso: **Psychic Blaster Mk-II** da 0 a 0,32. Il suo effetto rapido bandisce un mostro dal Cimitero come costo, e ora i suoi materiali generici sono lì, come dopo una vera Evocazione Synchro.

Si aggiungono le Magie Equipaggiamento (DoomZ Command, Tails of the Fairy Tails, Dracoback, Infernoble Arms - Durendal, le Bamboo Sword, Phoenix Gearblade): valgono tutte 0. O non fermano nulla da sole, o non si possono equipaggiare al mostro di prova (Durendal vuole un Guerriero).

Apollousa non è in nessuna di queste liste. Il suo valore viene dalla negazione, non dall'ATK: lo dice il test del motore.

### Caccia (`combo_hunt.py --seed 7`, 41 deck × 2 mani)
82 righe ok e 0 problemi, prima e dopo. Cambia solo **psychics-0**, per il valore di Psychic Blaster: la board migliore misurata passa da campo 0,41 · mano 1,1 · resilienza 0,16 a campo 0,62 · mano 0,87 · resilienza 0,34. Con i pesi del banco fa 1,50 → 1,61.

### Ruoli (`roles`, 12 deck)
Gli starter restano gli stessi (83). Gli estensori scendono da 44 a 23, i mattoni salgono da 17 a 23.

| deck | estensori prima | dopo |
|---|---|---|
| branded | 6 | 3 |
| code_talker | 4 | 3 |
| invoked | 0 | 0 |
| maliss | 2 | 0 |
| mitsurugi_yummy | 0 | 1 |
| phantom_knights | 3 | 2 |
| ryu_ge | 2 | 2 |
| sky_striker | 3 | 4 |
| snake_eye | 11 | 3 |
| tearlaments | 7 | 3 |
| white_forest_azamina | 1 | 2 |
| yubel | 5 | 0 |

Escono le carte che contavano sull'Evocazione Normale o sugli effetti già usati dallo starter:
- Yubel e Spirit of Yubel, che vogliono tributi e un'Evocazione Normale;
- Speedroid Taketomborg e Silent Boots per Backup @Ignister;
- Snake-Eyes Flamberge Dragon, Diabellstar ed Elfnote per Magicians' Souls.

Alcune di queste diventano mattoni: da sole non si evocano, e nello stesso turno non allungano più.

Entrano le carte che servono a turno avanzato:
- Yummy☆Surprise per Cupsy☆Yummy;
- Phantom Knights' Fog Blade, Divine Temple of the Snake-Eye e Dramatic Snake-Eye Chase;
- Tearlaments Grief;
- Upstart Goblin.

Prima queste carte perdevano contro la board ricreata con un'Evocazione Normale libera.

Controllo del meccanismo, su yubel: senza estensore, la continuazione dello starter (Samsara D Lotus) ritrova la stessa stima della sua linea (12,39). Con Super Polymerization in mano sale a 12,99, con Eternal Favorite a 13,39.

Tempi di `roles`: da 7–21 s a 6–46 s per deck. Il massimo è tearlaments: la ricerca riparte da una linea lunga, e ogni nodo la rigioca.
