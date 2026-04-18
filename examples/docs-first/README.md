# docs-first example

This example demonstrates a small workspace for repositories where the most
important traced artifacts are documentation, shell automation, and checked-in
configuration rather than richly inspected application code.

It keeps the example intentionally small while showing two useful pattern-based
adapter shapes:

- an explicit shell symbol trace for a single script function
- wildcard YAML ownership for a file that intentionally belongs to one feature

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-DOCS-001` — docs-first repositories should still keep traceable intent |
| `docs/syu/policies/policies.yaml` | `POL-DOCS-001` linked to both requirements |
| `docs/syu/requirements/core/docs.yaml` | `REQ-DOCS-001` and `REQ-DOCS-002` |
| `docs/syu/features/documentation/docs.yaml` | `FEAT-DOCS-001` and `FEAT-DOCS-002` |
| `scripts/publish-docs.sh` | shell implementation containing `publish_release_notes` |
| `config/navigation.yaml` | a whole-file YAML artifact intentionally owned by one feature |
| `README.md` | markdown-backed acceptance anchors for both requirements |

## Try it

```bash
cd examples/docs-first
syu validate .
syu show REQ-DOCS-001
syu show FEAT-DOCS-002
```

## DocsFirstAcceptanceChecklist

- `REQ-DOCS-001` expects the release-note publishing flow to stay explicit.
- `FEAT-DOCS-001` traces directly to the shell symbol `publish_release_notes`.
- This mapping is valid without `doc_contains` because shell only supports
  pattern-based symbol existence today.

## DocsFirstNavigationChecklist

- `REQ-DOCS-002` expects one checked-in navigation file to stay easy to inspect.
- `FEAT-DOCS-002` owns `config/navigation.yaml` with `symbols: ["*"]`.
- Use wildcard ownership carefully: it works best when one file intentionally
  belongs to one feature instead of collecting unrelated concerns.
