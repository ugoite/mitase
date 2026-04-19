# csharp-fallback example

This example demonstrates the current fallback pattern for a repository whose
main application language still does not have a built-in `syu` trace adapter.

It keeps a real C# source file and test file in the repository, but it does
not add unsupported `csharp:` trace mappings yet. Instead, the example keeps
the higher-layer spec links explicit and validates the surrounding automation in
supported lightweight adapters.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-CSHARP-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-CSHARP-001` linked to `PHIL-CSHARP-001` |
| `docs/syu/requirements/core/csharp.yaml` | `REQ-CSHARP-001` with a markdown-backed acceptance anchor |
| `docs/syu/features/languages/csharp.yaml` | `FEAT-CSHARP-001` with a shell-backed implementation trace |
| `scripts/check-workspace.sh` | shell implementation containing `verify_spec_links` |
| `src/OrderSummary.cs` | real C# source kept visible but intentionally untraced for now |
| `tests/OrderSummaryTests.cs` | real C# test kept visible but intentionally untraced for now |
| `README.md` | explains why the C# files are not traced directly yet |

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

- `REQ-CSHARP-001` expects unsupported-language repositories to keep the spec
  layers explicit from the first commit.
- `FEAT-CSHARP-001` traces the supporting shell workflow with the named
  `verify_spec_links` symbol.
- The checked-in C# files stay visible to contributors, but this example does
  **not** add `tests.csharp` or `implementations.csharp` entries yet because
  those mappings still fail with `SYU-trace-language-001`.

## Key things to notice

- **Real unsupported-language files still belong in the repository** — `syu`
  can help the team connect intent and workflow before the language adapter
  exists.
- **Supported lightweight traces are the bridge** — shell and markdown keep the
  repository mechanically connected to the spec while code-level C# traces wait
  for native support.
- **Go is no longer the workaround** — use `examples/go-only` when you want the
  built-in Go adapter, and use this example when your main implementation
  language is still unsupported.
