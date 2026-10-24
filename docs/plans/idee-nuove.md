# Idee nuove per DuelForge (24/10/2026)

Ricerca sul web del 24/10 (strumenti per Yu-Gi-Oh!, richieste dei giocatori, letteratura recente) confrontata con `docs/STATO.md`, `docs/TASKS.md`, `docs/PIANO.md` e i piani in `docs/plans/`. Sono escluse le idee già provate o scartate e quelle già coperte dalle due revisioni bibliografiche (`revisione-bibliografica.md`, `revisione-bibliografica-2.md`): ricerca, trasposizioni, novità, scenari dell'avversario, valutatore imparato.

## In breve
Cinque idee, in ordine di beneficio sul costo. Le prime quattro usano solo pezzi che esistono già (turno di prova contro un deck, livello 2, riproduttore della linea, Laboratorio) e **non cambiano la misura**. La quinta aggiunge una misura nuova, separata da quella del primo turno.

| # | Idea | Costo | Cambia la misura? |
|---|---|---|---|
| 1 | Tabella delle tech contro il meta: quale handtrap ferma più linee vere | S-M | No |
| 2 | Allenamento sulla combo: la linea trovata diventa un quiz passo per passo | S | No |
| 3 | Piano di side generato dal motore, matchup per matchup | M | No (usa la 1) |
| 4 | Monte Carlo guidato dal motore nell'ottimizzatore (PIANO §7.4) | M-L | No (la usa come obiettivo) |
| 5 | Andare secondo: quanta board del meta rompe la mia mano da 6 | L | Sì, una misura nuova a parte |

## Cosa dice la ricerca
- **Nessuno strumento sceglie le tech contro il meta.** I calcolatori esistenti (YGOrg Combo Rate Calculator, il calcolatore ipergeometrico di YGOPRODeck) contano le carte nella mano; nessuno dice quale handtrap ferma più linee dei deck che si incontrano. Le guide parlano di «resistenza alle handtrap» e di side deck senza numeri ([YGOrg](https://ygorganization.com/combo), [YGOPRODeck](https://ygoprodeck.com/yu-gi-oh-hypergeometric-probability-distribution-function)).
- **Per secondo i giocatori fanno i conti a mano.** Le liste per secondo dichiarano rapporti come 18 breaker, 19 motore e 3 altro, con la probabilità di vederne almeno uno in 6 carte ([YGOPRODeck, Spright per secondo](https://ygoprodeck.com/deck/spright-aggro-otk-blind-second-master-duel-ladder-grinder-727956)); nessuno misura quanto della board avversaria si rompe davvero.
- **Allenarsi sulle combo è una richiesta vera.** Konami ha messo «My Combos» in Neuron; Ygo101 vende un «combo maker» e un simulatore con annulla e ripeti ([Neuron](https://www.yugioh-card.com/eu/neuron-combo-guide-campaign/), [Ygo101](https://www.patreon.com/ygo101)). In Duel Links manca un modo di provare un deck con carte che non si hanno ([Steam](https://steamcommunity.com/app/601510/discussions/0/1848072002760365836)).
- **Letteratura.** Nei CCG la ricerca sul turno avversario resta rara e costosa ([sintesi della competizione LOCM](https://arxiv.org/pdf/2305.11814)); il resto (MCTS per Duelyst, GO-MCTS, MAP-Elites per i deck di Hearthstone) è già nelle revisioni o in `ricerca-letteratura.md` §8.4. L'archivio per descrittori alla MAP-Elites è già in piano (`revisione-motore.md`, «sì, piccolo»): qui non si ripropone.

## 1. Tabella delle tech contro il meta
Oggi `probe-line --vs <deck> --stops` dice quali handtrap **dell'avversario** fermano **la mia** linea. La stessa macchina, girata, dice quali handtrap **mie** fermano **le linee vere** dei deck del meta (quelle che il livello 2 già rigioca da 12 mani). Pesate per diffusione del deck, danno una classifica: «Ash Blossom ferma il 58% delle linee del meta pesato, Infinite Impermanence il 41%, Nibiru il 22%», con il momento in cui la carta entra. Nessuno strumento lo fa. Nessun nome di carta nel codice: le candidate sono le handtrap lette dal motore (`hand-traps`) nel deck e nel side dell'utente, o in una lista scelta da lui.

```
Compito: comando `tech-table --deck <mio.ydk> --meta <cartella>` che per ogni handtrap del mio Main e Side dà la quota di linee del meta che ferma, pesata per diffusione, e il passo in cui entra
Perché: docs/plans/idee-nuove.md §1; nessuno strumento sceglie le tech con le linee vere
Branch: claude/tech-table, da main
File da toccare: crates/ygo-solver/src/matchup.rs (nuova funzione accanto a rival_lines e against_moves), crates/ygo-cli/src/main.rs, scripts/mdm_decks.py se serve il peso di diffusione
Da leggere: docs/STATO.md, docs/plans/turno-di-prova-matchup.md (livello 2, --stops), docs/plans/collaudo-strumenti.md L4
Verifica: cargo test -p ygo-solver; duelforge-cli tech-table su 3 deck di tests/fixtures contro le liste di mdm_decks.py; confronto a mano di 5 righe con probe-line --vs --stops
Criterio di accettazione: per 3 deck la classifica è stabile su due semi (stesso ordine delle prime 3) e ogni «ferma» ha il passo della linea avversaria
PR: template; «Non cambia la misura», tempi per deck
Modello: Opus 5.5, effort alto (matchup.rs); poi Sonnet medio per la CLI
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## 2. Allenamento sulla combo
Il riproduttore della linea (`components/combo/Steps.tsx`) mostra già il campo a ogni passo. In modo allenamento nasconde il passo successivo e chiede «cosa fai adesso?» tra 3-4 azioni legali di quel nodo (quelle che la ricerca ha visto come fratelli), poi mostra la risposta e il campo. Alla fine dà il punteggio e i passi sbagliati, da ripassare. Solo interfaccia, se i fratelli sono già nella linea salvata; altrimenti un campo in più nel contratto. È la versione leggera del simulatore #36.

```
Compito: modo «Allenati» nel riproduttore della linea: a ogni passo l'utente sceglie tra le azioni legali del nodo, poi vede la risposta; riepilogo degli errori a fine linea
Perché: docs/plans/idee-nuove.md §2; richiesta dei giocatori (Neuron «My Combos», Ygo101)
Branch: claude/combo-drill, da main
File da toccare: apps/desktop/src/components/combo/Steps.tsx, componente nuovo in components/combo/; se mancano i fratelli dei passi: lib.rs, api.ts, mock.ts e fixture insieme (file caldi, da chiedere in TASKS.md)
Da leggere: docs/STATO.md, AGENTS.md «Protocollo per chi tocca l'interfaccia», docs/plans/ui-revisione-2.md; mockup in Claude Design prima del codice
Verifica: pnpm typecheck; pnpm build; VITE_MOCK=1 pnpm dev e node scripts/screens.mjs nei due temi; pnpm tauri dev su una linea vera
Criterio di accettazione: su una linea di 10 passi il quiz offre a ogni passo l'azione giusta più almeno due legali diverse, e il riepilogo elenca i passi sbagliati
PR: template; screenshot nei due temi e anteprima Artifact
Modello: Sonnet 5.5, effort medio
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## 3. Piano di side generato dal motore
Il PIANO §4.4 prevede una tabella matchup × carte (fuori/dentro) scritta a mano. Con la tabella della §1 la colonna «dentro» si riempie da sola: per ogni deck del meta, le carte del Side che fermano più linee di quelle del Main che fermano meno; per il «fuori», le carte del Main con il valore più basso contro quel deck (`card-values` e la resilienza contro X che esistono già). L'utente corregge e stampa. Dipende dalla §1.

```
Compito: pagina «Piano di side» nella Fucina: per ogni deck della Bottega o del meta, carte del Side da mettere e del Main da togliere, proposte dal motore e modificabili, stampabili
Perché: docs/plans/idee-nuove.md §3; PIANO §4.4 (side plan) mai fatto
Branch: claude/side-plan, da main, dopo la PR della §1
File da toccare: apps/desktop/src-tauri/src/lib.rs, apps/desktop/src/lib/api.ts, mock.ts, fixture (file caldi), pagina nuova in pages/ o sezione della Fucina
Da leggere: docs/STATO.md, PIANO §4.4, docs/plans/idee-nuove.md §1; mockup in Claude Design
Verifica: cargo test -p ygo-solver; pnpm typecheck; pnpm build; screenshot nei due temi; pnpm tauri dev con un deck e due avversari della Bottega
Criterio di accettazione: con un Side di 15 carte e 3 avversari, ogni proposta ha il numero che la giustifica (linee fermate o valore) e il deck dopo il side resta valido (40-60, Extra ≤ 15)
PR: template; screenshot e anteprima Artifact
Modello: Sonnet 5.5, effort medio (Opus alto per il contratto)
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## 4. Monte Carlo guidato dal motore nell'ottimizzatore
L'ottimizzatore «Quante copie?» oggi massimizza probabilità calcolate sui ruoli. Il PIANO §7.4 prevede il Monte Carlo guidato dal motore, mai fatto. Proposta concreta: si pescano N mani, la ricerca del Combo Lab dà il valore della board migliore di ognuna, e il valore si mette in cache per **multinsieme delle carte che la ricerca ha usato** (le carte mai toccate da nessuna linea contano come vuote), così cambiare una copia ricalcola solo le mani nuove. L'obiettivo dell'ottimizzatore diventa «P(board ≥ soglia)» misurata, non stimata dai ruoli. Calcolo lungo: in background, con `summary.txt`.

```
Compito: obiettivo «board misurata» nell'ottimizzatore: P(board ≥ soglia) da N mani giocate dal motore, con cache per multinsieme delle carte usate
Perché: docs/plans/idee-nuove.md §4; PIANO §7.4
Branch: claude/engine-mc, da main
File da toccare: crates/ygo-stats/src/optimize.rs, modulo nuovo in crates/ygo-solver per la cache delle mani, crates/ygo-cli/src/main.rs (comando `engine-mc`)
Da leggere: docs/STATO.md, PIANO §7.4 e §20, docs/plans/velocita-ricerca.md (tempi per mano)
Verifica: cargo test -p ygo-stats -p ygo-solver; duelforge-cli engine-mc --deck <deck> --hands 200 in background con out/engine-mc/summary.txt; confronto con i ruoli automatici
Criterio di accettazione: su 2 deck la stima con 200 mani ha un intervallo al 95% sotto ±5 punti, e la cache evita almeno il 50% delle ricerche quando si cambia una copia
PR: template; tempi e quota di cache; «Non cambia la misura»
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## 5. Andare secondo: quanta board del meta rompe la mia mano
*24/10: prototipo CLI fatto (`break-board`), numeri e limiti in `docs/plans/andare-secondo.md`.*

Oggi tutto il Combo Lab gioca per primo (PIANO §22.7, punto 1: «per secondo nella Fase 6»). Proposta: l'avversario è un deck del meta, la sua board è la migliore che la ricerca trova da una sua mano; la mia mano da 6 gioca il turno 2 con la ricerca che c'è, e il valore è **quanta parte del valore misurato della board avversaria resta** dopo il mio turno (con la stessa misura del turno di prova, girata). È una misura nuova per il secondo turno, che non tocca quella del primo, quindi niente prova equa sulle 406 board; serve però un suo banco. È il lavoro più grosso: la ricerca deve partire da uno stato con il campo avversario pieno.

```
Compito: prototipo CLI `break-board --deck <mio> --vs <deck>`: la mia mano da 6 contro la board migliore di 12 mani avversarie, valore = quota della board avversaria rimasta dopo il mio turno
Perché: docs/plans/idee-nuove.md §5; PIANO §22.7 punto 1 e Fase 6
Branch: claude/going-second, da main
File da toccare: crates/ygo-solver/src/ (modulo nuovo che costruisce lo stato iniziale dalla board avversaria; search.rs solo se serve un punto d'ingresso: file caldo), crates/ygo-cli/src/main.rs
Da leggere: docs/STATO.md, docs/plans/turno-di-prova-matchup.md (Kit::with_line), docs/plans/revisione-motore.md §5.3
Verifica: cargo test -p ygo-solver; cargo test --release -p ygo-solver --test search_regression (invariata); break-board su 3 coppie di deck di tests/fixtures
Criterio di accettazione: per 3 coppie la mano con più breaker letti dal motore lascia meno board avversaria della mano senza, su almeno 10 mani per coppia
PR: template; numeri prima/dopo di search_regression; «misura nuova, separata dal primo turno»
Modello: Opus 5.5, effort alto
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

## Scartate o già in piano
- Self-play e RL come componente: esclusi (`revisione-bibliografica.md` §0.2).
- Archivio per descrittori (MAP-Elites) al posto della top N: già in `revisione-motore.md` e `ricerca-esperimenti.md`.
- Mano di prova da 6 per secondo: c'è già nel Laboratorio (`optimize.rs`, «Per primo / per secondo»).
- Prezzi e collezione posseduta: utili per il TCG, fuori dal primo formato (Master Duel) e dal centro del progetto.
