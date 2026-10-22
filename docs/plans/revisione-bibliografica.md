# Revisione bibliografica: pianificazione del primo turno in un gioco di carte a regole, con generalizzazione a carte mai viste

*Corsia 10, 22-23/10/2026, branch `claude/literature-review`. Per il proprietario e il coordinatore. Estende e corregge `ricerca-letteratura.md` (22/10), che resta il riferimento per le idee già adottate; qui non si ripetono le fonti già verificate lì (§8.3), se non per correggerle o per collocarle rispetto a fonti nuove.*

**Bozza in lavorazione.** Le sezioni si riempiono una alla volta e si pushano a ogni passo.

## 0. Sommario esecutivo
*(da scrivere alla fine: 15 righe, le 5 direzioni più promettenti con i lavori chiave)*

## 1. Domande, metodo e registro delle interrogazioni

### 1.1 Le due domande
1. Che cosa esiste di applicabile al nostro problema: pianificazione del primo turno in un gioco di carte enorme, guidato da regole (motore EDOPro con script Lua), in parte casuale, con generalizzazione a carte mai viste?
2. Quali tecniche conviene trasferire, e quali ipotesi di quei lavori reggono o si rompono in Yu-Gi-Oh!?

### 1.2 Cosa rende Yu-Gi-Oh! diverso (i criteri di trasferibilità)
Ogni fonte è giudicata contro queste sette proprietà del nostro problema. «Regge» o «si rompe» si riferisce a queste.
- **P1. Catene e finestre di risposta.** Ogni attivazione apre una finestra in cui entrambi i giocatori possono rispondere; la risoluzione va al contrario. Il turno non è una sequenza piatta di azioni.
- **P2. Fattore di ramificazione enorme e profondo.** Una linea di combo ha decine di decisioni (Evocazioni, bersagli, materiali, zone, sì/no), e molte sono scelte dentro un'altra scelta.
- **P3. Caso nel proprio turno.** Pescate, carte mandate dal Deck, monete e dadi: il turno è stocastico anche senza l'avversario.
- **P4. Carte mai viste.** Escono carte nuove ogni mese; il motore le sa giocare dallo script, e il cercatore non deve conoscerle per nome.
- **P5. Primo turno senza azioni dell'avversario, ma valutato contro il suo turno.** La ricerca è a un giocatore; il valore della board è una misura contro un avversario (turno di prova, livello 2).
- **P6. Nodi che costano millisecondi.** Ogni stato si genera rigiocando il motore; non esiste un modello compatto e invertibile.
- **P7. Obiettivo: la board più forte, non il cammino più corto.** Le linee che raggiungono un obiettivo si confrontano per forza della board; la lunghezza è solo spareggio.

### 1.3 Criteri di inclusione ed esclusione
*(da compilare)*

### 1.4 Dove si è cercato e con quali limiti
*(da compilare: motori raggiungibili, livello di lettura)*

### 1.5 Registro delle interrogazioni
*(tabella: data, motore, interrogazione, risultati guardati, fonti tenute)*

## 2. Risultati per gruppo
### 2.1 Yu-Gi-Oh! (gruppo 1)
### 2.2 IA per giochi di carte collezionabili (gruppo 2)
### 2.3 Apprendimento per rinforzo e self-play (gruppo 3)
### 2.4 Ricerca e pianificazione (gruppo 4)
### 2.5 Informazione nascosta e teoria dei giochi (gruppo 5)
### 2.6 Generalizzazione (gruppo 6)
### 2.7 Architetture ibride e rappresentazione (gruppo 7)
### 2.8 Aggiunte: pianificazione a un giocatore su sequenze lunghe, novità e larghezza, limiti ammissibili, riduzione per ordine parziale, Hearthstone/LOCM/Forge, Prismata

## 3. Tabella di tutti i lavori
*(riferimento, tipo, livello di lettura, trasferibilità)*

## 4. Raccomandazioni sul piano (A1b, voci B di `ricerca-piano-critico.md`)
*(ogni voce con un controllo misurabile)*

## 5. Lacune: dove la letteratura non c'è
*(opportunità di ricerca)*
