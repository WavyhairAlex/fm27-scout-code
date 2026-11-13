import { expect, test } from "playwright/test";

test("analysis centre opens on the managed club squad", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Analysis", exact: true }).click();

  const dialog = page.getByRole("dialog");
  await expect(dialog.getByRole("button", { name: "Wonderkid radar" })).toHaveCount(0);
  await expect(dialog.getByRole("button", { name: "Needs", exact: true })).toHaveCount(0);
  await expect(dialog.getByRole("button", { name: "Squad depth" })).toHaveCount(0);
  await expect(dialog.getByRole("button", { name: "Overview", exact: true })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Casciavit Milano" })).toHaveCount(0);
  await expect(
    dialog.getByText("Casciavit Milano · grouped by squad · sorted by CA"),
  ).toBeVisible();
});

test("overview switches squads with tabs", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Analysis", exact: true }).click();

  const dialog = page.getByRole("dialog");
  const tabs = dialog.getByTestId("squad-tab");
  await expect(tabs).toHaveCount(4);
  await expect(tabs.nth(0)).toContainText("All");
  await expect(tabs.nth(1)).toContainText("First team");
  await expect(tabs.nth(2)).toContainText("U20");
  await expect(tabs.nth(3)).toContainText("U18");
  await expect(tabs.nth(2)).toHaveAttribute(
    "title",
    /Italian U20 Primavera 1 · 38 fixtures/,
  );

  const rows = dialog.getByTestId("squad-row");
  await expect(tabs.nth(1)).toHaveAttribute("aria-pressed", "true");
  const firstTeamSlots = await rows.evaluateAll((elements) =>
    elements.map((element) => element.getAttribute("data-squad-slot")),
  );
  expect(firstTeamSlots.length).toBeGreaterThan(0);
  expect(firstTeamSlots.every((slot) => slot === "0")).toBe(true);

  await tabs.nth(0).click();
  const allCount = await rows.count();
  expect(allCount).toBeGreaterThan(firstTeamSlots.length);

  await tabs.nth(2).click();
  await expect(tabs.nth(2)).toHaveAttribute("aria-pressed", "true");
  const slots = await rows.evaluateAll((elements) =>
    elements.map((element) => element.getAttribute("data-squad-slot")),
  );
  expect(slots.length).toBeGreaterThan(0);
  expect(slots.length).toBeLessThan(allCount);
  expect(slots.every((slot) => slot === "1")).toBe(true);

  await tabs.nth(0).click();
  await expect(rows).toHaveCount(allCount);
});

test("overview squad sorts by CA and PA", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Analysis", exact: true }).click();

  const dialog = page.getByRole("dialog");
  const rows = dialog.getByTestId("squad-row");
  const column = async (index: number) => {
    const values: number[] = [];
    for (let row = 0; row < (await rows.count()); row += 1) {
      const text = await rows.nth(row).getByRole("cell").nth(index).textContent();
      const value = Number(text?.trim());
      if (Number.isFinite(value)) values.push(value);
    }
    return values;
  };

  await dialog.getByRole("button", { name: /^CA/ }).click();
  const caAscending = await column(3);
  expect(caAscending.length).toBeGreaterThan(1);
  expect(caAscending).toEqual([...caAscending].sort((left, right) => left - right));

  await dialog.getByRole("button", { name: /^PA/ }).click();
  const paDescending = await column(4);
  expect(paDescending.length).toBeGreaterThan(1);
  expect(paDescending).toEqual([...paDescending].sort((left, right) => right - left));
});

test("opening a player from analysis switches to the players view", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Clubs", exact: true }).click();
  await page.getByRole("button", { name: "Analysis", exact: true }).click();

  const dialog = page.getByRole("dialog");
  await dialog.getByTestId("squad-row").first().click();

  await expect(dialog).toHaveCount(0);
  await expect(page.getByPlaceholder("Search players, clubs…")).toBeVisible();
  await expect(page.getByText("Player profile")).toBeVisible();
});

test("scouting advises the managed club", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Analysis", exact: true }).click();

  const dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "Scouting" }).click();
  const scout = page.getByTestId("scout-panel");
  await expect(scout).toBeVisible();
  await expect(scout.getByText("In your squad", { exact: true })).toBeVisible();
  await expect(scout.getByText(/rated here/)).toBeVisible();

  await scout.getByTestId("scout-view-depth").click();
  await expect(scout.getByText(/best XI level/)).toBeVisible();

  await scout.getByTestId("scout-view-departures").click();
  await expect(scout.getByText(/most pressing first/)).toBeVisible();
});
