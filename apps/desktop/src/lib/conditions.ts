/**
 * Lab conditions: the editable tree behind the condition builder, the
 * recipes every deck gets, and a plain-Italian description.
 */
import type { Cond, LabOp, LabTarget } from "./api";

/** The builder edits a tree of groups and clauses, turned into a `Cond`. */
export interface Clause {
  kind: "clause";
  id: number;
  target: LabTarget;
  op: LabOp;
  n: number;
}
export interface Group {
  kind: "group";
  id: number;
  mode: "all" | "any";
  negate: boolean;
  items: CondNode[];
}
export type CondNode = Clause | Group;

let nextId = 1;
export const clause = (target: LabTarget, op: LabOp = "atLeast", n = 1): Clause => ({ kind: "clause", id: nextId++, target, op, n });
export const group = (mode: "all" | "any", items: CondNode[], negate = false): Group => ({ kind: "group", id: nextId++, mode, negate, items });
export const role = (value: string): LabTarget => ({ kind: "role", value });

export function toCond(node: CondNode): Cond {
  if (node.kind === "clause") return { type: "count", target: node.target, op: node.op, n: node.n };
  const inner: Cond = { type: node.mode, items: node.items.map(toCond) };
  return node.negate ? { type: "not", item: inner } : inner;
}

export function fromCond(cond: Cond): Group {
  const asNode = (c: Cond): CondNode => {
    switch (c.type) {
      case "count":
        return clause(c.target, c.op, c.n);
      case "not": {
        const inner = asNode(c.item);
        return inner.kind === "group" ? { ...inner, negate: !inner.negate } : group("all", [inner], true);
      }
      default:
        return group(c.type, c.items.map(asNode));
    }
  };
  const root = asNode(cond);
  return root.kind === "group" ? root : group("all", [root]);
}

const at = (r: string, n = 1): Cond => ({ type: "count", target: role(r), op: "atLeast", n });
const resist: Cond = { type: "any", items: [at("starter", 2), { type: "all", items: [at("starter"), at("extender")] }] };
/** Recipes every deck gets. */
export const BUILTIN_RECIPES: { name: string; cond: Cond }[] = [
  { name: "Mano ideale", cond: { type: "all", items: [at("starter"), at("handtrap")] } },
  { name: "Gioca attraverso Ash", cond: resist },
  { name: "Mano solida", cond: { type: "all", items: [resist, at("handtrap")] } },
  { name: "Doppio starter", cond: at("starter", 2) },
  { name: "Starter senza brick", cond: { type: "all", items: [at("starter"), { type: "count", target: role("brick"), op: "exactly", n: 0 }] } },
  { name: "Breaker per secondo", cond: { type: "all", items: [at("starter"), at("breaker")] } },
];

export function usedRoles(c: Cond, out: string[] = []): string[] {
  if (c.type === "count") {
    if (c.target.kind === "role" && !out.includes(c.target.value)) out.push(c.target.value);
  } else if (c.type === "not") usedRoles(c.item, out);
  else c.items.forEach((i) => usedRoles(i, out));
  return out;
}

export const OPS: { op: LabOp; label: string }[] = [
  { op: "atLeast", label: "almeno" },
  { op: "atMost", label: "al massimo" },
  { op: "exactly", label: "esattamente" },
];

/** "circa 2 mani su 3", or "circa 1 mano su 8" for rare hands. */
export function plain(p: number): string {
  if (p >= 0.995) return "quasi sempre";
  if (p < 0.005) return "quasi mai";
  if (p < 0.5) return `circa 1 mano su ${Math.round(1 / p)}`;
  let best = { k: 1, d: 2, err: 1 };
  for (let d = 2; d <= 10; d++) {
    const k = Math.round(p * d);
    const err = Math.abs(p - k / d);
    if (err < best.err - 1e-9) best = { k, d, err };
  }
  return `circa ${best.k} mani su ${best.d}`;
}

/** "almeno 1 Starter e (almeno 2 Starter o almeno 1 Estensore)". */
export function describe(c: Cond, roleName: (key: string) => string, cardName: (id: number) => string, nested = false): string {
  switch (c.type) {
    case "count": {
      const what = c.target.kind === "role" ? roleName(c.target.value) : cardName(c.target.value);
      const op = OPS.find((o) => o.op === c.op)?.label ?? c.op;
      return `${op} ${c.n} ${what}`;
    }
    case "not":
      return `non (${describe(c.item, roleName, cardName, true)})`;
    default: {
      const text = c.items.map((i) => describe(i, roleName, cardName, true)).join(c.type === "all" ? " e " : " o ");
      return nested && c.items.length > 1 ? `(${text})` : text;
    }
  }
}
