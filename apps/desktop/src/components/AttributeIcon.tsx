import { ATTRIBUTE_COLORS } from "./CardDetails";

/** Original line-art glyphs, 24×24, one per card attribute. */
const paths: Record<string, string[]> = {
  DARK: ["M19.5 14.5A8 8 0 019.5 4.5a8 8 0 1010 10z", "M16 4v3M14.5 5.5h3"],
  LIGHT: ["M12 8a4 4 0 100 8 4 4 0 000-8z", "M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M18.7 5.3l-2.1 2.1M7.4 16.6l-2.1 2.1"],
  EARTH: ["M2.5 19.5L9 7l3.5 6.5L15 10l6.5 9.5z", "M9 7l1.5 4.5-2 1.5"],
  WATER: ["M12 2.8c3.5 4.2 6 7.4 6 10.7a6 6 0 01-12 0c0-3.300 2.500-6.500 6-10.700z", "M9.200 14a3 3 0 002.300 2.800"],
  FIRE: ["M12 2.5c.5 3-1.500 4.500-3 6.500-1.300 1.800-2 3.200-2 5a6.500 6.500 0 0013 0c0-2.500-1.200-4-2.500-5.500.100 1.500-.5 2.500-1.500 3 .6-3.500-1-6.500-4-9z", "M12 20a2.500 2.500 0 01-2.500-2.500c0-1.500 1.500-2.500 2.500-4 1 1.500 2.500 2.500 2.500 4A2.500 2.500 0 0112 20z"],
  WIND: ["M3 8.500h10.500a2.500 2.500 0 10-2.500-2.500", "M3 12.500h15a3 3 0 11-3 3", "M3 16.500h6.500a2 2 0 11-2 2"],
  DIVINE: ["M12 2.500l2.500 6.500 7 .5-5.500 4.500 1.800 7L12 17l-5.800 4 1.800-7L2.500 9.500l7-.5z", "M12 8.500v4"],
};

export function AttributeIcon({ attribute, className = "h-4 w-4", colored = true }: { attribute: string; className?: string; colored?: boolean }) {
  const d = paths[attribute];
  if (!d) return <span>{attribute}</span>;
  return (
    <svg
      viewBox="0 0 24 24"
      className={`${className} shrink-0 align-[-0.15em]`}
      fill="none"
      stroke={colored ? ATTRIBUTE_COLORS[attribute] : "currentColor"}
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      role="img"
      aria-label={attribute}
    >
      <title>{attribute}</title>
      {d.map((p) => (
        <path key={p} d={p} />
      ))}
    </svg>
  );
}
