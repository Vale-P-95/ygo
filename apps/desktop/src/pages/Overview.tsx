import type { ReactNode } from "react";
import { DataSetup } from "../components/DataSetup";
import { EmblemIcon } from "../components/Emblems";
import { SectionHeader } from "../components/SectionHeader";
import { formatDate, type DataStatus } from "../lib/api";
import { sectionById, type SectionId } from "../sections";

const ROADMAP: { phase: number; title: string; state: "done" | "now" | "next" }[] = [
  { phase: 0, title: "Fondamenta e prova del motore", state: "done" },
  { phase: 1, title: "Database carte, visualizzatore, suoni ed effetti", state: "done" },
  { phase: 2, title: "Deck Builder e banlist (Master Duel)", state: "done" },
  { phase: 3, title: "La Bottega: deck box 3D", state: "done" },
  { phase: 4, title: "Il Laboratorio statistico", state: "done" },
  { phase: 5, title: "Circolo Arcano: ricerca automatica delle combo e classifica delle board", state: "next" },
  { phase: 6, title: "Handtrap, andare secondo, turno avversario", state: "next" },
  { phase: 7, title: "Rifinitura e ricerca", state: "next" },
] as const;

function Tile({ label, value, hint, accent }: { label: string; value: ReactNode; hint?: string; accent?: boolean }) {
  return (
    <div className="panel p-4">
      <p className="eyebrow">{label}</p>
      <p className={`display mt-1 text-2xl font-bold tabular-nums ${accent ? "text-gold" : ""}`}>{value}</p>
      {hint && <p className="mt-1 truncate text-xs text-faint">{hint}</p>}
    </div>
  );
}

export function Overview({ status, onStatus, go }: { status: DataStatus | null; onStatus: (s: DataStatus) => void; go: (id: SectionId) => void }) {
  const section = sectionById("overview");
  const n = (v: number | null | undefined) => (v == null ? "—" : v.toLocaleString("it-IT"));
  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={!!status?.cards} />
      <div className="flex-1 overflow-y-auto px-10 py-8">
        {status && !status.cards ? (
          <DataSetup onReady={onStatus} />
        ) : (
          <div className="flex flex-col gap-6 anim-rise">
            <div className="grid grid-cols-5 gap-4">
              <Tile label="Carte" value={n(status?.cards?.count)} hint={status?.cards ? `YGOPRODeck, versione ${status.cards.version}` : undefined} accent />
              <Tile label="Script del motore" value={n(status?.engine.scripts)} hint="Project Ignis CardScripts" />
              <Tile label="Banlist Master Duel" value={status?.banlistMd ? formatDate(status.banlistMd) : "—"} hint="in vigore dal" />
              <Tile label="Immagini in cache" value={n(status?.imagesCached)} hint="scaricate quando servono" />
              <Tile label="Motore di gioco" value={`API ${status?.engine.apiVersion ?? "—"}`} hint="EDOPro core" />
            </div>
            <div className="grid grid-cols-[1.3fr_1fr] gap-6">
              <section className="panel p-6">
                <p className="eyebrow">Da dove cominciare</p>
                <h2 className="display mt-1 text-xl font-bold">Entra nella Fucina</h2>
                <p className="mt-2 text-sm text-muted">
                  Sfoglia tutte le carte, filtra per tipo, attributo, statistiche, archetipo e rarità di Master Duel, e apri qualsiasi carta nella lente per
                  vederne ogni dettaglio.
                </p>
                <button type="button" className="btn btn-molten mt-5" onClick={() => go("deck-builder")}>
                  <EmblemIcon emblem="anvil" className="h-4 w-4" /> Apri la Fucina
                </button>
              </section>
              <section className="panel p-6">
                <p className="eyebrow">Roadmap</p>
                <ol className="mt-3 flex flex-col gap-2 text-sm">
                  {ROADMAP.map((step) => (
                    <li key={step.phase} className="flex items-center gap-3">
                      <span
                        className={`grid h-6 w-6 shrink-0 place-items-center rounded-full text-[11px] font-bold ${
                          step.state === "done" ? "bg-ok text-bg" : step.state === "now" ? "btn-molten !p-0" : "border border-border text-faint"
                        }`}
                      >
                        {step.phase}
                      </span>
                      <span className={step.state === "next" ? "text-muted" : "font-medium"}>{step.title}</span>
                      {step.state === "now" && <span className="chip chip-active ml-auto !text-[10px]">in corso</span>}
                    </li>
                  ))}
                </ol>
              </section>
            </div>
            <p className="text-xs text-faint">Cartella dati: {status?.dataDir}</p>
          </div>
        )}
      </div>
    </div>
  );
}
