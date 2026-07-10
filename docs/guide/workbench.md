# Workbench

Syu Workbench is the browser projection of the canonical v1 flow:

`WorkRequest -> WorkPlan -> ExecutionSlice -> ContextPack -> ValidationResult`

Start it with a request:

```sh
syu workbench serve --workspace . --request request.yaml
```

The default address is `http://127.0.0.1:7737`. Use `--bind` and `--port` to
change the listener. `syu workbench project` remains available for JSON/YAML
projection debugging.

The persistent roles are Work, Scope, Items, and Diagnostics. Settings is a
utility page. Work is the default page. The command palette navigates to the
owning page and focuses its control; it does not create generic result pages.

- Work presents the request, plan status, isolated execution slices, context,
  and plan validation. When no Work is selected, it starts from a branch
  change, a specification item, or a plain-language description.
- Scope explains why a change is needed, which specification anchors support
  it, and only then the exact editable, verification, and readonly targets.
- Items projects typed anchors, bindings, targets, and contracts from
  `syu/spec/v1`.
- Diagnostics projects canonical diagnostics by validation context and phase.
- Settings projects `syu/config/v1`; parsing and validation remain server-side.

The UI never parses YAML or infers ownership, contracts, target scope, or
validation meaning. Those decisions belong to the workspace, planner, and
validation crates and arrive through `syu-workbench-server`.

Interactive operations are backed by local server endpoints:

- WorkRequest edit/replan and Item-based Work creation run the canonical
  planner.
- Context export calls canonical `export_context` and refuses stale or
  non-ready plans.
- Workspace, Git range, Work plan, and Slice validation call the shared
  validator synchronously.
- Item and config writes require a preview, strict schema validation, and a
  matching source hash. Apply rolls back when the resulting workspace cannot
  be indexed.
- Item and Work editors show human-facing fields first. Exact anchors,
  bindings, contracts, selectors, and planning budgets remain editable under
  collapsed advanced settings and are preserved even when left collapsed.
- The server rejects paths outside the workspace and remote binding unless
  `--allow-remote-bind` is explicit.

For automation or debugging, use the projection without starting the server:

```sh
syu workbench project --workspace . --request request.yaml --format json
```
