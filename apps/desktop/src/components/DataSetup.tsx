import { useRef, useState } from "react";
import { api, isDesktop, type DataStatus, type Progress, type ProgressItem } from "../lib/api";
import { sound } from "../lib/sound";
import { burstFrom } from "../lib/sparks";
import { EmblemIcon } from "./Emblems";

const ITEMS: { item: ProgressItem; label: string; size: string }[] = [
  { item: "cards", label: "Elenco delle carte (YGOPRODeck)", size: "~25 MB" },
  { item: "scripts", label: "Script delle carte (Project Ignis)", size: "~40 MB" },
  { item: "cdb", label: "Database del motore (Project Ignis)", size: "~15 MB" },
];

function describe(p: Progress | undefined): string {
  if (!p) return "In attesa";
  const mb = (n: number) => (n / 1_048_576).toFixed(1);
  switch (p.stage) {
    case "checking":
      return "Controllo della versione…";
    case "downloading":
      return p.total ? `Download ${mb(p.done)} / ${mb(p.total)} MB` : `Download ${mb(p.done)} MB`;
    case "extracting":
      return "Estrazione…";
    case "indexing":
      return "Indicizzazione…";
    case "done":
      return "Aggiornato";
    case "upToDate":
      return "Già aggiornato";
  }
}

function percent(p: Progress | undefined): number {
  if (!p) return 0;
  if (p.stage === "done" || p.stage === "upToDate") return 100;
  if (p.stage === "extracting" || p.stage === "indexing") return 96;
  if (p.stage === "downloading" && p.total) return Math.min(94, (p.done / p.total) * 94);
  if (p.stage === "downloading") return 40;
  return 4;
}

export function DataSetup({ onReady, compact = false }: { onReady: (status: DataStatus) => void; compact?: boolean }) {
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<Partial<Record<ProgressItem, Progress>>>({});
  const [error, setError] = useState<string | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const start = async (force: boolean) => {
    if (!isDesktop) return;
    sound.play("select");
    setRunning(true);
    setError(null);
    setProgress({});
    const unlisten = await api.onProgress((p) => setProgress((prev) => ({ ...prev, [p.item]: p })));
    try {
      const status = await api.updateData(force);
      if (status.engineError) {
        sound.play("error");
        setError(`script e database del motore non scaricati (${status.engineError}). Il catalogo delle carte funziona lo stesso; riprova più tardi.`);
      } else {
        sound.play("forge");
        burstFrom(buttonRef.current, 46, 1.4);
      }
      onReady(status);
    } catch (e) {
      sound.play("error");
      setError(String(e));
    } finally {
      unlisten();
      setRunning(false);
    }
  };

  const rows = (
    <ul className="flex flex-col gap-3">
      {ITEMS.map(({ item, label, size }) => {
        const p = progress[item];
        const finished = p?.stage === "done" || p?.stage === "upToDate";
        return (
          <li key={item} className="rounded-xl border border-border bg-bg-2 px-4 py-3">
            <div className="mb-2 flex items-center justify-between gap-4 text-sm">
              <span className="flex items-center gap-2">
                <span className={finished ? "text-ok" : running && p ? "text-gold" : "text-faint"}>
                  {finished ? "✓" : running && p ? <EmblemIcon emblem="gear" className="h-4 w-4 animate-[spin-slow_2.4s_linear_infinite]" /> : "○"}
                </span>
                {label}
              </span>
              <span className="text-xs text-muted tabular-nums">{p ? describe(p) : size}</span>
            </div>
            <div className="molten-track">
              <div className="molten-fill" style={{ width: `${percent(p)}%`, animationPlayState: finished ? "paused" : "running" }} />
            </div>
          </li>
        );
      })}
    </ul>
  );

  const buttons = (
    <div className="flex flex-wrap items-center gap-3">
      <button ref={buttonRef} type="button" className="btn btn-molten" disabled={running || !isDesktop} onClick={() => start(false)}>
        <EmblemIcon emblem="anvil" className="h-4 w-4" />
        {running ? "La fucina è al lavoro…" : compact ? "Aggiorna i dati" : "Accendi la fucina"}
      </button>
      {compact && (
        <button type="button" className="btn btn-ghost" disabled={running || !isDesktop} onClick={() => start(true)}>
          Riscarica tutto
        </button>
      )}
      {!isDesktop && <span className="text-xs text-warn">Disponibile solo nell'app desktop.</span>}
    </div>
  );

  if (compact) {
    return (
      <div className="flex flex-col gap-4">
        {rows}
        {buttons}
        {error && <p className="text-sm text-danger">Errore: {error}</p>}
      </div>
    );
  }

  return (
    <div className="mx-auto flex max-w-2xl flex-col items-center gap-6 py-10 text-center anim-rise">
      <span
        className="grid h-24 w-24 place-items-center rounded-3xl border text-ember"
        style={{
          borderColor: "color-mix(in oklab, var(--df-ember) 40%, transparent)",
          background: "radial-gradient(circle at 50% 70%, color-mix(in oklab, var(--df-ember) 30%, transparent), transparent 70%), var(--df-surface)",
          boxShadow: "0 0 60px -10px color-mix(in oklab, var(--df-ember) 70%, transparent)",
        }}
      >
        <EmblemIcon emblem="anvil" className="h-12 w-12" strokeWidth={1.4} />
      </span>
      <div>
        <h2 className="display text-3xl font-bold">La fucina è spenta</h2>
        <p className="mt-2 text-muted">
          Per accenderla servono i dati delle carte e del motore di gioco (circa 80 MB, una volta sola). Le immagini delle carte si scaricano solo quando
          servono.
        </p>
      </div>
      <div className="w-full text-left">{rows}</div>
      {buttons}
      {error && <p className="text-sm text-danger">Errore: {error}</p>}
    </div>
  );
}
