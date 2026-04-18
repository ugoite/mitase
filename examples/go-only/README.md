# go-only example

This example demonstrates a minimal Go-first workspace using the built-in Go
trace adapter.

It contains one philosophy, one policy, one requirement, and one feature, plus
one Go source file and one Go test file. The example uses pattern-based symbol
matching for the real `.go` files, so `syu validate .` proves the Go-backed
links directly.

This workspace is reference-only: it does not correspond to a
`syu init --template ...` starter.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-GO-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-GO-001` linked to `PHIL-GO-001` |
| `docs/syu/requirements/core/go.yaml` | `REQ-GO-001` with a Go test trace |
| `docs/syu/features/languages/go.yaml` | `FEAT-GO-001` with a Go implementation trace |
| `go/app.go` | Go source file containing `GoFeatureImpl` |
| `go/app_test.go` | Go test file containing `TestGoRequirement` |
| `README.md` | Explains what the Go adapter validates today |

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

## What the Go adapter validates today

- `TestGoRequirement` lives in `go/app_test.go` and is the validated test symbol
  for `REQ-GO-001`.
- `GoFeatureImpl` lives in `go/app.go` and is the validated implementation
  symbol for `FEAT-GO-001`.
- The Go adapter currently supports pattern-based symbol validation and strict
  ownership coverage, but not `doc_contains` checks.

## Key things to notice

- **Go files are traced directly** — the requirement and feature point at the
  real `.go` files instead of a markdown workaround.
- **Pattern-based matching is enough here** — `syu` validates that the named Go
  symbols exist, even though the adapter does not inspect doc comments.
- **Strict coverage is available elsewhere** — the Go adapter participates in
  `validate.require_symbol_trace_coverage`, but this example keeps the minimal
  `go/` layout and is not demonstrating the stricter ownership-inventory path.
