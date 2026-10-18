import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BoxEditor } from "../components/BoxEditor";
import { DeckBox3D } from "../components/DeckBox3D";
import { SectionHeader } from "../components/SectionHeader";
import { api, type DeckBox, type DeckFormat, type ShelfDeck } from "../lib/api";
import { resolveLook, SIZES } from "../lib/boxes";
import { FORMAT_LABELS } from "../lib/deck";
import { sound } from "../lib/sound";
import { toast } from "../lib/toast";
import { sectionById } from "../sections";

type Sort = "recent" | "name" | "format";

const SORTS: { id: Sort; label: string }[] = [
  { id: "recent", label: "Recenti" },
  { id: "name", label: "Nome" },
  { id: "format", label: "Formato" },
];

const GAP = 18;
const ROW_PADDING = 56;

/** Width of a box on the shelf (see DeckBox3D). */
const slot = (d: ShelfDeck) => SIZES[resolveLook(d.box, d.id).shape].w + 70;

interface Props {
  artReady: boolean;
  onOpenDeck: (id: number) => void;
  onNewDeck: () => void;
}

/** La Bottega: every deck as a 3D deck box on wooden shelves. */
export function Shelf({ artReady, onOpenDeck, onNewDeck }: Props) {
  const [decks, setDecks] = useState<ShelfDeck[] | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [format, setFormat] = useState<DeckFormat | null>(null);
  const [sort, setSort] = useState<Sort>("recent");
  const [width, setWidth] = useState(0);
  const areaRef = useRef<HTMLDivElement>(null);
  const saveTimers = useRef(new Map<string, number>());

  const load = useCallback(() => api.shelfDecks().then(setDecks), []);

  useEffect(() => {
    load().catch((e) => {
      setDecks([]);
      toast(`Scaffale non caricato: ${e}`, "error");
    });
  }, [load]);

  useEffect(() => {
    const el = areaRef.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setWidth(el.clientWidth));
    observer.observe(el);
    return () => observer.disconnect();
  }, [decks == null]);

  // Saves shortly after the last change, one timer per deck and kind of change.
  const later = (key: string, run: () => Promise<unknown>) => {
    window.clearTimeout(saveTimers.current.get(key));
    saveTimers.current.set(
      key,
      window.setTimeout(() => {
        run().catch((e) => toast(`Modifica non salvata: ${e}`, "error"));
      }, 350),
    );
  };

  const patch = (id: number, change: Partial<ShelfDeck>) => setDecks((list) => list?.map((d) => (d.id === id ? { ...d, ...change } : d)) ?? null);

  const setBox = (deck: ShelfDeck, box: DeckBox) => {
    patch(deck.id, { box, preview: box.showcase.length ? box.showcase : deck.preview });
    later(`box-${deck.id}`, async () => {
      await api.setDeckBox(deck.id, box);
      // Without a showcase the shelf shows the first Main Deck cards again.
      if (!box.showcase.length) await load();
    });
  };

  const rename = (deck: ShelfDeck, name: string) => {
    patch(deck.id, { name });
    later(`name-${deck.id}`, async () => {
      const full = await api.getDeck(deck.id);
      if (full) await api.saveDeck({ ...full, name });
    });
  };

  const duplicate = async (deck: ShelfDeck) => {
    const copy = await api.duplicateDeck(deck.id);
    sound.play("knock");
    await load();
    setSelected(copy.id);
    toast(`Creato «${copy.name}»`, "ok");
  };

  const remove = async (deck: ShelfDeck) => {
    await api.deleteDeck(deck.id);
    sound.play("close");
    setSelected(null);
    setDecks((list) => list?.filter((d) => d.id !== deck.id) ?? null);
    toast(`«${deck.name}» eliminato`);
  };

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = (decks ?? []).filter((d) => (!format || d.format === format) && (!q || d.name.toLowerCase().includes(q)));
    if (sort === "name") list.sort((a, b) => a.name.localeCompare(b.name, "it"));
    if (sort === "format") list.sort((a, b) => a.format.localeCompare(b.format) || a.name.localeCompare(b.name, "it"));
    return list;
  }, [decks, query, format, sort]);

  // Fill each shelf with as many boxes as fit.
  const rows = useMemo(() => {
    const out: ShelfDeck[][] = [];
    let row: ShelfDeck[] = [];
    let used = ROW_PADDING;
    const room = Math.max(width, 400);
    for (const d of shown) {
      const w = slot(d) + GAP;
      if (row.length && used + w > room) {
        out.push(row);
        row = [];
        used = ROW_PADDING;
      }
      row.push(d);
      used += w;
    }
    if (row.length) out.push(row);
    return out;
  }, [shown, width]);

  const current = decks?.find((d) => d.id === selected) ?? null;
  const section = sectionById("shelf");

  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={artReady} />
      <div className="flex flex-wrap items-center gap-3 border-b border-border bg-surface/60 px-8 py-3">
        <label className="relative block w-64">
          <span className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-faint">⌕</span>
          <input className="field !rounded-lg !py-1.5 !pl-8" placeholder="Cerca un deck…" value={query} onChange={(e) => setQuery(e.target.value)} />
        </label>
        <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1">
          {[null, "md", "tcg", "ocg"].map((f) => (
            <button
              key={f ?? "all"}
              type="button"
              onClick={() => {
                sound.play("toggle");
                setFormat(f as DeckFormat | null);
              }}
              className={`rounded-lg px-2.5 py-1 text-xs font-semibold ${format === f ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
            >
              {f ? FORMAT_LABELS[f as DeckFormat] : "Tutti"}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-1.5 text-xs text-muted">
          Ordina
          {SORTS.map((s) => (
            <button
              key={s.id}
              type="button"
              onClick={() => {
                sound.play("toggle");
                setSort(s.id);
              }}
              className={`chip !px-2 !py-0.5 !text-[11px] ${sort === s.id ? "chip-active" : ""}`}
            >
              {s.label}
            </button>
          ))}
        </div>
        <span className="ml-auto text-xs text-faint tabular-nums">
          {decks ? `${shown.length} deck` : ""}
        </span>
        <button type="button" className="btn btn-molten !py-1.5" onClick={onNewDeck}>
          ＋ Nuovo deck
        </button>
      </div>

      <div className="flex min-h-0 flex-1">
        <div
          ref={areaRef}
          className="relative min-w-0 flex-1 overflow-y-auto pt-8"
          style={{ background: "radial-gradient(ellipse at 50% -10%, color-mix(in oklab, var(--sec) 12%, transparent), transparent 60%)" }}
          onClick={(e) => e.target === e.currentTarget && setSelected(null)}
        >
          {decks && decks.length === 0 && (
            <div className="mx-auto mt-10 flex max-w-md flex-col items-center gap-3 text-center anim-rise">
              <p className="display text-xl">Lo scaffale è vuoto</p>
              <p className="text-sm text-muted">Ogni deck che forgi nella Fucina finisce qui, nel suo deck box.</p>
              <button type="button" className="btn btn-molten" onClick={onNewDeck}>
                Forgia il primo deck
              </button>
            </div>
          )}
          {decks && decks.length > 0 && shown.length === 0 && <p className="mt-10 text-center text-sm text-muted">Nessun deck corrisponde alla ricerca.</p>}
          {rows.map((row, r) => (
            <div key={r} className="anim-rise" style={{ animationDelay: `${r * 80}ms` }}>
              <div className="shelf-row">
                {row.map((d) => (
                  <DeckBox3D
                    key={d.id}
                    look={resolveLook(d.box, d.id)}
                    name={d.name}
                    format={d.format}
                    legal={d.legal}
                    errors={d.errors}
                    cards={d.preview}
                    interactive
                    selected={d.id === selected}
                    onHover={() => sound.play("hover")}
                    onClick={() => {
                      sound.play(d.id === selected ? "close" : "knock");
                      setSelected(d.id === selected ? null : d.id);
                    }}
                  />
                ))}
              </div>
              <div className="shelf-plank mx-4" />
            </div>
          ))}
          {/* An empty plank under the last row, so the shelf never looks cut off. */}
          {rows.length > 0 && (
            <div className="opacity-60">
              <div className="shelf-row !min-h-[120px]" />
              <div className="shelf-plank mx-4" />
            </div>
          )}
        </div>
        {current && (
          <BoxEditor
            key={current.id}
            deck={current}
            onBox={(box) => setBox(current, box)}
            onRename={(name) => rename(current, name)}
            onOpen={() => onOpenDeck(current.id)}
            onDuplicate={() => void duplicate(current)}
            onDelete={() => void remove(current)}
            onClose={() => {
              sound.play("close");
              setSelected(null);
            }}
          />
        )}
      </div>
    </div>
  );
}
