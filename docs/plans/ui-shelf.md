# Piano: Bottega (scatole e mensole) — Passo 5

Riferimento: `docs/plans/ui-revisione-2.md`, Passo 5.
Branch: `gemini/ui-shelf`.
Agente: Gemini / Antigravity.

## Obiettivo

Rendere visivamente realistiche, tattili e immersive le scatole (deck box) e le mensole di legno nella sezione **Bottega**, mantenendo l'architettura leggera in CSS 3D senza Three.js (zero impatto negativo sulle prestazioni, zero complessità superflua e bundle snello tramite lazy loading).

## Requisiti e Soluzioni

### 1. Materiali e Texture CC0
Scarichiamo e ottimizziamo texture ad alta fedeltà in formato WebP (risoluzione 512×512, qualità ~82%, peso < 100 KB l'una, ben sotto la soglia massima di 150 KB) con licenza CC0 (Public Domain Dedication) da ambientCG:
- **Pelle** (`leather.webp`): ambientCG Leather037. Superficie a grana naturale con cuciture a contrasto.
- **Plastica Opaca** (`matte.webp`): ambientCG Plastic010. Finitura satinata morbida con sottile riflesso diffuso.
- **Metallo Spazzolato** (`metal.webp`): ambientCG Metal009. Riflesso speculare con grana lineare orizzontale e rivetti metallici.
- **Cartoncino** (`cardboard.webp`): ambientCG Cardboard001. Fibra naturale pressata con bordi leggermente consumati.
- **Legno** (`wood.webp`): ambientCG Wood049. Venature calde per scatole in legno e per il ripiano delle mensole.
- Documento di licenza ed attribuzione CC0 in `apps/desktop/src/assets/textures/LICENSE.md`.
- Espansione del tipo `Material` in `apps/desktop/src/lib/boxes.ts` con `'cardboard'` («Cartoncino»).

### 2. Scatole 3D Meno Spigolose e Più Realistiche (`DeckBox3D.tsx`)
- **Smussatura bordi**: `border-radius` sulle facce, raccordi perimetrali ed evidenziazione dei bordi con gradienti di spessore.
- **Illuminazione dinamica e riflesso speculare**:
  - Tracciamento della posizione del puntatore su `onPointerMove` calcolato come coordinate normalizzate `px, py` (`-0.5` .. `+0.5`).
  - Variabili CSS `--light-x` e `--light-y` (in percentuale o gradi) applicate all'elemento `.dbox`.
  - Strato di riflesso speculare (`.dbox-specular`) che segue la sorgente luminosa virtuale.
- **Coperchio e cerniera visibile**:
  - Componente cerniera metallica posteriore (`.dbox-hinge`) con perno scanalato in corrispondenza dell'asse di rotazione del coperchio.
- **Ombra di contatto morbida**:
  - Ombra ellittica su `.dbox-shadow` con decadimento radiale e blur realistico che si espande e attenua quando la scatola si solleva.

### 3. Movimento, Estrazione e Accessibilità
- **Hover**: Sollevamento delicato della scatola (`--lift: -12px`) con le carte vetrina (`.dbox-card`) che sporgono maggiormente verso l'alto a ventaglio accentuato.
- **Clic ed estrazione**: Animazione breve di estrazione (`data-extracting` prima della transizione o apertura) con feedback sonoro appropriato.
- **Accessibilità (`prefers-reduced-motion`)**:
  - Disattivazione dell'animazione di oscillazione automatica (`@keyframes dbox-sway`).
  - Transizioni immediate senza salti o distorsioni prospettiche.
  - Piena operabilità da tastiera (`tabIndex`, `onKeyDown` per Enter/Space, focus ring visibile).

### 4. Mensole di Legno e Sfondo della Bottega (`Shelf.tsx`, `index.css`)
- **Ripiano in legno**: Texture CC0 `wood.webp` applicata al ripiano `.shelf-plank` con bordo frontale arrotondato (beveled edge).
- **Staffe metalliche**: Staffe di supporto in metallo rifinito con viti/rivetti (`.shelf-bracket`) agli estremi del ripiano.
- **Ombra della mensola**: Ombra proiettata morbida sulla parete retrostante.
- **Sfondo della parete**: Tonalità sobria ed elegante che funziona perfettamente sia nel tema scuro (`forge`) sia nel tema chiaro (`light`).

### 5. Ottimizzazione Bundle e Code Splitting (`App.tsx`)
- Caricamento differito di `Shelf` con `React.lazy` e `Suspense`.
- Nessun peso delle texture o del codice specifico della Bottega al caricamento iniziale delle altre pagine (Fucina, Laboratorio, ecc.).

## File Coinvolti
- `docs/TASKS.md` (lock e avanzamento)
- `docs/plans/ui-shelf.md` (questo piano)
- `apps/desktop/src/assets/textures/leather.webp`
- `apps/desktop/src/assets/textures/matte.webp`
- `apps/desktop/src/assets/textures/metal.webp`
- `apps/desktop/src/assets/textures/cardboard.webp`
- `apps/desktop/src/assets/textures/wood.webp`
- `apps/desktop/src/assets/textures/LICENSE.md`
- `apps/desktop/src/lib/boxes.ts`
- `apps/desktop/src/components/DeckBox3D.tsx`
- `apps/desktop/src/components/BoxEditor.tsx`
- `apps/desktop/src/pages/Shelf.tsx`
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/index.css`

## Controlli e Verifica
- `pnpm lint`
- `pnpm typecheck`
- `pnpm test`
- `pnpm build` (con verifica del chunk generato per la Bottega e confronto dimensioni bundle)
- Rispetto di tutte le regole di `AGENTS.md` (no `as any`, id di richiesta, pulizia timer/listener).
