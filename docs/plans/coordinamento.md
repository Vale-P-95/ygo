# Coordinamento: stato e regole per la sessione coordinatrice

*Passaggio di consegne del 23/10/2026. La sessione coordinatrice precedente (session_01Qb5hHYmGc86GMHx89tiDSZ) aveva un contesto troppo grande, quindi la si sostituisce con una nuova che parte da questo file. Va aggiornato a ogni cambio importante: è la memoria del coordinamento.*

**Coordinatrice attuale:** session_017U35XxBgAq3CxA7yvL8UCT, dal 23/10. Tutte le corsie attive sono state avvisate di scrivere a lei.

## Ruolo
- La coordinatrice (corsia 1) non scrive codice del motore. Smista il lavoro alle corsie, apre le loro PR, rivede quelle di Gemini e fa il merge.
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
| 2, velocità | session_01By7ydn8GEDXadN1ZzRbcz2 | Opus | `tt-min` (locale), poi nuovi | #104 unita. Punto 2 della caccia ai bug (tabella delle trasposizioni) **respinto** dal test appaiato (179 mani: 3 meglio, 5 peggio, la peggiore −23,5%): lo chiude come «misurata, non tenuta» in `velocita-ricerca.md` con una PR di soli documenti. Ora fa il punto 1 e chiederà il via per `lib.rs` (`look_key`). Decisione sul punto 1: rappresentante del campo con l'opzione C (K = 2), varianti scelte in modo deterministico (più carte in mano, poi più risorse, poi linea più corta) e misurate con la misura economica; fasi finali proporzionali; controllo su 20 mani dell'oracolo a 4000 e 40 000 nodi. In coda: 3, poi 5 (`matchup.rs`, passato dalla corsia 9), poi 6 della caccia ai bug, una PR per punto. Poi l'arena in `probe.rs` quando la corsia 4 lo libera. |
| 3, banco oggettivo | session_01JdBXSyFKWwXxdwUdmvT1Bb | Opus | `claude/tools-qa` | Rifà la baseline su main 1af365d: banco principale, poi le coppie del proprietario (circa 3 ore dalle 11). Quando finisce: aprire la sua PR e dire alla corsia 7 di cominciare §7. |
| 4, misura | session_01ATNjbjqk6NCAw5MYA4EqJg | Opus | `claude/placed-cards` | Opzione (d) **non unita**: `paired_test` REJECTED (90 mani: 9 meglio, 5 peggio, p = 0,42; Dracotail −41%). Migliora l'ordine delle board ma, cambiando i valori delle carte, peggiora la ricerca. Branch fermo a 4cbb5f3; si decide con la ritaratura §7 della corsia 7. `probe.rs` liberato. Ora fa il punto 4 della caccia ai bug (etichette pigre in `actions_with`, `ygo-engine`). |
| 6, oracolo | session_01JJXLcV6HHwvS1WVeEsLBMW | Opus | `claude/oracle-staged` | Ricostruisce le 91 fixture con `--staged 300,30`. Poi apre la PR con i numeri. |
| 7, ricerca | session_01DvhPsFiQuNR9TvtXdWGcRm | Opus | `claude/research-fair` | §6c finita (PR #103). Aspetta la baseline della corsia 3 per §7, la ritaratura dei pesi, che include SUMMONS e il peso del turno dopo. La §7 decide anche la (d) della corsia 4: va misurata con `paired_test` (la ricerca, non solo l'ordine). |
| 8, misura a stadi | session_01VbeVoLcD5jkKB7HDPdmqQk | Opus | `claude/generality-bench` | Opzione (a) nella PR #105. Ora fa il punto 2, il banco di generalità (deck fissati come `deck_hash` più uno script che li ricostruisce, perché le liste scaricate non si committano). Poi il punto 3, lo sweep di robustezza su deck casuali. Liste fissate (43 tuning + 20 fresh): nel repo solo `sets.json` con `deck_hash` e lo script che le ricostruisce controllando l'hash; i `.ydk` restano in `data/` (decisione del 23/10; creare un repo privato da qui dà 403). |
| 9, caccia ai bug (Fable) | session_01MiW9iEY8RRPEsj91Z5Rb2X | Fable | `claude/engine-bughunt` | **Finita** (PR #106). Fermata per risparmiare; il suo punto 5 è passato alla corsia 2. |
| 10, bibliografia (Fable) | session_01FbWtNwW3xdKU3QgDvDGiK2 | Fable | `claude/literature-review-2` | Sta chiudendo `revisione-bibliografica-2.md`. Al massimo 2 lettori, in quest'ordine: la bibliografia annotata dell'utente (16 lavori, con il filone poker: DeepStack, Libratus, Pluribus, ReBeL, Student of Games), il gruppo di articoli 2023-2026 sui giochi di carte collezionabili, le piste del §4. Poi PR e stop. |

**File caldi** (uno alla volta): `search.rs` (corsia 2), `probe.rs` (libero dal 23/10), `value.rs`, `apps/desktop/src-tauri/src/lib.rs` (corsia 2, solo `combo_score`, dal 23/10) e `apps/desktop/src/lib/api.ts` (chi li prende avvisa prima).

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
- #107 (questo file) unita il 23/10.
- #108 (corsia 6, oracolo a gradini) e #109 (corsia 10, bibliografia, seconda passata) unite il 23/10. La corsia 10 è ferma. La corsia 6 prepara l'A1b; il passo 1 lo fa con la corsia 3 dopo la sua baseline.
- #110 (corsia 4: turno di prova senza etichette, −12% di tempo, (d) non tenuta) e #111 (corsia 8: banco di generalità, scarto tuning/fresh −0,4 punti, 63/63 liste ricostruite) unite il 23/10. La corsia 4 è ferma; la corsia 8 passa al punto 3 (sweep di robustezza).
- Gemini sa già che #105 è su `main` (commento sulla #100). Il prompt con «Confronta con il meta» (`gemini/combo-meta`) e poi il passo 4 è stato dato al proprietario il 23/10.

## Gemini (Antigravity)
- Lavora su `apps/desktop/src/` con branch `gemini/*`. Il proprietario lo avvia a mano; si comunica con i commenti sulle sue PR.
- Ogni commento o prompt per Gemini contiene sempre due sezioni (regola di `CLAUDE.md`):
  - i controlli automatici con i comandi esatti;
  - le attività in pausa da ricontrollare in `docs/TASKS.md` su `main`, con cosa fare quando si sbloccano.
- Ogni commento finisce con il piè di pagina `---` / `_Generated by [Claude Code](https://claude.ai/code)_`.
- **Ultimo stato:** #100 (passo 3, ricerca del catalogo) unita. Gli è stato dato il **passo 4** di `docs/plans/ui-revisione-2.md` (`gemini/ui-polish`, passata su tutte le sezioni). L'interfaccia del valore contro il meta (`ComboScore.meta`, `combo_meta`) ha la precedenza appena #105 è su `main`.
- In pausa: le board `cheap: true` (spente con `SEND_CHEAP_SCORES = false` in `lib.rs`); B8, gli obiettivi ricchi (aspetta il contratto da Claude).
- Revisione delle sue PR: area giusta, regole di `AGENTS.md` (protocollo dell'interfaccia), CI verde. Mai commit, rebase o force push sul suo branch.

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
- `docs/plans/ui-revisione-2.md`: passi dell'interfaccia di Gemini.
