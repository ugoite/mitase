# csharp-fallback example

This example demonstrates a lighter staged adoption pattern for a repository
whose main application language is C#, even when the team is not ready to trace
every C# file directly yet.

It keeps a real C# source file and test file in the repository, but it does not
trace those C# files directly yet. Instead, the example keeps the higher-layer
spec links explicit and validates the surrounding automation in supported
lightweight adapters.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-CSHARP-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-CSHARP-001` linked to `PHIL-CSHARP-001` |
| `docs/syu/requirements/core/csharp.yaml` | `REQ-CSHARP-001` with a markdown-backed acceptance anchor |
| `docs/syu/features/languages/csharp.yaml` | `FEAT-CSHARP-001` with a shell-backed implementation trace |
| `scripts/check-workspace.sh` | shell implementation containing `verify_spec_links` |
| `src/OrderSummary.cs` | real C# source kept visible while the example focuses on surrounding workflow traces first |
| `tests/OrderSummaryTests.cs` | real C# test kept visible while the example focuses on surrounding workflow traces first |
| `README.md` | explains why the example delays direct C# tracing |

## Try it

```bash
cd examples/csharp-fallback
syu validate .
syu list feature
syu show REQ-CSHARP-001
syu app .
```

A successful `syu validate .` produces output similar to:

```text
syu validate passed
workspace: examples/csharp-fallback
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=1/1 traces validated; features=1/1 traces validated
```

## CsharpFallbackAcceptanceChecklist

- `REQ-CSHARP-001` expects staged-adoption repositories to keep the spec layers
  explicit from the first commit.
- `FEAT-CSHARP-001` traces the supporting shell workflow with the named
  `verify_spec_links` symbol.
- The checked-in C# files stay visible to contributors, but this example
  intentionally delays `tests.csharp` and `implementations.csharp` entries so
  the lighter adoption path stays obvious.

## Key things to notice

- **Real C# files still belong in the repository even before full tracing** —
  `syu` can help the team connect intent and workflow before they trace every
  code symbol directly.
- **Supported lightweight traces are the bridge** — shell and markdown keep the
  repository mechanically connected to the spec while code-level C# traces roll
  out more gradually.
- **Go is no longer the workaround** — use `examples/go-only` when you want the
  built-in Go adapter, and use this example when you want a lighter C#-first
  rollout.
