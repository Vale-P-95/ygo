# Sistema di design DuelForge

Piano ambiente §3.1 e §10 punto 6. Sessione del 24/10/2026, thread «Costruire il sistema di design».

## Decisioni
- La sorgente è il codice: la pagina `/galleria` (solo con `VITE_MOCK=1`, indirizzo `#/galleria` o `/galleria`) mostra token e componenti veri, ogni scheda nei due temi affiancati e negli stati vuoto, caricamento, errore, dati.
- Le anteprime di `apps/desktop/design/` si generano dalla galleria con `pnpm design` (`scripts/design-export.mjs`): una pagina per scheda con il marcatore `@dsCard`, il CSS compilato dell'app e gli asset. Non si scrivono a mano, così non divergono dal codice.
- Un componente nuovo entra con una `DsCard` in `Gallery.tsx` e il diff di `design/` nella stessa PR.
- Il caricamento su claude.ai lo fa `/design-sync`, che l'utente lancia: la sessione non scrive nel progetto di design da sola, e l'utente approva il piano dei file.
- La stessa esportazione produce l'anteprima Artifact (`--artifact`), sempre allo stesso URL.

## Stato
- Schede (16): Fondamenta (colori, accenti delle sezioni, ruoli e metriche, cornici delle carte, tipografia, spaziature e superfici), Componenti (pulsanti, chip, campi, barra degli strumenti, aiuto e pannello laterale, stati), Carte, Grafici, Sezioni (intestazione), Bottega (scatole 3D).
- Anteprima Artifact: https://claude.ai/artifact/UQsPmQuirQoVtSeUgoZxG9
- Non ci sono ancora: `Modal` e i popover aperti (vivono in un portale a schermo intero), `CardInspector`, le schede del Combo Lab (`Score`, `Field`). Si aggiungono quando servono a un mockup.

## Prossimo passo
1. L'utente lancia `/design-sync` da una sessione Claude Code su `apps/desktop/design/`, verso un progetto nuovo «Sistema di design DuelForge» (attenzione: Claude Design come sito a sé chiude il 14/12/2026 e vive dentro Claude).
2. Primo mockup sul sistema di design: il passo 6 (Impostazioni, effetti e suoni), §10 punto 8 del piano ambiente.
