# Note per /design-sync

- Su richiesta dell'utente (2026-10-24) si caricano le anteprime già generate in `apps/desktop/design/` così come sono (`pnpm design` dalla galleria), senza ricostruire i componenti né creare `_ds_bundle.js`.
- Lo stesso layout di cartelle va nel progetto; in più un `styles.css` che importa `duelforge.css`, perché i design ricevono solo la catena di `@import` di `styles.css`.
- Niente `_ds_sync.json`: senza il convertitore non c'è un'ancora onesta, e ogni sync ricarica tutto.
