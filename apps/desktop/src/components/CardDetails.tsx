import type { ReactNode } from "react";
import type { Card } from "../lib/api";
import { rarityCode } from "./CardTile";

export const ATTRIBUTE_COLORS: Record<string, string> = {
  DARK: "#8b5cc8",
  LIGHT: "#e6c84f",
  EARTH: "#a07a43",
  WATER: "#3a8fd6",
  FIRE: "#e2532d",
  WIND: "#46b062",
  DIVINE: "#d9a441",
};

/** Accent of each card frame, used for the stripe above the name. */
export function frameColor(frame: string): string {
  if (frame.endsWith("_pendulum")) {
    const base = frame.split("_")[0];
    return `linear-gradient(90deg, var(--frame-${base}, #cf7a43), var(--frame-spell))`;
  }
  const base = frame.split("_")[0];
  return `var(--frame-${base}, var(--df-border-strong))`;
}


const MARKERS = ["Top-Left", "Top", "Top-Right", "Left", "", "Right", "Bottom-Left", "Bottom", "Bottom-Right"];

function LinkArrows({ markers }: { markers: string[] }) {
  return (
    <span className="inline-grid grid-cols-3 gap-[2px] align-middle" aria-label={`Frecce Link: ${markers.join(", ")}`}>
      {MARKERS.map((m, i) => (
        <span
          key={i}
          className="h-2 w-2 rounded-[2px]"
          style={{ background: m === "" ? "transparent" : markers.includes(m) ? "#ff5d3a" : "var(--df-surface-3)" }}
        />
      ))}
    </span>
  );
}

/** Effect text with "quoted card names" turned into search links. */
function EffectText({ text, onSearch }: { text: string; onSearch?: (query: string) => void }) {
  const parts = text.split(/("[^"\n]+")/g);
  return (
    <p className="whitespace-pre-line text-[13px] leading-relaxed text-text/90 select-text">
      {parts.map((part, i) =>
        part.startsWith('"') && onSearch ? (
          <button key={i} type="button" className="text-sec underline decoration-dotted underline-offset-2 hover:text-sec-2" onClick={() => onSearch(part)}>
            {part}
          </button>
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </p>
  );
}

function Stat({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="min-w-0 rounded-lg border border-border bg-bg-2 px-3 py-1.5">
      <p className="eyebrow !text-xs">{label}</p>
      <p className="truncate font-semibold tabular-nums" title={typeof value === "string" ? value : undefined}>
        {value}
      </p>
    </div>
  );
}

const BAN_LABEL: Record<string, string> = { Banned: "Proibita", Forbidden: "Proibita", Limited: "Limitata", "Semi-Limited": "Semi-limitata" };

export function CardDetails({ card, onSearch }: { card: Card; onSearch?: (query: string) => void }) {
  const isMonster = card.kind === "monster";
  const isXyz = card.typeLine.includes("Xyz");
  const rarity = rarityCode(card);
  return (
    <div className="flex flex-col gap-4">
      <div>
        <div className="mb-2 h-1 w-16 rounded-full" style={{ background: frameColor(card.frame) }} />
        <h2 className="display text-xl leading-tight font-bold select-text">{card.name}</h2>
        <p className="mt-1 text-sm text-muted">{isMonster ? `[${card.typeLine.join(" / ")}]` : card.humanType}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2 text-xs">
        {card.attribute && (
          <span className="chip" style={{ borderColor: ATTRIBUTE_COLORS[card.attribute], color: ATTRIBUTE_COLORS[card.attribute] }}>
            ● {card.attribute}
          </span>
        )}
        {card.level != null && (
          <span className="chip" title={isXyz ? "Rango" : "Livello"}>
            <span style={{ color: isXyz ? "#c9c9c9" : "#f2a33a" }}>★</span> {isXyz ? "Rango" : "Livello"} {card.level}
          </span>
        )}
        {card.linkRating != null && (
          <span className="chip">
            LINK-{card.linkRating} <LinkArrows markers={card.linkMarkers} />
          </span>
        )}
        {card.scale != null && <span className="chip">Scala {card.scale}</span>}
        {card.archetype && (
          <button type="button" className="chip chip-active hover:brightness-125" onClick={() => onSearch?.(`arch:"${card.archetype}"`)}>
            ⚑ {card.archetype}
          </button>
        )}
        {rarity && (
          <span className="chip">
            <span className="rarity-gem !static" data-rarity={rarity}>
              {rarity.toUpperCase()}
            </span>
            Master Duel
          </span>
        )}
      </div>

      {isMonster && (
        <div className="grid grid-cols-3 gap-2">
          <Stat label="ATK" value={card.atk ?? "?"} />
          {card.linkRating == null ? <Stat label="DEF" value={card.def ?? "?"} /> : <Stat label="Link" value={card.linkRating} />}
          <Stat label="Tipo" value={card.race} />
        </div>
      )}

      {card.pendDesc && (
        <div className="rounded-lg border border-border bg-bg-2 p-3">
          <p className="eyebrow mb-1">Effetto Pendulum</p>
          <EffectText text={card.pendDesc} onSearch={onSearch} />
        </div>
      )}
      <div className="rounded-lg border border-border bg-bg-2 p-3">
        <p className="eyebrow mb-1">{card.pendDesc ? "Effetto del mostro" : "Testo"}</p>
        <EffectText text={card.desc} onSearch={onSearch} />
      </div>

      <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-xs">
        <dt className="text-faint">Formati</dt>
        <dd className="flex flex-wrap gap-1">
          {card.formats.length ? card.formats.map((f) => <span key={f} className="text-muted">{f}</span>).reduce<ReactNode[]>((acc, el, i) => (i ? [...acc, " · ", el] : [el]), []) : "—"}
        </dd>
        <dt className="text-faint">Banlist MD</dt>
        <dd className={card.banMd ? "text-warn" : "text-muted"}>{card.banMd ? BAN_LABEL[card.banMd] ?? card.banMd : card.formats.includes("Master Duel") ? "Senza limiti" : "Non in Master Duel"}</dd>
        <dt className="text-faint">Banlist TCG</dt>
        <dd className={card.banTcg ? "text-warn" : "text-muted"}>{card.banTcg ? BAN_LABEL[card.banTcg] ?? card.banTcg : "Senza limiti"}</dd>
        <dt className="text-faint">Banlist OCG</dt>
        <dd className={card.banOcg ? "text-warn" : "text-muted"}>{card.banOcg ? BAN_LABEL[card.banOcg] ?? card.banOcg : "Senza limiti"}</dd>
        <dt className="text-faint">Uscita</dt>
        <dd className="text-muted">
          {card.tcgDate ? `TCG ${card.tcgDate}` : "TCG —"} · {card.ocgDate ? `OCG ${card.ocgDate}` : "OCG —"}
        </dd>
        <dt className="text-faint">Passcode</dt>
        <dd className="font-mono text-muted select-text">{card.id}</dd>
      </dl>
    </div>
  );
}
