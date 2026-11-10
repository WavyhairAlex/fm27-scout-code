import { expect, test } from "playwright/test";

test("help dialog shows the current save player count", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "About & help" }).click();

  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText("Parser notes", { exact: true })).toBeVisible();
  await expect(dialog.getByText("320 players in this save", { exact: true })).toBeVisible();
});
