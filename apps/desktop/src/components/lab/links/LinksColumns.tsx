import { useMemo } from "react";
import type { Card, CardLink, LinkCard } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { CardImage } from "../../combo/Field";
import type { LinksFilterState } from "./types";

interface Props {
  cards: LinkCard[];
  links: CardLink[];
  getCard: (code: number) => Card | undefined;
  filters: LinksFilterState;
  activeCardCode: number | null;
  onSelectCard: (code: number | null) => void;
}

export function LinksColumns({
  cards,
  links,
  getCard,
  filters,
  activeCardCode,
  onSelectCard,
}: Props) {
  const incomingMap = useMemo(() => {
    const map = new Map<number, number>();
    for (const l of links) {
      map.set(l.to, (map.get(l.to) || 0) + 1);
    }
    return map;
  }, [links]);

  const outgoingMap = useMemo(() => {
    const map = new Map<number, number>();
    for (const l of links) {
      map.set(l.from, (map.get(l.from) || 0) + 1);
    }
    return map;
  }, [links]);

  // Categorize cards
  const columns = useMemo(() => {
    const starters: LinkCard[] = [];
    const intermediates: LinkCard[] = [];
    const extra: LinkCard[] = [];
    const isolated: LinkCard[] = [];

    for (const c of cards) {
      if (filters.searchQuery) {
        const name = getCard(c.code)?.name.toLowerCase() ?? "";
        if (!name.includes(filters.searchQuery.toLowerCase())) continue;
      }

      if (c.extra) {
        extra.push(c);
      } else {
        const inc = incomingMap.get(c.code) || 0;
        const out = outgoingMap.get(c.code) || 0;
        if (out === 0 && inc === 0) {
          isolated.push(c);
        } else if (inc === 0 || c.reachAll >= 3) {
          starters.push(c);
        } else {
          intermediates.push(c);
        }
      }
    }

    // Sort columns
    starters.sort((a, b) => b.reachAll - a.reachAll || b.chance - a.chance);
    intermediates.sort((a, b) => (outgoingMap.get(b.code) || 0) - (outgoingMap.get(a.code) || 0));
    extra.sort((a, b) => (incomingMap.get(b.code) || 0) - (incomingMap.get(a.code) || 0));
    isolated.sort((a, b) => b.copies - a.copies);

    return [
      { id: "starters", title: "Starter & Motori", subtitle: "Avviano i collegamenti", list: starters, color: "var(--sec)" },
      { id: "bridges", title: "Ponti & Estensioni", subtitle: "Connettono la linea", list: intermediates, color: "var(--metric-resources)" },
      { id: "extra", title: "Extra Deck", subtitle: "Punti d'arrivo della board", list: extra, color: "#8b5cf6" },
      { id: "isolated", title: "Isolate & Handtrap", subtitle: "Nessun collegamento", list: isolated, color: "var(--muted)" },
    ];
  }, [cards, incomingMap, outgoingMap, filters.searchQuery, getCard]);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3 w-full h-full p-1 overflow-x-auto">
      {columns.map((col) => (
        <div key={col.id} className="flex flex-col gap-2 rounded-lg border border-border/70 bg-surface-1/60 p-2.5">
          <div className="flex items-baseline justify-between border-b border-border/50 pb-1.5 px-0.5">
            <div className="min-w-0">
              <h4 className="text-xs font-semibold text-text truncate">{col.title}</h4>
              <p className="text-[10px] text-faint truncate">{col.subtitle}</p>
            </div>
            <span className="chip !py-0 !px-1.5 text-[10px] font-bold tabular-nums">
              {col.list.length}
            </span>
          </div>

          <div className="flex flex-col gap-1.5 overflow-y-auto max-h-[520px] pr-1">
            {col.list.length === 0 ? (
              <p className="text-[11px] text-faint italic p-3 text-center">Nessuna carta in questa categoria.</p>
            ) : (
              col.list.map((c) => {
                const card = getCard(c.code);
                const name = card?.name ?? String(c.code);
                const isSelected = activeCardCode === c.code;

                return (
                  <div
                    key={c.code}
                    role="button"
                    tabIndex={0}
                    onClick={() => {
                      sound.play("page");
                      onSelectCard(isSelected ? null : c.code);
                    }}
                    className={`flex items-center gap-2 rounded-md border p-1.5 text-xs transition-colors cursor-pointer select-none ${
                      isSelected
                        ? "border-[var(--sec)] bg-sec/10 shadow-xs"
                        : "border-border/60 bg-surface-2 hover:border-border-strong hover:bg-surface-3"
                    }`}
                  >
                    <CardImage code={c.code} card={card} className="h-[44px] w-[30px] shrink-0 rounded-[2px]" />
                    <div className="flex flex-col min-w-0 flex-1 justify-center">
                      <div className="flex items-baseline justify-between gap-1">
                        <span className="font-medium text-text truncate" title={name}>
                          {name}
                        </span>
                        <span className="text-[10px] text-muted tabular-nums shrink-0">
                          ×{c.copies}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 mt-1 text-[10px] text-muted">
                        <span>
                          Raggiunge: <b className="text-text tabular-nums">{c.reachAll}</b>
                        </span>
                        {c.key > 0.05 && (
                          <span className="text-amber-500 font-semibold" title={`Chiave: -${c.key.toFixed(2)}`}>
                            ★ −{c.key.toFixed(1)}
                          </span>
                        )}
                        {!c.extra && c.chance > 0 && (
                          <span className="ml-auto text-sec tabular-nums font-medium">
                            {Math.round(c.chance * 100)}%
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
