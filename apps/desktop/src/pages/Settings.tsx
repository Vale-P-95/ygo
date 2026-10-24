import type { ReactNode } from "react";
import { Play, Sparkles } from "lucide-react";
import { DataSetup } from "../components/DataSetup";
import { SectionHeader } from "../components/SectionHeader";
import type { DataStatus } from "../lib/api";
import { updateSettings, useSettings, type Settings as SettingsShape, type Theme } from "../lib/settings";
import { sound, type SoundName } from "../lib/sound";
import { burst } from "../lib/sparks";
import { sectionById } from "../sections";

function Row({ title, hint, children }: { title: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6 border-b border-border py-3 last:border-0">
      <div>
        <p className="text-sm font-medium">{title}</p>
        {hint && <p className="text-xs text-muted">{hint}</p>}
      </div>
      {children}
    </div>
  );
}

function Toggle({ value, onChange, label }: { value: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      aria-label={label}
      onClick={() => {
        onChange(!value);
        sound.play("toggle");
      }}
      className="relative h-6 w-11 rounded-full border transition-colors"
      style={{
        background: value ? "linear-gradient(90deg, var(--df-gold), var(--df-ember))" : "var(--df-bg-2)",
        borderColor: value ? "transparent" : "var(--df-border-strong)",
      }}
    >
      <span className="absolute top-0.5 h-[18px] w-[18px] rounded-full bg-text shadow transition-all" style={{ left: value ? 22 : 3 }} />
    </button>
  );
}

function Slider({ value, onChange, label }: { value: number; onChange: (v: number) => void; label: string }) {
  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(value * 100)}
        aria-label={label}
        onChange={(e) => onChange(Number(e.target.value) / 100)}
        onPointerUp={() => sound.play("select", true)}
        className="w-44 accent-[var(--df-gold)]"
      />
      <span className="w-9 text-right text-xs text-muted tabular-nums">{Math.round(value * 100)}%</span>
    </div>
  );
}

const THEMES: { value: Theme; label: string }[] = [
  { value: "forge", label: "Fucina (scuro)" },
  { value: "light", label: "Pergamena (chiaro)" },
  { value: "system", label: "Come il sistema" },
];

const SOUND_TESTS: { name: SoundName; label: string }[] = [
  { name: "select", label: "Selezione" },
  { name: "open", label: "Apertura" },
  { name: "chime", label: "Conferma" },
  { name: "error", label: "Errore" },
  { name: "forge", label: "Martello" },
  { name: "knock", label: "Scatola" },
  { name: "latch", label: "Chiusura" },
];

export function Settings({ onStatus }: { onStatus: (s: DataStatus) => void }) {
  const s = useSettings();
  const set = (patch: Partial<SettingsShape>) => updateSettings(patch);
  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={sectionById("settings")} />
      <div className="grid flex-1 grid-cols-2 content-start gap-6 overflow-y-auto px-10 py-8">
        <section className="panel p-6 anim-rise">
          <h2 className="display mb-2 text-lg font-bold">Aspetto ed effetti</h2>
          <Row title="Tema">
            <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1">
              {THEMES.map((t) => (
                <button
                  key={t.value}
                  type="button"
                  onClick={() => {
                    set({ theme: t.value });
                    sound.play("toggle");
                  }}
                  className={`rounded-lg px-3 py-1.5 text-xs font-semibold ${s.theme === t.value ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
                >
                  {t.label}
                </button>
              ))}
            </div>
          </Row>
          <Row title="Braci nell'aria" hint="Particelle animate dietro i contenuti">
            <Toggle label="Braci" value={s.ambient} onChange={(v) => set({ ambient: v })} />
          </Row>
          <Row title="Effetti sulle carte" hint="Inclinazione 3D, riflessi olografici UR/SR, scintille">
            <Toggle label="Effetti sulle carte" value={s.cardEffects} onChange={(v) => set({ cardEffects: v })} />
          </Row>
          <Row title="Transizioni tra pagine" hint="Breve dissolvenza al cambio di sezione">
            <Toggle label="Transizioni tra pagine" value={s.pageTransitions} onChange={(v) => set({ pageTransitions: v })} />
          </Row>
          <Row title="Animazioni di liste e risultati" hint="Comparsa delle board, delle carte e dei grafici">
            <Toggle label="Animazioni di liste e risultati" value={s.listAnimations} onChange={(v) => set({ listAnimations: v })} />
          </Row>
          <Row title="Effetti della Bottega" hint="Oscillazione, riflessi e sollevamento delle scatole">
            <Toggle label="Effetti della Bottega" value={s.shelfEffects} onChange={(v) => set({ shelfEffects: v })} />
          </Row>
          <Row title="Effetti della mappa dei collegamenti" hint="Impulso lungo i collegamenti scelti">
            <Toggle label="Effetti della mappa dei collegamenti" value={s.linksEffects} onChange={(v) => set({ linksEffects: v })} />
          </Row>
          <Row title="Anteprima al passaggio del mouse">
            <Toggle label="Anteprima" value={s.hoverPreview} onChange={(v) => set({ hoverPreview: v })} />
          </Row>
          <button type="button" className="btn btn-ghost mt-3 flex items-center gap-1.5" onClick={(e) => burst(e.clientX, e.clientY, 40, 1.3)}>
            <Sparkles size={15} />
            <span>Prova le scintille</span>
          </button>
        </section>

        <section className="panel p-6 anim-rise" style={{ animationDelay: "60ms" }}>
          <h2 className="display mb-2 text-lg font-bold">Audio</h2>
          <Row title="Suoni" hint="Tutti i suoni sono sintetizzati dall'app">
            <Toggle label="Suoni" value={s.sound} onChange={(v) => set({ sound: v })} />
          </Row>
          <Row title="Interfaccia" hint="Passaggio del mouse, clic, schede, cambio pagina">
            <Toggle label="Suoni dell'interfaccia" value={s.soundUi} onChange={(v) => set({ soundUi: v })} />
          </Row>
          <Row title="Eventi" hint="Ricerca finita, conferme, errori, laboratorio">
            <Toggle label="Suoni degli eventi" value={s.soundEvents} onChange={(v) => set({ soundEvents: v })} />
          </Row>
          <Row title="Carte e Bottega" hint="Scatole posate sulla mensola, chiusure">
            <Toggle label="Suoni di carte e Bottega" value={s.soundShelf} onChange={(v) => set({ soundShelf: v })} />
          </Row>
          <Row title="Volume generale">
            <Slider label="Volume generale" value={s.masterVolume} onChange={(v) => set({ masterVolume: v })} />
          </Row>
          <Row title="Interfaccia" hint="Passaggio del mouse, interruttori, cambio pagina">
            <Slider label="Volume interfaccia" value={s.uiVolume} onChange={(v) => set({ uiVolume: v })} />
          </Row>
          <Row title="Effetti" hint="Selezioni, aperture, conferme">
            <Slider label="Volume effetti" value={s.fxVolume} onChange={(v) => set({ fxVolume: v })} />
          </Row>
          <div className="mt-3 flex flex-wrap gap-2">
            {SOUND_TESTS.map((t) => (
              <button key={t.name} type="button" className="btn btn-ghost !py-1.5 text-xs flex items-center gap-1.5" onClick={() => sound.play(t.name, true)}>
                <Play size={13} className="fill-current text-sec" />
                <span>{t.label}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="panel col-span-2 p-6 anim-rise" style={{ animationDelay: "120ms" }}>
          <h2 className="display mb-1 text-lg font-bold">Dati</h2>
          <p className="mb-4 text-sm text-muted">
            Elenco delle carte da YGOPRODeck, script e database del motore da Project Ignis. L'aggiornamento scarica solo ciò che è cambiato.
          </p>
          <DataSetup compact onReady={onStatus} />
        </section>

        <p className="col-span-2 text-xs text-faint">
          DuelForge 0.1 · open source (AGPL-3.0) · progetto non ufficiale, non affiliato né approvato da Konami. Dati delle carte: YGOPRODeck. Motore e
          script: Project Ignis (EDOPro).
        </p>
      </div>
    </div>
  );
}
