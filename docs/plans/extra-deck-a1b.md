# La classe «Extra Deck» dei disaccordi di A1b

Corsia 4, ottavo task. La classe è la più grande dei disaccordi tra misura e livello 2 del passo 1 di A1b (`ricerca-esperimenti.md` §6, corsia 7): 19 coppie su 57.

## Ipotesi di partenza: non confermata
L'ipotesi era che il kit evocasse troppo poco dall'Extra Deck.
- **I dati.** Nelle linee vere le Evocazioni dall'Extra Deck sono 2,8 a linea: 573 in 203 linee, le prime 5 board di 41 deck. È il 17% delle giocate, il tipo più frequente. Il kit ne fa una. Il numero lo stampa ora `duelforge-cli kit-mix`.
- **Il prototipo.** Il kit evoca 3 volte, con mostri senza effetti e materiali esatti: Link Spider, Scrap Archfiend da Glow-Up Bulb e Bystial, un secondo Link Spider. Con pesi 0,33 ciascuna oppure 0,6/0,2/0,2, la classe resta a 18 coppie su 19. In più `search_regression` peggiora su Invoked.
  - Clown Crew Finale si usa una volta per turno. Nel kit distrugge il primo Link Spider. Con tre Evocazioni quella prima pesa meno, e Finale scende da 0,21 a 0,15.
  - I mostri aggiunti non hanno effetti: distruggerli non ferma nulla di più.
  - Il prototipo resta fuori da `main`.

## Come è fatta l'analisi
- **Le board.** Sono le 27 board delle 19 coppie, dalle fixture della corsia 7 (`tests/fixtures/research/a1b`, con le sue liste MDM).
- **Il livello 2.** Ogni board gioca `probe-line --vs <deck> --moves` contro i 19 avversari del giro, pesati con il popRank del 22/10. È `main` baf1d82, quindi con le #93 e #94: le verità di Megalith e Snake-Eye salgono di 0,05-0,13 rispetto alle fixture, e le coppie restano nello stesso ordine.
- **Il confronto.** Per ogni risposta delle due board si mettono a confronto:
  - la copertura media al livello 2 e la quota delle mosse fermate che sono Evocazioni dall'Extra Deck;
  - la copertura data dalla misura.

## Cosa si trova: due meccanismi, nessuno legato al numero di Evocazioni
### M1. Una rimozione su un mostro che l'avversario ha appena portato in campo vale molto più nelle linee vere (14 coppie su 19)
Nella board migliore c'è una risposta che toglie, distrugge o nega un mostro sul campo dell'avversario. Al livello 2 copre molto più che nella misura, e circa metà delle mosse che ferma sono Evocazioni dall'Extra Deck o mosse che vengono subito dopo:

| risposta (board migliore) | copertura livello 2 | copertura misura | Extra Deck tra le mosse fermate |
|---|---|---|---|
| Clown Crew Finale (Clown Crew) | 0,49-0,51 | 0,28-0,35 | 45% |
| The Fallen & The Virtuous (Resonators) | 0,50 | 0,35 | 45% |
| Red Dragon Archfiend's Chain (Resonators) | 0,45 | 0,23 | 1% |
| Spright Elf (Mitsurugi Yummy) | 0,44 | 0,35 | 61% |
| Elfnote Lucina (Snake-Eye) | 0,40 | 0,28 | 61% |
| S:P Little Knight (White Forest Azamina) | 0,39 | 0,13 | 56% |
| Invocation - "Sword" (Invoked) | 0,42 | 0,35 | 56% |

**Perché.** Nelle linee vere il mostro appena evocato, spesso dall'Extra Deck, è quello da cui dipende il resto della linea. Toglierlo ferma tutte le mosse dopo. Nel kit le sonde sono quasi indipendenti tra loro: dopo Link Spider c'è solo il suo effetto, poi le sonde che non dipendono da lui. L'Extra Deck c'entra perché è da lì che arrivano i mostri motore, non per quante volte si evoca.

### M2. La prima grande negazione copre tutto nella misura e circa tre quarti nelle linee vere (le stesse coppie, come rinforzo)
Mechaba, WANTED: Seeker of Sinful Spoils, Mitsurugi Great Purification e Red-Eyes Dark Dragoon hanno coperture molto diverse:
- nella misura dall'83% al 100%: il kit fa poche mosse, e una negazione al momento giusto le ferma quasi tutte;
- al livello 2 dal 56% al 74%: l'avversario va avanti con altre carte.

Una board con una sola grande negazione batte così, nella misura, una con più rimozioni piccole. Invoked è l'esempio: Mechaba più Oceanus contro Mechaba più Sword coperta.

### Il resto (5 coppie)
- Warrior Combo (3 coppie): la board peggiore non ferma niente al livello 2, verità 0. La misura le dà 0,26 per floodgate e campo che contro le linee vere non contano. La migliore vale 0,15. È un caso di «floodgate del kit», non di Extra Deck.
- Megalith (2 coppie): scarto di verità di 0,02 dopo la #93, dentro il rumore.

## Proposta
1. **La correzione vera di M1 e M2 è il kit dalle aperture vere** (`revisione-motore.md` §5.3.2):
   - **Aperture.** Le prime 6-8 mosse delle linee migliori dei deck più diffusi, calcolate una volta per aggiornamento del meta e salvate su disco.
   - **Misura.** Il turno di prova ne usa un campione di 3-4, pesato con il meta, al posto delle 13 sonde fisse. Così un'Evocazione dall'Extra Deck è seguita dai suoi effetti veri, e una negazione non copre più tutto.
   - **Sonde fisse.** Restano come misura economica (T1).
   - Lo strumento c'è già: è il livello 2 (`against_moves`, `rival_lines`). Servono il campione, il salvataggio delle aperture e la scelta di cosa fa la misura con più aperture: media, poi copertura per apertura.
2. **Prima, una prova economica senza toccare il kit.** Sulle fixture di A1b con `scripts/research/a1b_remeasure.py` più il livello 2, si confronta l'ordine della misura con quello di «misura + livello 2 su 3 aperture fisse»: Elfnote, Dracotail, Invoked, i primi tre del meta. Si vede quanto si guadagna prima di cambiare `probe.rs`.
3. **Da non fare**:
   - aggiungere al kit Evocazioni o carte (il §5.4 della revisione lo dice già, e il prototipo lo conferma);
   - ritoccare i pesi delle sonde per questa classe.
