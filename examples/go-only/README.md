# go-only example

This example demonstrates a minimal Go-first workspace for repositories that
want to adopt `syu` before Go gets a built-in trace adapter.

It contains one philosophy, one policy, one requirement, and one feature, plus
one Go source file and one Go test file. The current workaround keeps the
validated trace evidence in this README so `syu validate .` can still prove the
links today while the real Go files stay visible in the repository.

This workspace is reference-only: it does not correspond to a
`syu init --template ...` starter.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-GO-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-GO-001` linked to `PHIL-GO-001` |
| `docs/syu/requirements/core/go.yaml` | `REQ-GO-001` with a markdown-backed test trace |
| `docs/syu/features/languages/go.yaml` | `FEAT-GO-001` with a markdown-backed implementation trace |
| `go/app.go` | Go source file containing `GoFeatureImpl` |
| `go/app_test.go` | Go test file containing `TestGoRequirement` |
| `README.md` | Current-day trace evidence for the Go symbols |

## Try it

```bash
cd examples/go-only
syu validate .
syu list requirement
syu show REQ-GO-001
syu app .
```

A successful `syu validate .` produces output similar to:

```text
syu validate passed
workspace: examples/go-only
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=1/1 traces validated; features=1/1 traces validated
```

## Current-day workaround trace anchors

- `TestGoRequirement` lives in `go/app_test.go` and is the real test symbol for
  `REQ-GO-001`.
- `GoFeatureImpl` lives in `go/app.go` and is the real implementation symbol for
  `FEAT-GO-001`.
- Until `syu` ships a Go adapter, the validated trace mappings point at this
  README under the `markdown` adapter instead of the `.go` files directly.

## Key things to notice

- **Go files are present today** — the repository shape still looks like a real
  Go project with implementation and test files you can inspect immediately.
- **Trace evidence stays explicit** — the requirement and feature are marked
  `implemented`, but the validated `tests:` and `implementations:` mappings point
  at this README because `go` is not a supported trace language yet.
- **Direct Go tracing is future work** — once `syu` gains a Go adapter, the same
  spec IDs can move from these markdown-backed anchors to the real `.go` files.
