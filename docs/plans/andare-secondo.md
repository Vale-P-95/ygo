# Andare secondo: quanta board del meta rompe la mia mano (24/10/2026)

Idea 5 di `docs/plans/idee-nuove.md`; PIANO §22.7 punto 1. Prima PR: il prototipo nella CLI, senza interfaccia. È una **misura nuova, separata da quella del primo turno**: la ricerca e il turno di prova del primo turno non cambiano.

## Com'è fatto
- **Comando:** `duelforge-cli break-board --deck <mio.ydk> --vs <suo.ydk> [--hands 10] [--rival-hands 12] [--nodes 4000] [--rival-nodes 4000] [--top 5] [--json <file>]`. Codice in `crates/ygo-solver/src/second.rs`.
- **La board avversaria.** Il deck avversario cerca la sua linea migliore da `--rival-hands` mani casuali da 5, come il Combo Lab (contro il kit standard). Ogni board si misura con il turno di prova, dalla sua parte: campo più floodgate a 0,4, come il profilo «Bilanciato», con la sua mano rimasta (Mechaba scarta una carta). Si tiene la board che vale di più.
- **Quanto vale ogni sua carta.** Per ogni carta del suo campo si rimisura la board senza quella carta: la differenza (almeno 0,3, un corpo) è quanto vale toglierla.
- **Il mio turno.** La board si piazza sul campo avversario all'inizio del duello (`Opponent::placed` nel motore, con materiali ed equipaggiamenti come per le board piazzate dalla mia parte); restano alla sua parte la mano, il Deck e l'Extra Deck di fine turno. La mia mano da 6 gioca con la ricerca che c'è. La stima conta le sue carte tolte dal campo al valore sopra (`SearchConfig::opponent_field`).
- **Il valore della mano.** Le prime `--top` board della ricerca: per ognuna si rimisura ciò che resta del suo campo, Cimitero e bandite. Vale la quota più bassa del valore di partenza meno gli LP tolti diviso 8000 (1 se la mano non fa nulla, 0 per un letale; dal 24/10, con la Battle Phase).
- **Le risposte della board** (24/10, limite 1). Nel mio turno l'avversario usa solo gli effetti che la misura della sua board ha trovato come interruzioni (`RivalBoard::responders`, il più prezioso per primo, in `AutoPolicy::opponent_responders`): a ogni occasione di rispondere attiva il primo della lista tra quelli offerti; nel suo anello della catena (attivazione e risoluzione) prende per prime le mie carte e non annulla mai; il resto è passivo come prima. `--passive` torna alla board che non risponde, per il confronto.
- **Breaker.** Il riepilogo divide le mani tra quelle con almeno un breaker che il motore sa giocare (`Rival::breakers`, gli stessi del livello 1) e quelle senza.

## Numeri (seme 1, 12 mani per coppia, 4000 nodi; circa 40 s per coppia su 4 core)
| Mio deck | Contro | Board avversaria | Resta con breaker | Resta senza |
|---|---|---|---|---|
| Sky Striker | Invoked (fixture MD) | 1,49 (Aiwass, Invocation) | 9% (10 mani) | 60% (2 mani) |
| Branded | Snake-Eye Rescue-ACE | 1,21 (Red-Eyes Dark Dragoon, Azamina, Masquerena) | 0% (3 mani) | 28% (9 mani) |
| Yummy | Labrynth | 0,57 (Lady Labrynth, Muckraker, Daruma Cannon, Trap Trick) | 57% (7 mani) | 80% (5 mani) |

Criterio della PR rispettato: in tutte e tre le coppie le mani con breaker lasciano meno board. Kashtira contro Labrynth è stata scartata come coppia perché il motore non trova breaker nel deck Kashtira (resta in media il 42%). Esempio di linea: Branded fonde Fallen of Albaz con Red-Eyes Dark Dragoon e Azamina dell'avversario (The Dragon that Devours the Dogma), che è il modo vero di rompere quella board.

### Con le risposte (24/10; stesse coppie, seme 1, 12 mani, 4000 nodi; liste MDM scaricate il 24/10, board avversarie identiche)
| Mio deck | Contro | Risponde con | Con breaker: passiva → risponde | Senza breaker: passiva → risponde |
|---|---|---|---|---|
| Sky Striker | Invoked (fixture MD) | Mechaba, Ash Blossom, Invocation, Raidjin | 9% → 22% (10 mani) | 60% → 56% (2 mani) |
| Branded | Snake-Eye Rescue-ACE | Red-Eyes Dark Dragoon, Masquerena, Maxx "C" | 0% → 9% (3 mani) | 28% → 59% (9 mani) |
| Yummy | Labrynth | Back Jack, Maxx "C", Daruma Cannon | 57% → 72% (7 mani) | 80% → 83% (5 mani) |

La board resta di più in tutte e tre le coppie, e le mani con breaker lasciano ancora meno di quelle senza. Ricerche più corte quando la board nega subito (Dragoon sulla prima Magia di Branded: 13 nodi, resta tutto). Tre mani Sky Striker scendono invece (100% → 13%, due volte 20% → 0%): le risposte costano all'avversario (Mechaba scarta dalla mano) e la misura dopo il turno lo vede. Alcune mani Yummy superano il 100% (fino al 149%): Back Jack e Maxx "C" nel mio turno lasciano all'avversario più carte di prima; la quota non è tagliata a 1. Un ciclo infinito trovato nella prova (l'avversario annullava la scelta dei materiali di Invocation e il motore gliela riproponeva) è chiuso così: nel suo anello non annulla mai, e dopo 200 sue decisioni in una sola avanzata torna passivo.

### Con la Battle Phase (24/10; stesse coppie, seme 1, 12 mani, 4000 nodi, con le risposte; liste MDM scaricate il 24/10 sera)
Il mio turno ora viene dopo il suo, con la Battle Phase, e il valore conta anche gli LP tolti. Il motore dà sempre il turno 1 al giocatore 0, quindi il duello parte con due turni vuoti senza pescate (il mio, poi il suo) e il mio turno è il 3 (`Opponent::went_first`). Valore della mano: quota della board che resta meno gli LP tolti diviso 8000 (peso 1, `LP_WEIGHT`), non sotto 0: un letale vale 0, metà degli LP vale metà della board. Nella ricerca togliere tutti gli LP vale quanto togliere tutte le sue carte (`SearchConfig::opponent_lp`); tra le board misurate entra anche il primo letale trovato, anche fuori dalle prime `--top`.

| Mio deck | Contro | Con breaker: prima → dopo | Senza breaker: prima → dopo | Mani con danno (letali) |
|---|---|---|---|---|
| Sky Striker | Invoked (fixture MD), board 1,49 | 18% → 6% (10 mani) | 56% → 68% (2 mani) | 9 (0) |
| Branded | Snake-Eye Rescue-ACE, board 1,00 | 33% → 0% (3 mani) | 56% → 56% (9 mani) | 7 (3) |
| Yummy | Labrynth, board 0,57 | 87% → 69% (7 mani) | 62% → 49% (5 mani) | 7 (0) |

- Le mani con Forbidden Droplet o Infinite Impermanence: Sky Striker da 30% a 9% in media (6 mani), Yummy da 87% a 69% (7 mani). Le negazioni ora aprono gli attacchi: Sky Striker toglie 1500-4000 LP. Nessun letale da queste mani a 4000 nodi; i letali sono di Branded (8000 LP, tre mani).
- Alcune mani peggiorano (Sky Striker 0% → 39%, Yummy 0% → 105%): al turno 3 le carte avversarie hanno di nuovo i loro effetti una volta per turno e le sue Trappole Settate si possono attivare, e la ricerca spende nodi anche negli attacchi.
- La board Branded vale 1,00 invece di 1,21: le liste MDM sono cambiate dal mattino; prima e dopo sono sulle stesse liste.

## Limiti (da sapere leggendo i numeri)
1. **Le risposte sono ingenue** (dal 24/10): l'avversario usa un'interruzione alla prima occasione, non al momento migliore (Dragoon può negare Pot of Desires invece della Fusione), e solo quelle trovate dalla misura contro il kit generico. La misura dopo il mio turno rimette le sue carte fresche, quindi non sa che un effetto una volta per turno è già stato usato (limite 3). Prima del 24/10 la board non rispondeva affatto e un avversario che usava il primo effetto offerto si distruggeva da solo (Super Polymerization con i suoi mostri).
2. **Battle Phase** (dal 24/10): il mio turno è il terzo del duello, dopo due turni vuoti; nessuno pesca, quindi la mano da 6 è già quella con la pescata. Gli effetti dei turni vuoti (Fasi di Standby e Finale dell'avversario) avvengono senza che nessuno risponda. Il danno vale come la board con peso 1: un letale conta come board rotta, ma non come partita vinta.
3. Le carte piazzate perdono lo stato del turno (effetti usati, contatori), come le board piazzate del primo turno.
4. La board avversaria è una sola (la migliore di 12 mani), misurata contro il kit generico.

## Prossimi passi
- [fatto il 24/10] Risposte dell'avversario dalle interruzioni misurate (limite 1). Resta il momento giusto per rispondere.
- [fatto il 24/10] Turno con Battle Phase e danno (limite 2). Poi una tabella «per secondo» per il Laboratorio e il banco della misura (40 mani × 6 deck del meta).
- [fatto il 24/10] Interfaccia: scheda «Per secondo» nel Laboratorio (menu «Altro»): si sceglie un deck tra i 12 più giocati del meta e 6, 10 o 16 mani; la board avversaria, le mani una per una (quota che resta, LP tolti, linea) e «Ferma». Comandi `second_opponents`, `second_run`, `second_cancel`, evento `second-progress`. Senza mockup di Design e provata solo con il backend finto (numeri inventati, nessuna risposta registrata).
- Poi: tabella «per secondo» e banco della misura (40 mani × 6 deck).
