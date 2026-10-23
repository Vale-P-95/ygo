import { useEffect, useState, type ReactNode } from "react";
import { Boxes, Dices, Filter, Plus, Redo2, Search, Sparkles, Trash2, Undo2 } from "lucide-react";
import { CardTile } from "../components/CardTile";
import { BarChart, FlaskMeter, Legend } from "../components/charts";
import { DeckBox3D } from "../components/DeckBox3D";
import { ChartTooltip } from "../components/Overlays";
import { SectionHeader } from "../components/SectionHeader";
import { FilterChip, HelpPopover, IconButton, SidePanel, Toolbar, type ToolbarItem } from "../components/ui";
import { api, type Card } from "../lib/api";
import { MATERIALS, SHAPES, type Look } from "../lib/boxes";
import { sections, type SectionId } from "../sections";

/*
 * Component gallery, reachable only in mock mode (#/galleria). Every card shows
 * one component or token group in both themes side by side, in all the states
 * the app uses (empty, loading, error, data). It is the source of the design
 * system previews: scripts/design-export.mjs saves each [data-ds-card] as a
 * standalone page in apps/desktop/design/ for /design-sync.
 */

type Theme = "forge" | "light";
const THEMES: { id: Theme; name: string }[] = [
  { id: "forge", name: "Fucina (scuro)" },
  { id: "light", name: "Pergamena (chiaro)" },
];

/** Cards from the mock catalog, one per rarity and frame. */
const SAMPLE_CARDS = [81674782, 57554545, 57554544, 98159737, 10045474, 27548199];

/** One design-system card: the same content rendered once per theme. */
function DsCard({ id, group, name, note, accent, children }: { id: string; group: string; name: string; note?: string; accent: SectionId; children: ReactNode }) {
  return (
    <section data-ds-card={id} data-ds-group={group} data-ds-name={name} className="flex flex-col gap-2">
      <div className="flex items-baseline gap-3 px-1">
        <h2 className="text-title text-text">{name}</h2>
        <span className="eyebrow">{group}</span>
        {note && <span className="text-small text-faint">{note}</span>}
      </div>
      <div className="ds-pair grid grid-cols-2 overflow-hidden rounded-xl border border-border">
        {THEMES.map((t) => (
          <div key={t.id} data-theme={t.id} className="min-w-0" style={{ background: "var(--df-bg)", color: "var(--df-text)" }}>
            <div data-section={accent} className="flex h-full flex-col gap-3 p-5">
              <p className="eyebrow">{t.name}</p>
              {children}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-caption text-faint">{label}</span>
      <div className="flex flex-wrap items-center gap-2">{children}</div>
    </div>
  );
}

function Swatch({ token, label }: { token: string; label?: string }) {
  return (
    <div className="flex w-28 flex-col gap-1">
      <div className="h-10 rounded-lg border border-border" style={{ background: `var(${token})` }} />
      <span className="text-caption text-muted">{label ?? token.replace(/^--(df-)?/, "")}</span>
    </div>
  );
}

function Spinner({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-2 text-xs text-muted">
      <div className="h-5 w-5 animate-spin rounded-full border-2 border-sec border-t-transparent" />
      {label}
    </div>
  );
}

function ErrorBox({ children }: { children: ReactNode }) {
  return <div className="rounded-xl border border-danger/40 bg-danger/10 p-3 text-xs text-danger">{children}</div>;
}

const SURFACES = ["--df-bg", "--df-bg-2", "--df-surface", "--df-surface-2", "--df-surface-3", "--df-border", "--df-border-strong"];
const TEXTS = ["--df-text", "--df-muted", "--df-faint", "--df-gold", "--df-ember"];
const STATES = ["--df-ok", "--df-warn", "--df-danger"];
const ROLES = ["starter", "handtrap", "extender", "breaker", "brick", "custom-1", "custom-2", "custom-3"];
const METRICS = ["interruptions", "hand", "floodgates", "resilience", "followup", "resources"];
const FRAMES = ["normal", "effect", "fusion", "synchro", "xyz", "link", "ritual", "spell", "trap", "token", "skill"];
const SPACES = [1, 2, 3, 4, 6, 8];
const TYPE_SCALE: { cls: string; label: string }[] = [
  { cls: "text-title-xl", label: "Titolo XL · 24 px · Cinzel" },
  { cls: "text-title-lg", label: "Titolo L · 20 px · Cinzel" },
  { cls: "text-title", label: "Titolo · 18 px · Cinzel" },
  { cls: "text-body", label: "Corpo · 14 px · Inter" },
  { cls: "text-body-sm", label: "Corpo piccolo · 13 px" },
  { cls: "text-small", label: "Piccolo · 12 px" },
  { cls: "text-caption", label: "Didascalia · 11 px, solo badge" },
];
const ACCENTS: { id: SectionId; name: string }[] = [
  { id: "overview", name: "Generale (oro)" },
  { id: "deck-builder", name: "Fucina (brace)" },
  { id: "stats", name: "Laboratorio (ciano)" },
  { id: "combo", name: "Combo Lab (viola)" },
  { id: "shelf", name: "Bottega (ottone)" },
];

const TOOLBAR_ITEMS: ToolbarItem[] = [
  { id: "monster", label: "Mostri", priority: 3, active: true },
  { id: "spell", label: "Magie", priority: 2 },
  { id: "trap", label: "Trappole", priority: 2 },
  { id: "extra", label: "Extra Deck", priority: 1 },
  { id: "banned", label: "Proibite", priority: 0, active: true },
];

function GalleryToolbar({ compact }: { compact: boolean }) {
  return (
    <Toolbar
      items={compact ? TOOLBAR_ITEMS : TOOLBAR_ITEMS.slice(0, 3)}
      config={compact ? { maxVisibleItems: 2 } : {}}
      renderItem={(item) => <FilterChip label={item.label} active={item.active} count={item.active ? 12 : 4} size="sm" />}
      left={
        <label className="flex items-center gap-2 rounded-lg border border-border bg-bg-2 px-2.5 py-1">
          <Search size={14} className="text-faint" aria-hidden="true" />
          <input className="w-36 bg-transparent text-small outline-none placeholder:text-faint" placeholder="Cerca una carta…" aria-label="Cerca una carta" />
        </label>
      }
      right={
        <>
          <IconButton icon={<Undo2 size={15} />} tooltip="Annulla" ariaLabel="Annulla" variant="ghost" />
          <IconButton icon={<Redo2 size={15} />} tooltip="Ripeti" ariaLabel="Ripeti" variant="ghost" disabled />
          <IconButton icon={<Dices size={15} />} tooltip="Mano di prova" ariaLabel="Mano di prova" />
        </>
      }
    />
  );
}

const LOOKS: Look[] = [
  { color: "#7a1f1f", material: "leather", shape: "box" },
  { color: "#2b4a7a", material: "metal", shape: "tin" },
  { color: "#5a3a1c", material: "wood", shape: "chest" },
  { color: "#3a2a5c", material: "holo", shape: "box" },
  { color: "#1f4a3a", material: "matte", shape: "box" },
  { color: "#8a6a3a", material: "cardboard", shape: "box" },
];

export function Gallery() {
  const [accent, setAccent] = useState<SectionId>("overview");
  const [cards, setCards] = useState<Card[] | null>(null);
  const [cardsError, setCardsError] = useState<string | null>(null);

  useEffect(() => {
    // Drop the answer if the gallery unmounts first (StrictMode runs this twice).
    let live = true;
    api
      .getCards(SAMPLE_CARDS)
      .then((list) => {
        if (live) setCards(list);
      })
      .catch((e: unknown) => {
        if (live) setCardsError(`Impossibile leggere le carte di prova: ${String(e)}`);
      });
    return () => {
      live = false;
    };
  }, []);

  // The export script waits for this before reading the cards.
  const ready = cards !== null || cardsError !== null;
  const byId = new Map((cards ?? []).map((c) => [c.id, c]));
  const card = (id: number) => byId.get(id);
  const sectionOf = (id: SectionId) => sections.find((s) => s.id === id)!;

  return (
    <div data-gallery-ready={ready ? "1" : undefined} className="h-full overflow-y-auto">
      <header className="sticky top-0 z-20 flex items-center justify-between gap-4 border-b border-border bg-surface/90 px-8 py-3 backdrop-blur-sm">
        <div>
          <p className="eyebrow">Solo in modo mock</p>
          <h1 className="display text-2xl font-bold">Galleria dei componenti</h1>
        </div>
        <label className="flex items-center gap-2 text-small text-muted">
          Accento della sezione
          <select className="field !w-auto !py-1" value={accent} onChange={(e) => setAccent(e.target.value as SectionId)}>
            {ACCENTS.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
        </label>
      </header>

      <div className="mx-auto flex max-w-[1500px] flex-col gap-10 px-8 py-8">
        <DsCard id="colori" group="Fondamenta" name="Colori" accent={accent}>
          <Row label="Superfici e bordi">{SURFACES.map((t) => <Swatch key={t} token={t} />)}</Row>
          <Row label="Testo e metalli">{TEXTS.map((t) => <Swatch key={t} token={t} />)}</Row>
          <Row label="Stati">{STATES.map((t) => <Swatch key={t} token={t} />)}</Row>
        </DsCard>

        <DsCard id="accenti" group="Fondamenta" name="Accenti delle sezioni" note="--sec e --sec-2, scelti dalla sezione" accent={accent}>
          <div className="flex flex-wrap gap-3">
            {ACCENTS.map((a) => (
              <div key={a.id} data-section={a.id} className="flex w-36 flex-col gap-1">
                <div className="flex h-10 overflow-hidden rounded-lg border border-border">
                  <div className="flex-1" style={{ background: "var(--sec)" }} />
                  <div className="flex-1" style={{ background: "var(--sec-2)" }} />
                </div>
                <span className="text-caption text-muted">{a.name}</span>
              </div>
            ))}
          </div>
        </DsCard>

        <DsCard id="ruoli" group="Fondamenta" name="Ruoli e metriche" note="Controllati per il daltonismo" accent={accent}>
          <Row label="Ruoli delle carte (Laboratorio)">{ROLES.map((r) => <Swatch key={r} token={`--role-${r}`} />)}</Row>
          <Row label="Metriche del punteggio (Combo Lab)">{METRICS.map((m) => <Swatch key={m} token={`--metric-${m}`} />)}</Row>
        </DsCard>

        <DsCard id="cornici" group="Fondamenta" name="Cornici delle carte" accent={accent}>
          <Row label="Contrasto almeno 3:1 sulla superficie">{FRAMES.map((f) => <Swatch key={f} token={`--frame-${f}`} />)}</Row>
        </DsCard>

        <DsCard id="tipografia" group="Fondamenta" name="Tipografia" note="Mai sotto i 12 px, salvo i badge" accent={accent}>
          {TYPE_SCALE.map((t) => (
            <div key={t.cls} className="flex items-baseline gap-4">
              <span className={`${t.cls} w-64 shrink-0 truncate text-text`}>Blue-Eyes White Dragon</span>
              <span className="text-caption text-faint">{t.label}</span>
            </div>
          ))}
          <div className="flex flex-col gap-0.5">
            <p className="eyebrow">Eyebrow · etichetta di sezione</p>
            <p className="display text-2xl font-bold">La Fucina</p>
          </div>
        </DsCard>

        <DsCard id="spaziature" group="Fondamenta" name="Spaziature e superfici" accent={accent}>
          <Row label="Scala 4, 8, 12, 16, 24, 32 px">
            {SPACES.map((s) => (
              <div key={s} className="flex flex-col items-center gap-1">
                <div className="rounded-sm bg-sec" style={{ width: `var(--df-space-${s})`, height: `var(--df-space-${s})` }} />
                <span className="text-caption text-faint">{s * 4}</span>
              </div>
            ))}
          </Row>
          <Row label="Pannello, griglia tecnica">
            <div className="panel h-20 w-48 p-3 text-small text-muted">.panel</div>
            <div className="bg-blueprint h-20 w-48 rounded-xl border border-border p-3 text-small text-muted">.bg-blueprint</div>
          </Row>
        </DsCard>

        <DsCard id="pulsanti" group="Componenti" name="Pulsanti" accent={accent}>
          <Row label="Azione principale, secondaria, disattivata">
            <button type="button" className="btn btn-molten">
              <Sparkles size={15} /> Cerca le combo
            </button>
            <button type="button" className="btn btn-ghost">
              <Plus size={15} /> Nuovo deck
            </button>
            <button type="button" className="btn btn-molten" disabled>
              In attesa
            </button>
          </Row>
          <Row label="IconButton: default, ghost, surface, primary, danger">
            <IconButton icon={<Boxes size={15} />} tooltip="Apri nella Bottega" ariaLabel="Apri nella Bottega" />
            <IconButton icon={<Undo2 size={15} />} tooltip="Annulla" ariaLabel="Annulla" variant="ghost" />
            <IconButton icon={<Search size={15} />} tooltip="Cerca" ariaLabel="Cerca" variant="surface" />
            <IconButton icon={<Sparkles size={15} />} tooltip="Valuta" ariaLabel="Valuta" variant="primary" />
            <IconButton icon={<Trash2 size={15} />} tooltip="Elimina" ariaLabel="Elimina" variant="danger" />
          </Row>
          <Row label="Dimensioni sm, md, lg; attivo; disattivato">
            <IconButton icon={<Filter size={13} />} tooltip="Filtri" ariaLabel="Filtri" size="sm" />
            <IconButton icon={<Filter size={15} />} tooltip="Filtri" ariaLabel="Filtri" size="md" />
            <IconButton icon={<Filter size={17} />} tooltip="Filtri" ariaLabel="Filtri" size="lg" />
            <IconButton icon={<Filter size={15} />} tooltip="Filtri attivi" ariaLabel="Filtri attivi" active />
            <IconButton icon={<Filter size={15} />} tooltip="Filtri" ariaLabel="Filtri" disabled />
          </Row>
        </DsCard>

        <DsCard id="chip" group="Componenti" name="Chip di filtro" accent={accent}>
          <Row label="Normale, attivo, con colore, a zero, piccolo">
            <FilterChip label="Mostri" count={24} />
            <FilterChip label="Starter" count={9} active />
            <FilterChip label="Handtrap" count={6} color="var(--role-handtrap)" />
            <FilterChip label="Rituali" count={0} />
            <FilterChip label="Link" count={3} size="sm" />
            <FilterChip label="Filtri" icon={<Filter size={13} />} count={2} active />
          </Row>
          <Row label="Chip semplici (.chip)">
            <span className="chip px-2.5 py-1 text-small">Master Duel</span>
            <span className="chip chip-active px-2.5 py-1 text-small">TCG</span>
          </Row>
        </DsCard>

        <DsCard id="campi" group="Componenti" name="Campi" accent={accent}>
          <Row label="Testo, scelta, errore">
            <input className="field !w-56" placeholder="Nome del deck" aria-label="Nome del deck" />
            <select className="field !w-44" aria-label="Formato" defaultValue="md">
              <option value="md">Master Duel</option>
              <option value="tcg">TCG</option>
            </select>
            <input className="field !w-56 !border-danger" defaultValue="Deck senza nome" aria-label="Nome non valido" aria-invalid="true" />
          </Row>
          <Row label="Avanzamento (.molten-track)">
            <div className="molten-track h-2 w-72">
              <div className="molten-fill h-full" style={{ width: "62%" }} />
            </div>
          </Row>
        </DsCard>

        <DsCard id="barra" group="Componenti" name="Barra degli strumenti" note="Toolbar: piena e raccolta in «Filtri»" accent={accent}>
          <Row label="Piena (tre filtri)">
            <div className="w-full">
              <GalleryToolbar compact={false} />
            </div>
          </Row>
          <Row label="Raccolta in «Filtri» (cinque filtri, spazio stretto)">
            <div className="w-full">
              <GalleryToolbar compact />
            </div>
          </Row>
        </DsCard>

        <DsCard id="aiuto" group="Componenti" name="Aiuto e pannello laterale" note="HelpPopover, SidePanel" accent={accent}>
          <Row label="«?» al posto dei testi fissi">
            <span className="text-small text-muted">Probabilità di aprire uno starter</span>
            <HelpPopover title="Come si calcola">La probabilità è esatta: ipergeometrica sulle carte del deck.</HelpPopover>
          </Row>
          <div className="relative flex h-56 overflow-hidden rounded-xl border border-border bg-bg-2">
            <div className="flex-1 p-3 text-small text-faint">Contenuto della pagina</div>
            <SidePanel id="gallery" title="Dettagli" icon={<Sparkles size={14} />} isOpen overlayBelow={0} width={240}>
              <p className="text-small text-muted">Il pannello ricorda se è aperto o chiuso.</p>
            </SidePanel>
          </div>
        </DsCard>

        <DsCard id="stati" group="Componenti" name="Stati" note="Vuoto, caricamento, errore, dati" accent={accent}>
          <div className="grid grid-cols-2 gap-3">
            <div className="panel flex min-h-24 flex-col justify-center gap-1 p-3">
              <span className="eyebrow">Vuoto</span>
              <p className="text-small italic text-faint">Nessun deck salvato. Creane uno nella Fucina.</p>
            </div>
            <div className="panel flex min-h-24 flex-col justify-center gap-1 p-3">
              <span className="eyebrow">Caricamento</span>
              <Spinner label="Ricerca delle combo…" />
            </div>
            <div className="panel flex min-h-24 flex-col justify-center gap-1 p-3">
              <span className="eyebrow">Errore</span>
              <ErrorBox>Il motore non risponde. Controlla i dati in Panoramica e riprova.</ErrorBox>
            </div>
            <div className="panel flex min-h-24 flex-col justify-center gap-1.5 p-3">
              <span className="eyebrow">Dati</span>
              <div className="flex items-center gap-3">
                <FlaskMeter value={0.78} size={40} />
                <div>
                  <p className="text-body font-semibold tabular-nums">78,4%</p>
                  <p className="text-small text-muted">almeno uno starter in 5 carte</p>
                </div>
              </div>
            </div>
          </div>
        </DsCard>

        <DsCard id="carte" group="Carte" name="Carte" note="CardTile: rarità N, R, SR, UR; selezionata, proibita, non valida" accent={accent}>
          {cardsError ? (
            <ErrorBox>{cardsError}</ErrorBox>
          ) : (
            <div className="grid grid-cols-6 gap-2">
              <CardTile id={SAMPLE_CARDS[0]} card={card(SAMPLE_CARDS[0])} compact />
              <CardTile id={SAMPLE_CARDS[1]} card={card(SAMPLE_CARDS[1])} compact />
              <CardTile id={SAMPLE_CARDS[2]} card={card(SAMPLE_CARDS[2])} compact badge={{ text: "1", tone: "limit" }} />
              <CardTile id={SAMPLE_CARDS[3]} card={card(SAMPLE_CARDS[3])} compact selected />
              <CardTile id={SAMPLE_CARDS[4]} card={card(SAMPLE_CARDS[4])} compact badge={{ text: "0", tone: "ban" }} />
              <CardTile id={SAMPLE_CARDS[5]} card={card(SAMPLE_CARDS[5])} compact invalid />
            </div>
          )}
        </DsCard>

        <DsCard id="grafici" group="Grafici" name="Grafici" note="BarChart, Legend, FlaskMeter" accent={accent}>
          <Legend
            items={[
              { name: "Starter", color: "var(--role-starter)" },
              { name: "Handtrap", color: "var(--role-handtrap)" },
            ]}
          />
          <BarChart
            ariaLabel="Probabilità per numero di carte in mano"
            labels={["0", "1", "2", "3", "4+"]}
            series={[
              { name: "Starter", color: "var(--role-starter)", values: [8, 31, 36, 19, 6] },
              { name: "Handtrap", color: "var(--role-handtrap)", values: [22, 40, 26, 10, 2] },
            ]}
            unit="%"
            height={160}
          />
          <div className="flex items-end gap-4">
            <FlaskMeter value={0.15} />
            <FlaskMeter value={0.5} />
            <FlaskMeter value={0.92} bubbling />
          </div>
        </DsCard>

        <DsCard id="intestazione" group="Sezioni" name="Intestazione di sezione" note="SectionHeader, senza l'arte della carta" accent={accent}>
          <div className="overflow-hidden rounded-xl border border-border">
            <SectionHeader section={sectionOf(accent === "overview" ? "deck-builder" : accent)} artReady={false} />
          </div>
        </DsCard>

        <DsCard id="scatole" group="Bottega" name="Scatole dei deck" note="DeckBox3D: materiali e forme" accent="shelf">
          <div className="shelf-wall flex flex-wrap items-end justify-around gap-6 rounded-xl px-4 pt-6 pb-4">
            {LOOKS.map((look, i) => (
              <div key={look.material} className="flex flex-col items-center gap-2">
                <DeckBox3D
                  look={look}
                  name={["Snake-Eye", "Fire King", "Tenpai", "Yubel", "Branded", "Labrynth"][i]}
                  format={i % 3 === 0 ? "md" : i % 3 === 1 ? "tcg" : "ocg"}
                  legal={i === 4 ? false : true}
                  errors={i === 4 ? 2 : 0}
                  cards={SAMPLE_CARDS.slice(i % 3, (i % 3) + 3)}
                  scale={0.62}
                />
                <span className="text-caption text-muted">
                  {MATERIALS.find((m) => m.id === look.material)?.name} · {SHAPES.find((s) => s.id === look.shape)?.name}
                </span>
              </div>
            ))}
          </div>
        </DsCard>
      </div>
      <ChartTooltip />
    </div>
  );
}
