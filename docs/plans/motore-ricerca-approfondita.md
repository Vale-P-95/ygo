# Motore: sprechi misurati, difetti e algoritmi dalla letteratura

*Thread «Pianificare i miglioramenti del motore con la letteratura», 24-25/10/2026, branch `claude/project-thread-0z5me6` da `main` e853b7c (dopo #178; #179 aperta, letta come stato attuale). Richiesta di Valerio: un piano approfondito, dal punto di vista analitico, statistico e algoritmico, con ricerca bibliografica, per trovare miglioramenti del motore, bug e rallentamenti inutili. Solo ricerca: nessun codice del motore in questa PR. Le misure vengono da contatori provvisori in `probe.rs`, tolti prima del commit.*

## In breve
- **Il primo guadagno è esatto e grande: le corse che si ricongiungono.** Nella misura di una board, il 43% delle corse «usa questa risposta a questa finestra» (`UseAt`) arriva, dopo la risposta, a un inizio di passo del kit con lo stesso stato di una corsa già giocata della stessa risposta. Il resto di quelle corse è già noto: 100 s su 312 s di corse `UseAt`, circa **−10/−11% del tempo totale** sulle 63 mani `quick`, con uscite identiche se la chiave è completa (proposta 1).
- **Le seconde finestre di un passo servono solo se la prima ha fermato qualcosa.** Quando la prima finestra di un passo non ferma niente, la seconda porta un'informazione nuova nello 0,5% dei casi (28 corse su 6005), e costa 28 s: circa **−3/−4%** con 28 risposte su 5968 che cambiano (proposta 2).
- **Un difetto di aggregazione nella misura:** coperture ed effetto prendono il meglio fra le finestre, «mirata» (×0,85) il peggio. Una risposta che ferma senza mirare a una finestra e mirando a un'altra perde il 15%; e provare più finestre può abbassare il valore, quindi misura economica e completa non sono coerenti (53 risposte su 1716 mirate, proposta 3).
- **Perché la #177 è caduta, in termini di letteratura:** la misura economica è una «fedeltà bassa» con un errore sistematico, e tagliare con uno scarto di 0,01 quando l'errore è 0,5 butta via la board giusta. La correzione nota è eliminare solo con uno scarto più grande dell'errore stimato (proposta 5).
- **Misurate e scartate qui, per non spenderci una PR:** regole di pazienza sulle finestre, branch-and-bound sulle finaliste (Fagin), salto delle finestre «sature».

## Come si è misurato
- **Mani:** le 63 mani `quick` del banco congelato (`tests/fixtures/banco-motore/v1`), come `banco.py run`: 4000 nodi, un thread, `--top 10`, il seme della mano, quattro mani alla volta su 4 core.
- **CLI:** `main` e853b7c con contatori provvisori su stderr in `measure_partial` e in `play` (non nella PR): per ogni corsa `UseAt` la risposta, la finestra `(passo, finestra nel passo)`, il tempo, se ferma qualcosa, se porta un'informazione nuova (una copertura in più, un effetto migliore o il segno «mirata» cambiato), se la finestra è «satura» (tutto ciò che quella finestra può fermare è già coperto); all'inizio di ogni passo del kit dopo la risposta usata, un'impronta dello stato (`Duel::fingerprint`, passo, prossimo passo, esito dei passi già giocati).
- **Tre giri** (845-892 s ciascuno; il terzo più lento per l'impronta calcolata a ogni passo). Le quote sono prese ciascuna dal giro che la misura.

**Dove va il tempo** (giro 2, 845 s): ricerca (giri di apprendimento compresi) 300 s (36%), turno di prova finale 323 s (38%), valori delle carte e avvio circa 220 s (26%). Delle 626 board misurate per intero alla fine: risposte 118 s, insieme 27 s, breaker 43 s, turno dopo 11 s; il resto del turno di prova finale è `expected_loss` (43 s) e le varianti. In tutto (valori delle carte, giri di apprendimento, misura finale) le corse `UseAt` sono 64 561 e costano 263 s, il 31% del tempo totale: è la voce più grande.

**Le corse `UseAt`, una per una:**

| | Corse | Tempo | Quota del tempo `UseAt` |
|---|---:|---:|---:|
| Tutte | 64 561 | 263 s | 100% |
| Non portano niente di nuovo | 47 394 (73%) | 203 s | 77% |
| Non fermano niente | 37 265 (58%) | 166 s | 63% |
| Prima finestra di un passo | 49 573 | 203 s | 77%; nuove nel 33,7% |
| Seconda finestra di un passo | 12 909 | 54 s | 21%; nuove nel 3,5% |
| Terza e oltre | 2 079 | 6 s | 2%; nuove: 1 |

**La guida della ricerca contro la misura** (prime 10 board di ogni mano): Spearman medio 0,36 fra stima e misura; la #1 misurata è la #1 della ricerca in 37 mani su 63, è fra la 7ª e la 10ª in 8. In 35 mani su 63 la #1 e la #2 misurate sono pari (scarto sotto lo 0,5%).

## Le proposte, in ordine di guadagno sul rischio

| # | Proposta | Guadagno misurato | Classe | Rischio | Taglia |
|---|---|---|---|---|---|
| 1 | Memoria dei resti di corsa (trasposizioni nel turno di prova) | −10/−11% del tempo totale (tetto) | E | medio: completezza della chiave | M, 15-20 $ |
| 2 | Seconde finestre di un passo solo dopo una prima che ferma | −3/−4% | M | basso: 28 risposte su 5968 | S, 5-10 $ |
| 3 | «Mirata» coerente con coperture ed effetto | qualità; misura monotona nelle finestre | M | basso | S, 5-10 $ |
| 4 | Greedy pigro per le risposte usate insieme | −1/−2% (fase «insieme» 4,3% del totale) | E | basso | S, 5-10 $ |
| 5 | Eliminazione con il margine dell'errore della misura economica (seguito della #177) | il tempo dei dimezzamenti senza la caduta di Lunalight | R | medio | M, 15-20 $ |

### 1. Memoria dei resti di corsa
**Lo spreco.** Una corsa `UseAt` gioca il turno avversario fino alla finestra (oggi da una copia, B13 e #179), usa la risposta e poi gioca tutto il resto del turno. Il resto dipende solo dallo stato del duello e da dove è arrivato il kit. Sulle 63 mani, 27 520 corse su 64 561 (43%) arrivano all'inizio di un passo del kit con la stessa impronta (stato, passo, prossimo passo, esito dei passi già giocati) di una corsa già giocata della stessa risposta nella stessa misura; il loro resto costa 100 s su 312 s di corse `UseAt` di quel giro. Succede soprattutto quando la risposta non ferma niente o ferma la stessa cosa da finestre diverse: dopo, il duello è lo stesso.

**La proposta.** Una tabella di trasposizione per misura (Zobrist 1970; in MCTS, Childs, Brodeur & Kocsis 2008): all'inizio di ogni passo, dopo la risposta usata, la chiave; se c'è, la corsa finisce lì e prende dalla tabella il resto: l'esito dei passi da lì in poi, le finestre offerte, gli usi delle risposte derivate (`known`), le carte prese. È la «memoria delle corse» (tecnica 5 di `tempi-motore.md`, −4%) portata a metà corsa: lì si riusano le corse uguali dall'inizio, qui i resti uguali dopo punti diversi.

**Il rischio è la chiave.** `fingerprint` ordina il Deck (non ne vede l'ordine) e non vede lo stato in Lua (le carte dell'avversario già usate in questo turno, i flag «questo turno»). Quindi:
- la chiave aggiunge l'ordine del Deck dei due giocatori e le attivazioni dell'avversario (oggi `activity` è solo tua);
- un controllo come `DUELFORGE_COPY_CHECK`: una corsa su n presa dalla tabella si gioca anche per intero e si confronta; sul banco, 0 differenze o la chiave si allarga;
- si spegne con una variabile, come le copie.

**Validazione.** Classe E: `banco.py same` sul set `full` (193 uscite uguali) e il controllo sopra su Fur Hire, Elfnote, Phantom Knights e Ryu-Ge. Il tetto del guadagno è il 32% delle corse `UseAt`; con il costo della chiave, atteso −8/−10%. File caldo: `probe.rs`.

### 2. Seconde finestre solo dopo una prima che ferma
**Lo spreco.** Le finestre di una risposta si provano in ordine: la prima di ogni passo, poi la seconda di ogni passo, fino a 16. Le seconde portano qualcosa di nuovo nel 3,5% delle corse, ma dipende da cosa ha fatto la prima dello stesso passo:

| La prima finestra del passo | Seconde finestre | Nuove | Tempo |
|---|---:|---:|---:|
| non ha fermato niente | 6005 | 28 (0,5%) | 28 s |
| ha fermato qualcosa di nuovo | 2541 | 87 (3,4%) | 10 s |
| ha fermato qualcosa già coperto | 4363 | 339 (7,8%) | 16 s |

Le terze finestre e oltre: 2079 corse, 6 s, una sola nuova.

**La proposta.** Una seconda finestra di un passo si prova solo se la prima dello stesso passo ha fermato qualcosa; le terze mai. Nessuna regola per carta: è una regola sull'ordine delle finestre del kit.

**Guadagno e rischio.** 34 s su 845 (−4%). Cambiano al più 28 + 1 risposte su 5968, in valori delle carte, giri di apprendimento o misura finale. Classe M: `banco.py judge --measure-cli` sul set `full`, e le mani che perdono con il loro perché.

### 3. «Mirata» coerente con coperture ed effetto
**Il difetto** (`measure_partial`, ciclo delle finestre). Per ogni risposta le coperture sono l'unione su tutte le finestre provate e l'effetto il migliore (`min` sull'ordine di `Effect`), ma `targeted |= u.targeted`: basta una finestra in cui mira perché tutto il valore prenda ×0,85. Due conseguenze:
- una risposta che ferma lo stesso passo senza mirare a una finestra e mirando a un'altra vale il 15% in meno di quanto può;
- il valore non è monotono nelle finestre: con 2 finestre (misura economica) può valere più che con 16. La scala economica → completa (B14, #177) assume il contrario.

**Quanto pesa.** Delle risposte che fermano qualcosa, 1716 su 4609 sono mirate; per 53 c'è una finestra che ferma senza mirare. Pochi casi, ma tutti sbagliati nello stesso verso.

**La proposta.** «Mirata» solo se mira in ogni finestra che ferma qualcosa (AND al posto di OR), oppure il valore di una risposta come il massimo sulle finestre di copertura × effetto × mira di quella finestra. La prima è una riga.

**Riproduzione.** Con `DUELFORGE_DEBUG_PROBE=1` la misura stampa per ogni risposta e finestra cosa ferma; le 53 risposte si trovano con il contatore descritto sopra (si rifà in 15 minuti sulle 63 mani). Classe M, `judge --measure-cli`.

### 4. Greedy pigro per le risposte usate insieme
**Lo spreco.** Quando le risposte usate insieme tengono meno della somma, la misura prova a toglierne una alla volta e ad aggiungerne una alla volta: 2n + 1 corse `UseAll`. La fase costa 38 s su 845 (4,3%), 27 s nella misura finale.

**La proposta.** La valutazione pigra di Minoux (1978): il valore da sola (`alone`) è un limite superiore di quanto una risposta aggiunge; nel ciclo «aggiungi», una risposta il cui limite non supera il guadagno minimo che servirebbe non si prova. È esatta se il valore tenuto è sottomodulare (ciò che una risposta aggiunge non cresce con le altre), che qui non è garantito: Super Polymerization che usa una tua board come materiale è proprio un caso non sottomodulare. Quindi si tiene solo se `banco.py same` dà 193 uscite uguali.

**Guadagno.** Metà della fase al più: −1/−2%. Piccolo, ma di classe E.

### 5. Eliminazione con il margine dell'errore (seguito della #177)
**Cosa è andato storto.** La #177 sceglieva le finaliste con la misura economica (2 finestre per risposta, niente breaker né turno dopo). In `tuning.lunalight.0` la board giusta aveva 0,47 contro 0,48 di altre dieci e restava fuori, ma la misura completa le dà 1,03. La letteratura sui bandit a più fedeltà (Kandasamy et al. 2017; Poiani, Metelli & Restelli 2022) dice esattamente questo: una fedeltà bassa con un errore sistematico ζ si può usare per eliminare un braccio solo quando lo scarto supera ζ; i dimezzamenti semplici (Karnin et al. 2013, Hyperband) non lo sanno e scartano i bracci lenti a crescere.

**La proposta.** Prima di qualunque scelta a dimezzamenti (proposta 2 di `prossimi-passi-motore.md`):
- ζ si stima sulle mani di taratura: per ogni board misurata in tutti e due i modi, misura completa meno economica; si prende un quantile alto (per esempio il 95°), per deck o globale;
- una board esce solo se la sua misura economica è sotto quella della migliore di più di ζ; tutte quelle dentro la banda hanno la misura completa;
- con la proposta 3 fatta, la misura è monotona nelle finestre e ζ è solo positivo, il che rende la banda più stretta.

**Guadagno.** Il tempo dei dimezzamenti dove la misura economica separa davvero, senza le cadute: Lunalight resta dentro, perché 0,01 di scarto è meno di ζ. **Rischio** medio: se ζ è grande, la banda contiene tutte e 15 e non si guadagna niente; si sa subito contando, senza giri nuovi, quante board cadono nella banda. Classe R, `banco.py judge` sul set `full`.

## Misurate e scartate
- **Pazienza sulle finestre** (smettere dopo K finestre di fila senza niente di nuovo, come nelle corse a eliminazione di Maron & Moore 1994): le informazioni arrivano tardi, perché le prime finestre di ogni passo vengono prima delle seconde. K = 4 risparmia il 42% delle corse `UseAt` ma cambia 998 risposte in 61 mani su 63; K = 8 risparmia il 16% e ne cambia 89 in 24 mani. La proposta 2 prende quasi tutto il guadagno sicuro con 28 cambi.
- **Branch-and-bound sulle finaliste** (Threshold Algorithm di Fagin, Lotem & Naor 2003; MPro di Chang & Hwang 2002): breaker e turno dopo pesano nel punteggio solo per il turno dopo (0,12 per effetto), quindi si potrebbero saltare per le board che non possono più arrivare prime. Con il limite più largo sicuro (due effetti per carta nel GY o bandita) si salterebbero per 84 board su 626, cioè 4,6 s su 845 (0,5%). Non vale una PR.
- **Finestre sature** (tutto ciò che quella finestra può fermare è già coperto): 13% delle corse ma solo il 3% del tempo `UseAt`, e non sono esatte (42 corse cambiano l'effetto o la copertura, perché alcuni obiettivi si guardano alla fine del passo).
- **Le corse per la risposta dell'avversario alla tua interruzione** (`countered`): 29,5 s (3,5%), con peso zero nel punteggio; ma l'app e il JSON le mostrano per ogni board, quindi toglierle è una scelta di interfaccia (calcolarle solo quando si apre la scheda), non del motore.

## Brief della prima
```
Compito: memoria dei resti delle corse UseAt nel turno di prova: all'inizio di ogni passo del kit dopo la risposta usata, una chiave (stato, ordine del Deck, attivazioni dei due giocatori, passo, esito dei passi giocati); se c'è, la corsa prende il resto dalla tabella
Perché: docs/plans/motore-ricerca-approfondita.md §1 (43% delle corse UseAt, 100 s su 312 s sulle 63 mani quick)
Branch: claude/run-suffix-memo, da main dopo la #179
File da toccare: crates/ygo-solver/src/probe.rs (caldo: lock in TASKS.md), crates/ygo-engine/src/state.rs (se la chiave ha bisogno delle attivazioni dell'avversario)
Da leggere: docs/STATO.md, regole-motore.md, tempi-motore.md «Le due correzioni fatte»
Verifica: controllo «una corsa su n giocata anche per intero» con 0 differenze su Fur Hire, Elfnote, Phantom Knights, Ryu-Ge; banco.py run --set full e same, in background
Criterio di accettazione: ACCEPTED con 193 uscite uguali; tempo new/old ≤ 0,93
PR: template, classe E; Modello: Opus 5.5 alto
```

## Fonti nuove (non nelle revisioni bibliografiche precedenti)
- Zobrist, *A New Hashing Method with Application for Game Playing*, 1970 (TR 88, University of Wisconsin): la chiave di trasposizione.
- Childs, Brodeur & Kocsis, *Transpositions and Move Groups in Monte Carlo Tree Search*, IEEE CIG 2008: riuso dei risultati fra stati raggiunti per strade diverse.
- Minoux, *Accelerated greedy algorithms for maximizing submodular set functions*, 1978; riassunto e limiti in [submodlib, Lazy Greedy](https://submodlib.readthedocs.io/en/latest/optimizers/lazyGreedy.html) e [Bilmes, EE596B lezione 15](https://people.ece.uw.edu/bilmes/classes/ee596b_spring_2016/lecture15_print.pdf).
- Fagin, Lotem & Naor, *Optimal aggregation algorithms for middleware*, JCSS 2003 ([PDF](https://arxiv.org/pdf/cs/0204046.pdf)): Threshold Algorithm, il top-k con limiti sui punteggi parziali.
- Chang & Hwang, *Minimal Probing: Supporting Expensive Predicates for Top-k Queries*, SIGMOD 2002 ([scheda](https://experts.illinois.edu/en/publications/minimal-probing-supporting-expensive-predicates-for-top-k-queries/)): solo le valutazioni care necessarie.
- Poiani, Metelli & Restelli, *Multi-Fidelity Best-Arm Identification*, NeurIPS 2022 ([PDF](https://proceedings.nips.cc/paper_files/paper/2022/file/71c31ebf577ffdad5f4a74156daad518-Paper-Conference.pdf)); seguito ottimo in Poiani et al., NeurIPS 2024 ([arXiv:2406.03033](https://arxiv.org/abs/2406.03033)): eliminare con una fedeltà bassa solo oltre il suo errore massimo.
- Kandasamy et al., *Multi-fidelity Bayesian Optimisation with Continuous Approximations*, ICML 2017, e la rassegna [Multi-Fidelity Methods for Optimization](https://arxiv.org/pdf/2402.09638): perché Successive Halving e Hyperband scartano i candidati lenti a crescere.
- Maron & Moore, *Hoeffding Races*, NIPS 6 (1994): le corse a eliminazione, qui provate come pazienza e scartate.
