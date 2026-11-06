import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import {
  convertFromBase,
  currencySymbol,
  SAVE_UNITS,
  type CurrencyCode,
} from "./currency";
import { formatRelativeTime, translate, type Locale } from "./i18n";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

function compactAmount(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1_000_000_000) return `${(abs / 1_000_000_000).toFixed(1)}B`;
  if (abs >= 1_000_000) return `${(abs / 1_000_000).toFixed(2)}M`;
  if (abs >= 1_000) return `${(abs / 1_000).toFixed(0)}K`;
  return abs.toLocaleString("en-US", {
    maximumFractionDigits: abs < 10 ? 2 : 0,
  });
}

function formatConverted(value: number, currency: CurrencyCode): string {
  const converted = convertFromBase(value, currency);
  const sign = converted < 0 ? "-" : "";
  const symbol = currency === SAVE_UNITS ? "" : currencySymbol(currency);
  return `${sign}${symbol}${compactAmount(converted)}`;
}

export function formatValue(
  value: number | null | undefined,
  currency: CurrencyCode = SAVE_UNITS,
): string {
  if (value == null || value === 0) return "—";
  return formatConverted(value, currency);
}

export function formatMoney(
  value: number | null | undefined,
  currency: CurrencyCode = SAVE_UNITS,
): string {
  if (value == null) return "—";
  if (value === 0) {
    return `${currency === SAVE_UNITS ? "" : currencySymbol(currency)}0`;
  }
  return formatConverted(value, currency);
}

export function formatWage(
  value: number | null | undefined,
  locale: Locale = "en",
  currency: CurrencyCode = SAVE_UNITS,
): string {
  if (value == null) return "—";
  const converted = convertFromBase(value, currency);
  const symbol = currency === SAVE_UNITS ? "" : currencySymbol(currency);
  const sign = converted < 0 ? "-" : "";
  const amount =
    Math.abs(converted) < 1_000_000
      ? Math.round(Math.abs(converted)).toLocaleString(
          locale === "zh-CN" ? "zh-CN" : "en-US",
        )
      : compactAmount(converted);
  return `${sign}${symbol}${amount}${translate(locale, "common.perWeek")}`;
}

export function formatDate(value: string | null | undefined): string {
  if (!value) return "—";
  return value;
}

export function initials(name: string | null | undefined): string {
  if (!name) return "?";
  const parts = name.split(/\s+/).filter(Boolean);
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

export function caTone(ca: number): string {
  if (ca >= 170) return "text-[var(--gold)] bg-[var(--gold-soft)] border-[var(--gold-line)]";
  if (ca >= 140) return "text-[var(--success)] bg-[var(--success-soft)] border-[var(--success-line)]";
  if (ca >= 110) return "text-[var(--blue)] bg-[var(--blue-soft)] border-[var(--blue-line)]";
  return "text-[var(--muted)] bg-[var(--surface-2)] border-[var(--line)]";
}

/** Convert a stored raw attribute (1-100) to the game's displayed 1-20 value. */
export function displayAttributeValue(raw: number): number {
  return Math.max(1, Math.floor((raw + 2) / 5));
}

export function attributeTone(value: number): string {
  if (value >= 16) return "bg-[var(--gold)] text-[var(--gold-contrast)]";
  if (value >= 13) return "bg-[var(--success-emphasis)] text-white";
  if (value >= 10) return "bg-[var(--accent-soft)] text-[var(--accent-strong)]";
  if (value >= 7) return "bg-[var(--surface-3)] text-[var(--muted)]";
  return "bg-[var(--danger-soft)] text-[var(--danger)]";
}

export function attributeBar(value: number, max = 20): string {
  const pct = Math.max(4, Math.min(100, (value / max) * 100));
  return `${pct}%`;
}

export function relativeTime(iso: string, locale: Locale = "en"): string {
  return formatRelativeTime(iso, locale);
}

export function suggestedEditedSavePath(
  source: string | undefined,
  saveName: string | undefined,
): string {
  const cleanName = (saveName || "fm27-save")
    .replace(/[\\/:*?"<>|]/g, "_")
    .replace(/\.fm$/i, "");
  const separator = source ? Math.max(source.lastIndexOf("/"), source.lastIndexOf("\\")) : -1;
  const directory = separator >= 0 ? source!.slice(0, separator + 1) : "";
  return `${directory}${cleanName} - edited.fm`;
}

export function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024).toFixed(0)} KB`;
}
