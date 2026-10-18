import type { Emblem } from "../sections";

/** Line-art emblems, 24×24, drawn with the current color. */
const paths: Record<Emblem, string[]> = {
  gear: [
    "M12 15.5a3.5 3.5 0 100-7 3.5 3.5 0 000 7z",
    "M19.4 13a7.6 7.6 0 000-2l2-1.5-2-3.4-2.3.9a7.5 7.5 0 00-1.7-1L15 3.5h-4l-.4 2.5a7.5 7.5 0 00-1.7 1l-2.3-.9-2 3.4 2 1.5a7.6 7.6 0 000 2l-2 1.5 2 3.4 2.3-.9c.5.4 1.1.8 1.7 1l.4 2.5h4l.4-2.5c.6-.2 1.2-.6 1.7-1l2.3.9 2-3.4z",
  ],
  anvil: ["M3 8h14c2 0 3.6.9 4.5 2.4l-4 .9-2.2 1.9v1.8h1.7v2.5H7v-2.5h1.7v-1.8L6.3 11H4.6A1.6 1.6 0 013 9.4z", "M9 3.5l.9 1.8M12.5 2.8v2M16 3.5l-.9 1.8"],
  shelf: ["M3 4h18M3 12h18M3 20h18M5 4v16M19 4v16", "M8 7v5M11 6v6M14 8l2 4M8 15v5M12 14v6"],
  flask: ["M9 3h6M10 3v6L4.5 18.5A2 2 0 006.2 21.5h11.6a2 2 0 001.7-3L14 9V3", "M7 15h10", "M10.5 18h.01M13.5 17h.01"],
  rune: ["M12 2.5a9.5 9.5 0 110 19 9.5 9.5 0 010-19z", "M12 6l5.2 9H6.8z", "M12 18l-5.2-9h10.4z"],
  bell: ["M6 9a6 6 0 1112 0c0 6 2.5 8 2.5 8h-17S6 15 6 9z", "M10 20.5a2 2 0 004 0"],
  cog: [
    "M12 15a3 3 0 100-6 3 3 0 000 6z",
    "M19 12a7 7 0 00-.1-1.2l2-1.5-2-3.4-2.3 1a7 7 0 00-2-1.2L14.3 3h-4.6l-.3 2.7a7 7 0 00-2 1.2l-2.3-1-2 3.4 2 1.5a7 7 0 000 2.4l-2 1.5 2 3.4 2.3-1a7 7 0 002 1.2l.3 2.7h4.6l.3-2.7a7 7 0 002-1.2l2.3 1 2-3.4-2-1.5c.1-.4.1-.8.1-1.2z",
  ],
};

export function EmblemIcon({ emblem, className = "h-5 w-5", strokeWidth = 1.7 }: { emblem: Emblem; className?: string; strokeWidth?: number }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      {paths[emblem].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}
