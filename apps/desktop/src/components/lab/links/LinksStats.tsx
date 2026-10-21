import type { Card, LinkCard } from "../../../lib/api";
import { sound } from "../../../lib/sound";

interface Props {
  cards: LinkCard[];
  getCard: (code: number) => Card | undefined;
  selectedCode: number | null;
  onSelectCard: (code: number) => void;
}

export function LinksStats({ cards, getCard, selectedCode, onSelectCard }: Props) {
  const starters = [...cards]
    .filter((c) => !c.extra && c.reachAll > 0)
    .sort((a, b) => b.reachAll - a.reachAll || b.chance - a.chance)
    .slice(0, 4);

  const keyCards = [...cards]
    .filter((c) => c.key > 0.05)
    .sort((a, b) => b.key - a.key)
    .slice(0, 4);

  const isolated = cards.filter((c) => c.reach === 0 && !c.extra);

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
      {/* Top starters */}
      <div className="rounded-lg border border-border bg-surface-2/70 p-2.5 flex flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-text flex items-center gap-1.5">
            <span className="text-sec">⚡</span> Starter principali
          </span>
          <span className="text-[10px] text-muted">per carte portate in gioco</span>
        </div>
        <div className="flex flex-wrap gap-1 mt-0.5">
          {starters.length === 0 ? (
            <span className="text-faint text-[11px]">Nessuno starter identificato.</span>
          ) : (
            starters.map((c) => {
              const name = getCard(c.code)?.name ?? String(c.code);
              const active = selectedCode === c.code;
              return (
                <button
                  key={c.code}
                  type="button"
                  onClick={() => {
                    sound.play("page");
                    onSelectCard(c.code);
                  }}
                  className={`chip !py-0.5 !px-2 flex items-center gap-1.5 ${active ? "chip-active" : ""}`}
                  title={`${name}: porta a ${c.reachAll} carte in totale (mano iniziale: ${Math.round(c.chance * 100)}%)`}
                >
                  <span className="truncate max-w-[120px]">{name}</span>
                  <span className="text-[10px] font-semibold text-sec tabular-nums">+{c.reachAll}</span>
                </button>
              );
            })
          )}
        </div>
      </div>

      {/* Key cards */}
      <div className="rounded-lg border border-border bg-surface-2/70 p-2.5 flex flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-text flex items-center gap-1.5">
            <span className="text-amber-500">★</span> Carte chiave
          </span>
          <span className="text-[10px] text-muted">da proteggere dalle negazioni</span>
        </div>
        <div className="flex flex-wrap gap-1 mt-0.5">
          {keyCards.length === 0 ? (
            <span className="text-faint text-[11px]">Nessun collo di bottiglia critico.</span>
          ) : (
            keyCards.map((c) => {
              const name = getCard(c.code)?.name ?? String(c.code);
              const active = selectedCode === c.code;
              return (
                <button
                  key={c.code}
                  type="button"
                  onClick={() => {
                    sound.play("page");
                    onSelectCard(c.code);
                  }}
                  className={`chip !py-0.5 !px-2 flex items-center gap-1.5 border-amber-500/40 ${active ? "chip-active !bg-amber-500/20" : ""}`}
                  title={`${name}: se negata si perdono in media ${c.key.toFixed(2)} carte`}
                >
                  <span className="truncate max-w-[120px]">{name}</span>
                  <span className="text-[10px] font-bold text-amber-500 tabular-nums">−{c.key.toFixed(1)}</span>
                </button>
              );
            })
          )}
        </div>
      </div>

      {/* Isolated cards */}
      <div className="rounded-lg border border-border bg-surface-2/70 p-2.5 flex flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-text flex items-center gap-1.5">
            <span className="text-muted">🔒</span> Carte isolate / Handtrap
          </span>
          <span className="text-[10px] text-muted">{isolated.length} carte del Main</span>
        </div>
        <div className="flex flex-wrap gap-1 mt-0.5">
          {isolated.slice(0, 4).map((c) => {
            const name = getCard(c.code)?.name ?? String(c.code);
            const active = selectedCode === c.code;
            return (
              <button
                key={c.code}
                type="button"
                onClick={() => {
                  sound.play("page");
                  onSelectCard(c.code);
                }}
                className={`chip !py-0.5 !px-1.5 text-[11px] text-muted ${active ? "chip-active" : ""}`}
                title={`${name}: nessuna carta del mazzo vi conduce direttamente`}
              >
                <span className="truncate max-w-[110px]">{name}</span>
              </button>
            );
          })}
          {isolated.length > 4 && (
            <span className="text-[10px] text-faint self-center">+{isolated.length - 4} altre</span>
          )}
        </div>
      </div>
    </div>
  );
}
