# rust-only example

This example demonstrates a minimal single-language Rust workspace.
It contains one philosophy, one policy, one requirement, and one feature —
all mutually linked and traced to a Rust source file.

It is the checked-in reference workspace that matches `syu init . --template rust-only`.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-RUST-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-RUST-001` linked to `PHIL-RUST-001` |
| `docs/syu/requirements/core/rust.yaml` | `REQ-RUST-001` with a test trace to `src/trace.rs` |
| `docs/syu/features/languages/rust.yaml` | `FEAT-RUST-001` with an implementation trace to `src/trace.rs` |
| `src/trace.rs` | Rust source containing the traced symbols |

## Try it

```bash
cd examples/rust-only
syu validate .
syu list requirement
syu show REQ-RUST-001
syu app .
```

A successful `syu validate .` produces output similar to:

```
syu validate passed
workspace: examples/rust-only
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=2/2 features=2/2
```

## Key things to notice

- **Reciprocal links** — `REQ-RUST-001` lists `FEAT-RUST-001` in its
  `linked_features`, and `FEAT-RUST-001` lists `REQ-RUST-001` back in its
  `linked_requirements`. Both directions are required.
- **`doc_contains`** — the trace entries require certain strings to appear in
  the doc comments of the traced symbols inside `src/trace.rs`. Open that file
  to see how `/// requirement doc line` and `/// feature doc line` satisfy the
  constraint.
- **`symbols`** — each trace names the exact function (`rust_requirement_test`,
  `rust_feature_impl`) that must exist in the file.
