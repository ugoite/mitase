# polyglot example

This example demonstrates a multi-language workspace where a single
requirement and feature are traced across **Rust**, **Python**, and
**TypeScript** source files simultaneously.

It is the checked-in reference workspace that matches
`syu init . --template polyglot` exactly.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-MIX-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-MIX-001` linked to `PHIL-MIX-001` |
| `docs/syu/requirements/core/polyglot.yaml` | `REQ-MIX-001` with traces in Rust, Python, and TypeScript |
| `docs/syu/features/languages/polyglot.yaml` | `FEAT-MIX-001` with implementation traces in all three languages |
| `src/trace.rs` | Rust sources with `mixed_rust_requirement` and `mixed_rust_feature` |
| `python/test_traceability.py` | Python test with `test_python_requirement` |
| `python/app.py` | Python source with `python_feature_impl` |
| `frontend/traceability.test.ts` | TypeScript test with `typescriptRequirementTest` |
| `frontend/feature.ts` | TypeScript source with `typescriptFeature` |

## Try it

```bash
cd examples/polyglot
syu validate .
syu list requirement
syu show REQ-MIX-001
syu app .
```

A successful `syu validate .` produces output similar to:

```
syu validate passed
workspace: examples/polyglot
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=6/6 features=6/6
```

## Key things to notice

- **Multi-language traces** — a single `REQ-MIX-001` declares test traces in
  three different `tests` language keys (`rust`, `python`, `typescript`). Each
  language block names the file, the symbol, and the required doc-comment string.
- **Per-language doc-comment conventions** — Rust uses `///` doc comments,
  Python uses docstrings (`"""…"""`), and TypeScript uses JSDoc (`/** … */`).
  Look at the respective source files to see how each format satisfies
  `doc_contains`.
- **Runtime configuration** — `syu.yaml` declares `runtimes.python.command`
  and `runtimes.node.command` so syu knows how to invoke each language parser.
- **Reciprocal links** — `REQ-MIX-001` and `FEAT-MIX-001` must reference
  each other in both directions.
