import { memo, useEffect, useMemo, useState } from "react";
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
  if (!inFormat) return <span className="text-muted text-xs font-semibold">—</span>;
  if (!status) return <CheckCircle2 size={14} className="text-ok shrink-0" />;
  if (status === "Banned" || status === "Forbidden") return <Ban size={14} className="text-danger shrink-0" />;
  return <AlertCircle size={14} className="text-warn shrink-0" />;
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
  const [imageStatus, setImageStatus] = useState<"loading" | "loaded" | "error">("loading");

  // Reset image status on card change
  useEffect(() => {
    setImageStatus("loading");
  }, [card?.id]);

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
        <div className="mb-3 rounded-full border border-border bg-surface-2 p-3.5 text-sec/80">
          <Eye size={24} />
        </div>
        <h4 className="display text-sm font-semibold tracking-wide text-text">Nessuna carta selezionata</h4>
        <p className="mt-1.5 text-xs leading-relaxed text-muted">
          Passa il mouse su una carta o fai clic per esaminarla e fissarla qui.
        </p>
        <div className="mt-4 flex flex-col gap-1.5 rounded-lg border border-border/80 bg-surface-2/60 p-3 text-left text-xs text-muted">
          <span>• <strong className="text-text">Clic singolo:</strong> fissa la carta</span>
          <span>• <strong className="text-text">Doppio clic:</strong> visore a schermo intero</span>
          <span>• <strong className="text-text">Esc:</strong> sblocca la carta fissata</span>
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
          <div className="flex items-center gap-1.5 text-text font-bold">
            <Pin size={14} className="text-sec shrink-0" />
            <span className="text-xs">Carta fissata</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-muted">
            <Eye size={14} className="text-muted/80 shrink-0" />
            <span className="text-xs font-medium">Anteprima passaggio</span>
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
        <div className="relative aspect-[421/614] w-full overflow-hidden bg-surface-3 flex items-center justify-center">
          {imageStatus === "error" ? (
            <div className="flex flex-col items-center justify-center p-4 text-center text-muted">
              <div className="mb-2 rounded-full border border-border bg-surface-2 p-2.5 text-muted">
                <Eye size={22} />
              </div>
              <p className="text-xs font-semibold text-text">{card.name}</p>
              <p className="text-xs text-muted mt-1">Immagine non disponibile</p>
            </div>
          ) : (
            <img
              key={card.id}
              src={cardImage(card.id, "full")}
              alt={card.name}
              loading="eager"
              decoding="async"
              onLoad={() => setImageStatus("loaded")}
              onError={() => setImageStatus("error")}
              className={`h-full w-full object-cover transition-opacity duration-200 ${
                imageStatus === "loaded" ? "opacity-100" : "opacity-0"
              }`}
            />
          )}
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
        <p className="text-xs text-muted font-medium">
          {isMonster ? `[ ${card.typeLine.join(" / ")} ]` : card.humanType}
        </p>
      </div>

      {/* Badges row: Attribute, Level/Rank/Link, MD Rarity, Archetype */}
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        {card.attribute && (
          <span
            className="chip !px-2 !py-0.5 text-xs font-semibold text-text"
            style={{
              borderColor: ATTRIBUTE_COLORS[card.attribute],
            }}
          >
            <span style={{ color: ATTRIBUTE_COLORS[card.attribute] }} aria-hidden="true">●</span>{" "}
            <span>{card.attribute}</span>
          </span>
        )}

        {card.level != null && (
          <span className="chip !px-2 !py-0.5 text-xs font-medium text-text">
            <span className="text-sec" aria-hidden="true">★</span>{" "}
            <span>{isXyz ? "Rango" : "Livello"} {card.level}</span>
          </span>
        )}

        {card.linkRating != null && (
          <span className="chip !px-2 !py-0.5 text-xs font-medium text-text">
            LINK-{card.linkRating}
          </span>
        )}

        {card.scale != null && (
          <span className="chip !px-2 !py-0.5 text-xs font-medium text-text">
            Scala {card.scale}
          </span>
        )}

        {card.archetype && (
          <button
            type="button"
            className="chip chip-active !px-2 !py-0.5 text-xs font-medium hover:brightness-125 transition-all text-text"
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
            <p className="text-xs uppercase tracking-wider text-muted font-semibold">ATK</p>
            <p className="font-bold text-text tabular-nums text-xs">{card.atk ?? "?"}</p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-wider text-muted font-semibold">
              {card.linkRating != null ? "LINK" : "DEF"}
            </p>
            <p className="font-bold text-text tabular-nums text-xs">
              {card.linkRating != null ? card.linkRating : (card.def ?? "?")}
            </p>
          </div>
          <div className="truncate">
            <p className="text-xs uppercase tracking-wider text-muted font-semibold">Tipo</p>
            <p className="font-semibold text-text truncate text-xs" title={card.race}>{card.race}</p>
          </div>
        </div>
      )}

      {/* Deck Copies & Roles Status */}
      <div className="rounded-lg border border-border/70 bg-surface-2/50 p-2.5 flex flex-col gap-2 text-xs">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted font-medium">Copie nel deck</span>
          <span className="font-bold tabular-nums">
            <span className={totalCopies > limit ? "text-danger" : "text-text font-bold"}>{totalCopies}</span>
            <span className="text-muted font-normal"> / {limit}</span>
            <span className="ml-1 text-xs text-muted font-normal">
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
                  className="inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-semibold text-text border border-border bg-surface-3 shadow-xs"
                  title={role.hint}
                >
                  <span
                    className="inline-block h-2 w-2 rounded-full shrink-0"
                    style={{ backgroundColor: role.color }}
                    aria-hidden="true"
                  />
                  <span className="font-bold text-text">{role.letter}</span>
                  <span className="text-muted">{role.label}</span>
                </span>
              ))
            ) : (
              <span className="text-xs text-muted">Nessun ruolo</span>
            )}
          </div>
        </div>
      </div>

      {/* Banlist across all 3 formats in readable rows */}
      <div className="rounded-lg border border-border/70 bg-surface-2/50 p-2.5 text-xs">
        <p className="text-xs font-semibold uppercase tracking-wider text-muted mb-2">
          Stato Banlist nei tre formati
        </p>
        <div className="flex flex-col gap-1.5">
          {/* Master Duel */}
          <div className="flex items-center justify-between rounded border border-border/50 bg-surface-3/40 px-2.5 py-1.5">
            <span className="font-semibold text-text">Master Duel</span>
            <div className="flex items-center gap-1.5">
              {banIcon(card.banMd, card.formats.includes("Master Duel"))}
              <span className="font-medium text-text">
                {!card.formats.includes("Master Duel")
                  ? "Non in MD"
                  : card.banMd
                    ? (BAN_TEXT[card.banMd] ?? card.banMd)
                    : "3 copie"}
              </span>
            </div>
          </div>

          {/* TCG */}
          <div className="flex items-center justify-between rounded border border-border/50 bg-surface-3/40 px-2.5 py-1.5">
            <span className="font-semibold text-text">TCG</span>
            <div className="flex items-center gap-1.5">
              {banIcon(card.banTcg, card.formats.includes("TCG"))}
              <span className="font-medium text-text">
                {!card.formats.includes("TCG")
                  ? "Non in TCG"
                  : card.banTcg
                    ? (BAN_TEXT[card.banTcg] ?? card.banTcg)
                    : "3 copie"}
              </span>
            </div>
          </div>

          {/* OCG */}
          <div className="flex items-center justify-between rounded border border-border/50 bg-surface-3/40 px-2.5 py-1.5">
            <span className="font-semibold text-text">OCG</span>
            <div className="flex items-center gap-1.5">
              {banIcon(card.banOcg, card.formats.includes("OCG"))}
              <span className="font-medium text-text">
                {!card.formats.includes("OCG")
                  ? "Non in OCG"
                  : card.banOcg
                    ? (BAN_TEXT[card.banOcg] ?? card.banOcg)
                    : "3 copie"}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Pendulum description if applicable */}
      {card.pendDesc && (
        <div className="rounded-lg border border-border/80 bg-surface-2/60 p-2.5 text-xs">
          <p className="eyebrow text-xs text-text mb-1 font-bold uppercase tracking-wider">Effetto Pendulum</p>
          <p className="whitespace-pre-line text-xs leading-relaxed text-text">
            {card.pendDesc}
          </p>
        </div>
      )}

      {/* Main card description */}
      <div className="rounded-lg border border-border/80 bg-surface-2/60 p-2.5 text-xs">
        <p className="eyebrow text-xs text-text mb-1 font-bold uppercase tracking-wider">
          {card.pendDesc ? "Effetto Mostro" : "Testo della carta"}
        </p>
        <p className="whitespace-pre-line text-xs leading-relaxed text-text">
          {card.desc}
        </p>
      </div>

      {/* Passcode footer */}
      <div className="flex items-center justify-between text-xs text-muted px-1 pb-1">
        <span>Passcode: <span className="font-mono text-text">{card.id}</span></span>
        {card.tcgDate && <span>TCG: {card.tcgDate}</span>}
      </div>
    </div>
  );
});
