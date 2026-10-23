@AGENTS.md

## Solo per Claude Code
Le regole di `AGENTS.md` valgono per tutti. Queste riguardano solo come una sessione Claude Code spende token e quota; il perché è in `docs/plans/piano-ambiente-e-risorse.md`.

### Modello e livello di ragionamento
| Compito | Modello | `/effort` |
|---|---|---|
| Architettura, bug difficili del motore, algoritmi di ricerca, decisioni di misura, contratto `lib.rs` ↔ `api.ts`, stato di `Combo.tsx` e caricamenti asincroni | Opus 5.5 | alto |
| Componenti e pagine dell'interfaccia, Tailwind, test, script Python, documenti, revisione di una PR, CLI | Sonnet 5.5 | medio |
| Coordinamento (PR, merge, `coordinamento.md`), lint, stringhe in italiano, rinomine | Sonnet 5.5 o Haiku 5.5 | basso |
| Subagenti: ricerca nel codice, riassunti di log, confronto di screenshot | Haiku 5.5 | basso |
| Ragionamento profondo su un problema delimitato, con contesto sotto 100k | Fable 5.1 | alto |

- Fasi di un compito: pianificare ad alto, implementare a medio, rifinire a basso.
- Fable 5.1 mai per letture lunghe, cicli prova-correggi o sessioni di ore.
- Ogni `Agent` riceve `model: "haiku"` (ricerche, letture) o `"sonnet"` (revisioni); altrimenti eredita Opus.
- `/fast` per scrivere documenti già decisi o codice meccanico.

### Contesto
- Una sessione, un compito; oltre i 200k token si passa la mano (`AGENTS.md`, «Coordinamento tra sessioni»).
- Le letture pesanti (molti file, log, confronti) le fa un subagente Haiku che restituisce la conclusione; il contesto principale resta corto.
- `Read` con `offset`/`limit`; `grep` prima di `cat`; mai `docs/PIANO.md` per intero. L'output dei comandi lunghi va in un file e si legge con `tail`.
- Screenshot: solo le pagine toccate, nei due temi; non l'intera galleria a ogni modifica.
- `list_events` sulle altre sessioni è vietato, salvo per diagnosticare una sessione bloccata. `list_sessions` solo quando l'utente chiede stato o costi, al massimo una volta per turno.
- **Avviso sui consumi** (richiesta dell'utente, 24/10): se i token corrono più del previsto, la sessione lo dice subito all'utente in una riga, senza aspettare la fine. Segnali: metà del budget del brief spesa prima di metà del lavoro, contesto oltre i 150k, tre cicli prova-correggi sullo stesso errore, un comando bloccato o rilanciato più volte, un compito che si allarga oltre il brief. La riga dice cosa costa, perché, e propone una via: fermarsi, stringere il compito, cambiare modello o effort, passare la mano.

- **A fine compito si aggiorna tutto su GitHub**, nella stessa PR: `docs/TASKS.md`, `docs/STATO.md`, il piano del tema (`docs/plans/<tema>.md`), il punto chiuso di `docs/plans/piano-ambiente-e-risorse.md` se è uno dei suoi, e le skill o i documenti che il lavoro cambia.

### Risvegli
- Nessun trigger orario, nessun `Monitor` sui log, nessun `sleep`. Ci si sveglia per: messaggio dell'utente, fine di un calcolo in background, evento GitHub della PR a cui si è iscritti, messaggio di un'altra sessione per una PR pronta o un blocco.
- Un calcolo oltre i dieci minuti parte con `run_in_background` e la sessione chiude il turno. Ogni script lungo scrive `out/<nome>/summary.txt` (dieci righe): si legge quello, non il log.
- Una iscrizione agli eventi per PR (`subscribe_pr_activity`), tolta al merge.
- Si scrive a un'altra sessione solo per una PR pronta, un blocco o un file caldo: ogni messaggio le costa un turno intero.

### PR e merge
- La PR si apre con il template di `.github/pull_request_template.md`; la CI parte da sola all'apertura. Dopo un push, si rilancia a mano una volta sola (`actions_run_trigger`, `run_workflow`, `ci.yml`), dopo l'ultimo push.
- Il merge lo fa l'utente dalla pagina GitHub, o la sessione quando l'utente lo delega: squash, `expectedHeadSha` con lo SHA completo, CI verde, nessun conflitto.
- I workflow multi-agente consumano molta quota: solo per revisioni o analisi grandi, su richiesta.
- Per verificare il motore si usa la CLI (`combos`, `coverage`, `card-values`, `roles`). Per l'app si usa il build release con `--features tauri/custom-protocol`.

### Skill del repository (`.claude/skills/`)
`brief` (il modello del brief di una sessione), `pr-motore` (protocollo del motore con i comandi), `pr-interfaccia` (verifica con screenshot e anteprima Artifact), `design-to-code` (dal mockup ai componenti), `steward` (convenzioni di merge e CI per chi guida una PR), `coordinamento` (per i giorni con una coordinatrice).
