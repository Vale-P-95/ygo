import type { ReactNode } from "react";
import { DataSetup } from "../components/DataSetup";
import { EmblemIcon } from "../components/Emblems";
import { SectionHeader } from "../components/SectionHeader";
import { formatDate, type DataStatus } from "../lib/api";
import { sectionById, type Emblem, type SectionId } from "../sections";

const ROADMAP: { phase: number; title: string; state: "done" | "now" | "next" }[] = [
  { phase: 0, title: "Fondamenta e prova del motore", state: "done" },
  { phase: 1, title: "Database carte, visualizzatore, suoni ed effetti", state: "done" },
  { phase: 2, title: "Deck Builder e banlist (Master Duel)", state: "done" },
  { phase: 3, title: "La Bottega: deck box 3D", state: "done" },
  { phase: 4, title: "Il Laboratorio statistico", state: "done" },
  { phase: 5, title: "Circolo Arcano: ricerca automatica delle combo e classifica delle board", state: "now" },
  { phase: 6, title: "Handtrap, andare secondo, turno avversario", state: "next" },
  { phase: 7, title: "Rifinitura e ricerca", state: "next" },
] as const;

function Tile({ label, value, hint, accent }: { label: string; value: ReactNode; hint?: string; accent?: boolean }) {
  return (
    <div className="panel px-3.5 py-2.5">
      <p className="eyebrow">{label}</p>
      <p className={`display mt-0.5 text-xl font-bold tabular-nums ${accent ? "text-gold" : ""}`}>{value}</p>
      {hint && <p className="mt-0.5 truncate text-[11px] text-faint">{hint}</p>}
    </div>
  );
}

interface QuickStep {
  step: number;
  title: string;
  badge: string;
  ready: boolean;
  description: string;
  emblem: Emblem;
  actionLabel: string;
  target: SectionId;
  secondaryLabel?: string;
  secondaryTarget?: SectionId;
}

export function Overview({
  status,
  deckCount = 0,
  onStatus,
  go,
  onOpenGuide,
}: {
  status: DataStatus | null;
  deckCount?: number;
  onStatus: (s: DataStatus) => void;
  go: (id: SectionId) => void;
  onOpenGuide?: () => void;
}) {
  const section = sectionById("overview");
  const n = (v: number | null | undefined) => (v == null ? "—" : v.toLocaleString("it-IT"));
  const dataReady = !!status?.cards && !!status?.engine.scripts;

  const steps: QuickStep[] = [
    {
      step: 1,
      title: "Dati e motore EDOPro",
      badge: dataReady ? `✓ ${n(status?.cards?.count)} carte` : "Da scaricare",
      ready: dataReady,
      description:
        "Catalogo YGOPRODeck, banlist di Master Duel e script Lua ufficiali per eseguire ogni effetto col motore reale.",
      emblem: "cog",
      actionLabel: "Impostazioni dati",
      target: "settings",
    },
    {
      step: 2,
      title: "Forgia un deck in Bottega",
      badge: deckCount > 0 ? `✓ ${deckCount} deck` : "Da creare",
      ready: deckCount > 0,
      description:
        "Cerca le carte con filtri rapidi, importa .ydk o ydke://, estrai una mano di prova e personalizza il deck box 3D.",
      emblem: "anvil",
      actionLabel: "Apri la Fucina",
      target: "deck-builder",
      secondaryLabel: "La Bottega",
      secondaryTarget: "shelf",
    },
    {
      step: 3,
      title: "Riconosci ruoli e probabilità",
      badge: "Laboratorio",
      ready: false,
      description:
        "Usa ✦ Riconosci per classificare Starter, Extender e Handtrap col motore, poi calcola probabilità esatte e tagli.",
      emblem: "flask",
      actionLabel: "Apri il Laboratorio",
      target: "stats",
    },
    {
      step: 4,
      title: "Combo Lab e turno di prova",
      badge: "Circolo Arcano",
      ready: false,
      description:
        "Scegli una mano iniziale o una carta: il motore trova da solo le linee migliori e misura le interruzioni della board.",
      emblem: "rune",
      actionLabel: "Apri il Circolo Arcano",
      target: "combo",
    },
  ];

  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={!!status?.cards} />
      <div className="flex-1 overflow-y-auto px-8 py-4">
        {status && !status.cards ? (
          <DataSetup onReady={onStatus} />
        ) : (
          <div className="flex flex-col gap-4 anim-rise">
            <div className="grid grid-cols-5 gap-3">
              <Tile label="Carte" value={n(status?.cards?.count)} hint={status?.cards ? `YGOPRODeck, versione ${status.cards.version}` : undefined} accent />
              <Tile label="Script del motore" value={n(status?.engine.scripts)} hint="Project Ignis CardScripts" />
              <Tile label="Banlist Master Duel" value={status?.banlistMd ? formatDate(status.banlistMd) : "—"} hint="in vigore dal" />
              <Tile label="Immagini in cache" value={n(status?.imagesCached)} hint="scaricate quando servono" />
              <Tile label="Motore di gioco" value={`API ${status?.engine.apiVersion ?? "—"}`} hint="EDOPro core" />
            </div>

            <div className="grid grid-cols-[1.45fr_1fr] gap-4">
              <section className="panel p-4">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <p className="eyebrow">Primi passi nell'officina</p>
                    <h2 className="display mt-0.5 text-base font-bold">Dal primo deck alla ricerca delle combo</h2>
                  </div>
                  {onOpenGuide && (
                    <button type="button" onClick={onOpenGuide} className="btn btn-ghost !px-3 !py-1 text-xs">
                      ? Guida rapida interattiva
                    </button>
                  )}
                </div>

                <div className="mt-3 grid grid-cols-2 gap-2.5">
                  {steps.map((item) => (
                    <div key={item.step} className="flex flex-col justify-between rounded-xl border border-border bg-bg-2/55 p-3">
                      <div>
                        <div className="flex items-center justify-between gap-2">
                          <span className="flex items-center gap-2 text-xs font-semibold text-text">
                            <span
                              className={`grid h-5 w-5 shrink-0 place-items-center rounded-full text-[11px] font-bold ${
                                item.ready ? "bg-ok text-bg" : "border border-border-strong text-gold"
                              }`}
                            >
                              {item.ready ? "✓" : item.step}
                            </span>
                            {item.title}
                          </span>
                          <span className={`chip !px-2 !py-0.5 !text-[10px] ${item.ready ? "border-ok/50 text-ok" : "text-muted"}`}>
                            {item.badge}
                          </span>
                        </div>
                        <p className="mt-1.5 text-xs leading-relaxed text-muted">{item.description}</p>
                      </div>
                      <div className="mt-2.5 flex flex-wrap items-center gap-2">
                        <button
                          type="button"
                          onClick={() => go(item.target)}
                          className={`btn !px-2.5 !py-1 text-xs ${item.step === 2 ? "btn-molten" : "btn-ghost"}`}
                        >
                          <EmblemIcon emblem={item.emblem} className="h-3.5 w-3.5" />
                          {item.actionLabel}
                        </button>
                        {item.secondaryLabel && item.secondaryTarget && (
                          <button
                            type="button"
                            onClick={() => go(item.secondaryTarget!)}
                            className="btn btn-ghost !px-2.5 !py-1 text-xs text-muted hover:text-text"
                          >
                            <EmblemIcon emblem="shelf" className="h-3.5 w-3.5" />
                            {item.secondaryLabel}
                          </button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </section>

              <section className="panel p-4">
                <p className="eyebrow">Roadmap</p>
                <h2 className="display mt-0.5 text-base font-bold">Stato del progetto</h2>
                <ol className="mt-3 flex flex-col gap-2 text-xs">
                  {ROADMAP.map((step) => (
                    <li key={step.phase} className="flex items-center gap-2.5">
                      <span
                        className={`grid h-5 w-5 shrink-0 place-items-center rounded-full text-[10px] font-bold ${
                          step.state === "done" ? "bg-ok text-bg" : step.state === "now" ? "btn-molten !p-0" : "border border-border text-faint"
                        }`}
                      >
                        {step.phase}
                      </span>
                      <span className={step.state === "next" ? "text-muted" : "font-medium"}>{step.title}</span>
                      {step.state === "now" && <span className="chip chip-active ml-auto !px-2 !py-0.5 !text-[10px]">in corso</span>}
                    </li>
                  ))}
                </ol>
              </section>
            </div>
            <p className="text-[11px] text-faint">Cartella dati: {status?.dataDir}</p>
          </div>
        )}
      </div>
    </div>
  );
}
