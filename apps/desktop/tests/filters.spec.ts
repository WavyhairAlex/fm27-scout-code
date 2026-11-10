import { expect, test } from "playwright/test";

test("gender All count includes players with unknown gender", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();

  const all = page.getByTestId("gender-all");
  await expect(all).toContainText("320");
  await expect(page.getByTestId("gender-unknown")).toContainText("1");

  await page.getByTestId("gender-unknown").click();
  await expect(page.getByText("1 players", { exact: true })).toBeVisible();
});
