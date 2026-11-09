import { expect, test } from "playwright/test";

test("player inspector shows other nationalities", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();

  await page.getByPlaceholder("Search players, clubs…").fill("Modrić");
  const row = page.locator("tbody tr", { hasText: "Luka Modrić" }).first();
  await expect(row).toBeVisible();
  await row.click();

  const inspector = page.locator("aside").last();
  await expect(inspector).toContainText("Other nationalities:");
  await expect(inspector).toContainText("Croatia");
});
