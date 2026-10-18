import { sound } from "../lib/sound";
import { sections, type SectionId } from "../sections";
import { EmblemIcon } from "./Emblems";

export function Sidebar({ active, onSelect }: { active: SectionId; onSelect: (id: SectionId) => void }) {
  return (
    <nav className="relative z-10 flex w-52 shrink-0 flex-col border-r border-border bg-surface/80 px-3 py-5 backdrop-blur" aria-label="Sezioni">
      <div className="mb-7 flex items-center gap-3 px-2">
        <span
          className="grid h-10 w-10 place-items-center rounded-xl text-[#1a0d05]"
          style={{ background: "linear-gradient(135deg, #ffd27a, var(--df-gold) 45%, var(--df-ember))", boxShadow: "0 0 24px -6px var(--df-ember)" }}
        >
          <EmblemIcon emblem="anvil" className="h-6 w-6" strokeWidth={2} />
        </span>
        <div>
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
                onPointerEnter={() => !current && sound.play("hover")}
                onClick={() => {
                  if (current) return;
                  sound.play("page");
                  onSelect(s.id);
                }}
                aria-current={current ? "page" : undefined}
                className={`group relative flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm transition-all duration-200 ${
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
                {s.label}
                {s.phase > 1 && <span className="ml-auto text-[10px] text-faint">F{s.phase}</span>}
              </button>
            </li>
          );
        })}
      </ul>
      <p className="mt-auto px-2 text-[10px] leading-relaxed text-faint">DuelForge 0.1 · progetto non ufficiale, non affiliato a Konami</p>
    </nav>
  );
}
