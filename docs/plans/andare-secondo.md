# Andare secondo: quanta board del meta rompe la mia mano (24/10/2026)

Idea 5 di `docs/plans/idee-nuove.md`; PIANO §22.7 punto 1. Prima PR: il prototipo nella CLI, senza interfaccia. È una **misura nuova, separata da quella del primo turno**: la ricerca e il turno di prova del primo turno non cambiano.

## Com'è fatto
- **Comando:** `duelforge-cli break-board --deck <mio.ydk> --vs <suo.ydk> [--hands 10] [--rival-hands 12] [--nodes 4000] [--rival-nodes 4000] [--top 5] [--json <file>]`. Codice in `crates/ygo-solver/src/second.rs`.
- **La board avversaria.** Il deck avversario cerca la sua linea migliore da `--rival-hands` mani casuali da 5, come il Combo Lab (contro il kit standard). Ogni board si misura con il turno di prova, dalla sua parte: campo più floodgate a 0,4, come il profilo «Bilanciato», con la sua mano rimasta (Mechaba scarta una carta). Si tiene la board che vale di più.
- **Quanto vale ogni sua carta.** Per ogni carta del suo campo si rimisura la board senza quella carta: la differenza (almeno 0,3, un corpo) è quanto vale toglierla.
- **Il mio turno.** La board si piazza sul campo avversario all'inizio del duello (`Opponent::placed` nel motore, con materiali ed equipaggiamenti come per le board piazzate dalla mia parte); restano alla sua parte la mano, il Deck e l'Extra Deck di fine turno. La mia mano da 6 gioca con la ricerca che c'è. La stima conta le sue carte tolte dal campo al valore sopra (`SearchConfig::opponent_field`).
- **Il valore della mano.** Le prime `--top` board della ricerca: per ognuna si rimisura ciò che resta del suo campo, Cimitero e bandite. Vale la quota più bassa del valore di partenza (1 se la mano non fa nulla).
- **Breaker.** Il riepilogo divide le mani tra quelle con almeno un breaker che il motore sa giocare (`Rival::breakers`, gli stessi del livello 1) e quelle senza.

## Numeri (seme 1, 12 mani per coppia, 4000 nodi; circa 40 s per coppia su 4 core)
| Mio deck | Contro | Board avversaria | Resta con breaker | Resta senza |
|---|---|---|---|---|
| Sky Striker | Invoked (fixture MD) | 1,49 (Aiwass, Invocation) | 9% (10 mani) | 60% (2 mani) |
| Branded | Snake-Eye Rescue-ACE | 1,21 (Red-Eyes Dark Dragoon, Azamina, Masquerena) | 0% (3 mani) | 28% (9 mani) |
| Yummy | Labrynth | 0,57 (Lady Labrynth, Muckraker, Daruma Cannon, Trap Trick) | 57% (7 mani) | 80% (5 mani) |

Criterio della PR rispettato: in tutte e tre le coppie le mani con breaker lasciano meno board. Kashtira contro Labrynth è stata scartata come coppia perché il motore non trova breaker nel deck Kashtira (resta in media il 42%). Esempio di linea: Branded fonde Fallen of Albaz con Red-Eyes Dark Dragoon e Azamina dell'avversario (The Dragon that Devours the Dogma), che è il modo vero di rompere quella board.

## Limiti (da sapere leggendo i numeri)
1. **La board avversaria non risponde nel mio turno**: valgono solo i suoi effetti continui (floodgate). Dragoon non nega, Lady Labrynth non attiva le trappole. Un avversario che usa subito il primo effetto offerto è stato provato e tolto: si distrugge da solo (Super Polymerization con i suoi mostri). Prossimo passo: risponde solo con gli effetti che la misura ha trovato come interruzioni, nei momenti del turno di prova.
2. **Niente Battle Phase**: il mio turno è il primo del duello, quindi non si attacca. Le negazioni fino a fine turno (Forbidden Droplet, Infinite Impermanence) lasciano la board intatta e valgono 0, mentre per secondo servono a fare OTK. Serve un turno 2 vero con la Battle Phase e una misura che conti anche il danno.
3. Le carte piazzate perdono lo stato del turno (effetti usati, contatori), come le board piazzate del primo turno.
4. La board avversaria è una sola (la migliore di 12 mani), misurata contro il kit generico.

## Prossimi passi
- Risposte dell'avversario dalle interruzioni misurate (limite 1).
- Turno 2 con Battle Phase e danno (limite 2); poi una tabella «per secondo» per il Laboratorio e il banco della misura (40 mani × 6 deck del meta).
- Interfaccia: dopo i due punti sopra, con un mockup.
