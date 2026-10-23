# Coordinamento: stato e regole per la sessione coordinatrice

*Passaggio di consegne del 23/10/2026. La sessione coordinatrice precedente (session_01Qb5hHYmGc86GMHx89tiDSZ) aveva un contesto troppo grande, quindi la si sostituisce con una nuova che parte da questo file. Va aggiornato a ogni cambio importante: è la memoria del coordinamento.*

**Coordinatrice attuale:** session_017U35XxBgAq3CxA7yvL8UCT, dal 23/10. Tutte le corsie attive sono state avvisate di scrivere a lei.

## Stato finale del giro (23/10, sera)
- **Tutte le corsie Claude hanno finito** e le loro PR sono unite: #108-#117, #119 (regole della nuova fase), #120 (A1b, corsia 6; il giro di convergenza a 800k è stato fermato per costo con 6 mani su 12). Le sessioni 2, 6 e 7 si archiviano; le altre lo sono già.
- **Gemini**: resta aperta solo la PR #118 (passo 5, Bottega) con 5 correzioni chieste nella revisione del 23/10 (texture pelle e plastica piatte, scia dell'ombra nel tema chiaro, smussatura, salvataggio perso all'uscita dalla Bottega, colori fuori dai token). È la sua ultima PR; la rivede una sessione Sonnet con gli screenshot (`pr-interfaccia`) e la unisce l'utente.
- **Questa coordinatrice si archivia** dopo la #118. La nuova fase non ha una coordinatrice permanente: le sessioni nascono dal Progetto Claude con il brief (`docs/plans/progetto-claude.md`), una per compito; l'utente unisce dalla pagina GitHub. Una Sonnet con la skill `coordinamento` si apre solo nei giorni con molte PR, e parte da questo file.
- **Branch remoti da cancellare dalla pagina GitHub** (la sessione cloud non può: il server rifiuta le cancellazioni). Tutti con PR unita o vuoti: `claude/` engine-review, links-chance, literature-review, literature-review-2, oracle, oracle-run, oracle-top, oracle-staged, oracle-a1b, research-a1b, research-b9, research-l23, research-lab, research-fair, research-weights, roles-fix, engine-bughunt, yugioh-deck-builder-simulator-cax0ya, merge-card-viewer, placed-cards, search-speed, staged-measure, tools-qa, matchup-diverged, coordinamento, chiusura-giro (dopo questa PR); `gemini/` tutti tranne `ui-shelf` finché la #118 è aperta. Da decidere: `claude/paired-test` (2 file, nessuna PR) e `claude/research-data` (105 file di dati, nessuna PR). Attivare «Automatically delete head branches» nelle impostazioni del repository.
- **Costi del giro** (`list_sessions`, 23/10): coordinatrice vecchia 1 244 $; corsie da 51 a 156 $ l'una, con contesti da 380k a 770k; questa coordinatrice circa 10 $. Diagnosi e regole in `piano-ambiente-e-risorse.md`, §1-2.
- **Prossimi passi**: §10 di `piano-ambiente-e-risorse.md`; voci in `docs/TASKS.md`; primo compito l'ambiente cloud; promemoria all'utente: configurare il Progetto Claude.

## Ruolo
- La coordinatrice (corsia 1) non scrive codice del motore. Smista il lavoro alle corsie, apre le loro PR, rivede quelle di Gemini (solo la #118) e fa il merge.
- Parla con l'utente in italiano, in breve, solo quando c'è qualcosa di nuovo: merge, numeri di miglioramento, problemi, decisioni da prendere.
- **Merge:** solo con CI verde e senza conflitti, `merge_method: squash`, `expectedHeadSha` con lo SHA completo. Mai automatico con conflitti.
- **CI:** gira da sola solo all'apertura della PR. Dopo un push nuovo si rilancia a mano: `actions_run_trigger` con `run_workflow`, `ci.yml` sul branch della PR. Le PR di soli documenti hanno solo il job «Changed».
- Le sessioni sorelle si raggiungono con `send_message` (server `claude-code-remote`) e rispondono allo stesso modo. Le loro risposte arrivano come notifiche: si leggono con `ReadNotifications`.

## Risparmio di token (richiesta dell'utente, 23/10)
- Niente monitor di git che svegliano la sessione a ogni push. Al massimo un monitor della CI per PR, che si chiude da solo al primo esito.
- Niente controllo orario: il trigger `trig_01SbfXjVRep5QMXiMgYjhZBi` è disattivato. Le corsie scrivono quando hanno una PR pronta o sono bloccate.
- Niente agenti di verifica in Chromium, tranne per le PR grandi di Gemini.
- Niente letture ripetute dei transcript delle altre sessioni (`list_events` costa molto): ci si fida dei loro messaggi.
- Le corsie Fable (9 e 10) costano di più per token: si fermano appena finito il loro compito.
- Il modello e il livello di ragionamento di una sessione esistente non si cambiano da qui: lo fa l'utente con `/model` e `/effort` nella sessione.

## Principi dell'utente (non negoziabili)
- Il motore è **generale**: deve funzionare con qualunque deck e con carte nuove. Nessuna regola per nome di carta.
- **Nessuna linea precalcolata o salvata su disco.** Le linee si riconoscono sul momento. Per questo l'opzione (b), aperture avversarie in cache, è stata **rifiutata**.
- L'oracolo e i riferimenti sono automatici, mai linee scritte a mano. Niente ricalcoli quando cambiano le liste del meta: la generalità si misura su deck mai visti.
- Ogni cambio della misura si giudica prima con `scripts/research/a1b_fair_measure.py` su tutte le 406 board (prova equa), non su campioni scelti.

## Corsie attive
| Corsia | Sessione | Modello | Branch | Stato al passaggio |
|---|---|---|---|---|
| 2, velocità | session_01By7ydn8GEDXadN1ZzRbcz2 | Opus | `claude/search-speed` | finita (PR #115), da archiviare |
| 3, banco oggettivo | session_01JdBXSyFKWwXxdwUdmvT1Bb | Opus | `claude/tools-qa` | archiviata (23/10) |
| 4, misura | session_01ATNjbjqk6NCAw5MYA4EqJg | Opus | `claude/placed-cards` | archiviata (23/10) |
| 6, oracolo | session_01JJXLcV6HHwvS1WVeEsLBMW | Opus | `claude/oracle-a1b` | finita (PR #120), da archiviare |
| 7, ricerca | session_01DvhPsFiQuNR9TvtXdWGcRm | Opus | `claude/research-weights` | finita (PR #117), da archiviare |
| 8, misura a stadi | session_01VbeVoLcD5jkKB7HDPdmqQk | Opus | `claude/generality-bench` | archiviata (23/10) |
| 9, caccia ai bug (Fable) | session_01MiW9iEY8RRPEsj91Z5Rb2X | Fable | `claude/engine-bughunt` | archiviata (23/10) |
| 10, bibliografia (Fable) | session_01FbWtNwW3xdKU3QgDvDGiK2 | Fable | `claude/literature-review-2` | archiviata (23/10) |

**File caldi** (uno alla volta): `search.rs`, `probe.rs`, `value.rs`, `apps/desktop/src-tauri/src/lib.rs` e `apps/desktop/src/lib/api.ts`: tutti liberi dalla sera del 23/10 (chi li prende lo scrive in `TASKS.md`).

## PR al passaggio
- Unite il 23/10:
  - #100 (Gemini, ricerca del catalogo);
  - #102 (corsia 4);
  - #103 (corsia 7, §6c);
  - #104 (corsia 2, copie nelle fasi dopo la ricerca);
  - #105 (corsia 8, opzione (a); la richiesta per Gemini è in `docs/TASKS.md`);
  - #106 (corsia 9, caccia ai bug).
- **Promemoria per l'utente:** quando tutte le corsie hanno finito e le sessioni sono archiviate, ricordargli di configurare un «progetto» claude.ai (memoria e avvisi tra sessioni; aprirlo ferma la sessione corrente).
- **Niente compiti nuovi** (decisione dell'utente, 23/10): ogni corsia chiude il compito in corso, manda la PR e si ferma.
- #112 (corsia 8, sweep di robustezza: 286 deck, 0 crash; solo Vendread tocca il tetto Lua) unita il 23/10. #113 (Gemini, «Confronta con il meta»): 4 correzioni chieste nel commento di revisione.
- #113 (Gemini, «Confronta con il meta», non provata nell'app vera) e #114 (corsia 3, nuova base del banco oggettivo) unite il 23/10. Sessioni archiviate: corsie 3, 4, 8, 9, 10. Restano 2, 6, 7.
- #115 (corsia 2: fasi proporzionali; TT e rappresentante del campo misurati e non tenuti), #116 (Gemini, passo 4 dell'interfaccia) e #117 (corsia 7, §7: (d) respinta; proposta resilienza 0,6→0 e floodgate 0,8→0,4, da confermare sul banco di generalità e cambiare insieme in bench.py, CLI, lib.rs, scoring.ts) unite il 23/10. Resta attiva solo la corsia 6 (A1b).
- #107 (questo file) unita il 23/10.
- #108 (corsia 6, oracolo a gradini) e #109 (corsia 10, bibliografia, seconda passata) unite il 23/10. La corsia 10 è ferma. La corsia 6 prepara l'A1b; il passo 1 lo fa con la corsia 3 dopo la sua baseline.
- #110 (corsia 4: turno di prova senza etichette, −12% di tempo, (d) non tenuta) e #111 (corsia 8: banco di generalità, scarto tuning/fresh −0,4 punti, 63/63 liste ricostruite) unite il 23/10. La corsia 4 è ferma; la corsia 8 passa al punto 3 (sweep di robustezza).
- Gemini sa già che #105 è su `main` (commento sulla #100). Il prompt con «Confronta con il meta» (`gemini/combo-meta`) e poi il passo 4 è stato dato al proprietario il 23/10.

## Gemini (Antigravity)
- Ultima PR: #118 (passo 5 dell'interfaccia, `gemini/ui-shelf`), in revisione con 5 correzioni chieste.
- Dopo la #118 niente più lavoro di Gemini dal 24/10: tutto il codice lo fanno sessioni di Claude Code (`docs/plans/piano-ambiente-e-risorse.md`).
- I suoi branch `gemini/*` vanno ripuliti (si cancellano quelli uniti).

## Opzioni della misura decise dall'utente
- **(a)** L2-3, ultimo gradino dal vivo sulle board migliori: PR #105. Su 80 mani la #1 cambia in 29, meglio in 12 e peggio in 1; la #1 è la migliore vera in 53 mani invece di 40. Costa 4-5 volte il resto della valutazione, quindi si lancia a richiesta.
- **(c)** Kit a catena: **rifiutata**. Sulla prova equa 73-80% contro 79% della base.
- **(d)** Pesi delle Evocazioni dalle linee vere: 82% contro 79% delle coppie, p = 0,0003. Il guadagno è concentrato su Sky Striker e Clown Crew e sparisce contando il turno dopo. In corso nella corsia 4, con la conferma sul banco della corsia 3.

## Regole fisse
- Mai modificare `vendor/ygopro-core`. Mai committare `data/`, liste di MDM, immagini o testi Konami.
- Niente ID di modello nei commit o nelle PR. I commit finiscono con le righe di attribuzione della sessione; i corpi delle PR finiscono con «🤖 Generated with [Claude Code](https://claude.com/claude-code)» e il link della sessione.
- I cambi del motore seguono il protocollo di `AGENTS.md` più `scripts/paired_test.py`. Nelle PR vanno i numeri prima e dopo.
- Le PR verso `main` le apre la coordinatrice; il merge lo fa la coordinatrice con CI verde (l'utente l'ha autorizzato).

## Documenti da conoscere
- `docs/TASKS.md`: lavori e lock.
- `docs/plans/ricerca-piano-critico.md`: piano critico, A1b.
- `docs/plans/ricerca-esperimenti.md`: §6, §6b, §6c.
- `docs/plans/velocita-ricerca.md`: B13, B14, L2-3.
- `docs/plans/caccia-bug-motore.md`: caccia ai bug.
- `docs/plans/revisione-motore.md`, `docs/plans/revisione-bibliografica.md` e `-2.md`.
- `docs/plans/ui-revisione-2.md`: passi dell'interfaccia (fino al passo 5 Gemini, poi Claude).
