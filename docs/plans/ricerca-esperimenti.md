# Esperimenti di ricerca (corsia 7)

*Sessione sorella di ricerca, 22/10/2026. Esperimenti offline che servono al piano critico (`ricerca-piano-critico.md`), senza toccare il codice del motore o della ricerca. Gli script sono in `scripts/research/`; i dati generati (JSON della caccia, tempi, soluzioni) restano fuori dal repository.*

**Macchina e versione.** Container Linux con 4 CPU, `main` cc78f89 (dopo la PR #68), build release. Liste di Master Duel Meta scaricate il 22/10 con `scripts/mdm_decks.py` (41 deck).

## 1. Gradini di misura per B14

**Domanda.** Oggi l'app misura a piena fedeltà le prime 30 board della stima rapida, e il turno di prova è il 58% del tempo di `combo_hunt`. Con quali gradini, e quante board a ogni gradino, si dimezza il tempo tenendo lo stesso #1 e almeno 2 board su 3 della top 3?

**Correzione del 22/10 sera.** Una prima versione di questa sezione, su 41 mani, proponeva un gradino con 2 finestre per risposta, che teneva il #1 in 41 mani su 41. Sulle 82 mani non regge: in 5 mani il #1 di oggi si perde, fino al 33,7% del valore. I numeri qui sotto sono quelli delle 82 mani.

### Dati e comandi
- **Board.** `combo_hunt.py --seed 7 --nodes 3000 --top 60` sui 41 deck MDM, con le due mani della caccia (`-0` e `-1`): 82 mani e 4490 board con la misura completa (mediana 60 a mano, minimo 8).
  ```
  python3 scripts/combo_hunt.py <mdm> --hands 2 --top 60 --seed 7 --json <w>/all82      # 4 shard con --shard K/4
  python3 scripts/research/b14_stages.py <w>/all82 --timing <w>/all82_t --cheap w2u0=<w>/all82_w2u0 --cheap w6u0=<w>/all82_w6u0 \
      --cheap w8u1=<w>/all82_w8u1 --cheap w16u0=<w>/all82_w16u0 [--ref-app]
  python3 scripts/research/b14_level2.py <w>/all60_t <w>/l2
  ```
- **Riferimento.** Il #1 «vero» è il migliore della misura completa su tutte le board elencate (fino a 60). Con `--ref-app` il riferimento è invece l'app di oggi, cioè la misura completa sulle prime 30 della stima.
- **Gradini dalla CLI.**
  - La stima rapida è `quick_score` del JSON.
  - La misura senza breaker è esatta: campo, mano e floodgate (`field + 0,9·hand + 0,8·floodgates`) si calcolano prima delle run dei breaker, e `probe-line` rigioca le stesse linee con valori identici a `combos` (0 differenze).
  - Il livello 2 è `probe-line --vs X --moves`.
- **I tempi.** Quando ho fatto le corse il JSON aveva solo i secondi della misura intera. Per dividerli ho usato una patch locale **mai committata**, in un worktree nello scratchpad: in `probe.rs` stampa su stderr i tempi delle parti di `measure_from` e il valore di `quick_value` (B6, due run).
  - La stessa patch legge due variabili d'ambiente: quante finestre provare per ogni risposta (oggi 16) e quanti giri del ciclo degli usi (oggi 3).
  - Così ho misurato cinque gradini, tutti senza breaker e senza `countered`. `wNuK` vuol dire: ogni risposta provata alle prime N finestre in cui è offerta, con K giri del ciclo degli usi.
  - Le corse dei tempi sono `probe-line` sulle linee salvate, 4 mani alla volta sui 4 core.
  - Dopo la PR #71 la CLI dà `seconds_by_part`, `--quick-value` e `--no-breakers`; il numero di finestre resta da esporre.

### Dove va il tempo della misura completa
Su 4490 board, 2648 s in tutto (mediana 0,47 s a board, media 0,59 s, p90 1,19 s):

| parte di `measure_from` | quota del tempo |
|---|---:|
| run con l'avversario che passa | 1,9% |
| **ciclo delle risposte** (ogni risposta provata fino a 16 finestre, poi fino a 3 giri degli usi; mediana 3 risposte) | **69,2%** |
| run con tutte le risposte insieme | 1,6% |
| run con la risposta dell'avversario alla prima interruzione (`countered`) | 3,0% |
| breaker e turno dopo (`follow_up`) | 24,2% |

**La misura «senza breaker» costa il 73% di quella completa.** Come gradino quindi risparmia poco: il costo sta nel ciclo delle risposte, e dentro il ciclo nelle finestre, non negli usi (16 finestre senza usi costano già il 64%).

| gradino | tempo a board (mediana) | quota del tempo della completa |
|---|---:|---:|
| B6, `quick_value` (due run) | 0,019 s | 3,7% |
| w2u0 | 0,063 s | 13% |
| w6u0 | 0,149 s | 32% |
| w8u1 | 0,194 s | 42% |
| w16u0 | 0,279 s | 64% |
| senza breaker (w16u3) | 0,321 s | 73% |
| completa | 0,465 s | 100% |

### Quanto ogni gradino ordina come la misura completa
Mediane sulle mani. NDCG@3 usa come guadagno la misura completa.

| gradino | Spearman | Kendall tau-b | NDCG@3 (media) | mani con NDCG@3 < 0,9 | #1 vero oltre la 15ª | #1 vero oltre la 30ª |
|---|---:|---:|---:|---:|---:|---:|
| stima | 0,57 | 0,42 | 0,82 | 41/81 | 20 | **10** |
| B6 | 0,64 | 0,58 | 0,81 | 40/81 | 18 | 6 |
| w2u0 | 0,76 | 0,66 | 0,86 | 34/81 | 12 | 4 |
| w6u0 | 0,90 | 0,79 | 0,94 | 19/81 | 7 | 3 |
| w8u1 | 0,92 | 0,83 | 0,95 | 15/81 | 4 | 2 |
| w16u0 | 0,95 | 0,89 | 0,98 | 6/81 | 3 | 1 |
| senza breaker | 0,96 | 0,89 | 0,98 | 5/81 | 2 | 1 |

Il #1 vero nell'ordine della stima: mediana 4ª posizione, 90° percentile 33ª, massimo 57ª. **L'app di oggi lo perde in 10 mani su 82**: trova il #1 vero in 72, e il suo #1 perde in media l'1,7% del valore, al massimo il 50%.

**Perché le finestre contano.** Nelle 5 mani dove il gradino a 2 finestre sbaglia (Branded, Radiant Typhoon Zoodiac, Snake-Eye, Orcust, Dracotail), le risposte della board giusta fermano qualcosa solo a una finestra oltre la seconda. Il gradino dà campo 0,00, la completa 0,68-1,29.

### Tagli contro l'app di oggi (`--ref-app`)
«#1 uguale o migliore» conta anche le mani dove il taglio trova una board migliore di quella dell'app. Il valore perso è quello del #1 rispetto al #1 dell'app: negativo vuol dire guadagnato. La colonna «tempo» conta la misura completa per intero dopo il gradino; «tempo con riuso» stima il costo se il gradino è un prefisso della misura completa, cioè se la completa riprende da dove il gradino si è fermato.

| gradini (board tenute) | #1 identico | #1 uguale o migliore | top 3 ≥ 2 su 3 | valore perso (media / max) | tempo | tempo con riuso (stima) |
|---|---:|---:|---:|---:|---:|---:|
| oggi: stima → 30 → completa | 82/82 | 82/82 | 82/82 | 0 / 0 | 100% | 100% |
| stima → 20 → completa | 73/82 | 73/82 | 77/82 | 1,4% / 32,3% | 71% | 71% |
| stima → 20 → w2u0 → 10 → completa | 71/82 | 71/82 | 67/82 | 2,1% / 32,3% | 46% | ~40% |
| stima → 40 → w2u0 → 15 → completa | 72/82 | 77/82 | 71/82 | 0,6% / 33,7% | 70% | ~60% |
| stima → 30 → w6u0 → 15 → completa | 80/82 | 81/82 | 79/82 | 0,3% / 24,6% | 86% | ~66% |
| **stima → 30 → w8u1 → 15 → completa** | **81/82** | **82/82** | 78/82 | **0 / 0** | 97% | **~71%** |
| stima → 30 → w8u1 → 10 → completa | 77/82 | 78/82 | 74/82 | 0,4% / 15,9% | 79% | ~62% |
| stima → 30 → senza breaker → 15 → breaker | 82/82 | 82/82 | 81/82 | 0 / 0 | 88% | 88% |
| stima → 30 → senza breaker → 10 → breaker | 79/82 | 79/82 | 77/82 | 0,2% / 15,9% | 84% | 84% |

Contro il riferimento «tutte misurate» nessuna scala che parte dalle prime 30 della stima può fare meglio dell'app (72 su 82). Partire da 40 aiuta poco: «stima → 40 → w2u0 → 15» trova il #1 vero in 69 mani, 71 di pari valore.

### Livello 2 (gradino 4)
Solo sulla prima metà del campione, la mano `-0` di ogni deck.

`probe-line --vs <mdm>/snake_eye.ydk --moves` (contro Branded per la mano Snake-Eye) sulle prime 8 board di ogni mano per la misura completa: 328 board, circa 40 minuti di CPU. Il livello 2 ordina per `left` (quota delle mosse dell'avversario che restano con tutte le tue risposte, più bassa è meglio), a parità per `value`.

- **Due mani escluse, da segnalare.**
  - In `railway-0` il livello 2 risponde «0 lines» per tutte le board, anche se le 12 linee di Snake-Eye sono state lette.
  - In `elfnote_kewl_tune-0` dà `matchup failed: the engine rejected the response` su 8 board su 8. Il rifiuto arriva su una SelectCard con hint 506 (aggiungi alla mano).
- **Ordine.** Sulle 39 mani rimaste, l'accordo tra misura completa e livello 2 sulle prime 8 è debole: Spearman 0,52 e Kendall 0,46 in mediana.
- **Dove sta il #1 del livello 2 nell'ordine della misura completa** (38 mani dove il livello 2 distingue): primo in 19, tra i primi 3 in 26, tra i primi 4 in 28, tra i primi 6 in 35.
- **Costo.** 7,0 s a board in mediana (p90 16,9 s), più circa 10-25 s per leggere le linee del deck una volta. Le 4 board del gradino 4 costano quindi circa 28 s, più di tutta la misura completa di oggi (30 × 0,53 s).
- **Conclusione.** Il gradino 4 del piano («livello 2 su 3-4») vede la board che il livello 2 preferisce solo in 28 mani su 38. Per vederla in 35 su 38 servono 6 board, e il costo raddoppia. Propongo di tenere il livello 2 a richiesta, come oggi nell'app, e di non metterlo nella cascata automatica. Va rifatto contro più deck: qui c'è il solo Snake-Eye.

### Conclusione e proposta per B14
1. **Il gradino «misura senza breaker» del piano va sostituito.** Costa il 73% della misura completa, perché il 69% del tempo è il ciclo delle risposte, non i breaker.
2. **Un gradino economico funziona solo se prova abbastanza finestre.** Con 2 finestre si perde la board giusta quando la sua risposta ferma qualcosa più avanti nel turno avversario: fino al 33,7% del valore. Con 8 finestre e un giro degli usi (42% del costo, Spearman 0,92 con la completa) la scala «stima → 30 → gradino → 15 → completa» tiene il #1 uguale o migliore in 82 mani su 82, senza perdite.
3. **Il risparmio c'è solo se il gradino è un prefisso della misura completa.** La completa deve riprendere dalle finestre 9-16 e dagli altri giri degli usi, senza rifare le run del gradino. Con il riuso la stima è circa il 71% del tempo di oggi; senza riuso il 97%, cioè nessun risparmio.
   - In `probe.rs`: un `Parts` con finestre per risposta e giri degli usi, e la misura completa che parte dai risultati per risposta già calcolati.
   - Lo fa la corsia 8; i numeri le sono stati mandati.
4. **Variante sicura senza codice nuovo nella misura:** «stima → 30 → misura senza breaker → 15 → breaker», 82 su 82, 88% del tempo. La misura senza breaker è già un prefisso della completa.
5. **Il dimezzamento con lo stesso #1 non si raggiunge con questi gradini.** Il limite è il primo taglio della stima: per non perdere il #1 di oggi bisogna partire dalle prime 30. Serve un primo ordinamento migliore. È B9: stima più B6 più misura economica, §2.
6. **Il livello 2 resta fuori dalla cascata automatica.** Ordina in modo diverso dalla misura completa (Spearman 0,52) e costa 7 s a board: si tiene a richiesta (sopra).
7. **Da rifare quando il gradino esiste nel codice:** gli stessi comandi senza la patch, con il tempo vero della valutazione nell'app (comprese le varianti di zona, che qui non ci sono).

## 2. Taratura della stima in stile Texel (preparazione di B9)

**Domanda.** Di quanto si può avvicinare l'ordine della stima rapida a quello della misura, con caratteristiche calcolabili dalla board?

### Metodo
`scripts/research/texel_fit.py`, sulle stesse 4490 board del §1 (82 mani, 41 deck):
- **Coppie.** Per ogni coppia di board della stessa mano con misure distanti più di 0,1, la probabilità che la migliore stia sopra è σ(s_a − s_b), con s = a·`quick_score` + Σ w_k·x_k. Sono 84 566 coppie.
- **Fit.** Newton sulla log-verosimiglianza media, più λ·|θ − θ₀|². θ₀ è la stima di oggi: a tarato da solo, w = 0. Le caratteristiche sono standardizzate sul gruppo di addestramento.
- **Verifica.** 70% dei deck per il fit e 30% lasciato fuori, con le due mani di un deck sempre dalla stessa parte. 20 divisioni a caso (seme 1), metriche sommate sulle divisioni:
  - **inversioni nella top 8**, come C5.7: coppie tra le 8 board che la stima mette prime, ordinate al contrario dalla misura per più di 0,1;
  - coppie giuste, Kendall e posizione del #1.
- **Caratteristiche della board** (27), da JSON e `data/cards.db`:
  - mostri nelle Zone Principali e nella Zona Extra;
  - mostri dell'Extra Deck, Link, Xyz, coperti;
  - Magie scoperte, Continue e Terreno, Magie e Trappole coperte, equipaggiamenti;
  - carte sul campo con «Quick Effect», «negate», o rimozione (distruggere, bandire, rimandare) nel testo;
  - carte coperte con «negate»;
  - carte in mano, handtrap, altre carte rapide in mano;
  - Cimitero, bandite;
  - Evocazioni, Evocazioni Speciali, ricerche, costo del Deck, LP pagati, carte usate, scelte della linea.
- **Sonde economiche** (`--probe`): il valore B6 e la misura w1u0 (una finestra per risposta, senza usi né breaker), misurati con la patch del §1. Insieme costano 0,06 s a board in mediana.
- **Comandi.**
  ```
  python3 scripts/research/texel_fit.py <w>/all82 --splits 20 --lam 0.01 0.1 1
  python3 scripts/research/texel_fit.py <w>/all82 --probe <w>/all82_w1u0 --splits 20 --lam 0.01 0.1 1 [--only-probe]
  ```

### Risultati (deck lasciati fuori)

| stima | inversioni top 8 | coppie giuste | Kendall | #1 nelle prime 10 | #1 nelle prime 15 |
|---|---:|---:|---:|---:|---:|
| oggi (`quick_score`) | 1971/13440 (14,7%) | 72,2% | 0,48 | 347/480 | 368/480 |
| + 27 caratteristiche della board, λ 0,01 | 2218/13440 (16,5%) | 74,7% | 0,48 | 350/480 | 368/480 |
| + 27 caratteristiche, λ 0,1 | 2095/13440 (15,6%) | 75,0% | 0,49 | 350/480 | 363/480 |
| + 27 caratteristiche, λ 1 | 1977/13440 (14,7%) | 74,5% | 0,47 | 337/480 | 364/480 |
| + B6 e w1u0, λ 0,01 | **1627/13440 (12,1%)** | **79,6%** | **0,61** | **360/480** | **399/480** |
| + B6 e w1u0, λ 0,1 | 1776/13440 (13,2%) | 77,7% | 0,56 | 356/480 | 383/480 |
| + caratteristiche + B6 e w1u0, λ 0,1 | 1701/13440 (12,7%) | 79,1% | 0,56 | 354/480 | 376/480 |

Sulla sola prima metà (41 mani) i numeri erano simili: le caratteristiche al massimo da 12,6% a 11,8% di inversioni, le sonde coppie giuste dal 73,6% all'80,4%.

Pesi (λ 0,01, per unità; deviazione tra le divisioni) delle caratteristiche più forti:
- Xyz +0,52 ± 0,09: la stima di oggi sottovaluta gli Xyz, come dice anche `ricerca-letteratura.md` §7.1;
- equipaggiamenti −0,91 ± 0,31;
- mostri coperti −0,39, Trappole coperte −0,30;
- carte in mano −0,17, mostri nelle Zone Principali −0,16, mostri nella Zona Extra −0,15;
- carte sul campo con «Quick Effect» +0,16.

Le caratteristiche alzano un po' le coppie giuste, ma fuori campione peggiorano la top 8 e non spostano il #1: è sovradattamento su 41 deck.

### Conclusione
- **Le caratteristiche lineari della board non bastano.** Fuori campione la top 8 peggiora (14,7% → 15,6-16,5% di inversioni) e il #1 non migliora. L'obiettivo di C5.7 (60 su 1110, il 5,4%) è lontano. Quello che la stima sbaglia non è il numero di carte per zona, ma cosa fanno nel turno avversario.
- **Due run economiche del turno di prova lo vedono meglio.** Aggiungere B6 e w1u0 (0,06 s a board) porta:
  - le coppie giuste dal 72,2% al 79,6%;
  - le inversioni della top 8 dal 14,7% al 12,1%;
  - il #1 nelle prime 15 da 368 a 399 su 480.

  Per B9 conviene quindi una stima a due livelli: `quick_score` nella ricerca, e una sonda economica sulle board finali, con pesi tarati così. È anche il primo taglio migliore che manca al §1.
- **Caratteristiche che servirebbero dal motore** e che oggi la board esportata non ha. Le chiedo alla coordinatrice:
  1. le carte tolte all'avversario nel tuo turno: il suo campo, la mano, l'Extra Deck e il Cimitero a fine turno (lo stato del giocatore 1 non è nel JSON);
  2. i materiali Xyz di ogni mostro, cioè il numero di usi rimasti (la stima e `value.rs` usano un 2 fisso);
  3. per ogni carta sul campo, se il suo effetto «una volta per turno» è già stato usato in questo turno;
  4. i segnalini sulle carte, tue e dell'avversario;
  5. le bandite coperte distinte da quelle scoperte;
  6. le zone puntate dai Link e le colonne occupate.
- **Proposta per B9.** Rifare il fit dopo B5a (l'avversario nella misura), con le caratteristiche 1-3 e una sonda economica come caratteristica. La verifica resta sui deck lasciati fuori, con le metriche di questa sezione.

## 3. Controllo esterno con ygo-combo-solver (caso Invoked)

**Domanda.** Sull'obiettivo Invoked (Invoked Oceanus nella Zona Mostri Extra, S:P Little Knight, Hyperinvoked Aeon Evocato con 3 o più materiali), il problema è che il motore non offre l'azione o che la nostra ricerca non la trova?

### Come si rifà
Il solver è [ygo-combo-solver](https://github.com/Armytille/ygo-combo-solver) (AGPL), compilato fuori dal repository; nessuna riga del suo codice entra in DuelForge.
- **Sorgenti.** Il fork macOS ([96jonesa/ygo-combo-solver](https://github.com/96jonesa/ygo-combo-solver), e0c7221 del 17/09) ha già un ramo POSIX dell'arena: `mmap` al posto di `VirtualAlloc`, `mprotect` più un gestore dei fault al posto di `GetWriteWatch`. Il gestore copre sia SIGBUS (macOS) sia SIGSEGV (Linux).
- **Core.** Il core è lo stesso del nostro submodule, `vendor/ygopro-core` baa3881 (v11.0-100), con il suo Lua 6e22fed. Il fork ne vorrebbe un altro (5a985af). Le cinque patch dell'arena (seme dell'hash di Lua, allocatore, `OCG_DuelQueryProcessorState`) si applicano senza conflitti con `tools/fetch_solver_deps.sh` e `COMMIT=baa3881…`. Ho saltato solo la parte che scarica gli script.
- **Compilazione.** Niente premake: g++ 13 a mano, `-O2 -ffp-contract=off`, Lua compilato come C++ con `-include luaconf-customize.h`, sorgenti LZMA di `edo9300/edopro` (`gframe/lzma`) e `-lsqlite3`. L'unica correzione serve per libstdc++: `-include cmath` per `operators.cpp`. Il binario parte e si ferma alla prima richiesta senza errori.
- **Dati.** La cartella di lavoro punta ai nostri `data/cdb` (`cards.cdb`, `release-*`, `prerelease-*`, come carica `ygo-engine`) e `data/scripts`. Quindi il core e gli script sono gli stessi della nostra ricerca.
- **Modello di duello.** Anche in modalità «solo obiettivo» il solver vuole un replay come modello: parametri e mazzi. `scripts/research/yrp_template.py` scrive un `.yrp` senza risposte con Master Rule 5, 8000 LP, il nostro deck e un deck avversario. L'avversario è Snake-Eye delle liste MDM, perché serve un Extra Deck da bandire ad Aeon. Il solver lo rigioca con `MSG_RETRY 0`.
- **Comandi.**
  ```
  python3 scripts/research/yrp_template.py tests/fixtures/decks/invoked-siracusa-md.ydk inv_vs_snake.yrp --opponent <mdm>/snake_eye*.ydk
  combosolver inv_vs_snake.yrp --workdir <wd> --deck tests/fixtures/decks/invoked-siracusa-md.ydk --hand "<codici>" --no-ref \
      --target 'Invoked Oceanus@mzone' --target 'S:P Little Knight@mzone' [--summon-min 'Hyperinvoked Aeon' --resolve 'Hyperinvoked Aeon'] \
      --threads 1 --solve-ms 60000 --seed 1
  duelforge-cli combos --deck tests/fixtures/decks/invoked-siracusa-md.ydk --hand "<nomi>" --goal "Invoked Oceanus@extra;S:P Little Knight" --seconds 60 --threads 1 --top 3 --lines
  ```
- **Mani.** Quelle del banco che stanno nel deck di Siracusa: `inv-r1`, `inv-r4`, `inv-r5` e `inv-r6` di `tests/fixtures/hands/bench.tsv`. `inv-remi`, `inv-aleister`, `inv-r2` e `inv-r3` hanno carte che questo deck non gioca (Droll & Lock Bird, Maxx "C", Ghost Ogre, The Book of the Law): la nostra CLI le rifiuta.

### Cosa sa chiedere il solver
- Il bersaglio dice la zona solo come `mzone`, senza distinguere la Zona Mostri Extra. `@extra` per lui è l'Extra Deck e non è una zona valida per un bersaglio.
- Il numero di materiali non si chiede. `--material` chiede «almeno un materiale di questi Attributi», `--summon-min` e `--resolve` chiedono che Aeon sia Evocato e che il suo effetto si risolva.
- Il numero di materiali di Aeon si legge comunque dalla linea: è il «max» della selezione del suo effetto («bandisci fino a N carte», N = materiali). Se le carte da bandire sono meno, il max scende, quindi è un limite inferiore.

### Risultati
Un thread, 60 s di budget per il solver (più 10 s della sua «sonda» iniziale), 60 s per la nostra ricerca.

| mano | nostra ricerca, Oceanus@extra + S:P | nostra ricerca, Oceanus ovunque + S:P | solver, A: Oceanus + S:P | solver, B: A + Aeon Evocato e risolto |
|---|---|---|---|---|
| inv-r1 | no (17 803 nodi) | no (20 931 nodi) | sì, 2,0 s di rollout | sì, 7,8 s |
| inv-r4 | no | no (24 884 nodi) | sì, 2,3 s | sì, 19,4 s |
| inv-r5 | no | no (22 359 nodi) | sì, 2,0 s | sì, 13,6 s |
| inv-r6 | no | no (20 106 nodi) | sì, 1,9 s | sì, 4,1 s |

Il solver scrive 16 linee per corsa, ordinate per carte spese. Le ho rigiocate tutte (`--verbose`) per leggere la zona di Oceanus e i materiali di Aeon:
- **Zona di Oceanus.** È sempre nella Zona Mostri Principale, in 128 linee su 128. S:P Little Knight sta nella Zona Extra.
- **Aeon.** Nelle corse B ha 2 materiali in 61 linee su 64 (1 nelle altre tre, dove le carte da bandire erano meno). Anche nelle corse A, quando Aeon compare, ha 2 materiali.
- **Corse più lunghe.** Con `--optimize`, 180 s e fino a 64 linee scritte per mano (r1, r5, r6): su 192 linee, Oceanus è sempre nella Zona Principale. Aeon ha 2 materiali in 139 linee e 1 in 51, e in 2 linee non si risolve. Nessuna linea ha Aeon a 3 materiali.

La linea più economica del solver per r1 (A, 11 azioni, 6 carte spese) usa Aleister the Reminiscent, Invocation, Invoked Elysium, Invoked Mechaba, Crowley e Artemis, the Magistus Moon Maiden, e finisce con Oceanus e S:P Little Knight. Passa per i Magistus come le guide del proprietario, ma senza Aeon.

### Conclusione
- **Per l'obiettivo di base (Oceanus + S:P) il difetto è della nostra ricerca, non del motore.** Con lo stesso core, gli stessi script e la stessa mano, il solver trova una linea in 2 s in tutte e 4 le mani. La nostra ricerca per obiettivo non la trova in 60 s e 20 000 nodi, nemmeno senza il vincolo sulla zona. Esplora molto (2 900-4 900 board) ma resta sulle board che la stima preferisce (Caliga o Mechaba con S:P). È proprio il caso di B4: una distanza dall'obiettivo che ordina i fratelli.
- **Per Aeon a 3 materiali e Oceanus nella Zona Extra il controllo non decide.** La grammatica del solver non sa chiederli, e la sua ottimizzazione (meno carte spese) spinge proprio verso Aeon a 2. Il fatto che in 320 linee non compaiano non dimostra che il motore non le offra.
  - Si chiude solo con una ricerca che li chiede davvero: B8, con il posto di ogni carta e i vincoli sui materiali.
  - In alternativa, rigiocando nel motore la linea delle guide, passo per passo, con `play --steps`.
- **Proposte per il piano.**
  - **B4.** Le 4 mani e l'obiettivo di base entrano nell'insieme fisso di «obiettivi raggiungibili» (`tests/fixtures/research/invoked-goals.tsv`). Oggi la nostra ricerca è a 0 su 4, il solver a 4 su 4. La causa principale è nella diagnosi qui sotto.
  - **Idee dal solver per B4.** Nella sua uscita il solver dichiara una «sonda» iniziale (10 s, 7 000 stati) e poi rollout NRPA guidati dal conteggio delle Evocazioni che il bersaglio richiede («1 Fusion, 1 Link»). Quel conteggio è la versione più semplice della distanza rilassata di B4: costa poco, e l'ha già chi gioca su questo motore.
  - **B8.** Il vincolo «materiali ≥ N» va chiesto in modo esplicito. Nessuno dei due strumenti lo ha oggi.

### Diagnosi: dove la nostra ricerca perde la linea (richiesta della coordinatrice)
**1. Il motore accetta la linea del solver.**
- Per leggere le linee del solver ho fatto una patch locale di sola stampa nel clone esterno: con `--verbose` stampa l'etichetta e i byte di ogni risposta registrata.
- Ho tradotto la linea più economica di r1 nei passi di `duelforge-cli play --steps`. Il nostro motore offre ogni azione e arriva alla stessa board: Oceanus nella Zona Principale, S:P Little Knight nella Zona Extra, in mano Aleister the Invoker e Ash Blossom. I passi:
  ```
  Normal Summon Crowley;No;Artemis;Select Crowley;Activate Aleister the Reminiscent;Select Artemis;Yes;Select Invoked Mechaba;
  Select Invocation (Deck);Activate Invocation - "Sword";Select Invoked Elysium;Select Artemis;Select Invoked Mechaba;
  Special Summon S:P;Select Aleister the Reminiscent;Select Invoked Elysium;No;Activate Invocation - "Sword" (GY);
  Select Aleister the Reminiscent;Activate Invocation (Hand);Select Invoked Oceanus;Select Crowley;Select Aleister the Reminiscent
  ```
- L'unica differenza: il solver attiva "Sword" nella finestra di catena dopo la ricerca di Reminiscent. Il nostro `play` passa quella finestra da solo, e "Sword" (Magia Rapida) si attiva subito dopo dalla Main Phase, con lo stesso effetto.
- Quindi **non è un difetto del motore**.

**2. La causa principale: nel modo obiettivo la ricerca sceglie per prime le carte dell'obiettivo anche quando la scelta le manda via.**
- `dive_order` mette per prima ogni azione che `Goal::involves` riconosce, cioè ogni azione la cui carta è una carta dell'obiettivo. Lo stesso segno (`wanted`) dà la precedenza anche alle alternative messe in coda.
- Ma `involves` guarda solo la carta, non dove va. Il costo di Aleister the Reminiscent («bandisci 1 Invoked dall'Extra Deck») offre Invoked Oceanus nella lista, e la ricerca lo sceglie per primo: Oceanus è bandito, e l'obiettivo diventa impossibile su tutto quel ramo.
- **Misura**, 3000 nodi, 40 board per mano, con il `main` di oggi:
  - con l'obiettivo «Invoked Oceanus», Oceanus è bandito in 40 board su 40 di r1, contro 0 su 40 della ricerca senza obiettivo, che trova Oceanus in campo in altre board;
  - con «Oceanus + S:P» è bandito in 151 board su 160 delle quattro mani.
- **Seconda forma dello stesso difetto.** Corretto il costo, in r1 è S:P Little Knight a finire bandito (30 board su 60): la ricerca la sceglie per prima come materiale di Fusione che Invocation bandisce.
- **Prova.** Una patch locale, mai committata, in un worktree nello scratchpad con la variabile `DIAG_GOAL_LEAVING`:
  - toglie il segno `wanted` quando la selezione ha un suggerimento del motore che manda via la carta: Tributo, scarto, distruzione, bandire, Cimitero, Deck (`HINTMSG` 500-504 e 507) e materiali di Fusione, Synchro, Xyz e Link (511-513 e 533);
  - con questa patch, un core per corsa e 60 s, l'obiettivo «Oceanus + S:P» si raggiunge in **3 mani su 4**: r5 in 0,1 s, r6 in 7,9 s, r4 in 10,7 s. Senza la patch è 0 su 4.
  - «Oceanus@extra + S:P» si raggiunge in 1 mano su 4: r5 in 15,5 s, con S:P in una Zona Principale.

**3. Cosa resta in r1.**
- Con la patch r1 non arriva all'obiettivo in 60 s (24 000 nodi), ma arriva a pezzi della linea: Elysium + S:P in 54,8 s, Artemis + Elysium in 36,5 s.
- Con l'obiettivo «Oceanus + S:P + Invocation - "Sword"» trova una linea in 14,3 s: la carta in più nell'obiettivo guida la ricerca su un'altra strada.
- La linea del solver è lunga, 23 passi della CLI. La stima la preferisce poco, perché S:P e Oceanus valgono meno di Caliga e Mechaba, e il segno `wanted` aiuta solo nel passo in cui la carta compare.
- Questo è il caso di B4: una distanza dall'obiettivo (h_add sulla mappa dei collegamenti) che ordina i fratelli lungo tutta la linea, non solo all'ultimo passo.

**4. «Oceanus nella Zona Extra + S:P».**
- S:P Little Knight è un Link: si Evoca nella Zona Extra o in una Zona Principale indicata da una freccia Link.
- Con Oceanus nella Zona Extra, quindi, S:P deve andare in una Zona Principale puntata da un altro Link. Nella linea di r5 trovata con la patch succede.
- È un obiettivo più stretto di quello di base, non un errore del motore.

**Cosa cambiare, in modo generale** (in `search.rs`, per la corsia 2 o per chi prende B4):
1. **Il segno `wanted` solo per le scelte che portano la carta verso il campo.** Le scelte che la mandano via (i suggerimenti sopra) non lo prendono. Meglio ancora: vanno per ultime, quando la carta è una carta dell'obiettivo ancora da mettere in campo. Il suggerimento c'è già nella `Decision` (`hint`, usato in `probe.rs` per `MSG_SPSUMMON`). Nessun nome di carta.
2. **Una carta dell'obiettivo bandita, mandata al Cimitero o usata come materiale** quando l'obiettivo la vuole in campo, e non ce n'è un'altra copia raggiungibile, rende il ramo inutile per l'obiettivo. Se ne può tenere conto nella priorità, non come potatura, così il verdetto «impossibile» resta corretto.
3. **Test di regressione:** le 4 mani di `tests/fixtures/research/invoked-goals.tsv`. Oggi 0 su 4; con la correzione 1, 3 su 4. r1 resta il primo caso per B4.

## 4. Bibliografia (§7 di `ricerca-letteratura.md`)
Fatto in `ricerca-letteratura.md`, §8: verifica di ogni fonte con il segno di confidenza, correzioni e note sulle cinque aree mancanti. In breve:
- **Confidenza.** Su 75 fonti distinte, 22 (GitHub) sono state lette davvero, come codice o README. Le altre 53 sono confermate solo come esistenza e abstract. Da questo container il proxy blocca i siti degli articoli, quindi nessun testo intero è stato letto.
- **Correzioni.**
  - Il dato di ygorl «41 contro 62 su 100» non è nella fonte. 41 su 120 è l'esplorazione casuale; la politica clonata risolve 12 obiettivi su 40.
  - Type-GBFS non ha garanzie formali, solo risultati empirici.
  - L'articolo del 2011 sulle simmetrie è di Pochter, Zohar e Rosenschein.
  - I deck di WindBot nel README sono 47, non 66.
- **Aree nuove**, con cosa si adatta:
  - Hierarchical Portfolio Search: semi e stili per B7 e B11.
  - Hearthstone e LOCM: valutatore più ricerca corta, a sostegno di B9 e B12.
  - MAP-Elites: un archivio per descrittori al posto della top N, insieme a B14.
  - Best-first parallelo: B6 si misura in tempo alla board migliore, non in nodi al secondo, perché GBFS parallelo è patologico.
  - Simmetrie delle scelte (copie identiche offerte una volta): B2b e B3.

## 5. B9: il primo ordinamento della scala di misura

**Domanda della coordinatrice.** Quale ordinamento economico delle board mette il #1 di oggi tra le prime K col K più piccolo? Si possono usare la stima, il valore B6 (`quick_value`) e una misura economica, da sole, combinate in modo lineare o in sequenza. Con quel K, quanto costa la scala? Obiettivo: lo stesso #1 in 82 mani su 82, con al massimo il 55% del tempo di oggi.

### Metodo
`scripts/research/b9_order.py`, sulle stesse 82 mani e 4490 board del §1, con i tempi della stessa patch locale.
- **Riferimento.** L'app di oggi: misura completa sulle prime 30 della stima. Una scala «tiene il #1» se la migliore misura completa tra le K board che misura vale almeno quanto il #1 dell'app (o entro una tolleranza, `--tolerance`).
- **Ordinamenti.**
  - Da soli: la stima, B6, e le misure economiche `w2u0`, `w6u0` e `w8u1` del §1.
  - Combinati in modo lineare: stima + B6, stima + una misura economica, stima + B6 + una misura economica.
- **Fit delle combinazioni.**
  - Regressione logistica a coppie, sulle coppie della stessa mano con misure distanti più di 0,1.
  - Ogni caratteristica è standardizzata dentro la mano, così la scala di ogni deck non conta.
  - Divisione in 5 gruppi per deck: ogni mano è ordinata da un modello che non ha visto il suo deck.
- **Costo** rispetto a 30 misure complete (1520 s su 82 mani, 0,59 s a board in media):
  - ogni sonda dell'ordinamento si paga su tutte le board che si ordinano;
  - la misura completa si paga sulle prime K;
  - con il riuso, la misura economica è un prefisso della completa, che ne paga solo il resto.
- **Comandi.**
  ```
  python3 scripts/research/b9_order.py <w>/all82 <w>/all82_t w2u0=<w>/all82_w2u0 w6u0=<w>/all82_w6u0 w8u1=<w>/all82_w8u1 [--tolerance 0.02]
  ```

### K che serve per tenere il #1 (board ordinate: le prime 30 della stima)

| ordinamento | K per 82/82 | K per 80/82 | K per 78/82 | tempo con K per 82/82 | con riuso |
|---|---:|---:|---:|---:|---:|
| stima (oggi) | 30 | 30 | 27 | 100% | 100% |
| B6 | 30 | 27 | 23 | 103% | 103% |
| w2u0 | 30 | 25 | 23 | 113% | 100% |
| w6u0 | 25 | 18 | 13 | 117% | 89% |
| w8u1 | 20 | 11 | 10 | 112% | 82% |
| stima + B6 | 30 | 27 | 23 | 103% | 103% |
| stima + w2u0 | 30 | 25 | 23 | 113% | 100% |
| **stima + w6u0** | **20** | 18 | 11 | 102% | **79%** |
| stima + w8u1 | 20 | 13 | 10 | 113% | 83% |
| stima + B6 + w6u0 | 25 | 13 | 11 | 120% | 93% |

Partire da 40 o 60 board non aiuta: il K cresce più di quanto l'ordinamento migliora.

Pesi medi (caratteristiche standardizzate dentro la mano):
- stima + w6u0: +0,62 e +2,09;
- stima + B6 + w6u0: +0,60, +0,43 e +1,96.

Il peso di B6 è sempre piccolo accanto a una misura economica.

**K mediano 1, coda lunga.** Per la combinazione stima + w6u0, in metà delle mani il #1 di oggi è già il primo dell'ordinamento. Il K per 82 su 82 lo decidono 3-4 mani:
- **Sky Striker (K 20).** Il #1 vince solo per le carte guadagnate nel turno dopo (`follow_up.gained` 6 contro 0-1, +0,24). Si misura nella run del breaker peggiore, che nessuna sonda economica fa. Le altre board hanno lo stesso valore economico (0,48).
- **Primite (K 19).** Il #1 vale 3,76 contro 3,70 delle board che la misura economica preferisce. È quasi una parità (1,6%).
- **Branded (K 18).** Le risposte fermano qualcosa solo a finestre lontane: il campo vale 0,27 nella misura economica e 0,68 nella completa (§1).

Con una tolleranza del 2% il K per 82/82 resta 20 (79%); con il 5% scende a 13 con w8u1 (69%).

### Con il tempo fissato
Mani su 82 dove il #1 di oggi resta, al miglior K e numero di board ordinate per il budget, con il riuso:

| ordinamento | ≤ 45% | ≤ 55% | ≤ 65% |
|---|---:|---:|---:|
| stima | 66 (20→12) | 68 (20→14) | 70 (20→18) |
| B6 | 68 (20→11) | 72 (30→13) | 74 (30→17) |
| w6u0 | 71 (30→4) | 75 (30→8) | 78 (30→13) |
| stima + B6 | 73 (40→11) | 74 (30→14) | 75 (40→16) |
| **stima + w6u0** | 71 (30→5) | **76 (30→9)** | **79 (30→13)** |
| stima + w8u1 | 65 (20→7) | 72 (20→10) | 79 (30→10) |
| stima + B6 + w6u0 | 67 (20→8) | 73 (30→6) | 78 (30→11) |

Con «stima + w6u0, 30 → 9» (circa 55%) il #1 cambia in 6 mani su 82 e la perdita media è lo 0,76%:
- Branded −32,7%;
- Paleozoic −15,9%;
- Orcust −5,3%;
- Sky Striker −4,0%;
- D/D/D −2,9%;
- Primite −1,7%.

Con «30 → 13» (circa 65%) le mani che perdono sono 3, con perdita media 0,33%: Branded −21,8%, Sky Striker −4,0%, Primite −1,7%.

### Conclusione
- **L'obiettivo (82/82 al 55%) non si raggiunge** con stima, B6 e misure economiche, né da sole né combinate.
  - La scala più economica che tiene il #1 in tutte le mani è «stima + w6u0 sulle prime 30 della stima, poi misura completa sulle prime 20», al 79% del tempo con il riuso.
  - Al 55% si tiene il #1 in 76 mani su 82.
- **Il limite non è la combinazione lineare.** Sono due cose che le sonde economiche non vedono:
  1. il turno dopo il breaker (`follow_up`);
  2. le risposte che fermano solo a finestre lontane.

  Una terza causa, minore, sono le quasi-parità.
- **Proposte.**
  1. **Per 82 mani su 82 resta migliore la scala del §1:** gradino a 8 finestre più un giro degli usi, prefisso della completa, «30 → 15», circa il 71%. La migliore di B9, «stima + w6u0, 30 → 20», costa il 79%.
     - B9 serve solo se si accetta di perdere poco. Con «stima + w6u0 (pesi 0,62 e 2,09 sui valori standardizzati nella mano), 30 → 13» il tempo è circa il 65%: il #1 resta in 79 mani su 82, con una perdita media dello 0,33%.
     - La scelta tra le due strade è del proprietario: tempo contro #1 garantito.
  2. **Il dato che manca alla sonda economica è il turno dopo.** Se la misura economica registrasse anche le carte guadagnate nella Fase Finale e nel turno dopo della run in cui l'avversario passa (senza breaker), Sky Striker non sarebbe più un caso. Va chiesto a chi scrive la misura: è un contatore, non una run in più.
  3. **Le quasi-parità** (Primite, 1,6%) andrebbero trattate come parità nel confronto tra scale: con il 2% di tolleranza il K non cambia, ma il criterio diventa più onesto.
- **Da rifare senza la patch** quando `probe.rs` esporrà le finestre per risposta (corsia 8).

## 6. A1b, passo 1: dove la misura e il livello 2 non sono d'accordo

*Prototipo del passo 1 di A1b (`ricerca-piano-critico.md`), 22-23/10. Giro completo con la CLI di `main` 0d747f1; il prima e dopo della PR #93 è in fondo.*

**Domanda.** Su board già trovate, quando la misura del turno di prova e il livello 2 contro il meta ordinano due board al contrario, e perché?

### Dati e comandi
- **Board della caccia.** Le 82 mani del §1 (41 deck MDM, due mani di `combo_hunt --seed 7`), con le prime 5 board di ogni mano per misura completa e campi diversi (le zone non contano): 410 board.
- **Board dell'oracolo.** Le fixture di `tests/fixtures/oracle` con la lista del deck ancora uguale (`deck_hash`): Branded e quattro mani Invoked (inv-r1, r2, r5, r6).
  - Dracotail, Maliss e Tearlaments hanno liste cambiate. L'obiettivo Invoked non ha una linea salvata.
  - Una fixture ha la linea della sola board migliore. Per confrontarla, la stessa mano è cercata con il budget normale (seme della fixture, 4000 nodi), e la board dell'oracolo si aggiunge alle prime 4 della ricerca normale.
- **Livello 2 e verità.** `scripts/objective_bench.py`, cioè lo stesso banco oggettivo di `banco-oggettivo.md`:
  - `probe-line --vs <avversario> --moves` contro i deck del meta con `popRank` di Master Duel Meta (letto il 22/10 sera), con Lunalight saltato;
  - verità = 1 − `left`, media pesata con il meta, senza le coppie con meno dell'80% di mosse fatte su campo vuoto o senza linee.
  - La misura si legge dagli stessi run, con lo stesso codice: la CLI di `main` 0d747f1.
- **Disaccordo:** due board della stessa mano la cui verità differisce di almeno 0,05, mentre la misura (il composito di `bench.py`) mette prima l'altra.
- **Comandi.**
  ```
  python3 scripts/research/a1b_setup.py <w>/a1b --hunt <w>/all82 --decks <mdm> --oracle tests/fixtures/oracle
  python3 scripts/objective_bench.py <w>/a1b --decks <mdm> --hunt - --skip lunalight --jobs 4
  python3 scripts/research/a1b_report.py <w>/a1b --json <w>/a1b_pairs.json --fixture tests/fixtures/research/a1b
  ```
  Per le mani dell'oracolo: `combos --seed <seme> --nodes 4000 --top 8 --json <h>/<deck>-<fixture>.json --save-lines …`, poi `a1b_setup.py <w>/a1bo --hunt <h> --per-hand 4 --oracle tests/fixtures/oracle`.

### Come si classifica un disaccordo
Per la coppia A (preferita dalla misura) e B (migliore al livello 2), `a1b_report.py` controlla, in quest'ordine:

1. **turno dopo:** senza il valore del turno dopo (`follow_up`) la misura ordinerebbe bene;
2. **trappole in mano:** la misura conta come interruzione una Trappola nella mano di A;
3. **Extra Deck:** B ferma una quota delle Evocazioni dall'Extra Deck dell'avversario più alta di quella delle sue altre mosse, di almeno 0,1 più di A. Il kit della misura ha un solo mostro nell'Extra Deck;
4. **risposte insieme:** al livello 2 le risposte di A valgono di più una per una, ma insieme fermano meno di quelle di B;
5. **un posto** (Cimitero e bandite, mano, mostri, Magie e Trappole): quello che valgono al livello 2 le risposte da quel posto, B meno A, supera di almeno 0,1 quello che dà la misura;
6. **altro.**

Una coppia può avere più cause; la prima è la principale.

### Risultati
**Giro.** 1653 invocazioni (87 file di linee × 19 avversari), circa 9 ore su 4 core, ripartito due volte dopo i riavvii del container.
- Board con verità e misura: 406 su 415, in 86 mani.
  - Mancano le board di `elfnote_kewl_tune-0` e `tearlaments-1`: il livello 2 vi fallisce (risposta rifiutata dal motore, o «0 lines»).
  - Altri fallimenti parziali: Elfnote Kewl Tune mano 1 (19 avversari), Railway mano 0 (13). Sono i difetti dello strumento già passati alla corsia 4.
- Accordo generale tra misura e verità (Spearman, divisione per deck del banco): 0,78 sulla taratura e 0,73 sulla verifica.
- Nelle coppie dello stesso deck l'ordine è giusto 673 volte su 856 e 283 su 347.

**Disaccordi.** Sulle 800 coppie della stessa mano ce ne sono **59 (7,4%) in 20 mani su 86**.

| causa | principale | presente |
|---|---:|---:|
| Extra Deck | 19 | 24 |
| turno dopo | 10 | 10 |
| risposte insieme | 8 | 14 |
| Magie e Trappole | 4 | 19 |
| mostri | 5 | 21 |
| mano | 3 | 5 |
| Cimitero | 0 | 0 |
| trappole in mano | 0 | 0 |
| altro | 10 | 10 |

Per deck: Clown Crew 8, Dracotail 8, Snake-Eye 4, Paleozoic 4, White Forest Azamina 4, Megalith 4, Invoked 3, Phantom Knights 3, Warrior Combo 3, Sky Striker 3, poi 1-2 negli altri.

**Esempi, con la linea da rigiocare** (`tests/fixtures/research/a1b/<mano>.lines.json`, indici nel file):
- **Extra Deck, Clown Crew mano 1.**
  - La misura preferisce «Biancaviso, Flair, Soiree Operations coperta» a «Biancaviso, Clown Crew Finale coperta» (verità +0,30 per la seconda, misura −0,22).
  - Finale, che distrugge, ferma 3 delle 17 Evocazioni dall'Extra Deck di Elfnote. La misura le dà 0,21, il livello 2 0,36.
  - Il kit della misura ha un solo mostro nell'Extra Deck: una carta che colpisce le Evocazioni dall'Extra Deck vale poco nel kit.
- **Magie e Trappole e mostri, Dracotail mani 0 e 1.**
  - Le board con Dracotail Flame o Horn coperte battono quelle con Blazing Cartesia (verità +0,15-0,26).
  - La misura dà a Blazing Cartesia e a Dracotail Sting più di quanto fermano contro le linee vere (Sting 0,26 nella misura, 0,13-0,17 al livello 2).
- **Turno dopo, Paleozoic mano 0.** La board con due Paleozoic Dinomischus coperte batte quattro board con Clown Crew sul campo (verità +0,15). La misura le separa solo per il turno dopo (`follow_up`), che premia le altre.
- **Risposte insieme.** Snake-Eye mano 1, Clown Crew mano 1, Phantom Knights mano 1, Megalith mano 1, White Forest Azamina mano 0, Mitsurugi Yummy mano 0: la board peggiore ha risposte che valgono di più una per una, ma insieme fermano meno.

**Mani dell'oracolo** (5 mani, 25 board, 19 avversari): **13 disaccordi su 50 coppie**, 8 per «risposte insieme».
- In inv-r5 la board dell'oracolo (Baronne de Fleur nella Zona Extra, Hyperinvoked Aeon, Gamma Summon e Super Polymerization coperte) ha la misura più alta di tutte, con 1,30 sopra la seconda. La sua verità però è 0,47, contro 0,66 della board «Baronne de Fleur, Super Polymerization coperta» della ricerca normale.
- È peggio contro 17 avversari su 19 (Elfnote: `left` 0,71 contro 0,33).
- Ogni sua risposta da sola copre il 75% delle mosse di Elfnote, ma tutte insieme ne lasciano passare il 71%.
- L'ipotesi era che Super Polymerization, usata come risposta, prendesse come materiale la tua Baronne. La corsia 4 l'ha confermata e corretta (PR #93).
- In Branded la board dell'oracolo ha verità 0,57, contro 0,66 di una board della ricerca normale.

**Cosa dice per l'oracolo.** Con la CLI di 0d747f1, in 2 delle 5 mani (Branded e inv-r5) la board migliore dell'oracolo non è la migliore al livello 2. L'oracolo eredita le cecità della misura, come temeva il piano. Dopo la PR #93 lo è in tutte e 5 (sotto), ma vale la regola: i riferimenti dell'oracolo vanno letti con il livello 2 accanto, e una fixture dovrebbe tenere anche le board migliori al livello 2.

### Prima e dopo la PR #93 («risposte insieme»)
La corsia 4 ha confermato l'ipotesi del caso inv-r5 e l'ha corretta nel livello 2 (PR #93, `main` 1af365d). Ho rigiocato con la CLI di 1af365d le mani toccate, con gli stessi avversari e pesi:
- le 6 mani della caccia con la causa «risposte insieme»: Clown Crew mano 1, Megalith mano 1, Mitsurugi Yummy mano 0, Phantom Knights mano 1, Snake-Eye mano 1, White Forest Azamina mano 0. Sono 30 board;
- le 5 mani dell'oracolo, 25 board.

In tutto 209 invocazioni, poco più di un'ora.

| insieme di mani | disaccordi prima | dopo | spariti / nuovi | «risposte insieme» prima → dopo |
|---|---:|---:|---:|---:|
| 6 mani della caccia (60 coppie) | 23 | 14 | 9 / 0 | 14 → 5 |
| 5 mani dell'oracolo (50 coppie) | 13 | 5 | 8 / 0 | 8 → 0 |

- **La misura non cambia** (stessi punteggi): cambia solo la verità del livello 2. Sale in 8 board su 30 della caccia (da +0,05 a +0,13: tutte le board di Phantom Knights, la #0 di Clown Crew, Snake-Eye, Azamina, Megalith) e in 10 su 25 delle mani dell'oracolo.
- **Oracolo.** La board dell'oracolo di inv-r5 passa da 0,47 a 0,74 e quella di Branded da 0,57 a 0,71. Ora la board migliore dell'oracolo è la migliore anche al livello 2 in tutte e 5 le mani.
- **Sul giro intero** i disaccordi passerebbero da 59 a circa 50. Le altre 80 mani non sono state rigiocate: hanno solo disaccordi senza «risposte insieme», ma anche la loro verità può essere cambiata un po'.
- **Restano**, nelle mani rigiocate:
  - Extra Deck: 8 come causa principale, presente in 10;
  - turno dopo: 2;
  - «risposte insieme»: 1 (5 come causa secondaria), da guardare uno per uno.

### Conclusione e proposte per B5
1. **Extra Deck: la cecità più frequente** (19 disaccordi su 59 come causa principale, e 8 su 14 dopo la #93). È il punto B5a del piano: un Extra Deck dell'avversario realistico nel kit. I deck dove pesa di più sono Snake-Eye 4, Clown Crew 3, Invoked 3, Warrior Combo 3, Resonators 2 e Megalith 2. Le risposte che colpiscono le Evocazioni dall'Extra Deck (come Clown Crew Finale) valgono poco nel kit, che ha un solo mostro nell'Extra Deck.
2. **Turno dopo: 10.** La misura premia il turno dopo (`follow_up`) di board che contro il meta fermano meno: Paleozoic 4, Psychics 2, Live Twin, Phantom Knights, Clown Crew e Mitsurugi Yummy 1. Il peso di `follow_up` nel composito va tarato con il banco oggettivo (D17), non alzato.
3. **Magie e Trappole, mostri: in Dracotail e Clown Crew.** La misura dà a certe risposte sul campo (Blazing Cartesia, Dracotail Sting) più di quanto fermano contro le linee vere, e a certe carte coperte (Dracotail Flame e Horn, Finale) meno. Va guardato con la protezione dei pezzi e con le finestre del §1.
4. **Cimitero e trappole in mano: nessun caso** in queste board, con 19 avversari.
5. **Il rapporto si rifà** con `a1b_report.py` dopo ogni correzione di B5, sulle stesse board (fixture `tests/fixtures/research/a1b`, 24 file di linee, uno per mano con disaccordi). Le liste MDM usate sono quelle del 22/10 mattina: con liste diverse le linee non si rigiocano (`deck_hash` nel file).

Analisi dei disaccordi «Extra Deck» della corsia 4: `docs/plans/extra-deck-a1b.md`.

## 6b. L2-3: il livello 2 contro i soli tre deck più giocati, la prova onesta

**Domanda della coordinatrice.** La corsia 4 (`extra-deck-a1b.md`) ha provato L2-3: il livello 2 pesato con il meta, ma solo contro le aperture dei tre deck con il `popRank` più alto (Elfnote, Dracotail, Invoked). Sul campione dei disaccordi A1b ordinava giusto l'84% delle coppie, contro il 16% della misura. Quel campione però era scelto proprio dove la misura sbaglia. Qual è il risultato su tutte le board, senza giudicare L2-3 con se stesso?

### Metodo
`scripts/research/a1b_l23.py`, sui run già fatti del giro A1b del §6. Nessuna invocazione nuova; CLI di `main` 0d747f1.
- **L2-3:** 1 − `left` contro Elfnote, Dracotail e Invoked, pesato con il loro `popRank`.
- **Verità:** 1 − `left` contro tutti gli **altri** avversari misurati, pesata con il meta. Sono 16 (Lunalight saltato), mediana 16 per board. Così L2-3 non si giudica con i suoi stessi deck.
  - Per prudenza ho rifatto il conto anche senza i «parenti» stretti dei tre deck nella verità: Elfnote Kewl Tune e Magistus Fairy Tail, che restano 14.
- **Misura:** il punteggio del banco (interruzioni, handtrap, floodgate e resilienza con i pesi di oggi), e il composito con il turno dopo.
- **Coppie:** due board dello stesso deck la cui verità differisce di almeno 0,02, come nel banco oggettivo. Taratura e verifica sono la divisione del banco, per deck, seme 47.
- **Comando.**
  ```
  python3 scripts/research/a1b_l23.py <w>/a1b [--top 3] [--exclude elfnote_kewl_tune,magistus_fairy_tail]
  ```

### Risultati (406 board, 1219 coppie)

| ordinamento | coppie giuste, taratura | coppie giuste, verifica | tutte | Spearman taratura | Spearman verifica |
|---|---:|---:|---:|---:|---:|
| misura (punteggio del banco) | 673/857 (79%) | 305/362 (84%) | 978/1219 (80%) | 0,78 | 0,73 |
| misura con il turno dopo | 676/857 (79%) | 303/362 (84%) | 979/1219 (80%) | 0,78 | 0,70 |
| **L2-3** | **819/857 (96%)** | **329/362 (91%)** | **1148/1219 (94%)** | **0,97** | **0,95** |
| misura + 1·L2-3 | 757/857 (88%) | 330/362 (91%) | 1087/1219 (89%) | 0,83 | 0,78 |
| misura + 2·L2-3 | 778/857 (91%) | 334/362 (92%) | 1112/1219 (91%) | 0,87 | 0,81 |
| misura + 4·L2-3 | 802/857 (94%) | 337/362 (93%) | 1139/1219 (93%) | 0,91 | 0,85 |

- **Senza i parenti nella verità** (14 avversari, 1213 coppie) il quadro non cambia. L2-3 ordina giusto il 93% (95% in taratura, 90% in verifica, Spearman 0,97 e 0,94); la misura l'80%; misura + 4·L2-3 il 93%.
- **Quanti deck servono:**
  - **L2-1** (solo Elfnote, 8,1 s a board): 82% delle coppie (84% in taratura, 77% in verifica), poco sopra la misura.
  - **L2-3** (22,5 s a board in mediana, 48 s al 90° percentile): 94%.
  - **L2-5** (32,5 s): 93%.

  I tre deck sono il punto giusto.
- **Costo.** Il livello 2 contro tre deck costa circa 50 volte la misura completa (0,47 s), più la lettura delle linee dei tre deck, una volta per sessione (10-25 s l'una).

### Conclusione
- **L2-3 segue la verità del meta molto meglio della misura** (94% contro 80% delle coppie, Spearman 0,95 contro 0,73 in verifica). Non è un effetto del campione: qui ci sono tutte le board, giudicate su avversari diversi.
- **Limite della prova.** La «verità» è sempre il livello 2, contro altri deck. L2-3 e la verità condividono il meccanismo, quindi anche le sue cecità: l'avversario rigioca le sue linee migliori, non risponde in modo nuovo. La prova dice che tre deck bastano a stimare il livello 2 contro il meta. Non dice che il livello 2 sia la verità del gioco.
- **Proposta per B14.** L2-3 come ultimo gradino, sulle 3-5 board che restano dopo la misura completa: circa 70-110 s in tutto, più la lettura delle linee. Con il §1 il #1 della misura è già quasi sempre tra le prime 15. L2-3 decide l'ordine finale fra le migliori, dove la misura sbaglia una coppia su cinque.
  - Nell'app ha senso a richiesta, o in sottofondo dopo la prima classifica.
- **Misura + k·L2-3** non batte L2-3 da solo. La misura serve a scegliere quali board mandare al livello 2, non a correggerlo.

## 6c. Il kit concatenato della corsia 4, la prova onesta

**Domanda della corsia 4.** Tre cambi del turno di prova, provati dietro tre interruttori d'ambiente: branch `claude/placed-cards`, 017aff9 (cioè `main` 9cc3005 con gli interruttori).
- `DUELFORGE_EXP_PERUSE`: la copertura di una risposta è quello che un suo uso ferma nella finestra migliore, non l'unione di tutte le finestre.
- `DUELFORGE_EXP_CASCADE`: conta anche una sonda persa perché è stata fermata quella da cui dipende.
- `DUELFORGE_EXP_CHAIN`: i passi del kit diventano una catena nell'ordine di gioco (tranne le giocate che non toccano il Deck), e fermarne uno ferma quelli dopo.

Sulle fixture dei disaccordi (§6, 81 coppie) la corsia 4 trovava le coppie giuste così:

| configurazione | coppie giuste |
|---|---:|
| base | 16% |
| CASCADE | 20% |
| PERUSE | 41% |
| PERUSE + CASCADE | 42% |
| tutte e tre | 38% |

Quel campione però è scelto dove la misura sbaglia.

### Metodo
- Ho rimisurato con la CLI di 017aff9 le 406 board del giro A1b: `probe-line --lines <a1b>/lines/<mano>.lines.json --json`, sola misura, nessun livello 2. 87 file per 4 configurazioni, 6,5 minuti sui 4 core.
- Le giudica `scripts/research/a1b_fair_measure.py` contro la verità del giro A1b: livello 2 contro tutti gli avversari, oppure senza i tre più giocati, come L2-3.
- Le coppie e le metriche sono quelle del §6b.
- **Controllo:** la configurazione base riproduce la misura del giro A1b (955 contro 956 coppie giuste su 1203). Cambia solo il codice tra 0d747f1 e 9cc3005.
```
for c in base peruse peruse_cascade all; do …probe-line --deck <mdm>/<deck>.ydk --lines <a1b>/lines/<mano>.lines.json --json <w>/remeasure/$c/<mano>.json; done
python3 scripts/research/a1b_fair_measure.py <w>/a1b base=<w>/remeasure/base peruse=… peruse+cascade=… tutte=…
```

### Risultati (406 board, verità su tutti gli avversari, 1203 coppie)

| misura | turno dopo | coppie giuste, taratura | coppie giuste, verifica | tutte | Spearman taratura | Spearman verifica |
|---|---|---:|---:|---:|---:|---:|
| giro A1b (0d747f1) | no | 79% | 82% | 79% | 0,78 | 0,73 |
| **base** (017aff9) | no | **79%** | **82%** | **79%** | **0,78** | **0,73** |
| base | sì | 79% | 82% | 80% | 0,78 | 0,71 |
| PERUSE | no | 75% | 69% | 73% | 0,56 | 0,42 |
| PERUSE + CASCADE | no | 77% | 69% | 75% | 0,62 | 0,52 |
| PERUSE + CASCADE | sì | 76% | 73% | 75% | 0,59 | 0,49 |
| PERUSE + CASCADE + CHAIN | no | 77% | 76% | 77% | 0,75 | 0,60 |
| PERUSE + CASCADE + CHAIN | sì | 78% | 77% | 77% | 0,75 | 0,60 |

Combinazioni senza PERUSE, cioè con la copertura di oggi (richiesta successiva della corsia 4):

| misura | turno dopo | coppie giuste, taratura | coppie giuste, verifica | tutte | Spearman taratura | Spearman verifica |
|---|---|---:|---:|---:|---:|---:|
| CASCADE | no | 79% | 82% | 80% (958) | 0,78 | 0,72 |
| CHAIN | no | 78% | 76% | 78% | 0,74 | 0,60 |
| CASCADE + CHAIN | no | 78% | 76% | 78% | 0,74 | 0,60 |
| **SUMMONS** | no | **81%** | **84%** | **82% (984)** | 0,77 | **0,74** |
| SUMMONS | sì | 81% | 81% | 81% (977) | 0,77 | 0,73 |
| SUMMONS + CASCADE + CHAIN | no | 78% | 73% | 76% | 0,72 | 0,57 |

CASCADE da solo è neutro: 958 coppie giuste contro 955, ρ 0,72 contro 0,73. CHAIN peggiora, e con CHAIN acceso CASCADE non cambia niente.

SUMMONS è l'opzione (d) della corsia 4, sulla CLI di 25d2994. Fuori dall'interruttore è identica a 017aff9, quindi la base resta la stessa. I pesi delle Evocazioni vengono dalla frequenza nelle linee vere (kit-mix): dalla mano 0,32, Normali 0,35, Extra Deck 0,99, invece di 0,3, 0,2 e 0,6.
- È l'unica configurazione che batte la base: 984 coppie giuste contro 955, anche sui deck di verifica (84% contro 82%).
- Contate coppia per coppia: 45 guadagnate e 16 perse, test del segno p = 0,0003.
- Il guadagno però è concentrato: Sky Striker +15/−0 e Clown Crew +15/−1 fanno 30 delle 45.
  - Le perdite sono sparse: Artmage −3, Phantom Knights +1/−4, Invoked, Psychics e Pumpking Zombies −2.
- Lo Spearman quasi non si muove (0,77/0,74 contro 0,78/0,73): migliora l'ordine dentro ai deck, non la scala tra deck.
- Con il turno dopo il guadagno in verifica sparisce (81% contro 82%).
- Insieme a CASCADE e CHAIN torna sotto la base, come CHAIN da solo.

Con la verità senza i tre deck più giocati (1219 coppie) l'ordine è lo stesso:

| configurazione | coppie giuste | Spearman in verifica |
|---|---:|---:|
| base | 80% | 0,73 |
| PERUSE | 74% | 0,43 |
| PERUSE + CASCADE | 76% | 0,54 |
| tutte e tre | 79% | 0,63 |
| CASCADE | 80% | 0,72 |
| CHAIN | 79% | 0,63 |
| CASCADE + CHAIN | 79% | 0,63 |
| SUMMONS | 82% | 0,73 |
| SUMMONS + CASCADE + CHAIN | 78% | 0,59 |

### Conclusione
- **Del kit concatenato nessun interruttore migliora la misura, in nessuna delle 7 combinazioni.** CASCADE da solo è neutro; le altre la peggiorano.
- **SUMMONS da solo (opzione d) è l'unico miglioramento: +29 coppie, 82% contro 79%, significativo (p = 0,0003).** Viene però per due terzi da due deck. Prima di accenderlo conviene ripeterlo sul banco obiettivo nuovo della corsia 3, e senza il turno dopo, che lo annulla in verifica.
  - PERUSE da solo fa il danno più grande: 79% → 73% delle coppie, Spearman in verifica 0,73 → 0,42.
  - CASCADE e CHAIN recuperano in parte, ma restano sotto la base: 77% e 0,60.
- Sul campione dei disaccordi il quadro era opposto: 16% → 42%. È l'effetto della scelta del campione: correggere dove la misura sbaglia costa di più dove oggi va bene.
- **Proposta:** non accendere PERUSE, CASCADE e CHAIN. La strada che migliora l'ordine resta il livello 2 contro i tre deck più giocati (§6b, 94%).
- **Regola per le prossime modifiche della misura:** si giudicano su tutte le board del giro A1b con `a1b_fair_measure.py`, non solo sulle coppie in disaccordo.
  - Costa pochi minuti: le linee e la verità ci sono già.
  - Con la verità che esclude i tre deck più giocati, il confronto con L2-3 resta onesto.

## 7. Ritaratura dei pesi del punteggio, con il turno dopo e l'opzione (d)

*23/10. Nuova base del banco oggettivo della corsia 3 (`claude/tools-qa` 6dce326: verità da `main` 1af365d, misure generiche da baf1d82). Ricerca con `main` 1fdde1f.*

**Domanda.** I pesi del profilo «Bilanciato» (interruzioni 1, handtrap 0,9, floodgate 0,8, resilienza 0,6, più il turno dopo 0,4·0,3·rebuild) vanno cambiati? Va acceso il peso del turno dopo? E l'opzione (d) della corsia 4 (pesi delle Evocazioni 0,32/0,35/0,99, `extra-deck-a1b.md`) regge sulla ricerca?

### Dati e comandi
- **Banco oggettivo nuovo:** 205 board di 41 deck, 145 di taratura e 60 di verifica (divisione per deck di `objective_bench.py`, seme 47). 185 coppie dello stesso deck con verità diverse di almeno 0,02 (135 e 50).
- **Giro A1b** (§6): 406 board, 1203 coppie (856 e 347), misurate con la base di 017aff9 e con l'opzione (d) (§6c). È un secondo insieme, più grande, con la stessa verità (livello 2 contro il meta) ma board diverse: le prime 5 di ogni mano della caccia.
- **Script:** `scripts/research/tune_follow_up.py`. È `tune_weights.py` con un quinto peso, che moltiplica il turno dopo di `bench.follow_up_value` (1 = oggi). Fa due controlli fuori campione:
  - la divisione del banco (taratura/verifica);
  - 50 divisioni 70/30 a caso per deck: ogni volta taro sul 70% e giudico sul 30%, contro i pesi di oggi sullo stesso 30%.
  - Con `--try` giudica pesi fissi su tutte le coppie, con il test del segno contro oggi.
  ```
  python3 scripts/research/tune_follow_up.py tests/fixtures/boards/objective-bench.json --splits 50 --step 0.05
  python3 scripts/research/tune_follow_up.py <results.json> --try 1,0.9,0.4,0,0 --try 1,0.9,0.8,0,0.7 ...
  python3 scripts/paired_test.py <out> --decks <mdm> --old <cli main> --new <cli con (d)> --measure-cli <cli con (d)>
  ```
- **L'opzione (d) sulla ricerca:** due build locali di 1fdde1f, mai committate.
  - (d) dappertutto, com'era la prova della corsia 4.
  - (d) solo nella misura: i valori delle carte (`card_values`), che guidano la ricerca, restano quelli di oggi.
  - `paired_test.py --measure-cli` su 90 mani, 4000 nodi, un thread. Le #1 dei due lati sono rimisurate con la misura (d).

### Taratura libera: non regge fuori campione
Sul banco nuovo, con la griglia a passo 0,05:

| pesi (interruzioni, handtrap, floodgate, resilienza, turno dopo) | taratura | verifica | ρ taratura | ρ verifica |
|---|---:|---:|---:|---:|
| oggi, senza turno dopo (1; 0,9; 0,8; 0,6; 0) | 84,4% | 96,0% | 0,87 | 0,63 |
| oggi, con turno dopo (…; 1) | 84,4% | 96,0% | 0,85 | 0,58 |
| tarati senza turno dopo (1; 0,9; 0,8; 0,1; 0) | 87,4% | 96,0% | 0,90 | 0,63 |
| tarati con turno dopo (1; 1; 0,8; 0; 0,7) | 89,3% | 92,0% | 0,90 | 0,63 |

- **Il massimo è piatto:** 5166 combinazioni stanno a una coppia dal migliore.
- **Sulle 50 divisioni a caso i pesi tarati non battono quelli di oggi.**
  - Senza turno dopo: meglio in 0 divisioni, peggio in 11.
  - Con turno dopo: meglio in 5, peggio in 19.
  - 135 coppie di taratura sono poche per cinque pesi: la taratura insegue il rumore.
- **Sul giro A1b**, quattro volte più grande, la stessa prova va meglio: i pesi tarati battono quelli di oggi in 35 divisioni su 50, +1,5-2,0% di coppie in media.

### Pesi fissi: resilienza a 0, floodgate a 0,4
Le tarature dei due insiemi concordano su una sola cosa: la resilienza va a 0 (0, 0,1 e 0,15 nelle diverse tarature) e il floodgate scende (0,2-0,4). La resilienza segue la verità quasi quanto il punteggio intero (ρ 0,74 da sola in taratura), ma è già contenuta nelle interruzioni: pesata di nuovo, conta due volte le stesse risposte. I pesi fissi, giudicati su tutte le coppie contro oggi:

| pesi | banco nuovo (185 coppie) | A1b, misura di oggi (1203) | A1b, misura (d) (1203) |
|---|---|---|---|
| oggi (1; 0,9; 0,8; 0,6; 0) | 87,6% | 80,4% | 82,8% |
| **1; 0,9; 0,4; 0; 0** | **90,0%: +6/−2, p = 0,29** | **83,7%: +63/−33, p = 0,003** | **85,2%: +39/−21, p = 0,027** |
| 1; 0,9; 0,8; 0; 0 | 90,0%: +6/−2 | 82,8%: +53/−35, p = 0,07 | 84,2%: +30/−23 |
| 1; 0,9; 0,8; 0,3; 0 | 86,5%: +0/−2 | 81,4%: +28/−16 | 82,7%: +8/−9 |
| 1; 0,9; 0,4; 0; 0,7 | 88,9%: +4/−2 | 84,9%: +72/−25 | 85,0%: +49/−29 |
| oggi con turno dopo (…; 1) | 87,6%: +3/−3 | 80,6%: +10/−6 | 82,1%: +13/−20 |

- **Fuori campione, sui deck di verifica, (1; 0,9; 0,4; 0) tiene le coppie e alza il ρ:**

  | insieme | coppie oggi | coppie (1; 0,9; 0,4; 0) | ρ oggi | ρ (1; 0,9; 0,4; 0) |
  |---|---:|---:|---:|---:|
  | banco nuovo | 96,0% | 96,0% | 0,63 | 0,70 |
  | A1b, misura di oggi | 81,8% | 83,0% | 0,73 | 0,80 |
  | A1b, misura (d) | 84,4% | 87,3% | 0,74 | 0,81 |

- **Dove guadagna:** nel giro A1b soprattutto Sky Striker +15/−0 e White Forest Azamina +10/−0, gli stessi deck dove guadagnava l'opzione (d). Perde su Resonators −5. Nel banco nuovo: Psychics +6, Snake-Eye −2.
- **Cautela:** i pesi sono nati dalla taratura su A1b, quindi lì il risultato è in parte nel campione. Il controllo indipendente è il banco nuovo: va nello stesso verso (+6/−2, ρ in verifica 0,63 → 0,70), ma con poche coppie non è significativo.
- **Effetto sulle #1:** sulle 90 mani della prova appaiata, ordinando le board trovate da `main` con i pesi nuovi, la #1 cambia in 9 mani. La #1 nuova vale al più 0,05 meno con i pesi di oggi (0,20 per Elfnote mano 0). La ricerca non usa questi pesi (`search.rs` non li legge): cambia solo l'ordine finale.
- **Verdetti del proprietario:** non decidono. Su 32 coppie con una preferenza sono d'accordo i pesi di oggi in 6, quelli nuovi in 4 e la verità del banco in 1.

### Il peso del turno dopo
- La verità è solo il turno dell'avversario: il turno dopo si può controllare contro di essa, non tarare.
- Il risultato è incerto e cambia con l'insieme:
  - sul banco nuovo non aiuta (ρ in verifica 0,63 → 0,58 con i pesi di oggi);
  - su A1b con la misura di oggi aiuta un poco (+72/−25 contro +63/−33);
  - su A1b con la misura (d) costa 12 coppie su Paleozoic.
- **Proposta:** il peso del turno dopo resta quello di oggi (0,4). È una scelta di prodotto («quanto conta poter ripartire»), non un dato del banco.

### L'opzione (d) sulla ricerca

| prova appaiata (90 mani) | meglio | peggio | pari | p | peggiore | secondi di ricerca |
|---|---:|---:|---:|---:|---|---|
| (d) dappertutto | 11 | 8 | 71 | 0,65 | Magistus Fairy Tail-0 −16,3%, bench-dracotail −16,0% | 926 → 668 |
| (d) solo nella misura | 10 | 5 | 75 | 0,30 | Vampire-1 −31,5%, Psychics-0 −8,3% | 926 → 678 |

- **Entrambe respinte:** una mano perde più del 5%. Con la corsia 4 (−41% su hunt-dracotail-0 con le sue liste) sono tre prove su tre.
- **Tenere i valori delle carte di oggi non basta.** La ricerca misura le board anche mentre cerca, e quelle misure la guidano: con la misura (d) trova board diverse. In hunt-vampire-1 le 10 board elencate non hanno nessuna firma in comune con quelle di `main`. Ho controllato che non sia rumore: `main` rifatto sulle stesse mani ridà le stesse board (10 su 10, 8 su 8).
- **Il guadagno di (d) sull'ordine viene in gran parte dagli stessi deck** (Sky Striker, White Forest Azamina) **che guadagnano con la resilienza a 0.** I pesi nuovi con la misura di oggi danno 83,7% delle coppie, contro 82,8% della sola (d), senza toccare la ricerca.

### Conclusione
- **(d): non si unisce.** Peggiora la ricerca in tutte e tre le prove, anche confinata alla misura, e il suo guadagno sull'ordine si ottiene in altro modo.
- **Pesi del punteggio: proposta resilienza 0,6 → 0 e floodgate 0,8 → 0,4**, il resto invariato, turno dopo compreso.
  - Va nello stesso verso su tre insiemi.
  - È significativo sul più grande (p = 0,003).
  - Alza il ρ sui deck di verifica (0,63 → 0,70 e 0,73 → 0,80).
  - Non tocca la ricerca, quindi la prova appaiata non serve. Cambia solo l'ordine finale: la #1 in 9 mani su 90, di poco.
- **Prima di accenderla**, una conferma sul banco di generalità della corsia 8 (deck fissati con `deck_hash`) sarebbe il controllo davvero indipendente.
- I pesi stanno in quattro posti da cambiare insieme. Sono codice di altre corsie, quindi non li tocco:
  - `scripts/bench.py` (`WEIGHTS`);
  - `composite` in `crates/ygo-cli/src/main.rs`;
  - `balanced` in `apps/desktop/src-tauri/src/lib.rs`;
  - il profilo «Bilanciato» in `apps/desktop/src/lib/scoring.ts`.

### Conferma sui deck freschi del banco di generalità (24/10)
*Thread «Confermare i nuovi pesi del punteggio», branch `claude/project-thread-4ly968`. CLI di `main` 4059075.*

**Domanda.** I pesi (1; 0,9; 0,4; 0) sono nati dalla taratura su A1b. Reggono su deck su cui nessun peso è mai stato tarato?

**Metodo.**
- I 20 deck freschi di `tests/fixtures/generality/sets.json`: le liste ricostruite con `generality.py rebuild`, uguali al manifesto (63 su 63).
- Le board come in A1b:
  - `combo_hunt.py --hands 2 --top 60 --seed 7 --nodes 3000`;
  - le prime 5 di ogni mano per la misura completa, con campi diversi (`a1b_setup.py --per-hand 5`).
- La verità come in A1b: `objective_bench.py --skip lunalight`, livello 2 contro i 19 deck del meta.
  - I `popRank` di Master Duel Meta sono letti il 24/10 alle 00:01.
  - Gli avversari sono solo le liste di taratura, così nessun deck fresco gioca contro sé stesso.
- **Costo:** 720 run, 9,2 ore di CPU, circa 2,5 ore sui 4 core.
  - Fur Hire mano 0 va oltre il limite di 300 s contro tutti: 195 board su 200.
  - 591 coppie dello stesso deck, con verità diverse di almeno 0,02.
- **I pesi erano fissati prima di questo giro: qui non si è tarato niente.** I risultati sono in `tests/fixtures/boards/generality-fresh.json`, compatti come `objective-bench.json`: solo numeri, nessuna lista.
  ```
  python3 scripts/research/tune_follow_up.py tests/fixtures/boards/generality-fresh.json --base 1,0.9,0.8,0.6,0 --try 1,0.9,0.4,0,0
  python3 scripts/research/tune_follow_up.py tests/fixtures/boards/generality-fresh.json --base 1,0.9,0.8,0.6,1 --try 1,0.9,0.4,0,1
  ```
  `--base` è nuovo: senza, `--try` si giudica contro i pesi di `bench.py`, che ora sono quelli nuovi.

**Risultati (195 board, 591 coppie).**

| confronto | coppie giuste | guadagnate/perse | p | deck meglio/peggio |
|---|---|---|---:|---|
| senza turno dopo: (1; 0,9; 0,8; 0,6) → (1; 0,9; 0,4; 0) | 80,4% → **86,0%** | +40/−8 | < 0,001 | 7/3 |
| con il turno dopo, come ordina l'app | 78,7% → **82,0%** | +32/−14 | 0,011 | 4/4 |

- **ρ con la verità**, su tutte le board: da 0,70 a 0,78.
- **Dove guadagna:** HERO +11/−0, Naturia +10/−0, Regenesis +6/−2, Sacred Beasts +4/−0 (con il turno dopo).
- **Dove perde** (con il turno dopo): Atlantean Mermail −3, Azamina Mitsurugi Dark Magician −3, Magistus Witchcrafter −3, Fiendsmith Unchained −2.
- **Il floodgate segue la verità al contrario** sui freschi: ρ −0,39 e −0,13 nelle due metà. A 0 la prova darebbe 88,1%, ma 0 non era la proposta: sarebbe tarato su questo giro.
- **Effetto sulle #1:** la #1 cambia in 2 mani su 39. La verità della #1 nuova è −0,034 e +0,011 rispetto a quella di oggi.

**Le tre prove insieme.**

| insieme | senza turno dopo | con il turno dopo |
|---|---|---|
| banco nuovo (185 coppie) | +6/−2 | +2/−3 |
| A1b (1203 coppie, nel campione) | +63/−33, p = 0,003 | non misurato |
| **freschi (591 coppie, fuori campione)** | **+40/−8, p < 0,001** | **+32/−14, p = 0,011** |

- **A1b non è rifatto.** I suoi `results.json` non sono nel repository, e rifarli costa circa 9 ore; i pesi vi erano tarati, quindi non sarebbe una prova nuova.
- **Il punto debole è il turno dopo.** Contato come lo conta l'app, il guadagno resta significativo sulle coppie, ma per deck è pari (4 meglio, 4 peggio). Il banco nuovo con il turno dopo è neutro.
- **La prova appaiata della ricerca** (`paired_test.py --decks <liste di taratura>`, 94 mani, 4000 nodi, un thread; CLI di `main` 42fe162 contro quella del branch, le due giudicate con i pesi nuovi): 1 meglio, 0 peggio, 93 pari, ACCEPTED. Secondi di ricerca 449 → 454.
  - La ricerca non legge i pesi. Nella CLI cambiano solo l'ordine finale e la scelta delle varianti di zona dei primi campi: le board elencate cambiano in 2 mani su 94.
  - La mano meglio è hunt-invoked-siracusa-ewcq2026-1 (+6,7%), per una variante di zona.

**Decisione: i pesi nuovi si accendono**, tutti e quattro i posti insieme: `bench.py`, `composite` della CLI, `balanced` di `lib.rs` e il profilo «Bilanciato» di `scoring.ts`. Il turno dopo resta a 0,4.
- **La misura economica** (`cheap_composite` della CLI, `cheap_value` di `lib.rs`) tiene il floodgate a 0,8. Sceglie quali board passano alla misura completa, quindi abbassarlo cambia le board misurate: va provato a parte, con la prova appaiata in modalità `--staged`.
- **Gli script di ricerca** con i pesi copiati (`a1b_report.py`, `b9_order.py`, `l23_final.py`, `objective_bench.py summary`) restano come erano: sono il registro dei loro esperimenti.

