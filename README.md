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

## Install from GitHub Packages

`syu` publishes repository-scoped GHCR packages under `ghcr.io/ugoite/syu` and
falls back to matching GitHub release assets if anonymous package pulls are not
available yet.

Latest published package for your platform:

```bash
curl -fsSL https://raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh | bash
```

Pin a specific release track:

```bash
curl -fsSL https://raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh | env SYU_VERSION=alpha bash
```

Install to a custom directory:

```bash
curl -fsSL https://raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh | env SYU_INSTALL_DIR=$HOME/bin bash
```

Install a specific prerelease:

```bash
curl -fsSL https://raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh | env SYU_VERSION=v0.0.1-alpha.7 bash
```

## Quick start

```bash
cargo run -- init .
cargo run -- browse .
cargo run -- app .
cargo run -- validate .
cargo run -- validate . --fix
cargo run -- report . --output reports/syu.md
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

### `syu app`

Start a local browser app for the current workspace:

```bash
syu app .
syu app . --bind 127.0.0.1 --port 3000
```

The browser app serves a VitePlus / React / Tailwind UI and uses Rust plus
WebAssembly to build the layered browser view from the live workspace data.

### `syu report`

Generate a Markdown validation report:

```bash
syu report .
syu report . --output reports/syu.md
syu report . --output docs/generated/syu-report.md
```

The self-hosted repository keeps its latest generated report at
`docs/generated/syu-report.md`.

## Browser app

The repository also ships a local browser app rooted at `app/` for richer spec
exploration.

```bash
cd app
npm install
npm run build:wasm
npm run build
cd ..
cargo run -- app .
```

It keeps the source UI in `app/`, checks in the generated production bundle in
`app/dist/`, and serves that bundle directly from the `syu app` command.

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
- `runtimes.*.command` can be set to `auto` or an explicit executable name/path

The self-hosted repository keeps a structured reference for supported config
fields under [`docs/syu/config/`](docs/syu/config).

The `syu` repository itself enables `validate.require_non_orphaned_items`,
`validate.require_reciprocal_links`, and
`validate.require_symbol_trace_coverage` in its root `syu.yaml`.

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

## Contributor environment

For VS Code / Codespaces-style development, use the devcontainer:

- [`.devcontainer/devcontainer.json`](.devcontainer/devcontainer.json)
- [`CONTRIBUTING.md`](CONTRIBUTING.md) for the GitHub Flow contribution path

## Local quality gates

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

## Documentation site

The repository ships a Docusaurus site rooted at `website/` that renders the
checked-in `docs/` tree directly.

```bash
cd website
npm install
npm run start
```

The landing page links the core guides, the self-hosted specification reference,
the latest checked-in validation report, and the published site is deployed
from `main` to GitHub Pages at `https://ugoite.github.io/syu/` via
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
