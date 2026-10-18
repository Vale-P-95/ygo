import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";

/** Mirrors `EngineStatus` in src-tauri/src/lib.rs. */
interface EngineStatus {
  apiVersion: string;
  dataDir: string;
  cards: number | null;
  scripts: number | null;
  error: string | null;
}

type State = { kind: "loading" } | { kind: "browser" } | { kind: "ready"; status: EngineStatus } | { kind: "failed"; message: string };

export function EngineStatusPanel() {
  const [state, setState] = useState<State>({ kind: "loading" });

  useEffect(() => {
    if (!isTauri()) {
      setState({ kind: "browser" });
      return;
    }
    invoke<EngineStatus>("engine_status")
      .then((status) => setState({ kind: "ready", status }))
      .catch((e) => setState({ kind: "failed", message: String(e) }));
  }, []);

  return (
    <section className="rounded-xl border border-border bg-surface p-5">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-muted">Motore di gioco</h2>
      {state.kind === "loading" && <p className="mt-3 text-muted">Caricamento dei dati…</p>}
      {state.kind === "browser" && (
        <p className="mt-3 text-muted">Anteprima nel browser: il motore è disponibile solo nell'app desktop.</p>
      )}
      {state.kind === "failed" && <p className="mt-3 text-warn">Errore: {state.message}</p>}
      {state.kind === "ready" && (
        <dl className="mt-3 grid grid-cols-[max-content_1fr] gap-x-6 gap-y-2 text-sm">
          <dt className="text-muted">Motore</dt>
          <dd>EDOPro core, API {state.status.apiVersion}</dd>
          <dt className="text-muted">Cartella dati</dt>
          <dd className="break-all font-mono text-xs leading-5">{state.status.dataDir}</dd>
          {state.status.error ? (
            <>
              <dt className="text-muted">Dati</dt>
              <dd className="text-warn">
                Non trovati. Scaricali con <code className="font-mono">duelforge-cli fetch-data --data &lt;cartella&gt;</code>.
              </dd>
            </>
          ) : (
            <>
              <dt className="text-muted">Carte</dt>
              <dd>{state.status.cards?.toLocaleString("it-IT")}</dd>
              <dt className="text-muted">Script</dt>
              <dd>{state.status.scripts?.toLocaleString("it-IT")}</dd>
            </>
          )}
        </dl>
      )}
    </section>
  );
}
