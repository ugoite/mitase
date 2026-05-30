# Workbench flow

Use this guide when a request needs to become a goal-centered Workbench flow
instead of just another CLI invocation. The Workbench is command-palette-first
and should work the same way in browser and desktop contexts.

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
- an active Goal Plan makes `goal.test_select` available;
- a loaded branch scope makes `branch.infer_goal` available;
- mutating actions require confirmation metadata;
- AI-eligible actions require a bounded scope.

## Why it exists

The Workbench keeps large changes reviewable by making the request, goal,
assignment, and evidence story explicit. That lets a user split work without
losing the parent request, and it keeps the goal centered even when the
implementation happens across more than one session or client.
