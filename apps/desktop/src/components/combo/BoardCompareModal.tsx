import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { api, type Card, ComboBoard, ComboInterruption, ComboScore, ZoneCard } from "../../lib/api";
import { METRICS, NEGATIVE, parts, PROFILES, score, useWeights, metricValues, getPriorWeights, type Weights } from "../../lib/scoring";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { CardImage, Field, type HoverCard, type Pile as PileKind } from "./Field";
import { ScoreBar } from "./Score";

function boardId(b: ComboBoard): string {
  return `${b.hand}:${b.key}`;
}

const fmt2 = (n: number) => n.toLocaleString("it-IT", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const fmt1 = (n: number) => n.toLocaleString("it-IT", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const signed = (n: number) => (n > 1e-4 ? `+${fmt2(n)}` : n < -1e-4 ? `−${fmt2(-n)}` : "0,00");

/** Extracts multiset of card codes from a list of codes. */
function diffMultisets(codesA: number[], codesB: number[]): { onlyA: number[]; shared: number[]; onlyB: number[] } {
  const countA = new Map<number, number>();
  const countB = new Map<number, number>();
  for (const c of codesA) countA.set(c, (countA.get(c) ?? 0) + 1);
  for (const c of codesB) countB.set(c, (countB.get(c) ?? 0) + 1);

  const allCodes = new Set([...countA.keys(), ...countB.keys()]);
  const onlyA: number[] = [];
  const shared: number[] = [];
  const onlyB: number[] = [];

  for (const c of allCodes) {
    const a = countA.get(c) ?? 0;
    const b = countB.get(c) ?? 0;
    const common = Math.min(a, b);
    for (let i = 0; i < common; i++) shared.push(c);
    for (let i = 0; i < a - common; i++) onlyA.push(c);
    for (let i = 0; i < b - common; i++) onlyB.push(c);
  }
  return { onlyA, shared, onlyB };
}

function fieldCodes(b: ComboBoard): number[] {
  const monsters = b.monsters.filter((z): z is ZoneCard => !!z).map((z) => z.code);
  const spells = b.spells.slice(0, 6).filter((z): z is ZoneCard => !!z).map((z) => z.code);
  return [...monsters, ...spells];
}

function CardRow({
  codes,
  emptyLabel,
  getCard,
  onHover,
  onHoverEnd,
}: {
  codes: number[];
  emptyLabel: string;
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
}) {
  if (codes.length === 0) return <span className="text-xs text-faint">{emptyLabel}</span>;
  return (
    <div className="flex flex-wrap gap-1">
      {codes.map((c, i) => (
        <span
          key={`${c}:${i}`}
          className="inline-flex items-center gap-1 rounded-md border border-border bg-surface-2/80 p-1"
          onPointerEnter={(e) => onHover(c, e.currentTarget)}
          onPointerLeave={onHoverEnd}
        >
          <CardImage code={c} card={getCard(c)} className="h-[34px] w-[23px] shrink-0" />
          <span className="max-w-[116px] truncate pr-1 text-[11px] font-medium text-text">{getCard(c)?.name ?? c}</span>
        </span>
      ))}
    </div>
  );
}

function InterruptionMini({
  d,
  getCard,
  onHover,
  onHoverEnd,
}: {
  d: ComboInterruption;
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
}) {
  const card = getCard(d.code);
  return (
    <li className="flex items-center gap-2 rounded-lg border border-border bg-bg-2/70 p-2 text-xs">
      <span className="shrink-0" onPointerEnter={(e) => onHover(d.code, e.currentTarget)} onPointerLeave={onHoverEnd}>
        <CardImage code={d.code} card={card} className="h-[40px] w-[28px]" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-2">
          <b className="truncate text-text">{card?.name ?? d.code}</b>
          <span className="shrink-0 font-semibold tabular-nums">{fmt2(d.value)}</span>
        </div>
        <p className="truncate text-[11px] text-muted">
          {d.effect} ({d.place}) · {d.uses === 1 ? "1 volta" : `${d.uses} volte`} · copre {Math.round(d.coverage * 100)}%
        </p>
      </div>
    </li>
  );
}

const PILE_TITLES: Record<PileKind, string> = { grave: "Cimitero", banished: "Bandite", deck: "Deck", extra: "Extra Deck (scoperte)" };

export function BoardCompareModal({
  open,
  deckId,
  boardA,
  rankA,
  scoreA,
  boardB,
  rankB,
  scoreB,
  weights,
  getCard,
  onHover,
  onHoverEnd,
  onSwap,
  onClose,
  initialChosenId,
  onChoiceRecorded,
}: {
  deckId?: number | null;
  initialChosenId?: string | null;
  onChoiceRecorded?: (chosenId: string, count: number) => void;
  open: boolean;
  boardA: ComboBoard;
  rankA: number;
  scoreA: ComboScore | null;
  boardB: ComboBoard;
  rankB: number;
  scoreB: ComboScore | null;
  weights: Weights;
  getCard: (id: number) => Card | undefined;
  onHover: HoverCard;
  onHoverEnd: () => void;
  onSwap: () => void;
  onClose: () => void;
}) {
  const [pileA, setPileA] = useState<PileKind | null>(null);
  const [pileB, setPileB] = useState<PileKind | null>(null);
  const { profile } = useWeights();
  const [choicesCount, setChoicesCount] = useState<number | null>(null);
  const [chosenBoardId, setChosenBoardId] = useState<string | null>(initialChosenId ?? null);
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    setChosenBoardId(initialChosenId ?? null);
  }, [initialChosenId]);

  useEffect(() => {
    if (!open) return;
    let active = true;
    const prior = getPriorWeights();
    api.learnedWeights(prior)
      .then((lw) => {
        if (active) setChoicesCount(lw.choices);
      })
      .catch((e) => {
        if (active) {
          toast(`Impossibile leggere le preferenze: ${e instanceof Error ? e.message : String(e)}`, "error");
        }
      });
    return () => {
      active = false;
    };
  }, [open]);

  const handlePrefer = async (which: "A" | "B") => {
    if (!scoreA || !scoreB || scoreA.error || scoreB.error || recording || chosenBoardId !== null) return;
    setRecording(true);
    sound.play("chime");
    try {
      const chosen = which === "A" ? metricValues(scoreA) : metricValues(scoreB);
      const other = which === "A" ? metricValues(scoreB) : metricValues(scoreA);
      const chosenId = which === "A" ? boardId(boardA) : boardId(boardB);
      const count = await api.recordBoardChoice(deckId ?? null, chosen, other);
      setChoicesCount(count);
      setChosenBoardId(chosenId);
      onChoiceRecorded?.(chosenId, count);
    } catch (e) {
      sound.play("error");
      toast(`Impossibile registrare la preferenza: ${e instanceof Error ? e.message : String(e)}`, "error");
    } finally {
      setRecording(false);
    }
  };

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        sound.play("close");
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const profileName = PROFILES.find((p) => p.id === profile)?.label;
  const totalA = scoreA && !scoreA.error ? score(scoreA, weights) : null;
  const totalB = scoreB && !scoreB.error ? score(scoreB, weights) : null;
  const partsA = scoreA && !scoreA.error ? parts(scoreA, weights) : null;
  const partsB = scoreB && !scoreB.error ? parts(scoreB, weights) : null;
  const posScale = Math.max(
    0.01,
    partsA ? partsA.filter((p) => p.value > 0).reduce((t, p) => t + p.value, 0) : 0,
    partsB ? partsB.filter((p) => p.value > 0).reduce((t, p) => t + p.value, 0) : 0,
  );

  const fieldDiff = diffMultisets(fieldCodes(boardA), fieldCodes(boardB));
  const handDiff = diffMultisets(boardA.handCards, boardB.handCards);
  const graveDiff = diffMultisets(boardA.grave, boardB.grave);

  const pileCodes = (b: ComboBoard, p: PileKind): number[] =>
    p === "grave" ? b.grave : p === "banished" ? b.banished : p === "extra" ? b.extraFaceUp : [];

  return createPortal(
    <div
      data-section="combo"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 xl:p-6 backdrop-blur-sm anim-fade"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) {
          sound.play("close");
          onClose();
        }
      }}
      role="dialog"
      aria-modal="true"
      aria-labelledby="board-compare-title"
    >
      <div className="panel flex max-h-[90vh] w-full max-w-6xl flex-col overflow-hidden border-border-strong">
        {/* Header */}
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-surface-2/70 px-6 py-3.5">
          <div>
            <p className="eyebrow">Circolo Arcano · Confronto diretto</p>
            <div className="flex items-center gap-2 mt-0.5">
              <h2 id="board-compare-title" className="display text-lg font-bold">
                Board #{rankA} <span className="text-muted">vs</span> Board #{rankB}
              </h2>
              {choicesCount != null && choicesCount > 0 && (
                <span className="chip !py-0.5 !px-2 text-[11px] text-muted">
                  ★ {choicesCount} {choicesCount === 1 ? "scelta registrata" : "scelte registrate"}
                </span>
              )}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => {
                sound.play("toggle");
                onSwap();
              }}
              className="btn btn-ghost !px-3 !py-1.5 text-xs"
              title="Scambia l'ordine delle due board"
            >
              ⇄ Inverti A e B
            </button>
            <button
              type="button"
              onClick={() => {
                sound.play("close");
                onClose();
              }}
              className="btn btn-ghost !px-3 !py-1.5 text-xs text-muted hover:text-text"
              aria-label="Chiudi confronto"
            >
              ✕ Chiudi
            </button>
          </div>
        </div>

        {/* Scrollable body */}
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <div className="flex flex-col gap-5">
            {/* 1. Side-by-side summary + playmats */}
            <div className="grid grid-cols-2 gap-4">
              {(
                [
                  { label: `Board #${rankA} (Riferimento A)`, b: boardA, s: scoreA, total: totalA, pile: pileA, setPile: setPileA },
                  { label: `Board #${rankB} (Confronto B)`, b: boardB, s: scoreB, total: totalB, pile: pileB, setPile: setPileB },
                ] as const
              ).map((col, idx) => (
                <section key={idx} className="lab-card flex flex-col gap-3 p-4">
                  <div className="flex flex-wrap items-baseline justify-between gap-2 border-b border-border pb-2.5">
                    <div>
                      <p className="eyebrow text-sec">{col.label}</p>
                      <p className="mt-0.5 text-xs text-muted tabular-nums">
                        {col.b.cardsUsed} {col.b.cardsUsed === 1 ? "carta usata" : "carte usate"} dalla mano · {col.b.choices} scelte ·{" "}
                        {col.b.handCards.length} in mano a fine turno
                      </p>
                    </div>
                    <div className="text-right tabular-nums">
                      {col.total != null ? (
                        <div>
                          <span className="display text-xl font-bold text-text">{fmt2(col.total)}</span>
                          <span className="ml-1 text-[11px] text-muted">pt</span>
                        </div>
                      ) : (
                        <div>
                          <span className="display text-lg font-bold text-faint">{fmt1(col.b.quickScore)}</span>
                          <span className="ml-1 text-[10px] text-faint">stima</span>
                        </div>
                      )}
                    </div>
                  </div>

                  {col.s && !col.s.error && <ScoreBar s={col.s} weights={weights} scale={posScale} />}

                  {(() => {
                    const isChosen = chosenBoardId === boardId(col.b);
                    const isOther = chosenBoardId !== null && !isChosen;
                    return (
                      <div className="flex items-center justify-between gap-2 border-b border-border/60 pb-2">
                        <button
                          type="button"
                          disabled={recording || chosenBoardId !== null || !scoreA || !scoreB || !!scoreA.error || !!scoreB.error}
                          onClick={() => void handlePrefer(idx === 0 ? "A" : "B")}
                          className={`btn !py-1 !px-2.5 text-xs flex items-center gap-1.5 transition-all shadow-xs ${
                            isChosen
                              ? "btn-molten font-semibold"
                              : isOther
                                ? "btn-ghost opacity-50 cursor-not-allowed"
                                : "btn-ghost border border-sec/40 hover:bg-sec/15 text-sec"
                          }`}
                          title={
                            isChosen
                              ? "Hai preferito questa board"
                              : isOther
                                ? "Hai già registrato la tua preferenza per questo confronto"
                                : "Registra che preferisci questa board rispetto all'altra per calibrare i tuoi pesi personali"
                          }
                        >
                          <span>{isChosen ? "★ Hai preferito questa" : "★ Preferisco questa"}</span>
                        </button>
                      </div>
                    );
                  })()}

                  <Field
                    board={col.b}
                    getCard={getCard}
                    onHover={onHover}
                    onHoverEnd={onHoverEnd}
                    pile={col.pile}
                    onPile={(p) => col.setPile((cur) => (cur === p ? null : p))}
                  />

                  <div className="border-t border-border pt-2.5">
                    <p className="eyebrow mb-1 text-faint">Mano a fine turno · {col.b.handCards.length}</p>
                    <CardRow codes={col.b.handCards} emptyLabel="Nessuna carta in mano" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                  </div>

                  {col.pile && (
                    <div className="border-t border-border pt-2.5 anim-fade">
                      <p className="eyebrow mb-1 text-faint">
                        {PILE_TITLES[col.pile]} · {pileCodes(col.b, col.pile).length}
                      </p>
                      <CardRow
                        codes={pileCodes(col.b, col.pile)}
                        emptyLabel="Nessuna carta"
                        getCard={getCard}
                        onHover={onHover}
                        onHoverEnd={onHoverEnd}
                      />
                    </div>
                  )}
                </section>
              ))}
            </div>

            {/* 2. Card differences on Field, Hand, and Graveyard */}
            <section className="lab-card flex flex-col gap-3 p-4">
              <div>
                <p className="eyebrow">Differenze sulle carte</p>
                <h3 className="text-sm font-semibold">Cosa cambia sul Campo, in Mano e nel Cimitero</h3>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div className="rounded-xl border border-border bg-bg-2/60 p-3">
                  <p className="eyebrow mb-2 text-sec">Solo in Board #{rankA}</p>
                  <div className="flex flex-col gap-2">
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">Sul campo ({fieldDiff.onlyA.length})</p>
                      <CardRow codes={fieldDiff.onlyA} emptyLabel="Nessuna carta esclusiva" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">In mano ({handDiff.onlyA.length})</p>
                      <CardRow codes={handDiff.onlyA} emptyLabel="—" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">Nel Cimitero ({graveDiff.onlyA.length})</p>
                      <CardRow codes={graveDiff.onlyA} emptyLabel="—" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                  </div>
                </div>

                <div className="rounded-xl border border-border bg-bg-2/40 p-3">
                  <p className="eyebrow mb-2 text-faint">In entrambe le board</p>
                  <div className="flex flex-col gap-2">
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">Sul campo ({fieldDiff.shared.length})</p>
                      <CardRow codes={fieldDiff.shared} emptyLabel="Nessuna carta in comune" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">In mano ({handDiff.shared.length})</p>
                      <CardRow codes={handDiff.shared} emptyLabel="—" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                  </div>
                </div>

                <div className="rounded-xl border border-border bg-bg-2/60 p-3">
                  <p className="eyebrow mb-2 text-sec">Solo in Board #{rankB}</p>
                  <div className="flex flex-col gap-2">
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">Sul campo ({fieldDiff.onlyB.length})</p>
                      <CardRow codes={fieldDiff.onlyB} emptyLabel="Nessuna carta esclusiva" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">In mano ({handDiff.onlyB.length})</p>
                      <CardRow codes={handDiff.onlyB} emptyLabel="—" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                    <div>
                      <p className="mb-1 text-[11px] font-semibold text-muted">Nel Cimitero ({graveDiff.onlyB.length})</p>
                      <CardRow codes={graveDiff.onlyB} emptyLabel="—" getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                    </div>
                  </div>
                </div>
              </div>
            </section>

            {/* 3. Score & Metrics Comparison (from ComboScore + METRICS) */}
            <section className="lab-card flex flex-col gap-3 p-4">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <div>
                  <p className="eyebrow">Turno di prova · Metriche a confronto</p>
                  <h3 className="text-sm font-semibold">
                    Punteggio e singole voci {profileName ? `(profilo «${profileName}»)` : profile === "learned" ? "(pesi imparati)" : "(pesi personalizzati)"}
                  </h3>
                </div>
                <span className="text-xs text-muted tabular-nums">
                  Scelte linea: <b>{boardA.choices}</b> vs <b>{boardB.choices}</b> · Carte mano usate: <b>{boardA.cardsUsed}</b> vs <b>{boardB.cardsUsed}</b>
                </span>
              </div>

              {scoreA && !scoreA.error && scoreB && !scoreB.error ? (
                <div className="overflow-x-auto">
                  <table className="w-full border-collapse text-xs">
                    <thead>
                      <tr className="border-b border-border text-left text-[11px] text-faint uppercase">
                        <th className="py-2 pr-3">Voce</th>
                        <th className="px-3 py-2 text-right">Misura #{rankA}</th>
                        <th className="px-3 py-2 text-right">Misura #{rankB}</th>
                        <th className="px-3 py-2 text-right">Punti #{rankA}</th>
                        <th className="px-3 py-2 text-right">Punti #{rankB}</th>
                        <th className="py-2 pl-3 text-right">Diff. punti (B − A)</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-border/60 tabular-nums">
                      {METRICS.map((m) => {
                        const rawA = scoreA[m.key];
                        const rawB = scoreB[m.key];
                        const w = weights[m.key];
                        const ptA = rawA * w;
                        const ptB = rawB * w;
                        const deltaPt = ptB - ptA;
                        const isNeg = NEGATIVE.includes(m.key);
                        return (
                          <tr key={m.key} className="hover:bg-surface-2/50">
                            <td className="py-2 pr-3">
                              <div className="flex items-center gap-2">
                                <span className="h-2.5 w-2.5 shrink-0 rounded-sm" style={{ background: m.color }} />
                                <span className={`font-medium ${isNeg ? "text-danger" : "text-text"}`}>{m.label}</span>
                              </div>
                            </td>
                            <td className="px-3 py-2 text-right text-muted">{fmt2(rawA)}</td>
                            <td className="px-3 py-2 text-right text-muted">{fmt2(rawB)}</td>
                            <td className="px-3 py-2 text-right font-medium">{signed(ptA)}</td>
                            <td className="px-3 py-2 text-right font-medium">{signed(ptB)}</td>
                            <td
                              className={`py-2 pl-3 text-right font-semibold ${
                                deltaPt > 1e-4 ? "text-ok" : deltaPt < -1e-4 ? "text-danger" : "text-faint"
                              }`}
                            >
                              {signed(deltaPt)}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                    <tfoot>
                      <tr className="border-t border-border-strong font-semibold text-text tabular-nums">
                        <td className="pt-2.5 pr-3">Totale</td>
                        <td className="px-3 pt-2.5 text-right text-faint">—</td>
                        <td className="px-3 pt-2.5 text-right text-faint">—</td>
                        <td className="px-3 pt-2.5 text-right">{totalA != null ? fmt2(totalA) : "—"}</td>
                        <td className="px-3 pt-2.5 text-right">{totalB != null ? fmt2(totalB) : "—"}</td>
                        <td
                          className={`pt-2.5 pl-3 text-right ${
                            totalA != null && totalB != null
                              ? totalB - totalA > 1e-4
                                ? "text-ok"
                                : totalB - totalA < -1e-4
                                  ? "text-danger"
                                  : "text-faint"
                              : "text-faint"
                          }`}
                        >
                          {totalA != null && totalB != null ? signed(totalB - totalA) : "—"}
                        </td>
                      </tr>
                    </tfoot>
                  </table>
                </div>
              ) : (
                <p className="text-xs text-faint">
                  Il dettaglio completo delle metriche compare quando entrambe le board sono state messe alla prova dal turno di prova.
                </p>
              )}
            </section>

            {/* 4. Interruptions, Floodgates & Exposure side-by-side */}
            {scoreA && !scoreA.error && scoreB && !scoreB.error && (
              <div className="grid grid-cols-2 gap-4">
                {(
                  [
                    { label: `Board #${rankA}`, s: scoreA },
                    { label: `Board #${rankB}`, s: scoreB },
                  ] as const
                ).map((col, idx) => (
                  <section key={idx} className="lab-card flex flex-col gap-3 p-4">
                    <h4 className="text-xs font-semibold tracking-wide text-sec uppercase">
                      {col.label} · Cosa ferma l'avversario ({col.s.details.length})
                    </h4>
                    {col.s.details.length === 0 ? (
                      <p className="text-xs text-faint">Nessun effetto attivo registrato nel turno di prova.</p>
                    ) : (
                      <ul className="flex flex-col gap-1.5">
                        {col.s.details.map((d, i) => (
                          <InterruptionMini key={i} d={d} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} />
                        ))}
                      </ul>
                    )}

                    {col.s.floodgateList.length > 0 && (
                      <div className="border-t border-border pt-2.5">
                        <p className="eyebrow mb-1.5 text-faint">Floodgate attivi</p>
                        <div className="flex flex-wrap gap-1">
                          {col.s.floodgateList.map((f) => (
                            <span key={f} className="chip !py-0.5 text-[11px]">
                              {f}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}

                    {col.s.exposureDetail && (
                      <div className="border-t border-border pt-2.5 text-xs text-muted">
                        <p className="eyebrow mb-1 text-faint">Esposizione durante la linea</p>
                        <div className="flex flex-wrap gap-x-4 gap-y-1 tabular-nums">
                          <span>Ash: <b className="text-text">{col.s.exposureDetail.ash}</b></span>
                          <span>Sul campo (Veiler/Imperm): <b className="text-text">{col.s.exposureDetail.onField}</b></span>
                          <span>Evocazioni: <b className="text-text">{col.s.exposureDetail.summons}</b> ({col.s.exposureDetail.specialSummons} Spec.)</span>
                          <span>Ricerche dal Deck: <b className="text-text">{col.s.exposureDetail.searches}</b></span>
                        </div>
                      </div>
                    )}
                  </section>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
