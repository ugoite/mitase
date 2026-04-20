# Getting started with syu

<!-- FEAT-DOCS-001 -->

:::tip New to `syu`?
If you already understand the four-layer model and only want the fastest command
path, jump to the
[README quick start on GitHub](https://github.com/ugoite/syu/blob/main/README.md#quick-start).
Stay on this page when you want the same first-run flow narrated step by step.
If you want the fuller mental model before you start editing YAML, read
[syu concepts](./concepts.md) first.
:::

Need a different level of guidance?

- Jump back to the
  [README quick start on GitHub](https://github.com/ugoite/syu/blob/main/README.md#choose-your-path)
  when you want the shortest install-to-validate path and are happy with a
  compact command card.
- Follow [existing repository adoption](./existing-repository.md) when the
  repository already has code and history and you want to add `syu` without
  treating it like a blank workspace.
- Stay on this page when you want the first workspace setup explained step by
  step, including why the manual YAML edits matter before validation and how the
  same commands fit together as one guided story.
- Follow the [end-to-end tutorial](./tutorial.md) when you want a realistic,
  worked repository story instead of the shortest setup path.
- Start with the [VS Code extension guide](./vscode-extension.md) when you want
  diagnostics, spec navigation, and related-file lookups in the editor before
  you settle into the terminal or browser flows.
- Use the [trace adapter capability matrix](./trace-adapter-support.md) when
  you need to know which built-in languages support symbol validation only
  versus `doc_contains` and strict coverage.
- Jump to [troubleshooting](./troubleshooting.md) when validation or linking is
  already failing and you need to unblock a workspace.

If you are still deciding whether to adopt `syu`, start with the
[repository-fit guide in the README](https://github.com/ugoite/syu/blob/main/README.md#is-syu-right-for-this-repository)
before installing anything.

## Is syu right for this repository?

Use the canonical fit check in the
[README](https://github.com/ugoite/syu/blob/main/README.md#is-syu-right-for-this-repository)
to decide whether `syu` is the right level of structure for this repository.
That section stays the source of truth for who benefits most, when `syu` is too
heavy, and what trade-offs it adds to the contributor loop.

If the README fit check sounds right and `syu` is already installed, continue
below. This guide is the canonical narrated first-run path: unlike the README
quick start, which works best as the compact command card, it slows down at the
first manual editing step so you can see where the scaffolded files live, how
reciprocal links fit together, and what to fix before the first
`syu validate .` run.

## Before you begin

Make sure `syu` is installed and available on your `PATH`.

**Option A — Verify the release installer before running it (recommended)**

Follow the canonical
[README installer verification flow](https://github.com/ugoite/syu/blob/main/README.md#recommended-verify-before-running)
when you want the current checked-in release tag, the `install-syu.sh` and
`checksums.sha256` download commands, plus the macOS `shasum -a 256` variant in
one maintained place. This is the best fit for security-sensitive environments.

The getting-started path above verifies the installer script itself. If you need
additional archive-level provenance checks, follow the same README section and
then verify the platform archive separately before installation with
`gh release download`, `gh attestation verify`, and the same
`--signer-workflow` / `--source-ref` pinning the README shows.

**Option B — Run the installer directly**

Use the canonical
[README shortcut installer entrypoint](https://github.com/ugoite/syu/blob/main/README.md#shortcut-run-the-installer-directly)
when you already trust the release source and want the shortest path. That
section keeps the checked-in release tag aligned in one place while
`SYU_VERSION=alpha` still points the installer at the latest published alpha
package after it starts. The installer places `syu` in `~/.local/bin`; add that
directory to your `PATH` if it is not already there.

**Windows — Use PowerShell for the zip, Git Bash for the installer script**

If you are on Windows and want a first-party path that does not depend on Git
Bash or WSL, follow the canonical
[README PowerShell install flow](https://github.com/ugoite/syu/blob/main/README.md#windows-powershell-by-default-git-bash-on-windows-wsl-as-linux).
That section keeps the checked-in `$release`, `checksums.sha256`, and `syu.exe`
steps aligned with the published installer docs, including the
`syu-x86_64-pc-windows-msvc.zip` archive name.

If you prefer the shell installer on Windows, run `install-syu.sh` from Git
Bash. The same README section explains how it resolves the
`x86_64-pc-windows-msvc` archive and installs to
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

Prefer a guided first run?

```bash
syu init . --interactive
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

Need a closer starting point for a repository that is already docs-first,
Rust-first, Python-first, Go-first, Java-first, or polyglot?

```bash
syu templates
syu init . --template docs-first
syu init . --template rust-only
syu init . --template go-only
syu init . --template java-only
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
`--requirement-prefix`, or `--feature-prefix`. `--interactive` asks those same
questions in the terminal, including whether to enable stricter validation
defaults immediately, then writes the same transparent checked-in files.

For a genuinely mixed-language repository, keep the first adoption step small:

- start with `validate.require_symbol_trace_coverage: false`
- keep tracing every area you touch, but use file-level or wildcard ownership
  only in supported lightweight adapters that `syu` cannot inspect deeply yet
- keep unsupported implementation-language areas connected through the spec
  layers until adapter support lands
- turn stricter symbol coverage on later for the supported implementation
  languages you are tracing (Rust, Python, Go, Java, or
  TypeScript/JavaScript) once those traces are stable

That keeps the repository connected to the spec from day one without forcing a
polyglot team to fake symbol-level coverage before the current adapters are
ready.

Starter requirements and features begin as `status: planned`. Keep them planned
until you are ready to declare real tests and implementation traces.

### Unsupported implementation languages can still adopt the spec layers first

`syu` can validate code-level traces today in Rust, Python, Go, Java, and
TypeScript/JavaScript, plus lighter file/symbol ownership in `shell`, `yaml`,
`json`, `markdown`, and `gitignore`. Repositories that are mostly C# or another
unsupported implementation language can still adopt `syu` today, but they
should treat code-level mappings for those files as future work.

Go and Java already have built-in symbol validation and participate in strict
`validate.require_symbol_trace_coverage` inventory. Go now supports
`doc_contains` checks as well, while Java still stops at symbol validation. The
[trace adapter capability matrix](./trace-adapter-support.md) summarizes that
language-by-language support.

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
language-specific `tests:` or `implementations:` entries such as `csharp:`.
Those keys still fail validation before `doc_contains` support even becomes
relevant. If you need code-level tracing immediately with `doc_contains`, stay
with Rust, Python, Go, or TypeScript/JavaScript for now. For Go-first repositories,
use [`examples/go-only` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/go-only)
or `syu init . --template go-only`: both use real Go files plus symbol-level
trace mappings that validate today.
For Java-first repositories, use
[`examples/java-only` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/java-only)
or `syu init . --template java-only`: both use real Java files plus
symbol-level trace mappings that validate today, even though `doc_contains`
is still out of scope for Java.
For unsupported-language repositories, use the
[`examples/csharp-fallback` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/csharp-fallback)
to study the fallback pattern.

Keep this adoption path in mind for mixed-language repositories too: start with
declared traces, keep `validate.require_symbol_trace_coverage: false`, then turn
strict coverage on later for the languages `syu` can already scan deeply.

When `SYU-trace-docsupport-001` fires, read it as “this mapping can stay, but
without `doc_contains`, only if the language adapter already exists.” That
works for `go`, `java`, `shell`, `yaml`, `json`, `markdown`, and `gitignore`;
it does not bypass `SYU-trace-language-001` for C#.

Language-support roadmap:

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
  `requirements/core/go.yaml`, `requirements/core/java.yaml`, or
  `requirements/core/polyglot.yaml`)
- `<spec.root>/features/core/core.yaml` (template starters may use
  `features/languages/rust.yaml`, `features/languages/python.yaml`,
  `features/languages/go.yaml`, `features/languages/java.yaml`, or
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
version: 0.0.1-alpha.8
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

That richer `doc_contains` inspection is currently available for Rust, Python,
Go, and TypeScript / JavaScript traces. The same built-in matrix also tells you which
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

If you only remember the task and not the command name yet, use this chooser:

| If you want to... | Start with... | Why |
| --- | --- | --- |
| check whether the workspace is healthy before deeper exploration | `syu validate .` | runs the validation pass first so you can fix structural problems before navigating the graph |
| inspect the whole workspace interactively in a terminal | `syu browse .` | best first stop when you want the layered graph and current validation state together |
| render one layer or emit machine-friendly lists | `syu list ...` | keeps the output list-shaped and scriptable |
| open one exact definition by ID | `syu show ID` | jumps directly to the matched philosophy, policy, requirement, or feature |
| search when you only know a keyword or partial ID | `syu search QUERY` | searches titles, summaries, descriptions, and IDs across the spec |
| start from a traced file or symbol during review | `syu trace path --symbol name` | works code-first instead of spec-first |
| inspect everything connected to one ID, file, or symbol | `syu relate TARGET` | expands nearby links, traced files, and symbols to show the surrounding context |
| review change history for one requirement or feature | `syu log ID` | follows the checked-in definition plus traced evidence through Git history |
| switch to a browser-first workflow | `syu app .` | shows the same workspace graph in the local browser UI |

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
the success summary and next-step guidance while keeping the exit code behavior.

Need warnings to stay visible in text output but still fail the job? Add
`--warning-exit-code 3` (or another non-zero code) so warning-only runs return
that code while error-bearing runs continue to return exit code 1.

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
for Rust, Python, Go, and TypeScript without guessing at larger structural changes.

## 6. Generate a report

```bash
syu report . --output reports/syu.md
```

## 7. Study the examples

The repository includes complete examples:

- `examples/rust-only`
- `examples/python-only`
- `examples/go-only`
- `examples/java-only`
- `examples/polyglot`

If one of those examples is already close to your repository, use the matching
`syu init --template ...` option so the starter scaffold begins nearer to that
shape. `syu templates` prints the same mapping directly in the CLI, including
which starter is template-only (`generic`) versus backed by a checked-in
example.
Use `syu init . --template docs-first` when your repository is documentation-led:
it scaffolds the same markdown, shell, and YAML starter shape as the checked-in
example without forcing a language-specific code scaffold first.
Use `syu init . --template go-only` when your repository is Go-first today: it
scaffolds the same minimal spec plus `go.mod`, `go/app.go`, and
`go/app_test.go`, while the checked-in example shows the same shape in a
repository you can inspect
before generating files locally.
Use `syu init . --template java-only` when your repository is Java-first today:
it scaffolds the same minimal spec plus `pom.xml`,
`src/main/java/example/app/OrderSummary.java`, and
`src/test/java/example/app/OrderSummaryTest.java`, while the checked-in example
shows the same shape in a repository you can inspect before generating files
locally.

For a side-by-side decision table that explains which paths are template-backed,
example-backed, or both, see the
[examples and templates guide](./examples-and-templates.md).

## Keep exploring

- Follow the [end-to-end tutorial](./tutorial.md) to build a complete four-layer spec from scratch
- Read [syu concepts](./concepts.md) for the reasoning behind the four layers
- Review [configuration](./configuration.md) before tightening validation in a real repository
- Use the [reviewer workflow guide](./reviewer-workflow.md) when a PR already exists and you want one trace/relate/log loop to inspect it
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
