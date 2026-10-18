import { EmblemIcon } from "../components/Emblems";
import { SectionHeader } from "../components/SectionHeader";
import type { Section } from "../sections";

export function Upcoming({ section, artReady }: { section: Section; artReady: boolean }) {
  return (
    <div className="flex h-full flex-col">
      <SectionHeader section={section} artReady={artReady} />
      <div className="flex-1 overflow-y-auto px-10 py-8">
        <section className="panel max-w-2xl p-8 anim-rise">
          <p className="eyebrow text-sec">In arrivo nella Fase {section.phase}</p>
          <ul className="mt-4 flex flex-col gap-3">
            {section.planned.map((item, i) => (
              <li key={item} className="flex items-center gap-3 anim-rise" style={{ animationDelay: `${80 + i * 60}ms` }}>
                <span className="text-sec">
                  <EmblemIcon emblem={section.emblem} className="h-4 w-4" />
                </span>
                {item}
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
