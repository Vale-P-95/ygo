import { sound } from "../lib/sound";
import { sections, type SectionId } from "../sections";
import { EmblemIcon } from "./Emblems";

/** `compact`: an icon rail, to leave room to the deck editor. */
export function Sidebar({
  active,
  onSelect,
  onOpenPalette,
  onOpenGuide,
  compact = false,
}: {
  active: SectionId;
  onSelect: (id: SectionId) => void;
  onOpenPalette?: () => void;
  onOpenGuide?: () => void;
  compact?: boolean;
}) {
  return (
    <nav
      className={`relative z-10 flex shrink-0 flex-col border-r border-border bg-surface/80 py-5 backdrop-blur transition-[width] duration-300 ${compact ? "w-[68px] px-2" : "w-52 px-3"}`}
      aria-label="Sezioni"
    >
      <div className={`mb-7 flex items-center gap-3 ${compact ? "justify-center" : "px-2"}`}>
        <span
          className="grid h-10 w-10 place-items-center rounded-xl text-[#1a0d05]"
          style={{ background: "linear-gradient(135deg, #ffd27a, var(--df-gold) 45%, var(--df-ember))", boxShadow: "0 0 24px -6px var(--df-ember)" }}
        >
          <EmblemIcon emblem="anvil" className="h-6 w-6" strokeWidth={2} />
        </span>
        <div className={compact ? "hidden" : ""}>
          <p className="display text-lg leading-none font-bold">DuelForge</p>
          <p className="mt-1 text-[9px] tracking-[0.14em] whitespace-nowrap text-faint uppercase">Officina dei duellanti</p>
        </div>
      </div>
      <ul className="flex flex-col gap-1">
        {sections.map((s) => {
          const current = active === s.id;
          return (
            <li key={s.id} data-section={s.id}>
              <button
                type="button"
                data-nav-section={s.id}
                data-section={s.id}
                onPointerEnter={() => !current && sound.play("hover")}
                onClick={() => {
                  if (current) return;
                  sound.play("page");
                  onSelect(s.id);
                }}
                aria-current={current ? "page" : undefined}
                title={compact ? s.label : undefined}
                className={`group relative flex w-full items-center gap-3 rounded-xl py-2.5 text-left text-sm transition-all duration-200 ${compact ? "justify-center px-0" : "px-3"} ${
                  current ? "bg-surface-3 font-semibold text-text" : "text-muted hover:bg-surface-2 hover:text-text"
                }`}
              >
                <span
                  className="absolute top-2 bottom-2 left-0 w-[3px] rounded-full transition-all duration-300"
                  style={{ background: current ? "var(--sec)" : "transparent", boxShadow: current ? "0 0 12px var(--sec)" : undefined }}
                />
                <span className={current ? "text-sec" : "text-faint transition-colors group-hover:text-sec"}>
                  <EmblemIcon emblem={s.emblem} />
                </span>
                {!compact && s.label}
                {!compact && s.phase > 2 && <span className="ml-auto text-[10px] text-faint">F{s.phase}</span>}
              </button>
            </li>
          );
        })}
      </ul>

      <div className="mt-auto flex flex-col gap-2 pt-3">
        {onOpenPalette && (
          <button
            type="button"
            onPointerEnter={() => sound.play("hover")}
            onClick={() => {
              sound.play("open");
              onOpenPalette();
            }}
            title={compact ? "Palette dei comandi (Ctrl+K)" : undefined}
            aria-label="Palette dei comandi"
            className={`group flex w-full items-center gap-2.5 rounded-xl border border-border bg-surface-2/70 py-2 text-left text-xs font-medium text-muted transition hover:border-border-strong hover:bg-surface-3 hover:text-text ${
              compact ? "justify-center px-0" : "px-3"
            }`}
          >
            <span className="grid h-5 w-5 shrink-0 place-items-center rounded-md border border-border-strong bg-bg-2 font-mono text-[10px] font-bold text-gold group-hover:border-gold/60">
              ⌘
            </span>
            {!compact && (
              <>
                <span className="truncate">Comandi</span>
                <kbd className="ml-auto rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-faint">
                  Ctrl+K
                </kbd>
              </>
            )}
          </button>
        )}
        {onOpenGuide && (
          <button
            type="button"
            onPointerEnter={() => sound.play("hover")}
            onClick={() => {
              sound.play("open");
              onOpenGuide();
            }}
            title={compact ? "Guida rapida (?)" : undefined}
            aria-label="Guida rapida"
            className={`group flex w-full items-center gap-2.5 rounded-xl border border-border bg-surface-2/70 py-2 text-left text-xs font-medium text-muted transition hover:border-border-strong hover:bg-surface-3 hover:text-text ${
              compact ? "justify-center px-0" : "px-3"
            }`}
          >
            <span className="grid h-5 w-5 shrink-0 place-items-center rounded-full border border-border-strong bg-bg-2 text-[11px] font-bold text-gold group-hover:border-gold/60">
              ?
            </span>
            {!compact && (
              <>
                <span className="truncate">Guida rapida</span>
                <kbd className="ml-auto rounded border border-border-strong bg-bg-2 px-1.5 py-0.5 font-mono text-[10px] text-faint">
                  ?
                </kbd>
              </>
            )}
          </button>
        )}
        {!compact && <p className="px-2 text-[10px] leading-relaxed text-faint">DuelForge 0.1 · progetto non ufficiale, non affiliato a Konami</p>}
      </div>
    </nav>
  );
}
