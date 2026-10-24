# Banco congelato del motore, versione 1

Il banco su cui si accetta una modifica al motore, alla ricerca o alla misura (regole: `docs/plans/regole-motore.md`; script: `scripts/banco.py`). Congelato il 24/10/2026 e mai più cambiato: la versione successiva sarà `v2`.

- `decks/tuning/`, `decks/fresh/`: le 63 liste del banco di generalità (`tests/fixtures/generality/sets.json`), ricostruite dalle fonti con `generality.py rebuild` il 24/10. Solo passcode, nessun testo né immagine.
- `manifest.json`: per ogni lista la fonte (Master Duel Meta o YGOPRODeck, con l'id) e l'impronta (`deck_hash` di `generality.py`). Per `invoked` e `warrior_combo` la lista di oggi ha un'impronta diversa da `sets.json`: quella vecchia è in `manifest_hash`.
- `hands.tsv`: 193 mani. Per ogni deck 3 mani di 5 carte, estratte una volta con `random.Random("banco-v1-<deck>-<k>")` e scritte come passcode, con seme del duello k + 1; più le 4 mani Invoked di `tests/fixtures/hands/bench.tsv` ancora giocabili con la lista di oggi. L'ultima colonna dice in quali giri entra la mano (`quick`: la prima di ogni deck).
- `rules.json`: le soglie, fissate prima di ogni giro.
