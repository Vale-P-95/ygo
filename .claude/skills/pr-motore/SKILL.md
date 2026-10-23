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
- **Banco**: `python3 scripts/bench.py out/bench --decks out/decks --nodes 4000` prima e dopo. Oltre le due ore si usa `bench.yml` su GitHub Actions (`workflow_dispatch`, con `base` e `head`).
- **Test appaiato** (l'arbitro per ogni cambio della ricerca o della misura): `python3 scripts/paired_test.py <cartella base> <cartella nuova>`; si accetta solo se dice ACCEPTED. Si riporta: mani meglio/peggio/pari, la peggiore con il suo deck, p.
- **Prova equa della misura**: ogni cambio della misura si giudica con `scripts/research/a1b_fair_measure.py` su tutte le 406 board, mai su campioni scelti.
- **Banco di generalità** (per ogni costante nuova): `python3 scripts/generality.py` su tuning e fresh; si respinge se fresh scende di oltre 1 punto con tuning fermo, se lo scarto cresce di oltre 2 punti, o se qualcosa di nuovo si rompe su fresh (`docs/plans/banco-generalita.md`).
- **Copertura e caccia**: `duelforge-cli coverage --deck <deck>` sui deck toccati; `python3 scripts/combo_hunt.py out/decks` come regressione.
Ogni script lungo scrive `out/<nome>/summary.txt`: nella PR si incolla quello.

## Nella PR
```
| Misura | Prima (main <sha>) | Dopo (<sha>) |
|---|---|---|
| paired_test (N mani) | — | +a / −b / =c, peggiore <deck> −x%, p = … |
| Banco (somma / best) | … | … |
| search_regression | verde | verde |
| Tempo (ricerca / misura / turno di prova) | … s | … s |
| Prova equa (406 board) | …% | …% |   ← solo se cambia la misura
| Generalità tuning / fresh | …% / …% | …% / …% |   ← solo se c'è una costante nuova
```
Regole fisse: nessun caso speciale per nome di carta; una correzione vale per qualunque deck; se il test appaiato respinge, la modifica si scrive come «misurata, non tenuta» nel piano del tema e non entra.
