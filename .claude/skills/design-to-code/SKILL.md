---
name: design-to-code
description: Come passare da un mockup fatto in Claude Design (artifact «Design» sul sistema di design DuelForge) al codice di apps/desktop/src, e come tenere allineati componenti, galleria e sistema di design con /design-sync.
---

# Dal mockup al codice

## Leggere il mockup
- Il brief dà il link all'artifact Design. Si legge con `Artifact` (`action: "read"`, `url`): l'HTML degli artboard, uno per tema. Si leggono anche i commenti dell'utente sull'artifact (`ArtifactComments`).
- Il mockup è costruito sul sistema di design DuelForge: i suoi colori e componenti corrispondono ai token di `apps/desktop/src/index.css` e ai componenti di `apps/desktop/src/components/`. Un colore che non è un token è un errore del mockup, non un colore da aggiungere.

## Mappare
| Nel mockup | Nel codice |
|---|---|
| Barra con comandi su una riga | `components/Toolbar.tsx` |
| Chip di filtro, pillole rimovibili | `components/FilterChip.tsx` |
| Pulsante a icona | `components/IconButton.tsx` |
| Pannello laterale | `components/SidePanel.tsx` |
| «?» con spiegazione | `components/HelpPopover.tsx` |
| Icone | `lucide-react` (set unico) |
| Testo piccolo | `text-xs` (12 px) o `text-caption` per i badge; mai sotto |
Un componente che non esiste si scrive in `components/`, entra nella galleria (`/galleria`, solo in modo mock, tutti gli stati: vuoto, caricamento, errore, dati) e in `apps/desktop/design/` come anteprima per il sistema di design.

## Verificare
Si segue `pr-interfaccia`: screenshot propri delle pagine toccate nei due temi, messi accanto agli artboard del mockup. Le differenze volute (dati veri, testi lunghi) si scrivono nella PR; le altre si correggono.

## Tenere allineato il sistema di design
- Ogni anteprima in `apps/desktop/design/<gruppo>/<componente>.html` comincia con `<!-- @dsCard group="<gruppo>" -->` e mostra il componente nei due temi.
- Le anteprime non si scrivono a mano: si aggiunge una scheda `DsCard` in `src/pages/Gallery.tsx` e si rigenera con `pnpm design -- --chromium /opt/pw-browsers/chromium` (nel container cloud). Lo script ricrea `design/`, `duelforge.css` e gli asset; con `--artifact <cartella>` prepara anche l'anteprima Artifact, con `--shots <cartella>` gli screenshot di ogni scheda. Il diff di `design/` va nella stessa PR del componente.
- Quando cambiano i token o un componente base, si lancia `/design-sync` verso il progetto «Sistema di design DuelForge»: carica per differenza, un componente alla volta, mai una sostituzione in blocco. L'utente approva il piano dei file prima della scrittura.
- I mockup nuovi si fanno sempre sul sistema di design aggiornato; un mockup fatto senza non si implementa.
