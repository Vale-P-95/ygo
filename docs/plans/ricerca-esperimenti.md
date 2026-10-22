# Esperimenti di ricerca (corsia 7)

*Sessione sorella di ricerca, 22/10/2026. Esperimenti offline che servono al piano critico (`ricerca-piano-critico.md`), senza toccare il codice del motore o della ricerca. Gli script sono in `scripts/research/`; i dati generati (JSON della caccia, tempi, soluzioni) restano fuori dal repository.*

**Macchina e versione.** Container Linux con 4 CPU, `main` cc78f89 (dopo la PR #68), build release. Liste di Master Duel Meta scaricate il 22/10 con `scripts/mdm_decks.py` (41 deck).

## 1. Gradini di misura per B14

**Domanda.** Oggi l'app misura a piena fedeltà le prime 30 board della stima rapida, e il turno di prova è il 58% del tempo di `combo_hunt`. Con quali gradini, e quante board a ogni gradino, si dimezza il tempo tenendo lo stesso #1 e almeno 2 board su 3 della top 3?

### Dati e comandi
- **Board.** `combo_hunt.py --hands 1 --seed 7 --nodes 3000 --top 60` sui 41 deck MDM: una mano per deck, la mano `-0` della caccia. Ci sono 2190 board con la misura completa: mediana 60 a mano, minimo 12.
  ```
  python3 scripts/combo_hunt.py <mdm> --hands 1 --top 60 --seed 7 --json <w>/all60      # 4 shard con --shard K/4
  python3 scripts/research/b14_stages.py <w>/all60 --timing <w>/all60_t --cheap w1u0=<w>/all60_w1u0 --cheap w2u0=<w>/all60_w2u0 --cheap w4u1=<w>/all60_w4u1 [--ref-app]
  python3 scripts/research/b14_level2.py <w>/all60_t <w>/l2
  ```
- **Riferimento.** Il #1 «vero» è il migliore della misura completa su tutte le 60 board. Con `--ref-app` il riferimento è invece l'app di oggi, cioè la misura completa sulle prime 30 della stima.
- **Gradini dalla CLI.**
  - La stima rapida è `quick_score` del JSON.
  - La misura senza breaker è esatta: campo, mano e floodgate (`field + 0,9·hand + 0,8·floodgates`) si calcolano prima delle run dei breaker, e `probe-line` rigioca le stesse linee con valori identici a `combos` (0 differenze su 41 mani).
  - Il livello 2 è `probe-line --vs X --moves`.
- **I tempi non li dà la CLI.** Il JSON ha solo i secondi della misura intera. Per dividerli ho usato una patch locale **mai committata**, in un worktree nello scratchpad: in `probe.rs` stampa su stderr i tempi delle parti di `measure_from` e il valore di `quick_value` (B6, due run).
  - La stessa patch legge due variabili d'ambiente: quante finestre provare per ogni risposta (oggi 16) e quanti giri del ciclo degli usi (oggi 3).
  - Così ho misurato tre gradini economici, tutti senza breaker:
    - `w1u0`: ogni risposta provata alla prima finestra, senza ciclo degli usi;
    - `w2u0`: alle prime due finestre, senza ciclo degli usi;
    - `w4u1`: alle prime quattro, con un giro degli usi.
  - Le corse dei tempi sono `probe-line` sulle linee salvate, 4 mani alla volta sui 4 core.

### Dove va il tempo della misura completa
Su 2190 board, 1492 s in tutto (mediana 0,53 s a board, p90 1,42 s):

| parte di `measure_from` | quota del tempo |
|---|---:|
| run con l'avversario che passa | 1,8% |
| **ciclo delle risposte** (ogni risposta provata fino a 16 finestre, poi fino a 3 giri degli usi; mediana 3 risposte) | **70,3%** |
| run con tutte le risposte insieme | 1,6% |
| run con la risposta dell'avversario alla prima interruzione (`countered`) | 2,8% |
| breaker e turno dopo (`follow_up`) | 23,5% |

**La misura «senza breaker» costa il 74% di quella completa.** Come gradino quindi risparmia poco: il costo sta nel ciclo delle risposte.

| gradino | tempo a board (mediana) | quota del tempo della completa |
|---|---:|---:|
| B6, `quick_value` (due run) | 0,021 s | 3,5% |
| w1u0 | 0,046 s | 8% |
| w2u0 | 0,072 s | 13% |
| w4u1 | 0,139 s | 26% |
| senza breaker | 0,359 s | 74% |
| completa | 0,532 s | 100% |

### Quanto ogni gradino ordina come il successivo
Mediane sulle mani. NDCG@3 usa come guadagno la misura del gradino dopo.

| coppia di gradini | Spearman | Kendall tau-b | NDCG@3 (media) | mani con NDCG@3 < 0,9 |
|---|---:|---:|---:|---:|
| stima → B6 | 0,37 | 0,30 | 0,66 | 23/39 |
| B6 → w1u0 | 0,65 | 0,54 | 0,72 | 20/37 |
| w1u0 → w2u0 | 0,93 | 0,90 | 0,93 | 9/37 |
| w2u0 → w4u1 | 0,98 | 0,94 | 0,94 | 8/37 |
| w4u1 → senza breaker | 0,94 | 0,86 | 0,92 | 9/39 |
| senza breaker → completa | 0,96 | 0,90 | 0,98 | 2/40 |
| **stima → completa** | **0,61** | **0,48** | **0,81** | **21/40** |
| B6 → completa | 0,60 | 0,45 | 0,78 | 21/40 |
| w2u0 → completa | 0,74 | 0,66 | 0,85 | 17/40 |
| w4u1 → completa | 0,84 | 0,72 | 0,89 | 13/40 |

Dove sta il #1 vero nell'ordine di ogni gradino:
- **stima:** mediana 5ª posizione, 90° percentile 35ª. È oltre la 15ª in 12 mani su 41 e **oltre la 30ª in 8**.
- **B6:** mediana 6ª, oltre la 30ª in 4 mani.
- **w2u0:** mediana 2ª, oltre la 30ª in 3 mani.
- **senza breaker:** mediana 1ª, mai oltre la 25ª.

**Conseguenza: l'app di oggi perde il #1 vero in 8 mani su 41.** Il valore perso dal suo #1 è in media il 2,8%, al massimo il 50%. Prima ancora che lento, il taglio a 30 della stima è il punto debole.

### Tagli contro l'app di oggi (`--ref-app`)
«#1 uguale o migliore» conta anche le mani dove il taglio trova una board migliore di quella dell'app. Il valore perso è quello del #1 rispetto al #1 dell'app: negativo vuol dire guadagnato. Il tempo è la somma dei gradini rispetto a 30 misure complete.

| gradini (board tenute) | #1 identico | #1 uguale o migliore | top 3 ≥ 2 su 3 | valore perso (media / max) | tempo |
|---|---:|---:|---:|---:|---:|
| oggi: stima → prime 30 → completa | 41/41 | 41/41 | 41/41 | 0 / 0 | 100% |
| stima → 20 → completa | 38/41 | 38/41 | 39/41 | 0,3% / 7,2% | 71% |
| stima → 10 → completa | 34/41 | 34/41 | 33/41 | 2,8% / 68,8% | 36% |
| stima → 20 → B6 → 15 → completa | 36/41 | 37/41 | 36/41 | 0,4% / 8,1% | 55% |
| stima → 20 → w1u0 → 10 → completa | 37/41 | 37/41 | 33/41 | 0,8% / 26,2% | 43% |
| **stima → 20 → w2u0 → 10 → completa** | **38/41** | **38/41** | **34/41** | **0,7% / 26,2%** | **46%** |
| stima → 20 → w2u0 → 15 → completa | 38/41 | 38/41 | 38/41 | 0,7% / 26,2% | 63% |
| stima → 30 → w2u0 → 15 → completa | 39/41 | 40/41 | 37/41 | 0,6% / 26,2% | 67% |
| **stima → 40 → w2u0 → 15 → completa** | 37/41 | **41/41** | 36/41 | **−0,6% / 0,0%** | **70%** |
| stima → 30 → senza breaker → 15 → completa | 41/41 | 41/41 | 40/41 | 0 / 0 | 88% |

Contro il riferimento «tutte le 60 misurate»:
- l'app di oggi trova il #1 in 33 mani su 41;
- «stima → 40 → w2u0 → 15» in 35 (36 di pari valore), al 70% del tempo;
- «B6 su tutte → 20» in 35, al 73%.

### Livello 2 (gradino 4)
*In corso: livello 2 sulle prime 8 board della misura completa di ogni mano, contro Snake-Eye (contro Branded per la mano Snake-Eye).*

### Conclusione e proposta per B14
1. **Il gradino «misura senza breaker» del piano va sostituito.** Costa il 74% della misura completa, perché il 70% del tempo è il ciclo delle risposte, non i breaker.
2. **Il gradino che serve è una misura economica:**
   - ogni risposta provata solo alle prime 2 finestre in cui è offerta, al posto di 16;
   - niente ciclo degli usi, niente breaker, niente `countered`.

   Costa 0,07 s a board, il 13% della completa, e ordina come la completa molto meglio della stima: Spearman 0,74 contro 0,61, #1 vero oltre la 30ª in 3 mani contro 8.
   - In `probe.rs` è un `Parts` con due campi in più (finestre per risposta, giri degli usi), accanto a `measure_alone` e `measure_placed_unbroken`. È lavoro della corsia del coordinatore (B14 in `lib.rs`) o della 2: lo propongo, non lo tocco.
3. **Tagli proposti:**
   - **prudente:** stima su 40 → misura economica su 40, se ne tengono 15 → completa su 15. Il #1 è uguale o migliore in 41 mani su 41, nessuna mano perde valore, il tempo è il 70%. In 3 mani si trova una board migliore di quella dell'app di oggi (Doomz +18,7%), in una un'altra board di pari valore.
   - **veloce:** stima su 20 → misura economica su 20, se ne tengono 10 → completa su 10. Il tempo è il 46%, con #1 identico in 38 mani su 41 e top 3 almeno 2 su 3 in 34. Il #1 perde il 26% in una mano, meno del 3% nelle altre due.
   - Il controllo del piano (tempo −50% con il #1 identico e almeno 2 su 3 della top 3) non si raggiunge del tutto con questi gradini. Il «veloce» ci arriva vicino: 46% del tempo, 38 mani su 41.
4. **B6 (`quick_value`, già nel codice ma privato) costa pochissimo (3,5%) ma ordina come la stima** (Spearman 0,60 con la completa). Da solo serve poco; combinato con la stima migliora l'ordine (§2).
5. **Da rifare quando il gradino esiste nel codice:** gli stessi comandi senza la patch, con il tempo vero della valutazione nell'app (comprese le varianti di zona, che qui non ci sono).

## 2. Taratura della stima in stile Texel (preparazione di B9)

**Domanda.** Di quanto si può avvicinare l'ordine della stima rapida a quello della misura, con caratteristiche calcolabili dalla board?

### Metodo
`scripts/research/texel_fit.py`, sulle stesse 2190 board del §1:
- **Coppie.** Per ogni coppia di board della stessa mano con misure distanti più di 0,1, la probabilità che la migliore stia sopra è σ(s_a − s_b), con s = a·`quick_score` + Σ w_k·x_k. Sono 40 570 coppie in 41 mani.
- **Fit.** Newton sulla log-verosimiglianza media, più λ·|θ − θ₀|². θ₀ è la stima di oggi: a tarato da solo, w = 0. Le caratteristiche sono standardizzate sul gruppo di addestramento.
- **Verifica.** 70% dei deck per il fit e 30% lasciato fuori, su 20 divisioni a caso (seme 1). Le metriche sono sommate sulle divisioni:
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
- **Comandi.**
  ```
  python3 scripts/research/texel_fit.py <w>/all60 --splits 20 --lam 0.0001 0.001 0.01 0.1 1
  python3 scripts/research/texel_fit.py <w>/all60 --probe <w>/all60_w1u0 --splits 20 --lam 0.01 0.1 1 [--only-probe]
  ```

### Risultati (deck lasciati fuori)

| stima | inversioni top 8 | coppie giuste | Kendall | #1 nelle prime 10 | #1 nelle prime 15 |
|---|---:|---:|---:|---:|---:|
| oggi (`quick_score`) | 849/6720 (12,6%) | 73,6% | 0,54 | 164/240 | 170/240 |
| + 27 caratteristiche della board, λ 0,1 | 795/6720 (11,8%) | 75,6% | 0,51 | 163/240 | 166/240 |
| + 27 caratteristiche, λ 0,01 | 986/6720 (14,7%) | 74,5% | 0,48 | 163/240 | 171/240 |
| + B6 e w1u0 (due run e una misura economica), λ 0,01 | 856/6720 (12,7%) | **80,4%** | **0,62** | 169/240 | **189/240** |
| + B6 e w1u0, λ 0,1 | **753/6720 (11,2%)** | 78,9% | 0,60 | 164/240 | 184/240 |
| + caratteristiche + B6 e w1u0, λ 0,1 | 789/6720 (11,7%) | 79,7% | 0,58 | 165/240 | 174/240 |

Pesi (λ 0,01, per unità; deviazione tra le divisioni) delle caratteristiche più forti:
- Xyz +0,63 ± 0,15 e mostri dell'Extra Deck +0,13: la stima di oggi sottovaluta gli Xyz, come dice anche `ricerca-letteratura.md` §7.1;
- equipaggiamenti −0,90 ± 0,33;
- mostri coperti −0,45, Trappole coperte −0,44, ma carte coperte con «negate» +0,34;
- carte in mano −0,16 e carte usate −0,14: la stima conta già la mano, forse troppo.

Con λ piccolo i pesi cambiano molto tra una divisione e l'altra, e fuori campione l'ordine peggiora: è sovradattamento su 41 deck.

### Conclusione
- **Le caratteristiche lineari della board non bastano.** Fuori campione le inversioni scendono al massimo da 12,6% a 11,8%, e il #1 non migliora. L'obiettivo di C5.7 (60 su 1110, il 5,4%) è lontano. Quello che la stima sbaglia non è il numero di carte per zona ma cosa fanno nel turno avversario.
- **Due run del turno di prova lo vedono meglio.** Aggiungere B6 e la misura economica w1u0 (0,07 s a board) porta le coppie giuste dall'73,6% all'80,4% e il #1 nelle prime 15 da 170 a 189 su 240. Per B9 conviene quindi una stima a due livelli: `quick_score` nella ricerca, e una sonda economica (B6 o w1u0) sulle board finali, con pesi tarati così. Combacia con il §1.
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
  - **B4.** Le 4 mani e l'obiettivo di base entrano nell'insieme fisso di «obiettivi raggiungibili». Oggi la nostra ricerca è a 0 su 4, il solver a 4 su 4: è il controllo più semplice per B4.
  - **Idee dal solver per B4.** Nella sua uscita il solver dichiara una «sonda» iniziale (10 s, 7 000 stati) e poi rollout NRPA guidati dal conteggio delle Evocazioni che il bersaglio richiede («1 Fusion, 1 Link»). Quel conteggio è la versione più semplice della distanza rilassata di B4: costa poco, e l'ha già chi gioca su questo motore.
  - **B8.** Il vincolo «materiali ≥ N» va chiesto in modo esplicito. Nessuno dei due strumenti lo ha oggi.

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
