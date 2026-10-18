/**
 * Helpers to edit the search text. Every filter in the UI is a token of the
 * query (see crates/ygo-data/src/query.rs), so the search bar always shows
 * exactly what is being searched.
 */

export function tokenize(query: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let quoted = false;
  for (const ch of query) {
    if (ch === '"') {
      quoted = !quoted;
      current += ch;
    } else if (/\s/.test(ch) && !quoted) {
      if (current) tokens.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  if (current) tokens.push(current);
  return tokens;
}

interface Parts {
  key: string;
  op: string;
  value: string;
}

export function splitToken(token: string): Parts | null {
  if (token.startsWith('"')) return null;
  const m = /^([a-zà-ù]+)(>=|<=|>|<|=|:)(.*)$/i.exec(token);
  if (!m) return null;
  return { key: m[1].toLowerCase(), op: m[2] === ":" ? "=" : m[2], value: m[3].replace(/^"|"$/g, "") };
}

const join = (tokens: string[]) => tokens.join(" ");

export function hasToken(query: string, token: string): boolean {
  return tokenize(query).some((t) => t.toLowerCase() === token.toLowerCase());
}

export function removeToken(query: string, token: string): string {
  return join(tokenize(query).filter((t) => t !== token));
}

export function toggleToken(query: string, token: string): string {
  return hasToken(query, token)
    ? join(tokenize(query).filter((t) => t.toLowerCase() !== token.toLowerCase()))
    : join([...tokenize(query), token]);
}

/** Removes the tokens whose key is one of `keys` and whose value passes `match`. */
export function removeKeys(query: string, keys: string[], match: (value: string) => boolean = () => true): string {
  return join(
    tokenize(query).filter((t) => {
      const p = splitToken(t);
      return !(p && keys.includes(p.key) && match(p.value.toLowerCase()));
    }),
  );
}

export function quote(value: string): string {
  return /\s/.test(value) ? `"${value}"` : value;
}

/** Sets a single-valued filter (`null` removes it). */
export function setKey(query: string, keys: string[], value: string | null): string {
  const rest = removeKeys(query, keys);
  return value ? join([...tokenize(rest), `${keys[0]}:${quote(value)}`]) : rest;
}

export function getKey(query: string, keys: string[]): string | null {
  for (const t of tokenize(query)) {
    const p = splitToken(t);
    if (p && keys.includes(p.key) && p.op === "=") return p.value;
  }
  return null;
}

export interface Range {
  min: string;
  max: string;
}

export function getRange(query: string, keys: string[]): Range {
  const range: Range = { min: "", max: "" };
  for (const t of tokenize(query)) {
    const p = splitToken(t);
    if (!p || !keys.includes(p.key)) continue;
    if (p.op === "=") {
      const [a, b] = p.value.split("-");
      range.min = a ?? "";
      range.max = b ?? a ?? "";
    } else if (p.op === ">=") range.min = p.value;
    else if (p.op === "<=") range.max = p.value;
  }
  return range;
}

export function setRange(query: string, keys: string[], range: Range): string {
  const rest = removeKeys(query, keys);
  const { min, max } = { min: range.min.trim(), max: range.max.trim() };
  const key = keys[0];
  const token = min && max ? `${key}:${min}-${max}` : min ? `${key}>=${min}` : max ? `${key}<=${max}` : null;
  return token ? join([...tokenize(rest), token]) : rest;
}

export const KEYS = {
  type: ["type", "t", "tipo"],
  attr: ["attr", "a", "attribute", "attributo"],
  race: ["race", "r", "razza"],
  arch: ["arch", "archetype", "archetipo"],
  level: ["lv", "level", "livello", "rank"],
  atk: ["atk"],
  def: ["def"],
  format: ["format", "f", "formato"],
  rarity: ["rarity", "rarità", "rarita"],
  sort: ["sort", "ordina"],
};

export const KIND_VALUES = ["monster", "mostro", "mostri", "spell", "magia", "magie", "trap", "trappola", "trappole"];
