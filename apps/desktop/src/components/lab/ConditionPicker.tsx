import type { Card, Recipe } from "../../lib/api";
import { BUILTIN_RECIPES, describe, fromCond, toCond, type Group } from "../../lib/conditions";
import { roleLabel } from "../../lib/roles";
import { sound } from "../../lib/sound";

interface Props {
  root: Group;
  setRoot: (update: (g: Group) => Group) => void;
  recipes: Recipe[];
  getCard: (id: number) => Card | undefined;
  onEdit: () => void;
}

/** The shared condition, as a sentence, with the recipes to switch it. */
export function ConditionPicker({ root, setRoot, recipes, getCard, onEdit }: Props) {
  const text = describe(toCond(root), roleLabel, (id) => getCard(id)?.name ?? String(id));
  return (
    <section className="lab-card flex flex-col gap-2 px-4 py-3">
      <div className="flex items-baseline gap-3">
        <span className="eyebrow shrink-0">Condizione</span>
        <p className="min-w-0 flex-1 truncate text-sm" title={text}>
          {text || "nessuna"}
        </p>
        <button type="button" className="shrink-0 text-xs text-sec hover:underline" onClick={onEdit}>
          Modifica nel Calcolatore
        </button>
      </div>
      <div className="flex flex-wrap gap-1.5">
        {[...BUILTIN_RECIPES, ...recipes].map((r) => (
          <button
            key={r.name}
            type="button"
            className="chip !px-2 !py-0.5 !text-[11px]"
            title={"hint" in r && r.hint ? r.hint : r.name}
            onClick={() => {
              sound.play("page");
              setRoot(() => fromCond(r.cond));
            }}
          >
            {r.name}
          </button>
        ))}
      </div>
    </section>
  );
}
