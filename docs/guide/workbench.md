# Workbench

Syu Workbench is the browser projection of the canonical v1 flow:

`WorkRequest -> WorkPlan -> PlanApproval -> CompletionAttempt -> FinalizationReceipt`

Start it with a request:

```sh
syu workbench serve --workspace . --request request.yaml
```

The default address is `http://127.0.0.1:7737`. Use `--bind` and `--port` to
change the listener. `syu workbench project` remains available for JSON/YAML
projection debugging.

The server keeps one canonical workspace, inventory index, and static projection snapshot while repository content is unchanged. Warm reads and planning actions reuse that snapshot; tracked, staged, deleted, renamed, and untracked content changes invalidate it before the next operation. The server returns the browser shell immediately, the browser fetches the canonical projection once while showing startup progress, renders only the visible page, and shows an accessible busy state while actions are running.

Work is a guided journey for people who do not need to know repository
internals: choose a target specification, review the relevant criterion,
approve a bounded change, follow implementation, inspect evidence, and confirm
completion. The server supplies one next action at a time and explains why it
is available. Scope, diagnostics, identifiers, paths, selectors, and commands
are available only from Advanced details. Settings remains a utility page.

- Work starts from Specifications. Choose a requirement criterion and use its
  Create Work action; Work then separates advisory suggestions from the
  approved executable boundary and offers a concrete recovery action when work
  is blocked.
- After a criterion is selected, Work keeps its parent specification and exact
  criterion visible once in a read-only desktop split. Narrow layouts reduce
  the same context to one collapsed heading until the user opens it.
- Scope explains why a change is needed, which specification anchors support
  it, and only then the exact editable, verification, readonly, and derived
  generated targets. Contract counterparts appear as dependency-aware readonly
  context. Generated outputs can change only when their exact source target is
  editable and changed in the same slice.
- Items projects typed anchors, bindings, targets, and contracts from
  `syu/spec/v1`.
- Diagnostics projects canonical diagnostics by validation context and phase.
- Settings projects `syu/config/v1`; parsing and validation remain server-side.

The UI never parses YAML or infers ownership, contracts, target scope, or
validation meaning. Those decisions belong to the workspace, planner, and
validation crates and arrive through `syu-workbench-server`.

Interactive Work operations use the typed `/api/work/action` journey endpoint.
The browser and native WebView submit the same action and mutation basis; the
server returns the refreshed canonical projection. Cancelling a journey clears
the uncommitted session but never silently reverts file changes already made by
an implementation agent.

Supporting operations are backed by local server endpoints:

- WorkRequest edit/replan and Item-based Work creation run the canonical
  planner.
- Context export calls canonical `export_context` and refuses stale or
  non-ready plans.
- Plan approval is explicit and persisted outside the worktree. Verification
  appends immutable attempts to the shared store, so failed retries and history
  survive server restarts and are shared with the CLI.
- Finalization is a separate preview/apply handoff. It revalidates the attempt
  against the current workspace and changes only the exact planned slice before
  recording a `FinalizationReceipt`.
- Workspace, Git range, Work plan, and Slice validation call the shared
  validator synchronously.
- Item and config writes require a preview, strict schema validation, and a
  matching source hash. Apply rolls back when the resulting workspace cannot
  be indexed.
- Specifications can be found through typed candidate search across items and
  nested principles, rules, and criteria. The editor can update human-facing
  fields or create a planned Requirement/Feature in an existing specification
  document. Preview reports graph, ownership, readiness, target, test, and
  active-work impact before the exact token and source hash permit apply.
- Each criterion can request ranked implementation, verification,
  documentation, enforcement, and contract candidates. Every candidate shows
  its exact target, confidence, and evidence. Suggestions stay advisory until
  selected targets are approved; rejection is remembered for the reviewed
  evidence, and configured budget overflow asks the user to split the work.
- Item and Work editors show human-facing fields first. Exact anchors,
  bindings, contracts, selectors, and planning budgets remain editable under
  collapsed advanced settings and are preserved even when left collapsed.
- The server rejects paths outside the workspace and remote binding unless
  `--allow-remote-bind` is explicit.

For automation or debugging, use the projection without starting the server:

```sh
syu workbench project --workspace . --request request.yaml --format json
```
