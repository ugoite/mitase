# Tutorial

This tutorial creates a minimal v2 authoring workspace by hand. The v2
documents are normalized into Mitase's canonical specification graph before
validation. The first successful workspace is intentionally smaller than the
full four-layer story: one config file and one short requirement document are
enough to check a connected criterion, implementation target, and verification
target.

1. Add `mitase.yaml` with one line: `schema: mitase/config/v1`.
2. Create `docs/mitase/requirement.yaml` using the short form below.
3. Add the referenced `src/example.rs` and `tests/example.rs` files.
4. Run `mitase check .`.

The default `text` output is the Balanced Hybrid human view. For the same
validation result in a CI-friendly stream or a machine contract, run:

```bash
mitase check . --format compact
mitase check . --format json
```

Compact output keeps the outcome and each diagnostic on one line. JSON keeps
the same validation payload fields and adds the top-level
`schema_version: "mitase/cli/v1"`. The command exit code remains the authority
for automation; see the [CLI machine contract](../../workflows/repository/cli-machine-contract.md)
for the exit-code matrix.

A minimal requirement slice looks like this:

```yaml
schema: mitase/authoring/v2
kind: requirement
namespace: demo
category: Demo
requirement:
  id: REQ-DEMO-001
  title: Keep one behavior exact
  description: Example requirement.
  priority: high
  status: implemented
  criterion:
    id: exact-behavior
    kind: behavior
    statement: One implementation path has one verification path.
  implementation:
    facet: delivery
    responsibility: Implement the example behavior.
    target:
      path: src/example.rs
      satisfies: exact-behavior
  verification:
    facet: verification
    responsibility: Verify the example behavior.
    target:
      path: tests/example.rs
      verifies:
        criterion: exact-behavior
        covers: [source]
        runner: cargo-test
        arguments: { package: first-run, test: example_is_available }
```

The short form supplies only mechanical details: binding and target IDs,
file selectors, and a unique adapter inferred from each file extension. The
criterion text, binding responsibilities, claim relationships, coverage, and
verification runner remain explicit. The checked-in
[`fixtures/first-run-short`](../../../fixtures/first-run-short) is the CI
acceptance copy of this tutorial path.

Existing v1 source can still be converted during the 0.1.x dogfood period with
the read-only `mitase migrate <source> --stdout` command. Use the checked-in
examples and the [short authoring contract](../../workflows/repository/authoring-v2.md)
for larger v2 layouts.
