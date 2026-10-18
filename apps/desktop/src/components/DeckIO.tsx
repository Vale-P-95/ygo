import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api, type Deck, type Imported } from "../lib/api";
import { sound } from "../lib/sound";

function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  useEffect(() => {
    sound.play("open");
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [onClose]);
  return createPortal(
    <div className="fixed inset-0 z-50 grid place-items-center p-6 anim-fade" role="dialog" aria-modal="true" aria-label={title}>
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={onClose} />
      <div className="panel relative flex w-full max-w-2xl flex-col gap-4 p-6 anim-pop">
        <div className="flex items-center justify-between">
          <h2 className="display text-xl font-bold">{title}</h2>
          <button type="button" className="btn btn-ghost !px-2.5 !py-1" onClick={onClose} aria-label="Chiudi">
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>,
    document.body,
  );
}

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
        onChange={(e) => setText(e.target.value)}
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
