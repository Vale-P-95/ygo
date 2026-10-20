import { useEffect, useRef, useState } from "react";
import { api, type Card, type Deck, type MatchEntry, type MatchLog } from "../../lib/api";
import { OUTCOME, resultOf } from "../../lib/matches";
import { sound } from "../../lib/sound";
import { toast } from "../../lib/toast";

interface Props {
  deck: Deck;
  log: MatchLog | null;
  getCard: (id: number) => Card | undefined;
  onLogged: () => void;
}

type Bo = "bo1" | "bo3";
const BO_KEY = "duelforge.matchFormat";

function storedBo(): Bo {
  try {
    return localStorage.getItem(BO_KEY) === "bo3" ? "bo3" : "bo1";
  } catch {
    return "bo1";
  }
}

/** The local date and time as a datetime-local input wants it: "2026-10-19T08:30". */
function localNow() {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
}

/** Side panel of the Registro partite: logs a match in a few clicks. */
export function MatchEntryPanel({ deck, log, getCard, onLogged }: Props) {
  const [bo, setBo] = useState<Bo>(storedBo);
  const [opponent, setOpponent] = useState("");
  const [first, setFirst] = useState<boolean | null>(null);
  const [games, setGames] = useState<string[]>([]);
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState("");
  const [notes, setNotes] = useState("");
  const [when, setWhen] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const opponentRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    try {
      localStorage.setItem(BO_KEY, bo);
    } catch {
      /* the format is only a convenience */
    }
  }, [bo]);

  const result = resultOf(games.join(""));
  // Bo3 ends when someone has two games; the third game opens on 1–1.
  const wins = games.filter((g) => g === "W").length;
  const losses = games.filter((g) => g === "L").length;
  const shown = bo === "bo1" ? 1 : Math.min(3, Math.max(2, games.length + (wins < 2 && losses < 2 ? 1 : 0)));
  const ready = games.length > 0 && !busy;
  const version = log?.current;

  const setGame = (i: number, g: string) => {
    sound.play(g === "W" ? "chime" : g === "L" ? "knock" : "toggle");
    setGames((cur) => {
      const next = cur.slice(0, i);
      if (cur[i] !== g) next[i] = g;
      return next;
    });
  };

  const addTag = (t: string) => {
    const tag = t.trim();
    if (!tag || tags.some((x) => x.toLowerCase() === tag.toLowerCase())) return;
    sound.play("select");
    setTags([...tags, tag]);
  };

  const submit = async () => {
    if (!ready || deck.id == null) return;
    setBusy(true);
    const entry: MatchEntry = { deckId: deck.id, opponent, first, games: games.join(""), tags, notes, playedAt: (when ?? "").replace("T", " ") };
    try {
      await api.logMatch(entry);
      sound.play("bubble");
      toast(`Partita registrata: ${OUTCOME[result]?.long.toLowerCase() ?? "pareggio"}${opponent.trim() ? ` contro ${opponent.trim()}` : ""}`, "ok");
      setOpponent("");
      setFirst(null);
      setGames([]);
      setNotes("");
      setTags([]);
      setWhen(null);
      onLogged();
      opponentRef.current?.focus();
    } catch (e) {
      sound.play("error");
      toast(`Partita non registrata: ${e}`, "error");
    } finally {
      setBusy(false);
    }
  };

  // Ctrl+Enter logs the match from anywhere in the tab.
  const submitRef = useRef(submit);
  useEffect(() => {
    submitRef.current = submit;
  });
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        void submitRef.current();
      }
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  }, []);

  const seg = (on: boolean) => `flex-1 rounded-lg px-2 py-1.5 text-xs font-semibold transition-colors ${on ? "bg-sec text-bg" : "text-muted hover:text-text"}`;
  const quickOpponents = (log?.opponents ?? []).slice(0, 8);
  const quickTags = [...new Set([...(log?.tags ?? []), "ranked", "evento", "brick", "side"])].filter((t) => !tags.includes(t)).slice(0, 8);

  return (
    <aside className="flex min-h-0 flex-col border-l border-border bg-surface/80 backdrop-blur">
      <form
        className="flex min-h-0 flex-1 flex-col gap-2.5 xl:gap-4 overflow-y-auto px-3.5 pt-3 pb-0 xl:px-4 xl:pt-4"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <div className="flex items-center justify-between gap-2">
          <p className="display text-sm font-bold tracking-[0.12em] uppercase">Nuova partita</p>
          {version && (
            <span
              className="chip !px-2 !py-0.5 !text-[11px]"
              title={log?.currentIsNew ? "La lista salvata ora non ha ancora partite: le prossime misurano questa versione." : "Le partite vanno alla versione salvata ora."}
            >
              {version.name}
              {log?.currentIsNew && <b className="text-sec">nuova</b>}
            </span>
          )}
        </div>
        {log?.currentIsNew && log.current?.changes && log.current.changes.length > 0 && (
          <p className="-mt-2 text-[11px] leading-snug text-faint">
            Rispetto alla {log.versions[log.versions.length - 1]?.name}:{" "}
            {log.current.changes
              .slice(0, 6)
              .map((c) => `${c.delta > 0 ? "+" : "−"}${Math.abs(c.delta)} ${getCard(c.id)?.name ?? c.id}${c.zone === "main" ? "" : c.zone === "side" ? " (side)" : " (extra)"}`)
              .join(", ")}
            {log.current.changes.length > 6 && "…"}
          </p>
        )}

        <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1" role="group" aria-label="Formato">
          {(["bo1", "bo3"] as const).map((b) => (
            <button
              key={b}
              type="button"
              className={seg(bo === b)}
              aria-pressed={bo === b}
              onClick={() => {
                sound.play("toggle");
                setBo(b);
                setGames((g) => (b === "bo1" ? g.slice(0, 1) : g));
              }}
            >
              {b === "bo1" ? "Partita singola" : "Al meglio di 3"}
            </button>
          ))}
        </div>

        <label className="flex flex-col gap-1.5">
          <span className="eyebrow">Avversario</span>
          <input
            ref={opponentRef}
            className="field !py-1.5"
            list="df-opponents"
            placeholder="Snake-Eye, Tenpai, Yubel…"
            value={opponent}
            onChange={(e) => setOpponent(e.target.value)}
            maxLength={40}
          />
          <datalist id="df-opponents">
            {log?.opponents.map((o) => (
              <option key={o} value={o} />
            ))}
          </datalist>
          {quickOpponents.length > 0 && (
            <span className="flex flex-wrap gap-1">
              {quickOpponents.map((o) => (
                <button
                  key={o}
                  type="button"
                  className={`chip !px-2 !py-0.5 !text-[11px] ${opponent === o ? "chip-active" : ""}`}
                  onClick={() => {
                    sound.play("select");
                    setOpponent(opponent === o ? "" : o);
                  }}
                >
                  {o}
                </button>
              ))}
            </span>
          )}
        </label>

        <div className="flex flex-col gap-1.5">
          <span className="eyebrow">{bo === "bo3" ? "Turno nel game 1" : "Turno"}</span>
          <div className="flex gap-1 rounded-xl border border-border bg-bg-2 p-1" role="group" aria-label="Turno">
            {([true, false] as const).map((f) => (
              <button
                key={String(f)}
                type="button"
                className={seg(first === f)}
                aria-pressed={first === f}
                onClick={() => {
                  sound.play("toggle");
                  setFirst(first === f ? null : f);
                }}
              >
                {f ? "Per primo" : "Per secondo"}
              </button>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <span className="eyebrow">Risultato</span>
          {Array.from({ length: shown }, (_, i) => (
            <div key={i} className="flex items-center gap-1.5">
              {bo === "bo3" && <span className="w-14 shrink-0 text-xs text-muted">Game {i + 1}</span>}
              {(["W", "L", "D"] as const).map((g) => {
                const on = games[i] === g;
                const o = OUTCOME[g];
                return (
                  <button
                    key={g}
                    type="button"
                    disabled={i > games.length}
                    aria-pressed={on}
                    onClick={() => setGame(i, g)}
                    className={`rounded-lg border px-2 text-xs font-semibold transition-all disabled:opacity-35 ${g === "D" ? "py-1.5" : "flex-1 py-1.5"} ${bo === "bo1" && g !== "D" ? "!py-1.5 xl:!py-2.5 text-sm" : ""}`}
                    style={on ? { background: o.tone, borderColor: o.tone, color: "#fff" } : { borderColor: "var(--df-border)", color: "var(--df-muted)" }}
                    title={o.long}
                  >
                    {g === "D" ? o.short : o.long}
                  </button>
                );
              })}
            </div>
          ))}
          {bo === "bo3" && games.length > 0 && (
            <p className="text-xs text-muted">
              {wins < 2 && losses < 2 && games.length < 3 ? (
                <>
                  Match in corso {wins}–{losses}: registrandolo ora conta come {OUTCOME[result]?.long.toLowerCase()}.
                </>
              ) : (
                <>
                  Match: <b style={{ color: OUTCOME[result]?.tone }}>{OUTCOME[result]?.long}</b> {wins}–{losses}
                </>
              )}
            </p>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <span className="eyebrow">Tag</span>
          <span className="flex flex-wrap gap-1">
            {tags.map((t) => (
              <button key={t} type="button" className="chip chip-active !px-2 !py-0.5 !text-[11px]" onClick={() => setTags(tags.filter((x) => x !== t))} title="Togli il tag">
                {t} ✕
              </button>
            ))}
            {quickTags.map((t) => (
              <button key={t} type="button" className="chip !px-2 !py-0.5 !text-[11px] text-muted" onClick={() => addTag(t)}>
                + {t}
              </button>
            ))}
          </span>
          <input
            className="field !py-1 text-xs"
            placeholder="Nuovo tag e Invio"
            value={tagInput}
            maxLength={24}
            onChange={(e) => setTagInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.ctrlKey && !e.metaKey) {
                e.preventDefault();
                addTag(tagInput);
                setTagInput("");
              }
            }}
          />
        </div>

        <label className="flex flex-col gap-1.5">
          <span className="eyebrow">Note</span>
          <textarea className="field !py-1.5 text-xs" rows={2} placeholder="Cosa è successo: brick, handtrap decisive…" value={notes} onChange={(e) => setNotes(e.target.value)} maxLength={500} />
        </label>

        <div className="flex flex-col gap-1.5">
          <span className="eyebrow">Quando</span>
          {when == null ? (
            <span className="flex items-center gap-2 text-xs text-muted">
              Adesso
              <button type="button" className="text-sec hover:underline" onClick={() => setWhen(localNow())}>
                cambia
              </button>
            </span>
          ) : (
            <span className="flex items-center gap-2">
              <input type="datetime-local" className="field !py-1 text-xs" value={when} max={localNow()} onChange={(e) => setWhen(e.target.value)} aria-label="Data e ora della partita" />
              <button type="button" className="shrink-0 text-xs text-sec hover:underline" onClick={() => setWhen(null)}>
                adesso
              </button>
            </span>
          )}
        </div>

        <div className="sticky bottom-0 -mx-3.5 xl:-mx-4 mt-auto flex flex-col gap-1 border-t border-border bg-surface/95 px-3.5 pt-2.5 pb-3 xl:px-4 xl:pt-3 xl:pb-3.5 backdrop-blur">
          <button type="submit" className="btn btn-molten w-full justify-center" disabled={!ready}>
            {busy ? "Registro…" : "Registra partita"}
          </button>
          <p className="text-center text-[11px] text-faint">Ctrl+Invio per registrare</p>
        </div>
      </form>
    </aside>
  );
}
