import { useId, useMemo } from "react";
import type { Card, CardLink, LinkCard, LinksComparison } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { isLinkTextOnly, LINK_KINDS, linkKey, type LinksFilterState } from "./types";

interface Props {
  cards: LinkCard[];
  links: CardLink[];
  getCard: (code: number) => Card | undefined;
  filters: LinksFilterState;
  activeCardCode: number | null;
  hoveredCardCode: number | null;
  sampleHandCodes?: Set<number> | null;
  reachedFromHandCodes?: Set<number> | null;
  comparison?: LinksComparison | null;
  showLostLinks?: boolean;
  pathLinks?: CardLink[] | null;
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
  comparison = null,
  showLostLinks = true,
  pathLinks = null,
  onSelectCard,
  onHoverCard,
}: Props) {
  const markerIdPrefix = useId().replace(/:/g, "_");

  const gainedKeys = useMemo(
    () => (comparison ? new Set(comparison.gained.map(linkKey)) : null),
    [comparison],
  );
  const lostKeys = useMemo(
    () => (comparison ? new Set(comparison.lost.map(linkKey)) : null),
    [comparison],
  );
  const addedCodes = useMemo(
    () => (comparison ? new Set(comparison.added) : null),
    [comparison],
  );
  const removedCodes = useMemo(
    () => (comparison ? new Set(comparison.removed) : null),
    [comparison],
  );
  const changedCodes = useMemo(
    () => (comparison ? new Set(comparison.changed.map((c) => c.code)) : null),
    [comparison],
  );

  const pathLinkKeys = useMemo(
    () => (pathLinks ? new Set(pathLinks.map(linkKey)) : null),
    [pathLinks],
  );
  const pathNodeCodes = useMemo(() => {
    if (!pathLinks || pathLinks.length === 0) return null;
    const s = new Set<number>();
    for (const l of pathLinks) {
      s.add(l.from);
      s.add(l.to);
    }
    return s;
  }, [pathLinks]);

  // Cards to display: when comparing and showLostLinks is enabled, show removed cards as ghost nodes
  const displayCards = useMemo(() => {
    if (!comparison || !showLostLinks) return cards;
    const existing = new Set(cards.map((c) => c.code));
    const extraRemoved = comparison.before.cards.filter((c) => !existing.has(c.code));
    return [...cards, ...extraRemoved];
  }, [cards, comparison, showLostLinks]);

  // Order cards: Main deck cards first, then Extra deck cards
  const orderedCards = useMemo(() => {
    const mainCards = displayCards.filter((c) => !c.extra);
    const extraCards = displayCards.filter((c) => c.extra);
    return [...mainCards, ...extraCards];
  }, [displayCards]);

  // Combine links: include lost links if comparison is on and showLostLinks is true
  const allLinks = useMemo(() => {
    if (!comparison || !showLostLinks) return links;
    const currentKeys = new Set(links.map(linkKey));
    const extraLost = comparison.lost.filter((l) => !currentKeys.has(linkKey(l)));
    return [...links, ...extraLost];
  }, [links, comparison, showLostLinks]);

  // Filter links by active filters (kind, zone, onlyUsed)
  const filteredLinks = useMemo(() => {
    return allLinks.filter((l) => {
      if (filters.kind !== "all" && l.kind !== filters.kind) return false;
      if (filters.zone !== "all" && l.zone !== filters.zone) return false;
      if (filters.onlyUsed && l.used <= 0) return false;
      return true;
    });
  }, [allLinks, filters]);

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

  // Adaptive node sizing to guarantee nodes never overlap, even in 60-card decks
  const spacing = (2 * Math.PI * R) / Math.max(1, N);
  const maxAllowedRadius = Math.max(9, Math.min(18, Math.floor(spacing * 0.42)));
  const baseRadius = Math.max(7, Math.min(11, maxAllowedRadius - 3));
  const getNodeRadius = (keyVal: number) => {
    const bonus = Math.min(maxAllowedRadius - baseRadius, Math.round(keyVal * 4.5));
    return baseRadius + Math.max(0, bonus);
  };

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
          {/* Comparison markers with CSS tokens (var(--df-ok) and var(--df-danger)) */}
          <marker
            id={`${markerIdPrefix}-arrow-gained`}
            viewBox="0 0 10 10"
            refX="33"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="8"
            markerHeight="8"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--df-ok)" />
          </marker>
          <marker
            id={`${markerIdPrefix}-arrow-lost`}
            viewBox="0 0 10 10"
            refX="33"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--df-danger)" />
          </marker>
          {/* Path marker */}
          <marker
            id={`${markerIdPrefix}-arrow-path`}
            viewBox="0 0 10 10"
            refX="33"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="9"
            markerHeight="9"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--sec)" />
          </marker>
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

            const lKey = linkKey(l);
            const isPath = pathLinkKeys?.has(lKey) ?? false;
            const isGained = gainedKeys?.has(lKey) ?? false;
            const isLost = lostKeys?.has(lKey) ?? false;
            const isTextOnly = isLinkTextOnly(l);

            const isHandTraversed = sampleHandCodes
              ? (sampleHandCodes.has(l.from) || (reachedFromHandCodes?.has(l.from) ?? false)) &&
                (reachedFromHandCodes?.has(l.to) ?? false) &&
                (l.zone === "deck" || l.zone === "extra")
              : false;

            // Opacity & styling
            let opacity = 0.25;
            let strokeWidth = l.used > 0 ? 1.5 + l.used * 2 : 1.2;
            let strokeColor = meta.color;
            let strokeDash: string | undefined = l.possible && l.used === 0 ? "4 3" : undefined;
            let markerId = `${markerIdPrefix}-arrow-${l.kind}`;

            if (isPath) {
              // Highlighted shortest path from A to B
              strokeColor = "var(--sec)";
              markerId = `${markerIdPrefix}-arrow-path`;
              strokeWidth = 3.5;
              opacity = 1;
            } else if (sampleHandCodes) {
              // Sample hand overlay active (supports both normal and comparison mode)
              if (isGained) {
                strokeColor = "var(--df-ok)";
                markerId = `${markerIdPrefix}-arrow-gained`;
                strokeWidth = 3;
                opacity = isHandTraversed ? 0.95 : 0.05;
              } else if (isLost) {
                strokeColor = "var(--df-danger)";
                markerId = `${markerIdPrefix}-arrow-lost`;
                strokeDash = "4 3";
                strokeWidth = 2.5;
                opacity = isHandTraversed ? 0.85 : 0.05;
              } else if (isTextOnly) {
                strokeDash = "2 3";
                strokeWidth = 1;
                opacity = isHandTraversed ? 0.45 : 0.03;
              } else {
                opacity = isHandTraversed ? 0.85 : 0.04;
                strokeWidth = Math.max(strokeWidth, 2);
              }
            } else if (isGained) {
              strokeColor = "var(--df-ok)";
              markerId = `${markerIdPrefix}-arrow-gained`;
              strokeWidth = Math.max(strokeWidth * 1.4, 2.5);
              opacity = isRelated || targetCode == null ? 0.95 : 0.15;
            } else if (isLost) {
              strokeColor = "var(--df-danger)";
              markerId = `${markerIdPrefix}-arrow-lost`;
              strokeDash = "4 3";
              strokeWidth = 2;
              opacity = isRelated || targetCode == null ? 0.85 : 0.15;
            } else if (isTextOnly) {
              // Fainter stroke for text-only links
              strokeDash = "2 3";
              strokeWidth = 1;
              opacity = isRelated ? 0.45 : 0.08;
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
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                strokeOpacity={opacity}
                strokeDasharray={strokeDash}
                markerEnd={opacity > 0.07 ? `url(#${markerId})` : undefined}
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
            const isPathNode = pathNodeCodes?.has(c.code) ?? false;

            const isAdded = addedCodes?.has(c.code) ?? false;
            const isRemoved = removedCodes?.has(c.code) ?? false;
            const isChanged = changedCodes?.has(c.code) ?? false;

            const nodeR = getNodeRadius(c.key);

            const matches = matchesSearch(c.code);
            let nodeOpacity = 1;
            if (!matches) {
              nodeOpacity = 0.15;
            } else if (sampleHandCodes) {
              nodeOpacity = inSampleHand || isReachedFromHand ? 1 : isRemoved ? 0.15 : 0.18;
            } else if (isRemoved) {
              // Ghost node for removed card
              nodeOpacity = isFocused ? 0.9 : 0.4;
            } else if (targetCode != null && !isConnected && !isPathNode) {
              nodeOpacity = 0.3;
            }

            // Truncated label for circle perimeter
            const displayName = name.length > 18 ? name.slice(0, 16) + "…" : name;

            // Offset label radially outward with safety margin for node radius
            const labelDist = R + Math.max(18, nodeR + 8);
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
                    r={nodeR + 6}
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
                    r={nodeR + 4}
                    fill="none"
                    stroke="var(--sec)"
                    strokeWidth="2"
                    strokeDasharray="3 2"
                    className="anim-fade"
                  />
                )}

                {/* Path node highlight ring */}
                {isPathNode && !inSampleHand && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={nodeR + 5}
                    fill="none"
                    stroke="var(--sec)"
                    strokeWidth="2.5"
                    className="anim-fade"
                  />
                )}

                {/* Node outer pulse/ring if selected or comparison status */}
                {isFocused && !inSampleHand && !isPathNode && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={nodeR + 4}
                    fill="none"
                    stroke={
                      isAdded
                        ? "var(--df-ok)"
                        : isRemoved
                          ? "var(--df-danger)"
                          : c.key >= 0.2
                            ? "var(--df-gold)"
                            : "var(--sec)"
                    }
                    strokeWidth="2.5"
                    className="anim-fade"
                  />
                )}

                {/* Comparison ring (when not focused) */}
                {!isFocused && !inSampleHand && !isPathNode && (isAdded || isRemoved || isChanged) && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={nodeR + 3}
                    fill="none"
                    stroke={isAdded ? "var(--df-ok)" : isRemoved ? "var(--df-danger)" : "var(--role-custom-1)"}
                    strokeWidth="2"
                    strokeDasharray={isRemoved ? "3 2" : undefined}
                  />
                )}

                {/* Key card emphasis ring for bottleneck hubs */}
                {c.key >= 0.2 && !isRemoved && !inSampleHand && !isFocused && !isPathNode && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={nodeR + 3}
                    fill="none"
                    stroke="var(--df-gold)"
                    strokeWidth="1.5"
                    strokeDasharray="2 2"
                  />
                )}

                {/* Node circle */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={nodeR}
                  fill={
                    isRemoved
                      ? "var(--df-surface)"
                      : isAdded
                        ? "color-mix(in srgb, var(--df-ok) 20%, var(--df-surface-2))"
                        : c.extra
                          ? "color-mix(in srgb, var(--role-extender) 25%, var(--df-surface-2))"
                          : "var(--df-surface-2)"
                  }
                  stroke={
                    isAdded
                      ? "var(--df-ok)"
                      : isRemoved
                        ? "var(--df-danger)"
                        : isPathNode
                          ? "var(--sec)"
                          : c.key >= 0.2
                            ? "var(--df-gold)"
                            : c.extra
                              ? "var(--role-extender)"
                              : "var(--df-border-strong)"
                  }
                  strokeWidth={isFocused || inSampleHand || isPathNode ? 2.5 : c.key >= 0.2 ? 2 : 1.2}
                  strokeDasharray={isRemoved ? "3 2" : undefined}
                />

                {/* Copies indicator badge or +/- for added/removed */}
                <text
                  x={pos.x}
                  y={pos.y + Math.round(nodeR * 0.35)}
                  textAnchor="middle"
                  className="font-bold select-none"
                  fontSize={Math.max(8, Math.round(nodeR * 0.75))}
                  fill={
                    isAdded
                      ? "var(--df-ok)"
                      : isRemoved
                        ? "var(--df-danger)"
                        : isFocused || inSampleHand || isPathNode
                          ? "var(--df-text)"
                          : "var(--df-muted)"
                  }
                >
                  {isAdded ? "+" : isRemoved ? "−" : c.copies}
                </text>

                {/* Card label on the perimeter */}
                <text
                  x={lx}
                  y={ly + 3.5}
                  textAnchor={pos.isRight ? "start" : "end"}
                  className={`text-[10px] select-none transition-colors ${
                    isAdded
                      ? "font-semibold fill-[var(--df-ok)]"
                      : isRemoved
                        ? "font-semibold fill-[var(--df-danger)]"
                        : inSampleHand
                          ? "font-bold fill-[var(--df-gold)]"
                          : isPathNode
                            ? "font-bold fill-[var(--sec)]"
                            : isFocused
                              ? "font-bold fill-[var(--sec)]"
                              : isConnected
                                ? "font-medium fill-[var(--df-text)]"
                                : "fill-[var(--df-muted)]"
                  }`}
                >
                  {displayName}
                  {isAdded && <tspan fill="var(--df-ok)"> (+)</tspan>}
                  {isRemoved && <tspan fill="var(--df-danger)"> (− tolta)</tspan>}
                  {inSampleHand && <tspan fill="var(--df-gold)"> (mano)</tspan>}
                  {c.key >= 0.2 && <tspan fill="var(--df-gold)"> ★</tspan>}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
