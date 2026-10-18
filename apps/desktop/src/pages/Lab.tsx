import { useCallback, useEffect, useRef, useState } from "react";
import { CalculatorTab } from "../components/lab/CalculatorTab";
import { OverviewTab } from "../components/lab/OverviewTab";
import { RolesPanel } from "../components/lab/RolesPanel";
import { Popover, useOutsideClose } from "../components/Popover";
import { SectionHeader } from "../components/SectionHeader";
import { api, type Deck, type DeckSummary, type LabOverview } from "../lib/api";
import { resolveLook } from "../lib/boxes";
import { useCardCache } from "../lib/cards";
import { FORMAT_LABELS, lastDeck, rememberDeck } from "../lib/deck";
import { sound } from "../lib/sound";
import { toast } from "../lib/toast";
import { sectionById } from "../sections";

type Tab = "overview" | "calculator" | "simulator" | "optimizer" | "log";

const TABS: { id: Tab; label: string; soon?: string }[] = [
  { id: "overview", label: "Panoramica" },
  { id: "calculator", label: "Calcolatore" },
  { id: "simulator", label: "Simulatore", soon: "4B" },
  { id: "optimizer", label: "Ottimizzatore", soon: "4B" },
  { id: "log", label: "Registro partite", soon: "4C" },
];

const SOON: Record<string, string> = {
  simulator: "Monte Carlo con Pot, ricerche, combo e conflitti di Normal Summon, con intervallo d'errore e convergenza in diretta.",
  optimizer: "Valore di ogni copia, 40 contro 41–45 carte, mappa starter × handtrap e proposte di modifica.",
  log: "Registro delle partite con win-rate per matchup, primo/secondo, side e quante partite servono per capirlo.",
};

/** Il Laboratorio: statistics on a deck, built on the roles of its cards. */
export function Lab({ artReady, onGoForge }: { artReady: boolean; onGoForge: () => void }) {
  const [decks, setDecks] = useState<DeckSummary[] | null>(null);
  const [deck, setDeck] = useState<Deck | null>(null);
  const [roles, setRoles] = useState<Record<string, number[]>>({});
  const [overview, setOverview] = useState<LabOverview | null>(null);
  const [hand, setHand] = useState<"first" | "second">("first");
  const [tab, setTab] = useState<Tab>("overview");
  const [menu, setMenu] = useState(false);
  const menuRef = useOutsideClose(menu, () => setMenu(false));
  const saveTimer = useRef<number>(undefined);
  const cards = useCardCache();
  const { request } = cards;

  const open = useCallback(
    async (id: number) => {
      const d = await api.getDeck(id);
      if (!d) return;
      rememberDeck(id);
      setDeck(d);
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
    api.labOverview(deck.main, roles).then((o) => !cancelled && setOverview(o));
    return () => {
      cancelled = true;
    };
  }, [deck, roles]);

  const changeRoles = (next: Record<string, number[]>) => {
    setRoles(next);
    if (deck?.id == null) return;
    const id = deck.id;
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      api.setDeckRoles(id, next).catch((e) => toast(`Ruoli non salvati: ${e}`, "error"));
    }, 400);
  };

  const suggest = async () => {
    if (!deck) return;
    const inDeck = new Set(deck.main);
    const [ht, br] = await Promise.all([api.search("tag:handtrap"), api.search("tag:breaker")]);
    const pick = (ids: number[]) => ids.filter((id) => inDeck.has(id));
    const merge = (key: string, ids: number[]) => [...new Set([...(roles[key] ?? []), ...ids])];
    const handtraps = pick(ht.ids);
    const breakers = pick(br.ids);
    changeRoles({ ...roles, handtrap: merge("handtrap", handtraps), breaker: merge("breaker", breakers) });
    sound.play("glass");
    toast(`Segnate ${handtraps.length} handtrap e ${breakers.length} board breaker`, "ok");
  };

  const section = sectionById("stats");
  const current = decks?.find((d) => d.id === deck?.id);

  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={artReady} />
      <div className="relative z-20 flex flex-wrap items-center gap-3 border-b border-border bg-surface/70 px-6 py-2.5 backdrop-blur">
        <div ref={menuRef} className="relative">
          <button
            type="button"
            className="btn btn-ghost !py-1.5"
            onClick={() => {
              sound.play(menu ? "close" : "open");
              setMenu((v) => !v);
            }}
            aria-expanded={menu}
            disabled={!decks?.length}
          >
            {current && <span className="h-5 w-4 rounded-[3px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.25)]" style={{ background: resolveLook(current.box, current.id).color }} />}
            <span className="max-w-64 truncate">{current?.name ?? "Scegli un deck"}</span>
            {current && <span className="text-xs text-faint">{FORMAT_LABELS[current.format]}</span>}
            <span className="text-faint">▾</span>
          </button>
          <Popover open={menu} width={320}>
            {decks?.map((d) => (
              <button
                key={d.id}
                type="button"
                onClick={() => {
                  setMenu(false);
                  sound.play("page");
                  void open(d.id);
                }}
                className={`flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-surface-3 ${d.id === deck?.id ? "bg-surface-3" : ""}`}
              >
                <span className="h-7 w-5 shrink-0 rounded-[3px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.25)]" style={{ background: resolveLook(d.box, d.id).color }} />
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
        <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1">
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
        <nav className="flex gap-1" aria-label="Schede del laboratorio">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => {
                sound.play("page");
                setTab(t.id);
              }}
              className={`relative rounded-lg px-3 py-1.5 text-sm ${tab === t.id ? "bg-surface-3 text-text shadow-[inset_0_-2px_0_var(--sec)]" : "text-muted hover:text-text"}`}
            >
              {t.label}
              {t.soon && <span className="ml-1.5 rounded-full bg-surface-3 px-1.5 text-[9px] font-bold tracking-wider text-faint">{t.soon}</span>}
            </button>
          ))}
        </nav>
        {overview && <span className="ml-auto text-xs text-faint tabular-nums">{overview.size} carte nel Main Deck</span>}
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
        <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_330px]">
          <div key={tab} className="bg-blueprint min-h-0 overflow-y-auto px-6 py-5">
            {!deck || !overview ? (
              <p className="text-sm text-muted">Carico il deck…</p>
            ) : tab === "overview" ? (
              <OverviewTab overview={overview} hand={hand} roles={roles} getCard={cards.get} />
            ) : tab === "calculator" ? (
              <CalculatorTab deckId={deck.id!} main={deck.main} roles={roles} hand={hand} getCard={cards.get} />
            ) : (
              <div className="lab-card mx-auto mt-6 max-w-xl p-8 text-center anim-rise">
                <p className="eyebrow text-sec">In arrivo nella Fase {TABS.find((t) => t.id === tab)?.soon}</p>
                <p className="display mt-2 text-xl">{TABS.find((t) => t.id === tab)?.label}</p>
                <p className="mt-2 text-sm text-muted">{SOON[tab]}</p>
              </div>
            )}
          </div>
          {deck && <RolesPanel main={deck.main} roles={roles} getCard={cards.get} onChange={changeRoles} onSuggest={() => void suggest()} />}
        </div>
      )}
    </div>
  );
}
