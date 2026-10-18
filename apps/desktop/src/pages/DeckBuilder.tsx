import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { CardPreview } from "../components/CardPreview";
import { CardViewer } from "../components/CardViewer";
import { Catalog } from "../components/Catalog";
import { DataSetup } from "../components/DataSetup";
import { ExportDialog, ImageDialog, ImportDialog } from "../components/DeckIO";
import { DeckToolbar } from "../components/DeckToolbar";
import { DeckZone } from "../components/DeckZone";
import { SectionHeader } from "../components/SectionHeader";
import { TestHand } from "../components/TestHand";
import { api, cardImage, type Card, type DataStatus, type Deck, type DeckSummary, type Imported, type Legality } from "../lib/api";
import { useCardCache } from "../lib/cards";
import { addCard, counts, craftCost, emptyDeck, removeAt, sortIds, useDeckHistory, ZONE_LIMITS, type Zone } from "../lib/deck";
import { setDropHandler, type DragSource, type DropZone } from "../lib/drag";
import { useSettings } from "../lib/settings";
import { sound } from "../lib/sound";
import { burstFrom } from "../lib/sparks";
import { toast } from "../lib/toast";
import { sectionById } from "../sections";

const LAST_DECK = "duelforge.lastDeck";
const NONE = new Set<number>();

function rememberDeck(id: number | null) {
  try {
    if (id == null) localStorage.removeItem(LAST_DECK);
    else localStorage.setItem(LAST_DECK, String(id));
  } catch {
    // Only a convenience: reopening the last deck.
  }
}

function lastDeck(): number | null {
  try {
    const v = Number(localStorage.getItem(LAST_DECK));
    return Number.isFinite(v) && v > 0 ? v : null;
  } catch {
    return null;
  }
}

export function DeckBuilder({ status, onStatus }: { status: DataStatus | null; onStatus: (s: DataStatus) => void }) {
  if (!status?.cards) {
    return (
      <div className="flex h-full flex-col">
        <SectionHeader section={sectionById("deck-builder")} artReady={false} />
        <div className="flex-1 overflow-y-auto px-10">
          <DataSetup onReady={onStatus} />
        </div>
      </div>
    );
  }
  return <DeckEditor />;
}

function Dot({ color }: { color: string }) {
  return <span className="inline-block h-2 w-2 rounded-full" style={{ background: color }} />;
}

function DeckEditor() {
  const settings = useSettings();
  const cards = useCardCache();
  // The history functions are stable; the returned object is not.
  const { deck, set: setHistory, reset, undo, redo, canUndo, canRedo } = useDeckHistory(emptyDeck());
  const deckRef = useRef(deck);
  deckRef.current = deck;
  /** Id of the stored copy: new decks get one on their first autosave. */
  const idRef = useRef<number | null>(null);
  const dirty = useRef(false);
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [legality, setLegality] = useState<Legality | null>(null);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved">("idle");
  const [fresh, setFresh] = useState<{ zone: Zone; index: number; nonce: number } | null>(null);
  const [flash, setFlash] = useState<Set<number>>(new Set());
  const [hover, setHover] = useState<{ id: number; rect: DOMRect } | null>(null);
  const hoverTimer = useRef<number>(undefined);
  const [viewer, setViewer] = useState<{ id: number; list: number[] } | null>(null);
  const [dialog, setDialog] = useState<"import" | "export" | "image" | "hand" | null>(null);
  const [query, setQuery] = useState("format:md sort:popular");
  const getCard = cards.get;

  const refreshDecks = useCallback(() => api.listDecks().then((list) => (setDecks(list), list)), []);

  const openDeck = useCallback(
    async (id: number) => {
      const stored = await api.getDeck(id);
      if (!stored) return;
      dirty.current = false;
      idRef.current = stored.id;
      rememberDeck(stored.id);
      reset(stored);
      setSaveState("saved");
      sound.play("page");
    },
    [reset],
  );

  useEffect(() => {
    refreshDecks().then((list) => {
      const id = list.find((d) => d.id === lastDeck())?.id ?? list[0]?.id;
      if (id != null) void openDeck(id);
    });
  }, [refreshDecks, openDeck]);

  useEffect(() => {
    cards.request([...deck.main, ...deck.extra, ...deck.side, ...deck.maybe]);
  }, [deck, cards.request]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      api.validateDeck(deck).then(setLegality).catch(() => setLegality(null));
    }, 150);
    return () => window.clearTimeout(timer);
  }, [deck]);

  // Autosave shortly after each change.
  useEffect(() => {
    if (!dirty.current) return;
    const empty = deck.main.length + deck.extra.length + deck.side.length + deck.maybe.length === 0;
    if (idRef.current == null && empty) return;
    setSaveState("saving");
    const timer = window.setTimeout(async () => {
      try {
        const saved = await api.saveDeck({ ...deck, id: idRef.current });
        idRef.current = saved.id;
        rememberDeck(saved.id);
        dirty.current = false;
        setSaveState("saved");
        void refreshDecks();
      } catch (e) {
        setSaveState("idle");
        toast(`Salvataggio non riuscito: ${e}`, "error");
      }
    }, 700);
    return () => window.clearTimeout(timer);
  }, [deck, refreshDecks]);

  const update = useCallback(
    (next: Deck, record = true) => {
      dirty.current = true;
      setHistory(next, record);
    },
    [setHistory],
  );

  const setRoles = useCallback((roles: Record<string, number[]>) => update({ ...deckRef.current, roles }, false), [update]);

  const forged = useCallback((zone: Zone, index: number) => {
    sound.play("select");
    setFresh({ zone, index, nonce: Date.now() });
    requestAnimationFrame(() => burstFrom(document.querySelector(`[data-deck-slot="${zone}-${index}"]`), 18, 0.9));
  }, []);

  const tryAdd = useCallback(
    (base: Deck, card: Card | undefined, want: Zone | null) => {
      if (!card) return false;
      const result = addCard(base, card, want, getCard);
      if ("error" in result) {
        sound.play("error");
        toast(result.error, "error");
        return false;
      }
      update(result.deck);
      forged(result.zone, result.index);
      return true;
    },
    [getCard, update, forged],
  );

  const remove = useCallback(
    (zone: Zone, index: number) => {
      sound.play("close");
      setHover(null);
      update(removeAt(deckRef.current, zone, index));
    },
    [update],
  );

  useEffect(() => {
    setDropHandler((source: DragSource, target: DropZone) => {
      const current = deckRef.current;
      const card = getCard(source.id);
      if (source.from === "catalog") {
        if (target !== "catalog") tryAdd(current, card, target === "main" ? null : target);
        return;
      }
      if (target === source.from || source.index == null) return;
      const without = removeAt(current, source.from, source.index);
      if (target === "catalog") {
        sound.play("close");
        update(without);
        return;
      }
      tryAdd(without, card, target === "main" ? null : target);
    });
    return () => setDropHandler(null);
  }, [getCard, tryAdd, update]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || (e.target as HTMLElement).closest("input, textarea")) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) {
        e.preventDefault();
        dirty.current = true;
        undo();
        sound.play("toggle");
      } else if (k === "y" || (k === "z" && e.shiftKey)) {
        e.preventDefault();
        dirty.current = true;
        redo();
        sound.play("toggle");
      }
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [undo, redo]);

  const startHover = useCallback(
    (id: number, el: HTMLElement) => {
      sound.play("hover");
      window.clearTimeout(hoverTimer.current);
      if (!settings.hoverPreview) return;
      hoverTimer.current = window.setTimeout(() => setHover({ id, rect: el.getBoundingClientRect() }), 420);
    },
    [settings.hoverPreview],
  );
  const endHover = useCallback(() => {
    window.clearTimeout(hoverTimer.current);
    setHover(null);
  }, []);

  const flashCards = (ids: number[]) => {
    setFlash(new Set(ids));
    window.setTimeout(() => setFlash(new Set()), 2200);
  };

  const newDeck = () => {
    sound.play("forge");
    dirty.current = false;
    idRef.current = null;
    rememberDeck(null);
    reset(emptyDeck());
    setSaveState("idle");
  };

  const imported = async (result: Imported, name: string) => {
    setDialog(null);
    const all = [...result.main, ...result.extra, ...result.side];
    const data = new Map((await api.getCards([...new Set(all)])).map((c) => [c.id, c]));
    const lookup = (id: number) => data.get(id);
    idRef.current = null;
    dirty.current = true;
    reset({ ...emptyDeck(name, deckRef.current.format), main: sortIds(result.main, lookup), extra: sortIds(result.extra, lookup), side: sortIds(result.side, lookup) });
    sound.play("forge");
    toast(`Importato «${name}»: ${result.main.length}/${result.extra.length}/${result.side.length} carte`, "ok");
    if (result.unknown.length) toast(`${result.unknown.length} passcode non riconosciuti sono stati saltati`, "error");
  };

  const removeDeck = async () => {
    if (idRef.current == null) return;
    await api.deleteDeck(idRef.current);
    sound.play("close");
    toast(`«${deck.name}» eliminato`);
    const list = await refreshDecks();
    if (list[0]) void openDeck(list[0].id);
    else newDeck();
  };

  const invalid = new Set([...(legality?.issues.flatMap((i) => (i.severity === "error" ? i.cards : [])) ?? []), ...flash]);
  const main = counts(deck.main, getCard);
  const extraTypes = ["Fusion", "Synchro", "Xyz", "Link"].map((t) => [t, deck.extra.filter((id) => getCard(id)?.typeLine.includes(t)).length] as const);
  const viewerCard = viewer ? getCard(viewer.id) : undefined;
  const viewerIndex = viewer ? viewer.list.indexOf(viewer.id) : -1;
  const hoverCard = hover ? getCard(hover.id) : undefined;
  const themeArt = sectionById("deck-builder").themeCard!;

  const zoneProps = { format: deck.format, getCard, invalid, fresh, onRemove: remove, onHover: startHover, onHoverEnd: endHover };

  return (
    <div className="flex h-full flex-col">
      <DeckToolbar
        deck={deck}
        decks={decks}
        legality={legality}
        saveState={saveState}
        cost={craftCost(deck, getCard)}
        canUndo={canUndo}
        canRedo={canRedo}
        onRename={(name) => update({ ...deck, name }, false)}
        onFormat={(format) => update({ ...deck, format })}
        onNew={newDeck}
        onOpenDeck={(id) => void openDeck(id)}
        onImport={() => setDialog("import")}
        onExport={() => setDialog("export")}
        onTestHand={() => setDialog("hand")}
        onImage={() => setDialog("image")}
        onDelete={() => void removeDeck()}
        onUndo={() => {
          dirty.current = true;
          undo();
        }}
        onRedo={() => {
          dirty.current = true;
          redo();
        }}
        onFlash={flashCards}
      />

      <div
        data-deck-layout
        className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_min(var(--catalog-w),55vw)]"
        style={{ "--catalog-w": `${settings.catalogWidth}px` } as CSSProperties}
      >
        <div className="bg-blueprint relative min-h-0 overflow-y-auto px-6 py-5">
          <img
            src={cardImage(themeArt.id, "cropped")}
            alt=""
            className="pointer-events-none absolute top-0 right-0 h-72 w-[60%] object-cover opacity-[0.10]"
            style={{ maskImage: "radial-gradient(ellipse at 70% 20%, black, transparent 70%)" }}
          />
          <div className="relative mx-auto flex max-w-[1180px] flex-col gap-5">
            <DeckZone
              {...zoneProps}
              zone="main"
              title="Main Deck"
              ids={deck.main}
              columns={10}
              slots={40}
              max={60}
              onOpen={(id) => setViewer({ id, list: deck.main })}
              info={
                <>
                  <span className="flex items-center gap-1.5">
                    <Dot color="#cf7a43" /> Mostri {main.monsters}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Dot color="#1d9e74" /> Magie {main.spells}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Dot color="#b8457f" /> Trappole {main.traps}
                  </span>
                </>
              }
              emptyHint={
                <div className="flex max-w-sm flex-col items-center gap-3 text-sm text-muted">
                  <p className="display text-lg text-text">Il banco è vuoto</p>
                  <p>Trascina qui le carte dal catalogo, oppure clic destro su una carta per aggiungerla.</p>
                  <button type="button" className="btn btn-molten pointer-events-auto" onClick={() => setDialog("import")}>
                    Importa un deck
                  </button>
                </div>
              }
            />
            <DeckZone
              {...zoneProps}
              zone="extra"
              title="Extra Deck"
              ids={deck.extra}
              columns={15}
              slots={15}
              max={15}
              onOpen={(id) => setViewer({ id, list: deck.extra })}
              info={extraTypes
                .filter(([, n]) => n > 0)
                .map(([t, n]) => (
                  <span key={t}>
                    {t} {n}
                  </span>
                ))}
            />
            <DeckZone {...zoneProps} zone="side" title="Side Deck" ids={deck.side} columns={15} slots={15} max={15} onOpen={(id) => setViewer({ id, list: deck.side })} />
            <DeckZone
              {...zoneProps}
              zone="maybe"
              title="Maybe board"
              ids={deck.maybe}
              columns={15}
              slots={15}
              max={ZONE_LIMITS.maybe}
              invalid={NONE}
              dashed
              onOpen={(id) => setViewer({ id, list: deck.maybe })}
              info={<span>Carte in prova: non contano nel deck e non si esportano</span>}
              emptyHint={<p className="max-w-md text-xs text-faint">Trascina qui le carte che stai valutando.</p>}
            />
          </div>
        </div>

        <Catalog
          deck={deck}
          getCard={getCard}
          requestCards={cards.request}
          version={cards.version}
          onAdd={(id, _el, toSide) => tryAdd(deckRef.current, getCard(id), toSide ? "side" : null)}
          onOpen={(id, list) => setViewer({ id, list })}
          onHover={startHover}
          onHoverEnd={endHover}
          query={query}
          onQuery={setQuery}
        />
      </div>

      {hover && hoverCard && !viewer && <CardPreview card={hoverCard} anchor={hover.rect} />}
      {viewer && viewerCard && (
        <CardViewer
          card={viewerCard}
          position={`${viewerIndex + 1} / ${viewer.list.length}`}
          onClose={() => setViewer(null)}
          onPrev={viewerIndex > 0 ? () => setViewer({ ...viewer, id: viewer.list[viewerIndex - 1] }) : undefined}
          onNext={viewerIndex < viewer.list.length - 1 ? () => setViewer({ ...viewer, id: viewer.list[viewerIndex + 1] }) : undefined}
          onSearch={(q) => {
            setViewer(null);
            setQuery(q);
          }}
        />
      )}
      {dialog === "import" && <ImportDialog onClose={() => setDialog(null)} onImported={(r, n) => void imported(r, n)} />}
      {dialog === "export" && <ExportDialog deck={{ ...deck, id: idRef.current }} onClose={() => setDialog(null)} />}
      {dialog === "image" && <ImageDialog deck={deck} getCard={getCard} legal={legality?.legal ?? null} onClose={() => setDialog(null)} />}
      {dialog === "hand" && <TestHand deck={deck} getCard={getCard} onRoles={setRoles} onClose={() => setDialog(null)} />}
    </div>
  );
}
