import { useState } from "react";
import { EngineStatusPanel } from "./EngineStatus";
import { sections, type Section, type SectionId } from "./sections";

const roadmap = [
  { phase: 0, title: "Fondamenta e prova del motore", done: true },
  { phase: 1, title: "Database carte e visualizzatore", done: false },
  { phase: 2, title: "Deck Builder e banlist", done: false },
  { phase: 3, title: "Scaffale dei deck", done: false },
  { phase: 4, title: "Laboratorio statistico", done: false },
  { phase: 5, title: "Motore e sandbox", done: false },
  { phase: 6, title: "Combo Lab v1", done: false },
  { phase: 7, title: "Combo Lab v2", done: false },
];

function Icon({ path }: { path: string }) {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5 shrink-0" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d={path} />
    </svg>
  );
}

function Sidebar({ active, onSelect }: { active: SectionId; onSelect: (id: SectionId) => void }) {
  return (
    <nav className="flex w-60 shrink-0 flex-col border-r border-border bg-surface px-3 py-4" aria-label="Sezioni">
      <div className="mb-6 flex items-center gap-2 px-3">
        <span className="grid h-8 w-8 place-items-center rounded-lg bg-accent font-bold text-bg">D</span>
        <span className="text-lg font-semibold tracking-tight">DuelForge</span>
      </div>
      <ul className="flex flex-col gap-1">
        {sections.map((s) => (
          <li key={s.id}>
            <button
              type="button"
              onClick={() => onSelect(s.id)}
              aria-current={active === s.id ? "page" : undefined}
              className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors duration-150 ${
                active === s.id ? "bg-accent-soft font-medium text-text" : "text-muted hover:bg-surface-2 hover:text-text"
              }`}
            >
              <Icon path={s.icon} />
              {s.label}
            </button>
          </li>
        ))}
      </ul>
      <p className="mt-auto px-3 text-xs text-muted">Versione 0.1.0 · progetto non ufficiale, non affiliato a Konami</p>
    </nav>
  );
}

function Overview() {
  return (
    <div className="grid gap-6 lg:grid-cols-2">
      <EngineStatusPanel />
      <section className="rounded-xl border border-border bg-surface p-5">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-muted">Roadmap</h2>
        <ol className="mt-3 flex flex-col gap-2 text-sm">
          {roadmap.map((step) => (
            <li key={step.phase} className="flex items-center gap-3">
              <span
                className={`grid h-6 w-6 place-items-center rounded-full text-xs font-semibold ${
                  step.done ? "bg-ok text-bg" : "border border-border text-muted"
                }`}
              >
                {step.phase}
              </span>
              <span className={step.done ? "" : "text-muted"}>{step.title}</span>
            </li>
          ))}
        </ol>
      </section>
    </div>
  );
}

function Upcoming({ section }: { section: Section }) {
  return (
    <section className="max-w-2xl rounded-xl border border-border bg-surface p-6">
      <p className="text-sm font-medium text-accent">In arrivo nella Fase {section.phase}</p>
      <ul className="mt-4 flex flex-col gap-2 text-sm">
        {section.planned.map((item) => (
          <li key={item} className="flex gap-2">
            <span className="text-accent" aria-hidden>
              ◆
            </span>
            {item}
          </li>
        ))}
      </ul>
    </section>
  );
}

export default function App() {
  const [active, setActive] = useState<SectionId>("overview");
  const section = sections.find((s) => s.id === active) ?? sections[0];
  return (
    <div className="flex h-full">
      <Sidebar active={active} onSelect={setActive} />
      <main className="flex-1 overflow-y-auto px-10 py-8">
        <header className="mb-8">
          <h1 className="text-2xl font-semibold tracking-tight">{section.label}</h1>
          <p className="mt-1 text-muted">{section.summary}</p>
        </header>
        {section.id === "overview" ? <Overview /> : <Upcoming section={section} />}
      </main>
    </div>
  );
}
