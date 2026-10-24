import { useEffect, useRef, useState } from "react";
import { api, type Card, type Deck, type Imported } from "../lib/api";
import { renderDeckImage, saveDeckImage } from "../lib/deckImage";
import { sound } from "../lib/sound";
import { toast } from "../lib/toast";
import { Modal } from "./Modal";

async function copy(text: string, el?: HTMLTextAreaElement | HTMLInputElement | null) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    el?.select();
    document.execCommand("copy");
  }
  sound.play("chime");
}

export function ImportDialog({ onClose, onImported }: { onClose: () => void; onImported: (result: Imported, name: string) => void }) {
  const [text, setText] = useState("");
  const [name, setName] = useState("Deck importato");
  const [error, setError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const readFile = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      setText(String(reader.result ?? ""));
      setName(file.name.replace(/\.ydk$/i, ""));
    };
    reader.readAsText(file);
  };

  const run = async () => {
    try {
      const result = await api.importDeck(text);
      // Only unknown passcodes: importing would leave an empty deck.
      if (result.main.length + result.extra.length + result.side.length === 0) {
        sound.play("error");
        setError(`Nessuna carta riconosciuta: ${result.unknown.length} passcode sconosciuti. Aggiorna l'elenco delle carte nelle Impostazioni o controlla il testo.`);
        return;
      }
      onImported(result, name);
    } catch (e) {
      sound.play("error");
      setError(String(e));
    }
  };

  return (
    <Modal title="Importa un deck" onClose={onClose}>
      <p className="text-sm text-muted">Incolla il contenuto di un file .ydk (EDOPro, YGOPRODeck, Duelingbook…) o un link ydke://, oppure scegli il file.</p>
      <div className="flex gap-2">
        <input className="field" value={name} onChange={(e) => setName(e.target.value)} aria-label="Nome del deck" />
        <button type="button" className="btn btn-ghost shrink-0" onClick={() => fileRef.current?.click()}>
          Scegli file .ydk
        </button>
        <input ref={fileRef} type="file" accept=".ydk,text/plain" className="hidden" onChange={(e) => e.target.files?.[0] && readFile(e.target.files[0])} />
      </div>
      <textarea
        className="field h-56 resize-none font-mono text-xs"
        placeholder={"#main\n86120751\n…\n#extra\n…\n!side\n…"}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          setError(null);
        }}
        spellCheck={false}
      />
      {error && <p className="text-sm text-danger">{error}</p>}
      <div className="flex justify-end gap-2">
        <button type="button" className="btn btn-ghost" onClick={onClose}>
          Annulla
        </button>
        <button type="button" className="btn btn-molten" disabled={!text.trim()} onClick={run}>
          Importa
        </button>
      </div>
    </Modal>
  );
}

export function ExportDialog({ deck, onClose }: { deck: Deck; onClose: () => void }) {
  const [out, setOut] = useState<{ ydk: string; ydke: string } | null>(null);
  const ydkRef = useRef<HTMLTextAreaElement>(null);
  const ydkeRef = useRef<HTMLInputElement>(null);
  const key = JSON.stringify(deck);
  useEffect(() => {
    api.exportDeck(JSON.parse(key) as Deck).then(setOut);
  }, [key]);
  return (
    <Modal title={`Esporta «${deck.name}»`} onClose={onClose}>
      <div className="flex flex-col gap-2">
        <p className="eyebrow">Link ydke:// (EDOPro, Duelingbook, siti di deck)</p>
        <div className="flex gap-2">
          <input ref={ydkeRef} className="field font-mono text-xs" readOnly value={out?.ydke ?? ""} />
          <button type="button" className="btn btn-molten shrink-0" onClick={() => out && copy(out.ydke, ydkeRef.current)}>
            Copia
          </button>
        </div>
      </div>
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <p className="eyebrow">File .ydk</p>
          <button type="button" className="btn btn-ghost !py-1 text-xs" onClick={() => out && copy(out.ydk, ydkRef.current)}>
            Copia testo
          </button>
        </div>
        <textarea ref={ydkRef} className="field h-48 resize-none font-mono text-xs" readOnly value={out?.ydk ?? ""} />
      </div>
    </Modal>
  );
}

export function ImageDialog({ deck, getCard, legal, onClose }: { deck: Deck; getCard: (id: number) => Card | undefined; legal: boolean | null; onClose: () => void }) {
  const [side, setSide] = useState(true);
  const [url, setUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(true);
  const blob = useRef<Blob | null>(null);
  const key = JSON.stringify([deck.name, deck.format, deck.main, deck.extra, deck.side, side, legal]);

  useEffect(() => {
    let cancelled = false;
    let made: string | null = null;
    setBusy(true);
    renderDeckImage(deck, getCard, { side, legal })
      .then((b) => {
        if (cancelled) return;
        blob.current = b;
        made = URL.createObjectURL(b);
        setUrl(made);
      })
      .catch((e) => toast(`Immagine non creata: ${e}`, "error"))
      .finally(() => !cancelled && setBusy(false));
    return () => {
      cancelled = true;
      if (made) URL.revokeObjectURL(made);
    };
    // The key covers what the picture shows.
  }, [key]); // eslint-disable-line react-hooks/exhaustive-deps

  const save = async () => {
    if (!blob.current) return;
    try {
      const path = await saveDeckImage(blob.current, deck.name);
      sound.play("chime");
      toast(`Immagine salvata in ${path}`, "ok");
    } catch (e) {
      sound.play("error");
      toast(`Immagine non salvata: ${e}`, "error");
    }
  };

  const copyImage = async () => {
    if (!blob.current) return;
    try {
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob.current })]);
      sound.play("chime");
      toast("Immagine copiata negli appunti", "ok");
    } catch {
      sound.play("error");
      toast("Questo sistema non permette di copiare immagini: usa Salva PNG", "error");
    }
  };

  return (
    <Modal title="Immagine del deck" onClose={onClose} wide>
      <div className="relative grid min-h-[240px] place-items-center overflow-hidden rounded-xl border border-border bg-bg-2">
        {url && <img src={url} alt={`Immagine del deck ${deck.name}`} className="max-h-[62vh] w-full object-contain" />}
        {busy && <p className="absolute inset-0 grid place-items-center bg-bg-2/70 text-sm text-muted">Preparo l'immagine…</p>}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <label className="flex items-center gap-2 text-sm text-muted">
          <input type="checkbox" checked={side} onChange={(e) => setSide(e.target.checked)} className="accent-[var(--sec)]" />
          Includi il Side Deck
        </label>
        <span className="ml-auto text-xs text-faint">Salvata nella cartella Download/DuelForge</span>
        <button type="button" className="btn btn-ghost" onClick={copyImage} disabled={busy}>
          Copia
        </button>
        <button type="button" className="btn btn-molten" onClick={save} disabled={busy}>
          Salva PNG
        </button>
      </div>
    </Modal>
  );
}
