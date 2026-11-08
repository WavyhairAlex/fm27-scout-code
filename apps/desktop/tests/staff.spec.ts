import { expect, test } from "playwright/test";

test("staff view lists club staff and shows profile attributes and roles", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("fm27scout.language.v1", "en");
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open last save" }).click();
  await page.getByRole("button", { name: "Staff", exact: true }).click();

  await expect(page.getByRole("columnheader", { name: "Staff member" })).toBeVisible();
  const rows = page.locator("tbody tr");
  await expect(rows.first()).toBeVisible();

  await rows.first().click();
  const inspector = page.locator("aside").last();
  await expect(inspector.getByText("Staff profile", { exact: true })).toBeVisible();
  await expect(inspector.getByText("Profile attributes", { exact: true })).toBeVisible();
  await expect(inspector.getByText("Role suitability", { exact: true })).toBeVisible();
});
