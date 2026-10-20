import { useState } from "react";
import { cardImage, type Card, type ComboBoard, type ZoneCard } from "../../lib/api";

export type HoverCard = (code: number, el: HTMLElement) => void;

/** A card image that falls back to its name (tokens have no artwork). */
export function CardImage({ code, card, className = "" }: { code: number; card?: Card; className?: string }) {
  const [failed, setFailed] = useState(false);
  return failed ? (
    <span className={`flex items-center justify-center rounded-[3px] bg-surface-3 p-0.5 text-center text-[8px] leading-tight text-muted ${className}`}>{card?.name ?? code}</span>
  ) : (
    <img src={cardImage(code, "small")} alt={card?.name ?? ""} loading="lazy" draggable={false} onError={() => setFailed(true)} className={`rounded-[3px] object-cover ${className}`} />
  );
}

/** Rank or level, or the Link rating. */
function stars(z: ZoneCard): string {
  if (z.link) return `L${z.link}`;
  if (z.rank) return `R${z.rank}`;
  return z.level ? `★${z.level}` : "";
}

/**
 * A card in a zone: Defense Position monsters lie sideways, set cards show
 * their artwork darkened; materials, counters and negation are badges.
 */
function ZoneSlot({ z, getCard, onHover, onHoverEnd, monster }: { z: ZoneCard | null | undefined; getCard: (id: number) => Card | undefined; onHover?: HoverCard; onHoverEnd?: () => void; monster: boolean }) {
  if (!z) return <div className="combo-zone" />;
  return (
    <div className="combo-zone combo-zone--full" onPointerEnter={(e) => onHover?.(z.code, e.currentTarget)} onPointerLeave={onHoverEnd}>
      <div className={`combo-card ${monster && z.defense ? "rotate-90" : ""}`}>
        <CardImage code={z.code} card={getCard(z.code)} className={`h-full w-full ${z.faceUp ? "" : "combo-set"}`} />
        {!z.faceUp && <span className="absolute inset-x-0 bottom-1 text-center text-[8px] font-semibold tracking-wide text-white/90 uppercase drop-shadow">coperta</span>}
        {z.negated && <span className="combo-negated" title="Effetti annullati" />}
      </div>
      {monster && z.faceUp && (
        <span className="combo-stat" title={z.link ? `ATK ${z.attack}` : `ATK ${z.attack} / DEF ${z.def}`}>
          {stars(z)} {z.defense && !z.link ? z.def : z.attack}
        </span>
      )}
      {z.overlay.length > 0 && (
        <span className="combo-badge" title={`${z.overlay.length} materiali Xyz`}>
          ◆{z.overlay.length}
        </span>
      )}
      {z.counters > 0 && (
        <span className="combo-badge combo-badge--left" title={`${z.counters} segnalini`}>
          ●{z.counters}
        </span>
      )}
    </div>
  );
}

export type Pile = "grave" | "banished" | "deck" | "extra";

interface FieldProps {
  board: ComboBoard;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
  /** The pile shown below the field, and how to change it. */
  pile?: Pile | null;
  onPile?: (pile: Pile) => void;
}

/** A face-down card, drawn (no card artwork is bundled with the app). */
function CardBack({ className = "" }: { className?: string }) {
  return <span className={`combo-back ${className}`} aria-hidden />;
}

/**
 * A pile beside the field: the top card (or the back of the Deck), how
 * many cards, and a click to list them below.
 */
function PileSlot({ label, codes, count, back, active, getCard, onHover, onHoverEnd, onClick }: {
  label: string;
  codes: number[];
  count: number;
  back?: boolean;
  active: boolean;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
  onClick?: () => void;
}) {
  const top = codes[codes.length - 1];
  return (
    <button
      type="button"
      className={`combo-zone combo-pile ${active ? "combo-pile--active" : ""}`}
      onClick={onClick}
      disabled={!onClick || count === 0}
      aria-pressed={active}
      title={`${label}: ${count}`}
      onPointerEnter={(e) => top != null && !back && onHover?.(top, e.currentTarget)}
      onPointerLeave={onHoverEnd}
    >
      {count > 0 ? (
        <span className="combo-card">{back || top == null ? <CardBack className="h-full w-full" /> : <CardImage code={top} card={getCard(top)} className="h-full w-full" />}</span>
      ) : (
        <span className="combo-zone-label">{label}</span>
      )}
      {count > 0 && <span className="combo-count">{count}</span>}
    </button>
  );
}

/**
 * The player's side of the field laid out as the Master Duel mat:
 *
 *              [EMZ]      [EMZ]           [Bandite]
 *   [Terreno]  [M] [M] [M] [M] [M]        [Cimitero]
 *   [Extra]    [P] [S] [S] [S] [P]        [Deck]
 *
 * The outer Spell & Trap Zones are also the Pendulum Zones. It fills the
 * width it is given.
 */
export function Field({ board, getCard, onHover, onHoverEnd, pile, onPile }: FieldProps) {
  const zone = (z: ZoneCard | null | undefined, monster: boolean, label?: string) =>
    z ? <ZoneSlot z={z} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} monster={monster} /> : <div className="combo-zone">{label && <span className="combo-zone-label">{label}</span>}</div>;
  const piles = {
    grave: { label: "Cimitero", codes: board.grave, count: board.grave.length },
    banished: { label: "Bandite", codes: board.banished, count: board.banished.length },
    deck: { label: "Deck", codes: [], count: board.deckCount, back: true },
    extra: { label: "Extra Deck", codes: board.extraFaceUp, count: board.extraCount, back: board.extraFaceUp.length === 0 },
  };
  const pileSlot = (kind: Pile) => (
    <PileSlot {...piles[kind]} active={pile === kind} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} onClick={onPile && (() => onPile(kind))} />
  );
  // Columns: the left side, a gap, the five zones, a gap, the right side.
  const at = (column: number, row: number) => ({ gridColumn: column, gridRow: row });
  return (
    <div className="combo-field" role="group" aria-label="Il tuo campo a fine turno">
      <div className="combo-emz" style={at(4, 1)}>
        {zone(board.monsters[5], true, "EMZ")}
      </div>
      <div className="combo-emz" style={at(6, 1)}>
        {zone(board.monsters[6], true, "EMZ")}
      </div>
      <div style={at(9, 1)}>{pileSlot("banished")}</div>

      <div style={at(1, 2)}>{zone(board.spells[5], false, "Terreno")}</div>
      {[0, 1, 2, 3, 4].map((i) => (
        <div key={`m${i}`} style={at(3 + i, 2)}>
          {zone(board.monsters[i], true)}
        </div>
      ))}
      <div style={at(9, 2)}>{pileSlot("grave")}</div>

      <div style={at(1, 3)}>{pileSlot("extra")}</div>
      {[0, 1, 2, 3, 4].map((i) => (
        <div key={`s${i}`} className="relative" style={at(3 + i, 3)}>
          {zone(board.spells[i], false)}
          {(i === 0 || i === 4) && (
            <span className="combo-pendulum" title="Zona Pendulum">
              P
            </span>
          )}
        </div>
      ))}
      <div style={at(9, 3)}>{pileSlot("deck")}</div>
    </div>
  );
}

/** The board in one line: every card on the field, monsters first. */
export function BoardStrip({ board, width, getCard }: { board: ComboBoard; width: number; getCard: (id: number) => Card | undefined }) {
  const monsters = board.monsters.filter((z): z is ZoneCard => !!z);
  const spells = board.spells.slice(0, 6).filter((z): z is ZoneCard => !!z);
  const height = Math.round(width * 1.46);
  const tile = (z: ZoneCard, i: number) => (
    <span key={i} className="relative shrink-0" style={{ width, height }} title={getCard(z.code)?.name}>
      <CardImage code={z.code} card={getCard(z.code)} className={`h-full w-full ${z.faceUp ? "" : "combo-set"}`} />
      {z.overlay.length > 0 && <span className="combo-badge !text-[8px]">◆{z.overlay.length}</span>}
    </span>
  );
  return (
    <span className="flex min-w-0 flex-wrap items-center gap-x-1 gap-y-1">
      {monsters.map(tile)}
      {monsters.length > 0 && spells.length > 0 && <span className="mx-1 h-6 w-px shrink-0 bg-border-strong" />}
      {spells.map(tile)}
      {monsters.length + spells.length === 0 && <span className="text-xs text-faint">campo vuoto</span>}
      {board.handCards.length > 0 && (
        <span className="inline-flex items-center gap-1">
          <span className="mx-1 shrink-0 text-[10px] text-faint">mano</span>
          {board.handCards.map((c, i) => (
            <span key={`h${i}`} className="shrink-0 opacity-80" style={{ width: Math.round(width * 0.8), height: Math.round(height * 0.8) }} title={getCard(c)?.name}>
              <CardImage code={c} card={getCard(c)} className="h-full w-full" />
            </span>
          ))}
        </span>
      )}
    </span>
  );
}
