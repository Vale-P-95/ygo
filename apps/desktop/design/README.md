# Anteprime del sistema di design

Questa cartella è **generata**: non si modifica a mano. La sorgente è la galleria `/galleria` dell'app (`src/pages/Gallery.tsx`, solo con il backend finto).

- Ogni file `<gruppo>/<scheda>.html` è una scheda del sistema di design DuelForge: comincia con `<!-- @dsCard group="…" -->` e mostra un gruppo di token o un componente nei due temi, Fucina (scuro) e Pergamena (chiaro).
- `duelforge.css` è il foglio di stile compilato dell'app (token di `src/index.css` e classi Tailwind), con i soli font latini; `assets/` ha i font e le texture della Bottega.
- `index.html` mette tutte le schede in una pagina.

Per rigenerare, da `apps/desktop`:
```
pnpm design                                              # sul computer, con il Chromium di Playwright
pnpm design --chromium /opt/pw-browsers/chromium         # nel container cloud
pnpm design --shots <cartella> --artifact <cartella>     # anche gli screenshot e l'anteprima Artifact
```

Il caricamento su claude.ai si fa con `/design-sync` verso il progetto «Sistema di design DuelForge», per differenza (`docs/plans/sistema-di-design.md`).
