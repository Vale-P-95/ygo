import { useEffect, useId, useMemo, useRef, useState } from "react";
import { cardImage, type Card, type CardLink, type LinkCard, type LinksComparison } from "../../../lib/api";
import { prefersReducedMotion } from "../../../lib/settings";
import { sound } from "../../../lib/sound";
import { calculateCircleGeometry, getCardBorderColor, getLabelCoordinates } from "./layout";
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
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState<{ width: number; height: number }>({ width: 740, height: 620 });
  const markerIdPrefix = useId().replace(/:/g, "_");
  const reduceMotion = prefersReducedMotion();

  // ResizeObserver for dynamic circle scaling across any screen or split layout
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const update = () => {
      const { clientWidth, clientHeight } = el;
      if (clientWidth > 0 && clientHeight > 0) {
        setDimensions((prev) => {
          if (Math.abs(prev.width - clientWidth) < 2 && Math.abs(prev.height - clientHeight) < 2) return prev;
          return { width: clientWidth, height: clientHeight };
        });
      }
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

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

  // Dynamic geometric layout calculation
  const geometry = useMemo(() => {
    return calculateCircleGeometry({
      containerWidth: dimensions.width,
      containerHeight: dimensions.height,
      cardCount: orderedCards.length,
      cards: orderedCards,
    });
  }, [dimensions.width, dimensions.height, orderedCards]);

  const positions = geometry.positions;
  const R = geometry.radius;

  // Sets of connected nodes to current active/hovered card
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
    <div
      ref={containerRef}
      className="relative flex items-center justify-center w-full h-full min-h-[520px] overflow-hidden select-none"
    >
      <svg
        viewBox={geometry.viewBox}
        className="w-full h-full"
        style={{ overflow: "visible" }}
      >
        <defs>
          {(Object.keys(LINK_KINDS) as (keyof typeof LINK_KINDS)[]).map((kind) => (
            <marker
              key={kind}
              id={`${markerIdPrefix}-arrow-${kind}`}
              viewBox="0 0 10 10"
              refX="14"
              refY="5"
              markerUnits="userSpaceOnUse"
              markerWidth="7"
              markerHeight="7"
              orient="auto-start-reverse"
            >
              <path d="M 0 1 L 9 5 L 0 9 z" fill={LINK_KINDS[kind].color} />
            </marker>
          ))}
          {/* Comparison markers with CSS tokens */}
          <marker
            id={`${markerIdPrefix}-arrow-gained`}
            viewBox="0 0 10 10"
            refX="14"
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
            refX="14"
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
            refX="14"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="9"
            markerHeight="9"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--sec)" />
          </marker>

          {/* Card node artwork clip paths */}
          {orderedCards.map((c) => {
            const pos = positions.get(c.code);
            if (!pos) return null;
            return (
              <clipPath key={c.code} id={`${markerIdPrefix}-clip-${c.code}`}>
                <circle cx={pos.x} cy={pos.y} r={pos.radius} />
              </clipPath>
            );
          })}
        </defs>

        {/* Center decorative guidelines */}
        <circle
          cx="0"
          cy="0"
          r={R}
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          className="text-border/30"
          strokeDasharray="3 4"
        />
        <circle
          cx="0"
          cy="0"
          r={R * 0.45}
          fill="none"
          stroke="currentColor"
          strokeWidth="0.8"
          className="text-border/20"
        />

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
              // Sample hand overlay active
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

            // Directional animated pulse on active/focused links
            const shouldAnimate = !reduceMotion && (isRelated || isPath) && opacity > 0.5;
            if (shouldAnimate) {
              strokeDash = "7 4";
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
              >
                {shouldAnimate && (
                  <animate
                    attributeName="stroke-dashoffset"
                    from="22"
                    to="0"
                    dur="1.2s"
                    repeatCount="indefinite"
                  />
                )}
              </path>
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

            const nodeR = pos.radius;
            const borderColor = getCardBorderColor(card);

            const matches = matchesSearch(c.code);
            let nodeOpacity = 1;
            if (!matches) {
              nodeOpacity = 0.15;
            } else if (sampleHandCodes) {
              nodeOpacity = inSampleHand || isReachedFromHand ? 1 : isRemoved ? 0.15 : 0.18;
            } else if (isRemoved) {
              nodeOpacity = isFocused ? 0.9 : 0.4;
            } else if (targetCode != null && !isConnected && !isPathNode) {
              nodeOpacity = 0.3;
            }

            // Truncated label for circle perimeter (12-13px)
            const displayName = name.length > 20 ? name.slice(0, 18) + "…" : name;
            const labelCoord = getLabelCoordinates(pos, 8);

            // Copies badge geometry on the node border
            const badgeR = Math.max(7, Math.min(10, Math.round(nodeR * 0.38)));
            const badgeOffset = nodeR * 0.72;
            const badgeX = pos.x + (pos.isRight ? badgeOffset : -badgeOffset);
            const badgeY = pos.y + badgeOffset;

            // Custom chart tooltip payload (no native title attribute)
            const tooltipContent = [
              `${name}${c.extra ? " (Extra Deck)" : ""}`,
              `Tipo: ${card?.kind === "monster" ? card.humanType || "Mostro" : card?.kind === "spell" ? "Magia" : card?.kind === "trap" ? "Trappola" : "Carta"}`,
              `Copie nel mazzo: ×${c.copies}`,
              `Raggiungibilità: ${c.reach} su ${c.reachAll}`,
              c.key >= 0.2 ? `★ Carta chiave del deck (−${c.key.toFixed(2)})` : null,
              inSampleHand ? "Nella mano di prova attuale" : null,
              isAdded ? "Nuova carta aggiunta nel confronto" : null,
              isRemoved ? "Carta rimossa nel confronto" : null,
            ]
              .filter(Boolean)
              .join("\n");

            return (
              <g
                key={c.code}
                tabIndex={matches ? 0 : -1}
                role="button"
                aria-label={`Carta ${name}`}
                data-tip={tooltipContent}
                className={`transition-opacity duration-200 outline-none ${matches ? "cursor-pointer" : "pointer-events-none"}`}
                opacity={nodeOpacity}
                transform={isHovered ? `translate(${pos.x}, ${pos.y}) scale(1.18) translate(${-pos.x}, ${-pos.y})` : undefined}
                style={{
                  transition: "transform 0.18s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.2s ease",
                  transformOrigin: `${pos.x}px ${pos.y}px`,
                }}
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

                {/* Focus / Selection outer ring */}
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

                {/* Key card hub ring */}
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

                {/* Node base fill */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={nodeR}
                  fill="var(--df-surface-2)"
                />

                {/* Cropped card artwork inside circular frame */}
                <image
                  href={cardImage(c.code, "cropped")}
                  x={pos.x - nodeR}
                  y={pos.y - nodeR}
                  width={nodeR * 2}
                  height={nodeR * 2}
                  preserveAspectRatio="xMidYMid slice"
                  clipPath={`url(#${markerIdPrefix}-clip-${c.code})`}
                  opacity={isRemoved ? 0.45 : 1}
                />

                {/* Card-type colored circular frame border */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={nodeR}
                  fill="none"
                  stroke={
                    isAdded
                      ? "var(--df-ok)"
                      : isRemoved
                        ? "var(--df-danger)"
                        : isPathNode
                          ? "var(--sec)"
                          : isFocused
                            ? "var(--sec)"
                            : c.key >= 0.2
                              ? "var(--df-gold)"
                              : borderColor
                  }
                  strokeWidth={isFocused || inSampleHand || isPathNode ? 3 : c.key >= 0.2 ? 2.5 : 2}
                  strokeDasharray={isRemoved ? "3 2" : undefined}
                />

                {/* Copies indicator badge */}
                <g className="copies-badge">
                  <circle
                    cx={badgeX}
                    cy={badgeY}
                    r={badgeR}
                    fill="var(--df-surface)"
                    stroke={isAdded ? "var(--df-ok)" : isRemoved ? "var(--df-danger)" : "var(--df-border-strong)"}
                    strokeWidth="1"
                  />
                  <text
                    x={badgeX}
                    y={badgeY + Math.round(badgeR * 0.35)}
                    textAnchor="middle"
                    className="font-bold select-none tabular-nums"
                    fontSize={Math.max(8, Math.round(badgeR * 1.1))}
                    fill={
                      isAdded
                        ? "var(--df-ok)"
                        : isRemoved
                          ? "var(--df-danger)"
                          : "var(--df-text)"
                    }
                  >
                    {isAdded ? "+" : isRemoved ? "−" : c.copies}
                  </text>
                </g>

                {/* Adjacent card label (12-13px) */}
                <text
                  x={labelCoord.lx}
                  y={labelCoord.ly + 4}
                  textAnchor={labelCoord.textAnchor}
                  className={`text-[12px] select-none transition-colors ${
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
