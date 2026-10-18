/**
 * Card roles of the lab. The five built-in roles have fixed names and
 * colors; custom roles take the next free color. Colors are CSS tokens
 * (index.css), checked for color-vision deficiencies in both themes; a
 * role always shows its letter too, so color is never the only cue.
 */
export interface RoleDef {
  key: string;
  label: string;
  letter: string;
  color: string;
  hint: string;
}

export const BUILTIN_ROLES: RoleDef[] = [
  { key: "starter", label: "Starter", letter: "S", color: "var(--role-starter)", hint: "Da sola avvia la combo" },
  { key: "extender", label: "Estensore", letter: "E", color: "var(--role-extender)", hint: "Con uno starter, ti fa continuare dopo un'interruzione" },
  { key: "handtrap", label: "Handtrap", letter: "H", color: "var(--role-handtrap)", hint: "Interrompe l'avversario dalla mano" },
  { key: "breaker", label: "Board breaker", letter: "B", color: "var(--role-breaker)", hint: "Smonta una board già pronta" },
  { key: "brick", label: "Brick", letter: "X", color: "var(--role-brick)", hint: "Carta che non vuoi pescare (Garnet)" },
];

const CUSTOM_COLORS = ["var(--role-custom-1)", "var(--role-custom-2)", "var(--role-custom-3)"];
export const MAX_CUSTOM_ROLES = CUSTOM_COLORS.length;

/** Built-in roles, then the deck's custom roles in the order they appear. */
export function roleDefs(roles: Record<string, number[]>): RoleDef[] {
  const custom = Object.keys(roles).filter((k) => !BUILTIN_ROLES.some((r) => r.key === k));
  return [
    ...BUILTIN_ROLES,
    ...custom.slice(0, MAX_CUSTOM_ROLES).map((key, i) => ({ key, label: key, letter: key.slice(0, 1).toUpperCase(), color: CUSTOM_COLORS[i], hint: "Ruolo personalizzato" })),
  ];
}

export function roleLabel(key: string): string {
  return BUILTIN_ROLES.find((r) => r.key === key)?.label ?? key;
}
