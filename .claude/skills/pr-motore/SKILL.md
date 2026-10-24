---
name: pr-motore
description: Il protocollo per una PR che tocca il motore, la ricerca o il turno di prova (crates/ygo-engine, crates/ygo-solver), con i comandi esatti e la tabella dei numeri prima/dopo da mettere nella PR.
---

# PR del motore

Vale per `crates/ygo-engine` e `crates/ygo-solver`. Nessun cambio del motore entra senza numeri prima e dopo, misurati sullo stesso insieme di mani.

## Prima di cominciare
```
python3 scripts/mdm_decks.py --out out/decks        # liste di Master Duel Meta: mai committate
cargo build --release -p ygo-cli
```
Base di confronto: `main` al commit da cui parte il branch. I risultati della base si calcolano una volta e si tengono in `out/base/`.

## Controlli rapidi (in sessione, pochi minuti)
```
cargo fmt --all -- --check
cargo clippy -p ygo-engine -p ygo-data -p ygo-stats -p ygo-solver -p ygo-cli --all-targets -- -D warnings
cargo test --release -p ygo-solver --test <il test del pezzo toccato>
cargo test --release -p ygo-solver --test search_regression      # ~2 min
```

## Prove lunghe (in background: la sessione chiude il turno e riprende alla notifica)
- **Banco congelato** (l'arbitro del merge, `docs/plans/regole-motore.md`): classe E, R o M scritta nella PR prima del giro; `python3 scripts/banco.py run out/banco --old <cli di main> --new <cli del branch> --set full --jobs 4` (circa 35 minuti su 4 core), poi `banco.py same out/banco` (E), `banco.py judge out/banco` (R) o, per M, `run ... --measure-cli <cli del branch>` e `judge out/banco --measure-cli <cli del branch>`. Si unisce solo con ACCEPTED. Senza dati o tempo: `bench.yml` con `bench: banco-full` e `check`.
- **Test appaiato** (`paired_test.py`) e vecchio banco (`bench.py`): solo per gli studi su mani scelte, non per il merge.
- **Verità della misura**: fino alla verità v1 (`regole-motore.md` §4) un cambio della misura lo decide il proprietario.
- **Banco di generalità** (per ogni costante nuova): `python3 scripts/generality.py` su tuning e fresh; si respinge se fresh scende di oltre 1 punto con tuning fermo, se lo scarto cresce di oltre 2 punti, o se qualcosa di nuovo si rompe su fresh (`docs/plans/banco-generalita.md`).
- **Copertura e caccia**: `duelforge-cli coverage --deck <deck>` sui deck toccati; `python3 scripts/combo_hunt.py out/decks` come regressione.
Ogni script lungo scrive `out/<nome>/summary.txt`: nella PR si incolla quello.

## Nella PR
```
| Misura | Prima (main <sha>) | Dopo (<sha>) |
|---|---|---|
| Classe | E / R / M |
| banco.py same o judge (193 mani) | — | summary.txt: +a / −b / =c, media e intervallo, p, grandi perdite, ACCEPTED |
| search_regression | verde | verde |
| Tempo (ricerca / misura / turno di prova) | … s | … s |
| Generalità tuning / fresh | …% / …% | …% / …% |   ← solo se c'è una costante nuova
```
Regole fisse: nessun caso speciale per nome di carta; una correzione vale per qualunque deck; se `banco.py` respinge, la modifica si scrive come «misurata, non tenuta» nel piano del tema e non entra.
