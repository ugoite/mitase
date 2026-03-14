# syu

<!-- FEAT-DOCS-001 -->

`syu` is a Rust CLI for specification-driven development.

It manages machine-readable `philosophy`, `policy`, `requirements`, and
`feature` YAML definitions, validates their dependency graph, checks
requirement-to-test and feature-to-implementation traceability, applies safe
autofixes for missing trace documentation, and generates Markdown reports.

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
curl -fsSL https://raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh | env SYU_VERSION=v0.0.1-alpha.3 bash
```

## Quick start

```bash
cargo run -- init .
cargo run -- validate .
cargo run -- validate . --fix
cargo run -- report . --output reports/syu.md
```

`syu init` creates:

- `syu.yaml`
- `docs/spec/philosophy/`
- `docs/spec/policies/`
- `docs/spec/requirements/`
- `docs/spec/features/`

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
syu validate . --fix
syu validate . --no-fix
```

`check` remains available as a compatibility alias for `validate`.

### `syu report`

Generate a Markdown validation report:

```bash
syu report .
syu report . --output reports/syu.md
```

## Configuration

`syu` looks for `syu.yaml` in the workspace root:

```yaml
version: 0.0.1
spec:
  root: docs/spec
validate:
  default_fix: false
  allow_planned: true
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
- `runtimes.*.command` can be set to `auto` or an explicit executable name/path

## Traceability rules

- philosophy ↔ policy links must exist and be reciprocal
- policy ↔ requirement links must exist and be reciprocal
- requirement ↔ feature links must exist and be reciprocal
- requirements and features must use `status: planned` or `status: implemented`
- `planned` items must not declare tests or implementations yet
- `implemented` items must declare valid tests or implementations
- requirement test mappings must point to existing files and symbols
- feature implementation mappings must point to existing files and symbols
- traced files must mention the owning requirement / feature ID
- optional `doc_contains` snippets must be present in the traced symbol's
  documentation

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
python -m pip install pre-commit
pre-commit install --hook-type pre-commit --hook-type pre-push
pre-commit run --all-files --hook-stage pre-commit
pre-commit run --all-files --hook-stage pre-push
```

## Specification layout

```text
docs/spec/
  philosophy/*.yaml
  policies/*.yaml
  requirements/*.yaml
  features/features.yaml
  features/*.yaml
```

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

Release binaries are packaged with `scripts/ci/package-release.sh`, published to
GitHub Packages / GHCR, and uploaded as GitHub release assets.

See `docs/spec/` for `syu`'s own self-hosted specification.
