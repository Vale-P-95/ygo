import { BookOpen, Calculator, Gauge, Lightbulb, ListOrdered, ShieldOff, TriangleAlert, type LucideIcon } from "lucide-react";

/** What an info panel is about. It picks the mascot's icon, colour and mood. */
export type InfoKind = "what" | "how" | "rank" | "stat" | "block" | "tip" | "warn";

interface KindStyle {
  icon: LucideIcon;
  /** Token of the accent, from index.css. */
  color: string;
  /** Short label shown beside the title. */
  label: string;
  mood: "calm" | "think" | "happy" | "alert";
}

export const INFO_KINDS: Record<InfoKind, KindStyle> = {
  what: { icon: BookOpen, color: "var(--sec)", label: "Che cos'è", mood: "calm" },
  how: { icon: Calculator, color: "var(--df-gold)", label: "Come si calcola", mood: "think" },
  rank: { icon: ListOrdered, color: "var(--df-gold)", label: "Classifica", mood: "think" },
  stat: { icon: Gauge, color: "var(--df-ok)", label: "Misura", mood: "calm" },
  block: { icon: ShieldOff, color: "var(--df-danger)", label: "Blocchi", mood: "alert" },
  tip: { icon: Lightbulb, color: "var(--df-ok)", label: "Consiglio", mood: "happy" },
  warn: { icon: TriangleAlert, color: "var(--df-warn)", label: "Attenzione", mood: "alert" },
};

const MOUTH: Record<KindStyle["mood"], string> = {
  calm: "M13 24 Q18 27 23 24",
  think: "M15 25.5 H21",
  happy: "M12.5 23.5 Q18 30 23.5 23.5",
  alert: "M13 26.5 Q18 22.5 23 26.5",
};

/**
 * The little ember spirit that speaks from an info panel. The face follows
 * the kind of information and a badge with the kind's icon sits on its corner.
 */
export function Mascot({ kind, size = 40 }: { kind: InfoKind; size?: number }) {
  const k = INFO_KINDS[kind];
  const Icon = k.icon;
  return (
    <span className="mascot relative inline-flex shrink-0" style={{ width: size, height: size, ["--mascot-accent" as string]: k.color }} aria-hidden="true">
      <svg viewBox="0 0 36 36" width={size} height={size} className="mascot-body">
        <path
          d="M18 2 C20 8 28 11 28 21 C28 29 23.5 33.5 18 33.5 C12.5 33.5 8 29 8 21 C8 16 11 14 12 10 C14 12 15 12 15.5 11 C16.5 8 17 5 18 2 Z"
          fill="color-mix(in oklab, var(--mascot-accent) 28%, var(--df-surface-2))"
          stroke="var(--mascot-accent)"
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
        <g className="mascot-eyes" fill="var(--df-text)">
          <ellipse cx="14.5" cy="19" rx="1.7" ry="2.2" />
          <ellipse cx="21.5" cy="19" rx="1.7" ry="2.2" />
        </g>
        {kind === "warn" || kind === "block" ? (
          <path d="M12 15.5 L16.5 16.8 M24 15.5 L19.5 16.8" stroke="var(--df-text)" strokeWidth="1.3" strokeLinecap="round" fill="none" />
        ) : null}
        <path d={MOUTH[k.mood]} stroke="var(--df-text)" strokeWidth="1.4" strokeLinecap="round" fill="none" />
      </svg>
      <span
        className="mascot-badge absolute -bottom-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full border"
        style={{ background: "var(--df-surface)", borderColor: k.color, color: k.color }}
      >
        <Icon size={10} strokeWidth={2.4} />
      </span>
    </span>
  );
}
