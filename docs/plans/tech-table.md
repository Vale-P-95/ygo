# Tabella delle tech contro il meta (24/10/2026)

Idea 1 di `docs/plans/idee-nuove.md` (PR #152). Per ogni handtrap del Main e del Side dell'utente: quanto toglie alle linee vere dei deck più giocati del meta. **Non cambia la misura** del Combo Lab.

## Come funziona
- **Le linee del meta** sono quelle del livello 2 (`meta_rivals`, come `combo_meta`): i 6 deck con il `popRank` più alto su Master Duel Meta, 12 mani a caso ciascuno, la linea migliore di ogni mano a 1000 nodi.
- **Le candidate** sono le handtrap lette dal motore (`hand_traps`) nel Main e nel Side: nessun nome di carta nel codice.
- **Il rigioco** (`line_stops` in `matchup.rs`) è lo stesso di `matchup` girato: il deck del meta va per primo e rigioca la sua linea; l'utente tiene in mano tutte le candidate, e ognuna si prova a ogni momento in cui il motore la lascia usare (fino a 24). Il Deck dell'utente è tagliato alla misura del Deck del kit, come in `replay_opponent`.
- **Le misure** (`tech_table` in `meta.rs`), ogni deck pesato per il suo `popRank`:
  - *Board tolta*: la quota del campo e della mano finali della linea che manca usando la carta al suo momento migliore (il primo dei migliori; un rigioco fallito toglie tutto). La tabella è ordinata così.
  - *Linee*: quota di linee di cui cambia la fine (lo `stops` di `matchup`).
  - *Passo*: mosse della linea già giocate in quel momento migliore, in media; e la carta a cui risponde più spesso.
- Una linea che senza handtrap non si rigioca uguale non si conta (Elfnote: 2-3 su 12).

## Dove si usa
- CLI: `duelforge-cli tech-table --deck <deck.ydk> [--decks 6] [--seed 1] [--json <file>]`; il meta da Master Duel Meta, o da `--meta-file`/`--meta-decks` come `combos --meta-final`.
- App: Laboratorio → Altro → «Contro il meta», pulsante «Calcola» (`api.techTable(deckId)`, comando `tech_table`). Le linee del meta si leggono una volta per sessione e si condividono con `combo_meta` quando il seme è lo stesso (`meta_rivals_for` in `lib.rs`); l'ultima tabella resta finché le carte del deck non cambiano.
- Anteprima: `src/lib/fixtures/tech-table.json`, registrata dal CLI su `invoked-siracusa-md` con `scripts/tech_fixture.py`. Screenshot in `docs/screens/tech-table/`.

## Numeri (4 core, meta del 24/10: Elfnote, Dracotail, Invoked, Branded, DoomZ, Elfnote Kewl Tune)
| Deck | Seme | Prime tre (board tolta) | Tempo |
|---|---|---|---|
| invoked-siracusa-md | 1 | Ash Blossom 60%, Ghost Ogre 54%, Effect Veiler 49% | 100 + 77 s |
| invoked-siracusa-md | 2 | Ash Blossom 65%, Ghost Ogre 57%, Effect Veiler 55% | 64 + 36 s |
| invoked-siracusa-ewcq2026 | 1 | Ash Blossom 60%, Ghost Ogre 54%, Dimension Shifter 54% | 104 + 95 s |
| invoked-siracusa-ewcq2026 | 2 | Ash Blossom 65%, Ghost Ogre 57%, Dimension Shifter 56% | 92 + 63 s |
| Maliss (MDM) | 1 | Ash Blossom 60%, Infinite Impermanence 49%, Effect Veiler 49% | 119 + 60 s |
| Maliss (MDM) | 2 | Ash Blossom 65%, Infinite Impermanence 55%, Effect Veiler 55% | 62 + 34 s |

Il primo tempo è la lettura delle linee del meta (una volta per sessione nell'app), il secondo il rigioco. Su due semi l'ordine delle prime tre è lo stesso nei tre deck (criterio di accettazione). In coda: Droll & Lock Bird 34% (quasi nulla contro Dracotail, 1 linea su 9), Maxx "C" e le Mulcharmy 1%, perché fanno pescare e non tolgono nulla dalla board.

## Limiti
- I numeri di una handtrap dipendono dal meta, non dal deck dell'utente: il deck sceglie le candidate e le copie («In mano»). Due carte dello stesso tipo d'effetto danno numeri uguali (Infinite Impermanence ed Effect Veiler): il motore le usa dalla mano negli stessi momenti.
- Le linee del meta sono quelle della nostra ricerca a 1000 nodi: una ricerca debole dà linee deboli (stesso limite di `combo_meta`).
- «Board tolta» conta le carte, non il loro valore: togliere un mostro chiave pesa come togliere una carta qualsiasi della mano finale. Un passo successivo è pesarle con il valore d'interruzione del turno di prova.
- Il confronto a mano di 5 righe con `probe-line --vs --stops` non è fatto: le linee del meta non si salvano come file di `combos --save-lines`. Il rigioco è però lo stesso codice di `matchup` (`hand_trap_stops`), con i ruoli scambiati.

## Prossimo passo
Il piano di side (idea 3 di `idee-nuove.md`) usa questa tabella per la colonna «dentro».
