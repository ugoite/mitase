# Examples and templates

The active v1 CLI ships executable validation and work-planning commands. It does not currently expose a scaffold generator.

Use the checked-in `examples/` directories as the source of truth for starter layouts.

Do not copy the repository root `syu.yaml` as a starter. It is Syu's self-hosting dogfood profile: it governs a mature, multi-language repository with explicit inventory and readiness probes. Start from the closest example, then expand its scope only as exact ownership, verification, and plans become real.

Recommended flow:

- copy the closest example into your repository structure
- update `syu.yaml` with your real `workspace.spec_roots` and `workspace.excludes`
- rewrite the four spec layers in `docs/syu`
- run `cargo run --quiet -- validate <workspace>` until clean

Example families:

- `examples/generic`: smallest v1 layout
- `examples/docs-first`: markdown, shell, and YAML ownership
- `examples/rust-only`, `python-only`, `go-only`, `typescript-only`: one-language examples
- `examples/java-only`, `ruby-only`, `csharp-fallback`: file-level ownership for languages without active symbol adapters
- `examples/polyglot`, `team-scale`, `browser-ui`: broader layouts
