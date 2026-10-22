import { useMemo } from "react";
import { ArrowRight, Info, KeyRound, Sparkles, Zap } from "lucide-react";
import { cardImage, type Card, type CardLink, type LinkCard } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { CardImage } from "../../combo/Field";
import { formatChance, isLinkTextOnly, KEY_CARD_THRESHOLD, LINK_KINDS, LINK_ZONES } from "./types";

interface Props {
  card: LinkCard | null;
  allCards?: LinkCard[];
  getCard: (code: number) => Card | undefined;
  links: CardLink[];
  onSelectCard: (code: number) => void;
  targetCode?: number | null;
  onTargetCodeChange?: (code: number | null) => void;
  shortestPath?: CardLink[] | null;
  onGoCombo?: (targetGoalCardId: number, startCardId?: number) => void;
  onClose: () => void;
}

export function LinksCardDetail({
  card,
  allCards = [],
  getCard,
  links,
  onSelectCard,
  targetCode = null,
  onTargetCodeChange,
  shortestPath = null,
  onGoCombo,
  onClose,
}: Props) {
  const cardData = card ? getCard(card.code) : undefined;
  const cardName = card ? (cardData?.name ?? String(card.code)) : "";

  const outgoing = useMemo(() => (card ? links.filter((l) => l.from === card.code) : []), [card, links]);
  const incoming = useMemo(() => (card ? links.filter((l) => l.to === card.code) : []), [card, links]);

  // Bottleneck key cards when no card is selected
  const keyCards = useMemo(() => {
    return allCards
      .filter((c) => c.key >= KEY_CARD_THRESHOLD)
      .sort((a, b) => b.key - a.key);
  }, [allCards]);

  // Rich tactical overview panel when no card is selected
  if (!card) {
    return (
      <div className="flex flex-col gap-3.5 rounded-xl border border-border/80 bg-surface-2/80 p-4 text-small anim-fade shadow-xs">
        {/* Header and concise guide */}
        <div className="flex items-start gap-2.5 border-b border-border/70 pb-3">
          <div className="rounded-lg bg-sec/15 p-2 text-sec shrink-0">
            <Info className="h-4 w-4" aria-hidden />
          </div>
          <div className="flex flex-col min-w-0">
            <h4 className="font-semibold text-text text-body leading-tight">Mappa dei Collegamenti</h4>
            <p className="text-small text-muted mt-1 leading-normal">
              Passa con il mouse o clicca su una carta nella mappa circolare per visualizzare le connessioni in entrata e uscita,
              la raggiungibilità e il cammino ottimale.
            </p>
          </div>
        </div>

        {/* Color legend */}
        <div className="flex flex-col gap-2 rounded-lg border border-border/60 bg-surface-3/50 p-2.5">
          <p className="eyebrow text-small text-muted flex items-center gap-1.5 font-semibold">
            <Sparkles className="h-3 w-3 text-sec" aria-hidden />
            <span>Legenda dei collegamenti</span>
          </p>
          <div className="grid grid-cols-2 gap-1.5 text-small">
            {(Object.keys(LINK_KINDS) as (keyof typeof LINK_KINDS)[]).map((kind) => {
              const meta = LINK_KINDS[kind];
              return (
                <div key={kind} className="flex items-center gap-1.5 min-w-0">
                  <span
                    className="h-2 w-2 rounded-full shrink-0"
                    style={{ background: meta.color }}
                    aria-hidden
                  />
                  <span className="truncate text-muted hover:text-text transition-colors">
                    {meta.label}
                  </span>
                </div>
              );
            })}
          </div>
          <div className="mt-1 pt-1.5 border-t border-border/40 flex items-center justify-between text-small text-faint">
            <span className="flex items-center gap-1">
              <span className="h-2 w-2 rounded-full bg-[var(--df-gold)]" /> Carta chiave (★)
            </span>
            <span className="flex items-center gap-1">
              <span className="h-2 w-2 rounded-full bg-[var(--df-ok)]" /> Nuova
            </span>
            <span className="flex items-center gap-1">
              <span className="h-2 w-2 rounded-full bg-[var(--df-danger)]" /> Rimossa
            </span>
          </div>
        </div>

        {/* Key cards ranking */}
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <p className="eyebrow text-small text-muted flex items-center gap-1 font-semibold">
              <KeyRound className="h-3 w-3 text-[var(--df-gold)]" aria-hidden />
              <span>Carte chiave del mazzo ({keyCards.length})</span>
            </p>
            <span className="text-small text-faint">impatto sulle combo</span>
          </div>

          {keyCards.length === 0 ? (
            <p className="text-small text-faint italic py-2">
              Nessun collo di bottiglia critico identificato dal simulatore.
            </p>
          ) : (
            <div className="flex flex-col gap-1.5 max-h-[260px] overflow-y-auto pr-1">
              {keyCards.map((c) => {
                const cData = getCard(c.code);
                const name = cData?.name ?? String(c.code);
                return (
                  <button
                    key={c.code}
                    type="button"
                    onClick={() => {
                      sound.play("page");
                      onSelectCard(c.code);
                    }}
                    className="flex items-center justify-between gap-2 rounded-lg border border-border/60 bg-surface-3/60 hover:bg-surface-3 hover:border-gold/50 p-2 text-left transition cursor-pointer group"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <img
                        src={cardImage(c.code, "cropped")}
                        alt=""
                        loading="lazy"
                        className="h-8 w-8 rounded-full object-cover shrink-0 border border-gold/40 group-hover:scale-105 transition-transform"
                      />
                      <div className="flex flex-col min-w-0">
                        <span className="font-medium text-text text-small truncate group-hover:text-sec transition-colors">
                          {name}
                        </span>
                        <span className="text-small text-muted">
                          {c.extra ? "Extra Deck" : "Main"} · ×{c.copies} · Raggiunge {c.reach}
                        </span>
                      </div>
                    </div>
                    <div className="flex flex-col items-end shrink-0">
                      <span className="chip !py-0 !px-1.5 text-caption font-bold text-[var(--df-gold)] border-[var(--df-gold)]/40 bg-[var(--df-gold)]/10">
                        −{c.key.toFixed(2)}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>
    );
  }

  // Consistent chance formatting via types.ts
  const chance = formatChance(card);

  return (
    <div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-surface-2/80 p-3.5 text-small anim-fade shadow-xs">
      {/* Card Header with prominent large image preview */}
      <div className="flex items-start justify-between gap-3 border-b border-border/70 pb-3">
        <div className="flex gap-3 min-w-0">
          <CardImage code={card.code} card={cardData} className="h-[120px] w-[84px] shrink-0 rounded-md shadow-md" />
          <div className="flex flex-col min-w-0 justify-center">
            <h4 className="font-semibold text-text text-body truncate leading-tight" data-tip={cardName}>
              {cardName}
            </h4>
            <div className="flex flex-wrap items-center gap-1.5 mt-2 text-small text-muted">
              <span className="chip !py-0.5 !px-2 text-caption">
                {card.extra ? "Extra Deck" : "Main Deck"}
              </span>
              <span>×{card.copies} {card.copies === 1 ? "copia" : "copie"}</span>
            </div>
            {cardData?.race && (
              <p className="text-small text-muted mt-1 truncate">
                {cardData.kind === "monster" ? `[${cardData.typeLine?.join(" / ") || cardData.humanType}]` : cardData.humanType}
              </p>
            )}
          </div>
        </div>
        <button
          type="button"
          onClick={() => {
            sound.play("close");
            onClose();
          }}
          className="text-muted hover:text-text p-1 text-sm leading-none rounded-md hover:bg-surface-3 transition shrink-0"
          aria-label="Deseleziona carta"
        >
          ✕
        </button>
      </div>

      {/* Metric stats grid */}
      <div className="grid grid-cols-2 gap-2 text-center">
        <div className="rounded-lg border border-border/60 bg-surface-3/60 p-2">
          <p className="eyebrow text-small text-muted">Raggiungibilità</p>
          <p className="text-body font-semibold tabular-nums text-text mt-0.5">
            {card.reach} <span className="text-small text-muted font-normal">/ {card.reachAll}</span>
          </p>
          <p className="text-small text-faint leading-tight mt-0.5">1 passo / tutti i passi</p>
        </div>

        <div
          className="rounded-lg border border-border/60 bg-surface-3/60 p-2 cursor-help"
          data-tip={chance.tooltip}
        >
          <p className="eyebrow text-small text-muted">In mano iniziale</p>
          <p className="text-body font-semibold tabular-nums text-sec mt-0.5">{chance.text}</p>
          <p className="text-small text-faint leading-tight mt-0.5">
            {card.extra && card.chance === 0 && card.measured
              ? "non raggiunta nelle mani cercate"
              : chance.isEstimated
              ? "stima cercata (±5%)"
              : "pescata o raggiunta in 1 passo"}
          </p>
        </div>
      </div>

      {/* Key card metric */}
      {card.key >= KEY_CARD_THRESHOLD ? (
        <div className="rounded-lg border border-[var(--df-gold)]/40 bg-[var(--df-gold)]/10 p-2.5">
          <div className="flex items-center justify-between gap-2">
            <span className="text-small font-semibold text-[var(--df-gold)] flex items-center gap-1">
              <span>★</span> Carta chiave del deck
            </span>
            <span className="text-small font-bold tabular-nums text-[var(--df-gold)]">
              −{card.key.toFixed(2)}
            </span>
          </div>
          <p className="text-small text-muted mt-1 leading-snug">
            Se l'effetto di questa carta viene negato, perdi in media <b>{card.key.toFixed(2)}</b> carte raggiunte dalla mano iniziale.
          </p>
        </div>
      ) : (
        outgoing.length === 0 && incoming.length === 0 && (
          <div className="rounded-lg border border-border/60 bg-surface-3/50 p-2 text-small text-muted">
            <span>Carta isolata: nessuna carta del mazzo la raggiunge né la cerca.</span>
          </div>
        )
      )}

      {/* Outgoing links */}
      <div className="flex flex-col gap-1.5">
        <p className="text-small font-semibold text-text uppercase tracking-wider flex items-center justify-between">
          <span>Porta a ({outgoing.length})</span>
          <span className="text-small font-normal text-muted lowercase">in 1 passo</span>
        </p>
        {outgoing.length === 0 ? (
          <p className="text-small text-faint italic">Non porta a nessun'altra carta direttamente.</p>
        ) : (
          <ul className="flex flex-col gap-1 max-h-[130px] overflow-y-auto pr-1">
            {outgoing.map((l, i) => {
              const toCard = getCard(l.to);
              const meta = LINK_KINDS[l.kind];
              const zoneMeta = LINK_ZONES[l.zone];
              return (
                <li key={`${l.from}-${l.to}-${l.kind}-${i}`}>
                  <button
                    type="button"
                    className="w-full flex items-center justify-between gap-1.5 rounded-lg border border-border/50 bg-surface-3/50 px-2.5 py-1 text-small hover:border-sec/50 cursor-pointer transition-colors text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold/70"
                    onClick={() => {
                      sound.play("page");
                      onSelectCard(l.to);
                    }}
                    data-tip={`Clicca per vedere i collegamenti di ${toCard?.name ?? l.to}`}
                  >
                    <div className="flex items-center gap-1.5 min-w-0">
                      <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                      <span className="truncate font-medium text-text">{toCard?.name ?? l.to}</span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0 text-small">
                      <span className="chip !py-0 !px-1.5 text-caption font-medium text-text border-border flex items-center gap-1">
                        <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                        <span>{meta.verb}</span>
                      </span>
                      <span className="text-faint">{zoneMeta?.short ?? l.zone}</span>
                      {isLinkTextOnly(l) && (
                        <span
                          className="chip !py-0 !px-1.5 text-caption text-muted border-dashed border-border"
                          data-tip="Nominato nel testo della carta, ma le linee combo cercate non lo usano"
                        >
                          dal testo
                        </span>
                      )}
                      {l.used > 0 && (
                        <span className="text-sec font-medium tabular-nums" data-tip={`Usato nel ${Math.round(l.used * 100)}% delle linee migliori`}>
                          {Math.round(l.used * 100)}%
                        </span>
                      )}
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {/* Incoming links */}
      <div className="flex flex-col gap-1.5">
        <p className="text-small font-semibold text-text uppercase tracking-wider flex items-center justify-between">
          <span>Raggiungibile da ({incoming.length})</span>
        </p>
        {incoming.length === 0 ? (
          <p className="text-small text-faint italic">Nessun effetto di questo mazzo porta a questa carta.</p>
        ) : (
          <ul className="flex flex-col gap-1 max-h-[130px] overflow-y-auto pr-1">
            {incoming.map((l, i) => {
              const fromCard = getCard(l.from);
              const meta = LINK_KINDS[l.kind];
              return (
                <li key={`${l.from}-${l.to}-${l.kind}-${i}`}>
                  <button
                    type="button"
                    className="w-full flex items-center justify-between gap-1.5 rounded-lg border border-border/50 bg-surface-3/50 px-2.5 py-1 text-small hover:border-sec/50 cursor-pointer transition-colors text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold/70"
                    onClick={() => {
                      sound.play("page");
                      onSelectCard(l.from);
                    }}
                    data-tip={`Clicca per vedere i collegamenti di ${fromCard?.name ?? l.from}`}
                  >
                    <div className="flex items-center gap-1.5 min-w-0">
                      <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                      <span className="truncate font-medium text-text">{fromCard?.name ?? l.from}</span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0 text-small">
                      <span className="chip !py-0 !px-1.5 text-caption font-medium text-text border-border flex items-center gap-1">
                        <span className="h-1.5 w-1.5 rounded-full shrink-0" style={{ background: meta.color }} />
                        <span>{meta.verb}</span>
                      </span>
                      {isLinkTextOnly(l) && (
                        <span
                          className="chip !py-0 !px-1.5 text-caption text-muted border-dashed border-border"
                          data-tip="Nominato nel testo della carta, ma le linee combo cercate non lo usano"
                        >
                          dal testo
                        </span>
                      )}
                      {l.used > 0 && (
                        <span className="text-sec font-medium tabular-nums">
                          {Math.round(l.used * 100)}%
                        </span>
                      )}
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {/* Shortest path between cards */}
      {allCards.length > 1 && (
        <div className="flex flex-col gap-1.5 rounded-lg border border-border/70 bg-surface-3/40 p-2.5">
          <p className="text-small font-semibold text-muted uppercase tracking-wider">
            Percorso verso un'altra carta
          </p>
          <select
            value={targetCode ?? ""}
            aria-label="Seleziona carta d'arrivo per il percorso"
            onChange={(e) => {
              const val = e.target.value ? Number(e.target.value) : null;
              onTargetCodeChange?.(val);
              if (val) sound.play("toggle");
            }}
            className="field !py-1 !px-2 !text-small w-full rounded-lg"
          >
            <option value="">Seleziona carta d'arrivo…</option>
            {allCards
              .filter((c) => c.code !== card.code)
              .map((c) => (
                <option key={c.code} value={c.code}>
                  {getCard(c.code)?.name ?? c.code} {c.extra ? "(Extra)" : ""}
                </option>
              ))}
          </select>

          {targetCode != null && (
            <div className="flex flex-col gap-1 mt-1 text-small">
              {shortestPath == null ? (
                <p className="text-faint italic text-small">Nessun percorso trovato tra queste due carte.</p>
              ) : shortestPath.length === 0 ? (
                <p className="text-faint italic text-small">Stessa carta selezionata.</p>
              ) : (
                <div className="flex flex-col gap-1.5">
                  <p className="text-small text-muted">
                    Raggiunta in <b>{shortestPath.length} {shortestPath.length === 1 ? "passo" : "passi"}</b>:
                  </p>
                  <div className="flex flex-wrap items-center gap-1">
                    {shortestPath.map((step, idx) => (
                      <span key={idx} className="inline-flex items-center gap-1">
                        {idx > 0 && <ArrowRight className="h-3 w-3 text-muted shrink-0" aria-hidden />}
                        <button
                          type="button"
                          onClick={() => {
                            sound.play("page");
                            onSelectCard(step.from);
                          }}
                          className="chip !py-0 !px-1.5 text-caption font-medium hover:border-sec/60 transition-colors cursor-pointer"
                          style={{ color: LINK_KINDS[step.kind].color }}
                          data-tip={`Ispeziona ${getCard(step.from)?.name ?? step.from} (${LINK_KINDS[step.kind].verb})`}
                        >
                          {getCard(step.from)?.name ?? step.from} ({LINK_KINDS[step.kind].verb})
                        </button>
                      </span>
                    ))}
                    <ArrowRight className="h-3 w-3 text-muted shrink-0" aria-hidden />
                    <button
                      type="button"
                      onClick={() => {
                        sound.play("page");
                        onSelectCard(targetCode);
                      }}
                      className="chip !py-0 !px-1.5 text-caption font-bold text-sec hover:border-sec/80 transition-colors cursor-pointer"
                      data-tip={`Ispeziona ${getCard(targetCode)?.name ?? targetCode}`}
                    >
                      {getCard(targetCode)?.name ?? targetCode}
                    </button>
                  </div>
                  {onGoCombo && (
                    <button
                      type="button"
                      onClick={() => {
                        onGoCombo(targetCode, card.code);
                      }}
                      className="btn btn-molten !py-1 !px-2.5 text-small mt-1 flex items-center justify-center gap-1.5"
                    >
                      <Zap className="h-3.5 w-3.5" aria-hidden />
                      <span>Esplora combo con obiettivo {getCard(targetCode)?.name ?? targetCode}</span>
                    </button>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Combo lab button bridge for this card */}
      {onGoCombo && (
        <button
          type="button"
          onClick={() => {
            onGoCombo(card.code);
          }}
          className="btn btn-ghost !py-1.5 text-small text-sec hover:bg-sec/10 flex items-center justify-center gap-1.5 border border-sec/40"
        >
          <Zap className="h-3.5 w-3.5" aria-hidden />
          <span>Cerca combo che arrivano a questa carta</span>
        </button>
      )}
    </div>
  );
}
