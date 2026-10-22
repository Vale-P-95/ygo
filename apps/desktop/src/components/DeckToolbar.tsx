import { useEffect, useState } from "react";
import { cardImage, formatDate, isDesktop, type Deck, type DeckFormat, type DeckSummary, type Legality } from "../lib/api";
import { FORMAT_LABELS } from "../lib/deck";
import { sound } from "../lib/sound";
import { Eye } from "lucide-react";
import { EmblemIcon } from "./Emblems";
import { Popover, useOutsideClose } from "./Popover";
import { HelpPopover } from "./ui/HelpPopover";

interface Props {
  deck: Deck;
  decks: DeckSummary[];
  legality: Legality | null;
  saveState: "idle" | "saving" | "saved";
  cost: { ur: number; sr: number; r: number; n: number };
  canUndo: boolean;
  canRedo: boolean;
  onRename: (name: string) => void;
  onFormat: (format: DeckFormat) => void;
  onNew: () => void;
  onOpenDeck: (id: number) => void;
  onImport: () => void;
  onExport: () => void;
  onImage: () => void;
  onTestHand: () => void;
  onDelete: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onFlash: (cards: number[]) => void;
  inspectorOpen?: boolean;
  onToggleInspector?: () => void;
}

export function DeckToolbar(p: Props) {
  const [menu, setMenu] = useState(false);
  const [issuesOpen, setIssuesOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [exportMenu, setExportMenu] = useState(false);
  const exportRef = useOutsideClose(exportMenu, () => setExportMenu(false));
  const menuRef = useOutsideClose(menu, () => setMenu(false));
  const issuesRef = useOutsideClose(issuesOpen, () => setIssuesOpen(false));
  const errors = p.legality?.issues.filter((i) => i.severity === "error") ?? [];
  const warnings = p.legality?.issues.filter((i) => i.severity === "warning") ?? [];

  useEffect(() => {
    if (!confirmDelete) return;
    const t = window.setTimeout(() => setConfirmDelete(false), 3000);
    return () => window.clearTimeout(t);
  }, [confirmDelete]);

  return (
    <div className="relative z-20 border-b border-border bg-surface/70 backdrop-blur">
      <div className="flex items-center gap-3 xl:gap-4 px-4 xl:px-6 pt-2.5 xl:pt-3 pb-2">
        <div ref={menuRef} className="relative">
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => {
              sound.play(menu ? "close" : "open");
              setMenu((v) => !v);
            }}
            aria-expanded={menu}
          >
            <EmblemIcon emblem="shelf" className="h-4 w-4" /> I miei deck <span className="text-faint">▾</span>
          </button>
          <Popover open={menu}>
            <button
              type="button"
              className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm font-semibold text-sec hover:bg-surface-3"
              onClick={() => {
                setMenu(false);
                p.onNew();
              }}
            >
              ＋ Nuovo deck
            </button>
            <button
              type="button"
              className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm hover:bg-surface-3"
              onClick={() => {
                setMenu(false);
                p.onImport();
              }}
            >
              ⇣ Importa .ydk o ydke://
            </button>
            {p.decks.length > 0 && <div className="my-2 border-t border-border" />}
            {p.decks.map((d) => (
              <button
                key={d.id}
                type="button"
                onPointerEnter={() => sound.play("hover")}
                onClick={() => {
                  setMenu(false);
                  p.onOpenDeck(d.id);
                }}
                className={`flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-surface-3 ${d.id === p.deck.id ? "bg-surface-3" : ""}`}
              >
                <span className="h-11 w-8 shrink-0 overflow-hidden rounded bg-surface-3">
                  {d.cover != null && isDesktop && <img src={cardImage(d.cover, "small")} alt="" className="h-full w-full object-cover" />}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-medium">{d.name}</span>
                  <span className="text-[11px] text-faint">
                    {FORMAT_LABELS[d.format]} · {d.mainCount}/{d.extraCount}/{d.sideCount}
                  </span>
                </span>
              </button>
            ))}
          </Popover>
        </div>
        <div className="min-w-0 flex-1">
          <input
            className="display w-full max-w-xl truncate rounded border-b border-transparent bg-transparent text-xl font-bold outline-none hover:border-border focus:border-sec"
            value={p.deck.name}
            onChange={(e) => p.onRename(e.target.value)}
            spellCheck={false}
            aria-label="Nome del deck"
          />
        </div>
        <span className="w-20 text-right text-[11px] text-faint">{p.saveState === "saving" ? "Salvataggio…" : p.saveState === "saved" ? "Salvato ✓" : ""}</span>
        <div className="flex items-center gap-1">
          <button type="button" className="btn btn-ghost !px-2.5 !py-1.5" disabled={!p.canUndo} onClick={p.onUndo} title="Annulla (Ctrl+Z)" aria-label="Annulla">
            ↶
          </button>
          <button type="button" className="btn btn-ghost !px-2.5 !py-1.5" disabled={!p.canRedo} onClick={p.onRedo} title="Ripeti (Ctrl+Y)" aria-label="Ripeti">
            ↷
          </button>
          <button type="button" className="btn btn-ghost !py-1.5" onClick={p.onTestHand} disabled={p.deck.main.length === 0} title="Pesca mani di prova dal Main Deck">
            Mano di prova
          </button>
          <div ref={exportRef} className="relative">
            <button
              type="button"
              className="btn btn-ghost !py-1.5"
              aria-expanded={exportMenu}
              onClick={() => {
                sound.play(exportMenu ? "close" : "open");
                setExportMenu((v) => !v);
              }}
            >
              Esporta <span className="text-faint">▾</span>
            </button>
            <Popover open={exportMenu} align="right" width={240}>
              {[
                { label: "Testo: .ydk e ydke://", run: p.onExport },
                { label: "Immagine PNG", run: p.onImage },
              ].map((item) => (
                <button
                  key={item.label}
                  type="button"
                  className="flex w-full rounded-lg px-3 py-2 text-left text-sm hover:bg-surface-3"
                  onClick={() => {
                    setExportMenu(false);
                    item.run();
                  }}
                >
                  {item.label}
                </button>
              ))}
            </Popover>
          </div>
          {p.deck.id != null && (
            <button
              type="button"
              className={`btn !py-1.5 ${confirmDelete ? "bg-danger text-white" : "btn-ghost"}`}
              onClick={() => {
                if (confirmDelete) {
                  setConfirmDelete(false);
                  p.onDelete();
                } else {
                  sound.play("toggle");
                  setConfirmDelete(true);
                }
              }}
            >
              {confirmDelete ? "Conferma" : "Elimina"}
            </button>
          )}
        </div>
      </div>
      <div className="flex items-center gap-3 xl:gap-4 px-4 xl:px-6 pb-2 xl:pb-2.5">
        <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1">
          {(["md", "tcg", "ocg"] as const).map((f) => (
            <button
              key={f}
              type="button"
              onClick={() => {
                sound.play("toggle");
                p.onFormat(f);
              }}
              className={`rounded-lg px-2.5 py-1 text-xs font-semibold ${p.deck.format === f ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
            >
              {FORMAT_LABELS[f]}
            </button>
          ))}
        </div>
        <div ref={issuesRef} className="relative">
          <button
            type="button"
            onClick={() => {
              sound.play(issuesOpen ? "close" : "open");
              setIssuesOpen((v) => !v);
            }}
            className="chip !py-1.5 font-semibold"
            style={{
              borderColor: errors.length ? "var(--df-danger)" : "color-mix(in oklab, var(--df-ok) 70%, transparent)",
              color: errors.length ? "var(--df-danger)" : "var(--df-ok)",
            }}
          >
            {p.legality == null ? "…" : errors.length ? `✕ ${errors.length} ${errors.length === 1 ? "problema" : "problemi"}` : `✓ Legale in ${FORMAT_LABELS[p.deck.format]}`}
          </button>
          <Popover open={issuesOpen} width={420}>
            {p.legality?.banlist && <p className="px-2 pt-1 pb-2 text-[11px] text-faint">Banlist {FORMAT_LABELS[p.deck.format]} in vigore dal {formatDate(p.legality.banlist)}</p>}
            {errors.length === 0 && <p className="px-2 py-1.5 text-sm text-ok">Il deck rispetta le regole di {FORMAT_LABELS[p.deck.format]}.</p>}
            {[...errors, ...warnings].map((issue, i) => (
              <button
                key={i}
                type="button"
                onClick={() => issue.cards.length && p.onFlash(issue.cards)}
                className="flex w-full gap-2 rounded-lg px-2 py-1.5 text-left text-xs hover:bg-surface-3"
              >
                <span className={issue.severity === "error" ? "text-danger" : "text-warn"}>{issue.severity === "error" ? "✕" : "!"}</span>
                <span>{issue.message}</span>
              </button>
            ))}
          </Popover>
        </div>
        {p.deck.format === "md" && (
          <div className="flex items-center gap-1.5" title="Carte da creare in Master Duel, per rarità">
            {(["ur", "sr", "r", "n"] as const).map((r) => (
              <span key={r} className="flex items-center gap-1 text-xs tabular-nums text-muted">
                <span className="rarity-gem !static" data-rarity={r}>
                  {r.toUpperCase()}
                </span>
                {p.cost[r]}
              </span>
            ))}
          </div>
        )}
        <div className="ml-auto flex items-center gap-2 shrink-0">
          {p.onToggleInspector && (
            <button
              type="button"
              className={`btn btn-ghost !py-1 !px-2.5 text-xs flex items-center gap-1.5 font-medium ${
                p.inspectorOpen ? "text-sec font-bold" : "text-text"
              }`}
              onClick={p.onToggleInspector}
              title="Ispettore carta (pannello laterale)"
              aria-label="Attiva o disattiva ispettore carta"
            >
              <Eye size={14} className={p.inspectorOpen ? "text-sec" : "text-text"} />
              <span>Ispettore</span>
            </button>
          )}

          <HelpPopover title="Comandi della Fucina" tooltip="Guida della Fucina" align="right">
            <div className="flex flex-col gap-2.5 text-xs text-text/90">
              <div>
                <p className="font-semibold text-text">Gestione carte e deck</p>
                <ul className="mt-1 list-disc pl-4 space-y-1 text-muted">
                  <li><strong>Trascina:</strong> sposta le carte tra banco, Extra, Side e Catalogo.</li>
                  <li><strong>Clic singolo:</strong> esamina e fissa la carta nell'Ispettore a sinistra.</li>
                  <li><strong>Doppio clic:</strong> apri il visore a schermo intero con lente.</li>
                  <li><strong>Clic destro nel deck:</strong> rimuovi la carta dal deck.</li>
                  <li><strong>Trascina nel catalogo:</strong> rimuovi la carta dal deck.</li>
                </ul>
              </div>
              <div className="border-t border-border/60 pt-2">
                <p className="font-semibold text-text">Scorciatoie da tastiera</p>
                <ul className="mt-1 list-disc pl-4 space-y-1 text-muted">
                  <li><kbd className="rounded bg-surface-3 px-1 py-0.5 font-mono text-[11px]">Esc</kbd>: sblocca la carta fissata nell'Ispettore.</li>
                  <li><kbd className="rounded bg-surface-3 px-1 py-0.5 font-mono text-[11px]">Ctrl+Z</kbd> / <kbd className="rounded bg-surface-3 px-1 py-0.5 font-mono text-[11px]">Ctrl+Y</kbd>: annulla / ripeti modifica.</li>
                  <li><kbd className="rounded bg-surface-3 px-1 py-0.5 font-mono text-[11px]">Ctrl+F</kbd>: ricerca nel catalogo.</li>
                </ul>
              </div>
            </div>
          </HelpPopover>
        </div>
      </div>
    </div>
  );
}
