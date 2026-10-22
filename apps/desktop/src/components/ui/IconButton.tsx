import type { ButtonHTMLAttributes, ReactNode } from "react";

export type IconButtonVariant = "default" | "ghost" | "surface" | "primary" | "danger";
export type IconButtonSize = "sm" | "md" | "lg";

export interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "title" | "aria-label"> {
  /** The icon to display inside the button. */
  icon: ReactNode;
  /** Mandatory tooltip explaining the action in Italian. */
  tooltip: string;
  /** Mandatory accessible label for screen readers. */
  ariaLabel: string;
  /** Visual style variant. */
  variant?: IconButtonVariant;
  /** Button dimensions. */
  size?: IconButtonSize;
  /** Active / selected state (sets aria-pressed). */
  active?: boolean;
}

const SIZE_CLASSES: Record<IconButtonSize, string> = {
  sm: "h-7 w-7 text-xs rounded-lg p-1",
  md: "h-8 w-8 text-sm rounded-lg p-1.5",
  lg: "h-9 w-9 text-base rounded-xl p-2",
};

const VARIANT_CLASSES: Record<IconButtonVariant, string> = {
  default:
    "border border-border bg-surface-2 text-muted hover:border-border-strong hover:bg-surface-3 hover:text-text",
  ghost:
    "border border-transparent bg-transparent text-muted hover:bg-surface-2 hover:text-text",
  surface:
    "border border-border-strong bg-surface text-text hover:bg-surface-2 hover:border-gold/50",
  primary:
    "border border-gold/40 bg-gold/15 text-gold hover:bg-gold/25 hover:border-gold/70",
  danger:
    "border border-danger/40 bg-danger/10 text-danger hover:bg-danger/20 hover:border-danger/70",
};

/**
 * Universal accessible icon-only button adhering to UI revision principles:
 * mandatory `tooltip` (triggers global ChartTooltip / native fallback) and mandatory `ariaLabel`.
 */
export function IconButton({
  icon,
  tooltip,
  ariaLabel,
  variant = "default",
  size = "md",
  active = false,
  disabled = false,
  className = "",
  type = "button",
  onClick,
  ...rest
}: IconButtonProps) {
  const activeClass = active
    ? "border-sec/80 bg-sec/20 text-sec shadow-xs"
    : "";

  return (
    <button
      type={type}
      data-tip={tooltip}
      title={tooltip}
      aria-label={ariaLabel}
      aria-pressed={active}
      disabled={disabled}
      onClick={onClick}
      className={`inline-flex items-center justify-center transition cursor-pointer select-none disabled:pointer-events-none disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold/70 focus-visible:ring-offset-1 focus-visible:ring-offset-bg active:scale-95 ${SIZE_CLASSES[size]} ${VARIANT_CLASSES[variant]} ${activeClass} ${className}`}
      {...rest}
    >
      <span className="flex items-center justify-center shrink-0 leading-none">
        {icon}
      </span>
    </button>
  );
}
