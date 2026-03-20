// FEAT-APP-001
// REQ-CORE-017

import { expect, test } from "@playwright/test";

test("renders top tabs and linked spec content", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "syu app" })).toBeVisible();
  await expect(page.getByRole("button", { name: /philosophy/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /policies/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /features/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /requirements/i })).toBeVisible();

  await page.getByRole("button", { name: /^features/i }).click();
  await expect(page.getByText("FEAT-CHECK-001")).toBeVisible();
  await expect(page.getByText("Unified validation command")).toBeVisible();

  await page.getByRole("button", { name: "REQ-CORE-001" }).click();
  await expect(
    page.getByText("Validate the linked specification graph with rule-backed diagnostics"),
  ).toBeVisible();
  await expect(page.getByText("SYU-workspace-load-001")).toBeVisible();
});
