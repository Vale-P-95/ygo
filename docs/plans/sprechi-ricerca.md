# Sprechi della ricerca delle combo

Analisi del 25/10/2026, chiesta da Valerio il 24/10 dopo la PR #180: dove la ricerca perde tempo su linee uguali, cicli di annullamento, linee che cambiano solo la posizione delle evocazioni, negazioni delle proprie carte, e quanto pesa valutare le board senza il turno dell'avversario. Solo misure e proposte: nessun codice da unire.

**Lettura della domanda.** «Quando il motore cerca le mani migliori senza considerare il turno dopo l'avversario» è la fase di ricerca, prima del turno di prova: lì la board si giudica con la stima rapida (`quick_score`). I primi cinque punti misurano gli sprechi di questa fase; il §7 misura quanto costa non guardare il turno avversario durante la ricerca.

## Metodo
- **Mani.** Le 63 mani `quick` del banco congelato (`tests/fixtures/banco-motore/v1`), CLI di `main` 411361e, 4000 nodi, un thread, `--top 10`: le impostazioni di `banco.py`.
- **Contatori provvisori** in `search.rs` (non uniti), attivi con `DUELFORGE_SEARCH_STATS=1`: le ricerche restano identiche a `main` (stesse uscite, controllato). A ogni momento libero (scelta di cosa fare, finestra di catena) il nodo nuovo per la chiave esatta si confronta con tre chiavi più larghe: senza l'ordine delle carte in mano; anche senza le zone principali (Mostri e Magie/Trappole, la Zona Extra resta a parte); anche senza la posizione ATK/DIF. Se una chiave larga è già stata vista, da lì la linea ripete una situazione nota, e tutto ciò che sta sotto (nodi e tempo) si conta come spreco della sua classe.
- **Patch e script** nei file del progetto: `analisi/sprechi-ricerca/` (`contatori-e-prototipo.patch`, `run.py`, `agg.py`, `top30.py`, `agg.txt` con tutti i numeri).
- **Riprodurre un esempio:** con la patch, `DUELFORGE_WASTE_EXAMPLES=1 DUELFORGE_SEARCH_STATS=1 duelforge-cli combos --deck tests/fixtures/banco-motore/v1/decks/tuning/snake_eye.ydk --hand "66328392;59581480;59438930;64491754;66328392" --seed 1 --nodes 4000 --threads 1`.

## Riepilogo
Totale delle 63 mani: 217 949 nodi, 145 s di ricerca (somma dei tuffi), 982 s con il turno di prova.

| Spreco | Nodi | Tempo della ricerca | Stato | Proposta |
|---|---|---|---|---|
| Stesse giocate in altro ordine, che finiscono in zone diverse (§1) | 20,8% | 22,3% | non visto dalla ricerca | 1 |
| Stesse giocate con la mano in altro ordine (§2) | 3,0% | 3,5% | non visto | 2 |
| Tuffi che arrivano a un nodo già visto (§3) | 29,6% | 30,9% | visto, ma dopo il replay | 3 (da misurare) |
| Cicli e annullamenti (§4) | 0,05% | 0,1% | già risolto | nessuna |
| Negazioni delle proprie carte (§5) | 0,01% | 0,2% | già risolto | nessuna |
| Solo posizione ATK/DIF diversa (§6) | 0,0% | 0,0% | già risolto | nessuna |
| Board giudicate senza il turno avversario (§7) | — | ricerca 23%, turno di prova 70% del tempo | parziale | 4 |

Le righe §1 e §3 si sovrappongono in parte: un tuffo dentro una ripetizione per zone può finire anche su un nodo già visto.

## 1. Stesse giocate, zone diverse
- **Cosa succede.** La chiave di un nodo (`node_key`) contiene lo stato con le zone esatte (solo lo specchio è canonico) e la decisione con l'indice di zona di ogni carta. Attivare Magia A e poi B mette A nella zona 1 e B nella 2; B e poi A le scambia. Le due linee arrivano alla stessa situazione, ma con chiavi diverse: la ricerca la esplora due volte, con tutto ciò che viene dopo. `distinct_zones` riduce le scelte di zona offerte in un momento, ma non questi scambi dovuti all'ordine, né i buchi lasciati da materiali e tributi.
- **Esempio (snake_eye.0, Snake-Eye/White Forest).** «Normal Summon Ghost Ogre, Special Summon Elfnote Tinia, Deception of the Sinful Spoils, Elfnotes: Welcome Home, Deception» e la stessa linea con le due Magie scambiate: stessa situazione, chiavi diverse. In questa mano il 77% dei nodi (3109 su 4022) sta sotto una ripetizione del genere.
- **Quanto.** 20,8% dei nodi e 22,3% del tempo di ricerca sulle 63 mani: 12,7% senza Mostri Link sul campo, 8,1% con Mostri Link (lì le frecce rendono le zone importanti, e la correzione non tocca quei nodi). Oltre il 30% dei nodi in 20 mani, oltre il 10% in 30, zero in 8.
- **Controllo.** Le board lette solo dentro le ripetizioni (mai fuori) sono 3489 su 63 mani, e in 6 mani (4 con Mostri Link) una di queste vale quanto la migliore della ricerca: con 4000 nodi il ramo originale non è esplorato per intero, e la copia ne esplora un'altra parte. Non è la prova che le zone contino; è il motivo per cui la correzione va giudicata sul banco, non solo con il contatore.
- **Prototipo** (`DUELFORGE_PROTO_ZONES=2`, nella patch): ai momenti liberi, senza Mostri Link scoperti sul campo, la chiave dimentica le zone principali e l'ordine della mano. Le board migliori restano giocate in tutte le zone da `zone_variants` alla fine. Snake-Eye 0: stima della migliore da 11,1 a 12,8 a parità di nodi. Banco `quick`: 5 meglio, 7 peggio, 51 pari, media +0,8% (−2,6% a +4,8%), tempo 1,02, board trovate in più (artmage 1117 → 1507, paleozoic 1334 → 1786); REJECTED per 5 grandi perdite (artmage −42%, psychics −40%, fur_hire −23%, code_talker −17%, goblin_biker −13%) contro 2 grandi vincite (paleozoic +157%, vampire +77%). Banco `full` (193 mani): 24 meglio, 16 peggio, 153 pari, media +1,06% (−0,71% a +3,17%), segno p 0,92, freschi +0,25% (−0,73% a +1,24%), tempo 1,008, nessuna rottura; grandi perdite artmage 0 e 1, code_talker 0, doomz 0 e 1, compensate dalle grandi vincite: **ACCEPTED** («non peggiora», non «migliora»).
- **Lettura.** Liberare un quinto dei nodi vale circa un quarto di budget in più, e raddoppiare il budget dà +10,7% (`regole-motore.md` §6): ci si aspetta +2-3%, sotto il rumore di una mano singola, con oscillazioni grandi in entrambi i sensi perché la ricerca a 4000 nodi non è monotona. Il guadagno vero è di tempo: arrivare alle stesse board con meno nodi.

## 2. Mano in altro ordine
- **Cosa succede.** Lo stato ordina la mano, ma la decisione elenca le carte della mano con il loro posto (`CardRef::sequence`), e entra nella chiave. Aggiungere A e poi B alla mano, o B e poi A, dà due nodi diversi.
- **Quanto.** 3,0% dei nodi, 3,5% del tempo; 24 mani toccate.
- **Correzione.** Nella chiave dei momenti liberi la decisione si scrive senza il posto delle carte in mano (l'elenco ordinato delle azioni). È sicura: l'ordine della mano non cambia nessuna regola. Si può fare da sola o insieme al §1.

## 3. Tuffi che finiscono su un nodo già visto
- **Cosa succede.** La ricerca riconosce il nodo già visto (`Duplicate`, `OtherOrder*`, `Returns`), ma solo dopo aver rigiocato la linea dall'inizio (o da una copia) e averla seguita fino a lì.
- **Quanto.** 29,6% dei nodi e 30,9% del tempo (45 s su 145, di cui 26,5 s di replay). Per prima risposta del tuffo: le zone (`Place`, 1991 tuffi su 3971 finiscono così), le Set di Magie/Trappole (2488 su 5043 finiscono `OtherOrder`: la regola «Set in fila in un ordine solo» vale solo per le Set consecutive), le scelte di carte (`Choose`, 3814 `Returns`).
- **Proposta (da misurare prima).** Due strade: rinviare le Set di Magie e Trappole alla fine della linea quando nessuna carta del deck guarda le carte coperte nel turno (lettura dagli script, non per nome); oppure ricordare per ogni nodo le risposte già note come duplicate, così il fratello che ci arriva si scarta prima del replay. Dopo i §1-2 questa quota scende da sola (molti `Place` diventano trasposizioni riconosciute), quindi va rimisurata dopo.

## 4. Cicli e annullamenti
- **Quanto.** 100 tuffi su 63 mani finiscono in un ciclo (`Loop`), tutti nati da un annullamento: 100 nodi, 0,12 s. Le risposte che non cambiano niente di visibile sono soprattutto passi intermedi di un'evocazione (`Choose` 28 281, `SpecialSummon` 19 490), non cicli.
- **Perché è già risolto.** `SinceFree` riconosce la stessa scelta richiesta senza che sia successo niente, `without_returns` toglie l'annullamento quando si è appena cominciato qualcosa.
- **Nessuna proposta.**

## 5. Negazioni delle proprie carte
- **Quanto.** 98 tuffi (0,25 s) arrivano a una propria carta negata da un'altra propria: per esempio Invoked Mechaba che nega Virakam, the Divine Spirit of Artifice (invoked.0), o Fiendsmith Engraver in goblin_biker.0. Dopo la prima volta la ricerca impara chi nega e non lo concatena più alle proprie catene: 2592 risposte tagliate prima di giocarle.
- **Nessuna proposta.** Le risposte alla propria catena (`Activate@chain`) sono l'1% del tempo.

## 6. Solo la posizione ATK/DIF
- **Quanto.** 12 nodi su 217 949. La politica automatica sceglie già la posizione, e le scelte rimaste cambiano lo stato davvero.
- **Nessuna proposta.**

## 7. Board giudicate senza il turno dell'avversario
- **Dove va il tempo.** Sulle 63 mani: ricerca 222 s (23%), apprendimento dei valori delle carte 47 s, `confirm_best` 11 s, `zone_variants` 18 s, turno di prova delle 10 finaliste 684 s (70%). Il turno avversario si guarda davvero solo alla fine, sulle prime 10 board per la stima.
- **Quanto sbaglia la stima.** La migliore per il turno di prova è la #1 della ricerca in 37 mani su 63; è tra la #7 e la #10 in 8 mani. Correlazione di rango fra stima e misura nelle prime 10: mediana 0,75, sotto 0,3 in 13 mani.
- **Board perse oltre la decima.** Nelle 8 mani con la migliore tra la #7 e la #10, misurando le prime 40: in 2 la migliore sta più in basso. Doomz 0: la #13 misura 1,84 contro 1,18 (+56%). Vampire 0: la #31 misura 2,26 contro 1,91 (+18%).
- **Le carte che tolgono all'avversario** (Hyperinvoked Aeon che bandisce dall'Extra Deck) la stima le conta già (`taken_from`, B5a).
- **Proposta.** Finaliste adattive: si misurano le prime 10; se la migliore è tra le ultime 4 (#7-#10), si misurano le 10 successive, e così via, con un tetto di 40. Costa circa 1,1 s per board: sulle 63 mani circa +9% del tempo totale (8 mani × 10-30 board).

## Interventi proposti, con i brief

### Brief 1: chiave dei momenti liberi senza zone principali e ordine della mano (§1-2)
```
Compito: ai momenti liberi la chiave del nodo dimentica l'ordine della mano e, senza Mostri Link scoperti sul campo, le zone principali (Mostri e Magie/Trappole; la Zona Mostri Extra resta).
Perché: docs/plans/sprechi-ricerca.md §1-2: 24% dei nodi della ricerca ripete situazioni già esplorate.
Branch: claude/zone-key, da main
File da toccare: crates/ygo-solver/src/search.rs (node_key e la sua chiamata in play; file caldo, lock in TASKS.md)
Da leggere: docs/plans/sprechi-ricerca.md §1-2, la funzione loose_key della patch in analisi/sprechi-ricerca/, docs/plans/regole-motore.md §2-3
Verifica: cargo test -p ygo-solver; search_regression; banco.py run --set full (old main, new branch) e banco.py judge; contatori della patch sulle 63 quick (zone e hand_order a 0)
Criterio di accettazione: banco ACCEPTED in classe R; i Pendulum e le carte che guardano le colonne (Kashtira, Mekk-Knight) controllati a mano su 2 mani
PR: template; classe R, summary.txt intero, ogni mano oltre −5% con il perché
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

### Brief 2: finaliste adattive per il turno di prova (§7)
```
Compito: dopo il turno di prova delle prime N board, se la migliore misurata è tra le ultime 4, si misurano le N successive per stima, fino a un tetto (40).
Perché: docs/plans/sprechi-ricerca.md §7: in 2 mani su 8 la board migliore sta oltre la decima (+56%, +18%).
Branch: claude/finaliste-adattive, da main
File da toccare: crates/ygo-solver/src/probe.rs o il punto in cui la CLI e l'app scelgono le finaliste (cercarlo), crates/ygo-cli/src/main.rs
Da leggere: docs/plans/sprechi-ricerca.md §7, docs/plans/regole-motore.md §2-3
Verifica: cargo test -p ygo-solver -p ygo-cli; banco.py run --set full e judge
Criterio di accettazione: banco ACCEPTED con «migliora» o non inferiore; tempo totale entro +10%
PR: template; classe R, summary.txt intero
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

### Brief 3 (dopo il Brief 1): rimisurare i tuffi che finiscono su nodi già visti (§3)
```
Compito: rimisurare con la patch dei contatori, su main con il Brief 1, quanto pesano i tuffi Duplicate/OtherOrder/Returns, divisi per prima risposta, e scegliere tra «Set rinviate» e «memoria delle risposte duplicate».
Perché: docs/plans/sprechi-ricerca.md §3: oggi 31% del tempo di ricerca.
Branch: claude/sprechi-ricerca-2, da main
File da toccare: nessuno da unire, solo docs/plans/sprechi-ricerca.md §3
Da leggere: docs/plans/sprechi-ricerca.md §Metodo e §3
Verifica: python3 run.py / agg.py della cartella analisi/sprechi-ricerca/ sulle 63 quick
Criterio di accettazione: una tabella per prima risposta e una proposta con il guadagno stimato in nodi e secondi
PR: template; «Non tocca il motore»
Modello: Sonnet 5.5, effort medio
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## Decisioni da prendere (Valerio)
- Il Brief 1 passa il banco `full` ma non sposta il valore medio (+1%, non significativo): si accetta come guadagno di velocità (stesse board con meno nodi) o si chiede anche un guadagno di valore?
- Il Brief 2 costa circa il 9% di tempo in più: va bene per le partite lunghe, o solo dietro un'opzione?
