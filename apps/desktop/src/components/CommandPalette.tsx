import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { DeckSummary } from "../lib/api";
import { FORMAT_LABELS } from "../lib/deck";
import { updateSettings, useSettings } from "../lib/settings";
import { sound } from "../lib/sound";
import { sections, type Emblem, type SectionId } from "../sections";
import { EmblemIcon } from "./Emblems";

interface PaletteItem {
  id: string;
  group: "Sezioni" | "Deck in bottega" | "Azioni rapide";
  title: string;
  subtitle: string;
  emblem: Emblem;
  shortcut?: string;
  active?: boolean;
  keywords: string;
  run: () => void;
}

export function CommandPalette({
  open,
  activeSection,
  decks,
  onClose,
  onSelectSection,
  onOpenDeckIn,
  onNewDeck,
  onOpenGuide,
}: {
  open: boolean;
  activeSection: SectionId;
  decks: DeckSummary[];
  onClose: () => void;
  onSelectSection: (section: SectionId) => void;
  onOpenDeckIn: (deckId: number, section: SectionId) => void;
  onNewDeck: () => void;
  onOpenGuide: () => void;
}) {
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const settings = useSettings();

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSelectedIdx(0);
    const t = window.setTimeout(() => inputRef.current?.focus(), 10);
    return () => window.clearTimeout(t);
  }, [open]);

  const items = useMemo<PaletteItem[]>(() => {
    const list: PaletteItem[] = [];

    // 1. Sections
    sections.forEach((s, idx) => {
      list.push({
        id: `sec:${s.id}`,
        group: "Sezioni",
        title: `${s.label} — ${s.title}`,
        subtitle: s.summary,
        emblem: s.emblem,
        shortcut: `Ctrl+${idx + 1}`,
        active: activeSection === s.id,
        keywords: `${s.label} ${s.title} ${s.summary} sezione pagina`.toLowerCase(),
        run: () => {
          sound.play("page");
          onSelectSection(s.id);
        },
      });
    });

    // 2. Decks
    for (const d of decks) {
      const fmt = FORMAT_LABELS[d.format] ?? d.format.toUpperCase();
      const counts = `${fmt} · ${d.mainCount} Main / ${d.extraCount} Extra / ${d.sideCount} Side`;
      list.push({
        id: `deck-forge:${d.id}`,
        group: "Deck in bottega",
        title: `Apri «${d.name}» nella Fucina`,
        subtitle: `Modifica carte e lista · ${counts}`,
        emblem: "anvil",
        keywords: `${d.name} ${fmt} deck fucina modifica builder`.toLowerCase(),
        run: () => {
          sound.play("page");
          onOpenDeckIn(d.id, "deck-builder");
        },
      });
      list.push({
        id: `deck-stats:${d.id}`,
        group: "Deck in bottega",
        title: `Analizza «${d.name}» nel Laboratorio`,
        subtitle: `Ruoli, probabilità esatte e registro partite · ${counts}`,
        emblem: "flask",
        keywords: `${d.name} ${fmt} deck laboratorio statistiche probabilità ruoli`.toLowerCase(),
        run: () => {
          sound.play("page");
          onOpenDeckIn(d.id, "stats");
        },
      });
      list.push({
        id: `deck-combo:${d.id}`,
        group: "Deck in bottega",
        title: `Cerca combo di «${d.name}» nel Circolo Arcano`,
        subtitle: `Ricerca autonoma del primo turno e turno di prova · ${counts}`,
        emblem: "rune",
        keywords: `${d.name} ${fmt} deck combo circolo arcano linee board`.toLowerCase(),
        run: () => {
          sound.play("page");
          onOpenDeckIn(d.id, "combo");
        },
      });
    }

    // 3. Quick actions
    list.push({
      id: "act:new-deck",
      group: "Azioni rapide",
      title: "Forgia un nuovo deck vuoto",
      subtitle: "Apre la Fucina con un nuovo deck pronto da comporre",
      emblem: "anvil",
      keywords: "nuovo deck crea vuoto fucina",
      run: () => {
        sound.play("forge");
        onNewDeck();
      },
    });

    const isLight =
      settings.theme === "light" ||
      (settings.theme === "system" && typeof matchMedia !== "undefined" && matchMedia("(prefers-color-scheme: light)").matches);

    list.push({
      id: "act:theme",
      group: "Azioni rapide",
      title: isLight ? "Passa al tema scuro «Fucina»" : "Passa al tema chiaro «Pergamena»",
      subtitle: isLight ? "Carbone caldo, oro fuso e braci" : "Carta pergamena chiara ad alto contrasto",
      emblem: "cog",
      keywords: "tema aspetto chiaro scuro fucina pergamena colori",
      run: () => {
        sound.play("toggle");
        updateSettings({ theme: isLight ? "forge" : "light" });
      },
    });

    list.push({
      id: "act:sound",
      group: "Azioni rapide",
      title: settings.sound ? "Disattiva effetti sonori" : "Attiva effetti sonori",
      subtitle: settings.sound ? "Silenzia i suoni sintetizzati dell'interfaccia" : "Ripristina i suoni dell'officina",
      emblem: "bell",
      keywords: "audio suono volume muto silenzia effetti",
      run: () => {
        const next = !settings.sound;
        updateSettings({ sound: next });
        if (next) sound.play("chime");
      },
    });

    list.push({
      id: "act:guide",
      group: "Azioni rapide",
      title: "Apri la Guida rapida interattiva",
      subtitle: "Ripassa i 4 passi dell'officina e le scorciatoie da tastiera",
      emblem: "gear",
      shortcut: "?",
      keywords: "guida aiuto benvenuto scorciatoie tutorial",
      run: () => {
        sound.play("open");
        onOpenGuide();
      },
    });

    return list;
  }, [activeSection, decks, onNewDeck, onOpenDeckIn, onOpenGuide, onSelectSection, settings.sound, settings.theme]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    const tokens = q.split(/\s+/);
    return items.filter((item) => {
      const hay = `${item.title.toLowerCase()} ${item.subtitle.toLowerCase()} ${item.keywords}`;
      return tokens.every((t) => hay.includes(t));
    });
  }, [items, query]);

  useEffect(() => {
    setSelectedIdx(0);
  }, [query]);

  useEffect(() => {
    if (!open) return;
    const el = listRef.current?.querySelector<HTMLElement>(`[data-palette-idx="${selectedIdx}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIdx, open]);

  if (!open) return null;

  const execute = (item: PaletteItem | undefined) => {
    if (!item) return;
    onClose();
    item.run();
  };

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/70 px-4 pt-[11vh] backdrop-blur-sm anim-fade"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) {
          sound.play("close");
          onClose();
        }
      }}
      role="dialog"
      aria-modal="true"
      aria-label="Palette dei comandi"
    >
      <div
        className="panel flex max-h-[74vh] w-full max-w-2xl flex-col overflow-hidden border-border-strong"
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            sound.play("close");
            onClose();
          } else if (e.key === "ArrowDown") {
            e.preventDefault();
            setSelectedIdx((i) => (filtered.length ? (i + 1) % filtered.length : 0));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setSelectedIdx((i) => (filtered.length ? (i - 1 + filtered.length) % filtered.length : 0));
          } else if (e.key === "Enter") {
            e.preventDefault();
            execute(filtered[selectedIdx]);
          }
        }}
      >
        {/* Search input header */}
        <div className="flex items-center gap-3 border-b border-border bg-surface-2/70 px-4 py-3">
          <span className="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-border-strong bg-bg-2 text-gold">
            <EmblemIcon emblem="anvil" className="h-4 w-4" />
          </span>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Cerca una sezione, un deck in bottega o un'azione rapida…"
            className="min-w-0 flex-1 rounded-lg border border-border bg-bg-2/70 px-3 py-1.5 text-sm text-text placeholder:text-faint focus:border-gold/60 focus:outline-none"
            aria-label="Cerca comando"
          />
          {query && (
            <button
              type="button"
              onClick={() => {
                setQuery("");
                inputRef.current?.focus();
              }}
              className="rounded-md px-2 py-0.5 text-xs text-muted hover:bg-surface-3 hover:text-text"
            >
              Pulisci
            </button>
          )}
          <kbd className="rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-faint">Esc</kbd>
        </div>

        {/* Results list */}
        <div ref={listRef} className="flex-1 overflow-y-auto p-2.5">
          {filtered.length === 0 ? (
            <div className="py-10 text-center text-sm text-muted">
              Nessun comando trovato per <span className="font-semibold text-text">«{query}»</span>.
            </div>
          ) : (
            (["Sezioni", "Deck in bottega", "Azioni rapide"] as const).map((group) => {
              const groupItems = filtered
                .map((item, idx) => ({ item, idx }))
                .filter((entry) => entry.item.group === group);
              if (groupItems.length === 0) return null;
              return (
                <div key={group} className="mb-3 last:mb-0">
                  <p className="eyebrow px-2.5 py-1">{group}</p>
                  <div className="flex flex-col gap-1">
                    {groupItems.map(({ item, idx }) => {
                      const selected = idx === selectedIdx;
                      return (
                        <button
                          key={item.id}
                          type="button"
                          data-palette-idx={idx}
                          onMouseEnter={() => setSelectedIdx(idx)}
                          onClick={() => execute(item)}
                          className={`flex w-full items-center gap-3 rounded-xl border px-3 py-2 text-left transition ${
                            selected
                              ? "border-gold/60 bg-surface-3 text-text shadow-sm"
                              : "border-transparent bg-transparent text-muted hover:bg-surface-2 hover:text-text"
                          }`}
                        >
                          <span
                            className={`grid h-8 w-8 shrink-0 place-items-center rounded-lg border transition ${
                              selected ? "border-gold/50 bg-bg-2 text-gold" : "border-border bg-surface-2 text-faint"
                            }`}
                          >
                            <EmblemIcon emblem={item.emblem} className="h-4 w-4" />
                          </span>
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <span className="truncate text-xs font-semibold text-text">{item.title}</span>
                              {item.active && <span className="chip chip-active !px-2 !py-0 !text-[10px]">Aperta</span>}
                            </div>
                            <p className="truncate text-[11px] text-muted">{item.subtitle}</p>
                          </div>
                          {item.shortcut && (
                            <kbd className="shrink-0 rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] font-semibold text-gold">
                              {item.shortcut}
                            </kbd>
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer hints */}
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border bg-surface-2/60 px-4 py-2 text-[11px] text-muted">
          <div className="flex items-center gap-3">
            <span>
              <kbd className="rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-gold">↑</kbd>{" "}
              <kbd className="rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-gold">↓</kbd> Scorri
            </span>
            <span>
              <kbd className="rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-gold">Invio</kbd> Esegui
            </span>
            <span>
              <kbd className="rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-gold">Ctrl+1..7</kbd> Sezioni
            </span>
          </div>
          <span>
            <kbd className="rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-gold">?</kbd> Guida rapida
          </span>
        </div>
      </div>
    </div>,
    document.body,
  );
}
