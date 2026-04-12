// FEAT-APP-001
// REQ-CORE-017

import { expect, test } from "@playwright/test";

const usesFailingWorkspace =
  process.env.SYU_APP_E2E_WORKSPACE?.includes("tests/fixtures/workspaces/failing") ?? false;

type AppDataPayload = {
  validation: {
    issues: Array<{
      code: string;
      severity: "error" | "warning";
      subject: string;
      location: string | null;
      message: string;
      suggestion: string | null;
    }>;
  };
};

function swapDuplicateIssues(payload: AppDataPayload, code: string): AppDataPayload {
  const duplicateIndexes = payload.validation.issues
    .map((issue, index) => ({ issue, index }))
    .filter(({ issue }) => issue.code === code)
    .map(({ index }) => index);

  if (duplicateIndexes.length < 2) {
    throw new Error(`Expected at least two ${code} issues in the app payload.`);
  }

  const [firstIndex, secondIndex] = duplicateIndexes;
  const issues = [...payload.validation.issues];
  [issues[firstIndex], issues[secondIndex]] = [issues[secondIndex], issues[firstIndex]];

  return {
    ...payload,
    validation: {
      ...payload.validation,
      issues,
    },
  };
}

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
  await expect(page).toHaveURL(/#features\/FEAT-CHECK-001$/);
  await expect(page.getByText("SYU-workspace-load-001").first()).toBeVisible();

  await page.getByRole("button", { name: "REQ-CORE-001" }).click();
  await expect(
    page.getByRole("heading", {
      name: /REQ-CORE-001 .* Validate the linked specification graph with rule-backed diagnostics/i,
    }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#requirements\/REQ-CORE-001$/);
  await expect(page.getByRole("button", { name: "← Back" })).toBeVisible();

  await page.getByRole("button", { name: "← Back" }).click();
  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#features\/FEAT-CHECK-001$/);
});

test("loads deep links and supports keyboard search navigation", async ({ page }) => {
  await page.goto("/#/requirements/REQ-CORE-001");

  await expect(
    page.getByRole("heading", {
      name: /REQ-CORE-001 .* Validate the linked specification graph with rule-backed diagnostics/i,
    }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#requirements\/REQ-CORE-001$/);

  const searchInput = page.getByRole("searchbox", { name: "Search spec items" });
  await expect(
    page.getByText(
      /Tip: use ArrowUp and ArrowDown to move through results, Enter to open the highlighted item or the only result when there is one match, and Escape to clear the search\./,
    ),
  ).toBeVisible();
  await searchInput.fill("FEAT-CHECK-001");
  await searchInput.press("ArrowDown");
  await searchInput.press("ArrowUp");
  await searchInput.press("ArrowDown");
  await searchInput.press("Enter");

  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#features\/FEAT-CHECK-001$/);

  await searchInput.fill("no-such-result");
  await searchInput.press("ArrowUp");
  await searchInput.press("Enter");

  await expect(page.getByText("No items match.")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
});

test("keeps duplicate validation issues independently selectable", async ({ page }) => {
  test.skip(!usesFailingWorkspace, "requires the failing fixture workspace");

  await page.goto("/");

  const duplicateIssueRows = page.getByRole("button", { name: /SYU-trace-id-001/i });
  await expect(duplicateIssueRows).toHaveCount(2);
  const selectedIssue = page
    .getByRole("heading", { level: 3, name: "SYU-trace-id-001" })
    .locator("..");

  await duplicateIssueRows.nth(0).click();
  await expect(
    selectedIssue.getByText(
      "Declared implementation file `frontend/broken-feature.ts` does not mention `FEAT-FAIL-001`.",
    ),
  ).toBeVisible();
  await expect(selectedIssue.getByText("typescript:frontend/broken-feature.ts")).toBeVisible();

  await duplicateIssueRows.nth(1).click();
  await expect(
    selectedIssue.getByText(
      "Declared test file `src/broken_tests.rs` does not mention `REQ-FAIL-001`.",
    ),
  ).toBeVisible();
  await expect(selectedIssue.getByText("rust:src/broken_tests.rs")).toBeVisible();
});

test("keeps the selected validation issue stable across refresh reordering", async ({
  page,
  request,
}) => {
  test.skip(!usesFailingWorkspace, "requires the failing fixture workspace");

  await page.goto("/");

  const payloadResponse = await request.get("/api/app-data.json");
  expect(payloadResponse.ok()).toBeTruthy();

  const payload = (await payloadResponse.json()) as AppDataPayload;
  const reorderedPayload = swapDuplicateIssues(payload, "SYU-trace-id-001");

  const duplicateIssueRows = page.getByRole("button", { name: /SYU-trace-id-001/i });
  await expect(duplicateIssueRows).toHaveCount(2);

  const selectedIssue = page
    .getByRole("heading", { level: 3, name: "SYU-trace-id-001" })
    .locator("..");

  await duplicateIssueRows.nth(1).click();
  await expect(
    selectedIssue.getByText(
      "Declared test file `src/broken_tests.rs` does not mention `REQ-FAIL-001`.",
    ),
  ).toBeVisible();

  let refreshLoads = 0;
  await page.route("**/api/version", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ snapshot: "playwright-reordered-issues" }),
    });
  });
  await page.route("**/api/app-data.json", async (route) => {
    refreshLoads += 1;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      headers: {
        "x-syu-snapshot": "playwright-reordered-issues",
      },
      body: JSON.stringify(reorderedPayload),
    });
  });

  await expect.poll(() => refreshLoads, { timeout: 10000 }).toBeGreaterThan(0);
  await expect(
    selectedIssue.getByText(
      "Declared test file `src/broken_tests.rs` does not mention `REQ-FAIL-001`.",
    ),
  ).toBeVisible();
  await expect(
    selectedIssue.getByText(
      "Declared implementation file `frontend/broken-feature.ts` does not mention `FEAT-FAIL-001`.",
    ),
  ).toHaveCount(0);
});
