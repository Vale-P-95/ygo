# Piano: Guida di benvenuto e primi passi (`gemini/onboarding-guide`)

## Obiettivo
Accompagnare chi apre DuelForge per la prima volta lungo il flusso naturale dell'applicazione — **scaricare i dati → costruire o importare un deck → riconoscere i ruoli e misurare le probabilità → cercare e mettere alla prova le combo** — senza appesantire l'interfaccia per chi già la usa.

## Componenti e modifiche

1. **`apps/desktop/src/components/WelcomeModal.tsx` (nuovo)**
   - Finestra modale in stile forgia/pergamena che si apre automaticamente alla prima esecuzione (`localStorage.getItem("duelforge.onboardingSeen") !== "1"`) oppure su richiesta dal pulsante **«Guida rapida»** nella barra laterale.
   - Articolata in 4 passi navigabili (con indicatori di passo, icone `EmblemIcon` colorate con l'accento di ciascuna sezione e scorciatoie pratiche del mouse/tastiera):
     1. **Accendi la fucina (Dati e motore)**: cosa viene scaricato (carte da YGOPRODeck, script e database del motore da Project Ignis, ~80 MB una tantum) e perché le immagini vengono scaricate solo al bisogno.
     2. **La Fucina e La Bottega (Costruisci e organizza)**: ricerca con filtri rapidi (`format:md`, Handtrap, Staple), importazione `.ydk` / `ydke://`, mano di prova, scorciatoie (`clic destro` per aggiungere/togliere, `Maiusc+clic destro` per il Side Deck, `doppio clic` per la lente) e scaffale 3D.
     3. **Il Laboratorio (Ruoli, probabilità e partite)**: riconoscimento automatico dei ruoli col motore (`✦ Riconosci`), calcolatore ipergeometrico, simulatore Monte Carlo, ottimizzatore e registro partite.
     4. **Il Circolo Arcano (Ricerca delle combo e turno di prova)**: come il motore esplora da solo il primo turno da una mano precisa o da una carta, i vincoli (`lucchetto` in mano, `clic destro` sul deck per vietare una carta, `Obiettivo` sul campo) e come il turno di prova classifica le board finali.
   - Pulsanti per passare direttamente alla sezione del passo corrente o chiudere la guida (salvando `duelforge.onboardingSeen = "1"`).

2. **`apps/desktop/src/pages/Overview.tsx`**
   - Sostituisce il blocco singolo «Entra nella Fucina» con il percorso interattivo in 4 passi **«Primi passi nell'officina»**, che legge i dati reali (`status` e `api.listDecks()`):
     - Mostra lo stato di ciascun passo (`✓ Pronto` per i dati e il motore, `✓ N deck salvati` quando almeno un deck è presente).
     - Offre per ogni passo una spiegazione sintetica e il pulsante per aprire subito la sezione corrispondente (`La Fucina`, `La Bottega`, `Il Laboratorio`, `Il Circolo Arcano`), oltre a un bottone «Apri la guida completa» che apre `WelcomeModal`.

3. **`apps/desktop/src/components/Sidebar.tsx` e `apps/desktop/src/App.tsx`**
   - Aggiunge in fondo alla `Sidebar` (sia in modalità estesa sia in modalità compatta a icone) il pulsante **«Guida rapida»** (`?`), per richiamare `WelcomeModal` da qualunque sezione dell'app.
