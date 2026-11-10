import { expect, test } from "playwright/test";

test("analysis centre follows the managed club", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Analysis", exact: true }).click();

  const dialog = page.getByRole("dialog");
  await expect(dialog.getByRole("button", { name: "Wonderkid radar" })).toHaveCount(0);
  await expect(page.getByTestId("analysis-managed-club")).toContainText(
    "Casciavit Milano",
  );

  await dialog.getByRole("button", { name: "Squad depth" }).click();
  await expect(dialog.getByText("Casciavit Milano · sorted by CA")).toBeVisible();
});
