import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { DataStatus } from "../lib/api";
import { sound } from "../lib/sound";
import type { Emblem, SectionId } from "../sections";
import { EmblemIcon } from "./Emblems";

export const ONBOARDING_KEY = "duelforge.onboardingSeen";

export function hasSeenOnboarding(): boolean {
  try {
    return localStorage.getItem(ONBOARDING_KEY) === "1";
  } catch {
    return true;
  }
}

export function markOnboardingSeen(): void {
  try {
    localStorage.setItem(ONBOARDING_KEY, "1");
  } catch {
    /* localStorage disabled */
  }
}

interface GuideStep {
  number: number;
  title: string;
  subtitle: string;
  emblem: Emblem;
  section: SectionId;
  actionLabel: string;
  summary: string;
  highlights: { title: string; detail: string }[];
  shortcuts?: { key: string; label: string }[];
}

const STEPS: GuideStep[] = [
  {
    number: 1,
    title: "Accendi la fucina",
    subtitle: "Dati delle carte, script Lua e motore EDOPro",
    emblem: "gear",
    section: "overview",
    actionLabel: "Vai all'Officina",
    summary:
      "Al primo avvio DuelForge scarica una sola volta (~80 MB) l'elenco completo delle carte da YGOPRODeck, la banlist di Master Duel e gli script ufficiali di Project Ignis per il motore di gioco C++.",
    highlights: [
      {
        title: "Motore di gioco reale integrato",
        detail: "Non usa regole approssimate: ogni effetto, catena e costo viene eseguito dal vero motore di EDOPro.",
      },
      {
        title: "Immagini leggere su richiesta",
        detail: "Le illustrazioni delle carte vengono scaricate e salvate nella cache locale solo quando le visualizzi.",
      },
      {
        title: "Aggiornamenti in un clic",
        detail: "Dalle Impostazioni o dall'Officina puoi aggiornare carte, script e banlist in qualsiasi momento.",
      },
    ],
  },
  {
    number: 2,
    title: "La Fucina e La Bottega",
    subtitle: "Costruisci i tuoi deck e organizza le deck box 3D",
    emblem: "anvil",
    section: "deck-builder",
    actionLabel: "Apri la Fucina",
    summary:
      "Nella Fucina cerchi le carte con filtri rapidi (formato, archetipo, attributo, rarità Master Duel o scorciatoie testuali) e componi Main, Extra e Side Deck verificando sempre la legalità.",
    highlights: [
      {
        title: "Importa, esporta e prova la mano",
        detail: "Importa o esporta file .ydk e codici ydke:// dagli appunti, oppure estrai subito una Mano di prova di 5 o 6 carte.",
      },
      {
        title: "Lente d'ingrandimento sulla carta",
        detail: "Fai doppio clic su qualsiasi carta per aprirla a tutto schermo con testo formattato, prezzi, banlist e carte correlate.",
      },
      {
        title: "La Bottega: scaffale 3D",
        detail: "Ogni deck ha il suo portamazzo 3D con pelle, metallo, stemma e 3 carte in vetrina personalizzabili.",
      },
    ],
    shortcuts: [
      { key: "Clic destro", label: "Aggiunge la carta al deck o la rimuove dal banco" },
      { key: "Maiusc + Clic destro", label: "Invia direttamente la carta al Side Deck" },
      { key: "Doppio clic", label: "Apre la carta nella lente d'ingrandimento" },
      { key: "Ctrl+Z / Ctrl+Y", label: "Annulla o ripeti le ultime modifiche al deck" },
    ],
  },
  {
    number: 3,
    title: "Il Laboratorio",
    subtitle: "Ruoli automatici, probabilità esatte e registro partite",
    emblem: "flask",
    section: "stats",
    actionLabel: "Apri il Laboratorio",
    summary:
      "Il Laboratorio unisce il calcolo ipergeometrico multivariato esatto con le prove del motore di gioco per misurare la stabilità del deck prima di scendere in classifica.",
    highlights: [
      {
        title: "✦ Riconosci i ruoli col motore",
        detail: "Il motore prova ogni carta da sola per distinguere Starter (1-card combo), Extender, Handtrap, Board breaker e Brick.",
      },
      {
        title: "Calcolatore, Simulatore e Ottimizzatore",
        detail: "Costruisci condizioni E/O con ricette pronte, simula l'effetto di carte che pescano o scavano e scopri i tagli migliori col grafico Tornado.",
      },
      {
        title: "Registro partite e versioni",
        detail: "Segna i match per avversario, moneta e versione del deck: l'app calcola win rate con intervallo di Wilson al 95% e test esatto di Fisher.",
      },
    ],
  },
  {
    number: 4,
    title: "Il Circolo Arcano",
    subtitle: "Ricerca autonoma delle combo e turno di prova",
    emblem: "rune",
    section: "combo",
    actionLabel: "Apri il Circolo Arcano",
    summary:
      "Nel Circolo Arcano scegli una mano iniziale di 5 carte (oppure 1–2 carte specifiche più pescate casuali) e lasci che il motore esplori da solo migliaia di giocate legali del primo turno.",
    highlights: [
      {
        title: "Vincoli su mano, mazzo e campo",
        detail: "Col lucchetto sulle carte in mano le tieni per il turno avversario; col clic destro sul Main Deck vieti i garnet; con Obiettivo chiedi mostri specifici a fine turno.",
      },
      {
        title: "Turno di prova contro l'avversario simulato",
        detail: "Ogni board finale viene sfidata da un avversario simulato per contare quante interruzioni reali (negazioni, rimozioni, floodgate) produce.",
      },
      {
        title: "Replay passo per passo e copertura",
        detail: "Rivedi ogni mossa sul Tappetino interattivo oppure lancia Copertura del deck per misurare quante mani su 100 arrivano a una board solida.",
      },
    ],
    shortcuts: [
      { key: "Lucchetto in mano", label: "Vieta al motore di usare o scartare quella carta" },
      { key: "Clic destro sul mazzo", label: "Impedisce di mandare quella carta al cimitero/bandite" },
      { key: "← / → nel Replay", label: "Scorre i singoli passi della linea combo trovata" },
    ],
  },
];

export function WelcomeModal({
  open,
  status,
  deckCount,
  onClose,
  onGo,
}: {
  open: boolean;
  status: DataStatus | null;
  deckCount: number;
  onClose: () => void;
  onGo: (section: SectionId) => void;
}) {
  const [stepIdx, setStepIdx] = useState(0);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        markOnboardingSeen();
        onClose();
      } else if (e.key === "ArrowRight") {
        setStepIdx((i) => Math.min(STEPS.length - 1, i + 1));
      } else if (e.key === "ArrowLeft") {
        setStepIdx((i) => Math.max(0, i - 1));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const step = STEPS[stepIdx];
  const dataReady = !!status?.cards && !!status?.engine.scripts;

  const finishAndClose = () => {
    sound.play("close");
    markOnboardingSeen();
    onClose();
  };

  const jumpToSection = (section: SectionId) => {
    sound.play("page");
    markOnboardingSeen();
    onClose();
    onGo(section);
  };

  return createPortal(
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/75 p-6 backdrop-blur-sm anim-fade"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) finishAndClose();
      }}
      role="dialog"
      aria-modal="true"
      aria-labelledby="welcome-modal-title"
    >
      <div className="panel relative flex max-h-[88vh] w-full max-w-[820px] flex-col overflow-hidden border-border-strong">
        {/* Header */}
        <div className="flex items-start justify-between gap-4 border-b border-border bg-surface-2/60 px-6 py-4">
          <div className="flex items-center gap-3.5">
            <span
              className="grid h-11 w-11 shrink-0 place-items-center rounded-xl text-[#1a0d05]"
              style={{
                background: "linear-gradient(135deg, #ffd27a, var(--df-gold) 45%, var(--df-ember))",
                boxShadow: "0 0 24px -6px var(--df-ember)",
              }}
            >
              <EmblemIcon emblem={step.emblem} className="h-6 w-6" strokeWidth={2} />
            </span>
            <div>
              <p className="eyebrow">Guida rapida · Benvenuto in DuelForge</p>
              <h2 id="welcome-modal-title" className="display mt-0.5 text-xl font-bold">
                L'officina dei duellanti in 4 passi
              </h2>
            </div>
          </div>
          <button
            type="button"
            onClick={finishAndClose}
            className="btn btn-ghost !px-2.5 !py-1.5 text-xs text-muted hover:text-text"
            aria-label="Chiudi guida"
          >
            ✕ Chiudi
          </button>
        </div>

        {/* Step selector tabs */}
        <div className="grid grid-cols-4 gap-2 border-b border-border bg-bg-2/50 px-6 py-3">
          {STEPS.map((s, idx) => {
            const active = idx === stepIdx;
            const done = (idx === 0 && dataReady) || (idx === 1 && deckCount > 0);
            return (
              <button
                key={s.number}
                type="button"
                onClick={() => {
                  sound.play("toggle");
                  setStepIdx(idx);
                }}
                className={`flex items-center gap-2.5 rounded-xl border px-3 py-2 text-left transition ${
                  active
                    ? "border-gold/70 bg-surface-3 text-text shadow-sm"
                    : "border-border bg-surface/60 text-muted hover:border-border-strong hover:bg-surface-2 hover:text-text"
                }`}
              >
                <span
                  className={`grid h-6 w-6 shrink-0 place-items-center rounded-full text-xs font-bold ${
                    done
                      ? "bg-ok text-bg"
                      : active
                        ? "bg-gold text-[#1a0d05]"
                        : "border border-border-strong text-faint"
                  }`}
                >
                  {done ? "✓" : s.number}
                </span>
                <div className="min-w-0">
                  <p className="truncate text-xs font-semibold">{s.title}</p>
                  <p className="truncate text-[10px] text-faint">Passo {s.number} di 4</p>
                </div>
              </button>
            );
          })}
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <p className="eyebrow">
                Passo {step.number} · {step.subtitle}
              </p>
              <h3 className="display mt-0.5 text-lg font-bold">{step.title}</h3>
            </div>
            {step.number === 1 && (
              <span className={`chip ${dataReady ? "border-ok/50 text-ok" : "border-warn/50 text-warn"}`}>
                {dataReady ? `✓ Dati e motore pronti (${status?.cards?.count.toLocaleString("it-IT")} carte)` : "Dati da scaricare al primo avvio"}
              </span>
            )}
            {step.number === 2 && (
              <span className={`chip ${deckCount > 0 ? "border-ok/50 text-ok" : ""}`}>
                {deckCount > 0 ? `✓ ${deckCount} deck salvati nell'officina` : "Nessun deck ancora salvato"}
              </span>
            )}
          </div>

          <p className="mt-2.5 text-sm leading-relaxed text-muted">{step.summary}</p>

          <div className="mt-4 grid grid-cols-3 gap-3">
            {step.highlights.map((h) => (
              <div key={h.title} className="rounded-xl border border-border bg-bg-2/60 p-3.5">
                <p className="text-xs font-semibold text-text">{h.title}</p>
                <p className="mt-1.5 text-xs leading-relaxed text-muted">{h.detail}</p>
              </div>
            ))}
          </div>

          {step.shortcuts && (
            <div className="mt-4 rounded-xl border border-border bg-surface-2/50 px-4 py-3">
              <p className="eyebrow mb-2">Scorciatoie utili</p>
              <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
                {step.shortcuts.map((sc) => (
                  <div key={sc.key} className="flex items-center gap-2.5">
                    <kbd className="shrink-0 whitespace-nowrap rounded border border-border-strong bg-bg-2 px-2 py-0.5 font-mono text-[11px] font-semibold text-gold">
                      {sc.key}
                    </kbd>
                    <span className="leading-snug text-muted">{sc.label}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between gap-3 border-t border-border bg-surface-2/60 px-6 py-3.5">
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={stepIdx === 0}
              onClick={() => {
                sound.play("toggle");
                setStepIdx((i) => Math.max(0, i - 1));
              }}
              className="btn btn-ghost !px-3 !py-1.5 text-xs"
            >
              ← Precedente
            </button>
            <button
              type="button"
              disabled={stepIdx === STEPS.length - 1}
              onClick={() => {
                sound.play("toggle");
                setStepIdx((i) => Math.min(STEPS.length - 1, i + 1));
              }}
              className="btn btn-ghost !px-3 !py-1.5 text-xs"
            >
              Successivo →
            </button>
          </div>

          <div className="flex items-center gap-2.5">
            <button type="button" onClick={() => jumpToSection(step.section)} className="btn btn-ghost !px-3.5 !py-1.5 text-xs">
              <EmblemIcon emblem={step.emblem} className="h-4 w-4" />
              {step.actionLabel}
            </button>
            <button type="button" onClick={finishAndClose} className="btn btn-molten !px-4 !py-1.5 text-xs">
              Inizia a usare DuelForge
            </button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
