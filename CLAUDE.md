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
Regole complete e motivi in `docs/plans/token-e-contesto.md`.
- Una sessione, un compito; oltre i 200k token si passa la mano (`AGENTS.md`, «Coordinamento tra sessioni»), con un passaggio di consegne di 40 righe al massimo. `/compact` guidato tra due fasi, a 100-120k, meglio che alla compattazione automatica.
- All'avvio non si legge `docs/TASKS.md` intero (87 KB): `sed -n 1,12p`, poi `grep -n` del proprio tema e dei file caldi, poi `Read` con `offset`/`limit` sulla voce.
- Mai `Read` intero su `docs/PIANO.md`, `docs/TASKS.md` e sui piani oltre i 30 KB (`revisione-bibliografica*`, `ricerca-*`, `velocita-ricerca`, `classificazione-board`, `collaudo-strumenti`, `revisione-motore`): `grep -n '^#'` per l'indice, poi la sezione.
- Output lunghi su file e `tail -30`; `cargo test -q`, `--message-format short`; `git diff --stat` prima del diff. Non si rilegge un file appena modificato né ciò che è già nel contesto. Chiamate indipendenti nello stesso messaggio.
- GitHub: `minimal_output`, pagine da 5-10; log della CI solo dei job falliti e con poche righe.
- Le letture pesanti (più di tre `grep`, molti file, log, confronti) le fa un subagente Haiku che risponde in meno di 200 parole con `file:riga`.
- Voci di `TASKS.md` «In corso» di tre righe; il racconto e i numeri stanno nel piano del tema.
- Screenshot: solo le pagine toccate, un tema alla volta, il secondo solo se serve il confronto; non l'intera galleria a ogni modifica.
- `list_events` sulle altre sessioni è vietato, salvo per diagnosticare una sessione bloccata. `list_sessions` solo quando l'utente chiede stato o costi, al massimo una volta per turno.

### Avviso sui token
- Se in una sessione l'effort o il consumo superano il previsto (cicli prova-correggi lunghi, letture ripetute, contesto oltre i 150k senza esito vicino, subagenti Opus per lavori da Haiku, un comando bloccato o rilanciato più volte, un compito che si allarga oltre il brief), si avvisa subito l'utente con una riga: cosa sta costando, quanto, e la proposta (cambiare modello o effort, passare la mano, fermarsi). Non si aspetta la fine del compito.
- A fine compito si aggiornano su GitHub, nella stessa PR, `docs/TASKS.md`, `docs/STATO.md` se cambia qualcosa di importante, il piano toccato e ogni documento che il lavoro rende vecchio.

### Risvegli
- Nessun trigger orario, nessun `Monitor` sui log, nessun `sleep`. Ci si sveglia per: messaggio dell'utente, fine di un calcolo in background, evento GitHub della PR a cui si è iscritti, messaggio di un'altra sessione per una PR pronta o un blocco.
- Un calcolo oltre i dieci minuti parte con `run_in_background` e la sessione chiude il turno. Ogni script lungo scrive `out/<nome>/summary.txt` (dieci righe): si legge quello, non il log.
- Una iscrizione agli eventi per PR (`subscribe_pr_activity`), tolta al merge.
- Si scrive a un'altra sessione solo per una PR pronta, un blocco o un file caldo: ogni messaggio le costa un turno intero.

### PR e merge
- La PR si apre con il template di `.github/pull_request_template.md`; la CI parte da sola all'apertura. Dopo un push, si rilancia a mano una volta sola (`actions_run_trigger`, `run_workflow`, `ci.yml`), dopo l'ultimo push.
- Il merge lo fa la coordinatrice in automatico (squash, `expectedHeadSha` con lo SHA completo): solo con CI verde, nessun conflitto e, per il motore, la prova equa sulle 406 board non peggiorata. Non attende la CI a intervalli: si iscrive agli eventi della PR e si sveglia sull'esito. I thread aprono la PR e si fermano. Dopo l'ultimo push la CI si lancia a mano una volta (`ci.yml`).
- I workflow multi-agente consumano molta quota: solo per revisioni o analisi grandi, su richiesta.
- Per verificare il motore si usa la CLI (`combos`, `coverage`, `card-values`, `roles`). Per l'app si usa il build release con `--features tauri/custom-protocol`.

### Skill del repository (`.claude/skills/`)
`brief` (il modello del brief di una sessione), `pr-motore` (protocollo del motore con i comandi), `pr-interfaccia` (verifica con screenshot e anteprima Artifact), `design-to-code` (dal mockup ai componenti), `steward` (convenzioni di merge e CI per chi guida una PR), `coordinamento` (per i giorni con una coordinatrice).
