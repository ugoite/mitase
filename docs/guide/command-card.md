# syu command card

<!-- FEAT-DOCS-001 -->

Keep this page open when you already know the four-layer model and want the
fastest reminder of the core commands without hopping between multiple guides.

If you still need the first-run story, use [getting started](./getting-started.md).
If a pull request already exists, pair this page with the
[reviewer workflow](./reviewer-workflow.md).

## Core command chooser

| Task | Command | Choose it when |
| --- | --- | --- |
| Install or verify the CLI | `syu --version` | confirm the installed binary is on your `PATH` before you start editing a workspace |
| Compare starter layouts | `syu templates` | choose between docs-first, language-first, or polyglot scaffolds before `init` |
| Check local readiness | `syu doctor .` | confirm the Rust, Node, and browser-app dependencies are ready before you scaffold or validate |
| Scaffold a workspace | `syu init .` | create the default four-layer tree in the current directory |
| Scaffold with another starter | `syu init . --template rust-only` | begin from a language-shaped or docs-first layout instead of the generic starter |
| Check the workspace | `syu validate .` | run the full graph, trace, and coverage validation pass |
| Focus one validation view | `syu validate . --id FEAT-CHECK-001` | keep the visible output anchored on one requirement or feature after the normal validation run |
| Focus trace failures first | `syu validate . --genre trace` | inspect trace-specific problems before reading the full validation output |
| Audit a review target | `syu audit .` | scan for likely overlap, drift, and other review notes before you decide whether `validate` should fail |
| Generate the Markdown report | `syu report .` | save the current validation result as a shareable report |
| Inspect one spec item | `syu show FEAT-CHECK-001` | read the title, links, traces, and status for one philosophy, policy, requirement, or feature |
| Expand the nearby graph | `syu relate FEAT-CHECK-001` | see linked policies, requirements, features, files, and symbols around one selector |
| Review a PR range | `syu review --range origin/main...HEAD` | start with the affected philosophy, policy, requirement, and feature IDs, then drill into changed files with `show`, `relate`, or `log` |
| Strictly review a PR range | `syu review --range origin/main...HEAD --strict --allowed-id FEAT-CHECK-001 --format json` | fail the range review on unowned, ambiguously owned, or out-of-scope changes while keeping structured findings for CI |
| Guard a review range | `syu review --range origin/main...HEAD --allowed-id FEAT-CHECK-001` | block changes that step outside the named requirement or feature IDs and list the out-of-scope items |
| Draft a temporary goal plan | `syu task scaffold request.yaml` | preview a bounded delivery artifact with goal, scope, tests, coverage, and completion checks without adding a fifth persistent spec layer |
| Infer a goal plan from a diff | `syu task infer --range origin/main...HEAD` | derive a provisional Goal Plan from changed files, traced owners, evidence, and confidence before review |
| Check a temporary goal plan | `syu task check goal-plan.yaml --range origin/main...HEAD` | validate a temporary Goal Plan against the changed files, linked spec IDs, required tests, and completion commands before review |
| Jump from code to the owning spec | `syu trace src/command/check.rs --symbol run_check_command` | start in code and resolve the traced requirement and feature chain |
| List items by layer | `syu list feature` | print list-shaped output instead of the browser-style explorer |
| Search by keyword or ID | `syu search validation --kind feature` | find the right spec item before `show`, `relate`, or `log` |
| Review traced history | `syu log FEAT-CHECK-001 --kind implementation --path src/command` | inspect recent git history for the currently traced surface, or fall back to the historical index when an ID was deleted |
| Browse in the terminal | `syu browse .` | explore the graph interactively without leaving the shell |
| Browse in the browser | `syu app .` | use the local browser UI for visual navigation, tabs, and validation context |
| Start the editor protocol server | `syu lsp` | connect an editor client over stdio when you want the current hover-only LSP surface |

## Reviewer loop

| Task | Command | Choose it when |
| --- | --- | --- |
| Audit a review target | `syu audit .` | scan for overlap, tension, and orphaned-policy candidates before you hand findings back |
| Share the validation state | `syu report .` | render a Markdown snapshot the author can use to fix and re-run validation |

Use `syu audit .` before `syu report .` when you want the review handoff to read as one short loop instead of a loose list of checks.

If a reviewed diff includes an OpenAPI contract file, the range summary still
shows the owning requirements and features, separates direct matches from
indirect upstream/downstream context, and prints the traced method/path
selector so the change stays connected to the spec items that own the contract
operation.

## Common command bundles

### First workspace pass

```bash
syu doctor .
syu init .
syu validate .
syu browse .
```

### Reviewer loop

```bash
syu show FEAT-CHECK-001
syu relate FEAT-CHECK-001
syu trace src/command/check.rs --symbol run_check_command
syu log FEAT-CHECK-001 --kind implementation --path src/command
syu validate . --id FEAT-CHECK-001
syu audit .
syu report .
```

### Share the current state

```bash
syu validate .
syu report .
```

## Keep going

- Use [getting started](./getting-started.md) for the narrated install-to-validate flow.
- Use [examples and templates](./examples-and-templates.md) when you want the
  checked-in starter and example matrix.
- Use [goal plan format](./goal-plan-format.md) when you need a temporary
  delivery artifact instead of a persistent spec item.
- Use [LSP guide](./lsp.md) when you are connecting `syu` to an editor client
  over stdio instead of using the checked-in VS Code extension directly.
- Use [configuration](./configuration.md) when you need validation and runtime
  switches instead of the default workflow.
- Use [troubleshooting](./troubleshooting.md) when validation already fails and
  you need repair guidance instead of a command reminder.
