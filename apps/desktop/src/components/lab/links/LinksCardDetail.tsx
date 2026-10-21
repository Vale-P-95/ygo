import type { Card, CardLink, LinkCard } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { CardImage } from "../../combo/Field";
import { LINK_KINDS, LINK_ZONES } from "./types";

interface Props {
  card: LinkCard | null;
  getCard: (code: number) => Card | undefined;
  links: CardLink[];
  onSelectCard: (code: number) => void;
  onGoCombo?: (cardId: number) => void;
  onClose: () => void;
}

export function LinksCardDetail({
  card,
  getCard,
  links,
  onSelectCard,
  onGoCombo,
  onClose,
}: Props) {
  if (!card) {
    return (
      <div className="flex flex-col items-center justify-center p-6 text-center text-xs text-faint rounded-lg border border-dashed border-border/80 h-full min-h-[300px]">
        <span className="text-2xl mb-2" aria-hidden>🔍</span>
        <p className="font-medium text-text">Nessuna carta selezionata</p>
        <p className="text-[11px] text-muted mt-1 max-w-[200px]">
          Tocca o passa sopra una carta nel grafico per visualizzare le sue connessioni, la raggiungibilità e il valore come carta chiave.
        </p>
      </div>
    );
  }

  const cardData = getCard(card.code);
  const cardName = cardData?.name ?? String(card.code);

  const outgoing = links.filter((l) => l.from === card.code);
  const incoming = links.filter((l) => l.to === card.code);

  const chanceText = card.extra && card.chance === 0 ? "—" : `${Math.round(card.chance * 100)}%`;

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border bg-surface-2 p-3.5 text-xs anim-fade shadow-sm">
      <div className="flex items-start justify-between gap-2 border-b border-border/70 pb-3">
        <div className="flex gap-2.5 min-w-0">
          <CardImage code={card.code} card={cardData} className="h-[60px] w-[42px] shrink-0 rounded-[3px] shadow-sm" />
          <div className="flex flex-col min-w-0 justify-center">
            <h4 className="font-semibold text-text text-sm truncate leading-tight" title={cardName}>
              {cardName}
            </h4>
            <div className="flex items-center gap-1.5 mt-1 text-[11px] text-muted">
              <span className="chip !py-0 !px-1.5 text-[10px]">
                {card.extra ? "Extra Deck" : "Main Deck"}
              </span>
              <span>×{card.copies} {card.copies === 1 ? "copia" : "copie"}</span>
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="btn btn-ghost !p-1 text-faint hover:text-text shrink-0"
          title="Deseleziona carta"
        >
          ✕
        </button>
      </div>

      {/* Metric stats grid */}
      <div className="grid grid-cols-2 gap-2 text-center">
        <div className="rounded-md border border-border/60 bg-surface-1/80 p-2">
          <p className="text-[10px] text-muted uppercase tracking-wider font-medium">Raggiungibilità</p>
          <p className="text-base font-semibold tabular-nums text-text mt-0.5">
            {card.reach} <span className="text-xs text-muted font-normal">/ {card.reachAll}</span>
          </p>
          <p className="text-[9px] text-faint leading-tight mt-0.5">1 passo / tutti i passi</p>
        </div>

        <div className="rounded-md border border-border/60 bg-surface-1/80 p-2">
          <p className="text-[10px] text-muted uppercase tracking-wider font-medium">In mano iniziale</p>
          <p className="text-base font-semibold tabular-nums text-sec mt-0.5">{chanceText}</p>
          <p className="text-[9px] text-faint leading-tight mt-0.5">pescata o raggiunta</p>
        </div>
      </div>

      {/* Key card metric */}
      {card.key > 0.005 ? (
        <div className="rounded-md border border-amber-500/30 bg-amber-500/10 p-2.5">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[11px] font-semibold text-amber-500 flex items-center gap-1">
              <span>★</span> Carta chiave del deck
            </span>
            <span className="text-xs font-bold tabular-nums text-amber-500">
              −{card.key.toFixed(2)}
            </span>
          </div>
          <p className="text-[10px] text-muted mt-1 leading-snug">
            Se l'effetto di questa carta viene negato, perdi in media <b>{card.key.toFixed(2)}</b> carte raggiunte dalla mano iniziale.
          </p>
        </div>
      ) : (
        card.reach === 0 && incoming.length === 0 && (
          <div className="rounded-md border border-border/60 bg-surface-1/50 p-2 text-[11px] text-muted">
            <span>Carta isolata: nessuna carta del mazzo la raggiunge né la cerca.</span>
          </div>
        )
      )}

      {/* Outgoing links */}
      <div className="flex flex-col gap-1.5">
        <p className="text-[11px] font-semibold text-text uppercase tracking-wider flex items-center justify-between">
          <span>Porta a ({outgoing.length})</span>
          <span className="text-[10px] font-normal text-muted lowercase">in 1 passo</span>
        </p>
        {outgoing.length === 0 ? (
          <p className="text-[11px] text-faint italic">Non porta a nessun'altra carta direttamente.</p>
        ) : (
          <ul className="flex flex-col gap-1 max-h-[140px] overflow-y-auto pr-1">
            {outgoing.map((l, i) => {
              const toCard = getCard(l.to);
              const meta = LINK_KINDS[l.kind];
              const zoneMeta = LINK_ZONES[l.zone];
              return (
                <li
                  key={i}
                  className="flex items-center justify-between gap-1.5 rounded-md border border-border/50 bg-surface-1/70 px-2 py-1 text-xs hover:border-sec/50 cursor-pointer transition-colors"
                  onClick={() => {
                    sound.play("page");
                    onSelectCard(l.to);
                  }}
                  title={`Clicca per vedere i collegamenti di ${toCard?.name ?? l.to}`}
                >
                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                    <span className="truncate font-medium text-text">{toCard?.name ?? l.to}</span>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0 text-[10px]">
                    <span className="chip !py-0 !px-1 text-[9px]" style={{ color: meta.color, borderColor: meta.color }}>
                      {meta.verb}
                    </span>
                    <span className="text-faint">{zoneMeta?.short ?? l.zone}</span>
                    {l.used > 0 && (
                      <span className="text-sec font-medium tabular-nums" title={`Usato nel ${Math.round(l.used * 100)}% delle linee migliori`}>
                        {Math.round(l.used * 100)}%
                      </span>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {/* Incoming links */}
      <div className="flex flex-col gap-1.5">
        <p className="text-[11px] font-semibold text-text uppercase tracking-wider flex items-center justify-between">
          <span>Raggiungibile da ({incoming.length})</span>
        </p>
        {incoming.length === 0 ? (
          <p className="text-[11px] text-faint italic">Nessun effetto di questo mazzo porta a questa carta.</p>
        ) : (
          <ul className="flex flex-col gap-1 max-h-[140px] overflow-y-auto pr-1">
            {incoming.map((l, i) => {
              const fromCard = getCard(l.from);
              const meta = LINK_KINDS[l.kind];
              return (
                <li
                  key={i}
                  className="flex items-center justify-between gap-1.5 rounded-md border border-border/50 bg-surface-1/70 px-2 py-1 text-xs hover:border-sec/50 cursor-pointer transition-colors"
                  onClick={() => {
                    sound.play("page");
                    onSelectCard(l.from);
                  }}
                  title={`Clicca per vedere i collegamenti di ${fromCard?.name ?? l.from}`}
                >
                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                    <span className="truncate font-medium text-text">{fromCard?.name ?? l.from}</span>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0 text-[10px]">
                    <span className="chip !py-0 !px-1 text-[9px]" style={{ color: meta.color, borderColor: meta.color }}>
                      {meta.verb}
                    </span>
                    {l.used > 0 && (
                      <span className="text-sec font-medium tabular-nums">
                        {Math.round(l.used * 100)}%
                      </span>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {/* Combo lab button bridge */}
      {onGoCombo && (
        <button
          type="button"
          onClick={() => {
            sound.play("select");
            onGoCombo(card.code);
          }}
          className="btn btn-molten w-full mt-1 text-xs !py-1.5 flex items-center justify-center gap-1.5 shadow-sm"
        >
          <span>🎯 Cerca combo con questa carta</span>
        </button>
      )}
    </div>
  );
}
