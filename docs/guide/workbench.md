# Workbench flow

Use this guide when a request needs to become a goal-centered Workbench flow
instead of just another CLI invocation. Run `syu workbench` for the local
browser Workbench; its root page server-renders the shared Dioxus Workbench UI
with the same Tailwind asset used by desktop. The first screen is
command-palette-first, with the goal canvas and evidence rail already visible.

The browser and desktop clients work the same way through one Rust-native UI
and server architecture so the same request, goal, assignment, and evidence
flow stays visible in both places.

## Role-oriented navigation

The fixed header keeps Syu, the command palette, and settings available while
the left menu separates the Workbench into four persistent roles:

- **Items** shows philosophy, policy, requirement, and feature documents as a
  collapsible file tree. Linked items open in the same role. Item fields,
  links, tests, and implementations can be edited without renaming or moving
  IDs or files; every edit requires a source-preserving diff preview, and
  adjacent-layer reciprocal links are reviewed and applied together. When
  `docs/syu` does not exist yet, workspace initialization starts here.
- **Work** contains the pulse, request, goals, assignment, and evidence
  subviews for the goal-centered delivery flow.
- **Scope** contains branch scope and the spec impact graph.
- **Diagnostics** keeps validation, contributor doctor, specification audit,
  and Goal Plan check results distinct, and can refresh all available checks.

The command palette remains the primary action launcher. Selecting or running a
command routes to its owning role, so browse and add commands open Items,
scope and graph commands open Scope, and checks open Diagnostics.

## Workbench CI

CI validates the Workbench as a Rust-native architecture. The Workbench gate
runs focused package checks for the task model, actions, code intelligence,
Workbench state, Workbench server, Dioxus UI crate, and the no-default-features
desktop shell compile path. Full Tauri runtime checks remain local/platform
checks when system UI libraries are not available in CI.

Tailwind is constrained to `crates/syu-app-ui/`: `crates/syu-app-ui/tailwind.css`
is the source, `crates/syu-app-ui/assets/tailwind.css` is the served asset, and
CI uses the scoped Tailwind CLI to rebuild that asset and verify the build path.
It must not add a Vite, React, TypeScript, Playwright, or old browser-app
package setup for the Workbench. The installed-binary smoke starts `syu
workbench`, checks `/`, `/assets/tailwind.css`, `/api/health`, `/api/actions`,
and `/api/workspace/snapshot`, and verifies the shared CSS asset is loaded by
the Workbench shell.

The target product flow is:

```text
Request
  → classification
  → scope
  → scaffold preview
  → Goal Plan
  → assignment
  → execution
  → evidence
  → completion check
```

## Request Intake and Goal Splitter

Request Intake is the focused Workbench canvas for turning a plain text request
into a temporary Workbench planning artifact. The request flow should remain
goal-centered: the user can inspect classification, relevant workspace/spec
context, scope, and scaffold preview before generating a Goal Plan.

The Goal Splitter renders the generated Goal Plan as one or more reviewable Goal
cards. Each card should keep non-goals, linked persistent spec items, required
spec updates, include/exclude scope, implementation steps, required or suggested
tests, completion commands, and evidence expectations visible. These Goal Plans
are temporary Workbench planning artifacts under `.syu/workbench/requests/` and
`.syu/workbench/goals/` until the user explicitly exports or commits them.

Goal Plan YAML exported from the Workbench must stay compatible with
`syu task check`; the UI must not invent a separate Goal Plan format.

## What each step means

- Request: capture the user intent, constraints, and expected outcome.
- Classification: decide whether the request is a create, change, or delete.
- Scope: map the request onto the current spec graph, branch scope, and likely
  impact.
- Scaffold preview: show the planned spec changes before anything is applied.
- Goal Plan: keep the delivery plan temporary instead of turning it into a
  fifth persistent spec layer.
- Assignment: give the goal to a human or AI with explicit scope and evidence
  expectations.
- Execution: carry out the bounded work for the assigned goal.
- Evidence: attach proof that the work happened and still matches the request.
- Completion check: compare the result against the goal plan and required
  evidence before closing the loop.

The Workbench evidence rail keeps a goal-scoped timeline of typed evidence
records. Each record carries a status, source, timestamp, summary, and optional
attachment so the panel can show validation, test selection, goal checks, and
manual decisions without collapsing them into a single log blob.

## Spec Impact and Branch Scope

Spec Impact Graph shows how philosophy, policy, requirement, feature, changed
file or symbol, and test nodes connect. Branch Scope Lens shows the selected
Git range, changed files, traced owners, unowned files, ambiguous ownership,
out-of-scope files, affected spec IDs, suggested goal split, test
recommendations, and strict review status.

The action surface for this view is `branch.scope`, `branch.infer_goal`,
`spec.impact`, `trace.range`, and `relate.range`. Graph nodes, edges, badges,
and evidence states must use Workbench token names such as `spec-linked`,
`code-linked`, `test-linked`, `scope-in`, `scope-out`, `scope-ambiguous`,
`ownership-known`, `ownership-missing`, `ownership-ambiguous`,
`evidence-pass`, `evidence-warn`, `evidence-fail`, and `evidence-pending`.

## Command palette registry

The Workbench UI should render actions from a registry instead of hardcoding
workflow buttons. The command palette registry should be exposed by
`WorkbenchActionRegistry`, and the registry needs to expose title, description, required
state, input schema, mutability, risk, output event, evidence kind, and AI
eligibility so the same action list can drive both browser and desktop views.

The core state surface is:

- `WorkbenchState`
- `WorkspaceSnapshot`
- `ActiveRequestState`
- `ActiveGoalState`
- `GoalListState`
- `BranchScopeState`
- `EvidenceTimelineState`
- `AssignmentState`
- `JobState`
- `CommandPaletteState`

The registry should make these availability rules explicit:

- no request means `request.scope` is unavailable;
- an active request makes `request.classify` available;
- an active request makes `request.scope`, `request.scaffold`, and
  `request.plan` visible from the same command palette registry;
- an active Goal Plan makes `goal.test_select` available;
- an active Goal Plan makes `goal.check` available for evidence-ready review;
- a loaded branch scope makes `branch.infer_goal` available;
- mutating actions require confirmation metadata;
- AI-eligible actions require a bounded scope.

Palette commands are also classified by intent so the selected command opens
in a predictable result surface:

- **Browse** commands show search or context above a result list and detail;
- **Check** commands show the overall pass, warn, or fail result above a check
  list and selected detail;
- **Plan** commands show their input and generated proposals;
- **Change** commands keep input and confirmation above the execution result;
- **Operate** commands show runtime state, controls, and events;
- **Generate** commands show generation options and the resulting artifact.

Every action and CLI command must produce a typed result for one of these
surfaces. Raw command output is retained only as optional diagnostics.

## Why it exists

The Workbench keeps large changes reviewable by making the request, goal,
assignment, and evidence story explicit. That lets a user split work without
losing the parent request, and it keeps the goal centered even when the
implementation happens across more than one session or client.
