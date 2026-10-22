# Taratura dei pesi del punteggio (D17)

*22/10/2026, coordinatrice. Fa parte di #47 (`classificazione-board.md`, passo D17). Dati: il banco oggettivo della corsia 3 (`tests/fixtures/boards/objective-bench.json`, `owner-bench.json`; lettura in `banco-oggettivo.md`).*

## Come si è misurato
- **Verità di una board:** la quota di mosse del meta che ferma nel turno avversario (1 − left di `probe-line --vs X --moves`), pesata con la popolarità degli avversari (MDM), senza le coppie dubbie (meno dell'80% di mosse fatte su campo vuoto).
- **Taratura e verifica:** 200 board su 41 deck; 70% dei deck per tarare e 30% per verificare, con seme 47.
- **Script:** `scripts/tune_weights.py`.
  - Griglia sui pesi di handtrap, floodgate e resilienza; le interruzioni restano a 1.
  - Si confrontano coppie di board dello stesso deck con verità diverse di almeno 0,02.
  - Si sceglie, tra le combinazioni a una coppia dal massimo, quella più vicina ai pesi di oggi.
- **Cosa resta fuori:** follow-up, risorse, costo ed esposizione non si tarano qui, perché la verità guarda solo il turno avversario.

## Risultati
| Pesi (interruzioni, handtrap, floodgate, resilienza) | Taratura: coppie giuste | Verifica: coppie giuste | ρ con la verità (taratura / verifica) |
|---|---|---|---|
| oggi (1; 0,9; 0,8; 0,6) | 106,5 su 137 (78%) | 45 su 50 (90%) | 0,84 / 0,64 |
| tarati (1; 0,95; 0,8; 0,15) | 112,5 su 137 (82%) | 45 su 50 (90%) | 0,87 / 0,63 |
| solo interruzioni | 74% | 91% | 0,74 / 0,50 |
| solo resilienza | 73% | 93% | 0,73 / 0,62 |
| solo floodgate | 51% | 50% | 0,04 / −0,28 |

- I pesi tarati migliorano solo le coppie di taratura; in verifica restano uguali, quindi è sovradattamento.
- `countered` non segue la verità (ρ 0,01 su 85 board): resta solo da mostrare.
- **Coppie del proprietario** (`owner-bench.json`, 77 board):
  - il banco quasi non le separa, perché 31 coppie su 38 differiscono di meno di 0,02;
  - l'accordo con i verdetti è basso per ogni misura: oggi 6 su 32, tarati 5 su 32, verità del banco 1 su 32.
  - I criteri del proprietario che il turno di prova non misura sono elencati in `banco-oggettivo.md`:
    - una trappola attivabile dalla mano vale più in mano che piazzata;
    - un mostro nel Cimitero può essere un'interruzione;
    - spostare una zona per schivare un bersaglio conta.

    Sono punti di B5 (`ricerca-piano-critico.md`), non questioni di pesi.

## Decisione
1. **I quattro pesi del turno avversario restano come oggi:** 1; 0,9; 0,8; 0,6. Una combinazione non batte l'altra fuori dai dati di taratura, quindi niente cambio senza una ragione misurata.
2. **Esposizione.** Dalla PR #65 `exposure` è la perdita attesa per le handtrap, in interruzioni (mediana 0,91 per board). Il peso di −0,04 era tarato sul vecchio conteggio dei punti critici, e oggi l'esposizione quasi non pesa.

   Pesi nuovi per profilo:

   | Profilo | Esposizione |
   |---|---|
   | Bilanciato | −0,5 |
   | Più negazioni | −0,4 |
   | Controllo e floodgate | −0,4 |
   | A prova di board breaker | −0,6 |
   | Risorse e follow-up | −0,3 |
   | Linee sicure | −1,0 |

   - Con −1 la perdita attesa conta quanto le interruzioni perse. Il Bilanciato ne conta la metà, perché la misura è ottimista sulle linee di riserva (V1, `esposizione.md`).
   - **Pesi salvati dall'utente o imparati:** se l'esposizione salvata è nella scala vecchia (valore assoluto sotto 0,2), si moltiplica per 12,5, cioè 0,5 diviso 0,04, una volta sola al caricamento.
3. **Carte tolte all'avversario** (`Measure::attrition`, PR #73). Per ora non entrano nel punteggio dell'app: serve prima il campo in `ScoreView`/`api.ts` e una verità che le veda. Il banco misura solo il turno avversario, e un Extra Deck bandito pesa sui turni dopo. Si rivede con B5 e con il banco rifatto.
4. **Il banco va rifatto come nuova base** dopo B5a (#73, Extra Deck del kit da 15 carte) e dopo la correzione del livello 2 della corsia 4. I numeri di questa pagina valgono per `main` prima di B5a.

## Richiesta a Gemini
In `docs/TASKS.md`, sezione «Interfaccia (Gemini)»: i pesi dell'esposizione in `scoring.ts`, più la conversione dei pesi salvati e un test Vitest.


## Implementazione (Gemini, branch `gemini/exposure-weights`)
- Pesi `exposure` in `PROFILES`: Bilanciato −0,5, Più negazioni −0,4, Controllo e floodgate −0,4, A prova di breaker −0,6, Risorse e follow-up −0,3, Linee sicure −1,0.
- Migrazione `loadChoice()` / `migrateLegacyWeights`: moltiplicazione per 12,5 se |exposure| < 0,2 e > 0.
- Test Vitest per la migrazione e per l'ordinamento di board con diversa esposizione.
- Testo di aiuto aggiornato in `METRICS`.
