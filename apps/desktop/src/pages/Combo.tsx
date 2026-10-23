import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BoardCompareModal } from "../components/combo/BoardCompareModal";
import { BoardDetail } from "../components/combo/BoardDetail";
import { BoardStrip, CardImage } from "../components/combo/Field";
import { plural, ScoreBar, WeightsPanel } from "../components/combo/Score";
import { CardPreview } from "../components/CardPreview";
import { CardTile } from "../components/CardTile";
import { DeckMenu } from "../components/DeckMenu";
import { SectionHeader } from "../components/SectionHeader";
import { api, type ComboBoard, type ComboGoal, type ComboHand, type ComboScore, type ComboStep, type GoalCard, type GoalZone, type Deck, type DeckSummary, type ZoneCard } from "../lib/api";
import { useCardCache } from "../lib/cards";
import { clearMetaError, compareMeta, continueCombo, scoreBoards, startCombo, stopCombo, useCombo } from "../lib/combo";
import { dominates, score, useWeights } from "../lib/scoring";
import { lastDeck, rememberDeck } from "../lib/deck";
import { sound } from "../lib/sound";
import { toast } from "../lib/toast";
import { sectionById } from "../sections";

type Mode = "hand" | "card";

/** What the player sets up for a deck; remembered per deck. */
interface Setup {
  mode: Mode;
  hand: number[];
  card: number | null;
  /** Cards that must stay in the hand at the end. */
  keep: number[];
  /** Cards the engine must not use. */
  forbid: number[];
  /** Cards wanted on the field at the end (the goal), all or one of them. */
  goal: GoalCard[];
  goalAll: boolean;
  /** Stop at the first line that reaches the goal. */
  goalFirst: boolean;
}

const NO_SETUP: Setup = { mode: "hand", hand: [], card: null, keep: [], forbid: [], goal: [], goalAll: true, goalFirst: true };

const BUDGETS = [
  { seconds: 30, label: "30 s" },
  { seconds: 60, label: "1 min" },
  { seconds: 180, label: "3 min" },
  { seconds: 600, label: "10 min" },
];

const HAND_COUNTS = [6, 12, 24];

const STOPS: Record<string, string> = {
  Exhausted: "tutte le linee esplorate",
  TimeLimit: "tempo scaduto",
  NodeLimit: "limite di scelte raggiunto",
  Cancelled: "fermata",
  Finished: "tutte le mani provate",
  GoalReached: "obiettivo raggiunto",
};

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...(JSON.parse(raw) as T) } : fallback;
  } catch {
    return fallback;
  }
}

function store(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Only a convenience.
  }
}

const boardId = (b: ComboBoard) => `${b.hand}:${b.key}`;
const pairKey = (id1: string, id2: string) => (id1 < id2 ? `${id1}<->${id2}` : `${id2}<->${id1}`);

const ZONES: { zone: GoalZone; label: string; hint: string }[] = [
  { zone: "any", label: "ovunque", hint: "in qualsiasi zona" },
  { zone: "extra", label: "Zona Extra", hint: "nella Zona Mostri Extra" },
  { zone: "main", label: "zona principale", hint: "in una Zona Mostri principale" },
];

/** A goal as saved before zones existed (plain codes) still loads. */
function goalCards(saved: unknown): GoalCard[] {
  if (!Array.isArray(saved)) return [];
  return saved.map((g) => (typeof g === "number" ? { code: g, zone: "any" as GoalZone } : (g as GoalCard)));
}

/**
 * What the player sees of a board: the field (with the Extra Monster Zone
 * apart, since some monsters work differently there) and the hand; the
 * piles and the main zones may differ.
 */
function lookKey(b: ComboBoard): string {
  const zone = (z: ZoneCard | null, i: number) => (z ? `${i >= 5 ? "E" : ""}${z.code}${z.faceUp ? "u" : "d"}${z.defense ? "d" : "a"}${z.overlay.length}` : "");
  const sorted = (list: string[]) => list.filter(Boolean).sort().join(",");
  return [b.hand, sorted(b.monsters.map(zone)), sorted(b.spells.slice(0, 6).map((z) => zone(z, 0))), sorted(b.handCards.map(String))].join("|");
}

const byRank = (a: ComboBoard, b: ComboBoard) => b.goalHits - a.goalHits || b.quickScore - a.quickScore || a.cardsUsed - b.cardsUsed || a.choices - b.choices;

/** Whether a board has the goal's cards (all of them, or one). */
const reaches = (b: ComboBoard, goal: ComboGoal | null) => !!goal && (goal.all ? b.goalHits >= goal.cards.length : b.goalHits > 0);

function duration(ms: number): string {
  const s = Math.round(ms / 1000);
  return s < 60 ? `${s} s` : `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, "0")} s`;
}

const count = (n: number) => n.toLocaleString("it-IT");

function LockIcon({ closed }: { closed: boolean }) {
  return (
    <svg viewBox="0 0 16 16" className="h-3 w-3" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden>
      <rect x="3" y="7" width="10" height="7" rx="1.5" fill={closed ? "currentColor" : "none"} />
      <path d={closed ? "M5.5 7V5a2.5 2.5 0 0 1 5 0v2" : "M5.5 7V5a2.5 2.5 0 0 1 5 0"} />
    </svg>
  );
}

function GoalMark({ status }: { status: boolean | null }) {
  return status === true ? (
    <span className="text-[11px] font-semibold text-ok" title="Obiettivo raggiunto">
      ✓
    </span>
  ) : status === false ? (
    <span className="text-[11px] text-danger" title="Impossibile: provate tutte le linee">
      ✕
    </span>
  ) : (
    <span className="text-[11px] text-faint" title="Non ancora deciso">
      ?
    </span>
  );
}

/** What the search says about the goal: reached (with its line), impossible, or not known yet. */
function GoalBanner(props: {
  goal: ComboGoal;
  running: boolean;
  byCard: boolean;
  groups: { board: ComboBoard }[];
  hands: ComboHand[];
  elapsedMs: number;
  name: (id: number) => string;
  onShow: (b: ComboBoard) => void;
}) {
  const { goal, running, byCard, groups, hands, elapsedMs, name, onShow } = props;
  const names = goal.cards.map((g) => `${name(g.code)}${g.zone === "extra" ? " (Zona Extra)" : g.zone === "main" ? " (zona principale)" : ""}`);
  const wanted = names.length === 1 ? names[0] : goal.all ? names.join(" + ") : `una tra ${names.join(", ")}`;
  const best = groups.map((g) => g.board).find((b) => reaches(b, goal));
  let tone = "border-border";
  let title: string;
  let text: string;
  if (byCard) {
    const total = hands.reduce((t, h) => t + h.weight, 0) || 1;
    const reached = hands.filter((h) => h.goal === true);
    const impossible = hands.filter((h) => h.goal === false).length;
    const share = reached.reduce((t, h) => t + h.weight, 0) / total;
    title = running ? `Cerco ${wanted} in ogni mano…` : `${wanted}: raggiunto da ${reached.length} mani su ${hands.length}`;
    text = running
      ? "Ogni mano viene giocata a turno; l'esito compare accanto a ciascuna."
      : `Circa il ${Math.round(share * 100)}% delle mani con la carta ci arriva (pesato per quanto sono probabili). ${impossible} impossibili, ${hands.length - reached.length - impossible} non decise nel tempo dato.`;
    if (!running) tone = reached.length ? "!border-[var(--df-ok)]" : "border-border";
  } else if (best) {
    tone = "!border-[var(--df-ok)]";
    title = `Obiettivo raggiunto: ${wanted}`;
    text = `La linea migliore trovata usa ${best.cardsUsed} ${best.cardsUsed === 1 ? "carta" : "carte"} della mano in ${best.choices} scelte.`;
  } else if (running) {
    title = `Cerco una linea che porti ${wanted} sul campo…`;
    text = "Le linee che ci si avvicinano vengono provate per prime.";
  } else if (hands[0]?.goal === false) {
    tone = "!border-[var(--df-danger)]";
    title = "Impossibile da questa mano";
    text = `Il motore ha provato tutte le linee del primo turno: nessuna porta ${wanted} sul campo.`;
  } else {
    title = "Non ancora trovato";
    text = `In ${Math.round(elapsedMs / 1000)} s nessuna linea ci è arrivata, ma non le ho provate tutte: continua la ricerca per esserne certo.`;
  }
  return (
    <section className={`lab-card flex flex-wrap items-center gap-3 p-4 ${tone}`} aria-live="polite">
      <div className="min-w-0 flex-1">
        <h3 className="text-sm font-semibold">{title}</h3>
        <p className="mt-0.5 text-xs text-muted">{text}</p>
      </div>
      {best && !byCard && (
        <button type="button" className="btn btn-ghost !py-1 text-xs" onClick={() => onShow(best)}>
          Vedi la linea
        </button>
      )}
    </section>
  );
}

/** Il Circolo Arcano: the engine plays every line from a hand by itself and ranks the boards. */
export function Combo({ artReady, onGoForge }: { artReady: boolean; onGoForge: () => void }) {
  const [decks, setDecks] = useState<DeckSummary[] | null>(null);
  const [deck, setDeck] = useState<Deck | null>(null);
  const [setup, setSetupState] = useState<Setup>(NO_SETUP);
  const [options, setOptionsState] = useState(() => load("duelforge.combo.options", { seconds: 60, hands: 12, useHandtraps: false }));
  const [handtraps, setHandtraps] = useState<Set<number>>(new Set());
  const [selected, setSelected] = useState<ComboBoard | null>(null);
  const [pinned, setPinned] = useState<ComboBoard | null>(null);
  const [compareTarget, setCompareTarget] = useState<ComboBoard | null>(null);
  const [steps, setSteps] = useState<ComboStep[] | null>(null);
  const [handFilter, setHandFilter] = useState<number | null>(null);
  const [hover, setHover] = useState<{ code: number; rect: DOMRect; el: HTMLElement } | null>(null);
  const [picker, setPicker] = useState<"extra" | "main" | null>(null);
  const explained = useRef(new Map<string, ComboStep[]>());
  const run = useCombo();
  const { profile, weights } = useWeights();
  const [weightsOpen, setWeightsOpen] = useState(false);
  const [allDecks, setAllDecks] = useState<DeckSummary[]>([]);
  const [sortByMatchup, setSortByMatchup] = useState(false);
  const [paretoOnly, setParetoOnly] = useState(false);
  const [choicesVersion, setChoicesVersion] = useState(0);
  const [recordedPairChoices, setRecordedPairChoices] = useState<Map<string, string>>(new Map());
  const metaReqIdRef = useRef(0);

  useEffect(() => {
    void api.listDecks().then(setAllDecks);
  }, []);

  const rivalDecks = useMemo(() => allDecks.filter((d) => !deck || d.id !== deck.id), [allDecks, deck]);
  const cards = useCardCache();
  const { request } = cards;

  const setSetup = (update: (s: Setup) => Setup) =>
    setSetupState((s) => {
      const next = update(s);
      if (deck?.id != null) store(`duelforge.combo.${deck.id}`, next);
      return next;
    });
  const setOptions = (change: Partial<typeof options>) =>
    setOptionsState((o) => {
      const next = { ...o, ...change };
      store("duelforge.combo.options", next);
      return next;
    });

  const open = useCallback(
    async (id: number) => {
      const d = await api.getDeck(id);
      if (!d) return;
      rememberDeck(id);
      setDeck(d);
      clearMetaError();
      metaReqIdRef.current++;
      const saved = load(`duelforge.combo.${id}`, NO_SETUP);
      setSetupState({ ...saved, goal: goalCards(saved.goal) });
      setPicker(null);
      request([...new Set([...d.main, ...d.extra])]);
    },
    [request],
  );

  useEffect(() => {
    api.listDecks().then((list) => {
      setDecks(list);
      const id = list.find((d) => d.id === lastDeck())?.id ?? list[0]?.id;
      if (id != null) void open(id);
    });
    api
      .search("tag:handtrap")
      .then((r) => setHandtraps(new Set(r.ids)))
      .catch(() => {});
  }, [open]);

  const unique = useMemo(() => [...new Set(deck?.main ?? [])], [deck]);
  const extraUnique = useMemo(() => [...new Set(deck?.extra ?? [])], [deck]);
  const copies = useMemo(() => {
    const m = new Map<number, number>();
    deck?.main.forEach((id) => m.set(id, (m.get(id) ?? 0) + 1));
    return m;
  }, [deck]);

  // The search shown is the one of this deck, if any.
  const ours = run.request && deck && run.request.deckId === deck.id;

  // All boards of this search
  const allBoards = useMemo(() => {
    if (!ours) return [];
    return run.outcome ? run.outcome.boards : Object.values(run.live).flat();
  }, [ours, run.outcome, run.live]);

  // Boards matching the active hand filter
  const handFilteredBoards = useMemo(() => {
    return allBoards.filter((b) => handFilter == null || b.hand === handFilter);
  }, [allBoards, handFilter]);

  // Board IDs dominated on all metrics by another measured board (Pareto dominance)
  // Computed among boards for the active handFilter, with optional vs-deck resilience
  const dominatedBoardIds = useMemo(() => {
    if (!ours) return new Set<string>();
    const dominated = new Set<string>();
    const measuredList: { id: string; score: ComboScore }[] = [];

    for (const b of handFilteredBoards) {
      const id = boardId(b);
      const s = run.scores[id];
      if (s && !s.error) {
        measuredList.push({ id, score: s });
      }
    }

    for (let i = 0; i < measuredList.length; i++) {
      for (let j = 0; j < measuredList.length; j++) {
        if (i === j) continue;
        if (dominates(measuredList[i].score, measuredList[j].score, sortByMatchup)) {
          dominated.add(measuredList[j].id);
        }
      }
    }
    return dominated;
  }, [ours, handFilteredBoards, run.scores, sortByMatchup]);

  const totalMeasuredCount = useMemo(() => {
    let count = 0;
    for (const b of handFilteredBoards) {
      const s = run.scores[boardId(b)];
      if (s && !s.error) count++;
    }
    return count;
  }, [handFilteredBoards, run.scores]);

  const nonDominatedCount = useMemo(() => {
    let count = 0;
    for (const b of handFilteredBoards) {
      const id = boardId(b);
      const s = run.scores[id];
      if (s && !s.error && !dominatedBoardIds.has(id)) count++;
    }
    return count;
  }, [handFilteredBoards, run.scores, dominatedBoardIds]);

  const boards = useMemo(() => {
    if (!ours) return [];
    // Boards with meta.value != null come first, ranked descending by meta.value.
    const metaValue = (b: ComboBoard): number | null => {
      const s = run.scores[boardId(b)];
      return s && !s.error && s.meta && s.meta.value != null ? s.meta.value : null;
    };
    const value = (b: ComboBoard) => {
      const s = run.scores[boardId(b)];
      if (sortByMatchup && s?.matchup && !s.error && !s.matchup.error) {
        return s.matchup.resilience;
      }
      return s && !s.error ? score(s, weights) : null;
    };
    return handFilteredBoards
      .filter((b) => !paretoOnly || !dominatedBoardIds.has(boardId(b)))
      .map((b) => ({ b, v: value(b), mv: metaValue(b) }))
      .sort((x, y) => {
        const xHasMeta = x.mv != null;
        const yHasMeta = y.mv != null;
        if (xHasMeta && !yHasMeta) return -1;
        if (!xHasMeta && yHasMeta) return 1;
        if (xHasMeta && yHasMeta) {
          const diff = y.mv! - x.mv!;
          if (Math.abs(diff) > 1e-6) return diff;
        }
        return y.b.goalHits - x.b.goalHits || (y.v ?? -1e9) - (x.v ?? -1e9) || byRank(x.b, y.b);
      })
      .map((x) => x.b);
  }, [ours, handFilteredBoards, run.scores, weights, sortByMatchup, paretoOnly, dominatedBoardIds]);
  // Boards that look the same (they differ only in the Graveyard, the banished cards or the zones) are one entry.
  const groups = useMemo(() => {
    const out: { board: ComboBoard; variants: number }[] = [];
    const at = new Map<string, number>();
    for (const b of boards) {
      const key = lookKey(b);
      const i = at.get(key);
      if (i == null) {
        at.set(key, out.length);
        out.push({ board: b, variants: 0 });
      } else out[i].variants++;
    }
    return out;
  }, [boards]);
  const hands = useMemo(() => {
    return (ours && (run.outcome?.hands ?? run.progress?.hands)) || [];
  }, [ours, run.outcome?.hands, run.progress?.hands]);
  const scoreOf = (b: ComboBoard) => {
    const s = run.scores[boardId(b)];
    return s && !s.error ? s : null;
  };
  const scale = Math.max(0.01, ...groups.map(({ board }) => scoreOf(board)).filter((s) => s != null).map((s) => score(s!, weights)));
  const measured = totalMeasuredCount;
  const hasMetaBoards = useMemo(() => {
    return handFilteredBoards.some((b) => run.scores[boardId(b)]?.meta?.value != null);
  }, [handFilteredBoards, run.scores]);

  // Card data for everything the boards show.
  useEffect(() => {
    const codes = new Set<number>();
    for (const b of boards.slice(0, 60)) {
      for (const z of [...b.monsters, ...b.spells]) if (z) [z.code, ...z.overlay].forEach((c) => codes.add(c));
      [...b.handCards, ...b.grave, ...b.banished].forEach((c) => codes.add(c));
    }
    hands.forEach((h) => h.cards.forEach((c) => codes.add(c)));
    request([...codes]);
  }, [boards, hands, request]);

  // The best board is shown as soon as a search ends.
  const outcome = ours ? run.outcome : null;
  useEffect(() => {
    if (!outcome) return;
    const best = [...outcome.boards].sort(byRank)[0];
    setSelected(best ?? null);
  }, [outcome]);
  const goal = (ours && run.request?.goal) || null;

  // The line is told once the search is over (it plays the line again).
  useEffect(() => {
    setSteps(null);
    if (!selected || run.running || !outcome) return;
    const id = boardId(selected);
    const cached = explained.current.get(id);
    if (cached) {
      setSteps(cached);
      return;
    }
    let cancelled = false;
    api
      .comboExplain(selected.hand, selected.key)
      .then((s) => {
        explained.current.set(id, s);
        if (!cancelled) setSteps(s);
      })
      .catch((e) => {
        if (cancelled) return;
        setSteps([]);
        toast(`Linea non ricostruita: ${e}`, "error");
      });
    return () => {
      cancelled = true;
    };
  }, [selected, run.running, outcome]);

  const search = async () => {
    if (!deck?.id) return;
    const byHand = setup.mode === "hand";
    if (byHand && setup.hand.length === 0) return toast("Scegli le carte della mano", "error");
    if (!byHand && setup.card == null) return toast("Scegli la carta da cui partire", "error");
    sound.play("open");
    setSelected(null);
    setPinned(null);
    setCompareTarget(null);
    setHandFilter(null);
    setParetoOnly(false);
    setRecordedPairChoices(new Map());
    explained.current.clear();
    clearMetaError();
    metaReqIdRef.current++;
    const done = await startCombo({
      deckId: deck.id,
      hand: byHand ? setup.hand : [],
      card: byHand ? null : setup.card,
      seconds: options.seconds,
      hands: byHand ? null : options.hands,
      keep: setup.keep,
      forbid: setup.forbid,
      useHandtraps: options.useHandtraps,
      goal: setup.goal.length ? { cards: setup.goal, all: setup.goalAll, first: setup.goalFirst } : null,
    });
    if (done.error) {
      sound.play("error");
      toast(done.error, "error");
    } else {
      sound.play("glass");
    }
  };

  const goOn = async () => {
    sound.play("open");
    explained.current.clear();
    clearMetaError();
    metaReqIdRef.current++;
    const done = await continueCombo(options.seconds);
    if (done.error) {
      sound.play("error");
      toast(done.error, "error");
    } else sound.play("glass");
  };

  const handleCompareMeta = async () => {
    if (!deck || deck.id == null || run.running || run.scoring || run.metaLoading) return;
    sound.play("select");
    const reqId = ++metaReqIdRef.current;
    const currentDeckId = deck.id;
    const measuredBoards = boards.filter((b) => {
      const s = run.scores[boardId(b)];
      return s && !s.error;
    });
    const targetBoards =
      profile === "balanced"
        ? undefined
        : measuredBoards.slice(0, 4).map((b) => ({ hand: b.hand, key: b.key }));

    try {
      const count = await compareMeta(targetBoards, currentDeckId, reqId);
      if (reqId !== metaReqIdRef.current || deck?.id !== currentDeckId) return;
      if (count > 0) {
        sound.play("glass");
        toast(`Confronto con il meta completato per ${plural(count, "board", "board")}`, "info");
      }
    } catch (e) {
      if (reqId !== metaReqIdRef.current || deck?.id !== currentDeckId) return;
      sound.play("error");
      const msg = e instanceof Error ? e.message : String(e);
      toast(`Errore nel confronto con il meta: ${msg}`, "error");
    }
  };

  const addToGoal = (id: number) => {
    const max = (deck?.main.filter((c) => c === id).length ?? 0) + (deck?.extra.filter((c) => c === id).length ?? 0);
    if (setup.goal.filter((g) => g.code === id).length >= max || setup.goal.length >= 5) return sound.play("error");
    sound.play("select");
    setSetup((s) => ({ ...s, goal: [...s.goal, { code: id, zone: "any" }] }));
  };

  const addToHand = (id: number) => {
    if (setup.mode === "card") {
      sound.play("select");
      return setSetup((s) => ({ ...s, card: s.card === id ? null : id }));
    }
    const inHand = setup.hand.filter((c) => c === id).length;
    if (setup.hand.length >= 6 || inHand >= (copies.get(id) ?? 0)) return sound.play("error");
    sound.play("select");
    setSetup((s) => ({ ...s, hand: [...s.hand, id] }));
  };

  const removeFromHand = (index: number) => {
    sound.play("close");
    setSetup((s) => {
      const hand = s.hand.filter((_, i) => i !== index);
      return { ...s, hand, keep: s.keep.filter((c) => hand.includes(c)) };
    });
  };

  const randomHand = () => {
    if (!deck) return;
    const pool = [...deck.main];
    const hand: number[] = [];
    while (hand.length < 5 && pool.length) hand.push(pool.splice(Math.floor(Math.random() * pool.length), 1)[0]);
    sound.play("page");
    setSetup((s) => ({ ...s, mode: "hand", hand, keep: [] }));
  };

  const toggle = (list: "keep" | "forbid", id: number) => {
    sound.play("toggle");
    setSetup((s) => ({ ...s, [list]: s[list].includes(id) ? s[list].filter((c) => c !== id) : [...s[list], id] }));
  };

  const onHover = useCallback((code: number, el: HTMLElement) => setHover({ code, rect: el.getBoundingClientRect(), el }), []);
  const onHoverEnd = useCallback(() => setHover(null), []);
  // A card removed under the pointer never fires pointerleave: drop its preview on the next move.
  useEffect(() => {
    if (!hover) return;
    const check = (e: PointerEvent) => {
      if (!hover.el.isConnected || !(e.target instanceof Node && hover.el.contains(e.target))) setHover(null);
    };
    window.addEventListener("pointermove", check);
    window.addEventListener("pointerdown", check);
    return () => {
      window.removeEventListener("pointermove", check);
      window.removeEventListener("pointerdown", check);
    };
  }, [hover]);
  const hoverCard = hover ? cards.get(hover.code) : undefined;
  const name = (id: number) => cards.get(id)?.name ?? String(id);

  // Progress over the whole search ("card + rest" splits the time among the hands).
  const progress = ours ? run.progress : null;
  const budgetMs = (run.request?.seconds ?? options.seconds) * 1000;
  const handCount = Math.max(1, hands.length);
  const byCard = run.request?.card != null;
  const doneMs = progress?.elapsedMs ?? 0;
  // "Card + rest" goes hand after hand: the bar follows the hands.
  const done = run.outcome ? 1 : byCard ? ((progress?.handIndex ?? 0) + 0.5) / handCount : Math.min(1, doneMs / budgetMs);
  const rate = progress && progress.elapsedMs > 0 ? Math.round((progress.nodes * 1000) / progress.elapsedMs) : 0;

  // "Card + rest": the best estimate reached from each hand, and their weighted mean.
  const perHand = hands.map((h, i) => {
    const list = (outcome ? outcome.boards : (run.live[i] ?? [])).filter((b) => b.hand === i);
    return { ...h, index: i, best: list.length ? Math.max(...list.map((b) => b.quickScore)) : null, boards: list.length };
  });
  const searched = perHand.filter((h) => h.best != null);
  const weight = searched.reduce((t, h) => t + h.weight, 0);
  const mean = weight > 0 ? searched.reduce((t, h) => t + h.weight * h.best!, 0) / weight : null;

  const rankOf = (b: ComboBoard) => {
    const idx = groups.findIndex((g) => boardId(g.board) === boardId(b));
    return idx >= 0 ? idx + 1 : 1;
  };
  const section = sectionById("combo");
  const selectedHand = selected ? (hands[selected.hand]?.cards ?? []) : [];

  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={artReady} />
      <div className="relative z-20 flex flex-wrap items-center gap-3 border-b border-border bg-surface/70 px-6 py-2.5 backdrop-blur">
        <DeckMenu decks={decks} current={deck?.id} onPick={(id) => void open(id)} />
        <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1" role="radiogroup" aria-label="Da dove partire">
          {(
            [
              ["hand", "Mano precisa"],
              ["card", "Carta + resto"],
            ] as const
          ).map(([m, label]) => (
            <button
              key={m}
              type="button"
              role="radio"
              aria-checked={setup.mode === m}
              onClick={() => {
                sound.play("toggle");
                setSetup((s) => ({ ...s, mode: m }));
              }}
              className={`rounded-lg px-2.5 py-1 text-xs font-semibold ${setup.mode === m ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
            >
              {label}
            </button>
          ))}
        </div>
        <span className="ml-auto text-xs text-faint">Il motore gioca da solo il primo turno, senza avversario che risponde.</span>
      </div>

      {decks && decks.length === 0 ? (
        <div className="bg-blueprint flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <p className="display text-xl">Nessun deck da provare</p>
          <p className="text-sm text-muted">Il Circolo Arcano cerca le combo dei deck che costruisci nella Fucina.</p>
          <button type="button" className="btn btn-molten" onClick={onGoForge}>
            Vai alla Fucina
          </button>
        </div>
      ) : (
        <div className={`grid min-h-0 flex-1 ${selected ? "grid-cols-[292px_minmax(0,1fr)_396px] xl:grid-cols-[330px_minmax(0,1fr)_460px]" : "grid-cols-[292px_minmax(0,1fr)] xl:grid-cols-[330px_minmax(0,1fr)]"}`}>
          {/* The setup: hand or card, options, search. */}
          <aside className="flex min-h-0 flex-col gap-3 xl:gap-4 overflow-y-auto border-r border-border bg-surface/60 px-3.5 pt-3.5 xl:px-4 xl:pt-4">
            {setup.mode === "hand" ? (
              <section className="flex flex-col gap-2">
                <div className="flex items-baseline justify-between">
                  <h3 className="text-sm font-semibold">La mano</h3>
                  <button type="button" className="text-xs text-muted hover:text-text" onClick={randomHand} disabled={!deck}>
                    Mano casuale
                  </button>
                </div>
                <div className="grid grid-cols-6 gap-1.5">
                  {Array.from({ length: 6 }, (_, i) => {
                    const id = setup.hand[i];
                    if (id == null)
                      return (
                        <div key={i} className={`combo-slot ${i === 5 ? "opacity-50" : ""}`}>
                          {i + 1}
                        </div>
                      );
                    const kept = setup.keep.includes(id);
                    const passive = !options.useHandtraps && handtraps.has(id);
                    return (
                      <div key={i} className="relative" onPointerEnter={(e) => onHover(id, e.currentTarget)} onPointerLeave={onHoverEnd}>
                        <button type="button" className="block w-full" onClick={() => removeFromHand(i)} title="Togli dalla mano">
                          <CardImage code={id} card={cards.get(id)} className="aspect-[59/86] w-full" />
                        </button>
                        <button
                          type="button"
                          className={`combo-lock ${kept ? "combo-lock--on" : ""}`}
                          onClick={() => toggle("keep", id)}
                          title={kept ? "Deve restare in mano: tocca per liberarla" : "Tienila in mano a fine turno"}
                          aria-pressed={kept}
                        >
                          <LockIcon closed={kept} />
                        </button>
                        {passive && <span className="combo-passive" title="Handtrap: il motore non la attiva (vedi opzioni)">HT</span>}
                      </div>
                    );
                  })}
                </div>
                <p className="text-[11px] leading-relaxed text-faint">Tocca una carta del deck per metterla in mano, una della mano per toglierla. Il lucchetto la tiene in mano fino alla fine del turno.</p>
              </section>
            ) : (
              <section className="flex flex-col gap-2">
                <h3 className="text-sm font-semibold">La carta</h3>
                {setup.card != null ? (
                  <div className="flex items-center gap-3">
                    <CardImage code={setup.card} card={cards.get(setup.card)} className="h-[72px] w-[49px]" />
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">{name(setup.card)}</p>
                      <p className="text-xs text-muted">più 4 carte casuali dal resto del deck</p>
                    </div>
                  </div>
                ) : (
                  <p className="text-xs text-faint">Tocca una carta del deck: il motore prova tante mani che la contengono.</p>
                )}
                <div className="flex items-center gap-2 text-xs text-muted">
                  Mani da provare
                  {HAND_COUNTS.map((n) => (
                    <button key={n} type="button" className={`chip !py-0.5 ${options.hands === n ? "chip-active" : ""}`} onClick={() => setOptions({ hands: n })}>
                      {n}
                    </button>
                  ))}
                </div>
              </section>
            )}

            <section className="flex flex-col gap-2">
              <div className="flex items-baseline justify-between">
                <h3 className="text-sm font-semibold">Obiettivo</h3>
                <span className="text-[11px] text-faint">facoltativo</span>
              </div>
              <p className="text-[11px] leading-relaxed text-faint">Carte da avere sul campo a fine turno: il motore cerca la linea che ci arriva, o prova che non esiste.</p>
              {setup.goal.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {setup.goal.map(({ code: id, zone }, i) => (
                    <span key={i} className="chip !py-0.5 !pl-1 text-xs" onPointerEnter={(e) => onHover(id, e.currentTarget)} onPointerLeave={onHoverEnd}>
                      <CardImage code={id} card={cards.get(id)} className="h-[22px] w-[15px]" />
                      <span className="max-w-32 truncate">{name(id)}</span>
                      {extraUnique.includes(id) && (
                        <button
                          type="button"
                          className={`rounded px-1 text-[10px] font-semibold ${zone === "any" ? "text-faint hover:text-text" : "bg-sec text-bg"}`}
                          title={`Sul campo ${ZONES.find((z) => z.zone === zone)?.hint}: tocca per cambiare`}
                          onClick={() => {
                            sound.play("toggle");
                            const next = ZONES[(ZONES.findIndex((z) => z.zone === zone) + 1) % ZONES.length].zone;
                            setSetup((s) => ({ ...s, goal: s.goal.map((g, j) => (j === i ? { ...g, zone: next } : g)) }));
                          }}
                        >
                          {ZONES.find((z) => z.zone === zone)?.label}
                        </button>
                      )}
                      <button
                        type="button"
                        aria-label={`Togli ${name(id)} dall'obiettivo`}
                        className="text-faint hover:text-danger"
                        onClick={() => {
                          sound.play("close");
                          setSetup((s) => ({ ...s, goal: s.goal.filter((_, j) => j !== i) }));
                        }}
                      >
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              )}
              <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
                {(
                  [
                    ["extra", "+ dall'Extra Deck"],
                    ["main", "+ dal Main Deck"],
                  ] as const
                ).map(([where, label]) => (
                  <button
                    key={where}
                    type="button"
                    className={`chip !py-0.5 ${picker === where ? "chip-active" : ""}`}
                    onClick={() => {
                      sound.play(picker === where ? "close" : "open");
                      setPicker((p) => (p === where ? null : where));
                    }}
                    aria-expanded={picker === where}
                  >
                    {label}
                  </button>
                ))}
              </div>
              {picker && (
                <div className="grid grid-cols-6 gap-1 rounded-lg border border-border bg-bg-2 p-1.5 anim-fade">
                  {(picker === "extra" ? extraUnique : unique).map((id) => (
                    <button key={id} type="button" title={name(id)} onClick={() => addToGoal(id)} onPointerEnter={(e) => onHover(id, e.currentTarget)} onPointerLeave={onHoverEnd}>
                      <CardImage code={id} card={cards.get(id)} className="aspect-[59/86] w-full" />
                    </button>
                  ))}
                </div>
              )}
              {setup.goal.length > 1 && (
                <div className="flex gap-1 self-start rounded-lg border border-border bg-bg-2 p-0.5 text-[11px]" role="radiogroup" aria-label="Quante carte dell'obiettivo">
                  {(
                    [
                      [true, "tutte"],
                      [false, "almeno una"],
                    ] as const
                  ).map(([all, label]) => (
                    <button
                      key={label}
                      type="button"
                      role="radio"
                      aria-checked={setup.goalAll === all}
                      onClick={() => setSetup((s) => ({ ...s, goalAll: all }))}
                      className={`rounded-md px-2 py-0.5 font-semibold ${setup.goalAll === all ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              )}
              {setup.goal.length > 0 && (
                <label className="flex items-center gap-2 text-xs text-muted">
                  <input type="checkbox" className="accent-[var(--sec)]" checked={setup.goalFirst} onChange={(e) => setSetup((s) => ({ ...s, goalFirst: e.target.checked }))} />
                  Fermati alla prima linea che ci arriva
                </label>
              )}
            </section>

            <section className="flex flex-col gap-2">
              <div className="flex items-baseline justify-between">
                <h3 className="text-sm font-semibold">Il Main Deck</h3>
                <span className="text-[11px] text-faint">clic destro: vietala al motore</span>
              </div>
              <div className="grid grid-cols-6 gap-1">
                {unique.map((id) => {
                  const left = (copies.get(id) ?? 0) - setup.hand.filter((c) => c === id).length;
                  const banned = setup.forbid.includes(id);
                  return (
                    <div key={id} className={`relative ${setup.mode === "card" && setup.card === id ? "combo-picked" : ""} ${banned ? "combo-banned" : ""} ${setup.mode === "hand" && left === 0 ? "combo-exhausted" : ""}`}>
                      <CardTile
                        id={id}
                        card={cards.get(id)}
                        compact
                        badge={banned ? { text: "✕", tone: "ban" } : { text: `${setup.mode === "hand" ? left : copies.get(id)}` }}
                        onSelect={() => addToHand(id)}
                        onHover={onHover}
                        onHoverEnd={onHoverEnd}
                        onContext={() => toggle("forbid", id)}
                      />
                    </div>
                  );
                })}
              </div>
            </section>

            <section className="flex flex-col gap-2.5">
              <h3 className="text-sm font-semibold">Opzioni</h3>
              <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
                Tempo
                {BUDGETS.map((b) => (
                  <button key={b.seconds} type="button" className={`chip !py-0.5 ${options.seconds === b.seconds ? "chip-active" : ""}`} onClick={() => setOptions({ seconds: b.seconds })}>
                    {b.label}
                  </button>
                ))}
              </div>
              {setup.mode === "card" && <p className="text-[11px] text-faint">Circa {Math.max(1, Math.round(options.seconds / options.hands))} s per mano.</p>}
              <label className="flex items-start gap-2 text-xs text-muted">
                <input type="checkbox" className="mt-0.5 accent-[var(--sec)]" checked={options.useHandtraps} onChange={(e) => setOptions({ useHandtraps: e.target.checked })} />
                <span>
                  Usa anche le handtrap nella combo
                  <span className="block text-[11px] text-faint">Di solito restano in mano per il turno avversario.</span>
                </span>
              </label>
              {setup.forbid.length > 0 && (
                <p className="text-[11px] text-faint">
                  Vietate: {setup.forbid.map(name).join(", ")}{" "}
                  <button type="button" className="underline hover:text-text" onClick={() => setSetup((s) => ({ ...s, forbid: [] }))}>
                    togli
                  </button>
                </p>
              )}
            </section>

            <div className="sticky bottom-0 -mx-3.5 xl:-mx-4 mt-auto border-t border-border bg-surface px-3.5 pt-2.5 pb-3.5 xl:px-4 xl:pt-3 xl:pb-4">
              {run.running ? (
                <button type="button" className="btn btn-ghost w-full justify-center" onClick={stopCombo}>
                  Ferma la ricerca
                </button>
              ) : (
                <button type="button" className="btn btn-molten w-full justify-center" onClick={() => void search()} disabled={!deck}>
                  Cerca le combo
                </button>
              )}
            </div>
          </aside>

          {/* The boards found, best first. */}
          <div className="bg-blueprint flex min-h-0 flex-col gap-3.5 xl:gap-4 overflow-y-auto px-4 py-4 xl:px-6 xl:py-5">
            {run.running && !ours && <p className="text-sm text-muted">Una ricerca su un altro deck è in corso.</p>}
            {(progress || run.running) && ours && (
              <section className="lab-card flex flex-col gap-2.5 p-4" aria-live="polite">
                <div className="flex items-baseline justify-between gap-3">
                  <h3 className="min-w-0 text-sm font-semibold">
                    {run.running ? "Il motore sta giocando" : `Ricerca finita: ${STOPS[run.outcome?.stop ?? ""] ?? run.outcome?.stop ?? ""}`}
                    {run.request?.card != null && progress && ` · mano ${Math.min(progress.handIndex + 1, handCount)} di ${handCount}`}
                  </h3>
                  <span className="shrink-0 whitespace-nowrap text-xs text-faint tabular-nums">{byCard ? (run.outcome ? duration(run.outcome.elapsedMs) : "") : `${duration(doneMs)} di ${duration(budgetMs)}`}</span>
                </div>
                <div className="molten-track h-1.5">
                  <div className="molten-fill h-full transition-[width] duration-300" style={{ width: `${done * 100}%` }} />
                </div>
                {progress && (
                  <div className="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted tabular-nums">
                    <span>
                      <b className="text-text">{count(run.outcome?.nodes ?? progress.nodes)}</b> scelte giocate
                    </span>
                    <span>
                      <b className="text-text">{count(run.request?.card != null ? boards.length : progress.boards)}</b> board diverse
                    </span>
                    <span>{count(rate)} scelte/s</span>
                    {run.running && <span>{count(progress.queued)} strade ancora da provare</span>}
                  </div>
                )}
                {run.outcome &&
                  (run.outcome.complete ? (
                    <p className="text-xs text-ok">Tutte le linee provate: la ricerca è completa, non ci sono altre board da trovare.</p>
                  ) : (
                    <div className="flex flex-wrap items-center gap-3">
                      <button type="button" className="btn btn-ghost !py-1 text-xs" onClick={() => void goOn()}>
                        Continua per {BUDGETS.find((b) => b.seconds === options.seconds)?.label ?? `${options.seconds} s`}
                      </button>
                      <span className="text-[11px] text-faint">riprende da dove si è fermata, senza ripartire da capo</span>
                    </div>
                  ))}
              </section>
            )}

            {goal && ours && (run.running || run.outcome) && (
              <GoalBanner
                goal={goal}
                running={run.running}
                byCard={byCard}
                groups={groups}
                hands={hands}
                elapsedMs={run.outcome?.elapsedMs ?? doneMs}
                name={name}
                onShow={(b) => {
                  sound.play("page");
                  setSelected(b);
                }}
              />
            )}

            {ours && run.request?.card != null && perHand.length > 0 && (
              <section className="lab-card flex flex-col gap-2 p-4">
                <div className="flex items-baseline justify-between gap-3">
                  <h3 className="text-sm font-semibold">Le mani con {name(run.request.card)}</h3>
                  {mean != null && (
                    <span className="text-xs text-muted">
                      stima media pesata <b className="text-text tabular-nums">{mean.toFixed(1)}</b>
                    </span>
                  )}
                </div>
                <div className="flex flex-col gap-1">
                  {perHand.map((h) => (
                    <button
                      key={h.index}
                      type="button"
                      onClick={() => {
                        sound.play("toggle");
                        setHandFilter((f) => (f === h.index ? null : h.index));
                      }}
                      className={`flex items-center gap-3 rounded-lg px-2 py-1 text-left hover:bg-surface-3 ${handFilter === h.index ? "bg-surface-3 shadow-[inset_2px_0_0_var(--sec)]" : ""}`}
                    >
                      <span className="w-5 text-right text-xs text-faint tabular-nums">{h.index + 1}</span>
                      <span className="flex gap-0.5">
                        {h.cards.map((c, i) => (
                          <CardImage key={i} code={c} card={cards.get(c)} className="h-[32px] w-[22px]" />
                        ))}
                      </span>
                      <span className="text-[11px] text-faint tabular-nums">{Math.round(h.weight * 100)}%</span>
                      {goal && <GoalMark status={h.goal} />}
                      <span className="ml-auto text-xs tabular-nums">
                        {h.best != null ? (
                          <b>{h.best.toFixed(1)}</b>
                        ) : run.running && progress?.handIndex === h.index ? (
                          <span className="text-sec">in corso…</span>
                        ) : (
                          <span className="text-faint">—</span>
                        )}
                      </span>
                    </button>
                  ))}
                </div>
                <p className="text-[11px] text-faint">La percentuale è quanto pesa la mano tra tutte quelle con la carta. Tocca una mano per vederne solo le board.</p>
              </section>
            )}

            {ours && boards.length > 0 ? (
              <section className="flex flex-col gap-1.5">
                <div className="flex flex-wrap items-center justify-between gap-2 px-1">
                  <div className="flex items-baseline gap-2">
                    <h3 className="text-sm font-semibold">{run.running ? "Le migliori board finora" : "Le board finali"}</h3>
                    <span className="text-[11px] text-faint" aria-live="polite">
                      {run.running
                        ? "stima provvisoria finché la ricerca gira"
                        : run.scoring
                          ? `turno di prova in corso: ${measured} board misurate…`
                          : run.metaLoading
                            ? "confronto con il meta in corso…"
                            : measured
                            ? "punteggio misurato con il turno di prova"
                            : "stima provvisoria"}
                    </span>
                  </div>
                  {!run.running && (
                    <div className="flex flex-wrap items-center gap-2 text-xs">
                      <label className="flex items-center gap-1.5 text-muted">
                        <span>Contro:</span>
                        <select
                          value={run.vsDeckId ?? ""}
                          onChange={(e) => {
                            const id = e.target.value ? Number(e.target.value) : null;
                            sound.play("toggle");
                            void scoreBoards(id);
                          }}
                          disabled={run.scoring || run.metaLoading}
                          className="select !py-0.5 !text-xs max-w-[200px]"
                          title="Scegli un deck della Bottega per misurare breaker e handtrap reali dell'avversario"
                        >
                          <option value="">Avversario generico</option>
                          {rivalDecks.map((d) => (
                            <option key={d.id} value={d.id}>
                              {d.name} ({d.mainCount} carte)
                            </option>
                          ))}
                        </select>
                      </label>
                      <button
                        type="button"
                        onClick={() => {
                          sound.play("select");
                          void scoreBoards(run.vsDeckId);
                        }}
                        disabled={run.scoring || run.metaLoading}
                        className="btn btn-ghost !px-2 !py-0.5 text-[11px]"
                        title="Misura le board che mancano con l'avversario scelto"
                      >
                        {run.scoring ? "Misurazione…" : "↻ Rimisura"}
                      </button>
                      {run.vsDeckId != null && (
                        <button
                          type="button"
                          onClick={() => {
                            sound.play("toggle");
                            setSortByMatchup((v) => !v);
                          }}
                          className={`chip !py-0.5 text-[11px] ${sortByMatchup ? "chip-active" : ""}`}
                          title="Ordina per resilienza contro il deck avversario"
                        >
                          {sortByMatchup ? "✓ Ordine: vs deck" : "Ordina per vs deck"}
                        </button>
                      )}
                      {(totalMeasuredCount > 1 || paretoOnly) && (
                        <button
                          type="button"
                          onClick={() => {
                            sound.play("toggle");
                            setParetoOnly((v) => !v);
                          }}
                          className={`chip !py-0.5 text-[11px] ${paretoOnly ? "chip-active" : ""}`}
                          title="Mostra solo le board non dominate (frontiera di Pareto): nessuna altra board misurata è uguale o migliore su tutte le metriche"
                        >
                          {paretoOnly
                            ? `✓ Solo non dominate (${nonDominatedCount}/${totalMeasuredCount})`
                            : "Solo non dominate (Pareto)"}
                        </button>
                      )}
                      {totalMeasuredCount > 0 && (
                        <button
                          type="button"
                          onClick={() => void handleCompareMeta()}
                          disabled={run.scoring || run.metaLoading}
                          className="btn btn-ghost !px-2 !py-0.5 text-[11px] text-sec border-sec/40 hover:bg-sec/10"
                          title="Confronta le migliori board contro i 3 deck più giocati del meta su Master Duel Meta (L2-3)"
                        >
                          {run.metaLoading ? "Meta in corso…" : hasMetaBoards ? "↻ Ricalcola meta" : "★ Confronta con il meta"}
                        </button>
                      )}
                    </div>
                  )}
                </div>
                {run.metaLoading && (
                  <div className="lab-card flex items-start gap-3 border-sec/60 bg-surface-2/80 p-3 text-xs anim-fade">
                    <span className="spinner mt-0.5 h-4 w-4 shrink-0 text-sec" aria-hidden />
                    <div className="flex flex-col gap-1">
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-text">Confronto con il meta in corso…</span>
                        <span className="chip !py-0 !px-1.5 text-[10px] text-sec font-semibold">L2-3</span>
                      </div>
                      <p className="text-[11px] leading-relaxed text-muted">
                        Il confronto con il meta è in corso. La prima volta richiede circa 1 minuto (lettura del meta da Master Duel Meta e delle linee dei 3 deck più giocati), poi circa 20 s per board.
                      </p>
                    </div>
                  </div>
                )}
                {run.metaError && (
                  <div className="lab-card flex items-start justify-between gap-3 border-danger/60 bg-danger/10 p-3 text-xs text-danger anim-fade">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="shrink-0 font-bold" aria-hidden>⚠</span>
                      <span className="min-w-0 font-medium">{run.metaError}</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => clearMetaError()}
                      className="btn btn-ghost !py-0.5 !px-2 text-[11px] text-danger hover:bg-danger/20"
                      title="Chiudi avviso"
                    >
                      ✕
                    </button>
                  </div>
                )}
                {!run.running && totalMeasuredCount > 0 && (
                  <WeightsPanel
                    profile={profile}
                    weights={weights}
                    open={weightsOpen}
                    compact={!!selected}
                    choicesVersion={choicesVersion}
                    onToggle={() => setWeightsOpen((v) => !v)}
                    onChoicesChanged={() => setChoicesVersion((v) => v + 1)}
                  />
                )}
                {pinned && (
                  <div className="lab-card flex flex-wrap items-center justify-between gap-2 border-sec/60 bg-surface-2/80 px-3 py-2 text-xs anim-fade">
                    <span className="text-muted">
                      Riferimento fissato: <b className="text-text">Board #{rankOf(pinned)}</b> · clicca <b className="text-sec">⇄ Confronta</b> su un'altra board per aprirle fianco a fianco.
                    </span>
                    <div className="flex items-center gap-2">
                      {selected && boardId(selected) !== boardId(pinned) && (
                        <button
                          type="button"
                          onClick={() => {
                            sound.play("open");
                            setCompareTarget(selected);
                          }}
                          className="btn btn-molten !px-2.5 !py-1 text-xs"
                        >
                          ⇄ Confronta con Board #{rankOf(selected)}
                        </button>
                      )}
                      <button
                        type="button"
                        onClick={() => {
                          sound.play("close");
                          setPinned(null);
                          setCompareTarget(null);
                        }}
                        className="text-xs text-muted hover:text-text"
                      >
                        Rimuovi
                      </button>
                    </div>
                  </div>
                )}
                {groups.slice(0, 60).map(({ board: b, variants }, i) => {
                  const active = selected && boardId(selected) === boardId(b);
                  const isPinned = pinned && boardId(pinned) === boardId(b);
                  return (
                    <div
                      key={boardId(b)}
                      role="button"
                      tabIndex={0}
                      onClick={() => {
                        sound.play("page");
                        setSelected(active ? null : b);
                      }}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          sound.play("page");
                          setSelected(active ? null : b);
                        }
                      }}
                      style={i < 12 ? { animationDelay: `${i * 22}ms` } : undefined}
                      className={`lab-card grid cursor-pointer grid-cols-[28px_minmax(0,1fr)_auto] items-center gap-3 px-3 py-2 text-left transition-colors hover:border-border-strong ${i < 12 ? "anim-rise" : ""} ${active ? "combo-board-active" : ""}`}
                    >
                      <span className="display text-center text-sm text-faint">{i + 1}</span>
                      <span className="flex min-w-0 flex-col gap-1.5">
                        <BoardStrip board={b} width={26} getCard={cards.get} />
                        {scoreOf(b) && <ScoreBar s={scoreOf(b)!} weights={weights} scale={scale} />}
                      </span>
                      <span className="flex flex-col items-end text-xs whitespace-nowrap text-muted tabular-nums">
                        {scoreOf(b) ? (
                          <b className="text-sm text-text" title="Punteggio misurato">
                            {score(scoreOf(b)!, weights).toFixed(2)}
                          </b>
                        ) : (
                          <span title="Stima provvisoria (mostri, magie e trappole, carte in mano)">
                            <b className="text-sm text-faint">{b.quickScore.toFixed(1)}</b> <span className="text-[10px]">stima</span>
                          </span>
                        )}
                        {scoreOf(b)?.meta?.value != null && (
                          <span
                            className="chip !px-1.5 !py-0 text-[10px] text-sec border-sec/40 font-semibold"
                            title={`Efficacia nel meta (L2-3): in media ferma il ${Math.round(scoreOf(b)!.meta!.value! * 100)}% delle giocate dei top deck`}
                          >
                            meta: {Math.round(scoreOf(b)!.meta!.value! * 100)}%
                          </span>
                        )}
                        {scoreOf(b) && !dominatedBoardIds.has(boardId(b)) && measured > 1 && (
                          <span
                            className="chip !px-1.5 !py-0 text-[10px] text-ok border-ok/40 font-semibold"
                            title="Board non dominata (frontiera di Pareto): nessuna altra board misurata è uguale o migliore su tutte le metriche"
                          >
                            Pareto
                          </span>
                        )}
                        {scoreOf(b)?.matchup && (
                          <span
                            className="chip !px-1.5 !py-0 text-[10px] text-sec font-semibold"
                            title={
                              scoreOf(b)!.matchup!.error
                                ? `Calcolo contro ${scoreOf(b)!.matchup!.deckName} fallito: ${scoreOf(b)!.matchup!.error}`
                                : `Resilienza contro ${scoreOf(b)!.matchup!.deckName}: in media restano ${scoreOf(b)!.matchup!.resilience.toFixed(2)} su ${scoreOf(b)!.interruptions.toFixed(2)}`
                            }
                          >
                            vs {scoreOf(b)!.matchup!.deckName}: {scoreOf(b)!.matchup!.error ? "errore" : scoreOf(b)!.matchup!.resilience.toFixed(2)}
                          </span>
                        )}
                        <span>
                          {!b.exact && (
                            <span className="mr-1 text-faint" title="Letta quando passi il turno: End Phase non ancora giocata">
                              ≈
                            </span>
                          )}
                          {b.cardsUsed} carte · {b.choices} scelte
                        </span>
                        {goal && (reaches(b, goal) ? <span className="text-[11px] font-semibold text-ok">✓ obiettivo</span> : <span className="text-[11px] text-faint">obiettivo {b.goalHits}/{goal.cards.length}</span>)}
                        {variants > 0 && <span className="text-[11px] text-faint">+{variants} {variants === 1 ? "variante" : "varianti"}</span>}
                        {groups.length > 1 && (
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              if (isPinned) {
                                sound.play("close");
                                setPinned(null);
                              } else if (pinned) {
                                sound.play("open");
                                setCompareTarget(b);
                              } else {
                                sound.play("toggle");
                                setPinned(b);
                              }
                            }}
                            className={`mt-1 rounded border px-1.5 py-0.5 text-[10px] font-semibold transition ${
                              isPinned
                                ? "border-sec bg-sec/20 text-sec"
                                : pinned
                                  ? "border-sec/60 bg-surface-2 text-sec hover:bg-sec hover:text-bg"
                                  : "border-border bg-surface-2/70 text-muted hover:border-border-strong hover:text-text"
                            }`}
                            title={
                              isPinned
                                ? "Board fissata per il confronto (clicca per rimuovere)"
                                : pinned
                                  ? `Confronta fianco a fianco con Board #${rankOf(pinned)}`
                                  : "Fissa questa board per confrontarla con un'altra"
                            }
                          >
                            {isPinned ? "✓ Fissata" : pinned ? `⇄ vs #${rankOf(pinned)}` : "⇄ Confronta"}
                          </button>
                        )}
                      </span>
                    </div>
                  );
                })}
              </section>
            ) : (
              !run.running &&
              !(ours && run.outcome) && (
                <section className="lab-card mx-auto mt-6 flex max-w-xl flex-col gap-3 p-6 text-sm text-muted">
                  <p className="display text-lg text-text">Come funziona</p>
                  <p>Scegli una mano (o una carta) e premi «Cerca le combo». Il motore di gioco prova da solo tutte le scelte possibili del primo turno: attivazioni, evocazioni, bersagli, ordine delle catene.</p>
                  <p>Ogni volta che passa il turno con una board nuova la conserva, con la linea più corta per arrivarci. Puoi cambiare sezione mentre cerca: la ricerca continua, e a ricerca finita puoi farla proseguire da dove si è fermata.</p>
                  <p>Con un <b className="text-text">obiettivo</b> (per esempio un mostro dell'Extra Deck da avere sul campo) il motore prova prima le linee che ci portano e ti dice se ci arriva, con la linea, o se è impossibile perché nessuna linea ci arriva.</p>
                  <p>
                    A ricerca finita le board migliori passano il <b className="text-text">turno di prova</b>: un avversario con carte vere prova a giocare (Magie, ricerche, effetti dalla mano e dal Cimitero, Evocazioni) e a smontarle con Dark Hole, Raigeki, Kaiju e altri. La classifica conta interruzioni, floodgate e resilienza misurati così, con pesi che puoi cambiare.
                  </p>
                </section>
              )
            )}
            {ours && run.outcome && boards.length === 0 && <p className="text-sm text-muted">Nessuna board trovata: il turno passa senza giocate.</p>}
          </div>

          {selected && (
            <div key={boardId(selected)} className="min-h-0 overflow-y-auto border-l border-border bg-surface/60 p-3.5 xl:p-4 anim-fade">
              <div className="mb-3 flex items-center justify-between gap-2">
                <h3 className="display text-base">Board {rankOf(selected)}</h3>
                <div className="flex items-center gap-2">
                  {groups.length > 1 && (
                    <button
                      type="button"
                      onClick={() => {
                        if (pinned && boardId(pinned) === boardId(selected)) {
                          sound.play("close");
                          setPinned(null);
                        } else if (pinned) {
                          sound.play("open");
                          setCompareTarget(selected);
                        } else {
                          sound.play("toggle");
                          setPinned(selected);
                        }
                      }}
                      className="chip !py-0.5 text-[11px] hover:border-sec/60 hover:text-text"
                    >
                      {pinned && boardId(pinned) === boardId(selected)
                        ? "✓ Fissata per confronto"
                        : pinned
                          ? `⇄ Confronta con #${rankOf(pinned)}`
                          : "⇄ Fissa per confronto"}
                    </button>
                  )}
                  <button type="button" className="text-xs text-muted hover:text-text" onClick={() => setSelected(null)}>
                    Chiudi
                  </button>
                </div>
              </div>
              <BoardDetail
                board={selected}
                score={run.scores[boardId(selected)] ?? null}
                scoring={run.scoring}
                weights={weights}
                hand={selectedHand}
                steps={run.running ? [] : steps}
                stepsNote={run.running ? "La linea sarà raccontata appena la ricerca finisce." : null}
                getCard={cards.get}
                onHover={onHover}
                onHoverEnd={onHoverEnd}
              />
            </div>
          )}
        </div>
      )}
      {pinned && compareTarget && (
        <BoardCompareModal
          open={true}
          deckId={deck?.id ?? null}
          boardA={pinned}
          rankA={rankOf(pinned)}
          scoreA={run.scores[boardId(pinned)] ?? null}
          boardB={compareTarget}
          rankB={rankOf(compareTarget)}
          scoreB={run.scores[boardId(compareTarget)] ?? null}
          weights={weights}
          getCard={cards.get}
          onHover={onHover}
          onHoverEnd={onHoverEnd}
          initialChosenId={recordedPairChoices.get(pairKey(boardId(pinned), boardId(compareTarget))) ?? null}
          onSwap={() => {
            const prevPinned = pinned;
            setPinned(compareTarget);
            setCompareTarget(prevPinned);
          }}
          onClose={() => setCompareTarget(null)}
          onChoiceRecorded={(chosenId) => {
            setRecordedPairChoices((prev) => new Map(prev).set(pairKey(boardId(pinned), boardId(compareTarget)), chosenId));
            setChoicesVersion((v) => v + 1);
          }}
        />
      )}
      {hover && hoverCard && <CardPreview card={hoverCard} anchor={hover.rect} />}
    </div>
  );
}
