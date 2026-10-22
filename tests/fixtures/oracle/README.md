# Oracolo della ricerca delle combo

Le board migliori trovate con un budget molto più grande di quello normale, per un insieme fisso di mani (piano: `docs/plans/ricerca-piano-critico.md`, A1). Le scrive `scripts/oracle.py fixtures`; `scripts/oracle.py measure` confronta con queste la ricerca normale.

- Un file per mano: `bench-<nome>` (mani del banco, seme 1), `hunt-<deck>-<k>` (mani di `combo_hunt --seed 7`), `goal-<nome>` (ricerche per obiettivo).
- Dentro: mano (passcode), seme, budget (`nodes`), impronta della lista (`deck_hash`, perché le liste di Master Duel Meta cambiano e non si committano), le board migliori con codici e zone, la misura e le risposte della linea migliore. Nessun testo di carta.
- Quelle di oggi vengono da un pilota a 40 000 nodi, sulle mani del banco: vanno rifatte con il giro lungo (50 volte i nodi, 3 semi).
