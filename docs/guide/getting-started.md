# Getting started with syu

<!-- FEAT-DOCS-001 -->

## Before you begin

Make sure `syu` is installed and available on your `PATH`.

**Option A — Install from a release (recommended)**

```bash
curl -fsSL https://github.com/ugoite/syu/releases/latest/download/install.sh | bash
```

This places `syu` in `~/.local/bin`. Add that directory to your `PATH` if it is not already there.

**Option B — Build from source**

Building from source requires [Rust and Cargo](https://rustup.rs). This option is only needed when contributing to `syu` itself, not for using it in your own project.

```bash
git clone https://github.com/ugoite/syu.git
cd syu
cargo install --path .
```

Verify the installation:

```bash
syu --version
```

## 1. Create a workspace

Bootstrap a new project:

```bash
syu init .
```

Or scaffold another directory:

```bash
syu init path/to/workspace --name my-project
```

This creates `syu.yaml` and a starter `docs/syu/` tree.

Starter requirements and features begin as `status: planned`. Keep them planned
until you are ready to declare real tests and implementation traces.

## 2. Fill in the starter spec

Start by editing:

- `docs/syu/philosophy/foundation.yaml`
- `docs/syu/policies/policies.yaml`
- `docs/syu/requirements/core/core.yaml`
- `docs/syu/features/core/core.yaml`

As the workspace grows, you can group requirement and feature files into nested
folders. Keep feature discovery explicit by updating `docs/syu/features/features.yaml`
whenever you add or move a feature document.

Make sure links are reciprocal:

- philosophy ↔ policy
- policy ↔ requirement
- requirement ↔ feature

## 3. Add traces to tests and implementations

When a requirement or feature becomes real, change its status to
`implemented` and add the corresponding traces.

Requirements should declare tests:

```yaml
status: implemented
tests:
  rust:
    - file: src/trace.rs
      symbols:
        - requirement_test
      doc_contains:
        - requirement doc line
```

Features should declare implementations:

```yaml
status: implemented
implementations:
  python:
    - file: python/app.py
      symbols:
        - feature_impl
      doc_contains:
        - feature doc line
```

## 4. Validate the workspace

```bash
syu validate .
syu browse .
syu list feature
syu show REQ-CORE-015
syu app .
```

Use JSON when integrating with automation:

```bash
syu validate . --format json
syu validate . --severity error --genre trace
syu list requirement --format json
syu show FEAT-BROWSE-001 --format json
```

Filters stay view-oriented: they narrow the visible diagnostics while preserving
the full validation result and exit code.

### Understanding validation output

Validation errors follow the pattern `SYU-[genre]-[content]-[NNN]`:

| Segment | Meaning | Examples |
|---------|---------|---------|
| `genre` | Which layer of the spec the rule checks | `workspace`, `graph`, `trace`, `delivery`, `coverage` |
| `content` | The specific concern within that genre | `orphaned`, `reciprocal`, `symbol`, `file` |
| `NNN` | Numeric index within the genre+content group | `001`, `002`, … |

**Common errors and their fixes**

| Code | What it means | How to fix it |
|------|--------------|---------------|
| `SYU-graph-orphaned-001` | A spec item has no links to adjacent layers | Add `philosophies`, `policies`, `requirements`, or `features` links |
| `SYU-graph-reciprocal-001` | A link exists in only one direction | Make the link mutual (e.g. if REQ links to FEAT, FEAT must also link back to REQ) |
| `SYU-graph-reference-001` | A link points to an ID that does not exist | Fix the typo in the ID, or add the missing definition |
| `SYU-trace-file-001` | A declared trace file does not exist on disk | Create the file or correct the path |
| `SYU-trace-symbol-001` | A declared symbol is not found in the file | Add the symbol or correct its name |
| `SYU-delivery-planned-001` | A `planned` item declares traces before it is implemented | Remove traces or change status to `implemented` |

The full rule catalog is in [`docs/syu/features/validation/validation.yaml`](../syu/features/validation/validation.yaml). Use `syu validate . --genre graph` to focus on a single genre.

## 5. Apply safe autofixes

If a traced symbol is missing required documentation snippets, run:

```bash
syu validate . --fix
```

Autofix is conservative. It currently repairs documentation-style trace gaps
for Rust, Python, and TypeScript without guessing at larger structural changes.

## 6. Generate a report

```bash
syu report . --output reports/syu.md
```

## 7. Study the examples

The repository includes complete examples:

- `examples/rust-only`
- `examples/python-only`
- `examples/polyglot`

## Keep exploring

- Follow the [end-to-end tutorial](./tutorial.md) to build a complete four-layer spec from scratch
- Read [syu concepts](./concepts.md) for the reasoning behind the four layers
- Review [configuration](./configuration.md) before tightening validation in a real repository
- Start `syu app .` when you want a browser view of the same workspace
- Browse the [Specification Reference](../generated/site-spec/index.md) to see how `syu` self-hosts its own contract
- Open the [latest validation report](../generated/syu-report.md) to inspect the checked-in repository status
