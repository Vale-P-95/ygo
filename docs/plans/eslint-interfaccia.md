# Piano: Controlli automatici dell'interfaccia nella CI (ESLint)

## Obiettivo
Aggiungere ESLint all'applicazione frontend desktop (`apps/desktop`) per far rispettare in modo automatico e continuativo le regole del protocollo stabilito in `AGENTS.md`:
- `@typescript-eslint/no-explicit-any`: livello `error` (nessun tipo `any` permesso).
- `react-hooks/rules-of-hooks`: livello `error`.
- `react-hooks/exhaustive-deps`: livello `error` (tutte le dipendenze degli effetti devono essere complete).
- Script `pnpm lint` in `apps/desktop/package.json`.
- Integrazione del comando `pnpm lint` nel workflow CI (`.github/workflows/ci.yml`) nel job «Frontend (TypeScript)».
- Bonifica di tutte le violazioni esistenti nel codice (`apps/desktop/src/**`) senza soppressioni riga per riga (`eslint-disable`).

## Architettura e Pacchetti
- Utilizzo della moderna configurazione flat di ESLint 9 (`apps/desktop/eslint.config.js`).
- Pacchetti dev in `apps/desktop`:
  - `eslint`
  - `@eslint/js`
  - `typescript-eslint`
  - `eslint-plugin-react-hooks`
- Regole configurate:
  - Raccomandate di ESLint JS.
  - Raccomandate di `typescript-eslint`, con `@typescript-eslint/no-explicit-any: error`.
  - Plugin `eslint-plugin-react-hooks` con `react-hooks/rules-of-hooks: error` e `react-hooks/exhaustive-deps: error`.
  - Ignorati: `dist/`, `src-tauri/`, `node_modules/`.

## Fasi di Implementazione
1. **Configurazione ESLint**:
   - Installare i pacchetti con `pnpm add -D ...` (con `BypassSandbox: true` secondo le regole di sistema per Windows).
   - Creare `apps/desktop/eslint.config.js`.
   - Aggiungere lo script `"lint": "eslint ."` in `apps/desktop/package.json`.
2. **Audit del codice**:
   - Eseguire `pnpm lint` per rilevare tutti i file che violano le regole.
3. **Risoluzione delle violazioni**:
   - Sostituire ogni occorrenza di `any` con tipi sicuri (`unknown`, narrowing TypeScript, tipi contrattuali da `api.ts`, tipi nativi per eventi/DOM).
   - Sistemare tutte le dipendenze degli effetti e dei callback React (`useEffect`, `useCallback`, `useMemo`), evitando loop o listener non ripuliti.
4. **Aggiornamento Workflow CI**:
   - Inserire `- run: pnpm lint` nel job `frontend` di `.github/workflows/ci.yml` subito prima di `typecheck`.
5. **Verifica e Collaudo**:
   - `cd apps/desktop; pnpm lint` -> 0 errori.
   - `cd apps/desktop; pnpm typecheck` -> 0 errori.
   - `cd apps/desktop; pnpm build` -> build di produzione riuscita.
   - `cargo fmt --all -- --check`.
6. **PR e Coordinamento**:
   - Creazione della PR su GitHub da `gemini/eslint` a `main`.
