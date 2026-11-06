import { describe, expect, it } from "vitest";
import { currencySymbol, isCurrencyCode } from "./currency";
import {
  displayAttributeValue,
  formatMoney,
  formatValue,
  initials,
  formatWage,
} from "./utils";

describe("formatValue", () => {
  it("formats millions", () => {
    expect(formatValue(4_408_650)).toBe("4.41M");
    expect(formatValue(1_500)).toBe("2K");
  });
  it("uses an em dash for missing values", () => {
    expect(formatValue(null)).toBe("—");
    expect(formatValue(0)).toBe("—");
  });
  it("converts save units into the selected display currency", () => {
    expect(formatValue(4_408_650, "GBP")).toBe("£4.41M");
    expect(formatValue(4_408_650, "USD")).toBe("$5.93M");
    expect(formatValue(4_408_650, "CNY")).toBe("CN¥39.63M");
    expect(formatMoney(10_000_000, "USD")).toBe("$13.46M");
    expect(formatMoney(0, "USD")).toBe("$0");
  });
});

describe("currency helpers", () => {
  it("validates stored currency codes", () => {
    expect(isCurrencyCode("save")).toBe(true);
    expect(isCurrencyCode("USD")).toBe(true);
    expect(isCurrencyCode("XYZ")).toBe(false);
    expect(isCurrencyCode(null)).toBe(false);
  });

  it("uses an unambiguous symbol where Intl provides one", () => {
    expect(currencySymbol("GBP")).toBe("£");
    expect(currencySymbol("EUR")).toBe("€");
    expect(currencySymbol("USD")).toBe("$");
    expect(currencySymbol("CAD")).toBe("CA$");
    expect(currencySymbol("save")).toBe("");
  });
});

describe("displayAttributeValue", () => {
  it("maps raw 1-100 to the in-game 1-20 scale", () => {
    expect(displayAttributeValue(1)).toBe(1);
    expect(displayAttributeValue(50)).toBe(10);
    expect(displayAttributeValue(97)).toBe(19);
    expect(displayAttributeValue(98)).toBe(20);
    expect(displayAttributeValue(100)).toBe(20);
  });
});

describe("formatWage", () => {
  it("appends per-week marker", () => {
    expect(formatWage(106965)).toContain("106,965");
    expect(formatWage(null)).toBe("—");
  });

  it("converts and prefixes the selected currency", () => {
    expect(formatWage(106965, "en", "GBP")).toBe("£106,965/wk");
    expect(formatWage(106965, "en", "USD")).toBe("$143,980/wk");
  });

  it("compacts large converted wages", () => {
    expect(formatWage(106965, "en", "JPY")).toBe("¥22.73M/wk");
  });
});

describe("initials", () => {
  it("handles multi-part names", () => {
    expect(initials("Luka Modrić")).toBe("LM");
    expect(initials("Modric")).toBe("MO");
    expect(initials(null)).toBe("?");
  });
});
