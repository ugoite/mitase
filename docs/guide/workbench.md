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
  and plan validation.
- Scope explains exact editable, verification, and readonly `PlannedTarget`s.
- Items projects typed anchors, bindings, targets, and contracts from
  `syu/spec/v1`.
- Diagnostics projects canonical diagnostics by validation context and phase.
- Settings projects `syu/config/v1`; parsing and validation remain server-side.

The UI never parses YAML or infers ownership, contracts, target scope, or
validation meaning. Those decisions belong to the workspace, planner, and
validation crates and arrive through `syu-workbench-server`.

For automation or debugging, use the projection without starting the server:

```sh
syu workbench project --workspace . --request request.yaml --format json
```
