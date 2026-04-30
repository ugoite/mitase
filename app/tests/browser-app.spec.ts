// FEAT-APP-001
// REQ-CORE-017

import { expect, test } from "@playwright/test";

const usesFailingWorkspace =
  process.env.SYU_APP_E2E_WORKSPACE?.includes("tests/fixtures/workspaces/failing") ?? false;

type AppDataPayload = {
  workspace_root?: string;
  source_documents: Array<{
    section: "philosophy" | "policies" | "features" | "requirements";
    path: string;
    content: string;
  }>;
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

function injectParseError(payload: AppDataPayload): { payload: AppDataPayload; path: string } {
  const target = payload.source_documents.find((document) => document.section === "philosophy");

  if (!target) {
    throw new Error("Expected a philosophy source document in the app payload.");
  }

  return {
    path: target.path,
    payload: {
      ...payload,
      source_documents: payload.source_documents.map((document) =>
        document.path === target.path
          ? { ...document, content: "category: Philosophy\nphilosophies: [" }
          : document,
      ),
    },
  };
}

type ValidationIssue = AppDataPayload["validation"]["issues"][number];

function requireFailingWorkspaceIssue(
  payload: AppDataPayload,
  expected: Pick<ValidationIssue, "location" | "message">,
): ValidationIssue {
  const issue = payload.validation.issues.find(
    (candidate) =>
      candidate.location === expected.location && candidate.message === expected.message,
  );

  if (!issue) {
    throw new Error(
      `Expected failing workspace issue for ${expected.location ?? "<no location>"}: ${
        expected.message
      }`,
    );
  }

  return issue;
}

function duplicateIssueCodeForFailingWorkspace(payload: AppDataPayload): string {
  const frontendIssue = requireFailingWorkspaceIssue(payload, {
    location: "typescript:frontend/broken-feature.ts",
    message: "Declared symbol `missingTsSymbol` was not found in `frontend/broken-feature.ts`.",
  });
  const rustIssue = requireFailingWorkspaceIssue(payload, {
    location: "rust:src/broken_tests.rs",
    message: "Declared symbol `missing_rust_symbol` was not found in `src/broken_tests.rs`.",
  });

  if (frontendIssue.code !== rustIssue.code) {
    throw new Error(
      `Expected duplicate failing-workspace issues to share a code, got ${frontendIssue.code} and ${rustIssue.code}.`,
    );
  }

  return frontendIssue.code;
}

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

test("repeats the YAML path inside parse-error banners", async ({ page }) => {
  let mutatedPath = "";

  await page.route("**/api/app-data.json", async (route) => {
    const response = await route.fetch();
    const payload = (await response.json()) as AppDataPayload;
    const mutated = injectParseError(payload);
    mutatedPath = mutated.path;
    await route.fulfill({
      response,
      body: JSON.stringify(mutated.payload),
    });
  });

  await page.goto("/");

  const banner = page
    .getByText("This document could not be parsed into the expected layer model.")
    .locator("..");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText(`File: ${mutatedPath}`);
  await expect(banner).toContainText("did not find expected node content");
});

test("renders top tabs and linked spec content", async ({ page }) => {
  await page.goto("/");

  const topLevelSections = page.getByRole("navigation", { name: "Top level sections" });

  await expect(page.getByRole("heading", { level: 1, name: /^syu\b/i })).toBeVisible();
  await expect(page.getByRole("button", { name: "syu — go to first item" })).toBeVisible();
  await expect(topLevelSections.getByRole("button")).toHaveText([
    /^philosophy\b/i,
    /^policies\b/i,
    /^requirements\b/i,
    /^features\b/i,
  ]);
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

test("scopes welcome-banner dismissal to the current workspace root", async ({ page }) => {
  let appDataRequests = 0;

  await page.route("**/api/app-data.json", async (route) => {
    appDataRequests += 1;

    const response = await route.fetch();
    const payload = (await response.json()) as AppDataPayload;
    const workspaceRoot =
      appDataRequests >= 2
        ? "/tmp/other-workspace"
        : (payload.workspace_root ?? "/tmp/current-workspace");

    await route.fulfill({
      response,
      body: JSON.stringify({
        ...payload,
        workspace_root: workspaceRoot,
      }),
    });
  });

  await page.goto("/");
  await expect(page.getByText("Welcome to syu.")).toBeVisible();

  await page.getByRole("button", { name: "Dismiss welcome banner" }).click();
  await expect(page.getByText("Welcome to syu.")).toHaveCount(0);

  await page.reload();
  await expect(page.getByText("Welcome to syu.")).toBeVisible();
});

test("renders remote-access warnings with bracketed IPv6 host literals", async ({ page }) => {
  await page.route("**/api/app-data.json", async (route) => {
    const response = await route.fetch();
    const payload = (await response.json()) as AppDataPayload & {
      app_server?: { bind: string; port: number; remotely_reachable: boolean };
    };

    await route.fulfill({
      response,
      body: JSON.stringify({
        ...payload,
        app_server: {
          ...(payload.app_server ?? { bind: "::1", port: 3000, remotely_reachable: true }),
          bind: "::1",
          port: 3000,
          remotely_reachable: true,
        },
      }),
    });
  });

  await page.goto("/");

  const banner = page.getByRole("alert");
  await expect(banner).toContainText("Remote access is enabled for this session.");
  await expect(banner).toContainText("http://[::1]:3000");
});

test("loads deep links and supports keyboard search navigation", async ({ page }) => {
  await page.goto("/#/requirements/REQ-CORE-001");

  await expect(
    page.getByRole("heading", {
      name: /REQ-CORE-001 .* Validate the linked specification graph with rule-backed diagnostics/i,
    }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#requirements\/REQ-CORE-001$/);

  const searchInput = page.getByRole("combobox", { name: "Search spec items" });
  await expect(searchInput).toHaveAttribute(
    "aria-describedby",
    "spec-search-shortcuts-description",
  );
  await expect(searchInput).toHaveAttribute("aria-autocomplete", "list");
  await expect(searchInput).toHaveAttribute("aria-expanded", "false");
  await expect(searchInput).not.toHaveAttribute("aria-controls", "spec-search-results-list");
  await expect(searchInput).toHaveAttribute(
    "placeholder",
    "Search items by ID or keyword (up to 20 matches)…",
  );
  const shortcutDescription = page.locator("#spec-search-shortcuts-description");
  await expect(shortcutDescription).toHaveText(
    "Keyboard shortcuts: ArrowDown and ArrowUp move through results, Enter opens the highlighted or only match, and Escape clears the search.",
  );
  const shortcutPanel = page.locator("#spec-search-shortcuts-panel");
  await expect(shortcutPanel).toBeVisible();
  await expect(shortcutPanel).toContainText("Search shortcuts");
  await expect(shortcutPanel).toContainText(
    "Keep focus in the search box and use the keyboard to move through results.",
  );
  await expect(shortcutPanel).toContainText("ArrowDown");
  await expect(shortcutPanel).toContainText("ArrowUp");
  await expect(shortcutPanel).toContainText("Enter");
  await expect(shortcutPanel).toContainText("Escape");
  await expect(
    page.getByText(
      "Search shows up to 20 matches at a time. Filter by layer or refine broad queries for a narrower result list.",
    ),
  ).toBeVisible();
  const filterGroup = page.getByRole("group", { name: "Search layer filters" });
  await expect(filterGroup.getByRole("button", { name: "All" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(filterGroup.getByRole("button", { name: "Requirements" })).toHaveAttribute(
    "aria-pressed",
    "false",
  );
  await searchInput.fill("REQ-CORE-001");
  await expect(
    page.getByText("Showing the first 20 matches — refine your query for fewer results."),
  ).toHaveCount(0);
  await searchInput.press("Escape");
  await expect(searchInput).toHaveValue("");
  await searchInput.fill("syu");
  await filterGroup.getByRole("button", { name: "Requirements" }).click();
  await expect(filterGroup.getByRole("button", { name: "Requirements" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(filterGroup.getByRole("button", { name: "All" })).toHaveAttribute(
    "aria-pressed",
    "false",
  );
  const requirementResults = page.locator('[role="option"]');
  await expect(requirementResults.first()).toBeVisible();
  await expect(
    page.locator('[role="option"] span').filter({ hasText: "requirements" }).first(),
  ).toBeVisible();
  await searchInput.press("Escape");
  await expect(searchInput).toHaveValue("");
  await expect(filterGroup.getByRole("button", { name: "All" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(filterGroup.getByRole("button", { name: "Requirements" })).toHaveAttribute(
    "aria-pressed",
    "false",
  );
  await expect(page.getByRole("listbox", { name: "Search results" })).toHaveCount(0);
  await searchInput.fill("syu");
  await filterGroup.getByRole("button", { name: "All" }).click();
  await expect(filterGroup.getByRole("button", { name: "All" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await searchInput.press("Escape");
  await expect(searchInput).toHaveValue("");
  await searchInput.fill("spec");
  await expect(
    page.getByText("Showing the first 20 matches — refine your query for fewer results."),
  ).toBeVisible();
  await searchInput.press("Escape");
  await expect(searchInput).toHaveValue("");
  await searchInput.fill("FEAT-CHECK-001");
  await expect(searchInput).toHaveAttribute("aria-expanded", "true");
  await expect(searchInput).toHaveAttribute("aria-controls", "spec-search-results-list");
  const searchResults = page.getByRole("listbox", { name: "Search results" });
  const firstSearchResult = searchResults.getByRole("option").first();
  await expect(firstSearchResult).toContainText("FEAT-CHECK-001");
  await searchInput.press("ArrowDown");
  const activeResultId = await searchInput.getAttribute("aria-activedescendant");
  expect(activeResultId).toBeTruthy();
  const activeResult = page.locator(`#${activeResultId!}`);
  await expect(activeResult).toContainText("FEAT-CHECK-001");
  await expect(activeResult).not.toHaveAttribute("aria-selected", "true");
  await searchInput.press("ArrowUp");
  await searchInput.press("ArrowDown");
  await searchInput.press("Enter");

  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#features\/FEAT-CHECK-001$/);

  await searchInput.fill("no-such-result");
  await expect(searchInput).toHaveAttribute("aria-expanded", "false");
  await expect(searchInput).not.toHaveAttribute("aria-controls", "spec-search-results-list");
  await searchInput.press("ArrowUp");
  await searchInput.press("Enter");

  await expect(page.getByText("No items match.")).toBeVisible();
  await searchInput.press("Escape");
  await expect(searchInput).toHaveValue("");
  await expect(searchInput).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("listbox", { name: "Search results" })).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: /FEAT-CHECK-001 .* Unified validation command/i }),
  ).toBeVisible();
});

test("explains requirement and feature trace metrics", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByRole("button", {
      name: /Requirement traces: Declared traces are the requirement test references written in the spec\./i,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: /Feature traces: Declared traces are the feature implementation references written in the spec\./i,
    }),
  ).toBeVisible();
});

test("keeps duplicate validation issues independently selectable", async ({ page, request }) => {
  test.skip(!usesFailingWorkspace, "requires the failing fixture workspace");

  await page.goto("/");

  const payloadResponse = await request.get("/api/app-data.json");
  expect(payloadResponse.ok()).toBeTruthy();

  const payload = (await payloadResponse.json()) as AppDataPayload;
  const duplicateIssueCode = duplicateIssueCodeForFailingWorkspace(payload);
  const duplicateIssueRows = page.getByRole("button", {
    name: new RegExp(duplicateIssueCode, "i"),
  });
  await expect(duplicateIssueRows).toHaveCount(2);
  const selectedIssue = page
    .getByRole("heading", { level: 3, name: duplicateIssueCode })
    .locator("..");

  await duplicateIssueRows.nth(0).click();
  await expect(
    selectedIssue.getByText(
      "Declared symbol `missingTsSymbol` was not found in `frontend/broken-feature.ts`.",
    ),
  ).toBeVisible();
  await expect(selectedIssue.getByText("typescript:frontend/broken-feature.ts")).toBeVisible();

  await duplicateIssueRows.nth(1).click();
  await expect(
    selectedIssue.getByText(
      "Declared symbol `missing_rust_symbol` was not found in `src/broken_tests.rs`.",
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
  const duplicateIssueCode = duplicateIssueCodeForFailingWorkspace(payload);
  const reorderedPayload = swapDuplicateIssues(payload, duplicateIssueCode);

  const duplicateIssueRows = page.getByRole("button", {
    name: new RegExp(duplicateIssueCode, "i"),
  });
  await expect(duplicateIssueRows).toHaveCount(2);

  const selectedIssue = page
    .getByRole("heading", { level: 3, name: duplicateIssueCode })
    .locator("..");

  await duplicateIssueRows.nth(1).click();
  await expect(
    selectedIssue.getByText(
      "Declared symbol `missing_rust_symbol` was not found in `src/broken_tests.rs`.",
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
      "Declared symbol `missing_rust_symbol` was not found in `src/broken_tests.rs`.",
    ),
  ).toBeVisible();
  await expect(
    selectedIssue.getByText(
      "Declared symbol `missingTsSymbol` was not found in `frontend/broken-feature.ts`.",
    ),
  ).toHaveCount(0);
});

test("shows a visible banner when version polling fails after the initial load", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1, name: /^syu\b/i })).toBeVisible();

  let pollAttempts = 0;
  await page.route("**/api/version", async (route) => {
    pollAttempts += 1;
    await route.fulfill({
      status: 500,
      contentType: "text/plain",
      body: "app data refresh failed",
    });
  });

  await expect.poll(() => pollAttempts, { timeout: 10000 }).toBeGreaterThan(0);

  const alert = page.getByRole("alert");
  await expect(alert).toContainText("Live refresh needs attention.");
  await expect(alert).toContainText("Showing the last successfully loaded workspace snapshot");
  await expect(alert).toContainText(
    "Could not check for workspace updates: Failed to poll app version: 500 Internal Server Error",
  );
  await expect(page.getByRole("heading", { level: 1, name: /^syu\b/i })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Workspace could not load" })).toHaveCount(0);
});

test("shows a visible banner when a workspace refresh reload fails after the initial load", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1, name: /^syu\b/i })).toBeVisible();

  let refreshLoads = 0;
  await page.route("**/api/version", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ snapshot: "playwright-refresh-error" }),
    });
  });
  await page.route("**/api/app-data.json", async (route) => {
    refreshLoads += 1;
    await route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({
        error: {
          code: "workspace-invalid",
          summary: "The workspace snapshot could not be rebuilt safely.",
          guidance:
            "Review recent workspace or syu.yaml changes, fix any broken files, then refresh again.",
        },
      }),
    });
  });

  await expect.poll(() => refreshLoads, { timeout: 10000 }).toBeGreaterThan(0);

  const alert = page.getByRole("alert");
  await expect(alert).toContainText("Live refresh needs attention.");
  await expect(alert).toContainText(
    "Could not reload the workspace snapshot: The workspace snapshot could not be rebuilt safely. Review recent workspace or syu.yaml changes, fix any broken files, then refresh again.",
  );
  await expect(page.getByRole("heading", { level: 1, name: /^syu\b/i })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Workspace could not load" })).toHaveCount(0);
});

test("allows a manual refresh and updates the last refresh timestamp after a stale snapshot banner", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1, name: /^syu\b/i })).toBeVisible();
  await expect(page.locator("header").getByLabel("Last successful refresh").first()).toBeVisible();

  const refreshTimestamp = page.getByLabel("Last successful refresh").first();
  const initialTimestamp = await refreshTimestamp.getAttribute("datetime");
  expect(initialTimestamp).not.toBeNull();

  let pollAttempts = 0;
  await page.route("**/api/version", async (route) => {
    pollAttempts += 1;
    await route.fulfill({
      status: 500,
      contentType: "text/plain",
      body: "app data refresh failed",
    });
  });

  let manualRefreshLoads = 0;
  await page.route("**/api/app-data.json", async (route) => {
    manualRefreshLoads += 1;
    await route.continue();
  });

  const alert = page.getByRole("alert");
  await expect.poll(() => pollAttempts, { timeout: 10000 }).toBeGreaterThan(0);
  await expect(alert).toContainText("Live refresh needs attention.");

  await page.waitForTimeout(20);
  await page.getByRole("button", { name: "Refresh now" }).first().click();

  await expect.poll(() => manualRefreshLoads, { timeout: 10000 }).toBeGreaterThan(0);
  await expect(alert).toHaveCount(0);
  await expect(refreshTimestamp).not.toHaveAttribute("datetime", initialTimestamp ?? "");
});
