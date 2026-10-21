# Turno di prova contro un deck scelto (#44): proposta

*Da decidere con il proprietario. Scritta il 21/10/2026 da Claude Code.*

## Obiettivo
Oggi una board si misura contro un avversario generico: un kit di carte semplici che fanno una cosa sola (11 azioni tipiche) e 9 board breaker comuni. L'esposizione della linea considera le handtrap più giocate (Ash Blossom, Veiler e Impermanence, Nibiru, Droll, Maxx "C").

L'obiettivo è misurare la board contro il deck che si incontrerà davvero, scelto dall'utente.

## Tre livelli possibili

| Livello | Cosa cambia | Costo | Fedeltà |
|---|---|---|---|
| **1. Breaker e handtrap dell'avversario** | Resilienza ed esposizione si misurano con le carte del deck scelto | medio: backend 3-4 ore, un selettore nell'interfaccia | buona per "la mia board regge ai suoi breaker?" e "le sue handtrap fermano la mia linea?" |
| **2. Azioni dell'avversario** | Anche le interruzioni: il kit usa le giocate tipiche del deck scelto | alto | buona anche per "cosa ferma la mia board di quel deck" |
| **3. Partita simulata** | Il motore cerca la linea migliore dell'avversario contro la board | molto alto (minuti per board) | la più fedele |

### Livello 1 nel dettaglio (consigliato come primo passo)
- **Handtrap.** Si leggono dal motore sul deck avversario: è la funzione `hand_traps` che il Combo Lab usa già, applicata al suo deck. L'esposizione conta solo i punti in cui le sue handtrap fermano la linea. Ogni punto pesa quanto è probabile che l'avversario ne abbia almeno una in mano (copie nel deck, 5 o 6 carte pescate).
- **Board breaker.**
  - Sono le carte del deck avversario con il ruolo "board breaker", lo stesso riconoscimento della Fucina, e si giocano come i breaker del kit.
  - Ognuno pesa quanto è probabile averlo in mano.
  - La resilienza diventa "quanto resta della board contro i suoi breaker".
  - Se il deck non ne ha, si usano quelli standard.
- **Interruzioni e floodgate** restano misurati con il kit generico (livello 2 per cambiarli).
- **Interfaccia** (per Gemini):
  - nel Circolo Arcano, un selettore "Contro: generico / deck salvato";
  - nella scheda del punteggio, resilienza ed esposizione "contro X" accanto a quelle generiche.

### Livello 2 (dopo)
- Le azioni del kit vengono dal deck avversario. Si prendono le sue prime giocate tipiche dalle linee migliori che la ricerca trova per lui, contate come fa `duelforge-cli kit-mix`.
- Le carte del kit diventano le sue.
- Serve far giocare al motore le linee dell'avversario nel turno 2, con la board in campo e risposte a metà linea: è il lavoro più grosso.

## Decisioni per il proprietario
1. **Livello:** consiglio il 1 adesso e il 2 più avanti.
2. **Scelta del deck avversario:** tra i deck salvati in Bottega (consigliato, già importabili da `.ydk`) o da una lista di Master Duel Meta scaricata al momento.
3. **Punteggio:** il punteggio "contro X" si affianca a quello generico (consigliato) o lo sostituisce mentre l'avversario è scelto.
