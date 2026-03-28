// FEAT-APP-001
// REQ-CORE-017

import { expect, test } from "@playwright/test";

test("renders top tabs and linked spec content", async ({ page }) => {
  await page.goto("/");

  const topLevelSections = page.getByRole("navigation", { name: "Top level sections" });

  await expect(page.getByRole("heading", { name: /^syu/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /^syu\b/i })).toBeVisible();
  await expect(topLevelSections.getByRole("button", { name: /^philosophy\b/i })).toBeVisible();
  await expect(topLevelSections.getByRole("button", { name: /^policies\b/i })).toBeVisible();
  await expect(topLevelSections.getByRole("button", { name: /^features\b/i })).toBeVisible();
  await expect(topLevelSections.getByRole("button", { name: /^requirements\b/i })).toBeVisible();
  await expect(page.getByText("Welcome to syu.")).toBeVisible();

  await page.getByRole("button", { name: "Dismiss welcome banner" }).click();
  await expect(page.getByText("Welcome to syu.")).toHaveCount(0);

  await page.reload();
  await expect(page.getByText("Welcome to syu.")).toHaveCount(0);

  await topLevelSections.getByRole("button", { name: /^features\b/i }).click();
  await page.getByRole("button", { name: /check\.yaml/i }).click();
  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
  await expect(page.getByText("SYU-workspace-load-001").first()).toBeVisible();

  await page.getByRole("button", { name: "REQ-CORE-001" }).click();
  await expect(
    page.getByRole("heading", {
      name: /REQ-CORE-001 .* Validate the linked specification graph with rule-backed diagnostics/i,
    }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "← Back" })).toBeVisible();

  await page.getByRole("button", { name: "← Back" }).click();
  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
});
