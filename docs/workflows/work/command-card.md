# mitase command card

<!-- FEAT-DOCS-001 -->

Keep this page open when you already know the v1 model and just need the
current command surface at a glance.

If you still need the first-run story, use [getting started](../../start-here/first-run/getting-started.md).
If the request is about work slicing or the Workbench product flow, pair this
page with [workbench](./workbench.md).

## Current command surface

| Task | Command | Choose it when |
| --- | --- | --- |
| Confirm the installed binary | `mitase --version` | verify the binary on your `PATH` before touching a workspace |
| Validate the current workspace | `mitase validate workspace .` | run the canonical v1 graph and work-plan validation pass |
| Validate a changed range | `mitase validate change . --range origin/main...HEAD` | check changed-file ownership and impact against a git range |
| Validate one work plan | `mitase validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>` | re-check a saved work plan against one exact execution boundary |
| Validate one verification result | `mitase validate result . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --attempt-id <attempt-id> --receipt receipt.yaml` | evaluate completion only from the exact durable verification attempt |
| Approve an exact work plan | `mitase task approve --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --workspace .` | persist explicit review before any verification attempt |
| Verify one work slice | `mitase task verify --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --workspace .` | append an immutable completion attempt, including failures and blockers |
| Inspect attempt history | `mitase task attempts list --workspace .` | recover current and previous attempts after retries or restarts |
| Finalize a complete attempt | `mitase task finalize preview/apply --attempt <attempt-id> --plan-digest <digest> --slice-id <slice-id> --workspace .` | revalidate and atomically promote only the exact planned slice |
| Plan executable work | `mitase work plan --request work.yaml --out plan.yaml --workspace .` | turn a v1 work request into canonical execution slices |
| Inspect a saved work plan | `mitase work show --plan plan.yaml` | read the exact YAML plan that the planner produced |
| Export one slice context pack | `mitase work export-context --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --workspace .` | materialize one validated slice into a portable context pack |
| Inspect the Workbench projection | `mitase workbench project --workspace . --format json` | view the current typed server payload without starting a UI |
| Inspect the Workbench projection for one request | `mitase workbench project --workspace . --request work.yaml --format yaml` | review how one work request appears through the current product projection |
| Start the editor protocol server | `mitase lsp` | connect an editor client over stdio when you want the current hover-only LSP surface |

## Common command bundles

### Validate the repository state

```bash
mitase validate workspace .
```

### Validate a proposed work plan

```bash
mitase work plan --request work.yaml --out plan.yaml --workspace .
mitase work show --plan plan.yaml
mitase validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
mitase task approve --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --workspace .
mitase task verify --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --workspace .
mitase task attempts list --workspace .
mitase task finalize preview --attempt <attempt-id> --plan-digest <digest> --slice-id <slice-id> --workspace .
mitase task finalize apply --attempt <attempt-id> --plan-digest <digest> --slice-id <slice-id> --workspace . --preview-token <preview-token>
```

### Export one execution slice

```bash
mitase work export-context --plan plan.yaml --plan-digest <digest> --slice-id <slice-id> --workspace .
```

### Inspect the current Workbench data

```bash
mitase workbench project --workspace . --format json
```

## Keep going

- Use [getting started](../../start-here/first-run/getting-started.md) for the install-to-validate flow.
- Use [examples and templates](../../start-here/adopt/examples-and-templates.md) when you want the
  checked-in starter and example matrix.
- Use [workbench](./workbench.md) when you want the request, slice, and
  projection story before wiring a client.
- Use [LSP guide](../../workflows/integrations/lsp.md) when you are connecting `mitase` to an editor client
  over stdio instead of using the checked-in VS Code extension directly.
- Use [configuration](../repository/configuration.md) when you need to tune workspace,
  validation, or planning limits.
- Use [troubleshooting](../repository/troubleshooting.md) when validation already fails and
  you need repair guidance instead of a command reminder.
