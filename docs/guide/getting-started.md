# Getting started

Install a release build or run the binary from source, then create a v1 workspace.

```bash
# x-release-please-start-version
RELEASE="$(gh release view --json tagName -q .tagName --repo ugoite/syu)"
# x-release-please-end
curl -fsSL "https://github.com/ugoite/syu/releases/download/${RELEASE}/install-syu.sh" | env SYU_VERSION=alpha bash
cargo run --quiet -- validate .
```

Current root CLI surface:

- `validate`
- `work plan`
- `work show`
- `work export-context`
- `workbench project`
- `lsp`

A new workspace needs:

- `syu.yaml`
- a `docs/syu` tree with philosophy, policy, requirement, and feature documents
- artifact roots that point at real implementation and verification files

Use `examples/` as the reference bootstrap path for concrete layouts.
