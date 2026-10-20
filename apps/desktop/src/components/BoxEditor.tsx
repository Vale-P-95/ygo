import { useEffect, useState } from "react";
import { api, cardImage, type DeckBox, type ShelfDeck } from "../lib/api";
import { MATERIALS, PALETTE, resolveLook, SHAPES } from "../lib/boxes";
import { useCardCache } from "../lib/cards";
import { FORMAT_LABELS } from "../lib/deck";
import { sound } from "../lib/sound";
import { DeckBox3D } from "./DeckBox3D";

interface Props {
  deck: ShelfDeck;
  onBox: (box: DeckBox) => void;
  onRename: (name: string) => void;
  onOpen: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onClose: () => void;
}

/** Side panel to customise a deck box: color, material, shape and showcase cards. */
export function BoxEditor({ deck, onBox, onRename, onOpen, onDuplicate, onDelete, onClose }: Props) {
  const [cards, setCards] = useState<number[]>([]);
  const [confirm, setConfirm] = useState(false);
  const cache = useCardCache();
  const { request } = cache;
  const look = resolveLook(deck.box, deck.id);
  const box: DeckBox = { color: look.color, material: look.material, shape: look.shape, showcase: deck.box.showcase };

  useEffect(() => {
    let cancelled = false;
    api.getDeck(deck.id).then((d) => {
      if (cancelled || !d) return;
      const ids = [...new Set([...d.main, ...d.extra, ...d.side])];
      setCards(ids);
      request(ids);
    });
    return () => {
      cancelled = true;
    };
  }, [deck.id, request]);

  useEffect(() => {
    if (!confirm) return;
    const t = window.setTimeout(() => setConfirm(false), 3000);
    return () => window.clearTimeout(t);
  }, [confirm]);

  const toggleShowcase = (id: number) => {
    const current = deck.box.showcase;
    const next = current.includes(id) ? current.filter((x) => x !== id) : [...current, id].slice(-3);
    sound.play(current.includes(id) ? "toggle" : "select");
    onBox({ ...box, showcase: next });
  };

  const label = "eyebrow mb-2";
  return (
    <aside className="flex w-[348px] xl:w-[380px] shrink-0 flex-col border-l border-border bg-surface/85 backdrop-blur anim-fade">
      <div className="flex items-center justify-between px-4 pt-3 xl:px-5 xl:pt-4">
        <p className="display text-sm font-bold tracking-[0.12em] uppercase">Personalizza</p>
        <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={onClose} aria-label="Chiudi">
          ✕
        </button>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-3.5 xl:gap-5 overflow-y-auto px-4 pt-2.5 pb-4 xl:px-5 xl:pt-3 xl:pb-5">
        <div className="grid shrink-0 place-items-center rounded-xl border border-border bg-bg-2/80 pt-1.5">
          <DeckBox3D
            look={look}
            name={deck.name}
            format={deck.format}
            legal={deck.legal}
            errors={deck.errors}
            cards={deck.box.showcase.length ? deck.box.showcase : deck.preview}
            scale={1.08}
            selected
            sway
          />
        </div>

        <div className="flex flex-col gap-1">
          <input
            className="display w-full rounded border-b border-transparent bg-transparent text-lg font-bold outline-none hover:border-border focus:border-sec"
            value={deck.name}
            onChange={(e) => onRename(e.target.value)}
            spellCheck={false}
            aria-label="Nome del deck"
          />
          <p className="text-xs text-muted">
            {FORMAT_LABELS[deck.format]} · {deck.mainCount}/{deck.extraCount}/{deck.sideCount} ·{" "}
            {deck.legal == null ? (
              "legalità non verificata"
            ) : deck.legal ? (
              <span className="text-ok">✓ legale</span>
            ) : (
              <span className="text-danger">
                ✕ {deck.errors} {deck.errors === 1 ? "problema" : "problemi"}
              </span>
            )}
          </p>
        </div>

        <section>
          <p className={label}>Colore</p>
          <div className="grid grid-cols-7 gap-2">
            {PALETTE.map((p) => (
              <button
                key={p.color}
                type="button"
                title={p.name}
                aria-label={p.name}
                aria-pressed={look.color === p.color}
                onClick={() => {
                  sound.play("toggle");
                  onBox({ ...box, color: p.color });
                }}
                className="h-8 rounded-lg border-2 transition-transform hover:scale-110"
                style={{ background: p.color, borderColor: look.color === p.color ? "var(--sec)" : "var(--df-border-strong)" }}
              />
            ))}
            <label
              className="relative grid h-8 cursor-pointer place-items-center overflow-hidden rounded-lg border-2 border-dashed border-border text-xs text-muted hover:text-text"
              title="Colore personalizzato"
            >
              ＋
              <input type="color" className="absolute inset-0 cursor-pointer opacity-0" value={look.color} onChange={(e) => onBox({ ...box, color: e.target.value })} />
            </label>
          </div>
        </section>

        <section>
          <p className={label}>Materiale</p>
          <div className="flex flex-wrap gap-1.5">
            {MATERIALS.map((m) => (
              <button
                key={m.id}
                type="button"
                aria-pressed={look.material === m.id}
                onClick={() => {
                  sound.play("latch");
                  onBox({ ...box, material: m.id });
                }}
                className={`chip !px-3 !py-1 ${look.material === m.id ? "chip-active" : ""}`}
              >
                {m.name}
              </button>
            ))}
          </div>
        </section>

        <section>
          <p className={label}>Forma</p>
          <div className="grid grid-cols-3 gap-2">
            {SHAPES.map((s) => (
              <button
                key={s.id}
                type="button"
                aria-pressed={look.shape === s.id}
                title={s.hint}
                onClick={() => {
                  sound.play("knock");
                  onBox({ ...box, shape: s.id });
                }}
                className={`flex flex-col items-center gap-1.5 rounded-xl border px-2 py-2.5 text-xs ${look.shape === s.id ? "border-sec bg-sec/10 text-text" : "border-border text-muted hover:text-text"}`}
              >
                <span
                  className="block rounded-[3px] border-2 border-current"
                  style={{ width: s.id === "box" ? 18 : 28, height: s.id === "box" ? 24 : s.id === "tin" ? 18 : 20, borderRadius: s.id === "tin" ? 6 : 3 }}
                />
                {s.name}
              </button>
            ))}
          </div>
        </section>

        <section>
          <div className="mb-2 flex items-baseline justify-between">
            <p className="eyebrow">Carte vetrina</p>
            <span className="text-[11px] text-faint">{deck.box.showcase.length}/3 · senza scelta: le prime del Main</span>
          </div>
          <div className="grid grid-cols-6 gap-1.5">
            {cards.map((id) => {
              const order = deck.box.showcase.indexOf(id);
              return (
                <button
                  key={id}
                  type="button"
                  title={cache.get(id)?.name}
                  onClick={() => toggleShowcase(id)}
                  className={`relative overflow-hidden rounded-[4px] border-2 transition-transform hover:scale-105 ${order >= 0 ? "border-sec" : "border-transparent opacity-80 hover:opacity-100"}`}
                  style={{ aspectRatio: "421 / 614" }}
                >
                  <img src={cardImage(id, "small")} alt="" loading="lazy" className="h-full w-full object-cover" />
                  {order >= 0 && <span className="tile-badge !top-0.5 !left-0.5">{order + 1}</span>}
                </button>
              );
            })}
          </div>
        </section>
      </div>
      <div className="flex gap-2 border-t border-border px-5 py-3">
        <button type="button" className="btn btn-molten flex-1" onClick={onOpen}>
          Apri nella Fucina
        </button>
        <button type="button" className="btn btn-ghost" onClick={onDuplicate}>
          Duplica
        </button>
        <button
          type="button"
          className={`btn ${confirm ? "bg-danger text-white" : "btn-ghost"}`}
          onClick={() => {
            if (confirm) onDelete();
            else {
              sound.play("toggle");
              setConfirm(true);
            }
          }}
        >
          {confirm ? "Conferma" : "Elimina"}
        </button>
      </div>
    </aside>
  );
}
