import { useState, type ReactNode } from "react";
import { InfoDialog } from "./InfoDialog";
import type { InfoKind } from "./Mascot";

/**
 * A small "i" button that opens the Archmage's explanation. `kind` picks
 * the glyph he chalks on his board.
 */
export function InfoButton({
  title,
  children,
  className = "",
  kind = "what",
}: {
  title: string;
  children: ReactNode;
  className?: string;
  kind?: InfoKind;
}) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        className={`info-button ${className}`}
        aria-label={`Che cos'è: ${title}`}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      >
        i
      </button>
      {open && (
        <InfoDialog kind={kind} title={title} onClose={() => setOpen(false)}>
          {children}
        </InfoDialog>
      )}
    </>
  );
}
