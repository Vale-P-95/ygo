---
name: coordinamento
description: Come lavora una sessione coordinatrice di DuelForge nei giorni con più PR in coda (aprire PR, iscriversi agli eventi, merge su delega, aggiornare coordinamento.md, passare la mano). Da usare solo quando l'utente apre una coordinatrice.
---

# Coordinare un giro di lavoro

La coordinatrice non è un servizio sempre acceso: lavora a eventi e si chiude a fine giornata. Con tre sessioni o meno non serve: le sessioni aprono le proprie PR e l'utente unisce dalla pagina GitHub.

## Memoria
- `docs/plans/coordinamento.md` è la memoria: sessioni attive con id, branch e compito; PR aperte e unite; decisioni del giorno; costi. Si aggiorna a ogni cambio importante, si pusha sul branch della coordinatrice e si unisce a `main` con una PR di soli documenti a fine giro.
- Sopra i 200k token di contesto si passa la mano: si aggiorna il file, si pusha, si chiede all'utente una sessione nuova che parte da quel file.

## Eventi, non controlli
- Risvegli ammessi: messaggio dell'utente, messaggio di una sessione (PR pronta, blocco, file caldo), evento GitHub di una PR iscritta. Nessun trigger orario, nessun monitor, nessun `list_events`.
- `list_sessions` una volta per turno utente, solo quando chiede stato o costi.

## Per ogni PR pronta
1. `git fetch` del branch e `git diff --stat origin/main...<branch>`: area giusta, niente file fuori dal brief, `git merge-tree` pulito.
2. Aprire la PR con il template e i numeri o gli screenshot mandati dalla sessione; iscriversi agli eventi (`subscribe_pr_activity`).
3. Al verde: merge su delega dell'utente (squash, SHA completo), oppure dire all'utente che è pronta da unire dalla pagina.
4. Annotare in `coordinamento.md`; dire alla sessione di fermarsi; dire all'utente che la sessione si può archiviare.

## Messaggi alle sessioni
- Brevi, uno per decisione. Ogni messaggio costa un turno intero alla sessione che lo riceve.
- Una sessione nuova si crea con `create_session` passando `model` e il brief (`brief`).

## Fine giornata
Tre righe in `coordinamento.md`: spesa per sessione (`list_sessions`, una volta), contesto massimo, PR unite; le sessioni finite da archiviare.
