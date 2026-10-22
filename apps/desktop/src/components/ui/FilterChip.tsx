import type { ButtonHTMLAttributes, ReactNode } from "react";

export interface FilterChipProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /** Optional icon shown before label (14-16px). */
  icon?: ReactNode;
  /** Visible label text. */
  label: string;
  /** Counter badge number or text. */
  count?: number | string;
  /** Whether the filter is active/selected. */
  active?: boolean;
  /** Custom color dot or indicator (e.g. role or link color). */
  color?: string;
  /** If true and count is 0, the chip is not rendered at all. */
  hideWhenZero?: boolean;
  /** If true (default), the chip is disabled when count is 0 and not active. Set to false for reset chips. */
  disableWhenZero?: boolean;
  /** Optional tooltip explaining what the filter selects. */
  tooltip?: string;
  /** Size variant: default (compact) or lg. */
  size?: "sm" | "md";
}

/**
 * Standard filter chip with icon, label, and counter badge.
 * Principle: filters with count 0 are hidden when `hideWhenZero` is set, or styled as faint.
 */
export function FilterChip({
  icon,
  label,
  count,
  active = false,
  color,
  hideWhenZero = false,
  disableWhenZero = true,
  tooltip,
  size = "md",
  disabled = false,
  className = "",
  type = "button",
  onClick,
  ...rest
}: FilterChipProps) {
  const numCount = typeof count === "number" ? count : null;
  if (hideWhenZero && numCount === 0) {
    return null;
  }

  const isZero = numCount === 0;
  const shouldDisable = disabled || (disableWhenZero && isZero && !active);
  const paddingClass = size === "sm" ? "px-2 py-0.5 text-small gap-1.5" : "px-2.5 py-1 text-small gap-2";

  return (
    <button
      type={type}
      data-tip={tooltip}
      aria-pressed={active}
      disabled={shouldDisable}
      onClick={onClick}
      className={`chip group cursor-pointer transition select-none disabled:pointer-events-none disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold/70 active:scale-97 ${paddingClass} ${
        active ? "chip-active font-medium" : "hover:border-border-strong hover:bg-surface-3"
      } ${isZero && !active && disableWhenZero ? "opacity-60 text-faint" : ""} ${className}`}
      {...rest}
    >
      {color && (
        <span
          className="inline-block h-2 w-2 shrink-0 rounded-full shadow-xs"
          style={{ backgroundColor: color }}
          aria-hidden="true"
        />
      )}
      {icon && (
        <span className="flex items-center shrink-0 leading-none text-muted group-hover:text-text transition-colors">
          {icon}
        </span>
      )}
      <span className="truncate">{label}</span>
      {count !== undefined && (
        <span
          className={`ml-0.5 shrink-0 rounded-full px-1.5 py-0.2 text-caption tabular-nums font-semibold transition-colors ${
            active
              ? "bg-sec/30 text-sec"
              : isZero
                ? "bg-bg-2 text-faint"
                : "bg-surface-3 text-muted group-hover:text-text"
          }`}
        >
          {count}
        </span>
      )}
    </button>
  );
}
