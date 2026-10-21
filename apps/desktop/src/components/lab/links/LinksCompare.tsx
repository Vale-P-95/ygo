import { useMemo } from "react";
import type { Card, DeckSummary, LinksComparison } from "../../../lib/api";
import { sound } from "../../../lib/sound";
import { CardImage } from "../../combo/Field";

interface Props {
  decks: DeckSummary[];
  currentDeckId: number;
  compareDeckId: number | null;
  onSelectCompareDeckId: (id: number | null) => void;
  comparison: LinksComparison | null;
  loading: boolean;
  parentLoading?: boolean;
  error: string | null;
  showLostLinks: boolean;
  onToggleShowLostLinks: (show: boolean) => void;
  isReversed: boolean;
  onToggleReversed: () => void;
  getCard: (code: number) => Card | undefined;
  selectedCardCode: number | null;
  onSelectCard: (code: number | null) => void;
}

export function LinksCompare({
  decks,
  currentDeckId,
  compareDeckId,
  onSelectCompareDeckId,
  comparison,
  loading,
  parentLoading = false,
  error,
  showLostLinks,
  onToggleShowLostLinks,
  isReversed,
  onToggleReversed,
  getCard,
  selectedCardCode,
  onSelectCard,
}: Props) {
  const otherDecks = useMemo(
    () => decks.filter((d) => d.id !== currentDeckId),
    [decks, currentDeckId],
  );

  const deltaReached = comparison ? comparison.reachedAfter - comparison.reachedBefore : 0;

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border bg-surface-2/80 p-3.5 text-xs anim-fade shadow-sm">
      {/* Top controls: Deck selection & Mode */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border/70 pb-3">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-semibold text-text text-xs">
            {isReversed ? "Confronta con la versione successiva:" : "Confronta con la versione precedente:"}
          </span>
          <select
            value={compareDeckId ?? ""}
            onChange={(e) => {
              const val = e.target.value ? Number(e.target.value) : null;
              sound.play("select");
              onSelectCompareDeckId(val);
            }}
            className="input !py-1 !px-2 text-xs bg-surface-1 border-border rounded min-w-[200px]"
            disabled={loading || parentLoading}
          >
            <option value="">Seleziona un deck della Bottega…</option>
            {otherDecks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>

          {compareDeckId != null && (
            <button
              type="button"
              onClick={() => {
                sound.play("toggle");
                onToggleReversed();
              }}
              disabled={loading || parentLoading}
              className="chip !py-1 !px-2 text-xs flex items-center gap-1 text-muted hover:text-text"
              title="Inverti l'ordine del confronto (prima ⇄ dopo)"
            >
              <span>⇄ Inverti verso ({isReversed ? "dopo → prima" : "prima → dopo"})</span>
            </button>
          )}

          {compareDeckId != null && (
            <button
              type="button"
              onClick={() => {
                sound.play("close");
                onSelectCompareDeckId(null);
              }}
              className="btn btn-ghost !py-1 !px-2 text-xs text-muted hover:text-text"
              title="Esci dalla modalità confronto"
            >
              ✕ Chiudi confronto
            </button>
          )}
        </div>

        {comparison && (
          <label className="flex items-center gap-1.5 cursor-pointer text-[11px] text-muted hover:text-text select-none">
            <input
              type="checkbox"
              checked={showLostLinks}
              onChange={(e) => onToggleShowLostLinks(e.target.checked)}
              className="checkbox rounded border-border"
            />
            <span>Mostra collegamenti persi (rossi tratteggiati)</span>
          </label>
        )}
      </div>

      {/* Loading state */}
      {loading && (
        <div className="flex items-center justify-center p-6 gap-2 text-muted anim-fade">
          <span className="animate-spin text-sec">↻</span>
          <span>Calcolo delle differenze tra i due deck in corso…</span>
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <div className="rounded-md border border-danger/40 bg-danger/10 p-3 text-xs text-danger anim-fade">
          <p className="font-semibold">Errore durante il confronto</p>
          <p className="text-[11px] mt-0.5">{error}</p>
        </div>
      )}

      {/* Empty / unselected prompt */}
      {compareDeckId == null && !loading && (
        <div className="p-3 text-center text-muted text-[11px]">
          Scegli una seconda versione del deck per visualizzare collegamenti guadagnati, persi e impatto sulla raggiungibilità media.
        </div>
      )}

      {/* Comparison metrics dashboard */}
      {comparison && !loading && (
        <div className="flex flex-col gap-3.5 anim-fade">
          {/* Key comparison metrics */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
            {/* Average reached cards */}
            <div className="rounded-md border border-border/70 bg-surface-3/60 p-2.5 flex flex-col gap-1">
              <span className="text-[10px] uppercase font-semibold text-muted tracking-wider">
                Raggiungibilità media
              </span>
              <div className="flex items-baseline gap-2">
                <span className="text-base font-bold text-text tabular-nums">
                  {comparison.reachedBefore.toFixed(2)} → {comparison.reachedAfter.toFixed(2)}
                </span>
                <span
                  className={`chip !py-0 !px-1.5 text-[10px] font-semibold tabular-nums ${
                    deltaReached > 0.01
                      ? "text-ok bg-ok/15 border-ok/30"
                      : deltaReached < -0.01
                        ? "text-danger bg-danger/15 border-danger/30"
                        : "text-muted"
                  }`}
                >
                  {deltaReached >= 0 ? `+${deltaReached.toFixed(2)}` : deltaReached.toFixed(2)}
                </span>
              </div>
              <span className="text-[10px] text-faint">Carte raggiunte in media da una mano di 5</span>
            </div>

            {/* Added and removed cards */}
            <div className="rounded-md border border-border/70 bg-surface-3/60 p-2.5 flex flex-col gap-1">
              <span className="text-[10px] uppercase font-semibold text-muted tracking-wider">
                Differenze carte
              </span>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="chip !py-0.5 !px-2 text-xs font-semibold text-ok bg-ok/15 border-ok/30">
                  +{comparison.added.length} aggiunte
                </span>
                <span className="chip !py-0.5 !px-2 text-xs font-semibold text-danger bg-danger/15 border-danger/30">
                  -{comparison.removed.length} tolte
                </span>
              </div>
              <span className="text-[10px] text-faint">
                {comparison.added.length === 0 && comparison.removed.length === 0
                  ? "Stesse carte nelle due versioni"
                  : `${comparison.added.length + comparison.removed.length} carte variate nel mazzo`}
              </span>
            </div>

            {/* Gained and lost links */}
            <div className="rounded-md border border-border/70 bg-surface-3/60 p-2.5 flex flex-col gap-1">
              <span className="text-[10px] uppercase font-semibold text-muted tracking-wider">
                Collegamenti
              </span>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="chip !py-0.5 !px-2 text-xs font-semibold text-ok bg-ok/15 border-ok/30">
                  +{comparison.gained.length} guadagnati
                </span>
                <span className="chip !py-0.5 !px-2 text-xs font-semibold text-danger bg-danger/15 border-danger/30">
                  -{comparison.lost.length} persi
                </span>
              </div>
              <span className="text-[10px] text-faint">Frecce verdi (nuove) e rosse (tagliate)</span>
            </div>
          </div>

          {/* Added & Removed Card Chips (if any) */}
          {(comparison.added.length > 0 || comparison.removed.length > 0) && (
            <div className="flex flex-wrap gap-2 items-center text-xs">
              {comparison.added.map((code) => {
                const c = getCard(code);
                return (
                  <button
                    key={`added-${code}`}
                    type="button"
                    onClick={() => {
                      sound.play("page");
                      onSelectCard(selectedCardCode === code ? null : code);
                    }}
                    className={`chip !py-0.5 !px-2 text-[11px] font-medium flex items-center gap-1.5 border-ok/40 text-ok ${
                      selectedCardCode === code ? "chip-active ring-1 ring-ok" : ""
                    }`}
                    title="Carta aggiunta nella nuova versione"
                  >
                    <span>+</span>
                    <span>{c?.name ?? code}</span>
                  </button>
                );
              })}
              {comparison.removed.map((code) => {
                const c = getCard(code);
                return (
                  <button
                    key={`removed-${code}`}
                    type="button"
                    onClick={() => {
                      sound.play("page");
                      onSelectCard(selectedCardCode === code ? null : code);
                    }}
                    className={`chip !py-0.5 !px-2 text-[11px] font-medium flex items-center gap-1.5 border-danger/40 text-danger opacity-85 ${
                      selectedCardCode === code ? "chip-active ring-1 ring-danger" : ""
                    }`}
                    title="Carta tolta rispetto alla versione precedente"
                  >
                    <span>−</span>
                    <span>{c?.name ?? code}</span>
                  </button>
                );
              })}
            </div>
          )}

          {/* Changed cards table */}
          {comparison.changed.length > 0 && (
            <div className="flex flex-col gap-1.5">
              <div className="flex items-center justify-between">
                <span className="font-semibold text-text text-xs uppercase tracking-wider">
                  Carte che cambiano ({comparison.changed.length})
                </span>
                <span className="text-[10px] text-muted">Differenza di probabilità ≥ 2% o raggiungibilità modificata</span>
              </div>
              <div className="rounded-md border border-border/70 overflow-hidden">
                <table className="w-full text-[11px] text-left border-collapse">
                  <thead className="bg-surface-3/70 text-muted uppercase text-[9px] tracking-wider border-b border-border/70">
                    <tr>
                      <th className="py-1.5 px-2.5">Carta</th>
                      <th className="py-1.5 px-2 text-right">Mano Iniziale</th>
                      <th className="py-1.5 px-2 text-right">Carta Chiave</th>
                      <th className="py-1.5 px-2.5 text-right">Raggiunge</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/40">
                    {comparison.changed.map((ch) => {
                      const c = getCard(ch.code);
                      const isSelected = selectedCardCode === ch.code;
                      const chanceDelta = ch.chanceAfter - ch.chanceBefore;
                      const keyDelta = ch.keyAfter - ch.keyBefore;
                      const reachDelta = ch.reachAllAfter - ch.reachAllBefore;

                      return (
                        <tr
                          key={ch.code}
                          onClick={() => {
                            sound.play("page");
                            onSelectCard(isSelected ? null : ch.code);
                          }}
                          className={`cursor-pointer transition-colors hover:bg-surface-3/50 ${
                            isSelected ? "bg-sec/15 font-medium" : ""
                          }`}
                        >
                          <td className="py-1.5 px-2.5">
                            <div className="flex items-center gap-2 min-w-0">
                              <CardImage
                                code={ch.code}
                                card={c}
                                className="h-6 w-4.5 rounded-[2px] object-cover shrink-0 shadow-xs"
                              />
                              <span className="truncate max-w-[180px] sm:max-w-[260px] text-text font-medium" title={c?.name ?? String(ch.code)}>
                                {c?.name ?? ch.code}
                              </span>
                            </div>
                          </td>
                          <td className="py-1.5 px-2 text-right tabular-nums">
                            <span className="text-muted">{Math.round(ch.chanceBefore * 100)}%</span>
                            <span className="mx-1 text-faint">→</span>
                            <span
                              className={
                                chanceDelta > 0.01
                                  ? "text-ok font-medium"
                                  : chanceDelta < -0.01
                                    ? "text-danger font-medium"
                                    : "text-text"
                              }
                            >
                              {Math.round(ch.chanceAfter * 100)}%
                            </span>
                          </td>
                          <td className="py-1.5 px-2 text-right tabular-nums">
                            <span className="text-muted">{ch.keyBefore.toFixed(2)}</span>
                            <span className="mx-1 text-faint">→</span>
                            <span
                              className={
                                keyDelta > 0.01
                                  ? "text-ok font-medium"
                                  : keyDelta < -0.01
                                    ? "text-danger font-medium"
                                    : "text-text"
                              }
                            >
                              {ch.keyAfter.toFixed(2)}
                            </span>
                          </td>
                          <td className="py-1.5 px-2.5 text-right tabular-nums">
                            <span className="text-muted">{ch.reachAllBefore}</span>
                            <span className="mx-1 text-faint">→</span>
                            <span
                              className={
                                reachDelta > 0
                                  ? "text-ok font-medium"
                                  : reachDelta < 0
                                    ? "text-danger font-medium"
                                    : "text-text"
                              }
                            >
                              {ch.reachAllAfter}
                            </span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
