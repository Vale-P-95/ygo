# Classifica per interazione a fine turno (24/10/2026)

Richiesta di Valerio: partendo da una mano (o da una carta), quali linee lasciano più interazione a fine turno (negazioni, effetti rapidi da mano, campo e Cimitero, handtrap, floodgate, risorse), senza pensare al turno dell'avversario.

## Cosa c'era già
- La ricerca trova le board di fine turno; il turno di prova le misura con un avversario generico (mosse campione), che serve a sapere se un effetto si attiva davvero e cosa ferma.
- `Measure` ha già ogni interruzione con tipo (`Effect`) e posto, le handtrap in mano, i floodgate, le carte usabili nel turno avversario (`offered`); l'app ha i profili «Più negazioni», «Controllo e floodgate», «Risorse e follow-up».
- Mancavano una classifica fatta solo su questo e il riepilogo per tipo.

## Decisione (CLI, `combos --interaction`)
- **Cosa**: le board si ordinano per `campo + mano + floodgate` (tutto in interruzioni, pesi 1), senza breaker, turno dopo ed esposizione alle handtrap; per ogni board una riga con negazioni, rimozioni e altro, da campo, Cimitero, bandite e mano, carte usabili nel turno avversario, mosse bloccate dai floodgate, carte rimaste in mano e nel Cimitero. Nel JSON, `interaction` per ogni board.
- **Conteggio**: un'interruzione conta i suoi usi (almeno uno); quella che ferma qualcosa solo da sola (valore 0 con le altre: Mechaba che manda l'ultima carta della mano) non conta e si mostra a parte («only alone»).
- **Perché pesi 1**: è la domanda di Valerio, quanta interazione c'è, non il punteggio bilanciato (che pesa la mano 0,9 e i floodgate 0,4).
- **Limiti**: con `--staged` il taglio alla misura completa e la variante tenuta seguono ancora il valore solito; con `--card` (più mani) non c'è misura, quindi niente classifica per interazione.
- **Misura e ricerca invariate**: cambia solo l'ordine stampato dalla CLI, la prova equa sulle 406 board non serve.

Esempio (Invoked, mano Aleister, Crowley, Ash, Meltdown, Droll; 3000 nodi): con il valore solito vince Mechaba + Endymion (1,12, Caliga 0,92); con `--interaction` vince Invoked Caliga (1,82: Ash in mano e 4 tipi di mosse bloccate), Mechaba scende a 1,17.

## Prossimi passi
- ~~Interfaccia~~ fatta il 24/10: il conteggio sta in `ygo-solver/src/interaction.rs` (la CLI lo usa da lì), `ScoreView.interaction` ↔ `ComboScore.interaction`, profilo «Interazione a fine turno» in `scoring.ts` (pesi 1 su interruzioni, handtrap, floodgate; 0 sul resto) e sezione nel dettaglio della board (`InteractionSection`). Senza mockup di Valerio: stile delle sezioni esistenti. Da provare nell'app vera.
- `--card`: misurare la board migliore di ogni mano e mediare l'interazione.
- Il passo successivo, con la ricerca che insegue l'interazione e la statistica da una carta: `docs/plans/linee-interazione.md`.
