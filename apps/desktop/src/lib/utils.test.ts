import { describe, expect, it } from "vitest";
import { displayAttributeValue, formatValue, initials, formatWage } from "./utils";

describe("formatValue", () => {
  it("formats millions", () => {
    expect(formatValue(4_408_650)).toBe("4.41M");
    expect(formatValue(1_500)).toBe("2K");
  });
  it("uses an em dash for missing values", () => {
    expect(formatValue(null)).toBe("—");
    expect(formatValue(0)).toBe("—");
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
});

describe("initials", () => {
  it("handles multi-part names", () => {
    expect(initials("Luka Modrić")).toBe("LM");
    expect(initials("Modric")).toBe("MO");
    expect(initials(null)).toBe("?");
  });
});
