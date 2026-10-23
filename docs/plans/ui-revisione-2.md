# Revisione dell'interfaccia, seconda parte: spazi, carte, filtri, Bottega, effetti

*Piano scritto il 22/10/2026 dal coordinatore (Claude Code), dalle prove dell'app vera fatte dal proprietario. Lo esegue Gemini, un passo per PR. La prima revisione (schermi 1280×800 e tema chiaro) è in `docs/plans/ui-review.md`: questa non la ripete, la continua.*

## Cosa ha visto il proprietario
1. **Collegamenti (Laboratorio).**
   - Il cerchio è piccolo dentro un riquadro grande.
   - Nodi e nomi sono minuscoli.
   - Mancano le immagini delle carte, che dicono più del nome.
2. **Barra dei filtri dei Collegamenti.** Due righe di chip con conteggi, alcune a zero («Materiale di (0)»), scritte piccole, spazi distribuiti male.
3. **Fucina.**
   - Manca un posto fisso dove vedere la carta su cui si passa o che si è cliccata, con le sue informazioni.
   - L'anteprima di oggi è una finestrella che compare accanto al cursore.
4. **Ricerca del catalogo.** Campo, segnalibro, «Filtri» e quattro chip rapidi sono affollati e poco ordinati.
5. **In tutta l'app:**
   - spazi poco usati e scritte troppo piccole o inutili;
   - zone affollate di opzioni;
   - testi d'aiuto fissi sullo schermo che servono una volta sola;
   - poche icone che spieghino un concetto senza parole.
6. **Bottega.** Le scatole dei deck sono spigolose. Le mensole sono piatte. Si vuole qualcosa di più realistico e d'impatto.
7. **Effetti e suoni.** Ne vuole di più, carini e non invadenti. Ogni effetto si deve poter spegnere, anche uno per uno, nelle Impostazioni.

## Principi (valgono per ogni passo)
- **Prima il contenuto, poi i comandi.**
  - Le carte e i numeri occupano lo spazio grande.
  - Filtri e opzioni stanno in una barra sola e compatta.
  - Quello che si usa poco va in un menu o in un popover.
- **Testo leggibile.** Corpo 13-14 px. Il minimo è 12 px, e 11 px solo per badge e numeri dentro un chip. I titoli delle carte sono in inglese e restano come sono.
- **Icone con nome.**
  - Un'icona sostituisce una parola solo quando il significato è chiaro.
  - Ogni pulsante con sola icona ha `aria-label` e tooltip (`lib/tooltip.ts`).
  - Si usa un solo set di icone in tutta l'app.
- **Niente testo fisso che serve una volta.** Le istruzioni («Trascina le carte · clic destro…», «Tocca o passa sopra una carta…») vanno in:
  - un pulsante «?» con popover;
  - un suggerimento al primo uso, salvato come già visto in `localStorage`;
  - un tooltip.
- **Niente opzioni vuote.** Un filtro con conteggio 0 non si mostra, oppure si mostra spento in coda.
- **Densità adattiva.** Ogni pagina deve funzionare a 1280×800 e sfruttare 1920×1080. I pannelli laterali si chiudono, oppure diventano sovrapposti sotto i 1440 px di larghezza.
- **Movimento sobrio.**
  - Durate di 120-250 ms, curve morbide.
  - Con `prefers-reduced-motion` restano solo le dissolvenze.
  - Ogni effetto nuovo ha il suo interruttore (passo 6).
- **Due temi.** Ogni modifica si prova nel tema «Fucina» (scuro) e in «Pergamena» (chiaro). I colori vengono solo dai token di `src/index.css`.

## Passi
Un branch e una PR per passo, in quest'ordine. Il passo 0 è la base degli altri.

### Passo 0. Fondamenta (`gemini/ui-foundations`)
- **Scala tipografica e spaziature come token** in `src/index.css`: corpo, piccolo, didascalia, titoli; spaziature 4/8/12/16/24. Si sostituiscono le dimensioni fisse più piccole di 12 px dove non sono badge.
- **Set di icone unico.** Proposta: `lucide-react` (licenza ISC, importa solo le icone usate). Le icone di sezione di `Emblems.tsx` restano, perché sono l'identità dell'app. Le icone di Lucide servono per azioni e filtri.
- **Componenti di base** per non ripetere il codice:
  - `Toolbar`: una riga che si chiude in «Filtri» quando lo spazio non basta;
  - `FilterChip`: icona, etichetta e conteggio;
  - `IconButton`: con tooltip e `aria-label` obbligatori;
  - `SidePanel`: richiudibile, ricorda se è aperto;
  - `HelpPopover`: il pulsante «?».
- **Strumento di screenshot** per confrontare prima e dopo: uno script con Playwright (`apps/desktop/scripts/screens.mjs`, solo devDependency) con `VITE_MOCK=1`.
  - Fotografa ogni pagina a 1280×800 e 1920×1080, in tutti e due i temi.
  - Le immagini restano fuori dal repository. Nella PR vanno solo le più utili.
- *Completato il 22/10/2026 (Gemini):* aggiunti i token di tipografia e spaziatura in `src/index.css` con classi di utilità; installato `lucide-react` (ISC) per il set unico di icone; creati i componenti base accessibili `IconButton`, `FilterChip`, `HelpPopover`, `SidePanel` e `Toolbar` con logica pura di collasso testata in Vitest (`Toolbar.test.ts`); creato lo script `screens.mjs` con Playwright e axe.

### Passo 1. Collegamenti (`gemini/ui-links`)
File: `src/components/lab/links/*`.
- **Il cerchio usa tutto il riquadro.**
  - Raggio e `viewBox` dalla dimensione reale del contenitore (`ResizeObserver`), non fissi (oggi `R = 230` e `viewBox` 740).
  - Con 40-60 carte i nodi non si sovrappongono. Se non c'è posto, i nodi meno importanti si rimpiccioliscono.
- **Nodi con l'immagine della carta.**
  - L'immagine ritagliata (`cardImage(id, "cropped")`) in una cornice tonda o a scudo, di 40-64 px secondo l'importanza della carta (`key`).
  - Il bordo ha il colore del tipo di carta.
  - Il numero di copie è un piccolo badge.
  - Il nome sta accanto, a 12-13 px, troncato con il nome intero nel tooltip.
  - Al passaggio del mouse la carta si ingrandisce.
- **Collegamenti più leggibili.**
  - Spessore e colore per tipo, come oggi.
  - Al passaggio o al clic su una carta, i suoi collegamenti si accendono e gli altri si attenuano.
  - Una freccia animata leggera sui collegamenti scelti, spegnibile.
- **Barra dei filtri su una sola riga.**
  - I tipi di collegamento diventano chip con icona e colore, con il conteggio come badge. Quelli a zero non si mostrano.
  - La zona di partenza diventa un menu a tendina con icone.
  - «Solo usati nelle combo migliori» diventa un interruttore con icona.
  - La ricerca di una carta resta.
  - «Cerchio / Colonne» diventa un selettore a icone.
  - Sotto i 1440 px tutto si raccoglie in «Filtri».
- **Pannello destro utile anche senza selezione.** Al posto di «Nessuna carta selezionata»:
  - la legenda dei colori;
  - le carte chiave del deck;
  - un suggerimento breve.
  Con una carta scelta: immagine grande, probabilità, collegamenti in entrata e in uscita.
- **La probabilità come stima** (seguito della PR #61, corsia 6). Quando `measured` è vero si scrive «≈ 45%», con un tooltip che spiega che è misurata sulle mani cercate (±5%).
  - Si fa in questo passo, appena la #61 è in `main`.
  - Non serve cambiare `api.ts`: i campi ci sono già.
- *Completato il 22/10/2026 (Gemini):* resa dinamica la geometria del cerchio con `ResizeObserver` e calcolo puro in `layout.ts` (testato con 40-60 carte senza sovrapposizione); nodi con artwork ritagliata (40-64 px), bordo colorato per tipo di carta Yu-Gi-Oh, badge copie, zoom al passaggio ed effetto freccia animata leggera; barra filtri monoriga con `Toolbar`, `FilterChip` e icone (collasso in «Filtri» < 1440 px); pannello destro arricchito con legenda colori e classifica interattiva carte chiave senza selezione, e stima esplicita «≈ X%» quando `measured`.

### Passo 2. Fucina: l'ispettore della carta (`gemini/ui-forge-inspector`)
File: `src/pages/DeckBuilder.tsx`, un componente nuovo `src/components/CardInspector.tsx`, `CardTile.tsx` e `Catalog.tsx` solo per gli eventi.
- **Pannello a sinistra**, richiudibile, largo 280-320 px. Mostra la carta:
  - su cui passa il mouse, nel catalogo o nel deck;
  - oppure quella cliccata una volta, che resta fissata finché non se ne clicca un'altra o si preme Esc.
- **Contenuto del pannello:**
  - immagine grande;
  - nome, tipo, Attributo, Livello o Rango o Link, ATK e DEF, testo;
  - stato in banlist nei tre formati, con icone;
  - rarità in Master Duel;
  - copie nel deck;
  - ruoli (starter, estensore…).
  Azioni rapide: +1 al Main o all'Extra, +1 al Side, preferita, apri nel Visore.
- **Clic:**
  - un clic fissa la carta nell'ispettore;
  - il doppio clic fa quello che fa oggi (la lente);
  - clic destro e trascinamento restano come sono.
- **L'anteprima fluttuante di oggi** (`CardPreview`, impostazione «Anteprima») diventa un'opzione: con l'ispettore aperto non compare. Si propone di spegnerla per chi ha l'ispettore.
- **Sotto i 1440 px** l'ispettore si apre sopra il deck come pannello sovrapposto, con un pulsante, invece di stringere il deck.
- Le istruzioni fisse («Trascina le carte · clic destro…» in alto e in fondo al catalogo) vanno nel pulsante «?».
- *Completato il 22/10/2026 (Gemini):* creato componente accessibile `CardInspector` (artwork grande, statistiche mostro, testo normale e pendulum, rarità MD, banlist nei 3 formati, ruoli, copie nel deck); integrato `SidePanel` richiudibile a sinistra (300 px, overlay reattivo < 1440 px con pulsante dedicato e auto-apertura al clic, soppressione anteprima fluttuante `CardPreview`); interazione con clic singolo (fissa carta, sblocco Esc) e passaggio mouse (anteprima); azioni rapide (+1 Main/Extra, +1 Side, preferita, visore zoom); istruzioni fisse sostituite da `HelpPopover`; 8 test Vitest unitari in `CardInspector.test.tsx`.

### Passo 3. Ricerca e filtri del catalogo (`gemini/ui-catalog-search`)
File: `src/components/Catalog.tsx`, `FilterPanel.tsx`.
- **Una riga sola:**
  - campo di ricerca largo;
  - accanto, un pulsante «Filtri» con il numero di filtri attivi.
  Le ricerche salvate (segnalibro) diventano un menu a tendina dentro il campo.
- **I chip rapidi** (Handtrap, Staple, Board breaker, Preferite) diventano interruttori a icona dentro la stessa riga, oppure la prima sezione del pannello «Filtri».
- **I filtri attivi** compaiono come pillole rimovibili sotto la ricerca, solo quando ci sono.
- **La scelta della vista** (carte grandi, medie, piccole, lista) diventa un solo pulsante con menu.
- Il conteggio «14.388 carte» resta, più discreto.
- *Completato il 23/10/2026 (Gemini):* barra monoriga compatta nel catalogo con campo di ricerca largo, ricerche salvate integrate come menu a tendina dentro il campo, interruttori a icona nella riga per i 4 filtri rapidi (Handtrap, Staple, Board breaker, Preferite) con stato `aria-pressed`, selettore vista a pulsante singolo con menu popover (carte grandi, medie, piccole, elenco), pulsante Filtri con badge conteggio filtri attivi, pillole rimovibili mostrate sotto solo quando ci sono filtri attivi o errori, conteggio carte discreto e 7 test Vitest in `Catalog.test.tsx`.

### Passo 4. Passata su tutte le sezioni (`gemini/ui-polish`)
- **Per ogni pagina** (Fucina, Bottega, Laboratorio con tutte le schede, Circolo Arcano, Banditore, Impostazioni) una tabella nel piano: cosa si toglie, cosa si sposta, cosa diventa icona.
  - Scritte sotto i 12 px.
  - Testi fissi d'aiuto.
  - Etichette ripetute.
  - Barre con più di 6-7 comandi visibili.
- **Le didascalie decorative** («Ispirato a…») restano solo dove non rubano spazio.
- **Le schede del Laboratorio** si possono raggruppare con icone; quelle meno usate vanno in un menu «Altro» se non ci stanno.
- *Completato il 23/10/2026 (Gemini):* piano per-pagina con tabelle in `docs/plans/ui-polish.md`; normalizzata la scala tipografica (nessun testo sotto 12 px, badge minimi a 11 px); rimossi testi d'aiuto fissi e prolissi in favore di tooltip e popover; riorganizzate le barre con icone Lucide (Undo, Redo, Dices, Search, Plus, Scale, History, Boxes, ArrowLeftRight, Play, Sparkles); didascalia «Ispirato a...» reattiva (`hidden xl:block`); schede del Laboratorio arricchite con icone esplicative e menu a tendina «Altro ▾» per schermi compatti.

### Passo 5. Bottega: scatole e mensole (`gemini/ui-shelf`)
File: `src/components/DeckBox3D.tsx`, `src/pages/Shelf.tsx`, `src/lib/boxes.ts`, texture nuove in `src/assets/textures/`.
- **Scatole meno spigolose.** Restano in CSS 3D, che è leggero e già funziona:
  - spigoli smussati, con facce intermedie sugli angoli oppure `border-radius` sulle facce e uno smusso ai bordi;
  - luce: un gradiente che segue l'inclinazione già presente e un riflesso speculare che si sposta con il puntatore;
  - ombra di contatto morbida sotto la scatola;
  - coperchio con cerniera visibile.
- **Materiali con texture vere**, in licenza CC0 da ambientCG o Poly Haven: pelle, plastica opaca, metallo spazzolato, cartoncino.
  - Formato webp, 512-1024 px, al massimo circa 150 KB l'una.
  - Le licenze vanno in `src/assets/textures/LICENSE.md`.
- **Mensole di legno.**
  - Texture di legno CC0, bordo frontale smussato, staffe metalliche, ombra sotto le scatole.
  - Una parete di fondo leggera, che si adatta al tema chiaro.
- **Movimento.** Al passaggio la scatola si solleva e le carte vetrina escono un po'. Al clic, breve animazione di estrazione prima di aprire il deck.
- **Peso.** La pagina della Bottega si carica a parte (`React.lazy`), perché texture e 3D non pesano sull'avvio.
- **Alternativa non consigliata per ora:** three.js con `@react-three/fiber`. È più realistica, ma aggiunge circa 600 KB e complessità. Da riconsiderare solo se il CSS 3D non basta.

### Passo 6. Effetti visivi e suoni, ognuno spegnibile (`gemini/ui-effects`)
File: `src/lib/settings.ts`, `src/lib/sound.ts`, `src/lib/sparks.ts`, `src/pages/Settings.tsx`, e i punti dove gli effetti si usano.
- **Impostazioni.** Ogni voce ha un interruttore e i valori predefiniti sono sobri.
  - *Effetti visivi:*
    - braci di sfondo (`ambient`, c'è già);
    - effetti sulle carte (`cardEffects`, c'è già);
    - transizioni tra pagine;
    - animazioni di liste e risultati;
    - effetti della Bottega;
    - effetti della mappa dei collegamenti.
  - *Suoni:*
    - generale (c'è già);
    - interfaccia (passaggio, clic, schede);
    - carte (aggiungi, togli, mescola);
    - eventi (ricerca finita, deck legale, errore).
    Ogni gruppo ha volume e interruttore.
  - Un pulsante «Prova» accanto a ogni suono.
  - I campi nuovi in `Settings` hanno valori predefiniti, così le impostazioni salvate prima continuano a funzionare. Questo va coperto da un test.
- **Effetti visivi proposti, tutti brevi:**
  - dissolvenza o scorrimento tra pagine;
  - comparsa scaglionata delle board trovate;
  - la carta che «vola» nella zona del deck quando si aggiunge;
  - impulso lungo i collegamenti scelti;
  - scintille quando un deck diventa legale;
  - scatola che si solleva nella Bottega.
- **Suoni.** Restano sintetizzati con Web Audio (`lib/sound.ts`): niente file audio, come oggi. Nuovi suoni:
  - carta posata (un «tump» di carta);
  - carta tolta;
  - mescolata (mano di prova);
  - scatola presa e posata sulla mensola (legno);
  - nodo scelto nella mappa;
  - ricerca finita (campanella breve).
  Si possono progettare con ZzFX (licenza MIT, circa 1 KB, un suono è un array di numeri) o a mano come quelli che ci sono. Se serve un suono campionato, solo CC0 (Kenney), al massimo circa 30 KB l'uno.

## Risorse (tutte compatibili con la licenza AGPL del progetto)
- **Icone:** `lucide-react` (ISC).
  - Per icone più evocative di gioco (incudine, spada, scudo, pergamena) c'è game-icons.net (CC BY 3.0). Richiede l'attribuzione: si scrivono i crediti in `docs/CREDITS.md`.
- **Animazioni:**
  - prima CSS e Web Animations API;
  - `motion` (MIT, l'ex framer-motion) solo se serve per transizioni di layout;
  - `@formkit/auto-animate` (MIT) per liste che cambiano.
- **Particelle:** si usano `src/lib/sparks.ts` ed `Embers.tsx`, che ci sono già. Eventualmente `canvas-confetti` (ISC), con parsimonia.
- **Suoni:**
  - ZzFX (MIT): https://github.com/KilledByAPixel/ZzFX, con l'editor in https://killedbyapixel.github.io/ZzFX/;
  - jsfxr (Unlicense), per progettare suoni retro;
  - Kenney (CC0), https://kenney.nl/assets, pacchetti «Interface Sounds», «UI Audio», «Casino Audio» (carte), «Impact Sounds»;
  - freesound.org, solo con il filtro CC0.
- **Texture:** ambientCG (CC0, https://ambientcg.com) e Poly Haven (CC0, https://polyhaven.com/textures): legno, pelle, metallo, carta.
- **Strumenti di verifica**, solo devDependencies:
  - Playwright (Apache-2.0) per gli screenshot;
  - `@axe-core/playwright` (MPL-2.0) per l'accessibilità: contrasto, etichette, tastiera.
- **Mai:** immagini, suoni o testi Konami nel repository. Le immagini delle carte si scaricano a runtime, come oggi.

## Controlli per ogni PR
- `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm typecheck`, `pnpm test` e `pnpm build`, tutti a zero errori.
- Test nuovi per la logica pura aggiunta:
  - calcolo delle posizioni del cerchio;
  - valori predefiniti delle impostazioni;
  - raccolta dei filtri in «Filtri».
- Screenshot prima e dopo, a 1280×800 e 1920×1080, nei due temi, con il backend finto. Nella PR i più significativi.
- **Accessibilità:**
  - tutto si usa da tastiera;
  - il fuoco è visibile;
  - le icone hanno un nome;
  - il contrasto è AA nei due temi (axe).
- `prefers-reduced-motion` rispettato.
- Ogni effetto nuovo ha il suo interruttore.
- Il peso del bundle prima e dopo, dall'uscita di `pnpm build`. Ogni dipendenza nuova va motivata nella PR, con la sua licenza.
- Il protocollo dell'interfaccia di `AGENTS.md`: id di richiesta, errori in italiano, effetti React puliti, dati del contratto.
- `src/lib/api.ts` e `src/lib/mock.ts` non si toccano finché li tiene la corsia 4 (D12). Questi passi non dovrebbero averne bisogno; se servisse un dato nuovo, si chiede a Claude in `docs/TASKS.md`.
- Le prove nell'app vera (`pnpm tauri dev`) le fa il proprietario a fine passo: la PR dice cosa provare.
