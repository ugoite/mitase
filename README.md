# syu

<!-- FEAT-DOCS-001 -->

`syu` is a Rust CLI for specification-driven development.

It keeps machine-readable `philosophy`, `policy`, `requirements`, and `feature`
YAML definitions connected to real repository work: validation, implementation
ownership, maintenance, contributor workflow, and release delivery.

The design goal is intentionally pragmatic:

- specification-driven development that keeps looking after implementation and maintenance
- a language-agnostic model that can fit Rust-only, Python-only, or polyglot repositories
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

- **Quick start**: stay in this README when the in-page four-layer refresher is
  enough, you already grasp the four-layer idea, want the shortest roughly
  5-minute path from install to `syu validate .`, and are comfortable making
  the first manual YAML edits after a short refresher.
- **Getting started**: follow
  [`docs/guide/getting-started.md`](docs/guide/getting-started.md) when you want
  the same first workspace setup explained step by step before the manual YAML
  edits and first validation run.
- **Existing repository adoption**: follow
  [`docs/guide/existing-repository.md`](docs/guide/existing-repository.md) when
  the repository already has code and history and you want an incremental
  adoption path instead of starting with `syu init`.
- **Upgrade or migration**: start with
  [`docs/guide/migration.md`](docs/guide/migration.md) when you are updating an
  existing `syu` workspace between alpha releases and need the release-specific
  upgrade steps first.
- **Visual explorer**: open [`docs/guide/app.md`](docs/guide/app.md) or run
  `syu app .` when you want graphical spec navigation before learning the full
  CLI workflow.
- **Tutorial**: follow [`docs/guide/tutorial.md`](docs/guide/tutorial.md) when you
  want a realistic end-to-end repository story instead of a short setup path.
- **Trace adapter matrix**: open
  [`docs/guide/trace-adapter-support.md`](docs/guide/trace-adapter-support.md)
  when you need to know which built-in languages support symbol validation
  only versus `doc_contains` and strict coverage.
- **Troubleshooting**: jump to
  [`docs/guide/troubleshooting.md`](docs/guide/troubleshooting.md) when you
  already have a workspace and validation or traceability errors are blocking
  the next step more than onboarding is.
- **Spec anti-patterns**: read
  [`docs/guide/spec-antipatterns.md`](docs/guide/spec-antipatterns.md) when the
  workspace validates but the layer boundaries still feel messy.

Keep the detailed guides close:

- [`docs/guide/concepts.md`](docs/guide/concepts.md)
- [`docs/guide/getting-started.md`](docs/guide/getting-started.md)
- [`docs/guide/migration.md`](docs/guide/migration.md)
- [`docs/guide/trace-adapter-support.md`](docs/guide/trace-adapter-support.md)
- [`docs/guide/existing-repository.md`](docs/guide/existing-repository.md)
- [`docs/guide/tutorial.md`](docs/guide/tutorial.md)
- [`docs/guide/configuration.md`](docs/guide/configuration.md)
- [`docs/guide/troubleshooting.md`](docs/guide/troubleshooting.md)
- [`docs/guide/spec-antipatterns.md`](docs/guide/spec-antipatterns.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)

## Is syu right for this repository?

`syu` is a good fit when the repository needs checked-in, reviewable traceability
between intent, rules, requirements, implementation, and tests.

Adoption usually pays off when you want to:

- keep repository expectations explicit instead of scattered across ADRs, wikis,
  or review folklore
- make requirement-to-code and requirement-to-test links visible in pull requests
- keep a shared model that still works across Rust-only, Python-only, or
  polyglot repositories
- preserve project intent for a long-lived codebase with multiple contributors

`syu` is probably too heavy when the repository is still a short-lived prototype,
a very small solo project, or a codebase where informal docs and code review are
already enough because nobody needs machine-readable traceability.

Compared with docs-only workflows, `syu` asks teams to keep structured YAML,
reciprocal links, and validation in the normal contributor loop. That is extra
authoring overhead, but in return the repository gets searchable, enforceable,
repository-native traceability instead of documentation that can drift away from
tests and implementation.

Repositories that benefit most are the ones where change review needs more than
good intentions: long-lived products, shared platforms, compliance-sensitive
systems, and multi-team or multi-language repositories where contributors need a
clear checked-in record of why behavior exists and how it is proven.

If that sounds like your repository, continue with the install flow below. If
not, a lighter docs-only workflow may be the better starting point for now.

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
RELEASE=v0.0.1-alpha.7
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
$release = 'v0.0.1-alpha.7'
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
the platform archive that the installer downloads before it unpacks the binary:

```bash
gh attestation verify syu-x86_64-unknown-linux-gnu.tar.gz --repo ugoite/syu
```

### Shortcut: run the installer directly

If you already trust the release source and want the shortest path, use the
one-line entrypoint:

The download URL stays pinned to the installer shipped with this checked-in
documentation version. Use `SYU_VERSION=alpha`, `stable`, or an explicit
version selector when you want that installer to fetch a different published
package track after it starts.

Current installer entrypoint:

```bash
RELEASE=v0.0.1-alpha.7
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | bash
```

During the alpha phase, prefer the `alpha` track selector so the same installer
entrypoint always resolves to the latest published alpha:

```bash
RELEASE=v0.0.1-alpha.7
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_VERSION=alpha bash
```

Install to a custom directory:

```bash
RELEASE=v0.0.1-alpha.7
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_INSTALL_DIR=$HOME/bin bash
```

Install a specific prerelease:

```bash
RELEASE=v0.0.1-alpha.7
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_VERSION="$RELEASE" bash
```

If you're contributing to `syu` itself from source, jump to
[Contributing and local development](#contributing-and-local-development)
below. The rest of this section assumes you installed the published CLI.

## Quick start

Treat this section as the compact command card. Stay in this README when the
in-page four-layer refresher is enough and you want the shortest install-to-validate path.
If you want the first-run walkthrough explained step by step instead, switch to
[`docs/guide/getting-started.md`](docs/guide/getting-started.md) before
continuing.

The only prerequisite for the commands below is the
[Why four layers?](#why-four-layers) refresher above, which gives you the
minimal context for `philosophy`, `policy`, `requirements`, and `features`.

The first manual edit in this quick start happens in the generated requirement
YAML: add `linked_policies:` and `linked_features:` there, then update the
adjacent policy and feature YAML so they add the reciprocal
`linked_requirements:` entry back to the new requirement.

Read [`docs/guide/concepts.md`](docs/guide/concepts.md) before continuing only
if you want the fuller rationale and authoring guidance instead of the shortest
README-first path.
If you want the canonical narrated first-run path instead, jump to
[`docs/guide/getting-started.md`](docs/guide/getting-started.md).

Step 0: required — run `syu init .` before any of the other commands in a new
repository. If the repository already exists and you do not want an in-place
scaffold flow, follow
[`docs/guide/existing-repository.md`](docs/guide/existing-repository.md)
instead.

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
Rust-first, Python-first, or polyglot? Start with a lightweight template:

```bash
syu templates
syu init . --template rust-only
syu init . --template python-only
syu init . --template polyglot
```

Use `syu templates` first when you want the built-in starter names, short
descriptions, and related checked-in example paths in one command.

You can combine both flags when you want a custom spec root and a closer
starter layout:

```bash
syu init . --spec-root spec/contracts --template rust-only
```

Not sure whether you should start from a template or inspect a checked-in
example first? See the
[examples and templates guide](docs/guide/examples-and-templates.md).

## Commands

### `syu init`

Bootstrap a new workspace:

```bash
syu init .
syu init path/to/workspace --name my-project
syu init . --id-prefix store
syu init . --template rust-only
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

Working in Go, Java, C#, or another unsupported implementation language? Start
with the spec layers first and add code-level traces once adapter support
lands. For supported lightweight adapters such as `shell`, `yaml`, `json`,
`markdown`, and `gitignore`, keep traces to file or symbol ownership without
`doc_contains`. The newcomer path and roadmap links live in
[`docs/guide/getting-started.md`](docs/guide/getting-started.md#unsupported-implementation-languages-can-still-adopt-the-spec-layers-first).

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
editing `syu.yaml`. Before you rely on `doc_contains` or strict trace coverage
in a mixed-language repository, check the [trace adapter capability
matrix](docs/guide/trace-adapter-support.md).

For a plain-English guide to common validation errors, see the
[troubleshooting guide](docs/guide/troubleshooting.md).

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
counts, and the current validation errors in plain text.

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

The installed CLI can launch a local browser app for richer spec exploration:

```bash
syu app .
syu app . --bind 127.0.0.1 --port 3000
```

The repository keeps the source UI in `app/` and generates the embedded
production bundle during Rust builds, so `syu app` still works from installed
binaries without checking `app/dist/` into `main`.

When contributors change browser app sources or build inputs, they should run
`scripts/ci/check-browser-app-freshness.sh`. That flow regenerates the local
`app/src/wasm` bridge, typechecks the browser app, and builds a fresh local
`app/dist/` artifact the same way CI does before uploading it.

Want the visual tour first? See the [browser UI guide with annotated
screenshots](docs/guide/app.md).

## Configuration

`syu` looks for `syu.yaml` in the workspace root:

```yaml
version: 0.0.1-alpha.7
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
- `validate.require_symbol_trace_coverage` opt-in checks that public Rust, Python, and TypeScript/JavaScript symbols belong to features and tests belong to requirements, while declared Go traces are still validated separately until strict Go ownership inventory support lands
- the [trace adapter capability matrix](docs/guide/trace-adapter-support.md) shows which built-in languages stop at symbol validation versus also supporting `doc_contains` and strict ownership inventory
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

- [`examples/rust-only`](examples/rust-only) — minimal single-language Rust starter, and the checked-in match for `syu init . --template rust-only`
- [`examples/go-only`](examples/go-only) — a Go-first example that keeps real `.go` files in the repository while showing the current unsupported-language adoption path
- [`examples/python-only`](examples/python-only) — minimal Python-first starter, and the checked-in match for `syu init . --template python-only`
- [`examples/polyglot`](examples/polyglot) — one requirement and feature traced across Rust, Python, and TypeScript, and the checked-in match for `syu init . --template polyglot`
- [`examples/team-scale`](examples/team-scale) — a larger Rust workspace that shows phased adoption, nested documents, and recovery drills; reference-only, not a starter template

Each one is validated in the automated test suite.

Need help choosing between a starter template and a full reference workspace?
See the [examples and templates guide](docs/guide/examples-and-templates.md).

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

```bash
cd app
npm install
npm run build:wasm
npm run build
cd ..
cargo run -- app .
```

## Documentation site

The repository ships a Docusaurus site rooted at `website/` that renders the
checked-in `docs/` tree directly, and the published site is available at
`https://ugoite.github.io/syu/`.

```bash
cd website
npm install
npm run start
```

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

`syu` validates these built-in implementation languages in this checked-in version:

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
