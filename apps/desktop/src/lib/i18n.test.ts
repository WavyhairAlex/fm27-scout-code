import { describe, expect, it } from "vitest";
import { resolveLocale, translate } from "./i18n";
import {
  attributeLabel,
  clauseKindLabel,
  clauseValueLabel,
  nationLabel,
  personalityLabel,
  roleName,
  traitLabel,
} from "./labels";
import {
  ATTRIBUTE_FILTER_GROUPS,
  ATTRIBUTE_FILTER_KEYS,
  ATTRIBUTE_GROUPS,
  ATTRIBUTE_LABELS,
  HIDDEN_ATTRIBUTE_KEYS,
  PERSONALITY_KEYS,
} from "./types";

describe("resolveLocale", () => {
  it("follows Simplified Chinese system locales", () => {
    expect(resolveLocale("system", ["zh-CN"])).toBe("zh-CN");
    expect(resolveLocale("system", ["zh-Hans-CN"])).toBe("zh-CN");
  });

  it("follows English system locales", () => {
    expect(resolveLocale("system", ["en-GB"])).toBe("en");
    expect(resolveLocale("system", ["en-US", "zh-CN"])).toBe("en");
  });

  it("falls back to English for unsupported languages", () => {
    expect(resolveLocale("system", ["fr-FR"])).toBe("en");
    expect(resolveLocale("system", [])).toBe("en");
  });

  it("honours an explicit preference", () => {
    expect(resolveLocale("zh-CN", ["en-US"])).toBe("zh-CN");
    expect(resolveLocale("en", ["zh-CN"])).toBe("en");
  });
});

describe("translate", () => {
  it("interpolates values", () => {
    expect(translate("en", "common.yearsOld", { age: 24 })).toBe("24 yrs");
    expect(translate("zh-CN", "common.yearsOld", { age: 24 })).toBe("24 岁");
  });

  it("translates chrome strings", () => {
    expect(translate("en", "filter.title")).toBe("Filters");
    expect(translate("zh-CN", "filter.title")).toBe("筛选");
  });
});

describe("attribute coverage", () => {
  it("labels every player attribute, including hidden ones", () => {
    const keys = Object.values(ATTRIBUTE_GROUPS).flatMap((groups) =>
      groups.flatMap((group) => group.keys),
    );
    expect(keys).toHaveLength(52);
    expect(new Set(keys).size).toBe(52);
    expect(Object.keys(ATTRIBUTE_LABELS).sort()).toEqual([...keys].sort());
    for (const key of keys) {
      expect(attributeLabel(key, "en")).not.toBe(key);
      expect(attributeLabel(key, "zh-CN")).not.toBe(key);
    }
    for (const key of HIDDEN_ATTRIBUTE_KEYS) {
      expect(ATTRIBUTE_LABELS[key]).toBeTruthy();
    }
  });

  it("includes every personality attribute in the filter options", () => {
    expect(PERSONALITY_KEYS).toHaveLength(8);
    expect(ATTRIBUTE_FILTER_KEYS).toHaveLength(60);
    for (const key of PERSONALITY_KEYS) {
      expect(ATTRIBUTE_FILTER_KEYS).toContain(key);
      expect(attributeLabel(key, "en")).not.toBe(key);
      expect(attributeLabel(key, "zh-CN")).not.toBe(key);
    }
  });

  it("groups every filter option exactly once", () => {
    const grouped = ATTRIBUTE_FILTER_GROUPS.flatMap((group) => group.keys);
    expect(grouped).toHaveLength(60);
    expect(new Set(grouped).size).toBe(60);
    expect([...grouped].sort()).toEqual([...ATTRIBUTE_FILTER_KEYS].sort());
    for (const group of ATTRIBUTE_FILTER_GROUPS) {
      expect(translate("en", group.labelKey)).not.toBe(group.labelKey);
      expect(translate("zh-CN", group.labelKey)).not.toBe(group.labelKey);
    }
  });
});

describe("localized domain labels", () => {
  it("translates country names only in English mode", () => {
    expect(nationLabel("英格兰", "en")).toBe("England");
    expect(nationLabel("英格兰", "zh-CN")).toBe("英格兰");
    expect(nationLabel("Wales", "en")).toBe("Wales");
    expect(nationLabel("科索沃", "en")).toBe("Kosovo");
    expect(nationLabel("法属圭亚那", "en")).toBe("French Guiana");
    expect(nationLabel("荷属圣马丁", "en")).toBe("Sint Maarten");
    expect(nationLabel("圣巴泰勒米", "en")).toBe("St Barthélemy");
  });

  it("translates attributes, traits, roles and personality", () => {
    expect(attributeLabel("passing", "en")).toBe("Passing");
    expect(attributeLabel("passing", "zh-CN")).toBe("传球");
    expect(traitLabel(8, "Shoots From Distance", "zh-CN")).toBe("远射");
    expect(traitLabel(10, "Places Shots", "zh-CN")).toBe("巧射");
    expect(traitLabel(53, null, "zh-CN")).toBe("未知习惯");
    expect(traitLabel(53, null, "en")).toBe("Unknown trait");
    expect(roleName("poacher", "抢点前锋（进攻）", "en")).toBe("Poacher (Attack)");
    expect(personalityLabel("professionalism", "en")).toBe("Professionalism");
  });
});

describe("contract clause labels", () => {
  // Every clause kind that appears in the current FM27 save.
  const OBSERVED_KINDS = [
    0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 37, 38, 39, 40, 41, 54, 58, 63,
  ];

  it("names every clause kind found in the save", () => {
    for (const kind of OBSERVED_KINDS) {
      expect(clauseKindLabel(kind, "en")).toBeTruthy();
      expect(clauseKindLabel(kind, "zh-CN")).toBeTruthy();
    }
    expect(clauseKindLabel(57, "en")).toBeNull();
  });

  it("names the newly reported clause kinds", () => {
    expect(clauseKindLabel(3, "en")).toBe("Yearly Wage Rise");
    expect(clauseKindLabel(3, "zh-CN")).toBe("年度涨薪条款");
    expect(clauseKindLabel(28, "en")).toBe("MLS Guaranteed Contract Clause");
    expect(clauseKindLabel(33, "en")).toBe("Goal Bonus");
    expect(clauseKindLabel(39, "en")).toBe("Assist Bonus");
    expect(clauseKindLabel(40, "en")).toBe("Seasonal Landmark Assist Bonus");
    expect(clauseKindLabel(41, "en")).toBe(
      "Seasonal Landmark Combined Goal and Assist Bonus",
    );
  });

  it("formats clause values by kind", () => {
    expect(clauseValueLabel({ kind: 18, parameter: 365, value: 4_408_650 }, "en")).toBe(
      "4.41M",
    );
    expect(clauseValueLabel({ kind: 22, parameter: 2, value: null }, "en")).toBe("2 yrs");
    expect(clauseValueLabel({ kind: 22, parameter: 2, value: null }, "zh-CN")).toBe("2 年");
    expect(clauseValueLabel({ kind: 15, parameter: 25, value: null }, "en")).toBe("25%");
    expect(clauseValueLabel({ kind: 3, parameter: 10, value: null }, "en")).toBe("10%");
    expect(clauseValueLabel({ kind: 28, parameter: null, value: null }, "en")).toBe(
      "Included",
    );
    expect(clauseValueLabel({ kind: 33, parameter: null, value: 25_000 }, "en")).toBe("25K");
    expect(clauseValueLabel({ kind: 39, parameter: null, value: 96_641 }, "en")).toBe("97K");
    expect(clauseValueLabel({ kind: 40, parameter: 15, value: 1_692_425 }, "en")).toBe(
      "1.69M · 15 league assists",
    );
    expect(clauseValueLabel({ kind: 41, parameter: 20, value: 500_000 }, "en")).toBe(
      "500K · 20 goals + assists",
    );
    expect(clauseValueLabel({ kind: 41, parameter: 20, value: 500_000 }, "zh-CN")).toBe(
      "500K · 20 进球 + 助攻",
    );
    expect(clauseValueLabel({ kind: 9, parameter: 20, value: 10_000_000 }, "en")).toBe(
      "20% of profit over 10.00M",
    );
    expect(clauseValueLabel({ kind: 11, parameter: 20, value: null }, "en")).toBe(
      "20 league games",
    );
    expect(clauseValueLabel({ kind: 13, parameter: 20, value: 5_000 }, "en")).toBe(
      "5,000/wk · 20 league games",
    );
    expect(clauseValueLabel({ kind: 58, parameter: 731, value: null }, "en")).toBe(
      "Included",
    );
  });
});
