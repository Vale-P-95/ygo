import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BoardDetail } from "../components/combo/BoardDetail";
import { BoardStrip, CardImage } from "../components/combo/Field";
import { CardPreview } from "../components/CardPreview";
import { CardTile } from "../components/CardTile";
import { DeckMenu } from "../components/DeckMenu";
import { SectionHeader } from "../components/SectionHeader";
import { api, type ComboBoard, type ComboStep, type Deck, type DeckSummary, type ZoneCard } from "../lib/api";
import { useCardCache } from "../lib/cards";
import { startCombo, stopCombo, useCombo } from "../lib/combo";
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
}

const NO_SETUP: Setup = { mode: "hand", hand: [], card: null, keep: [], forbid: [] };

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

/** What the player sees of a board: the field and the hand (the piles may differ). */
function lookKey(b: ComboBoard): string {
  const zone = (z: ZoneCard | null) => (z ? `${z.code}${z.faceUp ? "u" : "d"}${z.defense ? "d" : "a"}${z.overlay.length}` : "");
  const sorted = (list: string[]) => list.filter(Boolean).sort().join(",");
  return [b.hand, sorted(b.monsters.map(zone)), sorted(b.spells.slice(0, 6).map(zone)), sorted(b.handCards.map(String))].join("|");
}

const byRank = (a: ComboBoard, b: ComboBoard) => b.quickScore - a.quickScore || a.cardsUsed - b.cardsUsed || a.choices - b.choices;

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

/** Il Circolo Arcano: the engine plays every line from a hand by itself and ranks the boards. */
export function Combo({ artReady, onGoForge }: { artReady: boolean; onGoForge: () => void }) {
  const [decks, setDecks] = useState<DeckSummary[] | null>(null);
  const [deck, setDeck] = useState<Deck | null>(null);
  const [setup, setSetupState] = useState<Setup>(NO_SETUP);
  const [options, setOptionsState] = useState(() => load("duelforge.combo.options", { seconds: 60, hands: 12, useHandtraps: false }));
  const [handtraps, setHandtraps] = useState<Set<number>>(new Set());
  const [selected, setSelected] = useState<ComboBoard | null>(null);
  const [steps, setSteps] = useState<ComboStep[] | null>(null);
  const [handFilter, setHandFilter] = useState<number | null>(null);
  const [hover, setHover] = useState<{ code: number; rect: DOMRect } | null>(null);
  const explained = useRef(new Map<string, ComboStep[]>());
  const run = useCombo();
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
      setSetupState(load(`duelforge.combo.${id}`, NO_SETUP));
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
  const copies = useMemo(() => {
    const m = new Map<number, number>();
    deck?.main.forEach((id) => m.set(id, (m.get(id) ?? 0) + 1));
    return m;
  }, [deck]);

  // The search shown is the one of this deck, if any.
  const ours = run.request && deck && run.request.deckId === deck.id;
  const boards = useMemo(() => {
    if (!ours) return [];
    const list = run.outcome ? run.outcome.boards : Object.values(run.live).flat();
    return list.filter((b) => handFilter == null || b.hand === handFilter).sort(byRank);
  }, [ours, run.outcome, run.live, handFilter]);
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
  const hands = (ours && (run.outcome?.hands ?? run.progress?.hands)) || [];

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
    setHandFilter(null);
    explained.current.clear();
    const done = await startCombo({
      deckId: deck.id,
      hand: byHand ? setup.hand : [],
      card: byHand ? null : setup.card,
      seconds: options.seconds,
      hands: byHand ? null : options.hands,
      keep: setup.keep,
      forbid: setup.forbid,
      useHandtraps: options.useHandtraps,
    });
    if (done.error) {
      sound.play("error");
      toast(done.error, "error");
    } else {
      sound.play("glass");
    }
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

  const onHover = useCallback((code: number, el: HTMLElement) => setHover({ code, rect: el.getBoundingClientRect() }), []);
  const onHoverEnd = useCallback(() => setHover(null), []);
  const hoverCard = hover ? cards.get(hover.code) : undefined;
  const name = (id: number) => cards.get(id)?.name ?? String(id);

  // Progress over the whole search ("card + rest" splits the time among the hands).
  const progress = ours ? run.progress : null;
  const budgetMs = (run.request?.seconds ?? options.seconds) * 1000;
  const handCount = Math.max(1, hands.length);
  const doneMs = progress ? (run.request?.card != null ? (progress.handIndex * budgetMs) / handCount + progress.elapsedMs : progress.elapsedMs) : 0;
  const rate = progress && progress.elapsedMs > 0 ? Math.round((progress.nodes * 1000) / progress.elapsedMs) : 0;

  // "Card + rest": the best estimate reached from each hand, and their weighted mean.
  const perHand = hands.map((h, i) => {
    const list = (outcome ? outcome.boards : (run.live[i] ?? [])).filter((b) => b.hand === i);
    return { ...h, index: i, best: list.length ? Math.max(...list.map((b) => b.quickScore)) : null, boards: list.length };
  });
  const searched = perHand.filter((h) => h.best != null);
  const weight = searched.reduce((t, h) => t + h.weight, 0);
  const mean = weight > 0 ? searched.reduce((t, h) => t + h.weight * h.best!, 0) / weight : null;

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
        <div className={`grid min-h-0 flex-1 ${selected ? "grid-cols-[330px_minmax(0,1fr)_460px]" : "grid-cols-[330px_minmax(0,1fr)]"}`}>
          {/* The setup: hand or card, options, search. */}
          <aside className="flex min-h-0 flex-col gap-4 overflow-y-auto border-r border-border bg-surface/60 p-4">
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
                <h3 className="text-sm font-semibold">Il Main Deck</h3>
                <span className="text-[11px] text-faint">clic destro: vietala al motore</span>
              </div>
              <div className="grid grid-cols-6 gap-1">
                {unique.map((id) => {
                  const left = (copies.get(id) ?? 0) - setup.hand.filter((c) => c === id).length;
                  const banned = setup.forbid.includes(id);
                  return (
                    <div key={id} className={`relative ${setup.mode === "card" && setup.card === id ? "combo-picked" : ""} ${banned ? "combo-banned" : ""} ${setup.mode === "hand" && left === 0 ? "opacity-40" : ""}`}>
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

            <div className="sticky bottom-0 -mx-4 mt-auto border-t border-border bg-surface/95 px-4 pt-3 pb-1 backdrop-blur">
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
          <div className="bg-blueprint flex min-h-0 flex-col gap-4 overflow-y-auto px-6 py-5">
            {run.running && !ours && <p className="text-sm text-muted">Una ricerca su un altro deck è in corso.</p>}
            {(progress || run.running) && ours && (
              <section className="lab-card flex flex-col gap-2.5 p-4" aria-live="polite">
                <div className="flex items-baseline justify-between gap-3">
                  <h3 className="text-sm font-semibold">
                    {run.running ? "Il motore sta giocando" : `Ricerca finita: ${STOPS[run.outcome?.stop ?? ""] ?? run.outcome?.stop ?? ""}`}
                    {run.request?.card != null && progress && ` · mano ${Math.min(progress.handIndex + 1, handCount)} di ${handCount}`}
                  </h3>
                  <span className="text-xs text-faint tabular-nums">
                    {duration(run.outcome?.elapsedMs ?? doneMs)} di {duration(budgetMs)}
                  </span>
                </div>
                <div className="molten-track h-1.5">
                  <div className="molten-fill h-full transition-[width] duration-300" style={{ width: `${run.outcome ? 100 : Math.min(100, (doneMs / budgetMs) * 100)}%` }} />
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
              </section>
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
                <div className="flex items-baseline justify-between px-1">
                  <h3 className="text-sm font-semibold">{run.running ? "Le migliori board finora" : "Le board finali"}</h3>
                  <span className="text-[11px] text-faint">stima provvisoria: mostri, magie/trappole e carte rimaste in mano</span>
                </div>
                {groups.slice(0, 60).map(({ board: b, variants }, i) => {
                  const active = selected && boardId(selected) === boardId(b);
                  return (
                    <button
                      key={boardId(b)}
                      type="button"
                      onClick={() => {
                        sound.play("page");
                        setSelected(active ? null : b);
                      }}
                      className={`lab-card grid grid-cols-[28px_minmax(0,1fr)_auto] items-center gap-3 px-3 py-2 text-left transition-colors hover:border-border-strong ${active ? "!border-[var(--sec)]" : ""}`}
                    >
                      <span className="display text-center text-sm text-faint">{i + 1}</span>
                      <BoardStrip board={b} width={26} getCard={cards.get} />
                      <span className="flex flex-col items-end text-xs whitespace-nowrap text-muted tabular-nums">
                        <b className="text-sm text-text">{b.quickScore.toFixed(1)}</b>
                        <span>
                          {b.cardsUsed} carte · {b.choices} scelte
                        </span>
                        {variants > 0 && <span className="text-[11px] text-faint">+{variants} {variants === 1 ? "variante" : "varianti"}</span>}
                      </span>
                    </button>
                  );
                })}
              </section>
            ) : (
              !run.running &&
              !(ours && run.outcome) && (
                <section className="lab-card mx-auto mt-6 flex max-w-xl flex-col gap-3 p-6 text-sm text-muted">
                  <p className="display text-lg text-text">Come funziona</p>
                  <p>Scegli una mano (o una carta) e premi «Cerca le combo». Il motore di gioco prova da solo tutte le scelte possibili del primo turno: attivazioni, evocazioni, bersagli, ordine delle catene.</p>
                  <p>Ogni volta che passa il turno con una board nuova la conserva, con la linea più corta per arrivarci. Puoi cambiare sezione mentre cerca: la ricerca continua.</p>
                  <p className="text-faint">La classifica attuale è una stima semplice; la classifica completa per interruzioni, floodgate e resilienza arriva con la Fase 5C.</p>
                </section>
              )
            )}
            {ours && run.outcome && boards.length === 0 && <p className="text-sm text-muted">Nessuna board trovata: il turno passa senza giocate.</p>}
          </div>

          {selected && (
            <div className="min-h-0 overflow-y-auto border-l border-border bg-surface/60 p-4">
              <div className="mb-3 flex items-center justify-between">
                <h3 className="display text-base">Board {groups.findIndex((g) => boardId(g.board) === boardId(selected)) + 1 || ""}</h3>
                <button type="button" className="text-xs text-muted hover:text-text" onClick={() => setSelected(null)}>
                  Chiudi
                </button>
              </div>
              <BoardDetail
                board={selected}
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
      {hover && hoverCard && <CardPreview card={hoverCard} anchor={hover.rect} />}
    </div>
  );
}
