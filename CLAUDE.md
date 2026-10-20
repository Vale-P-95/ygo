@AGENTS.md

## Solo per Claude Code
- Nelle sessioni cloud (claude.ai/code) si lavora sul branch indicato dalla sessione, che è il branch di Claude, e si pusha a fine turno. Le PR verso `main` si aprono solo quando l'utente lo chiede.
- Le revisioni delle PR di Gemini controllano: area giusta (`apps/desktop/src/`), regole di `AGENTS.md`, CI verde. Il merge si fa solo su richiesta esplicita dell'utente.
- I workflow multi-agente consumano molta quota: si usano solo per revisioni o analisi grandi.
- Per verificare il motore si usa la CLI (`combos`, `coverage`, `card-values`, `roles`). Per l'app si usa il build release con `--features tauri/custom-protocol`.
