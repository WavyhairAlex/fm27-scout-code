import type { Locale } from "./i18n";

/**
 * Football Manager stores every money field in its own base unit (GBP). These
 * are approximate mid-market rates expressed as currency units per GBP, so a
 * stored value can be shown in the currency the user picked in Settings.
 *
 * Rates are a snapshot, not a live feed: the game's own exchange rates can
 * differ and are historically stored in the save's database.
 */
export const CURRENCY_RATES_DATE = "2026-11-06";

export const CURRENCY_RATES = {
  AED: 4.943395,
  AFN: 87.615018,
  ALL: 109.008535,
  AMD: 485.487162,
  ANG: 2.409443,
  AOA: 1280.885226,
  ARS: 2080.193404,
  AUD: 1.932444,
  AWG: 2.409443,
  AZN: 2.289805,
  BAM: 2.344902,
  BBD: 2.692115,
  BDT: 167.681921,
  BGN: 2.344902,
  BHD: 0.506118,
  BIF: 4033.135922,
  BMD: 1.346057,
  BND: 1.719919,
  BOB: 16.281371,
  BRL: 7.363171,
  BSD: 1.346057,
  BTN: 127.207484,
  BWP: 19.126272,
  BYN: 4.010319,
  BZD: 2.692115,
  CAD: 1.883478,
  CDF: 3123.406015,
  CHF: 1.098806,
  CLF: 0.032917,
  CLP: 1301.082382,
  CNH: 8.972534,
  CNY: 8.988652,
  COP: 4355.375433,
  CRC: 611.245149,
  CUC: 1.346049,
  CUP: 32.305374,
  CVE: 132.199961,
  CZK: 29.140983,
  DJF: 239.222644,
  DKK: 8.962582,
  DOP: 81.173664,
  DZD: 183.177064,
  EGP: 72.54915,
  ERN: 20.190859,
  ETB: 219.589432,
  EUR: 1.198913,
  FJD: 3.018052,
  FKP: 1,
  FOK: 8.961332,
  GBP: 1,
  GEL: 3.496451,
  GGP: 1,
  GHS: 15.762415,
  GIP: 1,
  GMD: 100.632145,
  GNF: 11904.62036,
  GTQ: 10.297955,
  GYD: 282.786249,
  HKD: 10.556017,
  HNL: 36.211255,
  HRK: 9.033334,
  HTG: 176.846743,
  HUF: 436.21509,
  IDR: 24327.22375,
  ILS: 4.295634,
  IMP: 1,
  INR: 127.205877,
  IQD: 2046.369458,
  IRR: 2531521.739,
  ISK: 163.843708,
  JEP: 1,
  JMD: 213.679257,
  JOD: 0.954355,
  JPY: 212.496698,
  KES: 175.082576,
  KGS: 117.869394,
  KHR: 5538.84,
  KID: 1.931509,
  KMF: 589.834646,
  KPW: 1211.4441,
  KRW: 1806.305616,
  KWD: 0.416157,
  KYD: 1.121714,
  KZT: 606.717481,
  LAK: 30116.7498,
  LBP: 120472.1256,
  LKR: 447.844277,
  LRD: 229.067844,
  LSL: 22.148964,
  LYD: 8.649212,
  MAD: 13.146796,
  MDL: 24.087442,
  MGA: 6161.292515,
  MKD: 71.9061,
  MMK: 2826.576636,
  MNT: 4816.91056,
  MOP: 10.875455,
  MRU: 54.523297,
  MUR: 62.967219,
  MVR: 20.831474,
  MWK: 2348.307073,
  MXN: 25.196657,
  MYR: 5.446849,
  MZN: 85.96634,
  NAD: 22.148964,
  NGN: 1796.965976,
  NIO: 49.578332,
  NOK: 12.512819,
  NPR: 203.531974,
  NZD: 2.398969,
  OMR: 0.517555,
  PAB: 1.346057,
  PEN: 4.64459,
  PGK: 6.074212,
  PHP: 84.774279,
  PKR: 372.111057,
  PLN: 5.272963,
  PYG: 8210.619858,
  QAR: 4.899648,
  RON: 6.402444,
  RSD: 141.024069,
  RUB: 109.801279,
  RWF: 1993.506635,
  SAR: 5.047715,
  SBD: 10.71926,
  SCR: 18.960104,
  SDG: 602.921626,
  SEK: 13.185681,
  SGD: 1.719904,
  SHP: 1,
  SLE: 31.007326,
  SLL: 31006.63051,
  SOS: 770.710575,
  SRD: 50.740564,
  SSP: 8218.464952,
  STN: 29.373773,
  SYP: 163.995251,
  SZL: 22.148964,
  THB: 45.025885,
  TJS: 12.387419,
  TMT: 4.712663,
  TND: 4.043101,
  TOP: 3.215442,
  TRY: 67.435804,
  TTD: 9.27241,
  TVD: 1.931509,
  TWD: 44.405575,
  TZS: 3533.960723,
  UAH: 61.271163,
  UGX: 5379.244357,
  USD: 1.346049,
  UYU: 54.212159,
  UZS: 16063.46242,
  VES: 1206.232833,
  VND: 34571.92786,
  VUV: 161.291563,
  WST: 3.696644,
  XAF: 786.446195,
  XCD: 3.634355,
  XCG: 2.409443,
  XDR: 0.994879,
  XOF: 786.446195,
  XPF: 143.070655,
  YER: 318.220422,
  ZAR: 22.141539,
  ZMW: 27.359602,
  ZWG: 35.8086,
  ZWL: 35.8086,
} as const;

export type IsoCurrencyCode = keyof typeof CURRENCY_RATES;
export type CurrencyCode = "save" | IsoCurrencyCode;

export const SAVE_UNITS = "save" as const;

/** Currencies shown before the full list in Settings. */
export const COMMON_CURRENCY_CODES: IsoCurrencyCode[] = [
  "GBP",
  "EUR",
  "USD",
  "CNY",
  "JPY",
  "KRW",
  "BRL",
  "INR",
  "RUB",
  "TRY",
  "SAR",
  "AED",
  "AUD",
  "CAD",
  "CHF",
];

const ALL_CURRENCY_CODES = Object.keys(CURRENCY_RATES) as IsoCurrencyCode[];

export function isCurrencyCode(value: string | null | undefined): value is CurrencyCode {
  return (
    value != null &&
    (value === SAVE_UNITS ||
      Object.prototype.hasOwnProperty.call(CURRENCY_RATES, value))
  );
}

export function isIsoCurrencyCode(value: string | null | undefined): value is IsoCurrencyCode {
  return value != null && Object.prototype.hasOwnProperty.call(CURRENCY_RATES, value);
}

export function currencyRate(currency: CurrencyCode): number {
  return currency === SAVE_UNITS ? 1 : CURRENCY_RATES[currency];
}

/** Convert a value stored in the save's GBP base unit into the display currency. */
export function convertFromBase(value: number, currency: CurrencyCode): number {
  return currency === SAVE_UNITS ? value : value * currencyRate(currency);
}

/** Convert a display-currency amount back into the save's GBP base unit. */
export function convertToBase(value: number, currency: CurrencyCode): number {
  return currency === SAVE_UNITS ? value : value / currencyRate(currency);
}

const displayNameCache = new Map<string, Intl.DisplayNames>();
const symbolCache = new Map<IsoCurrencyCode, string>();

function displayNames(locale: Locale): Intl.DisplayNames | null {
  if (typeof Intl === "undefined" || typeof Intl.DisplayNames === "undefined") return null;
  let names = displayNameCache.get(locale);
  if (!names) {
    try {
      names = new Intl.DisplayNames([locale], { type: "currency" });
      displayNameCache.set(locale, names);
    } catch {
      return null;
    }
  }
  return names;
}

export function currencyName(currency: IsoCurrencyCode, locale: Locale): string {
  try {
    return displayNames(locale)?.of(currency) ?? currency;
  } catch {
    return currency;
  }
}

/** A short, unambiguous symbol such as "£", "US$" or "CN¥". */
export function currencySymbol(currency: CurrencyCode): string {
  if (currency === SAVE_UNITS) return "";
  const cached = symbolCache.get(currency);
  if (cached) return cached;
  try {
    const symbol =
      new Intl.NumberFormat("en", {
        style: "currency",
        currency,
        currencyDisplay: "symbol",
      })
        .formatToParts(0)
        .find((part) => part.type === "currency")?.value ?? currency;
    symbolCache.set(currency, symbol);
    return symbol;
  } catch {
    return currency;
  }
}

/**
 * The full list of in-game-style currencies, ordered by localized name. The
 * common list is kept separate so the Settings picker stays easy to scan.
 */
export function allCurrencyCodes(locale: Locale): IsoCurrencyCode[] {
  const common = new Set(COMMON_CURRENCY_CODES);
  return ALL_CURRENCY_CODES.filter((code) => !common.has(code)).sort((a, b) =>
    currencyName(a, locale).localeCompare(currencyName(b, locale), locale),
  );
}
