import { useState, type ReactNode } from "react";
import { HelpCircle } from "lucide-react";
import { IconButton } from "./IconButton";
import { InfoDialog } from "../InfoDialog";
import type { InfoKind } from "../Mascot";

export interface HelpPopoverProps {
  /** Optional header title for the dialog. */
  title?: string;
  /** Content (tips, shortcuts, instructions) replacing fixed screen text. */
  children: ReactNode;
  /** Tooltip for the trigger button. */
  tooltip?: string;
  /** Accessible label for the trigger button. */
  ariaLabel?: string;
  /** What the guide is about: the Archmage chalks the matching glyph. */
  kind?: InfoKind;
  /** Optional custom CSS classes for the trigger wrapper. */
  className?: string;
}

/**
 * Accessible «?» help button that opens the Archmage's explanation.
 * Principle: replaces fixed instructional text on screen with on-demand guidance.
 */
export function HelpPopover({
  title = "Suggerimenti e guida",
  children,
  tooltip = "Guida e suggerimenti",
  ariaLabel = "Mostra suggerimenti e guida",
  kind = "tip",
  className = "",
}: HelpPopoverProps) {
  const [open, setOpen] = useState(false);
  return (
    <div className={`relative inline-flex items-center ${className}`}>
      <IconButton
        icon={<HelpCircle size={15} aria-hidden="true" />}
        tooltip={tooltip}
        ariaLabel={ariaLabel}
        variant="ghost"
        size="sm"
        active={open}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      />
      {open && (
        <InfoDialog kind={kind} title={title} onClose={() => setOpen(false)}>
          {children}
        </InfoDialog>
      )}
    </div>
  );
}
