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
(da completare)
