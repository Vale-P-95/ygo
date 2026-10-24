# Note per /design-sync

- Su richiesta dell'utente (2026-10-24) si caricano le anteprime già generate in `apps/desktop/design/` così come sono (`pnpm design` dalla galleria), senza ricostruire i componenti né creare `_ds_bundle.js`.
- Lo stesso layout di cartelle va nel progetto; in più un `styles.css` che importa `duelforge.css`, perché i design ricevono solo la catena di `@import` di `styles.css`.
- Niente `_ds_sync.json`: senza il convertitore non c'è un'ancora onesta, e ogni sync ricarica tutto.
- Dal secondo sync si carica solo ciò che è cambiato in `apps/desktop/design/` dall'ultimo sync (`git diff --name-only <commit dell'ultimo sync> HEAD -- apps/desktop/design`) e si cancellano nel progetto i file tolti dalla cartella. Ultimo sync: 24/10, da `3636da3` (le 72 schermate di `schermate/` e il README).
- Nel progetto ci sono file che non vengono da questa cartella e non si toccano: `templates/`, `Combo Lab.html`, `_ds_bundle.js`, `_ds_manifest.json`, `_adherence.oxlintrc.json` (creati in Claude Design o dall'app) e lo `styles.css` scritto dal primo sync.
