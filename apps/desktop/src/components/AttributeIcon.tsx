import { ATTRIBUTE_COLORS } from "./CardDetails";

/** Original line-art glyphs, 24×24, one per card attribute. */
const paths: Record<string, string[]> = {
  DARK: ["M12 2.500a9.500 9.500 0 100 19 9.500 9.500 0 000-19z", "M15 6.500a6 6 0 100 11 7.500 7.500 0 010-11z"],
  LIGHT: ["M12 2.500c.8 5.500 3.500 8.200 9.500 9.500-6 1.300-8.700 4-9.500 9.500-.8-5.500-3.500-8.200-9.500-9.500 6-1.300 8.700-4 9.500-9.500z", "M12 9.500c.3 1.500 1 2.200 2.500 2.500-1.500.3-2.200 1-2.500 2.500-.3-1.500-1-2.200-2.500-2.500 1.500-.3 2.200-1 2.500-2.500z"],
  EARTH: ["M2.500 20L9.500 6.500c.8-1.300 1.800-1.300 2.500 0L21.500 20z", "M7 12l2.500 1.500 2.500-2.500 3 3 2-1.500"],
  WATER: ["M12 2.8c3.5 4.2 6 7.4 6 10.7a6 6 0 01-12 0c0-3.300 2.500-6.500 6-10.700z", "M9.200 14a3 3 0 002.300 2.800"],
  FIRE: ["M12 2.5c.5 3-1.500 4.500-3 6.500-1.300 1.800-2 3.200-2 5a6.500 6.500 0 0013 0c0-2.500-1.200-4-2.500-5.500.100 1.500-.5 2.500-1.500 3 .6-3.500-1-6.500-4-9z", "M12 20a2.500 2.500 0 01-2.500-2.500c0-1.500 1.500-2.500 2.500-4 1 1.500 2.500 2.500 2.500 4A2.500 2.500 0 0112 20z"],
  WIND: ["M3 9.500c5 0 9.500.500 11.500-1.800 1.500-1.800.2-4-2-3.800", "M3 14c7 0 12 .500 14.500-2 1.600-1.800.2-4.200-2-4", "M3 18.500c4 0 6.500-.200 8.500 1.500 1.200 1 .5 2.500-1 2.500"],
  DIVINE: ["M12 2.500L22 20H2z", "M7.500 15.500c1.200-1.800 2.800-2.700 4.500-2.700s3.300.9 4.500 2.700c-1.200 1.800-2.800 2.700-4.500 2.700s-3.300-.9-4.500-2.700z", "M12 15.500h.01"],
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
