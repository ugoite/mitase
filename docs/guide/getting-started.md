# Getting started with syu

<!-- FEAT-DOCS-001 -->

:::tip New to syu?
New to `syu`? Read [syu concepts](./concepts.md) first to understand the four
layers before you start editing YAML.
:::

Need a different level of guidance?

- Jump back to the
  [README quick start on GitHub](https://github.com/ugoite/syu/blob/main/README.md#quick-start)
  when you want the shortest install-to-validate path and are happy with a
  compact command card.
- Follow [existing repository adoption](./existing-repository.md) when the
  repository already has code and history and you want to add `syu` without
  treating it like a blank workspace.
- Stay on this page when you want the first workspace setup explained step by
  step, including why the manual YAML edits matter before validation.
- Follow the [end-to-end tutorial](./tutorial.md) when you want a realistic,
  worked repository story instead of the shortest setup path.
- Use the [trace adapter capability matrix](./trace-adapter-support.md) when
  you need to know which built-in languages support symbol validation only
  versus `doc_contains` and strict coverage.
- Jump to [troubleshooting](./troubleshooting.md) when validation or linking is
  already failing and you need to unblock a workspace.

If you are still deciding whether to adopt `syu`, start with the [repository-fit guide in the README](https://github.com/ugoite/syu/blob/main/README.md#before-you-install-check-whether-syu-fits-this-repository) before installing anything. This guide assumes you have already made that call and want the slower first-workspace path. It still slows down at the first manual editing step so you can see where the scaffold lands, how reciprocal links fit together, and what to fix before the first `syu validate .` run.

## Is syu right for this repository?

Use the canonical fit check in the
[README](https://github.com/ugoite/syu/blob/main/README.md#is-syu-right-for-this-repository)
to decide whether `syu` is the right level of structure for this repository.
That section stays the source of truth for who benefits most, when `syu` is too
heavy, and what trade-offs it adds to the contributor loop.

If the README fit check sounds right and you want the first-run setup narrated
step by step, continue below. If not, stay with a lighter docs-only workflow
until the repository needs stronger traceability guarantees.

## Before you begin

Make sure `syu` is installed and available on your `PATH`.

**Option A — Verify the release installer before running it (recommended)**

For security-sensitive environments, download the installer and checksum file
first, verify the checksum locally, then run the checked file:

```bash
RELEASE=v0.0.1-alpha.7
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" -o install-syu.sh
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/checksums.sha256" -o checksums.sha256
sha256sum --ignore-missing -c checksums.sha256
bash install-syu.sh
```

On macOS, run the checksum check with the matching command instead:

```bash
shasum -a 256 --ignore-missing -c checksums.sha256
```

The getting-started path above verifies the installer script itself. If you need
additional archive-level provenance checks, download the platform archive
separately and verify it before installation.

**Option B — Run the installer directly**

```bash
RELEASE=v0.0.1-alpha.7
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_VERSION=alpha bash
```

This keeps the download URL pinned to the current checked-in release while
`SYU_VERSION=alpha` tells the installer to fetch the latest published alpha
package once it starts. Use this shortcut when you already trust the release
source and want the shortest path. It places `syu` in `~/.local/bin`. Add that
directory to your `PATH` if it is not already there.

**Windows — Use PowerShell for the zip, Git Bash for the installer script**

If you are on Windows and want a first-party path that does not depend on Git
Bash or WSL, download the Windows archive directly from PowerShell:

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

If a fresh shell still cannot find `syu`, add
`$env:LOCALAPPDATA\Programs\syu\bin` to your user `PATH`.

If you prefer the shell installer on Windows, run `install-syu.sh` from Git
Bash. It resolves the same `x86_64-pc-windows-msvc` archive and installs to
`%LOCALAPPDATA%\Programs\syu\bin`.

If you are inside WSL, use the Linux installer path instead. There, the script
resolves the Linux archive and installs to `~/.local/bin`.

**Option C — Build from source**

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

If the repository already exists and you do not want to start with `syu init`,
use [existing repository adoption](./existing-repository.md) instead. The steps
below assume a new workspace or a deliberate scaffold flow.

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
syu templates
syu init . --template rust-only
```

Run `syu templates` first if you want the starter names, one-line descriptions,
and matching checked-in example paths before choosing a scaffold.

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

For a genuinely mixed-language repository, keep the first adoption step small:

- start with `validate.require_symbol_trace_coverage: false`
- keep tracing every area you touch, but use file-level or wildcard ownership
  only in supported lightweight adapters that `syu` cannot inspect deeply yet
- keep unsupported implementation-language areas connected through the spec
  layers until adapter support lands
- turn stricter symbol coverage on later for the Rust, Python, and
  TypeScript/JavaScript areas once those traces are stable

That keeps the repository connected to the spec from day one without forcing a
polyglot team to fake symbol-level coverage before the current adapters are
ready.

Starter requirements and features begin as `status: planned`. Keep them planned
until you are ready to declare real tests and implementation traces.

### Unsupported implementation languages can still adopt the spec layers first

`syu` can validate code-level traces today in Rust, Python, and
TypeScript/JavaScript, plus lighter file/symbol ownership in `shell`, `yaml`,
`json`, `markdown`, and `gitignore`. Repositories that are mostly Go, Java,
C#, or another unsupported implementation language can still adopt `syu` today,
but they should treat code-level mappings for those files as future work.

Today you can still:

- document philosophy, policy, requirements, and features for
  unsupported-language areas
- keep those areas in the same repository while
  `validate.require_symbol_trace_coverage` stays `false`
- trace supported lightweight files such as shell scripts or YAML configs with
  file paths, explicit symbols, or wildcard ownership as long as you omit
  `doc_contains`

A minimal trace mapping without `doc_contains` looks like this:

```yaml
implementations:
  shell:
    - file: scripts/install-syu.sh
      symbols:
        - install_syu
```

When one feature intentionally owns the whole file, wildcard ownership still
works too:

```yaml
implementations:
  shell:
    - file: scripts/install-syu.sh
      symbols:
        - "*"
```

What you should avoid for unsupported-language files today is adding
language-specific `tests:` or `implementations:` entries such as `go:`,
`java:`, or `csharp:`. Those keys still fail validation before `doc_contains`
support even becomes relevant. If you need code-level tracing immediately, stay
with Rust, Python, or TypeScript/JavaScript for now; otherwise, keep the spec
layers connected and add the source mappings once adapter support lands.

Keep this adoption path in mind for mixed-language repositories too: start with
declared traces, keep `validate.require_symbol_trace_coverage: false`, then turn
strict coverage on later for the languages `syu` can already scan deeply.

When `SYU-trace-docsupport-001` fires, read it as “this mapping can stay, but
without `doc_contains`, only if the language adapter already exists.” That
works for `shell`, `yaml`, `json`, `markdown`, and `gitignore`; it does not
bypass `SYU-trace-language-001` for Go, Java, or C#.

Language-support roadmap:

- [Go trace validation and symbol ownership (#280)](https://github.com/ugoite/syu/issues/280)
- [Java trace validation and symbol ownership (#282)](https://github.com/ugoite/syu/issues/282)
- [C# trace validation and symbol ownership (#314)](https://github.com/ugoite/syu/issues/314)

Not sure whether you should scaffold a template or study a working repository
first? Use the [examples and templates guide](./examples-and-templates.md) to
choose the shorter path.

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
- **`doc_contains`** — Optional strings that must appear verbatim in the
  documentation comment of each listed symbol. Use this when you want the code
  itself to carry extra review breadcrumbs beyond the checked-in file/symbol
  mapping.

For example, a Rust test function that satisfies
`doc_contains: ["integrity-checked write"]` looks like this:

```rust
/// Verifies the integrity-checked write path.
#[test]
fn test_store_upload() { /* … */ }
```

`syu validate` reads the doc comment and checks that `"integrity-checked write"`
appears in it. If the string is missing, the rule `SYU-trace-doc-001`
fires with a suggestion to add the snippet (or run `syu validate --fix`).
If checked-in YAML plus the symbol name already gives you enough traceability,
you can omit `doc_contains` entirely and keep source files free of spec-ID
bookkeeping.

That richer `doc_contains` inspection is currently limited to Rust, Python, and
TypeScript / JavaScript traces. The same built-in matrix also tells you which
languages participate in strict `validate.require_symbol_trace_coverage`
inventory and which ones stop at symbol-existence checks: see the [trace
adapter capability matrix](./trace-adapter-support.md).

Requirements should declare tests:

```yaml
status: implemented
tests:
  rust:
    - file: src/trace.rs
      symbols:
        - requirement_test   # name of the test function
      doc_contains:
        - checksum mismatch is rejected
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
        - integrity-checked write
```

## 4. Validate the workspace

```bash
syu validate .
syu browse .
syu list feature
syu show REQ-001
syu app .
```

Use `syu list` when you want list-shaped output that can be narrowed to one
layer or emitted as JSON for automation. Use `syu browse --non-interactive`
when you want the browse snapshot instead: workspace metadata, per-layer
counts, and the current validation errors in plain text.

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
shape. `syu templates` prints the same mapping directly in the CLI, including
which starter is template-only (`generic`) versus backed by a checked-in
example.

For a side-by-side decision table that explains which paths are template-backed,
example-backed, or both, see the
[examples and templates guide](./examples-and-templates.md).

## Keep exploring

- Follow the [end-to-end tutorial](./tutorial.md) to build a complete four-layer spec from scratch
- Read [syu concepts](./concepts.md) for the reasoning behind the four layers
- Review [configuration](./configuration.md) before tightening validation in a real repository
- Check the [trace adapter capability matrix](./trace-adapter-support.md) before depending on `doc_contains` or strict ownership coverage in a mixed-language codebase
- Read the [syu app browser guide](./app.md) to learn how to navigate the browser UI
- Check the [troubleshooting guide](./troubleshooting.md) when `syu validate` returns an unfamiliar error code
- If you run `syu report`, your own project can generate a local Markdown report.
  The checked-in `docs/generated/` links below exist in the `syu` repository itself,
  so a freshly initialized project will not have them yet.
- Browse the live [Specification Reference](https://ugoite.github.io/syu/docs/generated/site-spec)
  to see how `syu` self-hosts its own contract
- Open the live [validation report](https://ugoite.github.io/syu/docs/generated/syu-report)
  to inspect the checked-in repository status
