import { useMemo, useState } from "react";
import type { Card, CardLink, LinkCard } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { CardImage } from "../../combo/Field";
import { LINK_KINDS, LINK_ZONES } from "./types";

interface Props {
  card: LinkCard | null;
  allCards?: LinkCard[];
  getCard: (code: number) => Card | undefined;
  links: CardLink[];
  onSelectCard: (code: number) => void;
  onGoCombo?: (cardId: number) => void;
  onClose: () => void;
}

function findShortestPath(fromCode: number, toCode: number, allLinks: CardLink[]): CardLink[] | null {
  if (fromCode === toCode) return [];
  const queue: { current: number; path: CardLink[] }[] = [{ current: fromCode, path: [] }];
  const visited = new Set<number>([fromCode]);

  while (queue.length > 0) {
    const { current, path } = queue.shift()!;
    const outgoing = allLinks.filter((l) => l.from === current);
    for (const link of outgoing) {
      if (link.to === toCode) {
        return [...path, link];
      }
      if (!visited.has(link.to)) {
        visited.add(link.to);
        queue.push({ current: link.to, path: [...path, link] });
      }
    }
  }
  return null;
}

export function LinksCardDetail({
  card,
  allCards = [],
  getCard,
  links,
  onSelectCard,
  onGoCombo,
  onClose,
}: Props) {
  const [targetCode, setTargetCode] = useState<number | null>(null);

  const cardData = card ? getCard(card.code) : undefined;
  const cardName = card ? (cardData?.name ?? String(card.code)) : "";

  const outgoing = useMemo(() => (card ? links.filter((l) => l.from === card.code) : []), [card, links]);
  const incoming = useMemo(() => (card ? links.filter((l) => l.to === card.code) : []), [card, links]);

  const shortestPath = useMemo(() => {
    if (!card || !targetCode) return null;
    return findShortestPath(card.code, targetCode, links);
  }, [card, targetCode, links]);

  if (!card) {
    return (
      <div className="flex flex-col items-center justify-center p-6 text-center text-xs text-faint rounded-lg border border-dashed border-border/80 h-full min-h-[300px]">
        <span className="text-2xl mb-2" aria-hidden>🔍</span>
        <p className="font-medium text-text">Nessuna carta selezionata</p>
        <p className="text-[11px] text-muted mt-1 max-w-[200px]">
          Tocca o passa sopra una carta nel grafico per visualizzare le sue connessioni, la raggiungibilità e il valore come carta chiave.
        </p>
      </div>
    );
  }

  const chanceText = (card as any).measured
    ? `~${Math.round(card.chance * 100)}% (stima)`
    : card.extra && card.chance === 0
      ? "—"
      : `${Math.round(card.chance * 100)}%`;

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border bg-surface-2 p-3.5 text-xs anim-fade shadow-sm">
      <div className="flex items-start justify-between gap-2 border-b border-border/70 pb-3">
        <div className="flex gap-2.5 min-w-0">
          <CardImage code={card.code} card={cardData} className="h-[60px] w-[42px] shrink-0 rounded-[3px] shadow-sm" />
          <div className="flex flex-col min-w-0 justify-center">
            <h4 className="font-semibold text-text text-sm truncate leading-tight" title={cardName}>
              {cardName}
            </h4>
            <div className="flex items-center gap-1.5 mt-1 text-[11px] text-muted">
              <span className="chip !py-0 !px-1.5 text-[10px]">
                {card.extra ? "Extra Deck" : "Main Deck"}
              </span>
              <span>×{card.copies} {card.copies === 1 ? "copia" : "copie"}</span>
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={() => {
            sound.play("close");
            onClose();
          }}
          className="text-muted hover:text-text p-1 text-sm leading-none"
          title="Deseleziona carta"
        >
          ✕
        </button>
      </div>

      {/* Metric stats grid */}
      <div className="grid grid-cols-2 gap-2 text-center">
        <div className="rounded-md border border-border/60 bg-surface-3/60 p-2">
          <p className="text-[10px] text-muted uppercase tracking-wider font-medium">Raggiungibilità</p>
          <p className="text-base font-semibold tabular-nums text-text mt-0.5">
            {card.reach} <span className="text-xs text-muted font-normal">/ {card.reachAll}</span>
          </p>
          <p className="text-[9px] text-faint leading-tight mt-0.5">1 passo / tutti i passi</p>
        </div>

        <div className="rounded-md border border-border/60 bg-surface-3/60 p-2">
          <p className="text-[10px] text-muted uppercase tracking-wider font-medium">In mano iniziale</p>
          <p className="text-base font-semibold tabular-nums text-sec mt-0.5">{chanceText}</p>
          <p className="text-[9px] text-faint leading-tight mt-0.5">pescata o raggiunta</p>
        </div>
      </div>

      {/* Key card metric */}
      {card.key > 0.005 ? (
        <div className="rounded-md border border-[var(--df-gold)]/40 bg-[var(--df-gold)]/10 p-2.5">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[11px] font-semibold text-[var(--df-gold)] flex items-center gap-1">
              <span>★</span> Carta chiave del deck
            </span>
            <span className="text-xs font-bold tabular-nums text-[var(--df-gold)]">
              −{card.key.toFixed(2)}
            </span>
          </div>
          <p className="text-[10px] text-muted mt-1 leading-snug">
            Se l'effetto di questa carta viene negato, perdi in media <b>{card.key.toFixed(2)}</b> carte raggiunte dalla mano iniziale.
          </p>
        </div>
      ) : (
        outgoing.length === 0 && incoming.length === 0 && (
          <div className="rounded-md border border-border/60 bg-surface-3/50 p-2 text-[11px] text-muted">
            <span>Carta isolata: nessuna carta del mazzo la raggiunge né la cerca.</span>
          </div>
        )
      )}

      {/* Outgoing links */}
      <div className="flex flex-col gap-1.5">
        <p className="text-[11px] font-semibold text-text uppercase tracking-wider flex items-center justify-between">
          <span>Porta a ({outgoing.length})</span>
          <span className="text-[10px] font-normal text-muted lowercase">in 1 passo</span>
        </p>
        {outgoing.length === 0 ? (
          <p className="text-[11px] text-faint italic">Non porta a nessun'altra carta direttamente.</p>
        ) : (
          <ul className="flex flex-col gap-1 max-h-[130px] overflow-y-auto pr-1">
            {outgoing.map((l, i) => {
              const toCard = getCard(l.to);
              const meta = LINK_KINDS[l.kind];
              const zoneMeta = LINK_ZONES[l.zone];
              return (
                <li
                  key={`${l.from}-${l.to}-${l.kind}-${i}`}
                  className="flex items-center justify-between gap-1.5 rounded-md border border-border/50 bg-surface-3/50 px-2 py-1 text-xs hover:border-sec/50 cursor-pointer transition-colors"
                  onClick={() => {
                    sound.play("page");
                    onSelectCard(l.to);
                  }}
                  title={`Clicca per vedere i collegamenti di ${toCard?.name ?? l.to}`}
                >
                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                    <span className="truncate font-medium text-text">{toCard?.name ?? l.to}</span>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0 text-[10px]">
                    <span className="chip !py-0 !px-1 text-[9px]" style={{ color: meta.color, borderColor: meta.color }}>
                      {meta.verb}
                    </span>
                    <span className="text-faint">{zoneMeta?.short ?? l.zone}</span>
                    {l.used > 0 && (
                      <span className="text-sec font-medium tabular-nums" title={`Usato nel ${Math.round(l.used * 100)}% delle linee migliori`}>
                        {Math.round(l.used * 100)}%
                      </span>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {/* Incoming links */}
      <div className="flex flex-col gap-1.5">
        <p className="text-[11px] font-semibold text-text uppercase tracking-wider flex items-center justify-between">
          <span>Raggiungibile da ({incoming.length})</span>
        </p>
        {incoming.length === 0 ? (
          <p className="text-[11px] text-faint italic">Nessun effetto di questo mazzo porta a questa carta.</p>
        ) : (
          <ul className="flex flex-col gap-1 max-h-[130px] overflow-y-auto pr-1">
            {incoming.map((l, i) => {
              const fromCard = getCard(l.from);
              const meta = LINK_KINDS[l.kind];
              return (
                <li
                  key={`${l.from}-${l.to}-${l.kind}-${i}`}
                  className="flex items-center justify-between gap-1.5 rounded-md border border-border/50 bg-surface-3/50 px-2 py-1 text-xs hover:border-sec/50 cursor-pointer transition-colors"
                  onClick={() => {
                    sound.play("page");
                    onSelectCard(l.from);
                  }}
                  title={`Clicca per vedere i collegamenti di ${fromCard?.name ?? l.from}`}
                >
                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                    <span className="truncate font-medium text-text">{fromCard?.name ?? l.from}</span>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0 text-[10px]">
                    <span className="chip !py-0 !px-1 text-[9px]" style={{ color: meta.color, borderColor: meta.color }}>
                      {meta.verb}
                    </span>
                    {l.used > 0 && (
                      <span className="text-sec font-medium tabular-nums">
                        {Math.round(l.used * 100)}%
                      </span>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {/* Shortest path between cards */}
      {allCards.length > 1 && (
        <div className="flex flex-col gap-1.5 rounded-md border border-border/70 bg-surface-3/40 p-2">
          <p className="text-[10px] font-semibold text-muted uppercase tracking-wider">
            Percorso verso un'altra carta
          </p>
          <select
            value={targetCode ?? ""}
            aria-label="Seleziona carta d'arrivo per il percorso"
            onChange={(e) => {
              const val = e.target.value ? Number(e.target.value) : null;
              setTargetCode(val);
              if (val) sound.play("toggle");
            }}
            className="field !py-1 !px-2 !text-xs w-full"
          >
            <option value="">Seleziona carta d'arrivo…</option>
            {allCards
              .filter((c) => c.code !== card.code)
              .map((c) => (
                <option key={c.code} value={c.code}>
                  {getCard(c.code)?.name ?? c.code} {c.extra ? "(Extra)" : ""}
                </option>
              ))}
          </select>

          {targetCode != null && (
            <div className="flex flex-col gap-1 mt-1 text-[11px]">
              {shortestPath == null ? (
                <p className="text-faint italic text-[10px]">Nessun percorso trovato tra queste due carte.</p>
              ) : shortestPath.length === 0 ? (
                <p className="text-faint italic text-[10px]">Stessa carta selezionata.</p>
              ) : (
                <div className="flex flex-col gap-1.5">
                  <p className="text-[10px] text-muted">
                    Raggiunta in <b>{shortestPath.length} {shortestPath.length === 1 ? "passo" : "passi"}</b>:
                  </p>
                  <div className="flex flex-wrap items-center gap-1">
                    {shortestPath.map((step, idx) => (
                      <span key={idx} className="inline-flex items-center gap-1">
                        {idx > 0 && <span className="text-muted">→</span>}
                        <span className="chip !py-0 !px-1 text-[10px] font-medium" style={{ color: LINK_KINDS[step.kind].color }}>
                          {getCard(step.from)?.name ?? step.from} ({LINK_KINDS[step.kind].verb})
                        </span>
                      </span>
                    ))}
                    <span className="text-muted">→</span>
                    <span className="chip !py-0 !px-1.5 text-[10px] font-bold text-sec">
                      {getCard(targetCode)?.name ?? targetCode}
                    </span>
                  </div>
                  {onGoCombo && (
                    <button
                      type="button"
                      onClick={() => {
                        onGoCombo(targetCode);
                      }}
                      className="btn btn-molten !py-1 !px-2 text-[11px] mt-1 flex items-center justify-center gap-1"
                    >
                      ⚡ Esplora combo con obiettivo {getCard(targetCode)?.name ?? targetCode}
                    </button>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Combo lab button bridge for this card */}
      {onGoCombo && (
        <button
          type="button"
          onClick={() => {
            onGoCombo(card.code);
          }}
          className="btn btn-ghost !py-1.5 text-xs text-sec hover:bg-sec/10 flex items-center justify-center gap-1.5 border border-sec/40"
        >
          <span>⚡ Cerca combo partendo da questa carta</span>
        </button>
      )}
    </div>
  );
}
