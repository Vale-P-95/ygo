import { useCallback, useEffect, useRef, useState } from "react";
import { BarChart2, Calculator, GitCompare, Network, Play, ScrollText, Shield, SlidersHorizontal, WandSparkles, type LucideIcon } from "lucide-react";
import { Popover, useOutsideClose } from "../components/Popover";
import { CalculatorTab } from "../components/lab/CalculatorTab";
import { CompareTab } from "../components/lab/CompareTab";
import { ConditionPicker } from "../components/lab/ConditionPicker";
import { LinksTab } from "../components/lab/links/LinksTab";
import { LogTab } from "../components/lab/LogTab";
import { MatchEntryPanel } from "../components/lab/MatchEntryPanel";
import { OptimizerTab } from "../components/lab/OptimizerTab";
import { OverviewTab } from "../components/lab/OverviewTab";
import { RolesPanel } from "../components/lab/RolesPanel";
import { SimulatorTab } from "../components/lab/SimulatorTab";
import { SpecialTab } from "../components/lab/SpecialTab";
import { TechTab } from "../components/lab/TechTab";
import { DeckMenu } from "../components/DeckMenu";
import { SectionHeader } from "../components/SectionHeader";
import { api, type Deck, type DeckLinks, type DeckSummary, type LabOverview, type MatchLog, type Recipe } from "../lib/api";
import { useCardCache } from "../lib/cards";
import { BUILTIN_RECIPES, fromCond, toCond, type Group } from "../lib/conditions";
import { BUILTIN_ROLES } from "../lib/roles";
import { lastDeck, rememberDeck, sortIds } from "../lib/deck";
import { sound } from "../lib/sound";
import { toast } from "../lib/toast";
import { sectionById } from "../sections";

type Tab = "overview" | "links" | "compare" | "calculator" | "special" | "simulator" | "optimizer" | "meta" | "log";

const PRIMARY_TABS: { id: Tab; label: string; icon: LucideIcon }[] = [
  { id: "overview", label: "Panoramica", icon: BarChart2 },
  { id: "links", label: "Collegamenti", icon: Network },
  { id: "compare", label: "Confronto", icon: GitCompare },
  { id: "calculator", label: "Calcolatore", icon: Calculator },
  { id: "special", label: "Carte speciali", icon: WandSparkles },
];

const SECONDARY_TABS: { id: Tab; label: string; icon: LucideIcon }[] = [
  { id: "simulator", label: "Simulatore", icon: Play },
  { id: "optimizer", label: "Ottimizzatore", icon: SlidersHorizontal },
  { id: "meta", label: "Contro il meta", icon: Shield },
  { id: "log", label: "Registro partite", icon: ScrollText },
];

/** Il Laboratorio: statistics on a deck, built on the roles of its cards. */
export function Lab({
  artReady,
  onGoForge,
  onGoCombo,
}: {
  artReady: boolean;
  onGoForge: () => void;
  onGoCombo?: (cardId?: number) => void;
}) {
  const [decks, setDecks] = useState<DeckSummary[] | null>(null);
  const [deck, setDeck] = useState<Deck | null>(null);
  const [roles, setRoles] = useState<Record<string, number[]>>({});
  const [guessing, setGuessing] = useState<{ done: number; total: number } | null>(null);
  const [reasons, setReasons] = useState<Record<number, string[]>>({});
  const [previousRoles, setPreviousRoles] = useState<Record<string, number[]> | null>(null);
  const [overview, setOverview] = useState<LabOverview | null>(null);
  const [hand, setHand] = useState<"first" | "second">("first");
  const [tab, setTab] = useState<Tab>("overview");
  const [moreMenu, setMoreMenu] = useState(false);
  const moreRef = useOutsideClose(moreMenu, () => setMoreMenu(false));
  const activeSecondary = SECONDARY_TABS.find((t) => t.id === tab);
  // The condition is shared by Calcolatore, Simulatore and Ottimizzatore.
  const [root, setRootState] = useState<Group>(() => fromCond(BUILTIN_RECIPES[2].cond));
  const setRoot = useCallback((update: (g: Group) => Group) => setRootState((r) => update(structuredClone(r))), []);
  const [recipes, setRecipesState] = useState<Recipe[]>([]);
  const setRecipes = useCallback((update: (list: Recipe[]) => Recipe[]) => setRecipesState(update), []);
  const [log, setLog] = useState<MatchLog | null>(null);
  const [logScope, setLogScope] = useState<"deck" | "all">("deck");
  const saveTimer = useRef<number>(undefined);
  // The deck open now and its roles, for answers that arrive later (an
  // analysis started on another deck must not land on this one).
  const deckIdRef = useRef<number | undefined>(undefined);
  const rolesRef = useRef(roles);
  rolesRef.current = roles;
  const cards = useCardCache();
  const { request } = cards;

  const [linksData, setLinksData] = useState<DeckLinks | null>(null);
  const [linksLoading, setLinksLoading] = useState(false);
  const [linksProgress, setLinksProgress] = useState<{ done: number; total: number } | null>(null);
  const [linksError, setLinksError] = useState<string | null>(null);
  const linksReqIdRef = useRef(0);

  const loadLinks = useCallback(
    async (targetDeckId: number) => {
      const reqId = ++linksReqIdRef.current;
      setLinksLoading(true);
      setLinksError(null);
      setLinksData(null);
      setLinksProgress(null);

      let unlisten: (() => void) | undefined;
      try {
        unlisten = await api.onLinksProgress((p) => {
          if (linksReqIdRef.current === reqId) {
            if (p.deckId == null || p.deckId === targetDeckId) {
              setLinksProgress({ done: p.done, total: p.total });
              if (p.map && deckIdRef.current === targetDeckId) {
                setLinksData(p.map);
                request(p.map.cards.map((c) => c.code));
              }
            }
          }
        });

        const res = await api.deckLinks(targetDeckId);
        if (linksReqIdRef.current === reqId && deckIdRef.current === res.deckId) {
          setLinksData(res);
          request(res.cards.map((c) => c.code));
        }
      } catch (err) {
        if (linksReqIdRef.current === reqId && deckIdRef.current === targetDeckId) {
          setLinksError(String(err));
        }
      } finally {
        unlisten?.();
        if (linksReqIdRef.current === reqId) {
          setLinksLoading(false);
          setLinksProgress(null);
        }
      }
    },
    [request],
  );

  const open = useCallback(
    async (id: number) => {
      const d = await api.getDeck(id);
      if (!d) return;
      rememberDeck(id);
      deckIdRef.current = id;
      linksReqIdRef.current++;
      setLinksLoading(false);
      // The last analysis belongs to the deck left.
      setPreviousRoles(null);
      setReasons({});
      setLinksData(null);
      setLinksError(null);
      setLinksProgress(null);
      setDeck(d);
      api.listRecipes(id).then(setRecipesState).catch(() => setRecipesState([]));
      setRoles(d.roles ?? {});
      request([...new Set(d.main)]);
    },
    [request],
  );

  useEffect(() => {
    api.listDecks().then((list) => {
      setDecks(list);
      const id = list.find((d) => d.id === lastDeck())?.id ?? list[0]?.id;
      if (id != null) void open(id);
    });
  }, [open]);

  useEffect(() => {
    if (!deck) return;
    let cancelled = false;
    api
      .labOverview(deck.main, roles)
      .then((o) => !cancelled && setOverview(o))
      .catch((e) => !cancelled && toast(`Panoramica non calcolata: ${e}`, "error"));
    return () => {
      cancelled = true;
    };
  }, [deck, roles]);

  const deckId = deck?.id;
  const updatedAt = deck?.updatedAt;
  const reloadLog = useCallback(() => {
    if (deckId == null) return;
    api
      .matchLog(logScope === "deck" ? deckId : null)
      .then(setLog)
      .catch((e) => toast(`Registro non letto: ${e}`, "error"));
  }, [deckId, logScope]);

  // The log also knows the deck's current version, so it reloads after every save.
  useEffect(() => {
    if (tab === "log") reloadLog();
  }, [tab, reloadLog, updatedAt]);

  useEffect(() => {
    if (tab === "links" && deck?.id != null && (!linksData || linksData.deckId !== deck.id) && !linksLoading && !linksError) {
      void loadLinks(deck.id);
    }
  }, [tab, deck?.id, linksData, linksLoading, linksError, loadLinks]);

  const changeRoles = (next: Record<string, number[]>, id = deck?.id) => {
    setRoles(next);
    if (id == null) return;
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      api.setDeckRoles(id, next).catch((e) => toast(`Ruoli non salvati: ${e}`, "error"));
    }, 400);
  };

  // The engine plays each card to find its role: the built-in roles of
  // the cards it tried are replaced (the previous ones can be restored),
  // custom roles stay.
  const suggest = async () => {
    if (deck?.id == null || guessing) return;
    const id = deck.id;
    setGuessing({ done: 0, total: 0 });
    const unlisten = await api.onRolesProgress((p) => setGuessing(p));
    try {
      const guesses = await api.guessDeckRoles(id);
      if (deckIdRef.current !== id) {
        toast("Analisi dei ruoli scartata: nel frattempo hai aperto un altro deck", "info");
        return;
      }
      // The roles as they are now (changed by hand meanwhile, too).
      const base = rolesRef.current;
      const tried = new Set(guesses.map((g) => g.code));
      const next: Record<string, number[]> = { ...base };
      for (const r of BUILTIN_ROLES) next[r.key] = (base[r.key] ?? []).filter((c) => !tried.has(c));
      for (const g of guesses) for (const r of g.roles) next[r] = [...(next[r] ?? []), g.code];
      setPreviousRoles(base);
      setReasons(Object.fromEntries(guesses.map((g) => [g.code, g.reasons])));
      changeRoles(next, id);
      sound.play("glass");
      const count = (key: string) => guesses.filter((g) => g.roles.includes(key)).length;
      toast(`Ruoli riconosciuti: ${count("starter")} starter, ${count("extender")} estensori, ${count("handtrap")} handtrap, ${count("breaker")} board breaker, ${count("brick")} brick`, "ok");
    } catch (e) {
      toast(`Ruoli non riconosciuti: ${e}`, "error");
    } finally {
      unlisten();
      setGuessing(null);
    }
  };

  const undoSuggest = () => {
    if (!previousRoles) return;
    changeRoles(previousRoles);
    setPreviousRoles(null);
    setReasons({});
    sound.play("toggle");
  };

  const applyChanges = async (changes: { id: number; from: number; to: number }[]) => {
    if (!deck) return;
    let main = [...deck.main];
    let maybe = [...deck.maybe];
    for (const c of changes) {
      const delta = c.to - c.from;
      for (let i = 0; i < Math.abs(delta); i++) {
        if (delta > 0) {
          main.push(c.id);
          const at = maybe.indexOf(c.id);
          if (at >= 0) maybe.splice(at, 1);
        } else {
          main.splice(main.indexOf(c.id), 1);
          maybe.push(c.id);
        }
      }
    }
    main = sortIds(main, cards.get);
    maybe = sortIds(maybe, cards.get);
    try {
      const saved = await api.saveDeck({ ...deck, main, maybe, roles });
      setDeck(saved);
      linksReqIdRef.current++;
      setLinksLoading(false);
      setLinksData(null);
      setLinksError(null);
      setLinksProgress(null);
      sound.play("forge");
      toast("Deck aggiornato: le copie tolte sono nella Maybe board", "ok");
    } catch (e) {
      sound.play("error");
      toast(`Deck non aggiornato: ${e}`, "error");
    }
  };

  const exploreCombo = useCallback(
    (cardId: number) => {
      if (!deck?.id) return;
      let existing: Record<string, unknown> = {};
      try {
        const raw = localStorage.getItem(`duelforge.combo.${deck.id}`);
        if (raw) existing = JSON.parse(raw);
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
      localStorage.setItem(`duelforge.combo.${deck.id}`, JSON.stringify(nextSetup));
      rememberDeck(deck.id);
      sound.play("forge");
      onGoCombo?.(cardId);
    },
    [deck?.id, onGoCombo],
  );

  const exploreComboAsGoal = useCallback(
    (targetGoalCardId: number, startCardId?: number) => {
      if (!deck?.id) return;
      let existing: Record<string, unknown> = {};
      try {
        const raw = localStorage.getItem(`duelforge.combo.${deck.id}`);
        if (raw) existing = JSON.parse(raw);
      } catch {
        existing = {};
      }
      const nextSetup = {
        mode: startCardId ? ("card" as const) : ((existing.mode as "card" | "hand") ?? ("card" as const)),
        hand: Array.isArray(existing.hand) ? existing.hand : [],
        card: startCardId ?? (typeof existing.card === "number" ? existing.card : null),
        keep: Array.isArray(existing.keep) ? existing.keep : [],
        forbid: Array.isArray(existing.forbid) ? existing.forbid : [],
        goal: [{ code: targetGoalCardId, zone: "any" }],
        goalAll: true,
        goalFirst: typeof existing.goalFirst === "boolean" ? existing.goalFirst : true,
      };
      localStorage.setItem(`duelforge.combo.${deck.id}`, JSON.stringify(nextSetup));
      rememberDeck(deck.id);
      sound.play("forge");
      onGoCombo?.(targetGoalCardId);
    },
    [deck?.id, onGoCombo],
  );

  const cond = toCond(root);
  const section = sectionById("stats");

  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={artReady} />
      <div className="relative z-20 flex flex-wrap items-center gap-2 border-b border-border bg-surface/70 px-5 py-2.5 backdrop-blur 2xl:gap-3 2xl:px-6">
        <DeckMenu decks={decks} current={deck?.id} onPick={(id) => void open(id)} />
        <div className={`flex gap-1 rounded-xl border border-border bg-bg-2 p-1 ${tab === "log" ? "invisible" : ""}`}>
          {(["first", "second"] as const).map((h) => (
            <button
              key={h}
              type="button"
              onClick={() => {
                sound.play("toggle");
                setHand(h);
              }}
              className={`rounded-lg px-2.5 py-1 text-xs font-semibold ${hand === h ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
            >
              {h === "first" ? "Primo · 5" : "Secondo · 6"}
            </button>
          ))}
        </div>
        <nav className="flex items-center gap-1" aria-label="Schede del laboratorio">
          {PRIMARY_TABS.map((t) => {
            const Icon = t.icon;
            return (
              <button
                key={t.id}
                type="button"
                onClick={() => {
                  sound.play("page");
                  setTab(t.id);
                }}
                className={`flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 2xl:px-3 text-sm ${
                  tab === t.id
                    ? "bg-surface-3 text-text shadow-[inset_0_-2px_0_var(--sec)] font-semibold"
                    : "text-muted hover:text-text"
                }`}
              >
                <Icon size={15} className={tab === t.id ? "text-sec" : "text-muted"} />
                <span>{t.label}</span>
              </button>
            );
          })}

          <div ref={moreRef} className="relative">
            <button
              type="button"
              aria-expanded={moreMenu}
              onClick={() => {
                sound.play(moreMenu ? "close" : "open");
                setMoreMenu((v) => !v);
              }}
              className={`flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 2xl:px-3 text-sm ${
                activeSecondary
                  ? "bg-surface-3 text-text shadow-[inset_0_-2px_0_var(--sec)] font-semibold"
                  : "text-muted hover:text-text"
              }`}
            >
              {activeSecondary ? (
                <>
                  <activeSecondary.icon size={15} className="text-sec" />
                  <span>{activeSecondary.label}</span>
                </>
              ) : (
                <span>Altro</span>
              )}
              <span className="text-faint text-xs">▾</span>
            </button>
            <Popover open={moreMenu} align="left" width={200}>
              {SECONDARY_TABS.map((t) => {
                const Icon = t.icon;
                return (
                  <button
                    key={t.id}
                    type="button"
                    onClick={() => {
                      setMoreMenu(false);
                      sound.play("page");
                      setTab(t.id);
                    }}
                    className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm ${
                      tab === t.id ? "bg-surface-3 text-sec font-semibold" : "text-text hover:bg-surface-3"
                    }`}
                  >
                    <Icon size={15} className={tab === t.id ? "text-sec" : "text-muted"} />
                    <span>{t.label}</span>
                  </button>
                );
              })}
            </Popover>
          </div>
        </nav>
        {overview && (
          <span className="ml-auto hidden text-xs whitespace-nowrap text-faint tabular-nums 2xl:inline" title="Carte nel Main Deck">
            {overview.size} carte
          </span>
        )}
      </div>

      {decks && decks.length === 0 ? (
        <div className="bg-blueprint flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <p className="display text-xl">Nessun deck da analizzare</p>
          <p className="text-sm text-muted">Il laboratorio lavora sui deck che costruisci nella Fucina.</p>
          <button type="button" className="btn btn-molten" onClick={onGoForge}>
            Vai alla Fucina
          </button>
        </div>
      ) : (
        <div className={tab === "links" || tab === "meta" ? "grid min-h-0 flex-1 grid-cols-1" : "grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_312px] xl:grid-cols-[minmax(0,1fr)_336px]"}>
          <div key={tab} className="bg-blueprint min-h-0 overflow-y-auto px-4 py-4 xl:px-6 xl:py-5">
            {!deck || !overview ? (
              <p className="text-sm text-muted">Carico il deck…</p>
            ) : tab === "overview" ? (
              <OverviewTab overview={overview} hand={hand} roles={roles} getCard={cards.get} onExploreCombo={onGoCombo ? exploreCombo : undefined} />
            ) : tab === "links" ? (
              <LinksTab
                key={deck.id}
                deck={deck}
                data={linksData}
                loading={linksLoading}
                progress={linksProgress}
                error={linksError}
                onReload={() => deck.id != null && void loadLinks(deck.id)}
                decks={decks ?? []}
                requestCards={request}
                getCard={cards.get}
                onGoCombo={onGoCombo ? exploreComboAsGoal : undefined}
                hand={hand}
              />
            ) : tab === "compare" ? (
              <CompareTab key={deck.id} deck={deck} overview={overview} decks={decks ?? []} hand={hand} getCard={cards.get} request={request} />
            ) : tab === "calculator" ? (
              <CalculatorTab deckId={deck.id!} main={deck.main} roles={roles} hand={hand} getCard={cards.get} root={root} setRoot={setRoot} recipes={recipes} setRecipes={setRecipes} />
            ) : tab === "special" ? (
              <SpecialTab key={deck.id} deck={deck} roles={roles} hand={hand} getCard={cards.get} request={request} />
            ) : tab === "meta" ? (
              <TechTab key={deck.id} deck={deck} getCard={cards.get} request={request} />
            ) : tab === "simulator" || tab === "optimizer" ? (
              <div className="flex flex-col gap-4">
                <ConditionPicker root={root} setRoot={setRoot} recipes={recipes} getCard={cards.get} onEdit={() => setTab("calculator")} />
                {tab === "simulator" ? (
                  <SimulatorTab key={deck.id} deckId={deck.id!} main={deck.main} roles={roles} cond={cond} hand={hand} getCard={cards.get} />
                ) : (
                  <OptimizerTab deck={deck} roles={roles} cond={cond} hand={hand} getCard={cards.get} onApply={(c) => void applyChanges(c)} />
                )}
              </div>
            ) : (
              <LogTab
                deck={deck}
                log={log}
                scope={logScope}
                setScope={(s) => {
                  setLog(null);
                  setLogScope(s);
                }}
                getCard={cards.get}
                onChanged={reloadLog}
              />
            )}
          </div>
          {deck &&
            (tab === "log" ? (
              <MatchEntryPanel
                deck={deck}
                log={log}
                getCard={cards.get}
                onLogged={reloadLog}
              />
            ) : tab === "links" || tab === "meta" ? null : (
              <RolesPanel
                main={deck.main}
                roles={roles}
                getCard={cards.get}
                onChange={changeRoles}
                onSuggest={() => void suggest()}
                guessing={guessing}
                reasons={reasons}
                onUndo={previousRoles ? undoSuggest : null}
                onExploreCombo={onGoCombo ? exploreCombo : undefined}
              />
            ))}
        </div>
      )}
    </div>
  );
}
