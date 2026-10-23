# Caccia ai bug e alle inefficienze del motore delle combo (bozza)

*Corsia 9, 23/10/2026, branch `claude/engine-bughunt` da `main` 9cc3005. Lettura del codice di `crates/ygo-solver`, `crates/ygo-engine` e della parte `combos` di `crates/ygo-cli`, più riproduzioni con la CLI (build release, dati del 22/10, liste MDM del 22/10). Nessuna modifica al codice. **Bozza in corso:** le voci sotto sono quelle già verificate; l'ordine per gravità e il resto arrivano nelle prossime spinte.*

## Verificate finora

### 1. La tabella delle trasposizioni chiude un nodo alla prima visita, con priorità che dipendono dal cammino (gravità media; già assegnata alla corsia 2)
- `search.rs`, `Shared::first_visit` e il suo uso in `play` (dopo il calcolo di `node_key`): il primo cammino che arriva a un nodo lo chiude; i figli entrano in coda con `discrepancies = edge.discrepancies + 1` di quel cammino (`work`). Un cammino con meno deviazioni che arriva dopo finisce come `End::Duplicate` e non riaccoda nulla.
- **Scenario:** nodo N raggiunto prima da una linea a 3 deviazioni → i suoi fratelli in coda a penalità 4 × 0,6 = 2,4. La linea principale (0 deviazioni) arriva a N più tardi e viene scartata: i fratelli restano a 2,4 invece di 0,6. A budget finito è una perdita di ordine, cioè di board.
- **Correzione:** la voce della tabella tiene la penalità minima vista; un arrivo con penalità minore riaccoda i figli (B3). **Controllo:** contare in `DUELFORGE_SEARCH_STATS` i nodi raggiunti una seconda volta con `discrepancies` minore (oggi ignoto); poi `best` del banco non scende e il tempo alla board migliore cala.

### 2. La board che rappresenta un campo cambia con il budget: una fonte misurata della non monotonia (gravità alta per l'oracolo e per la classifica)
- `crates/ygo-cli/src/main.rs`, `combos`: le board si raggruppano per `field_key` (solo le carte sul campo: mano, zone, Cimitero, Extra Deck ignorati). Il rappresentante del gruppo è la **prima board nell'ordine della ricerca** (stima rapida, poi carte usate, poi lunghezza); la misura, economica o intera, si fa sul rappresentante. Le altre varianti si misurano solo per i primi 3 campi, e solo 2 per campo (`FIELDS_WITH_VARIANTS`, `MEASURED_VARIANTS`).
- Con più nodi un campo accumula varianti e il rappresentante cambia. **Riproduzione** (lista Sky Striker MDM del 22/10, mano della fixture `hunt-sky_striker-1`, seme 7, 1 thread, `--top 5000`): a 1000 nodi 41 campi, a 20 000 nodi 76; dei 22 campi comuni, **6 cambiano rappresentante** (mano diversa) tra i due budget. Esempio: il campo «Zero [EMZ] + Widow Anchor» ha a 1000 nodi la mano (Raye, Ash, Ash), a 20 000 nodi (Raye, Ash, Raye, Ash, Engage) con 105 varianti. La misura vede una board diversa a ogni budget, e una variante che misura meglio può non essere mai misurata.
- La lista Sky Striker di oggi non è quella delle fixture (hash diverso: nelle mani di oggi Maxx "C" non c'è), quindi il caso esatto del piano non si rigioca qui; il meccanismo sì.
- Altre tre scelte a dimensione fissa che dipendono dal budget nello stesso modo: `confirm_top` 40 e `zone_top` 20 in `resume` (`confirm_best`, `zone_variants`), e il taglio a 300 campi di `oracle.py --staged`.
- **Correzione:** il rappresentante di un campo si sceglie con la misura economica tra le sue varianti (o la misura economica si fa per `look_key`, campo + mano, e si raggruppa dopo); le fasi dopo la ricerca si dimensionano in proporzione alle board trovate. **Controllo:** sulle 91 mani dell'oracolo, mani con valore in calo di oltre l'1% a budget crescente da 28 a meno di 5.

### Ipotesi controllate e scartate
- `ygo_engine::arena_duels` è thread-local (`ARENAS.with`): nessuna gara tra worker nella creazione del duello in arena.
- `max_choices` 60 non limita le linee di Snake-Eye: mano della fixture `hunt-snake_eye-0`, 3000 nodi, linea più lunga 45 scelte.
- `df_lua_alloc`: il ramo «stessa classe di dimensione» viene dopo il caso `nsize == 0`, quindi non perde le liberazioni.
