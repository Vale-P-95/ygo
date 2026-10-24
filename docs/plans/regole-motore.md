# Regole per accettare una modifica al motore

Sostituisce, dal 24/10/2026, la regola «per il motore, la prova equa sulle 406 board non peggiorata». Vale per ogni PR che tocca `crates/ygo-engine` o `crates/ygo-solver`. Strumento: `scripts/banco.py`; banco: `tests/fixtures/banco-motore/v1/`.

## Perché si cambia
- **La prova delle 406 board non si può rifare.** Le board e la verità del livello 2 del giro A1b stanno solo nella cartella di lavoro del proprietario; nel repo ci sono 24 file di linee su 87.
- **Le liste di Master Duel Meta cambiano ogni giorno.** Oggi (24/10) le fixture dell'oracolo del 23/10 si rigiocano solo su 49 mani su 91: 18 liste non esistono più uguali. Delle 63 liste del banco di generalità, 61 tornano uguali e 2 no.
- **Costa circa 9 ore** e giudica solo la misura, non la ricerca.
- **Le regole di `paired_test.py` sono fragili:** «peggio ≤ meglio» boccia per caso circa metà delle modifiche neutre, e «nessuna mano oltre −5%» diventa più severo man mano che le mani crescono, senza una ragione statistica.

## 1. Il banco congelato
- **Cosa c'è.** 63 liste (43 di taratura da Master Duel Meta, 20 fresche da tornei TCG/OCG e Master Duel Meta, le stesse del banco di generalità), solo passcode, con fonte e impronta in `manifest.json`. 193 mani: 3 per deck (5 carte estratte una volta con un seme fisso e scritte come passcode, semi del duello 1, 2 e 3), più 4 mani Invoked del vecchio banco ancora giocabili.
- **Due insiemi.** `quick`: la prima mano di ogni deck (63 mani). `full`: tutte (193).
- **Le soglie stanno in `rules.json`**, scritte prima di ogni giro. Si cambiano solo con una versione nuova del banco, in una PR a parte.
- **Una versione non si cambia mai.** La rotazione (ogni 2-3 mesi, insieme ai freschi del banco di generalità) crea `v2` con `banco.py freeze v2 --sets <liste>`. `v1` resta per rifare i confronti vecchi.
- **Controllo.** `banco.py check`: le liste del repo corrispondono alle impronte del manifesto.
- **Liste diverse dal manifesto di generalità.** Per `invoked` e `warrior_combo` la lista di oggi (40/15 carte, dallo stesso id di Master Duel Meta) ha un'impronta diversa da `sets.json`. Il manifesto del banco registra le due impronte, quella vecchia come `manifest_hash`.

## 2. Ogni modifica ha una classe, dichiarata prima del giro
La classe si scrive nella PR prima di lanciare il banco. Se il giro dice altro, vale il giro più severo.

| Classe | Esempi | Prova | Si accetta se |
|---|---|---|---|
| **0. Non tocca il motore** | interfaccia, documenti, stampa della CLI | CI | CI verde |
| **E. Equivalenza** | velocità, memoria, copie del duello, refactoring | `banco.py same`, set `full` | tutte le 193 uscite JSON uguali (numeri entro 1e-9, tempi esclusi) e nessuna run fallita su un lato solo |
| **R. Ricerca** | ordine, potature, budget, correzioni del motore che cambiano le board | `banco.py judge`, set `full` | le cinque regole del §3 |
| **M. Misura** | `value.rs`, pesi e parti del turno di prova | `banco.py judge --measure-cli`, set `full`, più la verità del §4 | le regole del §3 sulla scala nuova e quelle del §4 |

- **E** è la prova più forte: un'uguaglianza esatta non ha bisogno di statistica. Se anche una sola uscita cambia, la modifica non è più E: si passa a R.
- **Tempo, in E e R.** `run` fa girare vecchio e nuovo sulla stessa mano, uno dopo l'altro, sullo stesso processo e in ordine alternato; il rapporto dei tempi è la media geometrica con l'intervallo per deck. Un rallentamento certo oltre il 5% (limite basso dell'intervallo sopra 1,05) boccia una modifica E. In R, un rallentamento certo oltre il 10% si accetta solo con un guadagno dimostrato. I tempi contano solo se i due lati di ogni mano sono girati sulla stessa macchina con gli stessi job; lo script lo controlla.
- **Le correzioni del motore** (`ygo-engine`) hanno anche un test che riproduce il difetto, e passano da R.

## 3. Le regole della classe R
Il valore di una mano è quello della board migliore misurata, come nel banco di oggi. Il cambio di una mano è (nuovo − vecchio) / vecchio: meglio sopra +1%, peggio sotto −1%, pari in mezzo.
1. **Non inferiorità.** Il cambio medio (ogni mano limitata a ±100%) ha il limite basso dell'intervallo al 95% non sotto −3%. L'intervallo si calcola ricampionando i deck, non le mani (bootstrap, 10 000 volte, seme fisso), perché le mani di uno stesso deck si somigliano.
2. **Test del segno.** Le mani peggiorate non sono significativamente più di quelle migliorate: p unilaterale ≥ 0,05. Sostituisce «peggio ≤ meglio».
3. **Grandi perdite.** Le mani che perdono più del 5% sono al massimo il 2% delle mani (3 su 193), oppure non più di quelle che guadagnano più del 5%. Ognuna si nomina nella PR, con una riga sul perché.
4. **Deck freschi.** Sui soli 60 deck freschi il cambio medio non scende sotto −3% e il limite alto dell'intervallo non è sotto 0 (nessuna perdita dimostrata): una modifica che serve solo ai deck su cui si è tarato il motore non passa. Sui freschi non si chiede il limite basso, perché con 60 mani l'intervallo è largo ±5 punti (§6).
5. **Niente di rotto.** Nessuna run fallita, nessun turno di prova fallito e nessuna linea interrotta in più rispetto a `main`; `search_regression` verde.

- **Dire «migliora».** Serve il limite basso dell'intervallo sopra 0 e il test del segno a p < 0,05 a favore. Senza, la PR dice «non peggiora».
- **Costanti nuove** (un peso, una soglia, un ordine): devono dimostrare il miglioramento, non solo la non inferiorità; vale la regola del banco di generalità.
- **Determinismo.** Con un thread e un budget di nodi le board non dipendono dalla macchina. Se due giri di `main` sulla stessa mano danno uscite diverse, il banco non vale finché non si trova il perché. La taratura lo ha controllato (§6).

## 4. Le regole della classe M
Una misura nuova cambia la scala dei valori. Il confronto della ricerca si fa quindi rimisurando la board migliore dei due lati con la misura nuova (`--measure-cli`): è la regola del §3 sulla scala nuova. Resta da sapere se la misura nuova segue meglio la verità.
- **Verità v1 (da costruire, §7).** Le linee di 5 board per deck del banco e il loro livello 2 contro gli avversari più giocati (`objective_bench.py` sulle liste congelate), committati in `tests/fixtures/banco-motore/v1/verita/`. Rimisurare costa minuti, non ore: una sola misura per board, senza livello 2.
- **Metrica fissata prima:** le coppie di board dello stesso deck con verità distante almeno 0,02, nell'ordine giusto.
- **Regola:** test di McNemar esatto sulle coppie guadagnate e perse. Si accetta se le perse non sono significativamente più delle guadagnate (p ≥ 0,05) e la quota di coppie giuste non scende di oltre 1 punto, anche sui soli deck di verifica. «Migliora» se p < 0,05 a favore.
- **Fino alla verità v1:** la prova equa A1b, se il proprietario ha ancora la cartella; altrimenti la PR della misura non si unisce in automatico e decide il proprietario.
- **Le coppie del proprietario** (`owner-verdicts.tsv`, 38 giudizi) si riportano sempre, ma non decidono: sono troppo poche.

## 5. Costo e dove si lancia
Misurato il 24/10 nel contenitore cloud (4 core): mediana di 16 s per mano a 4000 nodi su un thread, 1 ora di calcolo per lato sulle 193 mani, la mano più lenta 138 s.

| Giro | Run | Su 4 core (sessione cloud) | Su Actions (`bench.yml`, 6 runner da 2 job) |
|---|---|---|---|
| `quick`, due lati | 126 | 11 minuti (misurati) | circa 12 minuti a runner con build e dati, circa 1,5 ore fatturate |
| `full`, due lati | 386 | circa 35 minuti (29 misurati con un lato a 2000 nodi) | circa 25 minuti a runner, circa 3 ore fatturate |

- **Di norma il giro lo fa la sessione stessa**, nel suo contenitore cloud, in background: non consuma minuti di Actions. Ogni comando scrive `summary.txt` (dieci righe): si legge quello.
- **`bench.yml`** (avvio a mano, `bench: banco-full`, `check: same|judge|judge-measure`) quando la sessione non ha dati o tempo; il verdetto finisce nel riepilogo del workflow.
- **`quick`** è per provare durante il lavoro. Nella PR va sempre il giro `full`.

```
python3 scripts/banco.py check
python3 scripts/banco.py run $S/banco --old <cli di main> --new <cli del branch> --set full --jobs 4
python3 scripts/banco.py same $S/banco             # classe E
# con una CLI che ha `combos --cases`, run gioca le mani di un deck in un processo; con quella di main, una mano per processo
python3 scripts/banco.py judge $S/banco            # classe R
python3 scripts/banco.py run $S/banco ... --measure-cli <cli del branch>; python3 scripts/banco.py judge $S/banco --measure-cli <cli del branch>   # classe M
```

**Nella PR:** la classe, il comando, `summary.txt` per intero, le grandi perdite spiegate. Il merge automatico della coordinatrice richiede `ACCEPTED`.

## 6. Taratura del 24/10 (CLI di `main` 3c986c1)
Prima di ogni verdetto si è provato il banco su modifiche di cui si sa già la risposta.

| Prova | Cosa deve dire | Cosa dice |
|---|---|---|
| `main` contro `main`, 63 mani `quick` | E: tutto uguale, tempo 1,00 | 63 uscite su 63 identiche; tempo 0,996 (0,986-1,006) → ACCEPTED |
| 4000 contro 2000 nodi, 193 mani | R: bocciare | 12 meglio, 71 peggio, 110 pari; media −6,7% (−9,3% a −4,2%); segno p < 0,001; tempo 0,85 → REJECTED su tre regole |
| 2000 contro 4000 nodi, 193 mani | R: «migliora» | media +10,7% (+7,5% a +14,1%); tempo 1,17 → ACCEPTED, BETTER |

- **Determinismo confermato**: con un thread e un budget di nodi le uscite sono identiche, e il rumore dei tempi sullo stesso contenitore è sotto l'1%. La soglia del 5% per il rallentamento resta larga.
- **Larghezza degli intervalli.** Su tutte le 193 mani il semi-intervallo al 90% è di circa 2,6 punti per una modifica che sposta molte mani (deviazione standard dei cambi 20%); sui 60 freschi di circa 5 punti.
- **Margini rivisti prima di ogni uso:** −3% invece di −2% su tutte le mani, perché con −2% una modifica neutra così dispersa verrebbe bocciata circa 6 volte su 10 (con −3% circa 4 su 10: una modifica che sposta tante mani deve mostrare un po' di guadagno; con meno mani cambiate l'intervallo si stringe); sui freschi la media e «nessuna perdita dimostrata» invece del limite basso. Nessun verdetto era stato dato con le soglie di prima, quindi la versione resta `v1`.
- **Le grandi perdite servono con il loro contrappeso**: anche raddoppiando i nodi 5 mani perdono più del 5% (la ricerca non è monotona nel budget), ed è la clausola «o non più delle grandi vincite» che non boccia il raddoppio.

## 7. Cosa resta da fare
1. **Verità v1** (§4): un giro lungo, di notte o su `bench.yml`, poi le fixture committate.
2. **`paired_test.py`** resta per gli studi su mani scelte; per il merge vale `banco.py`.

## Decisioni (24/10/2026)
- Il merge del motore richiede `banco.py` `ACCEPTED` sul banco congelato, non più la prova sulle 406 board.
- Le liste del banco sono committate (solo passcode, con fonte): è l'unica cosa che rende la prova ripetibile.
- Classe dichiarata prima del giro; soglie in `rules.json`, cambiate solo con una versione nuova.
- Non inferiorità con intervallo per deck (margine −3%, tarato il 24/10) e test del segno al posto di «peggio ≤ meglio» e «nessuna mano oltre −5%».
- Per la misura, la verità v1 committata sostituisce A1b; fino ad allora decide il proprietario.
