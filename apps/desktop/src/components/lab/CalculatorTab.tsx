import { useEffect, useState, type ReactNode } from "react";
import { api, type Card, type LabOp, type LabTarget, type Recipe } from "../../lib/api";
import { BUILTIN_RECIPES, clause, fromCond, group, OPS, plain, role, toCond, type Clause, type CondNode, type Group } from "../../lib/conditions";
import { roleDefs, roleLabel } from "../../lib/roles";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";
import { pct } from "../../lib/tooltip";
import { FlaskMeter } from "../charts";

interface Props {
  deckId: number;
  main: number[];
  roles: Record<string, number[]>;
  hand: "first" | "second";
  getCard: (id: number) => Card | undefined;
  /** The condition, shared with the simulator and the optimizer. */
  root: Group;
  setRoot: (update: (g: Group) => Group) => void;
  recipes: Recipe[];
  setRecipes: (update: (list: Recipe[]) => Recipe[]) => void;
}

/** Calcolatore: build a condition, get its exact probability. */
export function CalculatorTab({ deckId, main, roles, hand, getCard, root, setRoot, recipes, setRecipes }: Props) {
  const [draws, setDraws] = useState(0);
  const [result, setResult] = useState<{
    first: number | null;
    second: number | null;
    firstError: string | null;
    secondError: string | null;
  } | null>(null);
  const [name, setName] = useState("");
  const defs = roleDefs(roles);
  const ids = [...new Set(main)];
  const cond = toCond(root);
  const key = JSON.stringify([cond, draws, main, roles]);

  useEffect(() => {
    // An answer for an older condition must not overwrite a newer one.
    let stale = false;
    const t = window.setTimeout(async () => {
      try {
        const [a, b] = await Promise.all([
          api.labProbability(main, roles, cond, 5 + draws),
          api.labProbability(main, roles, cond, 6 + draws),
        ]);
        if (stale) return;
        setResult({
          first: a.p,
          second: b.p,
          firstError: a.error,
          secondError: b.error,
        });
        sound.play("bubble");
      } catch (e) {
        if (!stale) toast(`Probabilità non calcolata: ${e}`, "error");
      }
    }, 140);
    return () => {
      stale = true;
      window.clearTimeout(t);
    };
    // `key` covers the condition, the draws and the deck.
  }, [key]); // eslint-disable-line react-hooks/exhaustive-deps

  const update = (fn: (g: Group) => Group) => setRoot((r) => fn(structuredClone(r)));
  const findGroup = (g: Group, id: number): Group | null => {
    if (g.id === id) return g;
    for (const n of g.items) if (n.kind === "group") {
      const found = findGroup(n, id);
      if (found) return found;
    }
    return null;
  };
  const edit = (id: number, change: (n: CondNode) => void) =>
    update((r) => {
      const visit = (g: Group) => {
        if (g.id === id) change(g);
        g.items.forEach((n) => (n.id === id ? change(n) : n.kind === "group" && visit(n)));
      };
      visit(r);
      return r;
    });
  const remove = (id: number) =>
    update((r) => {
      const visit = (g: Group) => {
        g.items = g.items.filter((n) => n.id !== id);
        g.items.forEach((n) => n.kind === "group" && visit(n));
      };
      visit(r);
      return r;
    });
  const add = (groupId: number, node: CondNode) =>
    update((r) => {
      findGroup(r, groupId)?.items.push(node);
      return r;
    });

  const targetValue = (t: LabTarget) => `${t.kind}:${t.value}`;
  const parseTarget = (v: string): LabTarget => (v.startsWith("card:") ? { kind: "card", value: Number(v.slice(5)) } : { kind: "role", value: v.slice(5) });

  const renderClause = (c: Clause) => {
    const targetTitle = c.target.kind === "card" ? (getCard(c.target.value)?.name ?? String(c.target.value)) : roleLabel(c.target.value);
    return (
    <div key={c.id} className="flex items-center gap-1.5">
      <select className="field !w-28 xl:!w-32 shrink-0 !py-1 text-sm" value={c.op} onChange={(e) => edit(c.id, (n) => ((n as Clause).op = e.target.value as LabOp))} aria-label="Confronto">
        {OPS.map((o) => (
          <option key={o.op} value={o.op}>
            {o.label}
          </option>
        ))}
      </select>
      <input
        type="number"
        min={0}
        max={10}
        className="field !w-12 xl:!w-14 shrink-0 !py-1 text-center text-sm tabular-nums"
        value={c.n}
        onChange={(e) => edit(c.id, (n) => ((n as Clause).n = Math.max(0, Math.min(10, Number(e.target.value) || 0))))}
        aria-label="Numero di carte"
      />
      <select className="field min-w-0 flex-1 !py-1 text-sm" title={targetTitle} value={targetValue(c.target)} onChange={(e) => edit(c.id, (n) => ((n as Clause).target = parseTarget(e.target.value)))} aria-label="Ruolo o carta">
        <optgroup label="Ruoli">
          {defs.map((d) => (
            <option key={d.key} value={`role:${d.key}`}>
              {d.label} ({main.filter((id) => roles[d.key]?.includes(id)).length})
            </option>
          ))}
        </optgroup>
        <optgroup label="Carte">
          {ids.map((id) => (
            <option key={id} value={`card:${id}`}>
              {getCard(id)?.name ?? id}
            </option>
          ))}
        </optgroup>
      </select>
      <button type="button" className="btn btn-ghost !px-2 !py-1 text-xs text-faint hover:text-danger" onClick={() => remove(c.id)} aria-label="Elimina clausola">
        ✕
      </button>
    </div>
  );
  };

  const renderGroup = (g: Group, depth: number): ReactNode => (
    <div key={g.id} className={`flex flex-col gap-2 rounded-xl p-3 ${depth === 0 ? "bg-bg-2/50" : "border border-border/80 bg-surface/50"}`}>
      <div className="flex items-center gap-2 text-xs">
        <span className="text-muted">Condizione:</span>
        <div className="flex gap-1 rounded-md border border-border bg-bg-2 p-0.5">
          {(["all", "any"] as const).map((m) => (
            <button key={m} type="button" onClick={() => edit(g.id, (n) => ((n as Group).mode = m))} className={`rounded px-2 py-0.5 font-semibold ${g.mode === m ? "bg-sec text-bg" : "text-muted hover:text-text"}`}>
              {m === "all" ? "Tutte vere (AND)" : "Almeno una (OR)"}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={() => edit(g.id, (n) => ((n as Group).negate = !(n as Group).negate))}
          className={`chip !px-2 !py-0.5 ${g.negate ? "chip-active !bg-danger/20 !border-danger !text-danger" : ""}`}
        >
          {g.negate ? "Invertita (NON)" : "Non invertita"}
        </button>
        {depth > 0 && (
          <button type="button" className="ml-auto text-xs text-faint hover:text-danger" onClick={() => remove(g.id)}>
            Elimina gruppo
          </button>
        )}
      </div>
      {g.items.map((n) => (n.kind === "clause" ? renderClause(n) : renderGroup(n, depth + 1)))}
      <div className="flex gap-2">
        <button type="button" className="btn btn-ghost !px-2.5 !py-1 text-xs" onClick={() => add(g.id, clause(role("starter")))}>
          ＋ Condizione
        </button>
        {depth === 0 && (
          <button type="button" className="btn btn-ghost !px-2.5 !py-1 text-xs" onClick={() => add(g.id, group("any", [clause(role("starter"), "atLeast", 2), clause(role("extender"))]))}>
            ＋ Gruppo
          </button>
        )}
      </div>
    </div>
  );

  const currentP = hand === "first" ? result?.first : result?.second;
  const currentError = hand === "first" ? result?.firstError : result?.secondError;
  const otherP = hand === "first" ? result?.second : result?.first;
  const otherError = hand === "first" ? result?.secondError : result?.firstError;
  const otherLabel = hand === "first" ? "per secondo" : "per primo";
  const handSize = (hand === "first" ? 5 : 6) + draws;

  const saveRecipe = async () => {
    try {
      const r = await api.saveRecipe(deckId, name, cond);
      setRecipes((list) => [...list, r]);
      setName("");
      sound.play("glass");
      toast(`Ricetta «${r.name}» salvata`, "ok");
    } catch (e) {
      sound.play("error");
      toast(String(e), "error");
    }
  };

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_272px] xl:grid-cols-[minmax(0,1fr)_300px] gap-4">
      <div className="flex min-w-0 flex-col gap-4">
        <section className="lab-card flex flex-col gap-3 p-4">
          <div className="flex items-baseline justify-between">
            <h3 className="text-sm font-semibold">La condizione</h3>
            <span className="text-xs text-faint">Ruoli o carte precise, con soglie e operatori</span>
          </div>
          {renderGroup(root, 0)}
        </section>
        <section className="lab-card flex flex-wrap items-center gap-4 p-4">
          <span className="text-sm">Carte in mano</span>
          <span className="text-xs text-muted">mano iniziale (5 per primo, 6 per secondo) più</span>
          <div className="flex items-center gap-1">
            {[0, 1, 2, 3, 4, 5].map((d) => (
              <button key={d} type="button" onClick={() => setDraws(d)} className={`h-7 w-7 rounded-md text-xs font-semibold ${draws === d ? "bg-sec text-bg" : "border border-border text-muted hover:text-text"}`}>
                {d}
              </button>
            ))}
          </div>
          <span className="text-xs text-muted">pescate dopo</span>
        </section>
      </div>

      <div className="flex flex-col gap-4">
        <section className="lab-card flex flex-col items-center gap-2 p-5 text-center">
          <FlaskMeter value={currentP ?? 0} size={64} bubbling />
          {currentError ? (
            <p className="text-sm text-warn">{currentError}. Puoi stimarla nella scheda Simulatore, che usa la stessa condizione.</p>
          ) : (
            <>
              <p className="text-4xl font-semibold">{currentP == null ? "…" : pct(currentP)}</p>
              <p className="text-sm text-muted">{currentP == null ? "" : plain(currentP)}</p>
              <p className="text-xs text-faint">
                {handSize} carte, {hand === "first" ? "per primo" : "per secondo"} · {otherLabel} {otherError || otherP == null ? "—" : pct(otherP)}
              </p>
            </>
          )}
        </section>
        <section className="lab-card flex flex-col gap-2 p-4">
          <h3 className="text-sm font-semibold">Ricette</h3>
          <div className="flex flex-wrap gap-1.5">
            {BUILTIN_RECIPES.map((r) => (
              <button
                key={r.name}
                type="button"
                className="chip !px-2 !py-0.5 !text-[11px]"
                title={r.hint ?? r.name}
                onClick={() => (sound.play("page"), setRoot(() => fromCond(r.cond)))}
              >
                {r.name}
              </button>
            ))}
            {recipes.map((r) => (
              <span key={r.id} className="chip chip-active !gap-1 !px-2 !py-0.5 !text-[11px]">
                <button type="button" onClick={() => (sound.play("page"), setRoot(() => fromCond(r.cond)))}>
                  {r.name}
                </button>
                <button
                  type="button"
                  className="text-faint hover:text-danger"
                  aria-label={`Elimina ${r.name}`}
                  onClick={async () => {
                    await api.deleteRecipe(r.id);
                    sound.play("close");
                    setRecipes((list) => list.filter((x) => x.id !== r.id));
                  }}
                >
                  ✕
                </button>
              </span>
            ))}
          </div>
          <form
            className="flex gap-1.5"
            onSubmit={(e) => {
              e.preventDefault();
              if (name.trim()) void saveRecipe();
            }}
          >
            <input className="field min-w-0 flex-1 !py-1 text-xs" placeholder="Nome ricetta…" value={name} onChange={(e) => setName(e.target.value)} />
            <button type="submit" className="btn btn-ghost !py-1 text-xs" disabled={!name.trim()}>
              Salva
            </button>
          </form>
          <p className="text-[11px] text-faint">Salva questa condizione per ritrovarla con un clic anche nelle altre schede.</p>
        </section>
      </div>
    </div>
  );
}
