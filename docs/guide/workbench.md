# Workbench flow

Syu Workbench turns a request, specification Item, or branch diff into bounded,
human-readable implementation work. Run `syu workbench` to open the shared
Rust-native Dioxus UI used by the browser and desktop shells.

## Product flow

```text
Request / Item / branch diff
  -> specification graph and repository evidence
  -> Implementation Slices
  -> human-readable Work and Goal Plan
  -> assignment and execution
  -> goal-scoped evidence
  -> completion decision
```

Workbench is not an IDE, Kanban board, AI chat, or graphical copy of the CLI.
Purpose, outcome, boundaries, uncertainty, and completion conditions appear
before file paths, commands, JSON, or raw logs.

## Stable pages

The Role Sidebar contains exactly four pages in this order:

1. **Work** is the default. Brief explains the purpose, outcome, non-goals,
   warnings, confidence, and linked specifications. Scope explains the bounded
   Implementation Slices. Delivery keeps assignment, allowed and forbidden
   scope, tests, completion commands, and evidence requirements together.
   Evidence shows a goal-scoped timeline with raw attachments collapsed.
2. **Scope** starts with Code & Tests and groups branch-, Goal-, or Item-driven
   evidence into human-readable `ImplementationSlice` records. Each slice
   carries rationale, source, confidence, include/exclude boundaries, files,
   symbols, tests, specification IDs, ownership, evidence, and warnings.
3. **Items** remains the source of truth for Philosophy, Policy, Requirement,
   and Feature Items. New Items appear as drafts in the same Context Rail and
   Detail Canvas used for existing Items. Preview/apply preserves source,
   validates reciprocal links, and rejects stale source. An Item can start Work
   or an Item-driven scope review.
4. **Diagnostics** is an Explorer, not a launcher grid. Workspace, Goal Plan,
   Trace, and Repository tabs aggregate small accessible Status Circles. A
   single Run diagnostics action starts a job and updates checks through
   `/api/events` as queued, running, completed, or failed.

Settings is a full-page utility opened from the gear, never a fifth Role.
Its syu.yaml view offers structured fields, read-only raw YAML, schema and
semantic validation, a source-preserving diff, stale-source protection, and
Apply. Existing comments and unknown fields are retained.

## Explorer Frame

Pages use the lightest useful combination of Page Toolbar, Section Tabs,
Context Rail, and Detail Canvas. The hierarchy never exceeds Tabs -> Rail ->
Canvas. A Rail is omitted when only one meaningful selection exists. Detail
Canvas uses headings, separators, and at most two columns instead of nested
card stacks.

Status Circle meanings are consistent across pages: green success, orange
warning or inferred state, red error, blue running, and gray disabled. Every
circle has an accessible label, so state never depends on color alone.
Confidence, evidence source, scope, and ownership are explicit when values are
inferred.

## CommandTarget navigation

The Command Palette remains global, but it does not open or replace the page
with a command result. Every entry resolves to a typed `CommandTarget`:

```text
page + section + entity + component anchor + focus intent + execution policy
```

Navigation restores page, section, and entity through URL state. The target is
focused, scrolled into view, and outlined briefly with a red Focus Ring. The
ring disappears after three seconds and does not animate when reduced motion
is requested. Destructive actions only prepare the relevant page form; Enter
never performs them immediately.

Representative targets:

- show, browse, search -> Items search or selected Item;
- add -> Items draft editor;
- Item history -> Items history section;
- Work history -> Work / Evidence timeline;
- validate, doctor, audit, report -> Diagnostics;
- branch scope, trace, relate -> Scope / Code & Tests;
- request classify or plan -> Work create/Brief;
- assignment and agent run -> Work / Delivery;
- init and configuration -> Settings.

The old command-result page, legacy pane slugs, preview queries, generic result
lists, and Commands/Pulse compatibility views do not exist.

## Live state and APIs

The first HTML response is server-rendered Dioxus. Workspace snapshots, request
and Goal APIs, assignment and evidence models, job APIs, and SSE remain typed.
Diagnostics uses `POST /api/diagnostics/run`, `/api/jobs/*`, and
`/api/events`. Item-driven Work uses `POST /api/items/{id}/work` and records
`GoalPlanSourceMode::ItemDriven` plus the source Item ID. Settings uses
`/api/settings/preview` and `/api/settings/apply`.

Raw JSON and command output are evidence attachments, not the primary UI.

## Browser, desktop, and CI

Browser and desktop render the same `AppShell`, `WorkbenchPage`,
`CommandTarget`, Explorer components, page components, indicators, and
Tailwind asset. No React, Vite, TypeScript, or second frontend is introduced.

CI rebuilds the scoped Tailwind asset and tests page/section/entity restoration,
all palette mappings, Item draft and Item-driven flows, source-preserving
settings, diagnostics job transitions, accessibility labels, Rust-native server
routes, and desktop compilation.

## Model and migration terms

The Rust-native UI and Workbench server architecture still share the typed
`WorkbenchState`, `ActiveRequestState`, `ActiveGoalState`, and `AssignmentState`
domain models. Request Intake and Goal Splitter are creation concepts inside
Work, not persistent sidebar roles. A scaffold preview remains part of Work
creation, and a completion check remains the final evidence decision. This is
the same AppShell in browser and desktop; no separate client implementation is
maintained.
