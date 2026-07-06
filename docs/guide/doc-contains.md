# doc_contains migration note

The active v1 model does not use the historical `doc_contains` trace shape.

Use explicit bindings instead:

- `role: implementation` for owned delivery files
- `role: verification` for evidence files
- `role: documentation` for explanatory material

When a file must be owned without symbol-level parsing, use a file selector and a supported adapter.
