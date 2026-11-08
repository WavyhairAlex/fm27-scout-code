// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { GrowthHistory } from "./Inspector";
import type { SnapshotPoint } from "@/lib/types";
import { useAppStore } from "@/store/app";

function buildPoints(count: number): SnapshotPoint[] {
  return Array.from({ length: count }, (_, index) => ({
    game_date: `2026-01-${String(index + 1).padStart(2, "0")}`,
    ca: 100 + index,
    pa: 150,
    transfer_value: null,
    club_uid: null,
    club_name: null,
    wage: null,
    contract_end: null,
  }));
}

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  useAppStore.setState({ language: "en" });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function renderHistory(count: number) {
  act(() => {
    root.render(<GrowthHistory points={buildPoints(count)} />);
  });
}

function clickButton(label: string) {
  const button = container.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
  expect(button).not.toBeNull();
  act(() => button!.click());
}

describe("GrowthHistory", () => {
  it("shows the ten newest snapshots with a page indicator", () => {
    renderHistory(25);

    expect(container.textContent).toContain("2026-01-25");
    expect(container.textContent).toContain("2026-01-16");
    expect(container.textContent).not.toContain("2026-01-15");
    expect(container.textContent).toContain("Page 1 of 3");
  });

  it("pages through the history ten rows at a time", () => {
    renderHistory(25);

    clickButton("Next page");
    expect(container.textContent).toContain("2026-01-15");
    expect(container.textContent).toContain("2026-01-06");
    expect(container.textContent).not.toContain("2026-01-05");
    expect(container.textContent).toContain("Page 2 of 3");

    clickButton("Next page");
    expect(container.textContent).toContain("2026-01-05");
    expect(container.textContent).toContain("2026-01-01");
    expect(container.textContent).toContain("Page 3 of 3");
    expect(
      container.querySelector<HTMLButtonElement>('button[aria-label="Next page"]')?.disabled,
    ).toBe(true);

    clickButton("Previous page");
    expect(container.textContent).toContain("Page 2 of 3");
  });

  it("hides the pager when the history fits on one page", () => {
    renderHistory(10);

    expect(container.querySelector('button[aria-label="Next page"]')).toBeNull();
    expect(container.querySelector('button[aria-label="Previous page"]')).toBeNull();
  });
});
