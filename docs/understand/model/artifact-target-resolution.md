# Exact ArtifactTarget resolution

An exact `ArtifactTarget` is resolved through the active adapter's explicit
selector contract. The result has exactly one of four states:

- `resolved`: one active semantic inventory unit matches;
- `unresolved`: no active semantic unit matches;
- `ambiguous`: more than one active semantic unit matches;
- `unsupported`: the adapter and selector combination is not part of the
  current v1 contract.

Only `resolved` targets provide authoritative evidence. The other states are
diagnostics and cannot become Binding ownership, verification coverage, or a
reverse-index relation. Ownership selectors remain a separate concept: a
path-prefix or module scope may describe responsibility, but it never creates
an exact Artifact identity.

## Adapter × selector matrix

| Adapter | Supported exact selectors |
| --- | --- |
| Rust, TypeScript, JavaScript, Python, Go, Shell | `file`, `symbol` |
| Markdown | `file`, `heading` |
| OpenAPI | `file`, `operation` |
| JSON, JSON Schema | `file`, `json-pointer` |
| HTML | `file`, `marker` |
| Declared | `file` |

The matrix is implemented by `mitase-workspace` and is the single compatibility
contract used by indexing and validation. YAML remains an inventory format for
discovery, but does not claim source-location-aware JSON Pointer resolution.

## Symbol identity grammar

Symbol selectors use `::` for qualification. A bare name is an exact leaf-name
selector within one target file; if several semantic definitions share that
leaf, the result is `ambiguous`. Qualified names select the complete trailing
semantic path.

- Rust: modules, functions, constants, statics, structs, enums, unions,
  traits, type aliases, and methods. Trait or impl methods use the containing
  type in the selector, for example `Service::new`; test functions are normal
  function identities with test exposure metadata.
- TypeScript/JavaScript: functions, classes, variables, and class methods;
  class methods use `Service::submit`.
- Python: functions, classes, and class methods; methods use
  `Service::submit`.
- Go: package-level functions, named types, and receiver methods; methods use
  `Service::Submit`. Functions in `_test.go` are test-exposure artifacts.
- Shell: `name() { ... }` and `function name { ... }` definitions, selected by
  their exact function name.

Resolution identity is the adapter/path/semantic-name identity supplied by the
inventory. Source line and byte ranges are derived spans, so moving a symbol
without changing its semantic name does not change the identity. Rename
correlation belongs to semantic-diff logic and is not used to resolve the
current target.
