import {
  forwardRef,
  useEffect,
  useId,
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  type SelectHTMLAttributes,
} from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { useI18n } from "@/lib/useI18n";
import { cn } from "@/lib/utils";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "gold";
type ButtonSize = "sm" | "md" | "lg" | "icon";

export const Button = forwardRef<
  HTMLButtonElement,
  ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: ButtonSize }
>(function Button({ className, variant = "secondary", size = "md", ...props }, ref) {
  const styles: Record<ButtonVariant, string> = {
    primary:
      "bg-[var(--primary)] text-white border-[var(--primary)] hover:bg-[var(--primary-strong)] shadow-sm",
    secondary:
      "bg-[var(--surface)] text-[var(--ink)] border-[var(--line)] hover:bg-[var(--surface-2)] shadow-sm",
    ghost:
      "bg-transparent text-[var(--muted)] border-transparent hover:bg-[var(--surface-2)] hover:text-[var(--ink)]",
    danger:
      "bg-[var(--danger-soft)] text-[var(--danger)] border-[var(--danger-line)] hover:border-[var(--danger)]",
    gold:
      "bg-[var(--gold-soft)] text-[var(--gold)] border-[var(--gold-line)] hover:bg-[var(--gold)] hover:text-[var(--gold-contrast)]",
  };
  const sizes: Record<ButtonSize, string> = {
    sm: "h-7 px-2.5 text-[11px] gap-1.5",
    md: "h-8 px-3 text-xs gap-2",
    lg: "h-10 px-4 text-sm gap-2",
    icon: "h-8 w-8 p-0",
  };
  return (
    <button
      ref={ref}
      className={cn(
        "focus-ring inline-flex shrink-0 items-center justify-center rounded-[7px] border font-medium transition-colors disabled:pointer-events-none disabled:opacity-45",
        styles[variant],
        sizes[size],
        className,
      )}
      {...props}
    />
  );
});

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...props }, ref) {
    return (
      <input
        ref={ref}
        className={cn(
          "focus-ring h-8 w-full rounded-[7px] border border-[var(--line)] bg-[var(--surface)] px-2.5 text-xs text-[var(--ink)] placeholder:text-[var(--muted)]/70 transition-colors hover:border-[var(--line-strong)] focus:border-[var(--accent)]",
          className,
        )}
        {...props}
      />
    );
  },
);

export const Select = forwardRef<
  HTMLSelectElement,
  SelectHTMLAttributes<HTMLSelectElement> & { children: ReactNode }
>(function Select({ className, children, ...props }, ref) {
  return (
    <select
      ref={ref}
      className={cn(
        "focus-ring h-8 w-full appearance-none rounded-[7px] border border-[var(--line)] bg-[var(--surface)] px-2.5 text-xs text-[var(--ink)] transition-colors hover:border-[var(--line-strong)] focus:border-[var(--accent)]",
        className,
      )}
      {...props}
    >
      {children}
    </select>
  );
});

export function Label({
  children,
  htmlFor,
  className,
}: {
  children: ReactNode;
  htmlFor?: string;
  className?: string;
}) {
  return (
    <label
      htmlFor={htmlFor}
      className={cn("text-[11px] font-medium tracking-wide text-[var(--muted)]", className)}
    >
      {children}
    </label>
  );
}

export function CheckboxBox({
  checked,
  className,
}: {
  checked: boolean;
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "flex h-4 w-4 shrink-0 items-center justify-center rounded-[4px] border transition-colors",
        checked
          ? "border-[var(--accent)] bg-[var(--accent)] text-white"
          : "border-[var(--line-strong)] bg-[var(--surface)]",
        className,
      )}
    >
      {checked && (
        <svg viewBox="0 0 12 12" className="h-3 w-3" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M2.5 6.2 4.8 8.5 9.5 3.8" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      )}
    </span>
  );
}

export function Checkbox({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: ReactNode;
  disabled?: boolean;
}) {
  const id = useId();
  return (
    <label
      htmlFor={id}
      className={cn(
        "flex cursor-pointer items-center gap-2 text-xs text-[var(--ink)]",
        disabled && "cursor-not-allowed opacity-50",
      )}
    >
      <CheckboxBox checked={checked} />
      <input
        id={id}
        type="checkbox"
        className="sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="leading-tight">{label}</span>
    </label>
  );
}

export function Switch({
  checked,
  onChange,
  disabled,
  label,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "focus-ring relative h-[22px] w-[38px] shrink-0 rounded-full border transition-colors disabled:opacity-50",
        checked
          ? "border-[var(--accent)] bg-[var(--accent)]"
          : "border-[var(--line-strong)] bg-[var(--surface-3)]",
      )}
    >
      <span
        className={cn(
          "absolute left-[1px] top-[2px] h-4 w-4 rounded-full bg-white shadow-sm transition-transform",
          checked ? "translate-x-[18px]" : "translate-x-0",
        )}
      />
    </button>
  );
}

export function Badge({
  children,
  tone = "default",
  className,
  title,
}: {
  children: ReactNode;
  tone?: "default" | "accent" | "gold" | "blue" | "danger" | "outline";
  className?: string;
  title?: string;
}) {
  const tones = {
    default: "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)]",
    accent: "border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)]",
    gold: "border-[var(--gold-line)] bg-[var(--gold-soft)] text-[var(--gold)]",
    blue: "border-[var(--blue-line)] bg-[var(--blue-soft)] text-[var(--blue)]",
    danger: "border-[var(--danger-line)] bg-[var(--danger-soft)] text-[var(--danger)]",
    outline: "border-[var(--line)] bg-transparent text-[var(--muted)]",
  };
  return (
    <span
      title={title}
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold leading-tight",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  width = "max-w-2xl",
}: {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  width?: string;
}) {
  const { t } = useI18n();
  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;
  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        aria-label={t("common.close")}
        className="absolute inset-0 cursor-default bg-black/35 backdrop-blur-[2px] dark:bg-black/55"
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        className={cn(
          "relative flex max-h-[86vh] w-full flex-col overflow-hidden rounded-2xl border border-[var(--line)] bg-[var(--surface)] shadow-[var(--shadow-pop)]",
          width,
        )}
      >
        <div className="flex items-start justify-between gap-4 border-b border-[var(--line)] px-5 py-4">
          <div>
            <div className="text-[15px] font-semibold tracking-tight text-[var(--ink)]">{title}</div>
            {description && <div className="mt-0.5 text-xs text-[var(--muted)]">{description}</div>}
          </div>
          <Button variant="ghost" size="icon" onClick={onClose} aria-label={t("common.close")}>
            <X size={16} />
          </Button>
        </div>
        <div className="thin-scroll min-h-0 flex-1 overflow-auto">{children}</div>
        {footer && (
          <div className="flex items-center justify-end gap-2 border-t border-[var(--line)] bg-[var(--surface-2)] px-5 py-3">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}

export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 px-8 text-center">
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl border border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)]">
        {icon}
      </div>
      <div>
        <div className="text-sm font-semibold text-[var(--ink)]">{title}</div>
        <div className="mt-1 max-w-sm text-xs leading-relaxed text-[var(--muted)]">{description}</div>
      </div>
      {action}
    </div>
  );
}
