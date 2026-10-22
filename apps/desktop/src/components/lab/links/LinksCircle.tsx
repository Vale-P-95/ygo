import { useEffect, useId, useMemo, useRef, useState } from "react";
import { cardImage, type Card, type CardLink, type LinkCard, type LinksComparison } from "../../../lib/api";
import { prefersReducedMotion } from "../../../lib/settings";
import { sound } from "../../../lib/sound";
import {
  calculateCircleGeometry,
  getCardBorderColor,
  getEdgePath,
  getRadialLabelLayout,
} from "./layout";
import { isLinkTextOnly, KEY_CARD_THRESHOLD, LINK_KINDS, linkKey, type LinksFilterState } from "./types";

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
  const [availableHeight, setAvailableHeight] = useState<number>(560);
  const [dimensions, setDimensions] = useState<{ width: number; height: number }>({ width: 740, height: 560 });
  const markerIdPrefix = useId().replace(/:/g, "_");
  const reduceMotion = prefersReducedMotion();

  // Dynamic circle height calculation based on available viewport space
  useEffect(() => {
    const update = () => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const freeHeight = window.innerHeight - rect.top - 24;
      // Flexible bounds fitting within 1280x800 viewport without extending off-screen
      const targetH = Math.max(360, Math.min(720, Math.floor(freeHeight)));
      setAvailableHeight(targetH);

      const clientW = containerRef.current.clientWidth;
      if (clientW > 0) {
        setDimensions({ width: clientW, height: targetH });
      }
    };
    update();
    window.addEventListener("resize", update);
    const ro = new ResizeObserver(() => update());
    if (containerRef.current) ro.observe(containerRef.current);
    return () => {
      window.removeEventListener("resize", update);
      ro.disconnect();
    };
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
      style={{ height: `${availableHeight}px` }}
      className="relative w-full overflow-hidden select-none"
    >
      <svg
        viewBox={geometry.viewBox}
        className="absolute inset-0 w-full h-full"
        style={{ overflow: "visible" }}
      >
        <defs>
          {(Object.keys(LINK_KINDS) as (keyof typeof LINK_KINDS)[]).map((kind) => (
            <g key={kind}>
              <marker
                id={`${markerIdPrefix}-arrow-${kind}`}
                viewBox="0 0 10 10"
                refX="9"
                refY="5"
                markerUnits="userSpaceOnUse"
                markerWidth="7"
                markerHeight="7"
                orient="auto-start-reverse"
              >
                <path d="M 0 1 L 9 5 L 0 9 z" fill={LINK_KINDS[kind].color} fillOpacity="0.95" />
              </marker>
              <marker
                id={`${markerIdPrefix}-arrow-${kind}-faint`}
                viewBox="0 0 10 10"
                refX="9"
                refY="5"
                markerUnits="userSpaceOnUse"
                markerWidth="7"
                markerHeight="7"
                orient="auto-start-reverse"
              >
                <path d="M 0 1 L 9 5 L 0 9 z" fill={LINK_KINDS[kind].color} fillOpacity="0.25" />
              </marker>
            </g>
          ))}
          {/* Comparison markers with CSS tokens */}
          <marker
            id={`${markerIdPrefix}-arrow-gained`}
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="8"
            markerHeight="8"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--df-ok)" fillOpacity="0.95" />
          </marker>
          <marker
            id={`${markerIdPrefix}-arrow-gained-faint`}
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="8"
            markerHeight="8"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--df-ok)" fillOpacity="0.25" />
          </marker>
          <marker
            id={`${markerIdPrefix}-arrow-lost`}
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--df-danger)" fillOpacity="0.95" />
          </marker>
          <marker
            id={`${markerIdPrefix}-arrow-lost-faint`}
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--df-danger)" fillOpacity="0.25" />
          </marker>
          {/* Path marker */}
          <marker
            id={`${markerIdPrefix}-arrow-path`}
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerUnits="userSpaceOnUse"
            markerWidth="9"
            markerHeight="9"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" fill="var(--sec)" fillOpacity="0.95" />
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

            // Compute edge path ending precisely at target node's border, leaving clearance for selection and hand rings
            const isSelectedTarget = l.to === activeCardCode;
            const isHandTarget = sampleHandCodes?.has(l.to) ?? false;
            const isPathTarget = pathNodeCodes?.has(l.to) ?? false;
            const targetRing = isSelectedTarget || isHandTarget ? 8 : isPathTarget ? 7 : 0;
            const effectiveRTo = pTo.radius + targetRing;
            const edge = getEdgePath(pFrom, pTo, pFrom.radius, effectiveRTo);

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

            // Opacity & styling - preserve semantic stroke dash patterns
            let opacity = 0.25;
            let strokeWidth = l.used > 0 ? 1.5 + l.used * 2 : 1.2;
            let strokeColor = meta.color;
            let strokeDash: string | undefined = l.possible && l.used === 0 ? "4 3" : undefined;
            let markerId = `${markerIdPrefix}-arrow-${l.kind}`;

            if (isPath) {
              strokeColor = "var(--sec)";
              markerId = `${markerIdPrefix}-arrow-path`;
              strokeWidth = 3.5;
              opacity = 1;
            } else if (sampleHandCodes) {
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

            const shouldPulse = !reduceMotion && (isRelated || isPath) && opacity > 0.5;
            const isFaint = opacity < 0.5;
            const effectiveMarkerId = isFaint ? `${markerId}-faint` : markerId;

            return (
              <path
                key={`${l.from}-${l.to}-${l.kind}-${idx}`}
                d={edge.pathD}
                fill="none"
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                strokeOpacity={opacity}
                strokeDasharray={strokeDash}
                markerEnd={opacity > 0.07 ? `url(#${effectiveMarkerId})` : undefined}
                className="transition-all duration-200"
              >
                {shouldPulse && strokeDash && (
                  <animate
                    attributeName="stroke-dashoffset"
                    from="14"
                    to="0"
                    dur="1.2s"
                    repeatCount="indefinite"
                  />
                )}
                {shouldPulse && !strokeDash && (
                  <animate
                    attributeName="stroke-opacity"
                    values={`${opacity * 0.75};${opacity};${opacity * 0.75}`}
                    dur="1.5s"
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

            // Radial label transform & anchor ensuring clean divergence without overlap
            const displayName = name.length > 18 ? name.slice(0, 16) + "…" : name;
            const labelLayout = getRadialLabelLayout(pos, R);

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
              `Porta a ${c.reach} ${c.reach === 1 ? "carta" : "carte"} in 1 passo, ${c.reachAll} in tutti i passi`,
              c.key >= KEY_CARD_THRESHOLD ? `★ Carta chiave del deck (−${c.key.toFixed(2)})` : null,
              inSampleHand ? "Nella mano di prova attuale" : null,
              isAdded ? "Nuova carta aggiunta nel confronto" : null,
              isRemoved ? "Carta rimossa nel confronto" : null,
            ]
              .filter(Boolean)
              .join("\n");

            const visibleSuffix = [
              isAdded ? "(+)" : null,
              isRemoved ? "(− tolta)" : null,
              inSampleHand ? "(mano)" : null,
              c.key >= KEY_CARD_THRESHOLD ? "★" : null,
            ]
              .filter(Boolean)
              .join(" ");

            const accessibleName = `${displayName} (${name})${visibleSuffix ? ` ${visibleSuffix}` : ""}`;

            return (
              <g
                key={c.code}
                tabIndex={matches ? 0 : -1}
                role="button"
                aria-label={accessibleName}
                data-tip={tooltipContent}
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
                {/* Fixed invisible hit-area circle limited to half chord to prevent neighbor and arrow overlap */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={Math.min(
                    Math.max(nodeR + 2, 10),
                    geometry.spacingChord > 0 ? geometry.spacingChord * 0.48 : 18,
                  )}
                  fill="transparent"
                  pointerEvents="all"
                />

                {/* Inner graphical group that safely scales without moving pointer hit-area */}
                <g
                  transform={isHovered ? `translate(${pos.x}, ${pos.y}) scale(1.18) translate(${-pos.x}, ${-pos.y})` : undefined}
                  style={{ pointerEvents: "none", transition: "transform 0.18s cubic-bezier(0.16, 1, 0.3, 1)" }}
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
                            : c.key >= KEY_CARD_THRESHOLD
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
                  {c.key >= KEY_CARD_THRESHOLD && !isRemoved && !inSampleHand && !isFocused && !isPathNode && (
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

                  {/* Card-type colored circular frame border with CSS tokens */}
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
                              : c.key >= KEY_CARD_THRESHOLD
                                ? "var(--df-gold)"
                                : borderColor
                    }
                    strokeWidth={isFocused || inSampleHand || isPathNode ? 3 : c.key >= KEY_CARD_THRESHOLD ? 2.5 : 2}
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
                </g>

                {/* Radial outward label text */}
                <text
                  transform={labelLayout.transform}
                  textAnchor={labelLayout.textAnchor}
                  className={`text-[12px] select-none transition-colors pointer-events-none ${
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
                  {c.key >= KEY_CARD_THRESHOLD && <tspan fill="var(--df-gold)"> ★</tspan>}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
