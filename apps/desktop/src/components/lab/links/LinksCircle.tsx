import { useId, useMemo } from "react";
import type { Card, CardLink, LinkCard } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { LINK_KINDS, type LinksFilterState } from "./types";

interface Props {
  cards: LinkCard[];
  links: CardLink[];
  getCard: (code: number) => Card | undefined;
  filters: LinksFilterState;
  activeCardCode: number | null;
  hoveredCardCode: number | null;
  sampleHandCodes?: Set<number> | null;
  reachedFromHandCodes?: Set<number> | null;
  onSelectCard: (code: number | null) => void;
  onHoverCard: (code: number | null) => void;
}

export function LinksCircle({
  cards,
  links,
  getCard,
  filters,
  activeCardCode,
  hoveredCardCode,
  sampleHandCodes = null,
  reachedFromHandCodes = null,
  onSelectCard,
  onHoverCard,
}: Props) {
  const markerIdPrefix = useId().replace(/:/g, "_");

  // Filter links by active filters (kind, zone, onlyUsed)
  const filteredLinks = useMemo(() => {
    return links.filter((l) => {
      if (filters.kind !== "all" && l.kind !== filters.kind) return false;
      if (filters.zone !== "all" && l.zone !== filters.zone) return false;
      if (filters.onlyUsed && l.used <= 0) return false;
      return true;
    });
  }, [links, filters]);

  // Order cards: Main deck cards first, then Extra deck cards
  const orderedCards = useMemo(() => {
    const mainCards = cards.filter((c) => !c.extra);
    const extraCards = cards.filter((c) => c.extra);
    return [...mainCards, ...extraCards];
  }, [cards]);

  const targetCode = hoveredCardCode ?? activeCardCode;

  const query = filters.searchQuery.trim().toLowerCase();
  const matchesSearch = (code: number) => {
    if (!query) return true;
    const c = getCard(code);
    return (c?.name?.toLowerCase().includes(query) ?? false) || String(code).includes(query);
  };

  // Geometry
  const R = 230;
  const N = orderedCards.length;

  const positions = useMemo(() => {
    const map = new Map<number, { x: number; y: number; angle: number; isRight: boolean }>();
    orderedCards.forEach((c, i) => {
      const angle = (2 * Math.PI * i) / Math.max(1, N) - Math.PI / 2;
      const x = Math.round(R * Math.cos(angle) * 10) / 10;
      const y = Math.round(R * Math.sin(angle) * 10) / 10;
      const isRight = Math.cos(angle) >= 0;
      map.set(c.code, { x, y, angle, isRight });
    });
    return map;
  }, [orderedCards, N, R]);

  // Sets of connected nodes to current active card
  const connectedCodes = useMemo(() => {
    if (targetCode == null) return null;
    const set = new Set<number>([targetCode]);
    for (const l of filteredLinks) {
      if (l.from === targetCode) set.add(l.to);
      if (l.to === targetCode) set.add(l.from);
    }
    return set;
  }, [targetCode, filteredLinks]);

  return (
    <div className="relative flex items-center justify-center w-full h-full min-h-[500px] overflow-hidden select-none">
      <svg
        viewBox="-370 -370 740 740"
        className="w-full h-full max-w-[680px] max-h-[680px]"
        style={{ overflow: "visible" }}
      >
        <defs>
          {(Object.keys(LINK_KINDS) as (keyof typeof LINK_KINDS)[]).map((kind) => (
            <marker
              key={kind}
              id={`${markerIdPrefix}-arrow-${kind}`}
              viewBox="0 0 10 10"
              refX="33"
              refY="5"
              markerUnits="userSpaceOnUse"
              markerWidth="7"
              markerHeight="7"
              orient="auto-start-reverse"
            >
              <path d="M 0 1 L 9 5 L 0 9 z" fill={LINK_KINDS[kind].color} />
            </marker>
          ))}
        </defs>

        {/* Center decorative guidelines */}
        <circle cx="0" cy="0" r={R} fill="none" stroke="currentColor" strokeWidth="1" className="text-border/40" strokeDasharray="3 4" />
        <circle cx="0" cy="0" r={R * 0.4} fill="none" stroke="currentColor" strokeWidth="0.8" className="text-border/20" />

        {/* Drawn link arrows */}
        <g className="links-layer">
          {filteredLinks.map((l, idx) => {
            const pFrom = positions.get(l.from);
            const pTo = positions.get(l.to);
            if (!pFrom || !pTo) return null;

            // Compute quadratic bezier curve pulling towards center (0, 0)
            const kPull = 0.35;
            const cx = ((pFrom.x + pTo.x) / 2) * kPull;
            const cy = ((pFrom.y + pTo.y) / 2) * kPull;

            const isRelated = targetCode != null && (l.from === targetCode || l.to === targetCode);
            const meta = LINK_KINDS[l.kind];

            // Opacity & styling
            let opacity = 0.25;
            let strokeWidth = l.used > 0 ? 1.5 + l.used * 2 : 1.2;

            if (sampleHandCodes) {
              const fromInHand = sampleHandCodes.has(l.from);
              const toReached = reachedFromHandCodes?.has(l.to);
              if (fromInHand || toReached) {
                opacity = 0.85;
                strokeWidth = Math.max(strokeWidth, 2);
              } else {
                opacity = 0.04;
              }
            } else if (targetCode != null) {
              if (isRelated) {
                opacity = 0.95;
                strokeWidth = Math.max(strokeWidth * 1.5, 2.5);
              } else {
                opacity = 0.04;
              }
            } else if (l.zone !== "deck" && l.zone !== "extra") {
              opacity = 0.15;
            }

            if (query && !matchesSearch(l.from) && !matchesSearch(l.to)) {
              opacity = 0.03;
            }

            const pathD = `M ${pFrom.x} ${pFrom.y} Q ${cx} ${cy} ${pTo.x} ${pTo.y}`;

            return (
              <path
                key={`${l.from}-${l.to}-${l.kind}-${idx}`}
                d={pathD}
                fill="none"
                stroke={meta.color}
                strokeWidth={strokeWidth}
                strokeOpacity={opacity}
                strokeDasharray={l.possible && l.used === 0 ? "4 3" : undefined}
                markerEnd={opacity > 0.1 ? `url(#${markerIdPrefix}-arrow-${l.kind})` : undefined}
                className="transition-all duration-200"
              />
            );
          })}
        </g>

        {/* Nodes (cards on the circle perimeter) */}
        <g className="nodes-layer">
          {orderedCards.map((c) => {
            const pos = positions.get(c.code);
            if (!pos) return null;

            const card = getCard(c.code);
            const name = card?.name ?? String(c.code);
            const isSelected = activeCardCode === c.code;
            const isHovered = hoveredCardCode === c.code;
            const isFocused = isSelected || isHovered;
            const isConnected = connectedCodes == null || connectedCodes.has(c.code);

            const inSampleHand = sampleHandCodes?.has(c.code) ?? false;
            const isReachedFromHand = reachedFromHandCodes?.has(c.code) ?? false;

            const matches = matchesSearch(c.code);
            let nodeOpacity = 1;
            if (!matches) {
              nodeOpacity = 0.15;
            } else if (sampleHandCodes) {
              nodeOpacity = inSampleHand || isReachedFromHand ? 1 : 0.18;
            } else if (targetCode != null && !isConnected) {
              nodeOpacity = 0.3;
            }

            // Truncated label for circle perimeter
            const displayName = name.length > 18 ? name.slice(0, 16) + "…" : name;

            // Offset label radially outward
            const labelDist = R + 20;
            const lx = labelDist * Math.cos(pos.angle);
            const ly = labelDist * Math.sin(pos.angle);

            return (
              <g
                key={c.code}
                tabIndex={matches ? 0 : -1}
                role="button"
                aria-label={`Carta ${displayName}`}
                className={`transition-opacity duration-200 outline-none ${matches ? "cursor-pointer" : "pointer-events-none"}`}
                opacity={nodeOpacity}
                onClick={() => {
                  if (!matches) return;
                  sound.play("page");
                  onSelectCard(isSelected ? null : c.code);
                }}
                onKeyDown={(e) => {
                  if (!matches) return;
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    sound.play("page");
                    onSelectCard(isSelected ? null : c.code);
                  }
                }}
                onPointerEnter={() => onHoverCard(c.code)}
                onPointerLeave={() => onHoverCard(null)}
              >
                {/* Hand overlay highlight ring */}
                {inSampleHand && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={c.key > 0.2 ? 20 : 18}
                    fill="none"
                    stroke="var(--df-gold)"
                    strokeWidth="3"
                    className="anim-fade"
                  />
                )}
                {isReachedFromHand && !inSampleHand && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={c.key > 0.2 ? 18 : 16}
                    fill="none"
                    stroke="var(--sec)"
                    strokeWidth="2"
                    strokeDasharray="3 2"
                    className="anim-fade"
                  />
                )}

                {/* Node outer pulse/ring if selected */}
                {isFocused && !inSampleHand && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={c.key > 0.2 ? 18 : 16}
                    fill="none"
                    stroke={c.key > 0.2 ? "var(--df-gold)" : "var(--sec)"}
                    strokeWidth="2.5"
                    className="anim-fade"
                  />
                )}

                {/* Node circle */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={c.key > 0.2 ? 14 : 12}
                  fill={c.extra ? "color-mix(in srgb, var(--role-extender) 25%, var(--df-surface-2))" : "var(--df-surface-2)"}
                  stroke={c.key > 0.2 ? "var(--df-gold)" : c.extra ? "var(--role-extender)" : "var(--df-border-strong)"}
                  strokeWidth={isFocused || inSampleHand ? 2.5 : 1.2}
                />

                {/* Copies indicator badge */}
                <text
                  x={pos.x}
                  y={pos.y + 4}
                  textAnchor="middle"
                  className="text-[9px] font-bold fill-current"
                  fill={isFocused || inSampleHand ? "var(--df-text)" : "var(--df-muted)"}
                >
                  {c.copies}
                </text>

                {/* Card label on the perimeter */}
                <text
                  x={lx}
                  y={ly + 3.5}
                  textAnchor={pos.isRight ? "start" : "end"}
                  className={`text-[10px] select-none transition-colors ${
                    inSampleHand
                      ? "font-bold fill-[var(--df-gold)]"
                      : isFocused
                        ? "font-bold fill-[var(--sec)]"
                        : isConnected
                          ? "font-medium fill-[var(--df-text)]"
                          : "fill-[var(--df-muted)]"
                  }`}
                >
                  {displayName}
                  {inSampleHand && <tspan fill="var(--df-gold)"> (mano)</tspan>}
                  {c.key > 0.2 && <tspan fill="var(--df-gold)"> ★</tspan>}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
