import { useCallback, useEffect, useMemo, useState } from "react";
import { api, cardImage, type Card, type Deck } from "../lib/api";
import { pAtLeastOne, pBoth, percent, shuffle } from "../lib/probability";
import { BUILTIN_ROLES } from "../lib/roles";
import { sound } from "../lib/sound";
import { CardTile } from "./CardTile";
import { Modal } from "./Modal";

interface Stats {
  hands: number;
  starter: number;
  extender: number;
  handtrap: number;
  both: number;
  noBrick: number;
}

const NO_STATS: Stats = { hands: 0, starter: 0, extender: 0, handtrap: 0, both: 0, noBrick: 0 };

interface Props {
  deck: Deck;
  getCard: (id: number) => Card | undefined;
  onRoles: (roles: Record<string, number[]>) => void;
  guessing?: { done: number; total: number } | null;
  onSuggestRoles?: () => void;
  onTryInCombo?: (hand: number[]) => void;
  onClose: () => void;
}

/**
 * Draws opening hands from the Main Deck. Cards marked with built-in roles
 * are counted over the hands drawn, next to the exact probability, and any
 * drawn hand can be sent directly to the Arcane Circle (Combo Lab).
 */
export function TestHand({ deck, getCard, onRoles, guessing, onSuggestRoles, onTryInCombo, onClose }: Props) {
  const [first, setFirst] = useState(true);
  const [order, setOrder] = useState<number[]>([]);
  const [drawn, setDrawn] = useState(0);
  const [nonce, setNonce] = useState(0);
  const [stats, setStats] = useState<Stats>(NO_STATS);
  const opening = first ? 5 : 6;

  const roles = useMemo(
    () => ({
      starter: new Set(deck.roles.starter ?? []),
      extender: new Set(deck.roles.extender ?? []),
      handtrap: new Set(deck.roles.handtrap ?? []),
      breaker: new Set(deck.roles.breaker ?? []),
      brick: new Set(deck.roles.brick ?? []),
    }),
    [deck.roles],
  );

  // The first time, handtraps are pre-marked from the catalog's Handtrap group.
  useEffect(() => {
    if (deck.roles.handtrap) return;
    api
      .search("tag:handtrap")
      .then((r) => {
        const inDeck = new Set(deck.main);
        onRoles({ ...deck.roles, handtrap: r.ids.filter((id) => inDeck.has(id)) });
        setStats(NO_STATS);
      })
      .catch(() => {});
  }, [deck.roles, deck.main, onRoles]);

  const deal = useCallback(
    (count: number) => {
      const next = shuffle(deck.main);
      setOrder(next);
      setDrawn(Math.min(count, next.length));
      setNonce((n) => n + 1);
      sound.play("page");
      const hand = next.slice(0, count);
      const s = hand.some((id) => roles.starter.has(id));
      const e = hand.some((id) => roles.extender.has(id));
      const h = hand.some((id) => roles.handtrap.has(id));
      const b = hand.some((id) => roles.brick.has(id));
      setStats((st) => ({
        hands: st.hands + 1,
        starter: st.starter + +s,
        extender: st.extender + +e,
        handtrap: st.handtrap + +h,
        both: st.both + +(s && h),
        noBrick: st.noBrick + +!b,
      }));
    },
    [deck.main, roles],
  );

  useEffect(() => {
    // Only on open and when going first/second changes, not when roles change.
    if (deck.main.length) deal(opening);
  }, [opening]); // eslint-disable-line react-hooks/exhaustive-deps

  const drawOne = useCallback(() => {
    if (drawn >= order.length) return;
    sound.play("select");
    setDrawn((d) => d + 1);
  }, [drawn, order.length]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest("input, textarea")) return;
      if (e.key === "n" || e.key === "N" || e.key === " ") {
        e.preventDefault();
        deal(opening);
      } else if (e.key === "d" || e.key === "D") {
        drawOne();
      }
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [deal, drawOne, opening]);

  const toggleRole = (roleKey: string, id: number) => {
    sound.play("toggle");
    const current = new Set(deck.roles[roleKey] ?? []);
    if (current.has(id)) current.delete(id);
    else current.add(id);
    onRoles({ ...deck.roles, [roleKey]: [...current] });
    setStats(NO_STATS);
  };

  const size = deck.main.length;
  const copies = (set: Set<number>) => deck.main.filter((id) => set.has(id)).length;
  const starters = copies(roles.starter);
  const extenders = copies(roles.extender);
  const handtraps = copies(roles.handtrap);
  const bricks = copies(roles.brick);
  const eitherSH = deck.main.filter((id) => roles.starter.has(id) || roles.handtrap.has(id)).length;
  const exact = {
    starter: pAtLeastOne(size, starters, opening),
    extender: pAtLeastOne(size, extenders, opening),
    handtrap: pAtLeastOne(size, handtraps, opening),
    both: pBoth(size, starters, handtraps, eitherSH, opening),
    noBrick: size ? Math.max(0, 1 - pAtLeastOne(size, bricks, opening)) : 0,
  };
  const distinct = [...new Set(deck.main)];
  const hand = order.slice(0, drawn);

  const badge = (id: number) => {
    const text = BUILTIN_ROLES.filter((r) => roles[r.key as keyof typeof roles]?.has(id))
      .map((r) => r.letter)
      .join("·");
    return text ? { text } : undefined;
  };

  const row = (label: string, seen: number, p: number) => (
    <div className="grid grid-cols-[1fr_auto_auto] items-baseline gap-4 border-t border-border py-1.5 text-sm">
      <span className="text-muted">{label}</span>
      <span className="whitespace-nowrap tabular-nums" title="Nelle mani pescate qui">
        {stats.hands ? `${seen}/${stats.hands} · ${percent(seen / stats.hands)}` : "—"}
      </span>
      <span className="w-16 text-right font-semibold text-sec tabular-nums" title="Probabilità esatta">
        {percent(p)}
      </span>
    </div>
  );

  return (
    <Modal title="Mano di prova" onClose={onClose} wide>
      <div className="grid min-h-0 grid-cols-[minmax(0,1fr)_330px] gap-6">
        <div className="flex min-w-0 flex-col gap-4">
          <div className="flex flex-wrap items-center gap-2.5">
            <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1">
              {[true, false].map((f) => (
                <button
                  key={String(f)}
                  type="button"
                  onClick={() => {
                    sound.play("toggle");
                    setStats(NO_STATS);
                    setFirst(f);
                  }}
                  className={`rounded-lg px-3 py-1 text-xs font-semibold ${first === f ? "bg-sec text-bg" : "text-muted hover:text-text"}`}
                >
                  {f ? "Vai per primo (5)" : "Vai per secondo (6)"}
                </button>
              ))}
            </div>
            <button type="button" className="btn btn-molten" onClick={() => deal(opening)} disabled={!size}>
              Nuova mano <kbd className="text-[10px] opacity-70">N</kbd>
            </button>
            <button type="button" className="btn btn-ghost" onClick={drawOne} disabled={drawn >= order.length}>
              Pesca <kbd className="text-[10px] opacity-70">D</kbd>
            </button>
            {onTryInCombo && (
              <button
                type="button"
                className="btn btn-ghost border-sec/40 text-sec hover:border-sec"
                disabled={hand.length === 0}
                onClick={() => onTryInCombo(hand.slice(0, 6))}
                title="Invia questa mano (fino a 6 carte) al Circolo Arcano per calcolare le combo di primo turno"
              >
                ✦ Prova nel Circolo Arcano
              </button>
            )}
            <span className="ml-auto text-xs text-faint tabular-nums">{size ? `${size - drawn} nel deck` : "Main Deck vuoto"}</span>
          </div>

          <div className="flex min-h-[200px] flex-wrap content-start gap-3 rounded-xl border border-border bg-bg-2/70 p-4">
            {hand.map((id, i) => (
              <div key={`${nonce}-${i}`} className="deal w-[112px]" style={{ animationDelay: i < opening ? `${i * 70}ms` : "0ms" }}>
                <CardTile id={id} card={getCard(id)} badge={badge(id)} />
              </div>
            ))}
          </div>

          <div className="rounded-xl border border-border bg-bg-2/40 p-4">
            <div className="mb-1 flex items-baseline justify-between">
              <p className="eyebrow">Probabilità nella mano iniziale ({opening} carte)</p>
              <span className="text-[11px] text-faint">pescate qui · esatta</span>
            </div>
            {row(`Almeno 1 Starter (${starters} nel deck)`, stats.starter, exact.starter)}
            {row(`Almeno 1 Estensore (${extenders} nel deck)`, stats.extender, exact.extender)}
            {row(`Almeno 1 Handtrap (${handtraps} nel deck)`, stats.handtrap, exact.handtrap)}
            {row("Starter + Handtrap insieme", stats.both, exact.both)}
            {row(`Mano senza Brick (${bricks} nel deck)`, stats.noBrick, exact.noBrick)}
            <p className="mt-2 text-[11px] text-faint">
              La probabilità esatta è ipergeometrica: {opening} carte da {size || "—"}. Le percentuali delle mani pescate convergono al valore esatto con molte
              mani.
            </p>
          </div>
        </div>

        <aside className="flex min-h-0 flex-col gap-2">
          <div className="flex items-center justify-between gap-2">
            <p className="eyebrow">Ruoli nel deck</p>
            {onSuggestRoles && (
              <button
                type="button"
                disabled={guessing != null || size === 0}
                onClick={() => {
                  onSuggestRoles();
                  setStats(NO_STATS);
                }}
                className="btn btn-ghost !px-2 !py-0.5 text-[11px]"
                title="Riconosci automaticamente i ruoli delle carte tramite il motore"
              >
                {guessing ? `Analisi ${guessing.done}/${guessing.total || "…"}` : "✦ Riconosci"}
              </button>
            )}
          </div>
          <p className="text-[11px] text-faint">
            {BUILTIN_ROLES.map((r) => `${r.letter} = ${r.label}`).join(" · ")}. Clicca per assegnare o rimuovere un ruolo.
          </p>
          <div className="min-h-0 flex-1 overflow-y-auto pr-1" style={{ maxHeight: "52vh" }}>
            {distinct.map((id) => {
              const card = getCard(id);
              const n = deck.main.filter((x) => x === id).length;
              return (
                <div key={id} className="flex items-center gap-1.5 rounded-lg px-1 py-1 hover:bg-surface-3">
                  <img src={cardImage(id, "small")} alt="" className="h-9 w-[25px] shrink-0 rounded-[2px] object-cover" />
                  <span className="min-w-0 flex-1 truncate text-xs">
                    {card?.name ?? id} <span className="text-faint">×{n}</span>
                  </span>
                  {BUILTIN_ROLES.map((r) => {
                    const has = roles[r.key as keyof typeof roles]?.has(id) ?? false;
                    return (
                      <button
                        key={r.key}
                        type="button"
                        onClick={() => toggleRole(r.key, id)}
                        aria-pressed={has}
                        title={`${r.label}: ${r.hint}`}
                        style={has ? { background: r.color, borderColor: r.color, color: "#fff" } : undefined}
                        className={`grid h-5 w-5 shrink-0 place-items-center rounded border text-[10px] font-bold transition ${
                          has ? "" : "border-border text-faint hover:text-text"
                        }`}
                      >
                        {r.letter}
                      </button>
                    );
                  })}
                </div>
              );
            })}
          </div>
        </aside>
      </div>
    </Modal>
  );
}
