# Token e finestra di contesto: regole operative (24/10/2026)

Completa `piano-ambiente-e-risorse.md` §1-§2 (diagnosi e regole di base, che restano valide). Qui ci sono le leve che mancavano, in ordine di resa. Obiettivi invariati: contesto sotto 200k, meno di 30 $ per PR unita, una sessione un compito.

## 1. Cosa c'era già (non si tocca)
Una sessione un compito; soglia 200k con passaggio di consegne; letture mirate (`offset`/`limit`, `grep` prima di `cat`); letture pesanti a un subagente Haiku; calcoli lunghi in background con `summary.txt`; niente trigger, monitor, `sleep`; niente `list_events`; tabella modello/effort; CI rilanciata una volta; avviso a Valerio quando i token sforano.

## 2. Cosa mancava (e perché costa)
Il costo di un turno è la lunghezza della storia, riletta a ogni messaggio. Le regole sopra frenano la crescita, ma restavano tre fonti di contesto «fisso» o evitabile:

1. **L'avvio sessione costa già 25-30k token.** `AGENTS.md` dice di leggere `docs/TASKS.md` (87 KB, circa 22k token: «In corso» da solo è 60 KB) e `CLAUDE.md` + `AGENTS.md` pesano circa 4k. Ogni turno della sessione li rilegge.
2. **Documenti enormi a portata di `Read`**: `PIANO.md` 155 KB, `revisione-bibliografica.md` 228 KB, `-2` 112 KB, `ricerca-letteratura.md` 91 KB, `velocita-ricerca.md` 77 KB, `classificazione-board.md` 72 KB, `ricerca-esperimenti.md` 69 KB. Una lettura intera vale 20-60k token.
3. **Output di comandi e strumenti GitHub senza filtro**: log di `cargo`/`pnpm`, `get_job_logs`, diff di PR intere, elenchi non paginati.

## 3. Regole nuove

### 3.1 Avvio sessione
- Non si legge `docs/TASKS.md` per intero. Si guarda l'intestazione e si cerca ciò che serve: `sed -n 1,12p docs/TASKS.md`, `grep -n '<tema>\|search.rs\|probe.rs\|value.rs\|lib.rs\|api.ts' docs/TASKS.md` (lock dei file caldi), poi `Read` con `offset`/`limit` sulla sola voce che interessa.
- Si legge `docs/STATO.md` solo per le righe dell'area toccata. Il piano del tema, per la sola sezione citata dal brief.
- Il brief nomina i documenti e le sezioni: chi scrive il brief non scrive «leggi i piani», ma `docs/plans/<tema>.md §3`.

### 3.2 Documenti che non si leggono interi
Mai `Read` senza `offset`/`limit` su: `docs/PIANO.md`, `docs/TASKS.md`, `docs/plans/revisione-bibliografica*.md`, `ricerca-letteratura.md`, `ricerca-piano-critico.md`, `ricerca-esperimenti.md`, `velocita-ricerca.md`, `classificazione-board.md`, `collaudo-strumenti.md`, `revisione-motore.md`. Prima `grep -n '^#'` per l'indice, poi la sezione. Le ricerche di letteratura servono per un perché, non per lavorare: si interroga un subagente Haiku («cosa dice il documento su X, in 150 parole»).

### 3.3 Documenti che restano corti (chi li scrive)
- Una voce di `TASKS.md` «In corso» sta in tre righe: compito, branch, file (il lock), più i punti aperti. I numeri e il racconto vanno nel piano del tema. Una voce finita `[x]` passa a «Completati» in una riga e il resto sta nel piano.
- «Completati» tiene solo le ultime due settimane; il resto va in `docs/plans/archivio/`.
- Un piano finito si sposta in `archivio/`; ogni piano ha in cima il sommario di cinque righe (stato, decisione, numeri chiave).
- Il passaggio di consegne è di 40 righe al massimo: stato, numeri, prossimo passo, file toccati, comandi per riprendere.
- Compito di pulizia già pronto per una sessione Sonnet basso: ridurre `TASKS.md` sotto 20 KB (stima: da 22k a 5k token per ogni avvio, per ogni sessione).

### 3.4 Output dei comandi
- Ogni comando che può stampare più di una trentina di righe va filtrato: `cmd > out/x.log 2>&1; tail -30 out/x.log`. Per gli errori: `grep -n 'error\|FAILED\|panicked' out/x.log | head -20`.
- Cargo: `cargo test -q`, `cargo clippy --message-format short`, `cargo build --message-format short`. Pnpm: `pnpm build 2>&1 | tail -15`.
- Git: `git log --oneline -10`, `git diff --stat` prima del diff; il diff intero solo dei file che servono (`git diff -- <file>`).
- Non rileggere un file appena modificato con `Edit`/`Write` (l'esito dice se è riuscito). Non rileggere ciò che è già nel contesto.
- Chiamate indipendenti nello stesso messaggio, non una per turno: ogni turno rilegge la storia.

### 3.5 Strumenti GitHub
- `minimal_output: true` e pagine da 5-10 voci; mai elenchi di tutte le PR o issue.
- CI rossa: `get_job_logs` con `failed_only` e `tail_lines` piccolo (100); mai il log intero.
- Una PR si legge per metodo (`get_files` o `get_diff` solo se serve; `get_status` per la CI), non con tutto insieme. Un commento a una PR si scrive una volta, non a ogni passo.
- Il testo della PR è corto: i numeri stanno in una tabella, non in un racconto.

### 3.6 Subagenti
- Sempre `model: "haiku"` (ricerche, letture) o `"sonnet"` (revisioni). Nel prompt: scopo, file o cartelle, e «rispondi in meno di 200 parole con `file:riga`; nessun brano di codice se non chiesto». La conclusione, non i file, torna nel contesto principale.
- Una ricerca che richiede più di tre `grep` diversi si delega. Una che richiede un solo `grep` no: costa meno farla.
- Niente subagenti per verifiche che la sessione rifarebbe comunque.

### 3.7 Compattazione e durata
- Il momento giusto per `/compact` è tra due fasi (dopo la lettura e il piano, prima di scrivere; dopo i test, prima della PR), con un'istruzione: `/compact conserva: file toccati, decisioni, comandi di verifica e numeri; scarta letture e log`. Meglio a 100-120k che alla compattazione automatica.
- `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=70` in `.claude/settings.json` fa partire quella automatica prima, a un contesto più corto.
- Una pausa più lunga della cache (alcuni minuti, ore) fa pagare l'intero contesto di nuovo alla prima ripresa: un contesto grande e inattivo conviene chiuderlo con il passaggio di consegne piuttosto che riprenderlo.
- Fra due compiti: `/clear` o sessione nuova, mai la stessa.

### 3.8 Ragionamento e immagini
- Effort per fase, come nella tabella: pianificare alto, scrivere medio, rifinire basso. Un ciclo prova-correggi di tre giri senza progresso è il segnale per fermarsi e scrivere cosa si è provato, non per alzare l'effort.
- Uno screenshot costa più di mille token: solo le pagine toccate, un tema alla volta, poi l'altro solo se serve il confronto.
- Connettori e server MCP non necessari al compito (browser, design) non si caricano: ogni definizione di strumento è contesto fisso.

### 3.9 Il budget in pratica
- A 100k token il thread controlla: il compito ha ancora più di un terzo davanti? Se sì, `/compact` guidato o passaggio di consegne.
- A 150k, o se un comando è stato lanciato tre volte per lo stesso errore, avviso a Valerio con una riga (cosa costa, proposta), come già in `CLAUDE.md`.
- Una PR che si allarga oltre il brief si ferma e si divide: la parte nuova è un altro brief.

## 4. Modifiche di questo documento ai file di regole
- `CLAUDE.md`: sezione «Contesto» riscritta con i punti 3.1-3.6 in forma compatta.
- `.claude/settings.json`: `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`, `BASH_MAX_OUTPUT_LENGTH` e `MAX_MCP_OUTPUT_TOKENS` a valori più bassi; divieto di `Read` su `target/` e `node_modules/`.
- `AGENTS.md`, «Inizio sessione»: legge l'intestazione di `docs/TASKS.md` e cerca il proprio tema, non il file intero.
- `.claude/skills/brief/SKILL.md`: «Da leggere» indica sezioni, non documenti.

## 5. Come si misura
Come in `piano-ambiente-e-risorse.md` §11: spesa per PR unita (sotto 30 $), contesto massimo di ogni sessione chiusa (sotto 250k). In più, il contesto a fine lettura iniziale: oggi circa 25-30k, obiettivo sotto 12k.
