import { useState, type ReactNode } from "react";
import { cardImage, isDesktop } from "../lib/api";
import type { Section } from "../sections";
import { EmblemIcon } from "./Emblems";

/**
 * Section banner: themed title in engraved capitals, the section emblem, and
 * the artwork of the section's theme card faded into the background.
 */
export function SectionHeader({ section, children, artReady = true }: { section: Section; children?: ReactNode; artReady?: boolean }) {
  const [artLoaded, setArtLoaded] = useState(false);
  const art = isDesktop && artReady && section.themeCard;
  return (
    <header className="relative overflow-hidden border-b border-border">
      {art && (
        <img
          src={cardImage(section.themeCard!.id, "cropped")}
          alt=""
          onLoad={() => setArtLoaded(true)}
          className="pointer-events-none absolute inset-y-0 right-0 h-full w-[62%] object-cover transition-opacity duration-700"
          style={{
            opacity: artLoaded ? 0.42 : 0,
            maskImage: "linear-gradient(90deg, transparent, black 45%, black 75%, transparent)",
            filter: "saturate(1.1) contrast(1.05)",
          }}
        />
      )}
      <div
        className="pointer-events-none absolute inset-0"
        style={{ background: "linear-gradient(90deg, var(--df-bg) 30%, color-mix(in oklab, var(--df-bg) 55%, transparent) 70%, var(--df-bg))" }}
      />
      <div className="pointer-events-none absolute inset-x-0 bottom-0 h-px" style={{ background: "linear-gradient(90deg, transparent, var(--sec), transparent)" }} />
      <div className="relative flex items-end justify-between gap-6 px-8 pt-4 pb-3.5 xl:px-10 xl:pt-7 xl:pb-5">
        <div className="flex items-center gap-3.5 xl:gap-4 anim-rise">
          <span
            className="grid h-10 w-10 xl:h-12 xl:w-12 shrink-0 place-items-center rounded-xl border text-sec"
            style={{
              borderColor: "color-mix(in oklab, var(--sec) 45%, transparent)",
              background: "radial-gradient(circle at 30% 20%, color-mix(in oklab, var(--sec) 28%, transparent), transparent 70%), var(--df-surface)",
              boxShadow: "0 0 24px -6px color-mix(in oklab, var(--sec) 70%, transparent)",
            }}
          >
            <EmblemIcon emblem={section.emblem} className="h-5 w-5 xl:h-6 xl:w-6" />
          </span>
          <div>
            <p className="eyebrow">{section.label}</p>
            <h1 className="display text-2xl xl:text-3xl font-bold text-text">{section.title}</h1>
            <p className="mt-0.5 xl:mt-1 max-w-xl text-xs xl:text-sm text-muted">{section.summary}</p>
          </div>
        </div>
        <div className="flex flex-col items-end gap-2">
          {children}
          {art && <p className="hidden xl:block text-xs text-faint">Ispirato a «{section.themeCard!.name}»</p>}
        </div>
      </div>
    </header>
  );
}
