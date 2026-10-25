import { useEffect, useRef, useState, type ReactNode } from "react";
import { Highlighter } from "lucide-react";
import { api, type CardAnatomy, type EffectAnatomy, type TextPart, type TextPiece } from "../lib/api";

/**
 * The anatomy of a card's effects (docs/plans/anatomia-effetti.md §3): the
 * «Anatomia» button colours in the card text its condition, cost, action,
 * restriction and limit, and lists each effect as the script registers it.
 */

export const PART_LABELS: Record<TextPart, string> = {
  condition: "Condizione",
  cost: "Costo",
  action: "Azione",
  restriction: "Restrizione",
  limit: "Limite",
  materials: "Materiali",
};

const PART_ORDER: TextPart[] = ["condition", "cost", "action", "restriction", "limit", "materials"];

const partColor = (part: TextPart) => `var(--anat-${part})`;

const KIND_LABELS: Record<EffectAnatomy["kind"], string> = {
  activation: "Attivazione",
  ignition: "Ignizione",
  trigger: "Innescato",
  quick: "Rapido",
  flip: "Scoperta",
  continuous: "Continuo",
  procedure: "Procedura",
};

const ZONE_LABELS: Record<EffectAnatomy["zones"][number], string> = {
  hand: "dalla mano",
  field: "sul campo",
  spellTrap: "dalla zona Magie e Trappole",
  gy: "dal Cimitero",
  banished: "dalle carte bandite",
  deck: "dal Deck",
  extra: "dall'Extra Deck",
  fieldZone: "dalla zona Terreno",
  pendulumZone: "dalla zona Pendulum",
};

/** The engine's category names (`consts::category::NAMES`) in Italian. */
const CATEGORY_LABELS: Record<string, string> = {
  destroy: "distrugge",
  release: "offre come tributo",
  banish: "bandisce",
  "to hand": "aggiunge alla mano",
  "to deck": "rimanda nel Deck",
  "to GY": "manda al Cimitero",
  mill: "manda dal Deck al Cimitero",
  discard: "fa scartare",
  "Normal Summon": "Evocazione Normale",
  "Special Summon": "Evocazione Speciale",
  token: "Segna-mostro",
  flip: "scopre",
  position: "cambia posizione",
  control: "prende il controllo",
  "negate effect": "nega l'effetto",
  "negate summon": "nega l'Evocazione",
  draw: "pesca",
  search: "cerca nel Deck",
  equip: "equipaggia",
  damage: "infligge danno",
  recover: "fa guadagnare LP",
  "ATK change": "cambia l'ATK",
  "DEF change": "cambia la DEF",
  counter: "segnalini",
  coin: "lancia una moneta",
  dice: "tira un dado",
  "leave GY": "dal Cimitero",
  "level change": "cambia il Livello",
  "negate activation": "nega l'attivazione",
  announce: "dichiara",
  "Fusion Summon": "Evocazione Fusion",
  "to Extra Deck": "nell'Extra Deck",
  set: "posiziona",
};

const STORAGE_KEY = "duelforge.anatomy";

/** The button's state, kept for the session (shared by the inspector and the card viewer). */
export function useAnatomyToggle(): [boolean, () => void] {
  const [on, setOn] = useState(() => {
    try {
      return sessionStorage.getItem(STORAGE_KEY) === "1";
    } catch {
      return false;
    }
  });
  const toggle = () =>
    setOn((was) => {
      try {
        sessionStorage.setItem(STORAGE_KEY, was ? "0" : "1");
      } catch {
        // Storage not available (private window): the button still works for this view.
      }
      return !was;
    });
  return [on, toggle];
}

type Loaded = { id: number; anatomy: CardAnatomy } | { id: number; error: string };

/**
 * The anatomy of `cardId`, loaded only while `on`. A late answer for a card
 * no longer shown is dropped; a failure is not retried until the card changes.
 */
export function useCardAnatomy(cardId: number, on: boolean): { anatomy: CardAnatomy | null; error: string | null; loading: boolean } {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const request = useRef(0);
  const current = loaded?.id === cardId ? loaded : null;
  useEffect(() => {
    if (!on || current) return;
    const id = ++request.current;
    api
      .cardAnatomy(cardId)
      .then((anatomy) => {
        if (id === request.current) setLoaded({ id: cardId, anatomy });
      })
      .catch((e: unknown) => {
        if (id === request.current) setLoaded({ id: cardId, error: `Anatomia non disponibile: ${String(e)}` });
      });
    return () => {
      // A newer card or a closed button makes this answer old.
      request.current += 1;
    };
  }, [cardId, on, current]);
  return {
    anatomy: current && "anatomy" in current ? current.anatomy : null,
    error: current && "error" in current ? current.error : null,
    loading: on && !current,
  };
}

export function AnatomyToggle({ on, onToggle }: { on: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={on}
      title="Anatomia: colora nel testo condizione, costo, azione, restrizione e limite"
      onClick={onToggle}
      className={`inline-flex h-7 items-center gap-1.5 rounded-full border px-2.5 text-xs font-semibold transition-colors ${
        on ? "border-gold/70 bg-gold/15 text-gold" : "border-border text-muted hover:border-border-strong hover:text-text"
      }`}
    >
      <Highlighter size={14} aria-hidden="true" />
      Anatomia
    </button>
  );
}

/** Text with "quoted card names" turned into search links. */
function Linked({ text, onSearch }: { text: string; onSearch?: (query: string) => void }) {
  const parts = text.split(/("[^"\n]+")/g);
  return (
    <>
      {parts.map((part, i) =>
        part.startsWith('"') && onSearch ? (
          <button key={i} type="button" className="text-sec underline decoration-dotted underline-offset-2 hover:text-sec-2" onClick={() => onSearch(part)}>
            {part}
          </button>
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </>
  );
}

/** A card text, coloured by parts when `pieces` are given. */
export function AnatomyText({
  text,
  pieces,
  onSearch,
  className = "whitespace-pre-line text-[13px] leading-relaxed text-text/90 select-text",
}: {
  text: string;
  pieces?: TextPiece[] | null;
  onSearch?: (query: string) => void;
  className?: string;
}) {
  if (!pieces?.length) {
    return (
      <p className={className}>
        <Linked text={text} onSearch={onSearch} />
      </p>
    );
  }
  return (
    <p className={className}>
      {pieces.map((piece, i) =>
        piece.part ? (
          <span
            key={i}
            title={PART_LABELS[piece.part]}
            className="rounded-[3px] px-0.5 [box-decoration-break:clone]"
            style={{
              background: `color-mix(in oklab, ${partColor(piece.part)} 20%, transparent)`,
              borderBottom: `2px solid ${partColor(piece.part)}`,
            }}
          >
            <Linked text={piece.text} onSearch={onSearch} />
          </span>
        ) : (
          <Linked key={i} text={piece.text} onSearch={onSearch} />
        ),
      )}
    </p>
  );
}

export function AnatomyLegend({ pieces }: { pieces: TextPiece[] }) {
  const count = (part: TextPart) => pieces.filter((p) => p.part === part).length;
  return (
    <div className="flex flex-wrap gap-x-3 gap-y-1 border-t border-border pt-1.5 text-xs text-muted">
      {PART_ORDER.filter((p) => p !== "materials" || count(p) > 0).map((part) => (
        <span key={part} className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="h-1.5 w-3.5 rounded-sm" style={{ background: partColor(part) }} />
          {PART_LABELS[part]}
          <span className="text-faint">{count(part)}</span>
        </span>
      ))}
    </div>
  );
}

function limitLabel(e: EffectAnatomy): string | null {
  if (!e.limit) return null;
  const times = e.limit.count === 1 ? "1 volta" : `${e.limit.count} volte`;
  const shared = e.limit.sharedWith.length ? ` · condiviso con ${e.limit.sharedWith.map((i) => i + 1).join(", ")}` : "";
  return `${times} ${e.limit.perDuel ? "per duello" : "per turno"}${shared}`;
}

function Row({ part, children }: { part: TextPart; children: ReactNode }) {
  return (
    <div className="grid grid-cols-1 gap-x-2 gap-y-0.5 text-xs leading-relaxed @[26rem]:grid-cols-[6.5rem_minmax(0,1fr)]">
      <span className="flex items-center gap-1.5 self-start pt-px text-muted">
        <span aria-hidden="true" className="h-2 w-2 rounded-[2px]" style={{ background: partColor(part) }} />
        {PART_LABELS[part]}
      </span>
      <span className="text-text/90">{children}</span>
    </div>
  );
}

/** One card per effect of the script, with the parts of its text when they match. */
export function AnatomyEffects({ anatomy }: { anatomy: CardAnatomy }) {
  const { effects, text } = anatomy;
  const unmatched = effects.some((e) => !["continuous", "procedure"].includes(e.kind) && e.textEffect == null);
  if (anatomy.scriptError) {
    return <p className="text-xs text-muted">Lo script della carta non si è potuto leggere: restano colorate solo le parti del testo. ({anatomy.scriptError})</p>;
  }
  if (!effects.length) return null;
  const materials = text.filter((p) => p.part === "materials").map((p) => p.text.trim());
  return (
    <section aria-label="Anatomia degli effetti" className="@container flex flex-col gap-2">
      <p className="eyebrow">
        Anatomia · {effects.length} {effects.length === 1 ? "effetto" : "effetti"}
      </p>
      {effects.map((e, i) => {
        const pieces = e.textEffect == null ? [] : text.filter((p) => p.effect === e.textEffect && p.part);
        const limit = limitLabel(e);
        return (
          <div key={i} className="flex flex-col gap-1.5 rounded-lg border border-border bg-surface-2 p-2.5">
            <div className="flex flex-wrap items-center gap-1.5 text-xs">
              <span className="font-bold">{i + 1}</span>
              <span className="rounded-full bg-surface-3 px-2 py-0.5 font-semibold">
                {KIND_LABELS[e.kind]}
                {e.kind === "trigger" || (e.kind === "quick" && e.mandatory) ? (e.mandatory ? " · obbligatorio" : " · facoltativo") : ""}
              </span>
              {e.zones.length > 0 && <span className="text-muted">{e.zones.map((z) => ZONE_LABELS[z]).join(", ")}</span>}
              {limit && (
                <span className="rounded-full border px-2 py-0.5" style={{ borderColor: partColor("limit"), color: partColor("limit") }}>
                  {limit}
                </span>
              )}
              {e.targets && <span className="text-muted">◎ con bersaglio</span>}
            </div>
            {(e.categories.length > 0 || e.textOnly.length > 0) && (
              <div className="flex flex-wrap gap-1 text-xs">
                {e.categories.map((c) => (
                  <span key={c} className="chip">
                    {CATEGORY_LABELS[c] ?? c}
                  </span>
                ))}
                {e.textOnly.map((c) => (
                  <span key={c} className="chip border-dashed text-muted" title="Il testo lo dice, lo script non lo dichiara">
                    {CATEGORY_LABELS[c] ?? c} · solo dal testo
                  </span>
                ))}
              </div>
            )}
            {e.kind === "procedure" && materials.length > 0 && <Row part="materials">{materials.join(" ")}</Row>}
            {pieces.map((p, j) => p.part && (
              <Row key={j} part={p.part}>
                {p.text.trim()}
              </Row>
            ))}
            {pieces.length === 0 && e.description && <p className="text-xs text-muted">{e.description}</p>}
          </div>
        );
      })}
      {unmatched && (
        <p className="text-xs text-faint">Testo e script non hanno gli stessi effetti uno a uno: le parti restano colorate nel testo, non sotto ogni effetto.</p>
      )}
    </section>
  );
}

/** Loading and error lines under the text. */
export function AnatomyStatus({ loading, error }: { loading: boolean; error: string | null }) {
  if (error) return <p className="text-xs text-danger">{error}</p>;
  if (loading) return <p className="text-xs text-muted">Lettura degli effetti…</p>;
  return null;
}
