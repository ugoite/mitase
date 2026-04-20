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
| Scaffold a workspace | `syu init .` | create the default four-layer tree in the current directory |
| Scaffold with another starter | `syu init . --template rust-only` | begin from a language-shaped or docs-first layout instead of the generic starter |
| Check the workspace | `syu validate .` | run the full graph, trace, and coverage validation pass |
| Focus one validation view | `syu validate . --id FEAT-CHECK-001` | keep the visible output anchored on one requirement or feature after the normal validation run |
| Focus trace failures first | `syu validate . --genre trace` | inspect trace-specific problems before reading the full validation output |
| Generate the Markdown report | `syu report .` | save the current validation result as a shareable report |
| Inspect one spec item | `syu show FEAT-CHECK-001` | read the title, links, traces, and status for one philosophy, policy, requirement, or feature |
| Expand the nearby graph | `syu relate FEAT-CHECK-001` | see linked policies, requirements, features, files, and symbols around one selector |
| Jump from code to the owning spec | `syu trace src/command/check.rs --symbol run_check_command` | start in code and resolve the traced requirement and feature chain |
| List items by layer | `syu list feature` | print list-shaped output instead of the browser-style explorer |
| Search by keyword or ID | `syu search validation --kind feature` | find the right spec item before `show`, `relate`, or `log` |
| Review traced history | `syu log FEAT-CHECK-001 --kind implementation --path src/command` | inspect recent git history for the currently traced surface |
| Browse in the terminal | `syu browse .` | explore the graph interactively without leaving the shell |
| Browse in the browser | `syu app .` | use the local browser UI for visual navigation, tabs, and validation context |

## Common command bundles

### First workspace pass

```bash
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
- Use [configuration](./configuration.md) when you need validation and runtime
  switches instead of the default workflow.
- Use [troubleshooting](./troubleshooting.md) when validation already fails and
  you need repair guidance instead of a command reminder.
