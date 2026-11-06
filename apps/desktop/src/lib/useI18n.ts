import { useCallback, useEffect, useState } from "react";
import { useAppStore } from "@/store/app";
import {
  resolveLocale,
  translate,
  type Locale,
  type TranslationKey,
} from "./i18n";
import {
  formatMoney as formatMoneyValue,
  formatValue as formatValueValue,
  formatWage as formatWageValue,
} from "./utils";

export function useI18n() {
  const language = useAppStore((state) => state.language);
  const currency = useAppStore((state) => state.currency);
  const [systemLocale, setSystemLocale] = useState<Locale>(() => resolveLocale("system"));

  useEffect(() => {
    if (typeof window === "undefined") return;
    const update = () => setSystemLocale(resolveLocale("system"));
    update();
    window.addEventListener("languagechange", update);
    return () => window.removeEventListener("languagechange", update);
  }, []);

  const locale: Locale = language === "system" ? systemLocale : language;

  const t = useCallback(
    (key: TranslationKey, values?: Record<string, string | number>) =>
      translate(locale, key, values),
    [locale],
  );

  const formatValue = useCallback(
    (value: number | null | undefined) => formatValueValue(value, currency),
    [currency],
  );
  const formatMoney = useCallback(
    (value: number | null | undefined) => formatMoneyValue(value, currency),
    [currency],
  );
  const formatWage = useCallback(
    (value: number | null | undefined) => formatWageValue(value, locale, currency),
    [currency, locale],
  );

  return { t, locale, language, currency, formatValue, formatMoney, formatWage };
}
