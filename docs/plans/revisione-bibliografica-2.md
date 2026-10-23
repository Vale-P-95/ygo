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
