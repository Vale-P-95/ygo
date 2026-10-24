---
name: brief
description: Il modello del brief con cui nasce una sessione Claude Code di DuelForge (un compito, dieci righe). Da usare quando si prepara una sessione nuova o si passa la mano.
---

# Brief di una sessione

Una sessione fa un compito solo e nasce con questo brief. Dieci righe: la storia sta nei documenti, non nel prompt.

```
Compito: <una frase: cosa deve esistere alla fine>
Perché: <una frase, o il link alla decisione in docs/plans/<tema>.md>
Branch: claude/<tema>, da main
File da toccare: <elenco; i file caldi vanno chiesti in docs/TASKS.md>
Da leggere: docs/STATO.md (solo la riga dell'area), docs/plans/<tema>.md §<n> (sezioni, non il documento), <mockup: link all'artifact Design, se c'è>
Verifica: <comandi esatti: cargo test -p ..., pnpm typecheck, node scripts/screens.mjs, paired_test.py ...>
Criterio di accettazione: <numeri o comportamento osservabile>
PR: template di .github/pull_request_template.md; numeri prima/dopo o screenshot
Modello: <Opus 5.5 | Sonnet 5.5 | Haiku 5.5>, effort <alto | medio | basso>
Quando hai finito: TASKS.md aggiornato, PR aperta, fermati.
```

Regole:
- Un compito per brief. Se ne servono due, sono due sessioni.
- I calcoli lunghi del brief partono in background; la sessione chiude il turno e riprende alla notifica.
- Il brief non ripete `AGENTS.md` né `CLAUDE.md`: la sessione li legge da sola.
- Per il passaggio di consegne a 200k token si usa lo stesso modello, con «Stato» al posto di «Perché» e il link al documento che racconta dove si era arrivati.
