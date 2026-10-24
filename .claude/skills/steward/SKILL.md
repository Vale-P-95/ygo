---
name: steward
description: Convenzioni del repository per chi guida una pull request fino al merge (CI, rilanci, conflitti, revisione). Letta automaticamente da chi segue gli eventi di una PR di DuelForge.
---

# Guidare una PR di DuelForge

## CI
- La CI parte da sola quando la PR si apre (o esce dalla bozza), **non** a ogni push: i minuti di Actions sono contati. Dopo un push la si rilancia a mano **una volta sola**, dopo l'ultimo push: `actions_run_trigger`, `run_workflow`, `ci.yml`, `ref` = branch della PR.
- Una PR di soli documenti ha solo il job «Changed areas»: verde in dieci secondi. Un lancio manuale fa girare tutti i job: si annulla il run se la PR è di documenti.
- Niente commit vuoti o chiudi-e-riapri per far ripartire la CI. Niente test saltati o spenti per avere il verde.
- Un fallimento che non riguarda la PR (rete, runner) si rilancia al massimo una volta; la seconda volta è reale.

## Conflitti
- Si porta `main` nel branch con un merge, mai con rebase o force push. I file generati si rigenerano con gli strumenti del repository.

## Revisione
- Le PR piccole: la sessione stessa, con la lista del template. Le PR grandi: una sessione Sonnet separata con brief corto (diff, lista, screenshot).
- Le PR del motore devono avere i numeri prima/dopo (`pr-motore`); quelle dell'interfaccia gli screenshot e la lista (`pr-interfaccia`). Senza, non sono pronte.

## Merge
- Lo fa la coordinatrice in automatico (decisione del 23/10), non il thread: `merge_method: squash`, `expectedHeadSha` con lo SHA completo (40 caratteri), CI verde, nessun conflitto. Mai con conflitti, mai con CI rossa; per il motore, solo con `banco.py` ACCEPTED (`docs/plans/regole-motore.md`). Si attende l'esito dai soli eventi della PR, senza controlli a intervalli. Dopo l'ultimo push la CI si lancia a mano una volta (`ci.yml`).
- Dopo il merge: `docs/TASKS.md` aggiornato, iscrizione agli eventi tolta (automatica), branch cancellato.
