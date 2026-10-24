# Laboratorio: carte speciali

## Decisione (24/10/2026)
- Una scheda nuova del Laboratorio, «Carte speciali», dice di quanto ogni carta che pesca, scava, fa da ponte o cerca alza la probabilità di avere in mano un bersaglio (un ruolo o una carta).
- I calcoli conoscono meccanismi generali, mai nomi di carte: quale carta ha quale meccanismo è un dato, in `crates/ygo-stats/src/special.json`. Una carta nuova si aggiunge lì; un meccanismo nuovo solo se nessuno di quelli che ci sono la descrive.
- Tutto è esatto (gruppi di carte con le stesse proprietà, come `exact::probability`), provato nei test contro la forza bruta su tutte le sequenze di pescate di un deck piccolo.
- La carta che si attiva conta una volta sola: il bersaglio è in mano, oppure c'è la carta speciale e il suo effetto lo raggiunge. Le carte non si sommano tra loro (per due carte insieme c'è il Simulatore).
- «Prova una carta» aggiunge copie solo al calcolo (il deck resta com'è), per vedere se conviene metterla.

## Meccanismi
| Meccanismo | Che cosa fa | Carte (in `special.json`) |
|---|---|---|
| `draw` | bandisce `banishTop` carte dalla cima, poi pesca `n`; `minExtra` carte nell'Extra Deck come costo | Pot of Desires (10, 2), Pot of Extravagance (2, almeno 3 nell'Extra), Pot of Greed, Upstart Goblin |
| `excavate` | guarda le prime `n` (una delle scelte) e ne aggiunge 1; con `banishExtra` bandisce prima tante carte dall'Extra Deck | Pot of Prosperity (3 o 6), Pot of Duality (3) |
| `drawThenBanish` | pesca `n`, poi bandisce dalla mano una carta che rispetta `cost`; se non c'è, perde la mano | Allure of Darkness (mostro DARK) |
| `bridge` | mostro in mano → mostro del Deck con esattamente `share` proprietà in comune (Tipo, Attributo, Livello, ATK, DEF) → mostro da aggiungere, con `share` in comune col secondo | Small World (1) |
| `search` | aggiunge una carta del Deck che rispetta `filter` | Reinforcement of the Army (Warrior di Livello 4 o meno), Terraforming (Magia Terreno) |
| `revealSearch` | rivela un mostro dell'Extra Deck col nome in `reveal`, aggiunge un mostro col Livello pari al suo Rango e lo stesso Tipo o Attributo | Seventh Tachyon (Number 101-107) |

## Che cosa mostra la scheda
- Per ogni carta: probabilità di aprirla, probabilità di avere il bersaglio con la carta e la differenza in punti rispetto a «senza aiuti»; per Pot of Desires il rischio di bandire una copia del bersaglio; per Allure la probabilità di perdere la mano e le carte che pagano il costo; per Prosperity una riga per 3 e per 6; per le ricerche che cosa aggiungono.
- Per Small World la mappa dei ponti: verso il bersaglio, oppure da un mostro scelto, con i mostri in mezzo.
- Comandi `lab_special` (calcolo) e `lab_special_known` (le carte conosciute col passcode); nel backend finto lo stesso calcolo in `mock.ts`, esatto tranne la pesca con costo, che è una stima.

## Limiti
- Ponte: il mostro in mezzo si assume ancora nel Deck (sbaglia solo se tutte le sue copie sono in mano). Un ATK o DEF `?` non vale come proprietà in comune.
- Allure: il costo si paga con una carta DARK qualsiasi della mano, anche se servirebbe ad altro.
- Seventh Tachyon: la carta rimessa in cima non si toglie dalla mano.
- Nessuna somma tra carte speciali diverse.

## Verifica
- `cargo test -p ygo-stats special`: 6 test (pescate e scavi, costo dell'Extra Deck, pesca con costo, ponti, ricerche, `special.json`).
- App vera sotto Xvfb con dati reali e deck di Master Duel Meta: Kashtira (Small World, Reinforcement), Orcust (Allure), Exosister (Seventh Tachyon), Sky Striker (Desires, Upstart, Reinforcement), Radiant Typhoon (Extravagance); screenshot in `docs/screens/carte-speciali/`. Un ponte di Kashtira ricontrollato sui dati: Ash Blossom → Kashtira Riseheart (solo FIRE) → Evilswarm Heliotrope (solo Livello 4).

## Prossimi passi
- Più carte in `special.json`.
- Il Simulatore legge le stesse associazioni invece della sua tabella `KNOWN`.
- Rifinitura con un mockup di Claude Design.
