---
name: pr-interfaccia
description: Come si verifica e si presenta una PR dell'interfaccia (apps/desktop/src) con i propri screenshot, l'anteprima Artifact e la lista del protocollo di AGENTS.md. Da usare prima di dichiarare pronta una PR del frontend.
---

# PR dell'interfaccia

Il protocollo è in `AGENTS.md`, «Protocollo per chi tocca l'interfaccia». Qui ci sono i comandi e l'ordine.

## 1. Controlli automatici
```
cd apps/desktop
pnpm lint && pnpm typecheck && pnpm test && pnpm build 2>&1 | tail -5
```

## 2. Guardare con i propri occhi
```
cd apps/desktop
VITE_MOCK=1 pnpm dev > /tmp/vite.log 2>&1 &
node scripts/screens.mjs --url http://localhost:1420 --out /tmp/screens --axe
```
- Si guardano (con `Read`) gli screenshot delle sole pagine toccate, nei due temi, e si confrontano con il mockup se c'è. Non tutta la galleria.
- Si controlla `/tmp/screens` per gli errori di accessibilità di axe sulle pagine toccate (contrasto ≥ 4,5:1).
- Si legge `/tmp/vite.log` con `grep -i "error\|warn"`: nessun errore in console.
- Gli screenshot delle pagine toccate si mandano all'utente in chat (`SendUserFile`) o si allegano come artifact della CI.

## 3. Anteprima per l'utente
```
cd apps/desktop && VITE_MOCK=1 pnpm build 2>&1 | tail -3
```
Si pubblica `dist/index.html` con i file di `dist/assets/` come Artifact a più file, **allo stesso URL** dell'anteprima precedente (si legge in `docs/plans/coordinamento.md`; se non c'è, si crea e si annota). Font e texture pesanti restano fuori se superano i limiti.

## 4. App vera
Quando la PR tocca una pagina intera, la Bottega 3D, i suoni o le prestazioni del motore dall'app, serve `pnpm tauri dev` sul computer dell'utente (sessione Claude Code desktop, o Remote Control). Se non si può, la PR lo dice.

## 5. La lista nella PR (template)
Per ogni punto: come è stato verificato (app vera, screenshot, backend finto, solo `typecheck`).
- id di richiesta sui caricamenti; risposte vecchie o di un altro deck scartate;
- stato azzerato al cambio di deck, salvataggio o ricomincio; caricamento in corso annullato;
- nessun `catch` vuoto; ogni errore in italiano, visibile;
- dipendenze degli effetti complete; listener e timer tolti nella pulizia;
- nessun `as any`, `@ts-ignore`, `@ts-expect-error`;
- tipi di `api.ts` per tutto ciò che passa tra pagine o si salva;
- colori dai token di `src/index.css`, nei due temi;
- se il contratto cambia: `lib.rs`, `api.ts`, `mock.ts` e le fixture nella stessa PR.
