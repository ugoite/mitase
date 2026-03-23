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

See the detailed guides:

- [`docs/guide/concepts.md`](docs/guide/concepts.md)
- [`docs/guide/getting-started.md`](docs/guide/getting-started.md)
- [`docs/guide/configuration.md`](docs/guide/configuration.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)

## Install from published releases

`syu` publishes a release-hosted installer entrypoint and repository-scoped
GHCR packages under `ghcr.io/ugoite/syu`. Download the installer from the
current CLI release; the script prefers the matching package artifact and falls
back to GitHub release assets if anonymous package pulls are not available yet.

Current installer entrypoint:

```bash
curl -fsSL https://github.com/ugoite/syu/releases/download/v0.0.1-alpha.7/install-syu.sh | bash
```

Pin a specific release track:

```bash
curl -fsSL https://github.com/ugoite/syu/releases/download/v0.0.1-alpha.7/install-syu.sh | env SYU_VERSION=alpha bash
```

Install to a custom directory:

```bash
curl -fsSL https://github.com/ugoite/syu/releases/download/v0.0.1-alpha.7/install-syu.sh | env SYU_INSTALL_DIR=$HOME/bin bash
```

Install a specific prerelease:

```bash
curl -fsSL https://github.com/ugoite/syu/releases/download/v0.0.1-alpha.7/install-syu.sh | env SYU_VERSION=v0.0.1-alpha.7 bash
```

If you're contributing to `syu` itself from source, jump to
[Contributing and local development](#contributing-and-local-development)
below. The rest of this section assumes you installed the published CLI.

## Quick start

```bash
syu init .
syu validate .
syu validate . --fix
syu browse .
syu list requirement
syu show REQ-CORE-015
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

## Commands

### `syu init`

Bootstrap a new workspace:

```bash
syu init .
syu init path/to/workspace --name my-project
```

Use `--force` to overwrite generated files.

### `syu validate`

Validate definitions and traceability:

```bash
syu validate .
syu validate . --format json
syu validate . --severity error --genre trace
syu validate . --rule SYU-trace-file-002
syu validate . --fix
syu validate . --no-fix
```

`check` remains available as a compatibility alias for `validate`.
Use `--severity`, `--genre`, and `--rule` to narrow the rendered issue list
without changing the underlying validation result or exit code.

### `syu browse`

Browse philosophies, policies, features, requirements, and current validation
errors interactively:

```bash
syu
syu browse .
```

### `syu list`

List one layer without entering the interactive browser:

```bash
syu list philosophy
syu list feature path/to/workspace --format json
```

### `syu show`

Show one definition by ID:

```bash
syu show REQ-CORE-015
syu show FEAT-BROWSE-001 path/to/workspace --format json
```

### `syu app`

Start a local browser app for the current workspace:

```bash
syu app .
syu app . --bind 127.0.0.1 --port 3000
```

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

## Browser app

The installed CLI can launch a local browser app for richer spec exploration:

```bash
syu app .
syu app . --bind 127.0.0.1 --port 3000
```

The repository keeps the source UI in `app/`, checks in the generated
production bundle in `app/dist/`, and serves that bundle directly from the
`syu app` command, so end users do not need a separate frontend build step.

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
- `validate.require_symbol_trace_coverage` opt-in checks that public Rust symbols belong to features and tests belong to requirements
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
- missing `doc_contains` snippets for Rust, Python, and TypeScript symbols

It does **not** attempt speculative edits like renaming symbols or inventing
missing files.

## Example workspaces

The repository ships working example projects:

- [`examples/rust-only`](examples/rust-only)
- [`examples/python-only`](examples/python-only)
- [`examples/polyglot`](examples/polyglot)

Each one is validated in the automated test suite.

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

See `docs/syu/` for `syu`'s own self-hosted specification.
