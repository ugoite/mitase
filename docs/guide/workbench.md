# Workbench flow

Use this guide when a request needs to become a goal-centered Workbench flow
instead of just another CLI invocation. Run `syu workbench` for the local
Workbench server entrypoint; the Workbench is command-palette-first and should
work the same way in browser and desktop contexts.

The browser and desktop clients should share one Rust-native UI and server architecture so the same flow stays visible in both places.

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

## Why it exists

The Workbench keeps large changes reviewable by making the request, goal,
assignment, and evidence story explicit. That lets a user split work without
losing the parent request, and it keeps the goal centered even when the
implementation happens across more than one session or client.
