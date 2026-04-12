# Getting started with syu

<!-- FEAT-DOCS-001 -->

:::tip New to syu?
New to `syu`? Read [syu concepts](./concepts.md) first to understand the four
layers before you start editing YAML.
:::

Start here once `syu` is installed:

```bash
syu init .                           # 1. Create spec scaffold
syu add requirement REQ-AUTH-001     # 2. Generate a requirement stub
```

Before step 3, open the generated requirement YAML under `docs/syu/requirements/`
(or your configured `spec.root`), add at least one `linked_policies:` entry and
one `linked_features:` entry, then update those policy and feature documents so
they link back to the new requirement.

```bash
syu validate .                       # 3. Check everything is linked
syu app .                            # 4. Browse in the browser
```

## Before you begin

Make sure `syu` is installed and available on your `PATH`.

**Option A — Install from a release (recommended)**

```bash
curl -fsSL https://github.com/ugoite/syu/releases/download/v0.0.1-alpha.7/install-syu.sh | env SYU_VERSION=alpha bash
```

This uses the current installer entrypoint plus `SYU_VERSION=alpha` so you stay
on the latest alpha during the prerelease phase. It places `syu` in
`~/.local/bin`. Add that directory to your `PATH` if it is not already there.

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

Need another repository layout from the first command? Scaffold a custom spec
root directly:

```bash
syu init . --spec-root docs/spec
```

Need stable project-specific starter IDs from the first command?

```bash
syu init . --id-prefix store
```

Need a closer starting point for a repository that is already Rust-first,
Python-first, or polyglot?

```bash
syu init . --template rust-only
```

You can also combine both flags:

```bash
syu init . --spec-root docs/spec --template rust-only
```

This creates `syu.yaml` and a starter spec tree. By default the tree lives
under `docs/syu/`; `--spec-root` writes the same scaffold into another
repository-relative path and records that location in `syu.yaml`. `--template`
keeps the same four layers but swaps the starter IDs, requirement/feature file
names, and copy so the first edit looks more like the repository style you
already expect. `--id-prefix` seeds a shared stem into all four starter IDs so
the scaffold begins with `PHIL-STORE-001`, `POL-STORE-001`, `REQ-STORE-001`,
and `FEAT-STORE-001` instead of the generic defaults. When one layer needs a
different prefix, use `--philosophy-prefix`, `--policy-prefix`,
`--requirement-prefix`, or `--feature-prefix`.

Starter requirements and features begin as `status: planned`. Keep them planned
until you are ready to declare real tests and implementation traces.

## 2. Add and refine spec items

Start with the generated files under your configured `spec.root`
(default: `docs/syu`), then scaffold new items as the workspace grows:

```bash
syu add philosophy PHIL-002
syu add policy POL-002
syu add requirement REQ-AUTH-001
syu add feature FEAT-AUTH-LOGIN-001 --kind auth
```

Prefer a guided terminal flow for the next item?

```bash
syu add requirement --interactive
syu add feature path/to/workspace --interactive
```

When you omit the ID in a terminal, `syu add` prompts for it. With
`--interactive`, it also lets you confirm the feature kind and override the
target YAML path before writing the stub.

These commands generate correctly shaped YAML stubs, infer a default document
path from the ID, and keep feature discovery explicit by updating the feature
registry automatically when a new feature document is created. You can override
the target file when you want a more specific layout:

```bash
syu add feature FEAT-AUTH-001 --kind auth --file docs/syu/features/auth/login.yaml
```

Direct YAML editing is still supported. The default scaffolded files are:

- `<spec.root>/philosophy/foundation.yaml`
- `<spec.root>/policies/policies.yaml`
- `<spec.root>/requirements/core/core.yaml` (template starters may use
  `requirements/core/rust.yaml`, `requirements/core/python.yaml`, or
  `requirements/core/polyglot.yaml`)
- `<spec.root>/features/core/core.yaml` (template starters may use
  `features/languages/rust.yaml`, `features/languages/python.yaml`, or
  `features/languages/polyglot.yaml`)

As the workspace grows, you can group requirement and feature files into nested
folders. `syu add` chooses a default folder from the ID or feature `--kind`,
but you can still move documents later if another layout reads better in your
repository. When you move a feature document by hand, keep feature discovery
explicit by updating `<spec.root>/features/features.yaml` (default:
`docs/syu/features/features.yaml`). `syu validate` reports feature YAML files on
disk that are missing from that registry.

Requirements are discovered by walking the `requirements/` tree, but features use
an explicit registry because implementation claims should stay deliberate and
reviewable. That registry is a short YAML list of feature documents:

```yaml
version: 0.0.1-alpha.7
files:
  - kind: core
    file: core/core.yaml
```

Make sure links are reciprocal:

- philosophy ↔ policy
- policy ↔ requirement
- requirement ↔ feature

## 3. Add traces to tests and implementations

When a requirement or feature becomes real, change its status to
`implemented` and add the corresponding traces.

Traces tell `syu` where each spec item is tested or implemented in your
codebase. Each trace entry has two key fields:

- **`symbols`** — The names of the functions, methods, or classes in the
  referenced file that implement or test this spec item (e.g., a test
  function called `test_store_upload`).
- **`doc_contains`** — One or more strings that must appear verbatim in the
  documentation comment of each listed symbol. This proves the linkage is
  intentional: the developer explicitly wrote the spec ID in the comment.

For example, a Rust test function that satisfies
`doc_contains: ["FEAT-STORE-001"]` looks like this:

```rust
/// Verifies file upload round-trip — covers FEAT-STORE-001.
#[test]
fn test_store_upload() { /* … */ }
```

`syu validate` reads the doc comment and checks that `"FEAT-STORE-001"`
appears in it. If the string is missing, the rule `SYU-trace-doc-001`
fires with a suggestion to add the snippet (or run `syu validate --fix`).

Requirements should declare tests:

```yaml
status: implemented
tests:
  rust:
    - file: src/trace.rs
      symbols:
        - requirement_test   # name of the test function
      doc_contains:
        - REQ-CORE-001       # this string must appear in the function's doc comment
```

Features should declare implementations:

```yaml
status: implemented
implementations:
  python:
    - file: python/app.py
      symbols:
        - feature_impl       # name of the implementing function or class
      doc_contains:
        - FEAT-STORE-001     # this string must appear in the function's doc comment
```

## 4. Validate the workspace

```bash
syu validate .
syu browse .
syu list feature
syu show REQ-001
syu app .
```

Use JSON when integrating with automation:

```bash
syu validate . --format json
syu validate . --severity error --genre trace
syu validate . --id REQ-001
syu list requirement --format json
syu show FEAT-BROWSE-001 --format json
```

Filters stay view-oriented: they narrow the visible diagnostics while preserving
the full validation result and exit code.

For CI or shell scripts that still want text output, add `--quiet` to suppress
the next-step guidance block while keeping the validation summary and exit code.

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

If one of those examples is already close to your repository, use the matching
`syu init --template ...` option so the starter scaffold begins nearer to that
shape.

## Keep exploring

- Follow the [end-to-end tutorial](./tutorial.md) to build a complete four-layer spec from scratch
- Read [syu concepts](./concepts.md) for the reasoning behind the four layers
- Review [configuration](./configuration.md) before tightening validation in a real repository
- Read the [syu app browser guide](./app.md) to learn how to navigate the browser UI
- Check the [troubleshooting guide](./troubleshooting.md) when `syu validate` returns an unfamiliar error code
- If you run `syu report`, your own project can generate a local Markdown report.
  The checked-in `docs/generated/` links below exist in the `syu` repository itself,
  so a freshly initialized project will not have them yet.
- Browse the live [Specification Reference](https://ugoite.github.io/syu/docs/generated/site-spec)
  to see how `syu` self-hosts its own contract
- Open the live [validation report](https://ugoite.github.io/syu/docs/generated/syu-report)
  to inspect the checked-in repository status
