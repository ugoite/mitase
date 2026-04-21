# syu

<!-- FEAT-DOCS-001 -->

`syu` is a Rust CLI for specification-driven development.

It keeps machine-readable `philosophy`, `policy`, `requirements`, and `feature`
YAML definitions connected to real repository work: validation, implementation
ownership, maintenance, contributor workflow, and release delivery.

The design goal is intentionally pragmatic:

- specification-driven development that keeps looking after implementation and maintenance
- a language-agnostic model that can fit Rust-only, Python-only, Ruby-only, or polyglot repositories
- a simple, low-friction workflow that does not need to take over the whole project

## Why four layers?

`syu` treats the specification as a hierarchy of intent:

- `philosophy`: the ideal end-state and the values the project protects
- `policy`: the concrete rules that make philosophy operational
- `requirements`: the specific obligations that satisfy policy
- `features`: the implemented capabilities that satisfy requirements

This separation keeps teams from jumping straight to code without agreeing on
why the code should exist, what rules govern it, and how success will be
verified.

## Choose your path

Pick the newcomer path that matches what you need next:

- **Getting started**: choose this if you are new to `syu`, want a guided first
  run, and do **not** already know the four-layer model. Expect the most
  hand-holding and roughly 10-15 minutes for the first workspace setup.
- **Quick start**: stay in this README when you want a compact, self-contained
  reference card, are happy with a short layer refresher, and want the fastest
  install-to-`syu validate .` path in about 5 minutes.
- **Command card**: open [`docs/guide/command-card.md`](docs/guide/command-card.md)
  when you already know the model and want one compact guide page for the core
  install / init / validate / browse / app / review commands.
- **Tutorial**: follow [`docs/guide/tutorial.md`](docs/guide/tutorial.md) when you
  learn best from a realistic repository story, want more narrative context than
  Quick start, and do not mind a longer walkthrough.
- **Editor-first path**: start with
  [`docs/guide/vscode-extension.md`](docs/guide/vscode-extension.md) when you
  want diagnostics, spec navigation, and trace lookups inside VS Code before
  you memorize the CLI or browser flows.
- **Direct LSP path**: open [`docs/guide/lsp.md`](docs/guide/lsp.md) when you
  want the `syu lsp` stdio contract, current hover-only capability set, or a
  non-VS Code editor starting point.
- **Migration / upgrade**: open [`docs/guide/migration.md`](docs/guide/migration.md)
  when you already have a `syu` workspace and want the release-specific steps
  for moving between alpha versions safely.
- **Visual explorer**: start with [`docs/guide/app.md`](docs/guide/app.md) or run
  `syu app .` when you want graphical spec navigation before learning the full
  text-first CLI flow.
- **Reviewer workflow**: open
  [`docs/guide/reviewer-workflow.md`](docs/guide/reviewer-workflow.md) when a
  PR already exists and you want one concrete loop for moving between spec IDs,
  traced code, and recent Git history.
- **Contributor runtime setup**: open
  [`docs/guide/node-workflow.md`](docs/guide/node-workflow.md) when you are
  contributing to this repository and need one place to see which Node major
  the browser app, docs site, and VS Code extension expect today.
<!-- FEAT-DOCTOR-001 -->
- **Local readiness check**: run `syu doctor .` when you want one CLI summary of
  the current Rust toolchain, Node/npm expectations, optional dependency
  installs, and Playwright browser readiness before the contributor checks.
- **Trace adapter matrix**: open
  [`docs/guide/trace-adapter-support.md`](docs/guide/trace-adapter-support.md)
  when you already have a workspace and need a capability reference for which
  built-in languages support symbol validation only versus richer
  `doc_contains` and strict coverage checks.
- **Troubleshooting**: jump to
  [`docs/guide/troubleshooting.md`](docs/guide/troubleshooting.md) when validation
  or traceability errors are already blocking you and you want the shortest path
  to unblocking an existing workspace instead of following the onboarding flow.

Keep the detailed guides close:

- [`docs/guide/concepts.md`](docs/guide/concepts.md)
- [`docs/guide/getting-started.md`](docs/guide/getting-started.md)
- [`docs/guide/command-card.md`](docs/guide/command-card.md)
- [`docs/guide/tutorial.md`](docs/guide/tutorial.md)
- [`docs/guide/migration.md`](docs/guide/migration.md)
- [`docs/guide/app.md`](docs/guide/app.md)
- [`docs/guide/reviewer-workflow.md`](docs/guide/reviewer-workflow.md)
- [`docs/guide/node-workflow.md`](docs/guide/node-workflow.md)
- [`docs/guide/lsp.md`](docs/guide/lsp.md)
- [`docs/guide/trace-adapter-support.md`](docs/guide/trace-adapter-support.md)
- [`docs/guide/configuration.md`](docs/guide/configuration.md)
- [`docs/guide/spec-antipatterns.md`](docs/guide/spec-antipatterns.md)
- [`docs/guide/troubleshooting.md`](docs/guide/troubleshooting.md)
- [`docs/guide/vscode-extension.md`](docs/guide/vscode-extension.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)

## Install from published releases

`syu` publishes a release-hosted installer entrypoint and repository-scoped
GHCR packages under `ghcr.io/ugoite/syu`. Download the installer from the
current CLI release; the script prefers the matching package artifact and falls
back to GitHub release assets if anonymous package pulls are not available yet.

For security-sensitive environments, prefer the verified download flow below
before running the installer script.

### Recommended: verify before running

Each release publishes a `checksums.sha256` file alongside the installer.
Download both files, verify the checksum, then run the local copy:

```bash
RELEASE=v0.0.1-alpha.8
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" -o install-syu.sh
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/checksums.sha256" -o checksums.sha256
sha256sum --ignore-missing -c checksums.sha256
bash install-syu.sh
```

On macOS, replace `sha256sum` with `shasum -a 256`.

### Windows: PowerShell by default, Git Bash on Windows, WSL as Linux

If you are on Windows and want the clearest first-party path, use PowerShell
and download the Windows archive directly. This avoids requiring Git Bash or
WSL just to install `syu`.

```powershell
$release = 'v0.0.1-alpha.8'
$asset = 'syu-x86_64-pc-windows-msvc.zip'
$checksums = 'checksums.sha256'
Invoke-WebRequest "https://github.com/ugoite/syu/releases/download/$release/$asset" -OutFile $asset
Invoke-WebRequest "https://github.com/ugoite/syu/releases/download/$release/$checksums" -OutFile $checksums
$expected = (
  Get-Content $checksums |
  Select-String -SimpleMatch $asset |
  ForEach-Object { $_.Line.Split()[0].ToLower() }
)
if (-not $expected) { throw "Checksum for $asset not found" }
$actual = (Get-FileHash $asset -Algorithm SHA256).Hash.ToLower()
if ($actual -ne $expected) { throw 'Checksum mismatch' }
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\syu\bin'
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
Expand-Archive $asset -DestinationPath $installDir -Force
& (Join-Path $installDir 'syu.exe') --version
```

If a new PowerShell session still cannot find `syu`, add
`$env:LOCALAPPDATA\Programs\syu\bin` to your user `PATH`.

If you prefer the Bash installer on Windows, run `install-syu.sh` from Git Bash.
In that shell, the installer resolves the same `x86_64-pc-windows-msvc` archive
and defaults to `%LOCALAPPDATA%\Programs\syu\bin`.

If you are inside WSL, use the Linux installer path instead. There, the script
resolves the Linux archive and installs to `~/.local/bin`.

The checksum step above verifies the installer script itself. Published release
archives also carry GitHub artifact attestations, so you can separately verify
the platform archive that the installer downloads before it unpacks the binary.
For a stronger authenticity check, download the archive you expect to install
and pin the verification to this repository's release workflow plus the tag you
selected:

```bash
RELEASE="$(git describe --tags --abbrev=0)"
gh release download "${RELEASE}" --repo ugoite/syu --pattern 'syu-x86_64-unknown-linux-gnu.tar.gz'
gh attestation verify syu-x86_64-unknown-linux-gnu.tar.gz \
  --repo ugoite/syu \
  --signer-workflow ugoite/syu/.github/workflows/release-artifacts.yml \
  --source-ref "refs/tags/${RELEASE}" \
  --format json
```

Replace the archive name with the platform asset you plan to install
(`syu-x86_64-pc-windows-msvc.zip`, `syu-aarch64-apple-darwin.tar.gz`, and so
on). `--format json` lets you keep the verified certificate and provenance
details for your own policy checks.

### Shortcut: run the installer directly

If you already trust the release source and want the shortest path, use the
one-line entrypoint:

The download URL stays pinned to the installer shipped with this checked-in
documentation version. Use `SYU_VERSION=alpha`, `stable`, or an explicit
version selector when you want that installer to fetch a different published
package track after it starts.

If you jump straight to this section, set the checked-in release tag once before
using any of these shortcuts:

```bash
RELEASE=v0.0.1-alpha.8
```

Current installer entrypoint:

```bash
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | bash
```

During the alpha phase, prefer the `alpha` track selector so the same installer
entrypoint always resolves to the latest published alpha:

```bash
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_VERSION=alpha bash
```

Install to a custom directory:

```bash
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_INSTALL_DIR=$HOME/bin bash
```

Install a specific prerelease:

```bash
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_VERSION="$RELEASE" bash
```

If you're contributing to `syu` itself from source, jump to
[Contributing and local development](#contributing-and-local-development)
below. The rest of this section assumes you installed the published CLI.

## Quick start

Stay in this README for the shortest install-to-validate path. If you skipped
[`docs/guide/concepts.md`](docs/guide/concepts.md), use the
[Why four layers?](#why-four-layers) section above as the refresher on
`philosophy`, `policy`, `requirements`, and `features`. There is no other
required pre-read before the commands below.
If you want the same flow narrated step by step with more context around the
manual edits, switch to
[`docs/guide/getting-started.md`](docs/guide/getting-started.md) instead.

The first manual edit in this quick start happens in the generated requirement
YAML: add `linked_policies:` and `linked_features:` there, then update the
adjacent policy and feature YAML so they add the reciprocal
`linked_requirements:` entry back to the new requirement.

Read [`docs/guide/concepts.md`](docs/guide/concepts.md) first only when you want
the fuller rationale and authoring guidance before continuing.

Step 0: required — run `syu init .` before any of the other commands in a new repository.

```bash
syu init .                           # 1. Create spec scaffold
syu add requirement REQ-AUTH-001     # 2. Generate a requirement stub
```

`syu add requirement` prints the generated requirement path, the reciprocal-link
follow-up you still need, and matching `syu add ...` suggestions for adjacent
stubs before validation will pass.

Before step 3, open the generated requirement YAML under `docs/syu/requirements/`
(or your configured `spec.root`), add at least one `linked_policies:` entry and
one `linked_features:` entry, scaffold any still-missing adjacent policy or
feature document with the suggested commands, then update those policy and
feature documents so they link back to the new requirement.

```bash
syu validate .                       # 3. Check everything is linked
syu app .                            # 4. Browse in the browser
```

Use `syu browse .` when you want terminal-first exploration, or `syu app .`
when you want the local browser UI.

```bash
syu init .
syu validate .
syu validate . --fix
syu browse .
syu add feature FEAT-AUTH-LOGIN-001 --kind auth
syu list requirement
syu show REQ-001
syu search traceability --kind requirement
syu audit .
syu log REQ-CORE-002
syu relate REQ-001
syu trace src/command/check.rs --symbol run_check_command
syu app .
syu report . --output reports/syu.md
```

Running `syu` with no subcommand opens the interactive browser when stdin/stdout
are attached to a terminal.

`syu init` creates:

- `syu.yaml`
- `docs/syu/philosophy/`
- `docs/syu/policies/`
- `docs/syu/requirements/`
- `docs/syu/features/`

Prefer another repository layout such as `docs/spec` or `spec/contracts`?
Use `syu init . --spec-root docs/spec` to scaffold the same starter tree there
and write the matching `spec.root` value into `syu.yaml`.

Want stable project-specific starter IDs from the first commit? Seed a shared
stem directly into the scaffold:

```bash
syu init . --id-prefix store
```

That renders `PHIL-STORE-001`, `POL-STORE-001`, `REQ-STORE-001`, and
`FEAT-STORE-001`. If one layer should keep a different prefix, override it with
`--philosophy-prefix`, `--policy-prefix`, `--requirement-prefix`, or
`--feature-prefix`.

Want a closer starting point for a repository that is already clearly
docs-first, Rust-first, Python-first, Ruby-first, Go-first, Java-first, TypeScript-first, or polyglot? Start with a
lightweight template:

```bash
syu templates
syu init . --template docs-first
syu init . --template rust-only
syu init . --template ruby-only
syu init . --template go-only
syu init . --template java-only
syu init . --template python-only
syu init . --template typescript-only
syu init . --template polyglot
```

Run `syu templates` first if you want the starter names, one-line descriptions,
matching checked-in example paths, and whether a starter is example-backed or
starter-only before choosing a scaffold.

You can combine both flags when you want a custom spec root and a closer
starter layout:

```bash
syu init . --spec-root spec/contracts --template rust-only
```

## Commands

### `syu init`

Bootstrap a new workspace:

```bash
syu init .
syu init path/to/workspace --name my-project
syu init . --id-prefix store
syu init . --template docs-first
syu init . --template rust-only
syu init . --template go-only
syu init . --template java-only
syu init . --template typescript-only
syu init . --spec-root docs/spec
syu init . --requirement-prefix REQ-STORE --feature-prefix FEAT-STORE
```

Use `--force` to overwrite generated files. Use `--template` when you want the
starter IDs, file layout, and scaffold copy to start closer to the repository
style you already expect. Use `--id-prefix` when you want stable project-wide
starter IDs from the first command, and the per-layer `--*-prefix` flags when a
single shared stem is not enough. Use `--spec-root` to scaffold into a
repository-relative spec tree without moving the generated files by hand later.

### `syu templates`

List starter templates and their related checked-in examples before you
scaffold:

```bash
syu templates
syu templates --format json
```

Use this command when you want a quick discovery view. Each row shows the
template name, whether it is starter-only or backed by both a template and a
checked-in example, the related example path when one exists, and a short
description of the starter shape.

### `syu add`

Scaffold a new YAML stub after the initial workspace exists:

```bash
syu add philosophy PHIL-002
syu add requirement REQ-AUTH-001
syu add feature FEAT-AUTH-LOGIN-001 --kind auth
syu add feature FEAT-AUTH-001 --kind auth --file docs/syu/features/auth/login.yaml
```

`syu add` keeps the command intentionally small. It derives a default title and
document path from the ID, uses the configured `spec.root`, and updates
`features/features.yaml` automatically when you scaffold a new feature document.
It also prints matching scaffold suggestions for adjacent definitions so the
next reciprocal-link edits are concrete instead of guesswork. Edit the generated
stub fields and reciprocal links before you expect `syu validate` to pass
cleanly.

### `syu validate`

Validate definitions and traceability:

```bash
syu validate .
syu validate . --format json
syu validate . --severity error --genre trace
syu validate . --rule SYU-trace-file-002
syu validate . --id REQ-001
syu validate . --warning-exit-code 3
syu validate . --fix
syu validate . --no-fix
syu validate . --allow-planned=false
syu validate . --require-reciprocal-links=false
syu validate . --require-symbol-trace-coverage
```

`check` remains available as a compatibility alias for `validate`.
Use `--severity`, `--genre`, `--rule`, and `--id` to narrow the rendered issue list
without changing the underlying validation result or exit code.
Use the validate override flags for one-off stricter or looser runs without
editing `syu.yaml`.
By default, warning-only runs still exit 0. Add `--warning-exit-code <CODE>` when
CI or shell automation needs a distinct non-zero status for warnings while
keeping error-bearing runs on exit code 1.

For a plain-English guide to common validation errors, see the
[troubleshooting guide](docs/guide/troubleshooting.md).

### Command chooser

Use this quick chooser when you know the task but not yet the subcommand:

| If you want to... | Start with... | Why |
| --- | --- | --- |
| check whether your workspace currently validates | `syu validate .` | runs the full repository checks and points you to concrete errors before deeper browsing |
| inspect the whole workspace interactively in a terminal | `syu browse .` | shows the layered graph plus current validation errors in one terminal-first view |
| render one layer or emit automation-friendly lists | `syu list ...` | keeps the output list-shaped instead of opening the browser-style explorer |
| open one specific philosophy, policy, requirement, or feature by ID | `syu show ID` | jumps straight to the matched definition |
| look up IDs or keywords when you do not know the exact item yet | `syu search QUERY` | searches IDs, titles, summaries, and descriptions across layers |
| scan for overlap or policy drift before reviewing a bigger spec change | `syu audit` | summarizes heuristic overlap, tension, and orphaned-policy candidates |
| start from code or a test file and walk back to the owning spec item | `syu trace path/to/file --symbol name` | begins from traced implementation or test evidence instead of from YAML |
| inspect everything connected to one ID, symbol, or file | `syu relate TARGET` | expands upstream/downstream links plus traced files and symbols for review |
| explain whether one ID, file, or symbol still fits the connected chain | `syu explain TARGET` | turns the graph into a focused assessment with traces and obvious gaps |
| review what changed for one spec item in Git history | `syu log ID` | projects the traced definition and implementation paths onto checked-in commits |
| prefer a browser-first view for demos or visual navigation | `syu app .` | serves the same workspace graph in the local browser UI |

### `syu browse`

Browse philosophies, policies, features, requirements, and current validation
errors interactively:

```bash
syu
syu browse .
syu browse . --non-interactive
```

### `syu list`

Render list-shaped output without entering the interactive browser:

Use `syu list` when you want list-shaped output that can be narrowed to one
layer or emitted as JSON for automation. Use `syu browse --non-interactive`
when you want the browse snapshot instead: workspace metadata, per-layer
counts, grouped items, and the current validation errors in text or JSON.

```bash
syu list philosophy
syu list feature path/to/workspace --format json
```

### `syu show`

Show one definition by ID:

```bash
syu show REQ-001
syu show FEAT-BROWSE-001 path/to/workspace --format json
```

### `syu search`

Search definitions by ID, title, summary, or description:

```bash
syu search audit
syu search traceability --kind requirement
syu search FEAT-CHECK-001 --format json
```

### `syu audit`

Audit the four-layer graph for review-oriented semantic drift candidates:

```bash
syu audit
syu audit path/to/workspace
syu audit . --format json
```

`syu audit` keeps the result heuristic on purpose. It does not fail validation;
instead it summarizes likely overlapping requirements, feature wording that may
pull against linked policy or philosophy language, and policies that no longer
lead to concrete downstream requirements.

### `syu log`

Project one philosophy, policy, requirement, or feature onto checked-in Git history:

```bash
syu log REQ-CORE-002
syu log POL-004 --kind definition
syu log FEAT-CHECK-001 --kind implementation --path src/command
syu log REQ-CORE-019 --format json
```

`syu log` works from the current trace graph. It always includes the checked-in
definition path, and for requirements or features it also includes traced test
or implementation files. Then it shows the commits that touched those paths and
why each commit matched. Use `--kind` when you only want definition, test, or
implementation history, and `--path` when you want to narrow the traced paths
to one repository-relative file or directory prefix.
### `syu relate`

Inspect the connected graph around one definition, repository path, or traced
source symbol:

```bash
syu relate REQ-001
syu relate src/command/search.rs
syu relate run_search_command
syu relate FEAT-SEARCH-001 --format json
```

`syu relate` walks upstream and downstream across philosophy, policy,
requirement, and feature links, includes traced files and symbols for connected
requirements and features, and calls out sparse or suspicious gaps such as
missing adjacent links or missing evidence.

### `syu explain`

Turn one definition, repository path, or traced symbol into a focused review
summary:

```bash
syu explain REQ-001
syu explain src/command/search.rs
syu explain run_search_command
```

`syu explain` uses the same selector flexibility as `syu relate`, but reshapes
the result into a guided assessment: the connected philosophy/policy/
requirement/feature chain, the traces in scope, and any obvious gaps that still
need review.

### `syu trace`

Start from a traced file path and optional symbol when review or refactoring
begins in code instead of from a spec ID:

```bash
syu trace src/command/check.rs --symbol run_check_command
syu trace tests/report_command.rs path/to/workspace --format json
```

### `syu app`

Start a local browser app for the current workspace:

```bash
syu app .
syu app . --bind 127.0.0.1 --port 3000
```

After startup, `syu app` prints the local URL to open in your browser and a
`Ctrl-C` reminder for stopping the server.

The browser app serves a VitePlus / React / Tailwind UI and uses Rust plus
WebAssembly to build the layered browser view from the live workspace data.
When `syu.yaml` defines `app.bind` or `app.port`, `syu app` uses those defaults
unless the CLI flags override them.
Choose `syu app` when you want a browser-first view of the same workspace graph
that `syu browse` shows in the terminal.

### `syu report`

Generate a Markdown validation report:

```bash
syu report .
syu report . --output reports/syu.md
syu report . --output docs/generated/syu-report.md
```

The self-hosted repository keeps its latest generated report at
`docs/generated/syu-report.md`.
Set `report.output` in `syu.yaml` when a repository wants that checked-in path
to become the default, while keeping `--output` available for one-off overrides.
Contributors can refresh that report and the checked-in site-spec pages together
with `scripts/ci/check-generated-docs-freshness.sh`.

## Browser app

Use [`syu app`](#syu-app) when you want a local browser UI for searching the
four-layer graph, following links across related documents, and sharing the
same live workspace data that `syu browse` exposes in the terminal.

```bash
syu app .
syu app . --bind 127.0.0.1 --port 3000
```

For a visual tour with annotated screenshots, see the
[browser UI guide](docs/guide/app.md). Contributor-only implementation and
build details live under [Browser app development](#browser-app-development).

## VS Code extension

The repository now ships an editor-first extension scaffold under
[`editors/vscode/`](editors/vscode/).

It refreshes `syu validate --format json` diagnostics into the Problems panel,
adds a **syu Context** explorer view for the active file, opens YAML documents
from spec IDs, and jumps from a requirement or feature to its traced files.

To try it from source, open `editors/vscode/` in VS Code and press `F5` to
start an Extension Development Host. If `syu` is not already on your `PATH`,
set the extension's `syu.binaryPath` setting first.

See the full setup and command guide in
[`docs/guide/vscode-extension.md`](docs/guide/vscode-extension.md).

## Configuration

`syu` looks for `syu.yaml` in the workspace root:

```yaml
version: 0.0.1-alpha.8
spec:
  root: docs/syu
validate:
  default_fix: false
  allow_planned: true
  require_non_orphaned_items: true
  require_reciprocal_links: true
  require_symbol_trace_coverage: false
app:
  bind: 127.0.0.1
  port: 3000
runtimes:
  python:
    command: auto
  node:
    command: auto
```

Key behaviors:

- `version` defaults to the running `syu` CLI version, and `syu init` keeps them aligned
- `spec.root` changes where `syu` loads YAML definitions from
- `validate.default_fix` enables conservative autofix by default
- `validate.allow_planned` controls whether `planned` requirements and features are allowed at all
- `validate.require_non_orphaned_items` turns isolated layered definitions into validation errors
- `validate.require_reciprocal_links` keeps adjacent-layer backlinks mandatory by default while still allowing phased migration when disabled
- `validate.require_symbol_trace_coverage` opt-in checks that public Rust, Python, Go, Java, C#, and TypeScript/JavaScript symbols belong to features and tests belong to requirements, while still skipping configured repository-relative generated paths
- `report.output` sets the default `syu report` destination while `--output` still takes precedence
- `app.bind` and `app.port` define the default local browser-app address and port unless `--bind` / `--port` override them
- `report.output` sets the default `syu report` destination while `--output` still takes precedence
- `runtimes.*.command` can be set to `auto` or an explicit executable name/path

The self-hosted repository keeps a structured reference for supported config
fields under [`docs/syu/config/`](docs/syu/config).

The `syu` repository itself enables `validate.require_non_orphaned_items`,
`validate.require_reciprocal_links`, and
`validate.require_symbol_trace_coverage` in its root `syu.yaml`, and it sets
`report.output: docs/generated/syu-report.md`.

## Traceability rules

- philosophy ↔ policy links must exist and be reciprocal
- policy ↔ requirement links must exist and be reciprocal
- requirement ↔ feature links must exist and be reciprocal
- duplicate linked IDs inside one relationship list are rejected
- requirements and features must use `status: planned` or `status: implemented`
- `planned` items must not declare tests or implementations yet
- `implemented` items must declare valid tests or implementations
- linked requirements and features should not imply contradictory delivery states
- requirement test mappings must point to existing files and symbols
- feature implementation mappings must point to existing files and symbols
- trace file paths should use canonical repository-relative spelling
- duplicate trace mappings inside one language list are rejected
- traced files must mention the owning requirement / feature ID
- optional `doc_contains` snippets must be present in the traced symbol's
  documentation
- `symbols: ['*']` may be used when a feature or requirement intentionally owns
  every relevant symbol in the traced file

## Safe autofix

`syu validate --fix` is intentionally conservative. Today it only repairs
documentation-style trace gaps that can be updated mechanically:

- missing requirement / feature IDs in symbol documentation
- missing `doc_contains` snippets for Rust, Python, Go, and TypeScript symbols

It does **not** attempt speculative edits like renaming symbols or inventing
missing files.

## Example workspaces

The repository ships working example projects:

- [`examples/browser-ui`](examples/browser-ui)
- [`examples/csharp-fallback`](examples/csharp-fallback)
- [`examples/docs-first`](examples/docs-first)
- [`examples/go-only`](examples/go-only)
- [`examples/java-only`](examples/java-only)
- [`examples/rust-only`](examples/rust-only)
- [`examples/python-only`](examples/python-only)
- [`examples/ruby-only`](examples/ruby-only)
- [`examples/typescript-only`](examples/typescript-only)
- [`examples/polyglot`](examples/polyglot)
- [`examples/team-scale`](examples/team-scale)
`browser-ui` is the reference-only frontend example for traced React/TypeScript UI work.
`docs-first`, `rust-only`, `python-only`, `ruby-only`, `go-only`, `java-only`, `typescript-only`, and
`polyglot` match `syu init --template ...` starters directly.
`csharp-fallback` remains the reference-only example for teams that want a
lighter staged C# adoption path before tracing every C# symbol directly, and
`team-scale` remains a reference-only example for studying a larger
split-by-area repository shape.

Each one is validated in the automated test suite. If you are deciding between a
checked-in example and a scaffold template, start with
[`docs/guide/examples-and-templates.md`](docs/guide/examples-and-templates.md).

## Contributing and local development

If you're working on `syu` itself rather than using it in another repository:

- use [`.devcontainer/devcontainer.json`](.devcontainer/devcontainer.json) for
  a ready-to-run VS Code / Codespaces-style environment
- follow [`CONTRIBUTING.md`](CONTRIBUTING.md) for the GitHub Flow contributor
  path and repository expectations

### Local quality gates

Run the shared repository checks:

```bash
scripts/ci/quality-gates.sh
```

Run the 100% line-coverage gate:

```bash
cargo install cargo-llvm-cov --locked
scripts/ci/coverage.sh summary
```

Generate an LCOV artifact locally:

```bash
scripts/ci/coverage.sh lcov
```

Install pre-commit hooks:

```bash
scripts/install-precommit.sh
pre-commit run --all-files --hook-stage pre-commit
pre-commit run --all-files --hook-stage pre-push
```

If you prefer to install `pre-commit` manually, `pipx install pre-commit` or
`python -m pip install --user pre-commit` also work.

### Browser app development

The repository keeps the source UI in `app/` and generates the embedded
production bundle during Rust builds, so `syu app` still works from installed
binaries without checking `app/dist/` into `main`.

Use the checked-in Node 25 version from `app/package.json` and `app/.nvmrc`
when working in `app/`; that is the local contributor expectation, not only a
CI matrix detail. For the repo-wide app/docs/editor version map, use the
[repository Node workflow guide](docs/guide/node-workflow.md). Before you run
the install commands below, switch your shell to that Node major with a version
manager such as `nvm`, `fnm`, or `Volta` using the checked-in `app/.nvmrc`.
Before you run Cargo commands that embed the browser app, provision the app
dependencies intentionally from the repository root:

```bash
scripts/ci/pinned-npm.sh install app
npm --prefix app ci
```

If you prefer the checked-in bootstrap entrypoint for the same app-only setup,
switch to Node 25 first and use:

```bash
scripts/ci/bootstrap-contributor-tooling.sh --app
```

Cargo no longer runs `npm ci` for you during normal builds. If you are in a
fresh clone or fresh worktree and `app/node_modules` is missing or stale,
`build.rs` stops and points back to the commands above so offline, hermetic, and security-sensitive environments do not hide a networked package-manager install inside an ordinary Rust build.

When contributors change browser app sources or build inputs, they should run
`scripts/ci/check-browser-app-freshness.sh`. That flow regenerates the local
`app/src/wasm` bridge, typechecks the browser app, and builds a fresh local
`app/dist/` artifact the same way CI does before uploading it.

```bash
scripts/ci/pinned-npm.sh install app
npm --prefix app ci
npm --prefix app run build:wasm
npm --prefix app run build
cargo run -- app .
```

## Documentation site

The repository ships a Docusaurus site rooted at `website/` that renders the
checked-in `docs/` tree directly, and the published site is available at
`https://ugoite.github.io/syu/`.

Use the checked-in Node 20 version from `website/package.json` and
`website/.nvmrc` when working in `website/`, and use the
[repository Node workflow guide](docs/guide/node-workflow.md) when you need the
full app/docs/editor runtime map in one place. Then use the helper below so
local installs follow the same pinned npm flow CI uses:

```bash
bash scripts/ci/install-docs-site-deps.sh
npm --prefix website run start
```

If you want the bootstrap entrypoint instead of the raw docs-site commands,
switch to Node 20 first and use:

```bash
scripts/ci/bootstrap-contributor-tooling.sh
```

Without flags, the bootstrap only installs the surfaces that match the current
shell major from the checked-in `.nvmrc` files. In this repository that means
Node 25 selects the browser app, while Node 20 selects the docs site plus the
VS Code extension.

The install script removes `website/node_modules` first so repeated docs-site
setup stays deterministic across branch switches and reused worktrees.

The landing page links the core guides, the self-hosted specification reference,
the latest checked-in validation report, and the published site is deployed
from `main` to GitHub Pages via
`.github/workflows/deploy-pages.yml`.

## Agent skill

<!-- FEAT-SKILLS-001 -->

The repository also ships a checked-in agent skill inspired by Anthropics
Skills:

- [`skills/syu-maintainer/SKILL.md`](skills/syu-maintainer/SKILL.md)
- [`skills/README.md`](skills/README.md)

It documents a repeatable workflow for browsing the layered model, updating
adjacent links, running `syu validate .`, and refreshing
`docs/generated/syu-report.md`.

## Specification layout

```text
docs/syu/
  philosophy/*.yaml
  policies/*.yaml
  requirements/**/*.yaml
  features/features.yaml
  features/**/*.yaml
  config/*.yaml
skills/*/SKILL.md
```

`requirements/` documents may be grouped into nested folders. `features/` keeps the
explicit `features/features.yaml` registry, and each registry entry may point to a
nested YAML document under `features/`.

## Built-in language adapters

`syu` validates the languages used in `ugoite` today:

- Rust
- Python
- TypeScript

For self-hosting repository automation and metadata, it also ships adapters for:

- Shell
- YAML
- JSON

## Release automation

The repository includes:

- GitHub Actions CI for pre-commit, quality gates, coverage, and workflow linting
- release-please automation on `main` so stable releases stay aligned with GitHub Flow
- release artifact packaging for Linux, macOS (Intel/Apple Silicon), and Windows

Release notes come from GitHub Releases rather than a committed `CHANGELOG.md`.
Track-specific release notes are generated so alpha releases compare against the
previous alpha, beta releases compare against the previous beta, and stable
releases compare against the previous stable tag.

Release binaries are packaged with `scripts/ci/package-release.sh`, published to
GitHub Packages / GHCR, and uploaded as GitHub release assets.

When upgrading between alpha releases, consult the
[migration guide](docs/guide/migration.md) for breaking config changes, renamed
CLI flags, and new default-on validation rules.

See `docs/syu/` for `syu`'s own self-hosted specification.
