# Revisione bibliografica, seconda passata: ricerca ampliata, testi interi, piste nuove

*Corsia 10, 23/10/2026, branch `claude/literature-review-2` da `main` 9cc3005. Segue `revisione-bibliografica.md` (prima passata, PR #87). Bozza: le sezioni si riempiono una alla volta e si pushano a ogni passo.*

## 0. Sommario esecutivo
*(5-8 idee più attuabili, in ordine; da scrivere alla fine)*

## 1. Fonti aperte e cosa risponde davvero (sonda del 23/10/2026, 07:33 UTC, `curl` dal container)
| Host | Esito | Nota |
|---|---|---|
| arxiv.org, export.arxiv.org (API) | 200 | abstract e PDF scaricabili; `pdftotext` disponibile |
| api.semanticscholar.org | 429 | raggiungibile ma limitato nel ritmo (Too Many Requests) anche a chiamate singole: serve attesa o chiave API |
| www.semanticscholar.org | 200 | pagine web |
| openreview.net | 200 | |
| ojs.aaai.org, aaai.org | 200 | AIIDE, AAAI, SoCS, ICAPS |
| proceedings.mlr.press | 200 | ICML, CoRL |
| proceedings.neurips.cc, papers.nips.cc | 200 | |
| www.jair.org, jmlr.org | 200 | |
| www.frontiersin.org, journals.plos.org | 200 | |
| ieee-cog.org | 200 | |
| api.crossref.org | 200 | |
| dblp.org | 202 | |
| core.ac.uk, api.core.ac.uk | 200 | |
| api.unpaywall.org | 200 | richiede un indirizzo e-mail qualunque nel parametro; usato un indirizzo neutro |
| api.openalex.org | 429 | limitato nel ritmo anche con `mailto` |
| openalex.org | 403 | |
| dl.acm.org | 403 | |
| ieeexplore.ieee.org | 418 | blocco anti-bot |
| www.sciencedirect.com | 403 | |
| api.elsevier.com | 401 | serve una chiave API |
| link.springer.com | 000 (nessuna risposta) | |
| onlinelibrary.wiley.com | 403 | |
| www.mdpi.com | 403 | |
| www.nature.com | 000 | |
| doi.org | 000 | i DOI vanno risolti via Crossref/Unpaywall |
| www.ijcai.org | 000 | |
| cs.unh.edu, scitepress.org | 000 | non nell'elenco aperto: il PDF di Duelyst (UNH) e Sakurai & Hasebe vanno cercati su CORE/Unpaywall |

**Conseguenza.** I testi interi si prendono da arXiv, AAAI/ICAPS/SoCS/AIIDE (OJS), PMLR, NeurIPS, JAIR, JMLR, OpenReview, Frontiers, PLOS e dalle copie aperte trovate via Unpaywall e CORE. ACM, IEEE, Elsevier, Springer, Wiley, MDPI e Nature restano chiusi: per quei lavori si cerca la versione aperta, altrimenti si resta all'abstract.

## 2. Ricerca ampliata
*(i 7 gruppi di parole chiave e le query A-E della prima passata, rieseguiti sulle fonti nuove: registro, lavori nuovi, cosa cambia nelle conclusioni)*

### 2.1 Gruppo 1, Yu-Gi-Oh!: un articolo scientifico esiste
La prima passata scriveva «nessun articolo scientifico (peer-reviewed o arXiv) tratta Yu-Gi-Oh! come problema». **Va corretto.** L'API di arXiv (`all:"Yu-Gi-Oh"`, 23/10/2026) restituisce 4 lavori; tre sono di visione artificiale (un dataset di illustrazioni per StyleGAN2, due di classificazione multi-etichetta che usano le carte come dati) e uno è di logica matematica, **letto per intero** (sezioni 1, 2, 3 e 6, più la bibliografia):

- **N1. Nicolosi, Pisciotta & Bresolin, *Deciding winning strategies in Yu-Gi-Oh! TCG is hard*, arXiv 2603.02863 (math.LO), marzo 2026** (Università di Torino, Udine e Pisa; preprint, 15 pagine). *Problema.* Stabilire se, da una configurazione data, una strategia data (calcolabile) è vincente. *Risultato.* Il problema è indecidibile (il problema dell'arresto vi si riduce) e, più precisamente, **Π¹₁-completo**; lo stesso vale per le strategie non calcolabili (riduzione dall'insieme dei buoni ordini numerabili). *Costruzione.* Non incorporano una macchina di Turing universale come Churchill, Biderman & Herrick per Magic: codificano la configurazione della macchina nel **numero di Spell Counter** su Magical Citadel of Endymion, e il programma nella strategia. Due deck legali per la lista attuale (43 e 48 carte: Magician of Faith, Mask of Darkness, Vanity's Ruler, Yata-Garasu, Book of Eclipse, Bait Doll, Upstart Goblin, Desert Sunlight, Localized Tornado, Temple of the Kings) permettono al primo giocatore, con la pescata giusta e un avversario che non interrompe, di bloccare l'avversario (niente pescate con Protector of the Sanctuary e Yata-Garasu, niente Extra Deck con Vanity's Ruler) e poi di **aumentare o diminuire a piacere i contatori con cicli di attivazioni che si riciclano** in un turno solo («un numero illimitato di volte»). Due ipotesi dichiarate: un numero finito di operazioni per turno e nessun limite di tempo alla partita. Gli autori congetturano che la funzione di transizione del gioco sia calcolabile («più ragionevole che per Magic, perché gli effetti sono più semplici»), ma non lo dimostrano: servirebbe un'analisi esaustiva delle oltre 13 000 carte. *Note.* Gli autori scrivono di «non essere a conoscenza di alcuna letteratura accademica formale sulle proprietà computazionali o logiche di Yu-Gi-Oh! TCG», il che conferma per altra via l'esito della prima passata sul lato della pianificazione; citano Cowling & Ward 2009 e Cowling, Ward & Powley 2012 per Magic.
- *Cosa ne prendiamo.* (1) **La conferma formale che una linea del primo turno può essere lunga senza limite** anche in Yu-Gi-Oh!, con carte legali e senza interruzioni: il motore non può promettere completezza né terminazione senza un budget, e la nostra regola «budget di nodi più tetto di passi per linea» è l'unica cornice corretta (è il punto 6 di `revisione-motore.md` §0 e §3). (2) Il **ciclo di attivazioni che si riciclano** (Magician of Faith che rimette in mano Double Summon, Bait Doll che torna nel Deck) è esattamente il tipo di linea «legale ma inutile» che la novità alla Iterated Width (B11) e il contatore «nodi senza cambi visibili» (A2) devono tagliare: un test di regressione costruito su questo deck (la fixture è pubblica nell'articolo) misura quanti nodi la ricerca spende in un ciclo prima di fermarsi. (3) Per la generalità: il caso mostra che un deck «fuori meta» può avere linee infinite; nessuna regola per nome lo risolve, solo la novità o un tetto di passi. *Trasferibilità: media* (argomento di progetto, non algoritmo). *Lettura: intero.*

Il resto del gruppo 1 sulle fonti nuove (Crossref, CORE; Semantic Scholar e OpenAlex limitati nel ritmo: vedi §6): le interrogazioni con «Yu-Gi-Oh» restituiscono solo lavori di pedagogia, studi dei media, realtà aumentata e analisi linguistica (11 titoli guardati su Crossref, nessuno tenuto). **Resta vero che nessun lavoro accademico tratta la ricerca delle linee, la forza della board o la generalizzazione a carte nuove in Yu-Gi-Oh!**; l'unico articolo è di computabilità.

## 3. Letture a testo intero: aggiornamenti e correzioni alla prima passata
*(Duelyst, Sakurai & Hasebe, Sokota et al. 2026, Prismata HPS, limiti ammissibili, Burch & Holte, NRPA, Go-Explore, novità/larghezza, CVaR/SAA, letale in Hearthstone, LOCM, Forge)*

## 4. Piste nuove sui problemi aperti
### 4.1 Forza della board contro avversari ignoti, senza dati precalcolati (kit a catena)
### 4.2 Generalizzazione a carte e deck mai visti
### 4.3 Ricerca monotona e sicura nel budget
### 4.4 Velocità: snapshot, simulazione incrementale, cache delle valutazioni
### 4.5 Altro sulla pianificazione del turno nei giochi di carte

## 5. Tabella dei lavori nuovi

## 6. Registro delle interrogazioni
