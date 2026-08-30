# mitase command card

<!-- FEAT-DOCS-001 -->

Keep this page open when you already know the v1 model and just need the
current command surface at a glance.

If you still need the first-run story, use [getting started](../../start-here/first-run/getting-started.md).
The execution runtime documented here is transitional and belongs to external
repository tooling.

## Current command surface

| Task | Command | Choose it when |
| --- | --- | --- |
| Confirm the installed binary | `mitase --version` | verify the binary on your `PATH` before touching a workspace |
| Validate the current workspace | `mitase validate workspace .` | run the canonical v1 specification validation pass |
| Validate a changed range | `mitase validate change . --range origin/main...HEAD` | check changed-file ownership and impact against a git range |
| Start the editor protocol server | `mitase lsp` | connect an editor client over stdio when you want the current hover-only LSP surface |

## Common command bundles

### Validate the repository state

```bash
mitase validate workspace .
```

Planning, execution, retry, finalization, and validation of their execution
artifacts are repository-tooling concerns. The Mitase CLI does not expose
`work`, `task`, `validate plan`, or `validate result` commands.

## Keep going

- Use [getting started](../../start-here/first-run/getting-started.md) for the install-to-validate flow.
- Use [examples and templates](../../start-here/adopt/examples-and-templates.md) when you want the
  checked-in starter and example matrix.
- Use [LSP guide](../../workflows/integrations/lsp.md) when you are connecting `mitase` to an editor client
  over stdio instead of using the checked-in VS Code extension directly.
- Use [configuration](../repository/configuration.md) when you need to tune workspace,
  validation, or planning limits.
- Use [troubleshooting](../repository/troubleshooting.md) when validation already fails and
  you need repair guidance instead of a command reminder.
