# Prossimi passi del motore: difetti e sprechi misurati

*Thread «Proporre i prossimi miglioramenti del motore», 24/10/2026, branch `claude/project-thread-5uicmc`. Secondo giro: la prima lista (linee fortunate, dimezzamenti, verità v1, sonda nella ricerca, giudice) non convinceva Valerio ed è sostituita da questa, fatta girando il motore. CLI release di `main` 12d6f9d, 63 mani `quick` del banco congelato v1, un thread, `--top 10`, 4 mani alla volta su 4 core.*

## Cosa si è fatto girare
| Giro | Cosa | Esito |
|---|---|---|
| H1 | 63 mani a 4000 nodi con `DUELFORGE_COPY_CHECK=1`, `DUELFORGE_CHECK_REUSE=1`, `DUELFORGE_SEARCH_STATS=1` | 0 copie diverse dal rigioco, 0 duelli riusati diversi da uno nuovo, 0 linee interrotte, 0 turni di prova falliti, 0 crash |
| H2 | `banco.py run --set quick`: `main` contro `main` con l'Extra Deck del kit a 2 + 2 copie invece di 7 + 7 | tempo 0,918 (0,907-0,930), ma 59 uscite su 63 diverse |
| H3 | le stesse 63 mani a 16 000 nodi, con le statistiche della ricerca | 18 mani meglio di oltre l'1%, 13 peggio, 32 pari |

**Dove va il tempo oggi** (H2, lato `main`, 965 s per 63 mani): ricerca 34%, turno di prova 38%, il resto 29% (circa 4,4 s a mano: valori delle carte e avvio). Il profilo di `tempi-motore.md` (14% / 82%) era su mani più pesanti; sul banco la ricerca e i valori delle carte pesano molto di più.

**La macchina delle copie è sana.** H1 non trova nessun difetto nel rigioco, nelle copie del duello o nel riuso delle partite. I problemi veri stanno in cosa la ricerca sceglie di misurare e in quanto costa un nodo.

## In breve: tre proposte
| # | Proposta | Prova | Guadagno atteso | Classe | Taglia |
|---|---|---|---|---|---|
| 1 | Non buttare le board già trovate: scegliere cosa misurare con la misura, non con la stima | 13 mani su 63 peggiorano con 4 volte i nodi; in 11 la board migliore era già stata trovata e non è stata misurata | Via le cadute (fino a −36%); più board diverse misurate allo stesso costo | R | M, 20-25 $ |
| 2 | Valori delle carte: calcolarli una volta per deck, non per seme, e dalle copie | 29% del tempo di ogni mano della CLI; la chiave della cache contiene il seme | −19/−29% sul banco e su ogni comando della CLI; prima ricerca più rapida nell'app | E | S-M, 10-15 $ |
| 3 | Nodi più economici: il rigioco è il 72% del tempo delle discese | 1,5 ms a nodo, di cui 0,9 ms di rigioco; con 16 000 nodi 7 mani salgono di oltre il 50% | Con il rigioco dimezzato, circa 1,5 volte i nodi a parità di tempo | E | M, 20 $ |

Consiglio: la 1 per prima (è il difetto che l'utente vede: più tempo, board peggiore), poi la 2 (piccola ed esatta), poi la 3.

## 1. Non buttare le board già trovate
**Il difetto.** Con un thread la ricerca è deterministica: una ricerca a 16 000 nodi passa per gli stessi primi 4000 nodi di una a 4000 e trova le stesse board. Eppure in 13 mani su 63 la migliore misurata a 16 000 nodi vale meno di quella a 4000. In 11 di queste la board migliore di 4000 nodi ha una stima più bassa di tutte le 10 board misurate a 16 000: era stata trovata, ma è uscita dalle 10 da misurare, scalzata da board che la stima giudica meglio e la misura peggio.

| Mano | 4000 nodi | 16 000 nodi | |
|---|---:|---:|---:|
| fresh.fur_hire_gmx.0 | 1,96 | 1,26 | −36% |
| fresh.magnet_warrior.0 | 0,48 | 0,35 | −27% |
| tuning.ancient_gear.0 | 0,49 | 0,37 | −24% |
| tuning.vampire.0 | 1,91 | 1,46 | −24% |

**Riproduzione.**
```
duelforge-cli combos --deck tests/fixtures/banco-motore/v1/decks/fresh/fur_hire_gmx.ydk \
  --hand '47325505;10045474;23434538;47647354;29927283' --seed 1 --threads 1 --top 10 --nodes 4000   # poi --nodes 16000
```

**Uno spreco nello stesso punto.** Fra le 10 board misurate, il 6-8% ha gli stessi mostri di un'altra (solo le zone cambiano), e in alcune mani la metà: Vaalmonica 4 campi diversi su 10, Lunalight, Zoodiac e Railway 5 su 10.

**La proposta** (generale, nessuna carta per nome):
- le candidate alla misura si scelgono con la misura economica su un insieme più largo, raccolto lungo tutta la ricerca (non solo le prime per stima alla fine), come chiede `revisione-motore.md` §2.2 punto 4;
- una board misurata resta in classifica con il suo valore: una board nuova la supera solo con una misura migliore (è la monotonia, ma senza misurare di nuovo a ogni punto di controllo, che costava +91%);
- al massimo una board per insieme di mostri fra le misurate per intero; le altre varianti di zona solo se la prima vince.

**Guadagno.** Le 13 cadute spariscono per costruzione nelle mani dove la board era già trovata; le misure tolte alle varianti vanno a board diverse. **Rischio:** medio, la misura economica costa: il numero di candidate va tarato sul tempo (oggi la misura è il 38%). **Validazione:** classe R, `banco.py judge --set full`, più il confronto 4000/16 000 sulle 63 mani (cadute da 13 a 0-2).

## 2. Valori delle carte: una volta per deck, dalle copie
**Lo spreco.** Ogni mano della CLI passa circa 4,4 s fuori dalla ricerca e dal turno di prova (29% del banco). Quasi tutto è `card_values` (`value.rs`): un turno di prova per ogni carta e posto (Branded: 135 turni, 1,9 s su 4 core, circa 6 s su uno). La cache in memoria ha nella chiave anche il seme (`(&deck.main, &deck.extra, seed, hand_traps)`), quindi le 3 mani dello stesso deck del banco (semi 1, 2 e 3) li ricalcolano tre volte, e lo stesso succede nell'app a ogni seme diverso.

**La proposta** (dal vivo, niente su disco):
- verificare che i valori non dipendano dal seme (stessi valori con i semi 1-5 su 20 deck); se è così, il seme esce dalla chiave e i valori si calcolano una volta per deck nello stesso processo;
- i turni di prova delle carte piazzate partono da una copia del duello con il campo vuoto (arena di B13), invece che da un duello nuovo per carta.

**Guadagno.** Con la sola chiave senza seme, −19% sul banco (2 calcoli su 3 tolti) e su ogni comando che gira più mani dello stesso deck (`engine-mc`, `coverage`, `tech-table`); con le copie, fino a −29%. **Rischio:** basso. **Validazione:** classe E, `banco.py same` sul set `full`: 193 uscite uguali.

## 3. Nodi più economici
**Lo spreco.** Nel giro a 16 000 nodi (847 582 nodi, 1254 s di ricerca) un nodo costa 1,5 ms; 0,9 ms sono rigioco, cioè il 72% del tempo delle discese. I giri di apprendimento sono il 10%. Oggi le copie si fanno solo dove una linea finisce, perché farle lungo la strada era risultato più lento (`velocita-ricerca.md`, B13, su linee più corte).

**Perché conta.** Le board che si perdono a 4000 nodi stanno in fondo a linee lunghe: nelle mani che migliorano a 16 000 nodi la linea migliore ha 41 scelte (31 nelle altre) e arriva tardi (mediana al nodo 11 814). Sette mani salgono di oltre il 50%: Fiendsmith Unchained 0,44 → 1,28, Rokket 0,57 → 1,60, Paleozoic 0,55 → 1,40, Radiant Typhoon Zoodiac 0,29 → 0,72, Live☆Twin, Code Talker e Shaddoll da +49% a +67%. Sul banco, passare da 2000 a 4000 nodi alza il valore medio del 10,7% con +17% di tempo (`regole-motore.md` §6).

**La proposta.** Riprovare le copie lungo la strada solo dove rendono: ogni k scelte nelle linee oltre le 20, con il contatore del rigioco per profondità prima e dopo. È esatta per costruzione (le copie rimettono tutto il duello).

**Guadagno.** Con il rigioco dimezzato la ricerca scende di circa un terzo: circa 1,5 volte i nodi nello stesso tempo, quindi più board nelle mani dalle combo lunghe. **Rischio:** basso sulle uscite, medio sul guadagno (va misurato). **Validazione:** classe E, `banco.py same` e tempo new/old.

**Riproduzione.**
```
DUELFORGE_SEARCH_STATS=1 duelforge-cli combos --deck tests/fixtures/banco-motore/v1/decks/fresh/fiendsmith_unchained.ydk \
  --hand '94145021;53624265;27412542;41165831;67803035' --seed 1 --threads 1 --top 10 --nodes 16000
```

## Altre cose trovate
- **Extra Deck del kit 7 + 7 → 2 + 2** (H2): −8% di tempo, ma cambia 59 uscite su 63, anche la ricerca (attraverso i valori delle carte). Non è un guadagno gratis: è una modifica della misura (classe M), da giudicare solo con la verità v1.
- **Stesso schema del difetto della #173 in `crates/ygo-engine/src/auto.rs:210`**: `&actions[..selectable.len()]` presume una risposta per carta, ma la lista offre le copie una volta sola. Oggi non fa danni (la scelta di ripiego è la prima risposta, sempre una carta vera); da correggere insieme alla #173 per non lasciare la trappola.
- **Ordine della ricerca.** Sulla linea migliore, la risposta giusta è la prima provata nel 76,5% dei passi e oltre la terza nell'8%; le deviazioni nei primi 5 passi sono tante nelle mani che migliorano quanto nelle altre. Il distacco non sta nell'ordine ma nel budget e nella scelta delle board da misurare: per questo B1a e B1c non rendevano.
- **Già visti** (stessi numeri del 24/10, nessuna proposta nuova): il 27,7% dei nodi finisce su uno stato già visto, il 23,8% non cambia niente di visibile, 10 582 risposte tolte come bersagli nascosti (`velocita-ricerca.md`, B2 e B2b).

## Fonti
- Determinismo con un thread e cadute fra budget: `monotonia-budget.md`, `caccia-bug-motore.md` punto 1.
- Scegliere le board da misurare con la misura economica: `revisione-motore.md` §2.2.
- Copie del duello e tetto: `velocita-ricerca.md`, B13.
