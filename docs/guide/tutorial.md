# End-to-end tutorial

<!-- FEAT-DOCS-001 -->

This tutorial walks through building a small, realistic workspace from scratch. By the end you will have a four-layer spec (philosophy → policy → requirement → feature), a passing `syu validate .`, and a browsable `syu app .`.

The example project is **Filestore** — a minimal file-storage library.

---

## 1. Bootstrap the workspace

```bash
mkdir filestore && cd filestore
syu init .
```

If the repository already uses another documentation layout, initialize there
instead:

```bash
syu init . --spec-root docs/spec
```

If you already know the stable project stem you want to keep long-term, seed it
into the starter IDs immediately:

```bash
syu init . --id-prefix store
```

For a repository that is already clearly Rust-first, Python-first, or
polyglot, you can start from a closer scaffold instead:

```bash
syu init . --template rust-only
```

You can combine both flags, but the walkthrough below assumes the default
generic starter under `docs/syu` so the file names match verbatim.

`syu init` creates:

```
syu.yaml
docs/syu/
  philosophy/foundation.yaml
  policies/policies.yaml
  requirements/core/core.yaml
  features/
    features.yaml
    core/core.yaml
```

The relevant part of `syu.yaml` points the validator at that tree:

```yaml
# syu.yaml
spec:
  root: docs/syu
```

`--spec-root` changes both the generated directory and this `spec.root` value,
so you do not need to move the scaffold by hand after bootstrap. `--template`
keeps the same four layers but may swap the starter IDs and the initial
requirement/feature file names to better match the repository style.
`--id-prefix` replaces the generic starter IDs with project-specific IDs such
as `PHIL-STORE-001`, `POL-STORE-001`, `REQ-STORE-001`, and `FEAT-STORE-001`.
If only one layer needs a different prefix, use the corresponding
`--philosophy-prefix`, `--policy-prefix`, `--requirement-prefix`, or
`--feature-prefix` flag instead.

---

## 2. Write a philosophy entry

Replace the starter content in `docs/syu/philosophy/foundation.yaml`:

```yaml
category: Philosophy
version: 1
language: en

philosophies:
  - id: PHIL-STORE-001
    title: Data integrity is non-negotiable
    product_design_principle: >
      Every file stored by Filestore must arrive at the caller in exactly
      the byte-for-byte state it was written. Corruption must be detected,
      never silently accepted.
    coding_guideline: >
      All write paths must flush and fsync before returning success.
      All read paths must verify a checksum before returning data.
    linked_policies:
      - POL-STORE-001
```

> **Key fields**
> - `id` — a stable, unique identifier. The `PHIL-` prefix is conventional for philosophies.
> - `product_design_principle` — the *why* behind product decisions.
> - `coding_guideline` — the *how* that engineers must follow.
> - `linked_policies` — the policies that operationalise this philosophy.

---

## 3. Write a policy entry

Edit `docs/syu/policies/policies.yaml`:

```yaml
category: Policies
version: 1
language: en

policies:
  - id: POL-STORE-001
    title: Write paths must verify integrity on every operation
    summary: >
      Every store-write operation must confirm data integrity before
      reporting success to the caller.
    description: >
      Corruption at rest is one of the most expensive bugs to diagnose.
      This policy operationalises PHIL-STORE-001 by mandating an
      explicit integrity check (checksum or equivalent) on every write.
    linked_philosophies:
      - PHIL-STORE-001
    linked_requirements:
      - REQ-STORE-001
```

> **Reciprocal links are required.** `POL-STORE-001` links *up* to `PHIL-STORE-001`
> and the philosophy links *down* to `POL-STORE-001`. Both directions must be present
> or `SYU-graph-reciprocal-001` will fire.

---

## 4. Write a requirement

Edit `docs/syu/requirements/core/core.yaml`:

```yaml
category: Filestore Core Requirements
prefix: REQ-STORE

requirements:
  - id: REQ-STORE-001
    title: write() must return an error if the checksum does not match
    description: >
      After flushing bytes to disk, the store computes a checksum of the
      written data and compares it with the expected value. If they differ,
      write() returns an error; the partial file is removed.
    priority: high
    status: planned
    linked_policies:
      - POL-STORE-001
    linked_features:
      - FEAT-STORE-001
```

`status: planned` means the feature is not yet implemented. No trace entries are
required yet — the validator will reject them on a `planned` item.

---

## 5. Write a feature

Edit `docs/syu/features/core/core.yaml`:

```yaml
category: Filestore Core Features
version: 1

features:
  - id: FEAT-STORE-001
    title: Integrity-checked write
    summary: >
      The write() implementation computes a SHA-256 checksum after flush
      and returns an error when the checksum mismatches.
    status: planned
    linked_requirements:
      - REQ-STORE-001
```

In `docs/syu/features/features.yaml`, keep the generated metadata header from
`syu init` and add the new file entry under `files`:

```yaml
files:
  - kind: core
    file: core/core.yaml
```

If you add another feature document later, keep `docs/syu/features/features.yaml`
in sync. `syu validate` reports feature YAML files on disk that are missing from
that registry.

---

## 6. Validate — first run

```bash
syu validate .
```

The exact rule-count and next-step lines may change slightly across releases, but
a successful run should look like this:

```
syu validate passed
workspace: /absolute/path/to/filestore
definitions: philosophies=1 policies=1 requirements=1 features=1
...
What to do next:
  syu app /absolute/path/to/filestore
  syu browse /absolute/path/to/filestore
```

If you see `SYU-graph-reciprocal-001` it means a link is one-directional. Add
the missing reverse link and re-run. See the
[validation error reference](./getting-started.md#understanding-validation-output)
for a complete list of common errors.

---

## 7. Mark a feature as implemented

When the code for `FEAT-STORE-001` is written (say, in `src/store.rs`), change
the status and add trace entries in **both** the requirement and the feature:

**`docs/syu/requirements/core/core.yaml`** — add `tests`:

```yaml
    status: implemented
    linked_policies:
      - POL-STORE-001
    linked_features:
      - FEAT-STORE-001
    tests:
      rust:
        - file: src/store.rs
          symbols:
            - test_write_checksum_mismatch
          doc_contains:
            - REQ-STORE-001
```

**`docs/syu/features/core/core.yaml`** — add `implementations`:

```yaml
    status: implemented
    linked_requirements:
      - REQ-STORE-001
    implementations:
      rust:
        - file: src/store.rs
          symbols:
            - write
          doc_contains:
            - FEAT-STORE-001
```

In `src/store.rs` make sure the symbol and doc-comment are present:

```rust
/// FEAT-STORE-001: Integrity-checked write.
pub fn write(path: &Path, data: &[u8]) -> Result<()> {
    // ... flush, checksum, return error on mismatch
}

#[test]
fn test_write_checksum_mismatch() {
    // REQ-STORE-001: write() must return an error on checksum mismatch
    // ...
}
```

Run validation again — it should still pass with zero errors.

---

## 8. Explore with the CLI

```bash
# List all requirements
syu list requirement

# Inspect a single item
syu show REQ-STORE-001

# Interactive terminal browser
syu browse .
```

---

## 9. Open the browser UI

```bash
syu app .
```

This serves a local web UI at `http://localhost:<port>` (the port is printed on
startup). The UI shows the four layers as tabs, displays validation badges next
to each section, and lets you click through linked items.

---

## What's next?

- Add more layers: create additional requirement and feature documents, organise
  them in sub-folders, and update `features.yaml`.
- Read [syu concepts](./concepts.md) to understand the design decisions behind
  the four-layer model.
- Review [configuration](./configuration.md) to tighten validation thresholds
  for your team.
- Browse the built-in [validation rule catalog](../syu/features/validation/validation.yaml)
  to understand every check `syu validate` performs.
