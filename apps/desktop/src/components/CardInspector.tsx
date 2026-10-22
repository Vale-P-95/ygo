import { memo, useMemo, useState } from "react";
import {
  AlertCircle,
  Ban,
  CheckCircle2,
  Eye,
  Layers,
  Maximize2,
  Pin,
  PinOff,
  Plus,
  Star,
} from "lucide-react";
import { cardImage, type Card, type Deck } from "../lib/api";
import { copyLimit, isExtraDeckCard } from "../lib/deck";
import { roleDefs } from "../lib/roles";
import { ATTRIBUTE_COLORS, frameColor } from "./CardDetails";
import { rarityCode } from "./CardTile";
import { IconButton } from "./ui/IconButton";

export interface CardInspectorProps {
  card?: Card;
  deck: Deck;
  pinned: boolean;
  onUnpin?: () => void;
  isFavorite?: boolean;
  onToggleFavorite?: (id: number) => void;
  onAddMainExtra?: (card: Card) => void;
  onAddSide?: (card: Card) => void;
  onOpenViewer?: (id: number) => void;
  onSearchArchetype?: (archetype: string) => void;
  className?: string;
}

const BAN_TEXT: Record<string, string> = {
  Banned: "Proibita (0)",
  Forbidden: "Proibita (0)",
  Limited: "Limitata (1)",
  "Semi-Limited": "Semi-lim. (2)",
};

function banIcon(status: string | null | undefined, inFormat = true) {
  if (!inFormat) return <span className="text-faint text-[11px]">—</span>;
  if (!status) return <CheckCircle2 size={13} className="text-ok shrink-0" />;
  if (status === "Banned" || status === "Forbidden") return <Ban size={13} className="text-danger shrink-0" />;
  return <AlertCircle size={13} className="text-warn shrink-0" />;
}

export const CardInspector = memo(function CardInspector({
  card,
  deck,
  pinned,
  onUnpin,
  isFavorite = false,
  onToggleFavorite,
  onAddMainExtra,
  onAddSide,
  onOpenViewer,
  onSearchArchetype,
  className = "",
}: CardInspectorProps) {
  const [imageLoaded, setImageLoaded] = useState(false);

  // Deck copy counts
  const { copiesInMain, copiesInExtra, copiesInSide, totalCopies, limit, isExtra } = useMemo(() => {
    if (!card) {
      return { copiesInMain: 0, copiesInExtra: 0, copiesInSide: 0, totalCopies: 0, limit: 3, isExtra: false };
    }
    const inMain = deck.main.filter((id) => id === card.id).length;
    const inExtra = deck.extra.filter((id) => id === card.id).length;
    const inSide = deck.side.filter((id) => id === card.id).length;
    const total = inMain + inExtra + inSide;
    const maxCopies = copyLimit(card, deck.format);
    const extra = isExtraDeckCard(card);
    return {
      copiesInMain: inMain,
      copiesInExtra: inExtra,
      copiesInSide: inSide,
      totalCopies: total,
      limit: maxCopies,
      isExtra: extra,
    };
  }, [card, deck]);

  // Roles in current deck
  const activeRoles = useMemo(() => {
    if (!card || !deck.roles) return [];
    const allRoles = roleDefs(deck.roles);
    return allRoles.filter((def) => (deck.roles?.[def.key] ?? []).includes(card.id));
  }, [card, deck.roles]);

  if (!card) {
    return (
      <div className={`flex h-full flex-col items-center justify-center p-6 text-center text-muted ${className}`}>
        <div className="mb-3 rounded-full border border-border bg-surface-2 p-3.5 text-sec/70">
          <Eye size={24} />
        </div>
        <h4 className="display text-sm font-semibold tracking-wide text-text">Nessuna carta selezionata</h4>
        <p className="mt-1.5 text-xs leading-relaxed text-muted">
          Passa il mouse su una carta o fai clic per esaminarla e fissarla qui.
        </p>
        <div className="mt-4 flex flex-col gap-1 rounded-lg border border-border/60 bg-surface-2/40 p-2.5 text-left text-[11px] text-faint">
          <span>• <strong>Clic singolo:</strong> fissa la carta</span>
          <span>• <strong>Doppio clic:</strong> visore a schermo intero</span>
          <span>• <strong>Esc:</strong> sblocca la carta fissata</span>
        </div>
      </div>
    );
  }

  const isMonster = card.kind === "monster";
  const isXyz = card.typeLine.includes("Xyz");
  const rarity = rarityCode(card);
  const canAddMainExtra = totalCopies < limit && (isExtra ? deck.extra.length < 15 : deck.main.length < 60);
  const canAddSide = totalCopies < limit && deck.side.length < 15;

  return (
    <div className={`flex flex-col gap-3.5 text-text select-text ${className}`}>
      {/* Pin state header bar */}
      <div className="flex items-center justify-between rounded-lg border border-border/80 bg-surface-2/70 px-2.5 py-1.5 text-xs select-none">
        {pinned ? (
          <div className="flex items-center gap-1.5 text-sec font-semibold">
            <Pin size={13} className="text-sec" />
            <span className="text-[12px]">Carta fissata</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-muted">
            <Eye size={13} className="text-muted/80" />
            <span className="text-[12px]">Anteprima passaggio</span>
          </div>
        )}
        {pinned && onUnpin && (
          <IconButton
            icon={<PinOff size={13} />}
            tooltip="Sblocca carta (Esc)"
            ariaLabel="Sblocca carta fissata"
            size="sm"
            variant="ghost"
            onClick={onUnpin}
          />
        )}
      </div>

      {/* Card Artwork */}
      <div className="relative overflow-hidden rounded-lg border border-border-strong bg-surface-3 shadow-md">
        <div
          className="h-1.5 w-full shrink-0"
          style={{ background: frameColor(card.frame) }}
        />
        <div className="relative aspect-[421/614] w-full overflow-hidden bg-surface-3">
          <img
            src={cardImage(card.id, "full")}
            alt={card.name}
            loading="eager"
            decoding="async"
            onLoad={() => setImageLoaded(true)}
            className={`h-full w-full object-cover transition-opacity duration-200 ${
              imageLoaded ? "opacity-100" : "opacity-0"
            }`}
          />
          {rarity && (
            <span
              className="rarity-gem absolute top-2 right-2 shadow-md !text-[9px] !leading-[13px] px-1 py-0.5"
              data-rarity={rarity}
              title={`Rarità Master Duel: ${card.mdRarity}`}
            >
              {rarity.toUpperCase()}
            </span>
          )}
        </div>

        {/* Quick action toolbar */}
        <div className="flex items-center justify-around border-t border-border/80 bg-surface-2/95 p-1.5 select-none">
          <button
            type="button"
            className="btn btn-surface !py-1 !px-2 text-xs flex items-center gap-1 font-semibold"
            disabled={!canAddMainExtra}
            title={
              totalCopies >= limit
                ? `Raggiunto limite di ${limit} copie`
                : isExtra
                  ? "Aggiungi all'Extra Deck (+1)"
                  : "Aggiungi al Main Deck (+1)"
            }
            aria-label={isExtra ? "Aggiungi all'Extra Deck" : "Aggiungi al Main Deck"}
            onClick={() => onAddMainExtra?.(card)}
          >
            <Plus size={13} className="text-sec" />
            <span>{isExtra ? "Extra" : "Main"}</span>
          </button>

          <button
            type="button"
            className="btn btn-surface !py-1 !px-2 text-xs flex items-center gap-1 font-semibold"
            disabled={!canAddSide}
            title={
              totalCopies >= limit
                ? `Raggiunto limite di ${limit} copie`
                : "Aggiungi al Side Deck (+1)"
            }
            aria-label="Aggiungi al Side Deck"
            onClick={() => onAddSide?.(card)}
          >
            <Layers size={13} className="text-muted" />
            <span>Side</span>
          </button>

          {onToggleFavorite && (
            <IconButton
              icon={<Star size={14} className={isFavorite ? "fill-gold text-gold" : "text-muted"} />}
              tooltip={isFavorite ? "Togli dai preferiti" : "Aggiungi ai preferiti"}
              ariaLabel={isFavorite ? "Togli dai preferiti" : "Aggiungi ai preferiti"}
              variant="surface"
              size="sm"
              onClick={() => onToggleFavorite(card.id)}
            />
          )}

          {onOpenViewer && (
            <IconButton
              icon={<Maximize2 size={14} className="text-text" />}
              tooltip="Apri nel visore a schermo intero (doppio clic)"
              ariaLabel="Apri nel visore a schermo intero"
              variant="surface"
              size="sm"
              onClick={() => onOpenViewer(card.id)}
            />
          )}
        </div>
      </div>

      {/* Title & Type */}
      <div className="flex flex-col gap-1">
        <h3 className="display text-base font-bold leading-snug tracking-tight text-text">
          {card.name}
        </h3>
        <p className="text-xs text-muted">
          {isMonster ? `[ ${card.typeLine.join(" / ")} ]` : card.humanType}
        </p>
      </div>

      {/* Badges row: Attribute, Level/Rank/Link, MD Rarity, Archetype */}
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        {card.attribute && (
          <span
            className="chip !px-2 !py-0.5 !text-[11px] font-semibold"
            style={{
              borderColor: ATTRIBUTE_COLORS[card.attribute],
              color: ATTRIBUTE_COLORS[card.attribute],
            }}
          >
            ● {card.attribute}
          </span>
        )}

        {card.level != null && (
          <span className="chip !px-2 !py-0.5 !text-[11px]">
            <span style={{ color: isXyz ? "#c9c9c9" : "#f2a33a" }}>★</span>{" "}
            {isXyz ? "Rango" : "Livello"} {card.level}
          </span>
        )}

        {card.linkRating != null && (
          <span className="chip !px-2 !py-0.5 !text-[11px]">
            LINK-{card.linkRating}
          </span>
        )}

        {card.scale != null && (
          <span className="chip !px-2 !py-0.5 !text-[11px]">
            Scala {card.scale}
          </span>
        )}

        {card.archetype && (
          <button
            type="button"
            className="chip chip-active !px-2 !py-0.5 !text-[11px] hover:brightness-125 transition-all"
            onClick={() => onSearchArchetype?.(`arch:"${card.archetype}"`)}
            title={`Cerca carte dell'archetipo ${card.archetype}`}
          >
            ⚑ {card.archetype}
          </button>
        )}
      </div>

      {/* Monster stats: ATK / DEF / Race */}
      {isMonster && (
        <div className="grid grid-cols-3 gap-1.5 rounded-lg border border-border/70 bg-surface-2/60 p-2 text-center text-xs">
          <div>
            <p className="text-[10px] uppercase tracking-wider text-faint font-semibold">ATK</p>
            <p className="font-bold text-text tabular-nums">{card.atk ?? "?"}</p>
          </div>
          <div>
            <p className="text-[10px] uppercase tracking-wider text-faint font-semibold">
              {card.linkRating != null ? "LINK" : "DEF"}
            </p>
            <p className="font-bold text-text tabular-nums">
              {card.linkRating != null ? card.linkRating : (card.def ?? "?")}
            </p>
          </div>
          <div className="truncate">
            <p className="text-[10px] uppercase tracking-wider text-faint font-semibold">Tipo</p>
            <p className="font-medium text-text truncate" title={card.race}>{card.race}</p>
          </div>
        </div>
      )}

      {/* Deck Copies & Roles Status */}
      <div className="rounded-lg border border-border/70 bg-surface-2/50 p-2.5 flex flex-col gap-2">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted font-medium">Copie nel deck</span>
          <span className="font-bold tabular-nums">
            <span className={totalCopies > limit ? "text-danger" : "text-sec"}>{totalCopies}</span>
            <span className="text-faint"> / {limit}</span>
            <span className="ml-1 text-[11px] text-muted">
              ({isExtra ? `Extra: ${copiesInExtra}` : `Main: ${copiesInMain}`}, Side: {copiesInSide})
            </span>
          </span>
        </div>

        {/* Roles list if assigned */}
        <div className="flex items-center justify-between text-xs border-t border-border/40 pt-1.5">
          <span className="text-muted font-medium">Ruoli</span>
          <div className="flex flex-wrap items-center gap-1 justify-end">
            {activeRoles.length > 0 ? (
              activeRoles.map((role) => (
                <span
                  key={role.key}
                  className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-semibold text-text shadow-xs"
                  style={{
                    backgroundColor: `color-mix(in oklab, ${role.color} 25%, var(--df-surface-2))`,
                    borderColor: role.color,
                    borderWidth: 1,
                  }}
                  title={role.hint}
                >
                  <span className="font-bold text-[10px]" style={{ color: role.color }}>
                    {role.letter}
                  </span>
                  <span>{role.label}</span>
                </span>
              ))
            ) : (
              <span className="text-[11px] text-faint">Nessun ruolo</span>
            )}
          </div>
        </div>
      </div>

      {/* Banlist across all 3 formats */}
      <div className="rounded-lg border border-border/70 bg-surface-2/50 p-2.5 text-xs">
        <p className="text-[11px] font-semibold uppercase tracking-wider text-faint mb-2">
          Stato Banlist nei tre formati
        </p>
        <div className="grid grid-cols-3 gap-2 text-center">
          {/* Master Duel */}
          <div className="flex flex-col items-center gap-1 rounded border border-border/50 bg-surface-3/40 p-1.5">
            <span className="text-[10px] font-semibold text-muted">Master Duel</span>
            {banIcon(card.banMd, card.formats.includes("Master Duel"))}
            <span className="text-[11px] truncate max-w-full font-medium" title={card.banMd ? BAN_TEXT[card.banMd] : "Legale (3)"}>
              {card.banMd
                ? (BAN_TEXT[card.banMd] ?? card.banMd)
                : card.formats.includes("Master Duel")
                  ? "Senza limiti (3)"
                  : "Non in MD"}
            </span>
          </div>

          {/* TCG */}
          <div className="flex flex-col items-center gap-1 rounded border border-border/50 bg-surface-3/40 p-1.5">
            <span className="text-[10px] font-semibold text-muted">TCG</span>
            {banIcon(card.banTcg, card.formats.includes("TCG"))}
            <span className="text-[11px] truncate max-w-full font-medium" title={card.banTcg ? BAN_TEXT[card.banTcg] : "Legale (3)"}>
              {card.banTcg ? (BAN_TEXT[card.banTcg] ?? card.banTcg) : "Senza limiti (3)"}
            </span>
          </div>

          {/* OCG */}
          <div className="flex flex-col items-center gap-1 rounded border border-border/50 bg-surface-3/40 p-1.5">
            <span className="text-[10px] font-semibold text-muted">OCG</span>
            {banIcon(card.banOcg, card.formats.includes("OCG"))}
            <span className="text-[11px] truncate max-w-full font-medium" title={card.banOcg ? BAN_TEXT[card.banOcg] : "Legale (3)"}>
              {card.banOcg ? (BAN_TEXT[card.banOcg] ?? card.banOcg) : "Senza limiti (3)"}
            </span>
          </div>
        </div>
      </div>

      {/* Pendulum description if applicable */}
      {card.pendDesc && (
        <div className="rounded-lg border border-border/80 bg-surface-2/60 p-2.5">
          <p className="eyebrow text-[10px] text-sec mb-1 font-semibold uppercase">Effetto Pendulum</p>
          <p className="whitespace-pre-line text-[12px] leading-relaxed text-text/90">
            {card.pendDesc}
          </p>
        </div>
      )}

      {/* Main card description */}
      <div className="rounded-lg border border-border/80 bg-surface-2/60 p-2.5">
        <p className="eyebrow text-[10px] text-sec mb-1 font-semibold uppercase">
          {card.pendDesc ? "Effetto Mostro" : "Testo della carta"}
        </p>
        <p className="whitespace-pre-line text-[12px] leading-relaxed text-text/90">
          {card.desc}
        </p>
      </div>

      {/* Passcode footer */}
      <div className="flex items-center justify-between text-[11px] text-faint px-1 pb-1">
        <span>Passcode: <span className="font-mono">{card.id}</span></span>
        {card.tcgDate && <span>TCG: {card.tcgDate}</span>}
      </div>
    </div>
  );
});
