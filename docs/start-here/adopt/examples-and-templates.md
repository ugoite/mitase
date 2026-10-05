# Examples and templates

`mitase init` creates the minimal workspace: one `mitase.yaml` and the
spec-root marker. Use the checked-in `examples/` directories as reference
patterns for comparison and learning once the workspace exists, not as
bootstrap sources to copy.

Do not copy the repository root `mitase.yaml` as a starter. It is Mitase's
self-hosting dogfood profile: it governs a mature, multi-language repository
with explicit inventory and readiness probes. Start from `mitase init`, inspect
the resolved conventions with `mitase config effective .`, and consult the
closest example only when you need a concrete shape for a larger layout.

Recommended flow:

- run `mitase init .` in your repository
- run `mitase config effective .` and add explicit settings only for what the
  conventions miss
- write the first Requirement by hand, following the
  [tutorial](../first-run/tutorial.md)
- compare against the closest example when the layout grows beyond one
  capability

Example families:

- `examples/generic`: smallest v1 layout
- `examples/docs-first`: markdown, shell, and YAML ownership
- `examples/rust-only`, `python-only`, `go-only`, `typescript-only`: one-language examples
- `examples/java-only`, `ruby-only`, `csharp-fallback`: file-level ownership for languages without active symbol adapters
- `examples/polyglot`, `team-scale`, `browser-ui`: broader layouts
