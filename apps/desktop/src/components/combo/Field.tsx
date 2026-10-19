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
        <span className="combo-stat">
          {stars(z)} {z.attack}
          {!z.link && `/${z.def}`}
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

interface FieldProps {
  board: ComboBoard;
  getCard: (id: number) => Card | undefined;
  onHover?: HoverCard;
  onHoverEnd?: () => void;
}

/**
 * The player's side of the field as in Master Duel: the Extra Monster
 * Zones on top, then the Main Monster Zones, then the Spell & Trap Zones
 * with the Field Zone on their left. It fills the width it is given.
 */
export function Field({ board, getCard, onHover, onHoverEnd }: FieldProps) {
  const zone = (z: ZoneCard | null | undefined, monster: boolean) => <ZoneSlot z={z} getCard={getCard} onHover={onHover} onHoverEnd={onHoverEnd} monster={monster} />;
  return (
    <div className="combo-field">
      <div />
      <div />
      {zone(board.monsters[5], true)}
      <div />
      {zone(board.monsters[6], true)}
      <div />
      <div />
      {[0, 1, 2, 3, 4].map((i) => (
        <div key={`m${i}`}>{zone(board.monsters[i], true)}</div>
      ))}
      <div>{zone(board.spells[5], false)}</div>
      {[0, 1, 2, 3, 4].map((i) => (
        <div key={`s${i}`}>{zone(board.spells[i], false)}</div>
      ))}
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
    <span className="flex min-w-0 items-center gap-1 overflow-hidden">
      {monsters.map(tile)}
      {monsters.length > 0 && spells.length > 0 && <span className="mx-1 h-6 w-px shrink-0 bg-border-strong" />}
      {spells.map(tile)}
      {monsters.length + spells.length === 0 && <span className="text-xs text-faint">campo vuoto</span>}
      {board.handCards.length > 0 && (
        <>
          <span className="mx-1 shrink-0 text-[10px] text-faint">mano</span>
          {board.handCards.map((c, i) => (
            <span key={`h${i}`} className="shrink-0 opacity-80" style={{ width: Math.round(width * 0.8), height: Math.round(height * 0.8) }} title={getCard(c)?.name}>
              <CardImage code={c} card={getCard(c)} className="h-full w-full" />
            </span>
          ))}
        </>
      )}
    </span>
  );
}
