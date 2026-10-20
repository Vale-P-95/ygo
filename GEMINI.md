Leggi e segui `AGENTS.md`: sono le regole comuni a tutti gli agenti di questo repository.

## Solo per Antigravity / Gemini
- La tua area è l'interfaccia: `apps/desktop/src/`. Non modificare `crates/` né `apps/desktop/src-tauri/`. Se ti serve un dato nuovo dal backend, scrivi la richiesta in `docs/TASKS.md` per Claude.
- `apps/desktop/src/lib/api.ts` è il contratto con il backend: non cambiarne i tipi senza un task concordato.
- Salva i piani in `docs/plans/<tema>.md` prima di implementarli.
- Lavora su un branch `gemini/<tema>`, apri una pull request verso `main` e comunica il numero all'utente.
- Prima di ogni push esegui `pnpm typecheck` e `pnpm build` in `apps/desktop`.
- Non lanciare build release del Rust o benchmark senza che l'utente lo chieda.
- Usa Gemini Flash per i compiti piccoli (testi, rinomine, piccoli ritocchi).
