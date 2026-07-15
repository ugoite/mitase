# syu command card

<!-- FEAT-DOCS-001 -->

Keep this page open when you already know the v1 model and just need the
current command surface at a glance.

If you still need the first-run story, use [getting started](./getting-started.md).
If the request is about work slicing or the Workbench product flow, pair this
page with [workbench](./workbench.md).

## Current command surface

| Task | Command | Choose it when |
| --- | --- | --- |
| Confirm the installed binary | `syu --version` | verify the binary on your `PATH` before touching a workspace |
| Validate the current workspace | `syu validate workspace .` | run the canonical v1 graph and work-plan validation pass |
| Validate a changed range | `syu validate change . --range origin/main...HEAD` | check changed-file ownership and impact against a git range |
| Validate one work plan | `syu validate plan . --plan plan.yaml` | re-check a saved work plan against the current workspace |
| Complete one work slice | `syu validate result . --plan plan.yaml --receipt receipt.yaml` | prove the receipt, exact tests, lifecycle, scope, and readiness close the selected slice |
| Plan executable work | `syu work plan --request work.yaml --out plan.yaml --workspace .` | turn a v1 work request into canonical execution slices |
| Inspect a saved work plan | `syu work show --plan plan.yaml` | read the exact YAML plan that the planner produced |
| Export one slice context pack | `syu work export-context --plan plan.yaml --slice <slice-id> --workspace .` | materialize one validated slice into a portable context pack |
| Inspect the Workbench projection | `syu workbench project --workspace . --format json` | view the current typed server payload without starting a UI |
| Inspect the Workbench projection for one request | `syu workbench project --workspace . --request work.yaml --format yaml` | review how one work request appears through the current product projection |
| Start the editor protocol server | `syu lsp` | connect an editor client over stdio when you want the current hover-only LSP surface |

## Common command bundles

### Validate the repository state

```bash
syu validate workspace .
```

### Validate a proposed work plan

```bash
syu work plan --request work.yaml --out plan.yaml --workspace .
syu work show --plan plan.yaml
syu validate plan . --plan plan.yaml
syu work verify --plan plan.yaml --slice <slice-id> --workspace . --out receipt.yaml
syu validate result . --plan plan.yaml --receipt receipt.yaml --format text
```

### Export one execution slice

```bash
syu work export-context --plan plan.yaml --slice <slice-id> --workspace .
```

### Inspect the current Workbench data

```bash
syu workbench project --workspace . --format json
```

## Keep going

- Use [getting started](./getting-started.md) for the install-to-validate flow.
- Use [examples and templates](./examples-and-templates.md) when you want the
  checked-in starter and example matrix.
- Use [workbench](./workbench.md) when you want the request, slice, and
  projection story before wiring a client.
- Use [LSP guide](./lsp.md) when you are connecting `syu` to an editor client
  over stdio instead of using the checked-in VS Code extension directly.
- Use [configuration](./configuration.md) when you need to tune workspace,
  validation, or planning limits.
- Use [troubleshooting](./troubleshooting.md) when validation already fails and
  you need repair guidance instead of a command reminder.
