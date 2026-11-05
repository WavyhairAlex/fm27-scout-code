import { describe, expect, it } from "vitest";
import { estimateCaExact } from "./ca";
import { ATTRIBUTE_LABELS, type Attributes, type Positions } from "./types";

const ALL_65: Attributes = Object.fromEntries(
  Object.keys(ATTRIBUTE_LABELS).map((key) => [key, 65]),
);

function positions(entries: Partial<Positions>): Positions {
  return {
    gk: 0,
    sw: 0,
    dl: 0,
    dc: 0,
    dr: 0,
    dm: 0,
    ml: 0,
    mc: 0,
    mr: 0,
    aml: 0,
    amc: 0,
    amr: 0,
    stc: 0,
    wbl: 0,
    wbr: 0,
    ...entries,
  };
}

describe("exact RCA formula", () => {
  it("reproduces the article's single-striker example", () => {
    const estimate = estimateCaExact({
      attributes: ALL_65,
      positions: positions({ stc: 20 }),
      position: "STC",
      left_foot: 100,
      right_foot: 50,
    });
    expect(estimate).not.toBeNull();
    expect(estimate?.position).toBe("STC");
    expect(estimate?.estimated_ca).toBe(135);
    expect(estimate?.initial_ca).toBe(136);
    expect(estimate?.position_cost).toBe(0);
  });

  it("adds CA when both feet are strong", () => {
    const estimate = estimateCaExact({
      attributes: ALL_65,
      positions: positions({ stc: 20 }),
      position: "STC",
      left_foot: 100,
      right_foot: 100,
    });
    expect(estimate?.estimated_ca).toBe(151);
  });

  it("applies the position-combination cost for ST+DC", () => {
    const table = positions({ stc: 20, dc: 20 });
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: "STC",
        left_foot: 100,
        right_foot: 50,
      })?.estimated_ca,
    ).toBe(142);
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: "DC",
        left_foot: 100,
        right_foot: 50,
      })?.estimated_ca,
    ).toBe(144);
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: null,
        left_foot: 100,
        right_foot: 50,
      })?.estimated_ca,
    ).toBe(144);
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: null,
        left_foot: 100,
        right_foot: 50,
      })?.position_cost,
    ).toBe(25);
  });

  it("handles ST+AMC as in the article", () => {
    const table = positions({ stc: 20, amc: 20 });
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: "STC",
        left_foot: 100,
        right_foot: 50,
      })?.estimated_ca,
    ).toBe(135);
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: "AMC",
        left_foot: 100,
        right_foot: 50,
      })?.estimated_ca,
    ).toBe(134);
    expect(
      estimateCaExact({
        attributes: ALL_65,
        positions: table,
        position: null,
        left_foot: 100,
        right_foot: 50,
      })?.position_cost,
    ).toBe(1);
  });

  it("scales the result by position familiarity", () => {
    const estimate = estimateCaExact({
      attributes: ALL_65,
      positions: positions({ stc: 15 }),
      position: "STC",
      left_foot: 100,
      right_foot: 50,
    });
    expect(estimate?.estimated_ca).toBe(130);
  });

  it("computes the three-position cost example", () => {
    const estimate = estimateCaExact({
      attributes: ALL_65,
      positions: positions({ dl: 20, wbl: 20, ml: 20 }),
      position: "DL",
      left_foot: 100,
      right_foot: 50,
    });
    expect(estimate?.position_cost).toBe(8);
  });
});
