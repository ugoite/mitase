# ruby-only example

This example demonstrates a minimal single-language Ruby workspace.
It contains one philosophy, one policy, one requirement, and one feature -
all mutually linked and traced to real Ruby source and test files.

It is the checked-in reference workspace that matches
`syu init . --template ruby-only` exactly.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-RB-001` - the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-RB-001` linked to `PHIL-RB-001` |
| `docs/syu/requirements/core/ruby.yaml` | `REQ-RB-001` with a test trace to `test/order_summary_test.rb` |
| `docs/syu/features/languages/ruby.yaml` | `FEAT-RB-001` with an implementation trace to `lib/order_summary.rb` |
| `test/order_summary_test.rb` | Ruby test file containing the traced test method |
| `lib/order_summary.rb` | Ruby source containing the traced implementation method |

## Try it

```bash
cd examples/ruby-only
syu validate .
syu list requirement
syu show REQ-RB-001
syu app .
```

A successful `syu validate .` produces output similar to:

```
syu validate passed
workspace: examples/ruby-only
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=1/1 features=1/1
```

## Key things to notice

- **Ruby tracing** - syu validates the declared Ruby symbols directly against
  the checked-in `.rb` files without forcing a fallback language key.
- **Small first-run story** - the example keeps the first requirement and
  feature tied to one Minitest method and one Ruby implementation method.
- **Reciprocal links** - `REQ-RB-001` and `FEAT-RB-001` each reference the
  other, and syu validates both directions.
