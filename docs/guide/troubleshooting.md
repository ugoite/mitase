# Troubleshooting `syu validate` errors

When you run `syu validate .` for the first time on a project you will likely
encounter a handful of recurring error codes. This guide explains the most
common ones in plain English, shows a minimal example that triggers each
problem, and walks you through the fix — including when
`syu validate . --fix` is safe to use.

---

## How to read an error message

```
error[SYU-graph-orphaned-001] REQ-AUTH-001 (requirements/auth.yaml)
  → Definitions must not be isolated from the layered graph
```

- **`error` / `warning`** — severity. Use `--severity error` to show only
  blockers.
- **`SYU-graph-orphaned-001`** — the rule code. Use `--genre graph` to filter
  by genre.
- **`REQ-AUTH-001 (requirements/auth.yaml)`** — the offending item and the file
  that declares it.

> **Tip:** `syu validate . --format json` pipes machine-readable output to
> scripts or CI matchers.

---

## Workspace errors

### `SYU-workspace-load-001` — workspace fails to load

**What it means:** `syu` could not parse or read your workspace root, `syu.yaml`,
or one of your YAML spec files.

**Typical causes:**
- Malformed YAML (missing quotes, bad indentation, tab characters)
- `syu.yaml` missing or pointing to a non-existent `spec.root` directory
- File permissions that prevent reading

**Fix:**

1. Run `syu validate . 2>&1 | grep -Ei "error|parse"` to see the raw parse
   error.
2. Open the reported file and check the YAML syntax (use a YAML linter or
   `python3 -c "import yaml; yaml.safe_load(open('file.yaml'))"`).
3. Verify `syu.yaml` has a valid `spec.root` path pointing to an existing
   directory.

---

### `SYU-workspace-blank-001` — required fields are empty

**What it means:** A spec entry has a required field that is present but blank.
Depending on the layer, that can include fields such as `id`, `title`,
`status`, `description`, `priority`, `summary`, `product_design_principle`, or
`coding_guideline`.

**Example (triggers the error):**
```yaml
- id: REQ-001
  title: ""        # ← blank title
  description: Authenticate users
  status: implemented
```

**Fix:** Fill in every required field. `syu validate . --format json` shows
exactly which field is blank.

---

### `SYU-workspace-duplicate-001` — duplicate ID

**What it means:** Two entries share the same `id` value anywhere in the spec.

**Fix:** Rename one of them and update every cross-reference. Use
`grep -r "REQ-001" docs/` to find all usages before renaming.

---

## Graph errors

### `SYU-graph-orphaned-001` — item not connected to the graph

**What it means:** A philosophy, policy, requirement, or feature exists in the
spec but has no links to any adjacent layer.

**Example (triggers the error):**
```yaml
# requirements/auth.yaml
- id: REQ-AUTH-001
  title: Users must authenticate
  description: The system must verify identity before granting access.
  status: implemented
  # ← missing: linked_features field
```

**Fix:** Add a `linked_features:` list pointing to at least one feature that
implements this requirement, *and* add a `linked_requirements:` back-reference
in that feature.

```yaml
# requirements/auth.yaml
  linked_features:
    - FEAT-AUTH-001

# features/auth.yaml
- id: FEAT-AUTH-001
  ...
  linked_requirements:
    - REQ-AUTH-001
```

> **Escape hatch:** For a one-off run, use
> `syu validate . --require-non-orphaned-items=false`. If the repository should
> stay relaxed by default, set `validate.require_non_orphaned_items: false` in
> `syu.yaml`.

---

### `SYU-graph-reciprocal-001` — missing back-link

**What it means:** Item A references item B, but B does not reference A back.

**Example (triggers the error):**
```yaml
# requirements/auth.yaml — REQ-AUTH-001 lists FEAT-AUTH-001
linked_features:
  - FEAT-AUTH-001

# features/auth.yaml — FEAT-AUTH-001 does NOT list REQ-AUTH-001
linked_requirements: []   # ← missing back-reference
```

**Fix:** Add the reverse reference to the other layer's YAML file.

> **Escape hatch:** For a one-off run, use
> `syu validate . --require-reciprocal-links=false`. If the repository should
> stay relaxed by default, set `validate.require_reciprocal_links: false` in
> `syu.yaml`.

---

### `SYU-graph-reference-001` — referenced ID does not exist

**What it means:** An item's `linked_philosophies`, `linked_policies`,
`linked_requirements`, or `linked_features` list contains an ID that is not
declared anywhere in the spec.

**Typical causes:**
- Typo in the ID (`FEAT-AUTH-01` instead of `FEAT-AUTH-001`)
- The referenced item was deleted without updating its references

**Fix:** Either declare the missing item, or remove the stale reference.

---

### `SYU-graph-links-001` — missing adjacent-layer links (warning)

**What it means:** An item doesn't link to the adjacent layers it would
logically influence or satisfy. This is a *warning*, not a hard error.

**Fix:** Review whether the item should reference adjacent layers. If the gap
is intentional, treat this warning as advisory. There is no per-rule config to
disable only this warning; when you only want blockers, run
`syu validate . --severity error`.

---

## Delivery / status errors

### `SYU-delivery-planned-002` — planned item has delivery traces

**What it means:** An item with `status: planned` declares implementation or
test traces. Planning and claiming delivery at the same time is contradictory.

**Fix:** Either change the status to `implemented` once the work is actually
delivered, or remove the traces until the work is complete.

---

### `SYU-delivery-implemented-001` — implemented item has no traces

**What it means:** An item with `status: implemented` does not declare the
evidence that proves delivery: `tests:` for requirements or
`implementations:` for features.

**Example (triggers the error):**
```yaml
- id: FEAT-AUTH-001
  status: implemented
  # ← no implementations block
```

**Fix:** Add a `tests:` block (for requirements) or an `implementations:` block
(for features) pointing to the real file and symbol. For example, a feature
can declare:

```yaml
implementations:
  rust:
    - file: src/auth.rs
      symbols:
        - authenticate_user
```

---

## Trace errors

### `SYU-trace-symbol-003` — declared symbol not found in file

**What it means:** The `symbols:` entry in a trace does not match any function,
struct, method, or constant in the traced file.

**Common causes:**
- The function was renamed
- Wrong file path
- Symbol is in a sub-module, not the declared file

**Fix:** Run `grep -n "fn authenticate_user" src/auth.rs` (adjust for your
language) to confirm the exact symbol name, or change the trace to match the
current name.

---

### `SYU-trace-file-002` — trace file does not exist

**What it means:** The `file:` path in a trace does not exist on disk.

**Fix:** Check the path relative to the repository root. Common issues:
- The file was moved (`git log --follow -- old/path.rs`)
- Case mismatch on case-sensitive filesystems

---

### `SYU-trace-language-001` — unsupported trace adapter

**What it means:** The `lang:` key does not match any built-in adapter. Today
`syu` ships Rust, Python, TypeScript / JavaScript, Shell, YAML, JSON, Markdown,
and Gitignore adapters.

**Fix:** Change the trace to one of those built-in language aliases, or check
the [trace adapter capability matrix](./trace-adapter-support.md) before you
commit to `doc_contains` or strict coverage expectations for a new language.

---

### `SYU-trace-extension-001` — language/file mismatch

**What it means:** The `lang:` field says `rust` but the file ends in `.py`,
or vice versa.

**Fix:** Correct either the `lang:` value or the `file:` path so they agree.

---

### `SYU-trace-doc-001` — required doc snippet missing

**What it means:** A trace with `doc_contains:` asserts that a specific string
appears in the symbol's doc comment, but it doesn't.

**Fix:** Either update the doc comment to include the required phrase, or
remove the `doc_contains:` assertion from the trace if it no longer applies.

---

### `SYU-trace-docsupport-001` — language not supported for doc inspection

**What it means:** `doc_contains:` is only supported for `rust`, `python`, and
`typescript`. You declared it on a `lang:` that `syu` cannot inspect.

**Fix:** Remove the `doc_contains:` assertion from that mapping, or switch to a
supported language for rich doc inspection.

If the language already has a built-in adapter without rich doc inspection
(`shell`, `yaml`, `json`, `markdown`, or `gitignore`), you do **not** need to
remove the whole trace. Those mappings can still point to:

- the traced file
- explicit `symbols:`
- wildcard ownership with `symbols: ["*"]`

For example, this is still valid today:

```yaml
implementations:
  shell:
    - file: scripts/install-syu.sh
      symbols:
        - install_syu
```

Use that lighter mapping until the language gains richer inspection support.
If the mapping uses an unsupported implementation language such as `csharp`,
removing `doc_contains` is not enough: those entries still raise
`SYU-trace-language-001`. Keep the higher-layer spec link in place and wait for
adapter support before adding the code-level trace.
The
[`examples/csharp-fallback` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/csharp-fallback)
shows one concrete unsupported-language starting point that keeps real C#
source files in the repository while validated traces stay in supported shell
and markdown files. The
[`examples/go-only` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/go-only)
and `syu init . --template go-only` remain the concrete Go-first path with real
source files plus symbol-level trace mappings.

The [trace adapter capability matrix](./trace-adapter-support.md) shows which
built-in adapters stop at symbol validation, which ones can inspect docs, and
which languages participate in strict inventory coverage.

---

## Coverage errors

### `SYU-coverage-public-001` — public symbol has no owning feature

**What it means:** `validate.require_symbol_trace_coverage: true` is set and a
public API symbol in Rust, Python, or TypeScript is not referenced by any
feature trace.

**Fix:** Either add a trace to a feature that covers the symbol, or make the
symbol non-public if it is not part of the intended API.

- **Rust:** reduce visibility (for example, change `pub` to `pub(crate)` or
  private).
- **Python:** remove it from `__all__` or rename it to start with `_`.
- **TypeScript:** stop exporting it from the module.

> **Strictness toggle:** Use `syu validate . --require-symbol-trace-coverage`
> when you want to trial this stricter rule without committing a config change.
> The [trace adapter capability matrix](./trace-adapter-support.md) lists the
> languages that participate in this inventory in the current checked-in docs.

---

### `SYU-coverage-test-001` — test has no owning requirement

**What it means:** A test function or method is not referenced by any
requirement trace.

**Fix:** Add the test to a requirement's trace block, or rename/remove the test
if it is obsolete.

---

## When is `syu validate . --fix` safe?

`--fix` is safe for **additive, intent-preserving** repairs:

| What `--fix` does | Safe? |
|---|---|
| Inserts required `doc_contains` snippets using language-appropriate comment or doc-comment syntax | ✅ Yes |
| Rewrites or deletes symbols | ❌ No — `--fix` never does this |
| Adds missing `linked_*` graph links | ❌ No — only you know the correct links |
| Creates new spec entries | ❌ No |

Run `git diff` after `--fix` to review every change before committing.

---

## "Validation passes but traces feel wrong"

Passing `syu validate .` does not mean the spec perfectly reflects intent. For
common four-layer design smells that are still technically valid, read the
[spec anti-patterns guide](./spec-antipatterns.md) after you clear the blocking
errors below. Watch out for these false-confidence patterns:

- **Over-broad wildcards:** An empty `symbols:` list does not validate
  (`SYU-trace-symbol-001`). If you intentionally mean "this spec item owns the
  whole file", use `symbols: ["*"]` — but remember that wildcard traces are
  coarse and can hide missing symbol-level ownership.
- **Copy-pasted spec IDs:** If a symbol carries `// FEAT-001` but logically
  belongs to `FEAT-002`, validation won't complain — only reviewers will.
- **`status: implemented` without real evidence:** Adding `status: implemented`
  satisfies `SYU-delivery-implemented-001` only when traces also exist.
- **Disabled rules:** Check `syu.yaml` for `validate.*: false` settings
  introduced to silence errors rather than fix the underlying issue.

---

## Getting more help

- [Getting started guide](./getting-started.md) — use the shortest newcomer path
  when you want to rebuild a clean mental model before debugging
- [Trace adapter capability matrix](./trace-adapter-support.md) — check which
  built-in languages support symbol validation only versus `doc_contains` and
  strict ownership inventory
- [End-to-end tutorial](./tutorial.md) — follow a full working example when you
  want to compare your workspace against a realistic repository story
- [Spec anti-patterns](./spec-antipatterns.md) — use this when validation passes
  but the four-layer design still feels unstable, repetitive, or too broad
- Full rule catalog: [`docs/syu/features/validation/validation.yaml`](../syu/features/validation/validation.yaml)
- Filter by genre: `syu validate . --genre graph`
- Filter by severity: `syu validate . --severity error`
- Filter by spec item: `syu validate . --id REQ-001`
- Machine-readable output: `syu validate . --format json`
- [Configuration reference](./configuration.md) — explains every `syu.yaml`
  option that affects validation behaviour
