import { describe, expect, it } from "vitest";
import { resolveLocale, translate } from "./i18n";
import { attributeLabel, nationLabel, personalityLabel, roleName, traitLabel } from "./labels";

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
