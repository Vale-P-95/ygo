import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { CardPreview } from "../components/CardPreview";
import { CardViewer } from "../components/CardViewer";
import { Catalog } from "../components/Catalog";
import { DataSetup } from "../components/DataSetup";
import { DeckInsightsBar } from "../components/DeckInsightsBar";
import { ExportDialog, ImageDialog, ImportDialog } from "../components/DeckIO";
import { DeckToolbar } from "../components/DeckToolbar";
import { DeckZone } from "../components/DeckZone";
import { SectionHeader } from "../components/SectionHeader";
import { TestHand } from "../components/TestHand";
import { api, cardImage, type Card, type DataStatus, type Deck, type DeckSummary, type Imported, type Legality } from "../lib/api";
import { useCardCache } from "../lib/cards";
import {
  addCard,
  counts,
  craftCost,
  emptyDeck,
  lastDeck,
  rememberDeck,
  removeAt,
  removeCard,
  sortIds,
  takeNewDeckRequest,
  useDeckHistory,
  ZONE_LIMITS,
  type Zone,
} from "../lib/deck";
import { setDropHandler, type DragSource, type DropZone } from "../lib/drag";
import { BUILTIN_ROLES } from "../lib/roles";
import { useSettings } from "../lib/settings";
import { sound } from "../lib/sound";
import { burstFrom } from "../lib/sparks";
import { toast } from "../lib/toast";
import { sectionById } from "../sections";

const NONE = new Set<number>();

export function DeckBuilder({
  status,
  onStatus,
  onGoCombo,
}: {
  status: DataStatus | null;
  onStatus: (s: DataStatus) => void;
  onGoCombo?: () => void;
}) {
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
  return <DeckEditor onGoCombo={onGoCombo} />;
}

function Dot({ color }: { color: string }) {
  return <span className="inline-block h-2 w-2 rounded-full" style={{ background: color }} />;
}

function DeckEditor({ onGoCombo }: { onGoCombo?: () => void }) {
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
  const [activeRole, setActiveRole] = useState<string | null>(null);
  const [guessing, setGuessing] = useState<{ done: number; total: number } | null>(null);
  const [previousRoles, setPreviousRoles] = useState<Record<string, number[]> | null>(null);
  const getCard = cards.get;
  const viewerCard = viewer ? getCard(viewer.id) : undefined;
  const viewerIndex = viewer ? viewer.list.indexOf(viewer.id) : -1;

  const refreshDecks = useCallback(() => api.listDecks().then((list) => (setDecks(list), list)), []);

  const openDeck = useCallback(
    async (id: number) => {
      const stored = await api.getDeck(id);
      if (!stored) return;
      dirty.current = false;
      idRef.current = stored.id;
      rememberDeck(stored.id);
      reset(stored);
      setActiveRole(null);
      setPreviousRoles(null);
      setSaveState("saved");
      sound.play("page");
    },
    [reset],
  );

  useEffect(() => {
    // Coming from the shelf's "new deck": keep the empty deck.
    const fresh = takeNewDeckRequest();
    if (fresh) rememberDeck(null);
    refreshDecks().then((list) => {
      if (fresh) return;
      const id = list.find((d) => d.id === lastDeck())?.id ?? list[0]?.id;
      if (id != null) void openDeck(id);
    });
  }, [refreshDecks, openDeck]);

  useEffect(() => {
    cards.request([...deck.main, ...deck.extra, ...deck.side, ...deck.maybe]);
  }, [deck, cards.request]);

  useEffect(() => {
    if (viewer) cards.request([viewer.id]);
  }, [viewer?.id, cards.request]);

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

  /** Flushes any unsaved deck changes immediately so backend operations have the latest deck. */
  const ensureSaved = useCallback(async (): Promise<number | null> => {
    const current = deckRef.current;
    const empty = current.main.length + current.extra.length + current.side.length + current.maybe.length === 0;
    if (idRef.current == null && empty) return null;
    if (idRef.current == null || dirty.current) {
      setSaveState("saving");
      try {
        const saved = await api.saveDeck({ ...current, id: idRef.current });
        idRef.current = saved.id;
        rememberDeck(saved.id);
        dirty.current = false;
        setSaveState("saved");
        void refreshDecks();
        return saved.id;
      } catch (e) {
        setSaveState("idle");
        toast(`Salvataggio non riuscito: ${e}`, "error");
        return null;
      }
    }
    return idRef.current;
  }, [refreshDecks]);

  const update = useCallback(
    (next: Deck, record = true) => {
      dirty.current = true;
      setHistory(next, record);
    },
    [setHistory],
  );

  const setRoles = useCallback((roles: Record<string, number[]>) => update({ ...deckRef.current, roles }, false), [update]);

  const suggestRoles = useCallback(async () => {
    if (guessing) return;
    const id = await ensureSaved();
    if (id == null) return;
    setGuessing({ done: 0, total: 0 });
    const unlisten = await api.onRolesProgress((p) => setGuessing(p));
    try {
      const guesses = await api.guessDeckRoles(id);
      if (idRef.current !== id) {
        toast("Analisi dei ruoli scartata: nel frattempo hai aperto un altro deck", "info");
        return;
      }
      const base = deckRef.current.roles ?? {};
      const tried = new Set(guesses.map((g) => g.code));
      const next: Record<string, number[]> = { ...base };
      for (const r of BUILTIN_ROLES) next[r.key] = (base[r.key] ?? []).filter((c) => !tried.has(c));
      for (const g of guesses) for (const r of g.roles) next[r] = [...(next[r] ?? []), g.code];
      setPreviousRoles(base);
      setHistory({ ...deckRef.current, roles: next }, false);
      await api.setDeckRoles(id, next);
      sound.play("glass");
      const count = (key: string) => guesses.filter((g) => g.roles.includes(key)).length;
      toast(
        `Ruoli riconosciuti: ${count("starter")} starter, ${count("extender")} estensori, ${count("handtrap")} handtrap, ${count("breaker")} board breaker, ${count("brick")} brick`,
        "ok",
      );
    } catch (e) {
      toast(`Ruoli non riconosciuti: ${e}`, "error");
    } finally {
      unlisten();
      setGuessing(null);
    }
  }, [ensureSaved, guessing, setHistory]);

  const undoRoles = useCallback(() => {
    if (!previousRoles) return;
    update({ ...deckRef.current, roles: previousRoles }, false);
    setPreviousRoles(null);
    sound.play("toggle");
  }, [previousRoles, update]);

  const tryHandInCombo = useCallback(
    async (handCards: number[]) => {
      const id = await ensureSaved();
      if (id == null) return;
      const handForCombo = handCards.slice(0, 6);
      let existing: Record<string, unknown> = {};
      try {
        const raw = localStorage.getItem(`duelforge.combo.${id}`);
        if (raw) existing = JSON.parse(raw) as Record<string, unknown>;
      } catch {
        existing = {};
      }
      const prevKeep = Array.isArray(existing.keep) ? (existing.keep as number[]) : [];
      const nextSetup = {
        mode: "hand" as const,
        hand: handForCombo,
        card: typeof existing.card === "number" ? existing.card : (handForCombo[0] ?? null),
        keep: prevKeep.filter((c) => handForCombo.includes(c)),
        forbid: Array.isArray(existing.forbid) ? existing.forbid : [],
        goal: Array.isArray(existing.goal) ? existing.goal : [],
        goalAll: typeof existing.goalAll === "boolean" ? existing.goalAll : true,
        goalFirst: typeof existing.goalFirst === "boolean" ? existing.goalFirst : true,
      };
      localStorage.setItem(`duelforge.combo.${id}`, JSON.stringify(nextSetup));
      rememberDeck(id);
      setDialog(null);
      sound.play("forge");
      onGoCombo?.();
    },
    [ensureSaved, onGoCombo],
  );

  const tryCardInCombo = useCallback(
    async (cardId: number) => {
      const id = await ensureSaved();
      if (id == null) return;
      let existing: Record<string, unknown> = {};
      try {
        const raw = localStorage.getItem(`duelforge.combo.${id}`);
        if (raw) existing = JSON.parse(raw) as Record<string, unknown>;
      } catch {
        existing = {};
      }
      const nextSetup = {
        mode: "card" as const,
        hand: Array.isArray(existing.hand) ? existing.hand : [],
        card: cardId,
        keep: Array.isArray(existing.keep) ? existing.keep : [],
        forbid: Array.isArray(existing.forbid) ? existing.forbid : [],
        goal: Array.isArray(existing.goal) ? existing.goal : [],
        goalAll: typeof existing.goalAll === "boolean" ? existing.goalAll : true,
        goalFirst: typeof existing.goalFirst === "boolean" ? existing.goalFirst : true,
      };
      localStorage.setItem(`duelforge.combo.${id}`, JSON.stringify(nextSetup));
      rememberDeck(id);
      setViewer(null);
      sound.play("forge");
      onGoCombo?.();
    },
    [ensureSaved, onGoCombo],
  );

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

  const handleViewerAdd = useCallback(
    (zone: Zone) => {
      if (!viewerCard) return;
      tryAdd(deckRef.current, viewerCard, zone);
    },
    [viewerCard, tryAdd],
  );

  const handleViewerRemove = useCallback(
    (zone: Zone) => {
      if (!viewerCard) return;
      const current = deckRef.current;
      const next = removeCard(current, viewerCard.id, zone);
      if (next !== current) {
        sound.play("close");
        update(next);
      }
    },
    [viewerCard, update],
  );

  const toggleViewerRole = useCallback(
    (roleKey: string) => {
      if (!viewerCard) return;
      const currentRoles = deckRef.current.roles ?? {};
      const list = currentRoles[roleKey] ?? [];
      const on = list.includes(viewerCard.id);
      sound.play(on ? "toggle" : "select");
      const nextList = on ? list.filter((id) => id !== viewerCard.id) : [...list, viewerCard.id];
      setRoles({ ...currentRoles, [roleKey]: nextList });
    },
    [viewerCard, setRoles],
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
    setActiveRole(null);
    setPreviousRoles(null);
    setSaveState("idle");
  };

  const imported = async (result: Imported, name: string) => {
    setDialog(null);
    const all = [...result.main, ...result.extra, ...result.side];
    const data = new Map((await api.getCards([...new Set(all)])).map((c) => [c.id, c]));
    const lookup = (id: number) => data.get(id);
    idRef.current = null;
    dirty.current = true;
    setActiveRole(null);
    setPreviousRoles(null);
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
  const highlighted = useMemo(() => {
    if (!activeRole) return null;
    return new Set(deck.roles?.[activeRole] ?? []);
  }, [activeRole, deck.roles]);
  const main = counts(deck.main, getCard);
  const extraTypes = ["Fusion", "Synchro", "Xyz", "Link"].map((t) => [t, deck.extra.filter((id) => getCard(id)?.typeLine.includes(t)).length] as const);
  const hoverCard = hover ? getCard(hover.id) : undefined;
  const themeArt = sectionById("deck-builder").themeCard!;

  const zoneProps = { format: deck.format, getCard, invalid, fresh, highlighted, onRemove: remove, onHover: startHover, onHoverEnd: endHover };

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
          <div className="relative mx-auto flex max-w-[1180px] flex-col gap-4">
            <DeckInsightsBar
              deck={deck}
              getCard={getCard}
              activeRole={activeRole}
              onSelectRole={setActiveRole}
              guessing={guessing}
              onSuggestRoles={() => void suggestRoles()}
              onUndoRoles={previousRoles ? undoRoles : null}
            />
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
          deck={deck}
          onAddCard={handleViewerAdd}
          onRemoveCard={handleViewerRemove}
          onToggleRole={toggleViewerRole}
          onExploreCombo={onGoCombo ? (id) => void tryCardInCombo(id) : undefined}
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
      {dialog === "hand" && (
        <TestHand
          deck={deck}
          getCard={getCard}
          onRoles={setRoles}
          guessing={guessing}
          onSuggestRoles={() => void suggestRoles()}
          onTryInCombo={onGoCombo ? (h) => void tryHandInCombo(h) : undefined}
          onClose={() => setDialog(null)}
        />
      )}
    </div>
  );
}
