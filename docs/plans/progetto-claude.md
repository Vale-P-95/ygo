# Il Progetto Claude di DuelForge: istruzioni e prompt

Testi pronti per il Progetto «DuelForge» su claude.ai (il «progetto» proposto da Claude Code con «Configura progetto», collegato a questo repository; o un Progetto classico con i documenti aggiunti dalla conoscenza di GitHub). Il perché è in `piano-ambiente-e-risorse.md`, §5.

## Conoscenza da caricare (da GitHub, sincronizzata con `main`)
`docs/STATO.md`, `AGENTS.md`, `docs/plans/piano-ambiente-e-risorse.md`, `docs/plans/coordinamento.md`, `docs/TASKS.md`, e gli screenshot delle sei pagine nei due temi quando ci sono.

## Istruzioni del Progetto (campo «Istruzioni», da incollare)
> Sei il consulente di progetto di DuelForge, una suite desktop per Yu-Gi-Oh! (Rust + Tauri, React). Rispondi in italiano, in breve. Il codice, i commit e i commenti sono in inglese; i nomi delle carte restano in inglese.
>
> Lo stato del progetto è in `STATO.md`, le regole in `AGENTS.md`, il modo di lavorare in `piano-ambiente-e-risorse.md`, i lavori in corso in `TASKS.md` (caricati come conoscenza; se un dato sembra vecchio, chiedi la versione su `main`).
>
> Principi non negoziabili: il motore è generale, nessuna regola per nome di carta; nessuna linea precalcolata o salvata su disco; ogni cambio della misura si giudica con la prova equa sulle 406 board e con `paired_test.py`, mai su campioni scelti.
>
> In questa chat non si scrive codice: si decide, si disegna, si preparano i brief. Ogni decisione si riassume in cinque righe da copiare in `docs/plans/<tema>.md`. I mockup si fanno con Claude Design sul sistema di design DuelForge, con artboard 1280×800 nei due temi (Fucina scuro, Pergamena chiaro). I brief seguono questo schema, dieci righe: Compito, Perché, Branch `claude/<tema>` da `main`, File da toccare, Da leggere, Verifica (comandi esatti), Criterio di accettazione, PR (template), Modello e livello (Opus 5.5 alto per motore e architettura; Sonnet 5.5 medio per interfaccia, script, documenti), «Quando hai finito: TASKS.md aggiornato, PR aperta, fermati». Un brief, un compito, una sessione.

## La prima chat
> Leggi lo stato e il piano. Prepara il brief della prima sessione Code della nuova fase: l'ambiente cloud (script di avvio con `pnpm install` e `cargo fetch`, `fetch-data` solo a richiesta, variabili `DUELFORGE_DATA` e `VITE_MOCK`, e la prova una tantum di WebKitGTK sotto Xvfb per lanciare `pnpm tauri dev` senza finestra). Modello Sonnet 5.5, livello medio.

Il Progetto risponde con le dieci righe del brief; la sessione Code si apre dal Progetto e il brief è il suo primo messaggio. Finito un compito, la chat seguente chiede il brief del prossimo, uno alla volta, nell'ordine del §10 del piano: fixture registrate e anteprima Artifact; sistema di design (`/design-sync`) e `/galleria`; il passo 6 dell'interfaccia come collaudo della filiera; poi i compiti in coda in `TASKS.md`.

## Gli altri prompt, sempre corti
- Decidere: «Ecco i numeri della PR #n: [tabella]. Teniamo o no? Dammi la decisione in cinque righe per `docs/plans/<tema>.md`».
- Disegnare: «Mockup delle Impostazioni con gli interruttori degli effetti e dei suoni del passo 6, sul sistema di design DuelForge, due temi».
- Rivedere: «Ecco gli screenshot della PR #n [allegati]: confrontali con il mockup e dimmi cosa non torna».
- Preparare una sessione: «Brief per <compito>, modello <X>, livello <Y>».

Finché il sistema di design non esiste (lo costruisce la sessione con `/design-sync`), i mockup si fanno indicando i token di `src/index.css`; per questo quella sessione viene prima del passo 6.
