import type { ComboBoard, ComboGoal, GoalCard, GoalZone } from "./api";

export const MAX_GOAL_CARDS = 5;

/** Every place a card of the goal can be wanted; `late` ones only count on boards played through the End Phase. */
export const GOAL_ZONES: { zone: GoalZone; label: string; hint: string; late?: boolean }[] = [
  { zone: "any", label: "Sul campo", hint: "ovunque sul tuo campo (mostro scoperto o Magia/Trappola)" },
  { zone: "monster", label: "Zona Mostri", hint: "in una Zona Mostri, scoperto" },
  { zone: "main", label: "Zona principale", hint: "in una Zona Mostri principale" },
  { zone: "extra", label: "Zona Extra", hint: "nella Zona Mostri Extra" },
  { zone: "faceup", label: "M/T scoperta", hint: "Magia o Trappola scoperta" },
  { zone: "set", label: "M/T coperta", hint: "Magia o Trappola coperta" },
  { zone: "fieldzone", label: "Zona Terreno", hint: "nella Zona Terreno" },
  { zone: "grave", label: "Cimitero", hint: "nel Cimitero", late: true },
  { zone: "banished", label: "Bandita", hint: "tra le carte bandite", late: true },
  { zone: "hand", label: "Mano", hint: "ancora in mano", late: true },
];

export const zoneLabel = (zone: GoalZone) => GOAL_ZONES.find((z) => z.zone === zone)?.label ?? zone;

export const isHard = (g: GoalCard) => g.hard ?? true;

/** A goal as saved before zones existed (plain codes) or before soft cards still loads. */
export function goalCards(saved: unknown): GoalCard[] {
  if (!Array.isArray(saved)) return [];
  return saved.map((g) => (typeof g === "number" ? { code: g, zone: "any" as GoalZone } : (g as GoalCard)));
}

/** Whether a board has the hard cards of the goal (all of them, or one); the soft ones only rank. */
export function reaches(b: ComboBoard, goal: ComboGoal | null): boolean {
  if (!goal) return false;
  const hard = goal.cards.filter(isHard).length;
  return hard > 0 && (goal.all ? b.goalHits >= hard : b.goalHits > 0);
}

/** The goal in words, with the place of each card: "A (Cimitero) + B (M/T coperta)", soft cards apart. */
export function describeGoal(goal: ComboGoal, name: (code: number) => string): string {
  const one = (g: GoalCard) => `${name(g.code)}${g.zone === "any" ? "" : ` (${zoneLabel(g.zone)})`}`;
  const hard = goal.cards.filter(isHard).map(one);
  const soft = goal.cards.filter((g) => !isHard(g)).map(one);
  const wanted = hard.length <= 1 ? (hard[0] ?? "") : goal.all ? hard.join(" + ") : `una tra ${hard.join(", ")}`;
  if (!soft.length) return wanted;
  return `${wanted}${wanted ? ", meglio se anche " : "meglio "}${soft.join(", ")}`;
}
